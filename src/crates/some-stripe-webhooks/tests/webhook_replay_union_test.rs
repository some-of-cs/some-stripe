//! Integration contract tests for Stripe webhook replay tolerance and event union deserialization.
//!
//! Validates:
//! - Replay tolerance window (300 seconds default, Gate G2) and future drift bounds (60 seconds).
//! - Rejection of expired webhooks (`now - 301s`) with [`StripeWebhookError::TimestampOutsideTolerance`].
//! - Strongly typed deserialization of official Stripe API v1 snapshot payloads (`checkout.session.completed`).
//! - Graceful handling of unpersisted objects with null identifiers (`invoice.upcoming` where `data.object.id` is null).

use some_stripe_types::CheckoutSession;
use some_stripe_webhooks::{StripeWebhookError, StripeWebhookWrapper, Webhook};

/// tc_2_2_0_01: Assert that events with valid timestamps within the 300-second tolerance window pass verification.
#[test]
fn tc_2_2_0_01_valid_event_timestamp_within_300s() {
    let now = chrono::Utc::now().timestamp();
    let secret = "whsec_valid_replay_test_secret_0123456789";
    let payload = br#"{"id":"evt_test_valid_01","object":"event","created":1728345600,"type":"payment_intent.succeeded","data":{"object":{"id":"pi_valid_123","amount":2000,"currency":"usd","status":"succeeded"}}}"#;

    // 1. Current timestamp (drift = 0s)
    let header_now = Webhook::generate_test_header_string(payload, secret, Some(now));
    let res_now = Webhook::verify_header(payload, &header_now, secret, Some(300));
    assert!(
        res_now.is_ok(),
        "Event at current timestamp must pass verification"
    );

    // 2. Event at now - 150s (mid tolerance)
    let header_mid = Webhook::generate_test_header_string(payload, secret, Some(now - 150));
    let res_mid = Webhook::verify_header(payload, &header_mid, secret, Some(300));
    assert!(
        res_mid.is_ok(),
        "Event 150s old must pass verification within 300s tolerance window"
    );

    // 3. Event at now - 299s (edge of tolerance)
    let header_edge = Webhook::generate_test_header_string(payload, secret, Some(now - 299));
    let res_edge = Webhook::verify_header(payload, &header_edge, secret, Some(300));
    assert!(
        res_edge.is_ok(),
        "Event 299s old must pass verification within 300s tolerance window"
    );

    // 4. Event at now + 30s (within future drift tolerance of 60s)
    let header_future = Webhook::generate_test_header_string(payload, secret, Some(now + 30));
    let res_future = Webhook::verify_header(payload, &header_future, secret, Some(300));
    assert!(
        res_future.is_ok(),
        "Event 30s in future must pass verification within 60s future drift bound"
    );

    // 5. Full construct_event validation
    let event = Webhook::construct_event(payload, &header_now, secret, Some(300))
        .expect("construct_event must succeed for valid timestamp and signature");
    assert_eq!(event.id, "evt_test_valid_01");
    assert_eq!(event.event_type, "payment_intent.succeeded");
}

/// tc_2_2_0_02: Assert that expired webhooks (now - 301s) are rejected with `TimestampOutsideTolerance`.
#[test]
fn tc_2_2_0_02_expired_webhook_rejected() {
    let now = chrono::Utc::now().timestamp();
    let expired_timestamp = now - 301;
    let secret = "whsec_expired_test_secret_0123456789";
    let payload = br#"{"id":"evt_test_expired_02","object":"event","created":1728345600,"type":"payment_intent.succeeded","data":{"object":{"id":"pi_expired_123"}}}"#;

    let header = Webhook::generate_test_header_string(payload, secret, Some(expired_timestamp));

    // Verify verify_header returns TimestampOutsideTolerance
    let err_header = Webhook::verify_header(payload, &header, secret, Some(300))
        .expect_err("Expired webhook (now - 301s) must be rejected");

    match err_header {
        StripeWebhookError::TimestampOutsideTolerance {
            timestamp,
            now: reported_now,
            tolerance,
        } => {
            assert_eq!(timestamp, expired_timestamp);
            assert_eq!(tolerance, 300);
            assert!(reported_now >= now);
        }
        other => panic!(
            "Expected StripeWebhookError::TimestampOutsideTolerance, got: {:?}",
            other
        ),
    }

    // Verify construct_event fails identically
    let err_event = Webhook::construct_event(payload, &header, secret, Some(300))
        .expect_err("construct_event must reject expired webhook");

    assert!(matches!(
        err_event,
        StripeWebhookError::TimestampOutsideTolerance { tolerance: 300, .. }
    ));

    // Assert future drift beyond MAX_FUTURE_DRIFT_SECONDS (now + 61s) is also rejected
    let excessive_future_ts = now + 65;
    let header_future =
        Webhook::generate_test_header_string(payload, secret, Some(excessive_future_ts));
    let err_future = Webhook::verify_header(payload, &header_future, secret, Some(300))
        .expect_err("Excessive future timestamp (> 60s) must be rejected");

    assert!(matches!(
        err_future,
        StripeWebhookError::TimestampOutsideTolerance { .. }
    ));
}

/// tc_2_2_0_03: Deserialize verified `checkout.session.completed` event and assert strongly typed domain snapshot.
#[test]
fn tc_2_2_0_03_strongly_typed_v1_snapshot_deserialization() {
    let now = chrono::Utc::now().timestamp();
    let secret = "whsec_v1_snapshot_secret_0123456789";

    // Official Stripe API v1 snapshot for checkout.session.completed
    let payload = br#"{
        "id": "evt_1P0XYZ2eZvKYlo2C89ABCDEF",
        "object": "event",
        "api_version": "2024-06-20",
        "created": 1728345600,
        "type": "checkout.session.completed",
        "data": {
            "object": {
                "id": "cs_test_b1c2d3e4f5g6h7",
                "object": "checkout.session",
                "amount_total": 7500,
                "currency": "usd",
                "status": "complete",
                "payment_status": "paid",
                "customer": "cus_987654321",
                "payment_intent": "pi_1234567890abcdef",
                "client_reference_id": "cust_order_42",
                "url": "https://checkout.stripe.com/c/pay/cs_test_b1c2d3e4f5g6h7"
            }
        }
    }"#;

    let header = Webhook::generate_test_header_string(payload, secret, Some(now));

    // 1. Verify signature and construct Event
    let event = Webhook::construct_event(payload, &header, secret, Some(300))
        .expect("Signature verification and event parsing must succeed");

    assert_eq!(event.id, "evt_1P0XYZ2eZvKYlo2C89ABCDEF");
    assert_eq!(event.object, "event");
    assert_eq!(event.api_version.as_deref(), Some("2024-06-20"));
    assert_eq!(event.created, 1728345600);
    assert_eq!(event.event_type, "checkout.session.completed");

    // 2. Strongly typed deserialization of event data object into CheckoutSession domain model
    use serde::Deserialize;
    let session = CheckoutSession::deserialize(&event.data.object)
        .expect("CheckoutSession domain model must deserialize cleanly from event payload");

    assert_eq!(session.id, "cs_test_b1c2d3e4f5g6h7");
    assert_eq!(session.object, "checkout.session");
    assert_eq!(session.amount_total, Some(7500));
    assert_eq!(session.currency.as_deref(), Some("usd"));
    assert_eq!(session.status.as_deref(), Some("complete"));
    assert_eq!(session.payment_status.as_deref(), Some("paid"));
    assert_eq!(session.customer.as_deref(), Some("cus_987654321"));
    assert_eq!(
        session.payment_intent.as_deref(),
        Some("pi_1234567890abcdef")
    );
    assert_eq!(
        session.client_reference_id.as_deref(),
        Some("cust_order_42")
    );
    assert_eq!(
        session.url.as_deref(),
        Some("https://checkout.stripe.com/c/pay/cs_test_b1c2d3e4f5g6h7")
    );

    // 3. Ergonomic StripeWebhookWrapper contract verification
    let wrapper = StripeWebhookWrapper::from_raw(payload, &header, secret, Some(300))
        .expect("StripeWebhookWrapper::from_raw must succeed");

    assert_eq!(wrapper.event_id(), "evt_1P0XYZ2eZvKYlo2C89ABCDEF");
    assert_eq!(wrapper.order_id(), Some("cs_test_b1c2d3e4f5g6h7"));
    assert_eq!(wrapper.customer_id(), Some("cus_987654321"));
    assert_eq!(wrapper.payment_intent_id(), Some("pi_1234567890abcdef"));
    assert_eq!(wrapper.amount_total(), Some(7500));
    assert_eq!(wrapper.currency().as_deref(), Some("USD"));
    assert!(
        wrapper.is_checkout_completed(),
        "is_checkout_completed must return true for complete/paid session"
    );
}

/// tc_2_2_0_04: Deserialize `invoice.upcoming` fixture where `data.object.id` is null; assert no deserialization error.
#[test]
fn tc_2_2_0_04_invoice_upcoming_without_id_handled() {
    let now = chrono::Utc::now().timestamp();
    let secret = "whsec_upcoming_invoice_secret_0123456789";

    // Stripe API event snapshot for invoice.upcoming where id is null (unpersisted preview invoice)
    let payload = br#"{
        "id": "evt_1UpcomingInvoiceFixture999",
        "object": "event",
        "api_version": "2024-06-20",
        "created": 1728345600,
        "type": "invoice.upcoming",
        "data": {
            "object": {
                "id": null,
                "object": "invoice",
                "customer": "cus_upcoming_merchant_456",
                "amount_due": 3499,
                "currency": "usd",
                "status": "draft"
            }
        }
    }"#;

    let header = Webhook::generate_test_header_string(payload, secret, Some(now));

    // Construct Event directly from raw webhook bytes
    let event = Webhook::construct_event(payload, &header, secret, Some(300))
        .expect("invoice.upcoming with null id must deserialize into Event without error");

    assert_eq!(event.id, "evt_1UpcomingInvoiceFixture999");
    assert_eq!(event.event_type, "invoice.upcoming");
    assert_eq!(event.created, 1728345600);

    // Verify data.object contains id: null
    let id_field = event.data.object.get("id");
    assert!(id_field.is_some(), "id field must exist in data.object");
    assert!(
        id_field.unwrap().is_null(),
        "id field in invoice.upcoming must be serde_json::Value::Null"
    );

    // Verify StripeWebhookWrapper handles null id gracefully without panicking
    let wrapper = StripeWebhookWrapper::from_raw(payload, &header, secret, Some(300))
        .expect("StripeWebhookWrapper must safely wrap invoice.upcoming");

    assert_eq!(wrapper.event_id(), "evt_1UpcomingInvoiceFixture999");
    // order_id() inspects /data/object/id and must return None when null
    assert_eq!(
        wrapper.order_id(),
        None,
        "order_id() must be None when /data/object/id is null"
    );
    // customer_id() extracts /data/object/customer successfully
    assert_eq!(wrapper.customer_id(), Some("cus_upcoming_merchant_456"));
    // Not a checkout session completed
    assert!(!wrapper.is_checkout_completed());
}
