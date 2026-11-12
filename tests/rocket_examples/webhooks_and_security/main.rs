//! Integration tests for Cryptographic Stripe Webhooks (`/webhooks/stripe`).
//!
//! Scenarios covered:
//! 1. Missing or blank `Stripe-Signature` header rejection (`400 Bad Request`).
//! 2. Forged HMAC-SHA256 signature rejection (`400 Bad Request`).
//! 3. Replay protection: expired timestamp (`> 300s` in past) and excessive future drift (`> 60s` in future) rejected.
//! 4. Valid API V1 event routing across:
//!    - `checkout.session.completed` (fulfillment ready vs delayed payment awaiting settlement)
//!    - `payment_intent.succeeded`
//!    - `invoice.paid`
//!    - `customer.subscription.deleted`
//!    - unhandled event types (`handled: false`)
//! 5. Valid API V2 Thin Event detection, signature verification, and automatic live hydration.
//! 6. Native `"$STRIPE_WEBHOOK_SECRET"` `$VAR_NAME` reference resolution (`PR #150`).

#[path = "../common/mod.rs"]
mod common;

use common::{read_http_request, setup_mock_rocket_client, write_json_response};
use rocket::http::{ContentType, Header, Status};
use serde_json::{Value, json};
use some_stripe::Webhook;

#[tokio::test]
async fn test_webhook_security_guards_missing_forged_and_replay_rejected() {
    let secret = "whsec_rocket_integration_secret_12345";
    let (_listener, client) = setup_mock_rocket_client(secret).await;
    let now = chrono::Utc::now().timestamp();

    let payload = json!({
        "id": "evt_sec_test_1",
        "object": "event",
        "created": now,
        "type": "payment_intent.succeeded",
        "data": { "object": { "id": "pi_sec_1" } }
    })
    .to_string();

    // 1. Missing Stripe-Signature header -> 400 Bad Request
    let missing_res = client
        .post("/webhooks/stripe")
        .header(ContentType::JSON)
        .body(&payload)
        .dispatch()
        .await;
    assert_eq!(missing_res.status(), Status::BadRequest);

    // 2. Empty Stripe-Signature header -> 400 Bad Request
    let empty_sig_res = client
        .post("/webhooks/stripe")
        .header(ContentType::JSON)
        .header(Header::new("Stripe-Signature", "   "))
        .body(&payload)
        .dispatch()
        .await;
    assert_eq!(empty_sig_res.status(), Status::BadRequest);

    // 3. Forged signature -> 400 Bad Request
    let forged_header = format!(
        "t={now},v1=0000000000000000000000000000000000000000000000000000000000000000"
    );
    let forged_res = client
        .post("/webhooks/stripe")
        .header(ContentType::JSON)
        .header(Header::new("Stripe-Signature", forged_header))
        .body(&payload)
        .dispatch()
        .await;
    assert_eq!(forged_res.status(), Status::BadRequest);

    // 4. Replay attack (> 300s old timestamp) -> 400 Bad Request
    let expired_header =
        Webhook::generate_test_header_string(payload.as_bytes(), secret, Some(now - 360));
    let expired_res = client
        .post("/webhooks/stripe")
        .header(ContentType::JSON)
        .header(Header::new("Stripe-Signature", expired_header))
        .body(&payload)
        .dispatch()
        .await;
    assert_eq!(expired_res.status(), Status::BadRequest);

    // 5. Future drift (> 60s in the future) -> 400 Bad Request
    let future_header =
        Webhook::generate_test_header_string(payload.as_bytes(), secret, Some(now + 120));
    let future_res = client
        .post("/webhooks/stripe")
        .header(ContentType::JSON)
        .header(Header::new("Stripe-Signature", future_header))
        .body(&payload)
        .dispatch()
        .await;
    assert_eq!(future_res.status(), Status::BadRequest);
}

#[tokio::test]
async fn test_webhook_v1_event_routing_all_variants() {
    let secret = "whsec_rocket_v1_routing_secret";
    let (_listener, client) = setup_mock_rocket_client(secret).await;
    let now = chrono::Utc::now().timestamp();

    let send_signed_event = |payload_str: String| {
        let sig = Webhook::generate_test_header_string(payload_str.as_bytes(), secret, Some(now));
        client
            .post("/webhooks/stripe")
            .header(ContentType::JSON)
            .header(Header::new("Stripe-Signature", sig))
            .body(payload_str)
    };

    // 1. checkout.session.completed (paid -> fulfillment ready)
    let cs_paid = json!({
        "id": "evt_cs_paid",
        "object": "event",
        "created": now,
        "type": "checkout.session.completed",
        "data": {
            "object": {
                "id": "cs_order_900",
                "status": "complete",
                "payment_status": "paid",
                "amount_total": 9900,
                "currency": "usd"
            }
        }
    })
    .to_string();
    let r1 = send_signed_event(cs_paid).dispatch().await;
    assert_eq!(r1.status(), Status::Ok);
    let j1: Value = r1.into_json().await.unwrap();
    assert_eq!(j1["data"]["handled"], true);
    assert_eq!(j1["data"]["reference_id"], "cs_order_900");
    assert!(
        j1["data"]["action_taken"]
            .as_str()
            .unwrap()
            .contains("Dispatched order fulfillment")
    );

    // 2. checkout.session.completed (unpaid async payment -> awaiting settlement)
    let cs_unpaid = json!({
        "id": "evt_cs_unpaid",
        "object": "event",
        "created": now,
        "type": "checkout.session.completed",
        "data": {
            "object": {
                "id": "cs_order_901",
                "status": "complete",
                "payment_status": "unpaid",
                "amount_total": 9900,
                "currency": "usd"
            }
        }
    })
    .to_string();
    let r2 = send_signed_event(cs_unpaid).dispatch().await;
    assert_eq!(r2.status(), Status::Ok);
    let j2: Value = r2.into_json().await.unwrap();
    assert!(
        j2["data"]["action_taken"]
            .as_str()
            .unwrap()
            .contains("Awaiting settlement")
    );

    // 3. payment_intent.succeeded
    let pi_succeeded = json!({
        "id": "evt_pi_ok",
        "object": "event",
        "created": now,
        "type": "payment_intent.succeeded",
        "data": {
            "object": {
                "id": "pi_settled_1",
                "payment_intent": "pi_settled_1"
            }
        }
    })
    .to_string();
    let r3 = send_signed_event(pi_succeeded).dispatch().await;
    assert_eq!(r3.status(), Status::Ok);
    let j3: Value = r3.into_json().await.unwrap();
    assert_eq!(j3["data"]["event_type"], "payment_intent.succeeded");
    assert_eq!(j3["data"]["handled"], true);

    // 4. invoice.paid
    let inv_paid = json!({
        "id": "evt_inv_paid",
        "object": "event",
        "created": now,
        "type": "invoice.paid",
        "data": {
            "object": {
                "id": "in_100",
                "customer": "cus_sub_100"
            }
        }
    })
    .to_string();
    let r4 = send_signed_event(inv_paid).dispatch().await;
    assert_eq!(r4.status(), Status::Ok);
    let j4: Value = r4.into_json().await.unwrap();
    assert_eq!(j4["data"]["reference_id"], "cus_sub_100");

    // 5. customer.subscription.deleted
    let sub_del = json!({
        "id": "evt_sub_del",
        "object": "event",
        "created": now,
        "type": "customer.subscription.deleted",
        "data": {
            "object": {
                "id": "sub_100",
                "customer": "cus_sub_100"
            }
        }
    })
    .to_string();
    let r5 = send_signed_event(sub_del).dispatch().await;
    assert_eq!(r5.status(), Status::Ok);
    let j5: Value = r5.into_json().await.unwrap();
    assert!(
        j5["data"]["action_taken"]
            .as_str()
            .unwrap()
            .contains("Revoked platform entitlement")
    );

    // 6. Unhandled event type acknowledged gracefully
    let unhandled = json!({
        "id": "evt_other_1",
        "object": "event",
        "created": now,
        "type": "Radar.early_fraud_warning.created",
        "data": { "object": { "id": "issfr_1" } }
    })
    .to_string();
    let r6 = send_signed_event(unhandled).dispatch().await;
    assert_eq!(r6.status(), Status::Ok);
    let j6: Value = r6.into_json().await.unwrap();
    assert_eq!(j6["data"]["handled"], false);
}

#[tokio::test]
async fn test_webhook_v2_thin_event_verification_and_hydration() {
    let secret = "whsec_rocket_v2_thin_secret";
    let (listener, client) = setup_mock_rocket_client(secret).await;
    let now = chrono::Utc::now().timestamp();

    let mock_server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let (headers, _) = read_http_request(&mut socket).await;
        assert!(headers.starts_with("GET /v2/core/events/evt_v2_webhook_thin_1 HTTP/1.1"));
        write_json_response(
            &mut socket,
            "200 OK",
            &json!({
                "id": "evt_v2_webhook_thin_1",
                "object": "v2.core.event",
                "type": "v1.billing.meter.error_report_triggered",
                "created": now,
                "livemode": false,
                "related_object": {
                    "id": "mtr_webhook_777",
                    "type": "billing.meter",
                    "url": "https://api.stripe.com/v2/billing/meters/mtr_webhook_777"
                },
                "data": { "summary": "Live hydrated payload" }
            })
            .to_string(),
        )
        .await;
    });

    let thin_payload = json!({
        "id": "evt_v2_webhook_thin_1",
        "object": "v2.core.event_notification",
        "type": "v1.billing.meter.error_report_triggered",
        "created": now,
        "livemode": false,
        "related_object": {
            "id": "mtr_webhook_777",
            "type": "billing.meter",
            "url": "https://api.stripe.com/v2/billing/meters/mtr_webhook_777"
        }
    })
    .to_string();

    let sig = Webhook::generate_test_header_string(thin_payload.as_bytes(), secret, Some(now));
    let res = client
        .post("/webhooks/stripe")
        .header(ContentType::JSON)
        .header(Header::new("Stripe-Signature", sig))
        .body(thin_payload)
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let body: Value = res.into_json().await.unwrap();
    assert_eq!(body["data"]["event_id"], "evt_v2_webhook_thin_1");
    assert_eq!(body["data"]["handled"], true);
    assert_eq!(body["data"]["reference_id"], "mtr_webhook_777");

    mock_server.await.unwrap();
}

#[tokio::test]
async fn test_webhook_dollar_env_secret_reference_resolution() {
    // Pass "$STRIPE_WEBHOOK_SECRET" reference directly to setup_mock_rocket_client
    // and Webhook::generate_test_header_string (PR #150 automatic resolution)
    let (_listener, client) = setup_mock_rocket_client("$STRIPE_WEBHOOK_SECRET").await;
    let now = chrono::Utc::now().timestamp();

    let payload = json!({
        "id": "evt_dollar_env_test_1",
        "object": "event",
        "created": now,
        "type": "payment_intent.succeeded",
        "data": {
            "object": {
                "id": "pi_dollar_env_1",
                "payment_intent": "pi_dollar_env_1"
            }
        }
    })
    .to_string();

    let sig = Webhook::generate_test_header_string(
        payload.as_bytes(),
        "$STRIPE_WEBHOOK_SECRET",
        Some(now),
    );
    let res = client
        .post("/webhooks/stripe")
        .header(ContentType::JSON)
        .header(Header::new("Stripe-Signature", sig))
        .body(payload)
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let body: Value = res.into_json().await.unwrap();
    assert_eq!(body["data"]["event_id"], "evt_dollar_env_test_1");
    assert_eq!(body["data"]["handled"], true);
}
