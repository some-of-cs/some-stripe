//! Offline `wiremock` conformance test suite for [`StripeProvider`] (`US-9.2.0` Unit 4).
//!
//! Verifies `AC-1`, `AC-2`, `AC-3`, and `TC-9.2.4` completely offline without network access:
//! - Provider identity and orthogonal [`PaymentCapabilities`] flags
//! - Fail-closed local currency guardrails (`KWD` / `ISK`) prior to HTTP dispatch
//! - Pinned `Stripe-Version` and `Idempotency-Key` header propagation and `NextAction` redaction
//! - Hosted Checkout creation and CheckoutSession `/expire` void routing
//! - PaymentIntent lifecycle (`get_payment`, partial `capture`, `/cancel` void, `refund`)
//! - Multi-`v1` webhook rotation, `v0` downgrade rejection, 300s replay window, raw-byte
//!   integrity, thin-event hydration detection, and payload secret scrubbing
//! - Redacted `Debug` output on [`StripeProvider`] and [`StripeProviderConfig`]

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use some_payment_core::{
    CaptureMode, CaptureRequest, CheckoutSessionRequest, CreatePaymentRequest, ErrorKind,
    HttpMethod, Money, NextAction, PaymentCapabilities, PaymentError, PaymentProvider,
    PaymentStatus, ProviderId, ProviderReference, RawWebhookRequest, RefundRequest, RefundStatus,
    VoidRequest,
};
use some_stripe_webhooks::crypto::compute_streaming_hmac_bytes;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::adapter::provider::{StripeProvider, StripeProviderConfig};

const TEST_API_KEY: &str = "sk_test_super_secret";
const TEST_WEBHOOK_SECRET: &str = "whsec_super_secret";
const PINNED_STRIPE_VERSION: &str = "2026-11-18.endive";

struct PaymentIntentFixture<'a> {
    id: &'a str,
    amount: u64,
    amount_received: u64,
    currency: &'a str,
    status: &'a str,
    client_secret: &'a str,
}

fn make_config(base_url: String) -> StripeProviderConfig {
    StripeProviderConfig::new(TEST_API_KEY)
        .with_api_base(base_url)
        .with_webhook_secret(TEST_WEBHOOK_SECRET)
        .with_stripe_version(PINNED_STRIPE_VERSION)
}

fn make_provider(base_url: String) -> StripeProvider {
    StripeProvider::new(make_config(base_url)).expect("valid StripeProviderConfig")
}

fn pi_reference(intent_id: &str) -> ProviderReference {
    ProviderReference::builder(ProviderId::new("stripe"))
        .intent_id(intent_id)
        .build()
        .expect("valid payment_intent ProviderReference")
}

fn assert_error_kind<T>(result: Result<T, PaymentError>, expected: ErrorKind) {
    match result {
        Err(err) => assert_eq!(err.kind(), expected, "unexpected error: {err:?}"),
        Ok(_) => panic!("expected Err with kind {expected:?}, got Ok"),
    }
}

fn json_ok(body: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(body)
}

fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX_EPOCH")
        .as_secs()
}

fn sign_payload(secret: &str, timestamp: u64, payload: &[u8]) -> String {
    let ts_text = timestamp.to_string();
    compute_streaming_hmac_bytes(secret.as_bytes(), ts_text.as_bytes(), payload)
        .expect("HMAC-SHA256 computation must succeed")
}

fn build_webhook_request(payload: &[u8], sig_header: String) -> RawWebhookRequest {
    let mut headers = BTreeMap::new();
    headers.insert("Stripe-Signature".to_owned(), sig_header);
    RawWebhookRequest {
        raw_body: payload.to_vec(),
        headers,
        query_params: BTreeMap::new(),
        method: HttpMethod::Post,
    }
}

fn snapshot_event_value() -> serde_json::Value {
    serde_json::json!({
        "id": "evt_snap_1",
        "object": "event",
        "type": "payment_intent.succeeded",
        "livemode": false,
        "data": {
            "object": {
                "id": "pi_wh_1",
                "object": "payment_intent",
                "amount": 2_500,
                "currency": "usd",
                "status": "succeeded",
                "client_secret": "pi_wh_1_secret_leaked",
                "api_key": "sk_live_should_redact"
            }
        }
    })
}

fn thin_event_value() -> serde_json::Value {
    serde_json::json!({
        "id": "evt_thin_1",
        "object": "v2.core.event",
        "type": "v2.billing.meter_event_session.created",
        "livemode": false,
        "related_object": {
            "id": "pi_thin_1",
            "type": "payment_intent",
            "url": "/v1/payment_intents/pi_thin_1"
        }
    })
}

fn payment_intent_json(pi: PaymentIntentFixture<'_>) -> serde_json::Value {
    serde_json::json!({
        "id": pi.id,
        "object": "payment_intent",
        "amount": pi.amount,
        "amount_received": pi.amount_received,
        "currency": pi.currency,
        "status": pi.status,
        "client_secret": pi.client_secret,
        "livemode": false
    })
}

fn sample_checkout_request() -> CheckoutSessionRequest {
    CheckoutSessionRequest {
        provider_id: Some(ProviderId::new("stripe")),
        money: Money::new(4_200, "USD").expect("money"),
        order_id: "ord-cs-1".to_owned(),
        success_url: "https://merchant.example/success".to_owned(),
        cancel_url: Some("https://merchant.example/cancel".to_owned()),
        description: Some("Conformance checkout".to_owned()),
        customer_email: None,
        customer_phone: None,
        capture_mode: CaptureMode::Automatic,
        idempotency_key: Some("idem-cs-1".to_owned()),
        metadata: BTreeMap::new(),
    }
}

async fn mount_kwd_accept_mock(server: &MockServer) {
    let body = payment_intent_json(PaymentIntentFixture {
        id: "pi_kwd_1050",
        amount: 1_050,
        amount_received: 0,
        currency: "kwd",
        status: "requires_payment_method",
        client_secret: "pi_kwd_1050_secret_ok",
    });
    Mock::given(method("POST"))
        .and(path("/v1/payment_intents"))
        .respond_with(json_ok(body))
        .expect(1)
        .mount(server)
        .await;
}

async fn mount_checkout_mocks(server: &MockServer, hosted_url: &str) {
    let created_body = serde_json::json!({
        "id": "cs_test_1",
        "object": "checkout.session",
        "status": "open",
        "payment_status": "unpaid",
        "amount_total": 4_200,
        "currency": "usd",
        "url": hosted_url,
        "livemode": false
    });
    Mock::given(method("POST"))
        .and(path("/v1/checkout/sessions"))
        .and(header("Stripe-Version", PINNED_STRIPE_VERSION))
        .respond_with(json_ok(created_body))
        .expect(1)
        .mount(server)
        .await;

    let expired_body = serde_json::json!({
        "id": "cs_test_1",
        "object": "checkout.session",
        "status": "expired",
        "payment_status": "unpaid",
        "amount_total": 4_200,
        "currency": "usd",
        "livemode": false
    });
    Mock::given(method("POST"))
        .and(path("/v1/checkout/sessions/cs_test_1/expire"))
        .respond_with(json_ok(expired_body))
        .expect(1)
        .mount(server)
        .await;
}

async fn mount_lifecycle_mocks(server: &MockServer) {
    let auth_body = payment_intent_json(PaymentIntentFixture {
        id: "pi_life_1",
        amount: 10_000,
        amount_received: 0,
        currency: "usd",
        status: "requires_capture",
        client_secret: "pi_life_1_secret_auth",
    });
    Mock::given(method("GET"))
        .and(path("/v1/payment_intents/pi_life_1"))
        .and(header("Stripe-Version", PINNED_STRIPE_VERSION))
        .respond_with(json_ok(auth_body))
        .expect(1)
        .mount(server)
        .await;

    let captured_body = payment_intent_json(PaymentIntentFixture {
        id: "pi_life_1",
        amount: 10_000,
        amount_received: 6_000,
        currency: "usd",
        status: "succeeded",
        client_secret: "pi_life_1_secret_auth",
    });
    Mock::given(method("POST"))
        .and(path("/v1/payment_intents/pi_life_1/capture"))
        .and(header("Idempotency-Key", "idem-cap-1"))
        .respond_with(json_ok(captured_body))
        .expect(1)
        .mount(server)
        .await;

    let canceled_body = payment_intent_json(PaymentIntentFixture {
        id: "pi_void_1",
        amount: 10_000,
        amount_received: 0,
        currency: "usd",
        status: "canceled",
        client_secret: "pi_void_1_secret_void",
    });
    Mock::given(method("POST"))
        .and(path("/v1/payment_intents/pi_void_1/cancel"))
        .and(header("Idempotency-Key", "idem-void-1"))
        .respond_with(json_ok(canceled_body))
        .expect(1)
        .mount(server)
        .await;

    mount_refund_mocks(server).await;
}

async fn mount_refund_mocks(server: &MockServer) {
    let action_body = serde_json::json!({
        "id": "re_action_1",
        "object": "refund",
        "amount": 2_500,
        "currency": "usd",
        "payment_intent": "pi_life_1",
        "status": "requires_action"
    });
    Mock::given(method("POST"))
        .and(path("/v1/refunds"))
        .and(header("Idempotency-Key", "idem-ref-action"))
        .respond_with(json_ok(action_body))
        .expect(1)
        .mount(server)
        .await;

    let ok_body = serde_json::json!({
        "id": "re_ok_1",
        "object": "refund",
        "amount": 3_500,
        "currency": "usd",
        "payment_intent": "pi_life_1",
        "status": "succeeded"
    });
    Mock::given(method("POST"))
        .and(path("/v1/refunds"))
        .and(header("Idempotency-Key", "idem-ref-ok"))
        .respond_with(json_ok(ok_body))
        .expect(1)
        .mount(server)
        .await;
}

async fn assert_refund_lifecycle(provider: &StripeProvider) {
    let action_refund = provider
        .refund(&RefundRequest {
            reference: pi_reference("pi_life_1"),
            amount: Some(Money::new(2_500, "USD").expect("money")),
            reason: Some("requested_by_customer".to_owned()),
            idempotency_key: Some("idem-ref-action".to_owned()),
        })
        .await
        .expect("requires_action refund should succeed");
    assert_eq!(action_refund.refund_id, "re_action_1");
    assert_eq!(action_refund.status, RefundStatus::RequiresAction);

    let ok_refund = provider
        .refund(&RefundRequest {
            reference: pi_reference("pi_life_1"),
            amount: Some(Money::new(3_500, "USD").expect("money")),
            reason: None,
            idempotency_key: Some("idem-ref-ok".to_owned()),
        })
        .await
        .expect("succeeded refund should succeed");
    assert_eq!(ok_refund.refund_id, "re_ok_1");
    assert_eq!(ok_refund.status, RefundStatus::Succeeded);
}

async fn assert_rejected_webhooks(
    provider: &StripeProvider,
    now: u64,
    snapshot: &serde_json::Value,
) {
    let compact = serde_json::to_vec(snapshot).expect("serialize snapshot");
    let pretty = serde_json::to_vec_pretty(snapshot).expect("pretty snapshot");
    let valid_v1 = sign_payload(TEST_WEBHOOK_SECRET, now, &compact);

    let v0_only = build_webhook_request(&compact, format!("t={now},v0={valid_v1}"));
    assert_error_kind(
        provider.parse_and_verify_webhook(&v0_only).await,
        ErrorKind::WebhookVerificationFailed,
    );

    let stale_ts = now.saturating_sub(301);
    let stale_v1 = sign_payload(TEST_WEBHOOK_SECRET, stale_ts, &compact);
    let stale_req = build_webhook_request(&compact, format!("t={stale_ts},v1={stale_v1}"));
    assert_error_kind(
        provider.parse_and_verify_webhook(&stale_req).await,
        ErrorKind::WebhookVerificationFailed,
    );

    let altered_req = build_webhook_request(&pretty, format!("t={now},v1={valid_v1}"));
    assert_error_kind(
        provider.parse_and_verify_webhook(&altered_req).await,
        ErrorKind::WebhookVerificationFailed,
    );
}

async fn assert_thin_event_webhook(provider: &StripeProvider, now: u64) {
    let thin_bytes = serde_json::to_vec(&thin_event_value()).expect("serialize thin event");
    let thin_v1 = sign_payload(TEST_WEBHOOK_SECRET, now, &thin_bytes);
    let thin_req = build_webhook_request(&thin_bytes, format!("t={now},v1={thin_v1}"));
    let thin_event = provider
        .parse_and_verify_webhook(&thin_req)
        .await
        .expect("thin v2.core.event must verify");
    assert!(thin_event.requires_hydration);
    assert_eq!(thin_event.event_id, "evt_thin_1");
}

#[test]
fn test_capabilities_and_provider_id() {
    let provider = make_provider("https://api.stripe.com".to_owned());
    assert_eq!(provider.id().as_str(), "stripe");

    let caps = provider.capabilities();
    let expected = PaymentCapabilities::EMBEDDED_INTENT
        | PaymentCapabilities::HOSTED_CHECKOUT
        | PaymentCapabilities::AUTH_CAPTURE
        | PaymentCapabilities::PARTIAL_CAPTURE
        | PaymentCapabilities::VOID_AUTHORIZED
        | PaymentCapabilities::REFUND
        | PaymentCapabilities::PARTIAL_REFUND
        | PaymentCapabilities::NATIVE_CREATE_IDEMPOTENCY
        | PaymentCapabilities::NATIVE_MUTATION_IDEMPOTENCY
        | PaymentCapabilities::SAVED_CARDS;

    assert!(caps.contains(expected), "missing expected flags: {caps:?}");
    assert!(!caps.contains(PaymentCapabilities::SPLITS));
    assert!(!caps.contains(PaymentCapabilities::WEBHOOK_VERIFICATION_HANDSHAKE));
}

#[tokio::test]
async fn test_currency_guardrails_preflight_rejection() {
    let reject_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/payment_intents"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&reject_server)
        .await;

    let reject_provider = make_provider(reject_server.uri());
    let bad_kwd = CreatePaymentRequest::new(Money::new(1_055, "KWD").expect("money"), "ord-kwd");
    assert_error_kind(
        reject_provider.create_payment(&bad_kwd).await,
        ErrorKind::Validation,
    );

    let bad_isk = CreatePaymentRequest::new(Money::new(1_050, "ISK").expect("money"), "ord-isk");
    assert_error_kind(
        reject_provider.create_payment(&bad_isk).await,
        ErrorKind::Validation,
    );

    let accept_server = MockServer::start().await;
    mount_kwd_accept_mock(&accept_server).await;

    let accept_provider = make_provider(accept_server.uri());
    let good_kwd = CreatePaymentRequest::new(Money::new(1_050, "KWD").expect("money"), "ord-ok");
    let res = accept_provider
        .create_payment(&good_kwd)
        .await
        .expect("KWD 1050 should pass currency guardrails");
    assert_eq!(res.money.amount(), 1_050);
    assert_eq!(res.money.currency().as_str(), "KWD");
}

#[tokio::test]
async fn test_create_payment_sends_version_and_idempotency_headers() {
    let server = MockServer::start().await;
    let secret = "pi_test_1_secret_sensitive_token";
    let body = payment_intent_json(PaymentIntentFixture {
        id: "pi_test_1",
        amount: 5_000,
        amount_received: 0,
        currency: "usd",
        status: "requires_payment_method",
        client_secret: secret,
    });

    Mock::given(method("POST"))
        .and(path("/v1/payment_intents"))
        .and(header("Stripe-Version", PINNED_STRIPE_VERSION))
        .and(header("Idempotency-Key", "idem-order-1"))
        .respond_with(json_ok(body))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(server.uri());
    let req = CreatePaymentRequest::new(Money::new(5_000, "USD").expect("money"), "order-1")
        .with_capture_mode(CaptureMode::Manual)
        .with_idempotency_key("idem-order-1");

    let res = provider
        .create_payment(&req)
        .await
        .expect("create_payment should succeed");
    assert_eq!(res.reference.intent_id(), Some("pi_test_1"));

    let action = res.next_action.as_ref().expect("expected NextAction");
    assert!(matches!(
        action,
        NextAction::EmbeddedClientSecret { client_secret } if client_secret == secret
    ));

    let debug_output = format!("{action:?}");
    assert!(debug_output.contains("[REDACTED]"));
    assert!(!debug_output.contains(secret));
}

#[tokio::test]
async fn test_create_checkout_and_expire_void() {
    let server = MockServer::start().await;
    let hosted_url = "https://checkout.stripe.com/c/pay/cs_test_1";
    mount_checkout_mocks(&server, hosted_url).await;

    let provider = make_provider(server.uri());
    let created = provider
        .create_checkout(&sample_checkout_request())
        .await
        .expect("create_checkout should succeed");
    assert_eq!(created.checkout_url.as_deref(), Some(hosted_url));
    assert_eq!(created.reference.checkout_session_id(), Some("cs_test_1"));

    let void_req = VoidRequest {
        reference: created.reference,
        reason: Some("abandoned".to_owned()),
        idempotency_key: Some("idem-cs-expire-1".to_owned()),
    };
    let expired = provider
        .void_payment(&void_req)
        .await
        .expect("void_payment on checkout session should route to /expire");
    assert!(matches!(
        expired.status,
        PaymentStatus::Canceled | PaymentStatus::Voided
    ));
}

#[tokio::test]
async fn test_get_capture_void_refund_lifecycle() {
    let server = MockServer::start().await;
    mount_lifecycle_mocks(&server).await;

    let provider = make_provider(server.uri());
    let fetched = provider
        .get_payment(&pi_reference("pi_life_1"))
        .await
        .expect("get_payment should succeed");
    assert_eq!(fetched.status, PaymentStatus::Authorized);

    let captured = provider
        .capture(&CaptureRequest {
            reference: pi_reference("pi_life_1"),
            amount: Some(Money::new(6_000, "USD").expect("money")),
            idempotency_key: Some("idem-cap-1".to_owned()),
        })
        .await
        .expect("partial capture should succeed");
    assert_eq!(captured.captured_amount, Some(6_000));
    assert!(matches!(
        captured.status,
        PaymentStatus::PartiallyCaptured | PaymentStatus::Captured | PaymentStatus::Paid
    ));

    let voided = provider
        .void_payment(&VoidRequest {
            reference: pi_reference("pi_void_1"),
            reason: Some("requested_by_customer".to_owned()),
            idempotency_key: Some("idem-void-1".to_owned()),
        })
        .await
        .expect("void_payment on PaymentIntent should route to /cancel");
    assert!(matches!(
        voided.status,
        PaymentStatus::Canceled | PaymentStatus::Voided
    ));

    assert_refund_lifecycle(&provider).await;
}

#[tokio::test]
async fn test_webhook_dual_v1_rotation_v0_downgrade_stale_and_thin_event() {
    let provider = make_provider("https://api.stripe.com".to_owned());
    let now = current_unix_secs();

    let snapshot_val = snapshot_event_value();
    let compact = serde_json::to_vec(&snapshot_val).expect("serialize snapshot");
    let valid_v1 = sign_payload(TEST_WEBHOOK_SECRET, now, &compact);
    let old_v1 = sign_payload("whsec_previous_rotated_secret", now, &compact);
    let rotated_header = format!(
        "t={now},v0=deadbeef0000,secret=ignored,destination=ignored,v1={old_v1},v1={valid_v1}"
    );

    let verified = provider
        .parse_and_verify_webhook(&build_webhook_request(&compact, rotated_header))
        .await
        .expect("dual-v1 rotated header with v0 and debug entries must pass");
    assert!(!verified.requires_hydration);
    assert_eq!(verified.event_id, "evt_snap_1");

    let redacted = verified.raw_redacted.to_string();
    assert!(redacted.contains("[REDACTED]"));
    assert!(!redacted.contains("pi_wh_1_secret_leaked"));
    assert!(!redacted.contains("sk_live_should_redact"));

    assert_rejected_webhooks(&provider, now, &snapshot_val).await;
    assert_thin_event_webhook(&provider, now).await;
}

#[test]
fn test_secret_redaction_in_debug() {
    let config = make_config("https://api.stripe.com".to_owned());
    let config_dbg = format!("{config:?}");
    assert!(config_dbg.contains("[REDACTED]"));
    assert!(!config_dbg.contains(TEST_API_KEY));
    assert!(!config_dbg.contains(TEST_WEBHOOK_SECRET));

    let provider = StripeProvider::new(config).expect("valid StripeProviderConfig");
    let provider_dbg = format!("{provider:?}");
    assert!(provider_dbg.contains("[REDACTED]"));
    assert!(!provider_dbg.contains(TEST_API_KEY));
    assert!(!provider_dbg.contains(TEST_WEBHOOK_SECRET));
}

#[tokio::test]
async fn test_void_payment_checkout_session_with_pi_enforces_tm6() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/payment_intents/pi_cs_open"))
        .respond_with(json_ok(serde_json::json!({
            "id": "pi_cs_open",
            "amount": 3000,
            "currency": "usd",
            "status": "requires_payment_method",
            "livemode": false
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/checkout/sessions/cs_void_open/expire"))
        .respond_with(json_ok(serde_json::json!({
            "id": "cs_void_open",
            "amount_total": 3000,
            "currency": "usd",
            "status": "expired",
            "payment_status": "unpaid",
            "payment_intent": "pi_cs_open",
            "livemode": false
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/payment_intents/pi_cs_open/cancel"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let provider = make_provider(server.uri());
    let dual_ref = ProviderReference::builder(ProviderId::new("stripe"))
        .checkout_session_id("cs_void_open")
        .payment_id("pi_cs_open")
        .build()
        .expect("valid dual reference");
    let res = provider
        .void_payment(&VoidRequest::new(dual_ref))
        .await
        .expect("should expire checkout session when PI is not requires_capture");
    assert_eq!(res.status, PaymentStatus::Canceled);
    assert_eq!(res.reference.checkout_session_id(), Some("cs_void_open"));

    Mock::given(method("GET"))
        .and(path("/v1/payment_intents/pi_cs_auth"))
        .respond_with(json_ok(serde_json::json!({
            "id": "pi_cs_auth",
            "amount": 3000,
            "currency": "usd",
            "status": "requires_capture",
            "livemode": false
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/payment_intents/pi_cs_auth/cancel"))
        .respond_with(json_ok(serde_json::json!({
            "id": "pi_cs_auth",
            "amount": 3000,
            "currency": "usd",
            "status": "canceled",
            "livemode": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let auth_ref = ProviderReference::builder(ProviderId::new("stripe"))
        .checkout_session_id("cs_void_auth")
        .payment_id("pi_cs_auth")
        .build()
        .expect("valid dual auth reference");
    let canceled = provider
        .void_payment(&VoidRequest::new(auth_ref))
        .await
        .expect("should cancel PI when status is requires_capture");
    assert_eq!(canceled.status, PaymentStatus::Canceled);
}

#[tokio::test]
async fn test_http_429_preserves_retry_after_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/payment_intents"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "42")
                .set_body_json(serde_json::json!({
                    "error": {
                        "type": "rate_limit_error",
                        "message": "Too many requests"
                    }
                })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(server.uri());
    let req = CreatePaymentRequest::new(Money::new(1_000, "USD").expect("money"), "ord-429");
    let err = provider
        .create_payment(&req)
        .await
        .expect_err("HTTP 429 must map to RateLimited");
    assert!(
        matches!(
            err,
            PaymentError::RateLimited {
                retry_after_secs: Some(42),
                ..
            }
        ),
        "expected RateLimited with retry_after_secs=Some(42), got {err:?}"
    );
}

#[tokio::test]
async fn test_idempotency_key_preflight_rejects_oversized_and_non_ascii() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/payment_intents"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let provider = make_provider(server.uri());
    for bad_key in ["k".repeat(256), "idem-non-ascii-é".to_owned()] {
        let mut req = CreatePaymentRequest::new(Money::new(1_000, "USD").expect("money"), "ord-ik");
        req.idempotency_key = Some(bad_key);
        assert_error_kind(provider.create_payment(&req).await, ErrorKind::Validation);
    }
}
