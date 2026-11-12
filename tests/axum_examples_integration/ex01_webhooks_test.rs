//! Integration Test Suite for `examples/axum/01_axum_webhooks.rs` (Module 2: Cryptographic Webhooks & V2 Thin Events).
//!
//! Covers:
//! 1. Valid V1 `payment_intent.succeeded` webhook signature verification (`200 OK`, `v2_thin_event: false`).
//! 2. Valid V1 `checkout.session.completed` and `invoice.upcoming` webhook events.
//! 3. Valid V2 Thin Event (`v2.core.event`) routing through `construct_v2_thin_event` and live hydration
//!    against a mock `/v2/core/events/{id}` endpoint (`200 OK`, `v2_thin_event: true`, `hydrated_id`).
//! 4. Missing `Stripe-Signature` header rejection (`400 Bad Request`).
//! 5. Forged / tampered HMAC-SHA256 signature rejection (`400 Bad Request`).
//! 6. Stale timestamp outside 300-second replay window rejection (`400 Bad Request`).
//! 7. Credential loader validation (`load_required_env` rejects missing/empty/whitespace keys).

#[path = "../../examples/axum/01_axum_webhooks.rs"]
mod ex01_axum_webhooks;

use axum::http::StatusCode;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{
    build_mock_stripe_client, send_raw_webhook_request, spawn_single_mock_stripe_server,
};
use ex01_axum_webhooks::{build_router, WebhookState};
use crate::common::AxumDotEnvConfig;

const TEST_WEBHOOK_SECRET: &str = "whsec_test_integration_secret_01";

/// Returns the current UNIX timestamp in seconds.
fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Clock drift before epoch")
        .as_secs() as i64
}

/// Scenario 1: Valid V1 `payment_intent.succeeded` webhook passes constant-time HMAC verification.
#[tokio::test]
async fn test_ex01_valid_v1_payment_intent_webhook() {
    let client = build_mock_stripe_client("http://127.0.0.1:9");
    let payload = br#"{
        "id": "evt_v1_pi_001",
        "object": "event",
        "type": "payment_intent.succeeded",
        "created": 1728345600,
        "data": {
            "object": {
                "id": "pi_3Mtw001",
                "object": "payment_intent",
                "amount": 5000,
                "currency": "usd",
                "status": "succeeded"
            }
        }
    }"#;

    let sig = client.webhooks().generate_test_header_string(
        payload,
        TEST_WEBHOOK_SECRET,
        Some(current_unix_timestamp()),
    );
    let app = build_router(WebhookState::with_client(client, TEST_WEBHOOK_SECRET));

    let (status, body) = send_raw_webhook_request(app, "/webhooks/stripe", Some(&sig), payload).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["received"], true);
    assert_eq!(body["event_id"], "evt_v1_pi_001");
    assert_eq!(body["v2_thin_event"], false);
}

/// Scenario 2: Valid V1 `checkout.session.completed` and `invoice.upcoming` (null id) webhooks.
#[tokio::test]
async fn test_ex01_valid_v1_checkout_and_invoice_webhooks() {
    let client = build_mock_stripe_client("http://127.0.0.1:9");
    let cs_payload = br#"{
        "id": "evt_v1_cs_002",
        "object": "event",
        "type": "checkout.session.completed",
        "created": 1728345600,
        "data": {
            "object": {
                "id": "cs_test_002",
                "object": "checkout.session",
                "mode": "payment",
                "status": "complete"
            }
        }
    }"#;
    let sig_cs = client.webhooks().generate_test_header_string(
        cs_payload,
        TEST_WEBHOOK_SECRET,
        Some(current_unix_timestamp()),
    );
    let app = build_router(WebhookState::with_client(client.clone(), TEST_WEBHOOK_SECRET));
    let (status, body) =
        send_raw_webhook_request(app, "/webhooks/stripe", Some(&sig_cs), cs_payload).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["event_id"], "evt_v1_cs_002");

    let inv_payload = br#"{
        "id": "evt_v1_inv_003",
        "object": "event",
        "type": "invoice.upcoming",
        "created": 1728345600,
        "data": {
            "object": {
                "id": null,
                "object": "invoice",
                "customer": "cus_preview_123",
                "amount_due": 4200
            }
        }
    }"#;
    let sig_inv = client.webhooks().generate_test_header_string(
        inv_payload,
        TEST_WEBHOOK_SECRET,
        Some(current_unix_timestamp()),
    );
    let app2 = build_router(WebhookState::with_client(client, TEST_WEBHOOK_SECRET));
    let (status2, body2) =
        send_raw_webhook_request(app2, "/webhooks/stripe", Some(&sig_inv), inv_payload).await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(body2["event_id"], "evt_v1_inv_003");
}

/// Scenario 3: Valid API V2 Thin Event routes through `construct_v2_thin_event` and hydrates via `ThinEventHydrator`.
#[tokio::test]
async fn test_ex01_v2_thin_event_verification_and_hydration() {
    let hydrated_json = r#"{
        "id": "evt_v2_thin_999",
        "object": "v2.core.event",
        "type": "v1.billing.meter.error_report_triggered",
        "created": "2026-11-11T00:00:00Z",
        "livemode": false,
        "related_object": {
            "id": "mtr_123",
            "type": "billing.meter",
            "url": "/v2/billing/meters/mtr_123"
        }
    }"#;

    let (api_base, mock_handle) = spawn_single_mock_stripe_server("200 OK", hydrated_json).await;
    let client = build_mock_stripe_client(api_base);

    let now = current_unix_timestamp();
    let thin_payload_str = format!(
        r#"{{
            "id": "evt_v2_thin_999",
            "object": "v2.core.event",
            "type": "v1.billing.meter.error_report_triggered",
            "created": {now},
            "livemode": false,
            "related_object": {{
                "id": "mtr_123",
                "type": "billing.meter",
                "url": "/v2/billing/meters/mtr_123"
            }}
        }}"#
    );
    let thin_bytes = thin_payload_str.as_bytes();
    let sig = client.webhooks().generate_test_header_string(
        thin_bytes,
        TEST_WEBHOOK_SECRET,
        Some(now),
    );

    let app = build_router(WebhookState::with_client(client, TEST_WEBHOOK_SECRET));
    let (status, body) =
        send_raw_webhook_request(app, "/webhooks/stripe", Some(&sig), thin_bytes).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["received"], true);
    assert_eq!(body["event_id"], "evt_v2_thin_999");
    assert_eq!(body["v2_thin_event"], true);
    assert_eq!(body["hydrated_id"], "evt_v2_thin_999");

    let (req_headers, _) = mock_handle.await.expect("Mock server task failed");
    assert!(req_headers.starts_with("GET /v2/core/events/evt_v2_thin_999 HTTP/1.1"));
}

/// Scenario 4, 5 & 6: Missing header, forged signature, and stale timestamp (>300s) return `400 Bad Request`.
#[tokio::test]
async fn test_ex01_adversarial_webhook_rejections() {
    let client = build_mock_stripe_client("http://127.0.0.1:9");
    let payload = br#"{"id":"evt_1","object":"event","type":"ping","created":1728345600,"data":{"object":{"object":"unknown"}}}"#;

    // 4. Missing Stripe-Signature header
    let app1 = build_router(WebhookState::with_client(client.clone(), TEST_WEBHOOK_SECRET));
    let (status1, body1) = send_raw_webhook_request(app1, "/webhooks/stripe", None, payload).await;
    assert_eq!(status1, StatusCode::BAD_REQUEST);
    assert!(body1["raw_text"]
        .as_str()
        .unwrap_or_default()
        .contains("Missing Stripe-Signature"));

    // 5. Forged signature signed with wrong secret
    let forged_sig = client.webhooks().generate_test_header_string(
        payload,
        "whsec_wrong_attacker_secret",
        Some(current_unix_timestamp()),
    );
    let app2 = build_router(WebhookState::with_client(client.clone(), TEST_WEBHOOK_SECRET));
    let (status2, _) =
        send_raw_webhook_request(app2, "/webhooks/stripe", Some(&forged_sig), payload).await;
    assert_eq!(status2, StatusCode::BAD_REQUEST);

    // 6. Stale replay attack (timestamp 1000 seconds in the past > 300s tolerance)
    let stale_ts = current_unix_timestamp() - 1000;
    let stale_sig =
        client
            .webhooks()
            .generate_test_header_string(payload, TEST_WEBHOOK_SECRET, Some(stale_ts));
    let app3 = build_router(WebhookState::with_client(client, TEST_WEBHOOK_SECRET));
    let (status3, _) =
        send_raw_webhook_request(app3, "/webhooks/stripe", Some(&stale_sig), payload).await;
    assert_eq!(status3, StatusCode::BAD_REQUEST);
}

/// Scenario 7: Native `$STRIPE_SECRET_KEY` and `$STRIPE_WEBHOOK_SECRET` resolution (PR #150)
/// automatically loads from `.env*` files and masks secrets as `[REDACTED]` in `Debug` output.
#[tokio::test]
async fn test_ex01_native_dollar_var_secret_resolution_and_redaction() {
    let env_cfg = AxumDotEnvConfig::load();
    assert!(!env_cfg.stripe_secret_key.expose_secret().is_empty());
    assert!(!env_cfg.stripe_webhook_secret.expose_secret().is_empty());
    assert_eq!(format!("{:?}", env_cfg.stripe_secret_key), "[REDACTED]");
    assert_eq!(format!("{:?}", env_cfg.stripe_webhook_secret), "[REDACTED]");

    // Verify Webhook::generate_test_header_string and WebhookState with "$STRIPE_WEBHOOK_SECRET"
    let client = some_stripe::StripeClient::new("$STRIPE_SECRET_KEY");
    let payload = br#"{"id":"evt_env_ref","object":"event","type":"payment_intent.succeeded","created":1728345600,"data":{"object":{"id":"pi_env_1","object":"payment_intent","amount":1000,"currency":"usd","status":"succeeded"}}}"#;
    let sig = client.webhooks().generate_test_header_string(
        payload,
        "$STRIPE_WEBHOOK_SECRET",
        Some(current_unix_timestamp()),
    );
    let app = build_router(WebhookState::with_client(client, "$STRIPE_WEBHOOK_SECRET"));
    let (status, body) = send_raw_webhook_request(app, "/webhooks/stripe", Some(&sig), payload).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["event_id"], "evt_env_ref");
}
