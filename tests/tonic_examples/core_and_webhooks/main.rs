//! Integration Test Suite for Example 1: `tonic_core_and_webhooks`
//!
//! Covers native `.env` `"$STRIPE_SECRET_KEY"` & `"$STRIPE_WEBHOOK_SECRET"` resolution,
//! `get_health` secret redaction (`[REDACTED]`), `preview_dual_protocol` 3-state `Nullable<T>`
//! serialization (`serialize_v1_form` & `serialize_v2_json`), constant-time HMAC-SHA256
//! `verify_v1_webhook`, tampered signature rejection (`Code::Unauthenticated`), expired
//! timestamp replay guard (`Code::Unauthenticated`), >1 MiB payload guard, and
//! `verify_v2_thin_webhook`.

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/tonic_core_and_webhooks.rs"]
mod example_grpc;

use common::{DotEnvConfig, sign_webhook_payload};
use example_grpc::{
    CoreWebhooksGrpcServer, CoreWebhooksGrpcService, DualProtocolPreviewRequest,
    HealthCheckRequest, WebhookVerifyRequest, build_configured_client,
};
use some_stripe::{MAX_WEBHOOK_PAYLOAD_BYTES, SecretKey};
use tonic::{Code, Request};

/// Verifies `.env` `"$STRIPE_SECRET_KEY"` & `"$STRIPE_WEBHOOK_SECRET"` resolution and
/// `[REDACTED]` secret formatting in `get_health`.
#[tokio::test]
async fn scenario_1_dotenv_loading_and_get_health_secret_redaction() {
    let env = DotEnvConfig::load();
    let raw_webhook_secret = env.stripe_webhook_secret.as_str().to_string();
    let server = CoreWebhooksGrpcServer {
        stripe: build_configured_client(env.stripe_secret_key.as_str(), Some("http://127.0.0.1:9")),
        webhook_secret: env.stripe_webhook_secret,
    };

    let resp = server
        .get_health(Request::new(HealthCheckRequest {}))
        .await
        .expect("get_health RPC should succeed")
        .into_inner();

    assert_eq!(resp.status, "SERVING");
    assert_eq!(resp.service, "tonic_core_and_webhooks");
    assert_eq!(resp.api_base, "http://127.0.0.1:9");
    assert!(resp.secret_debug_redacted.contains("REDACTED"));
    assert!(!resp.secret_debug_redacted.contains(&raw_webhook_secret));

    let env_server = CoreWebhooksGrpcServer::from_env();
    let env_health = env_server
        .get_health(Request::new(HealthCheckRequest {}))
        .await
        .expect("from_env get_health should succeed")
        .into_inner();
    assert_eq!(env_health.status, "SERVING");
    assert!(env_health.secret_debug_redacted.contains("REDACTED"));
}

/// Verifies `preview_dual_protocol` with `Nullable::Value`, `Nullable::Null`, and `RequestOptions`.
#[tokio::test]
async fn scenario_2_preview_dual_protocol_nullable_value_and_null_encoding() {
    let env = DotEnvConfig::load();
    let server = CoreWebhooksGrpcServer {
        stripe: env.client_for_mock("http://127.0.0.1:9"),
        webhook_secret: env.stripe_webhook_secret,
    };

    let resp = server
        .preview_dual_protocol(Request::new(DualProtocolPreviewRequest {
            email: Some("merchant@example.com".to_string()),
            clear_description: true,
            stripe_account: Some("acct_connected_123".to_string()),
            idempotency_key: Some("idem_key_abc".to_string()),
        }))
        .await
        .expect("preview_dual_protocol should succeed")
        .into_inner();

    assert_eq!(
        resp.v1_form_urlencoded,
        "email=merchant%40example.com&description="
    );
    assert_eq!(
        resp.v2_json_payload,
        r#"{"email":"merchant@example.com","description":null}"#
    );
    assert_eq!(resp.stripe_account, "acct_connected_123");
    assert_eq!(resp.idempotency_key, "idem_key_abc");
    assert_eq!(
        resp.max_webhook_payload_bytes,
        MAX_WEBHOOK_PAYLOAD_BYTES as u64
    );
}

/// Verifies `preview_dual_protocol` when both fields are `Nullable::Omit`.
#[tokio::test]
async fn scenario_3_preview_dual_protocol_nullable_omit_encoding() {
    let env = DotEnvConfig::load();
    let server = CoreWebhooksGrpcServer {
        stripe: env.client_for_mock("http://127.0.0.1:9"),
        webhook_secret: env.stripe_webhook_secret,
    };

    let resp = server
        .preview_dual_protocol(Request::new(DualProtocolPreviewRequest {
            email: None,
            clear_description: false,
            stripe_account: None,
            idempotency_key: None,
        }))
        .await
        .expect("preview_dual_protocol omit should succeed")
        .into_inner();

    assert_eq!(resp.v1_form_urlencoded, "");
    assert_eq!(resp.v2_json_payload, "{}");
    assert_eq!(resp.stripe_account, "");
    assert_eq!(resp.idempotency_key, "");
}

/// Verifies `verify_v1_webhook` with valid HMAC-SHA256 signatures via both message field and
/// gRPC `stripe-signature` metadata.
#[tokio::test]
async fn scenario_4_verify_v1_webhook_valid_hmac_signature_and_metadata() {
    let env = DotEnvConfig::load();
    let secret_str = env.stripe_webhook_secret.as_str().to_string();
    let server = CoreWebhooksGrpcServer {
        stripe: env.client_for_mock("http://127.0.0.1:9"),
        webhook_secret: env.stripe_webhook_secret,
    };

    let raw_payload = br#"{"id":"evt_checkout_001","object":"event","created":1728345600,"type":"checkout.session.completed","data":{"object":{"id":"cs_test_001","object":"checkout.session","status":"complete","payment_status":"paid"}}}"#;
    let now = chrono::Utc::now().timestamp();
    let sig_header = sign_webhook_payload(raw_payload, &secret_str, now);

    // 1. Signature passed via request message field
    let resp = server
        .verify_v1_webhook(Request::new(WebhookVerifyRequest {
            payload: raw_payload.to_vec(),
            signature_header: sig_header,
        }))
        .await
        .expect("verify_v1_webhook should succeed")
        .into_inner();

    assert!(resp.verified);
    assert_eq!(resp.event_id, "evt_checkout_001");
    assert_eq!(resp.event_type, "checkout.session.completed");
    assert!(resp.is_checkout_completed);
    assert_eq!(resp.object_variant, "checkout.session");

    // 2. Signature passed via gRPC MetadataMap (`stripe-signature`)
    let meta_sig = sign_webhook_payload(raw_payload, &secret_str, now);
    let mut grpc_req = Request::new(WebhookVerifyRequest {
        payload: raw_payload.to_vec(),
        signature_header: String::new(),
    });
    grpc_req.metadata_mut().insert(
        "stripe-signature",
        meta_sig.parse().expect("valid ascii metadata value"),
    );
    let meta_resp = server
        .verify_v1_webhook(grpc_req)
        .await
        .expect("verify_v1_webhook via gRPC metadata should succeed")
        .into_inner();
    assert!(meta_resp.verified);
    assert_eq!(meta_resp.event_id, "evt_checkout_001");
}

/// Verifies `verify_v1_webhook` rejects tampered signatures and missing signature headers
/// with `Code::Unauthenticated`.
#[tokio::test]
async fn scenario_5_verify_v1_webhook_rejects_tampered_and_missing_signatures() {
    let env = DotEnvConfig::load();
    let server = CoreWebhooksGrpcServer {
        stripe: env.client_for_mock("http://127.0.0.1:9"),
        webhook_secret: env.stripe_webhook_secret,
    };

    let raw_payload = br#"{"id":"evt_tampered","object":"event","type":"payment_intent.succeeded","data":{"object":{"id":"pi_1","object":"payment_intent","amount":1000,"currency":"usd","status":"succeeded"}}}"#;
    let now = chrono::Utc::now().timestamp();
    let forged_header = format!(
        "t={now},v1=0000000000000000000000000000000000000000000000000000000000000000"
    );

    let err = server
        .verify_v1_webhook(Request::new(WebhookVerifyRequest {
            payload: raw_payload.to_vec(),
            signature_header: forged_header,
        }))
        .await
        .expect_err("tampered signature must fail");
    assert_eq!(err.code(), Code::Unauthenticated);

    let missing_err = server
        .verify_v1_webhook(Request::new(WebhookVerifyRequest {
            payload: raw_payload.to_vec(),
            signature_header: String::new(),
        }))
        .await
        .expect_err("missing signature header must fail");
    assert_eq!(missing_err.code(), Code::Unauthenticated);
}

/// Verifies `verify_v1_webhook` rejects stale timestamps outside the 300s replay window
/// with `Code::Unauthenticated`, and rejects payloads exceeding 1 MiB (`MAX_WEBHOOK_PAYLOAD_BYTES`).
#[tokio::test]
async fn scenario_6_verify_v1_webhook_replay_guard_and_1mib_payload_guard() {
    let secret_str = "whsec_tonic_replay_and_size_guard_secret";
    let env = DotEnvConfig::load();
    let server = CoreWebhooksGrpcServer {
        stripe: env.client_for_mock("http://127.0.0.1:9"),
        webhook_secret: SecretKey::new(secret_str),
    };

    let raw_payload = br#"{"id":"evt_stale","object":"event","type":"checkout.session.completed","data":{"object":{"id":"cs_stale","object":"checkout.session"}}}"#;
    let stale_ts = chrono::Utc::now().timestamp() - 1_000;
    let stale_sig = sign_webhook_payload(raw_payload, secret_str, stale_ts);

    let stale_err = server
        .verify_v1_webhook(Request::new(WebhookVerifyRequest {
            payload: raw_payload.to_vec(),
            signature_header: stale_sig,
        }))
        .await
        .expect_err("expired replay timestamp must fail");
    assert_eq!(stale_err.code(), Code::Unauthenticated);

    // >1 MiB payload DoS guard
    let oversized_payload = vec![b'x'; MAX_WEBHOOK_PAYLOAD_BYTES + 1];
    let now = chrono::Utc::now().timestamp();
    let oversized_err = server
        .verify_v1_webhook(Request::new(WebhookVerifyRequest {
            payload: oversized_payload,
            signature_header: format!("t={now},v1=deadbeef"),
        }))
        .await
        .expect_err("payload > 1 MiB must be rejected");
    assert!(matches!(
        oversized_err.code(),
        Code::InvalidArgument | Code::ResourceExhausted
    ));
}

/// Verifies `verify_v2_thin_webhook` on lightweight Stripe API v2 `v2.core.event` payloads.
#[tokio::test]
async fn scenario_7_verify_v2_thin_webhook_event() {
    let env = DotEnvConfig::load();
    let secret_str = env.stripe_webhook_secret.as_str().to_string();
    let server = CoreWebhooksGrpcServer {
        stripe: env.client_for_mock("http://127.0.0.1:9"),
        webhook_secret: env.stripe_webhook_secret,
    };

    let thin_payload = br#"{"id":"evt_v2_thin_001","object":"v2.core.event","type":"v1.billing.meter.error_report_triggered","created":"2026-11-10T12:00:00Z","related_object":{"id":"mtr_123","type":"billing.meter","url":"/v2/billing/meters/mtr_123"}}"#;
    let now = chrono::Utc::now().timestamp();
    let sig_header = sign_webhook_payload(thin_payload, &secret_str, now);

    let resp = server
        .verify_v2_thin_webhook(Request::new(WebhookVerifyRequest {
            payload: thin_payload.to_vec(),
            signature_header: sig_header,
        }))
        .await
        .expect("verify_v2_thin_webhook should succeed")
        .into_inner();

    assert!(resp.verified);
    assert!(resp.thin_detected);
    assert_eq!(resp.event_id, "evt_v2_thin_001");
    assert_eq!(resp.event_type, "v1.billing.meter.error_report_triggered");
    assert_eq!(resp.related_object_id, "mtr_123");
}
