//! Integration Test Suite for Example 1: `actix_core_and_webhooks`
//!
//! Covers `.env` `STRIPE_SECRET_KEY` & `STRIPE_WEBHOOK_SECRET` loading, `/health` secret redaction,
//! dual-protocol 3-state `Nullable<T>` serialization (`serialize_v1_form` & `serialize_v2_json`),
//! constant-time v1 webhook signature verification, replay drift rejection, missing header handling,
//! and v2 thin event probing.

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/actix_core_and_webhooks.rs"]
mod example_app;

use actix_web::{App, http::StatusCode, test, web};
use common::DotEnvConfig;
use example_app::{DualProtocolDemoInput, WebhookAppState, build_configured_client, configure_routes};
use some_stripe::Webhook;

#[actix_web::test]
async fn scenario_1_dotenv_loading_and_health_secret_redaction() {
    let env = DotEnvConfig::load();
    let state = web::Data::new(WebhookAppState {
        stripe: build_configured_client(env.stripe_secret_key.as_str(), Some("http://127.0.0.1:9")),
        webhook_secret: env.stripe_webhook_secret.clone(),
    });

    let app = test::init_service(
        App::new()
            .app_data(state)
            .configure(configure_routes),
    )
    .await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "actix_core_and_webhooks");
    // Verify Gate G2: secret key is strictly redacted in Debug output
    let debug_str = body["secret_debug_redacted"].as_str().unwrap_or_default();
    assert!(debug_str.contains("REDACTED"));
    assert!(!debug_str.contains(env.stripe_webhook_secret.as_str()));
}

#[actix_web::test]
async fn scenario_2_dual_protocol_nullable_value_and_null_encoding() {
    let env = DotEnvConfig::load();
    let state = web::Data::new(WebhookAppState {
        stripe: env.client_for_mock("http://127.0.0.1:9"),
        webhook_secret: env.stripe_webhook_secret,
    });

    let app = test::init_service(App::new().app_data(state).configure(configure_routes)).await;

    let payload = DualProtocolDemoInput {
        email: Some("merchant@example.com".to_string()),
        clear_description: true,
        stripe_account: Some("acct_connected_123".to_string()),
        idempotency_key: Some("idem_key_abc".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/core/dual-protocol-preview")
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(
        body["v1_form_urlencoded"],
        "email=merchant%40example.com&description="
    );
    assert_eq!(
        body["v2_json_payload"],
        r#"{"email":"merchant@example.com","description":null}"#
    );
    assert_eq!(body["stripe_account"], "acct_connected_123");
    assert_eq!(body["idempotency_key"], "idem_key_abc");
}

#[actix_web::test]
async fn scenario_3_dual_protocol_nullable_omit_encoding() {
    let env = DotEnvConfig::load();
    let state = web::Data::new(WebhookAppState {
        stripe: env.client_for_mock("http://127.0.0.1:9"),
        webhook_secret: env.stripe_webhook_secret,
    });

    let app = test::init_service(App::new().app_data(state).configure(configure_routes)).await;

    let payload = DualProtocolDemoInput {
        email: None,
        clear_description: false,
        stripe_account: None,
        idempotency_key: None,
    };

    let req = test::TestRequest::post()
        .uri("/core/dual-protocol-preview")
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["v1_form_urlencoded"], "");
    assert_eq!(body["v2_json_payload"], "{}");
}

#[actix_web::test]
async fn scenario_4_v1_webhook_valid_signature_and_event_object_dispatch() {
    let env = DotEnvConfig::load();
    let secret_str = env.stripe_webhook_secret.as_str().to_string();
    let state = web::Data::new(WebhookAppState {
        stripe: env.client_for_mock("http://127.0.0.1:9"),
        webhook_secret: env.stripe_webhook_secret,
    });

    let app = test::init_service(App::new().app_data(state).configure(configure_routes)).await;

    let raw_payload = br#"{"id":"evt_checkout_001","object":"event","created":1728345600,"type":"checkout.session.completed","data":{"object":{"id":"cs_test_001","object":"checkout.session","status":"complete","payment_status":"paid"}}}"#;
    let now = chrono::Utc::now().timestamp();
    let sig_header = Webhook::generate_test_header_string(raw_payload, &secret_str, Some(now));

    let req = test::TestRequest::post()
        .uri("/webhooks/v1")
        .insert_header(("Stripe-Signature", sig_header))
        .set_payload(raw_payload.as_slice())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["received"], true);
    assert_eq!(body["event_id"], "evt_checkout_001");
    assert_eq!(body["is_checkout_completed"], true);
    assert_eq!(body["object_variant"], "checkout.session");
}

#[actix_web::test]
async fn scenario_5_v1_webhook_rejects_tampered_signature_and_missing_header() {
    let env = DotEnvConfig::load();
    let state = web::Data::new(WebhookAppState {
        stripe: env.client_for_mock("http://127.0.0.1:9"),
        webhook_secret: env.stripe_webhook_secret,
    });

    let app = test::init_service(App::new().app_data(state).configure(configure_routes)).await;

    let raw_payload = br#"{"id":"evt_tampered","object":"event","type":"payment_intent.succeeded","data":{"object":{"id":"pi_1","object":"payment_intent","amount":1000,"currency":"usd","status":"succeeded"}}}"#;
    let now = chrono::Utc::now().timestamp();
    let forged_header = format!("t={now},v1=0000000000000000000000000000000000000000000000000000000000000000");

    let bad_req = test::TestRequest::post()
        .uri("/webhooks/v1")
        .insert_header(("Stripe-Signature", forged_header))
        .set_payload(raw_payload.as_slice())
        .to_request();
    let bad_resp = test::call_service(&app, bad_req).await;
    assert_eq!(bad_resp.status(), StatusCode::BAD_REQUEST);

    // Missing header scenario
    let missing_req = test::TestRequest::post()
        .uri("/webhooks/v1")
        .set_payload(raw_payload.as_slice())
        .to_request();
    let missing_resp = test::call_service(&app, missing_req).await;
    assert_eq!(missing_resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn scenario_6_v1_webhook_rejects_stale_replay_timestamp() {
    let env = DotEnvConfig::load();
    let secret_str = env.stripe_webhook_secret.as_str().to_string();
    let state = web::Data::new(WebhookAppState {
        stripe: env.client_for_mock("http://127.0.0.1:9"),
        webhook_secret: env.stripe_webhook_secret,
    });

    let app = test::init_service(App::new().app_data(state).configure(configure_routes)).await;

    let raw_payload = br#"{"id":"evt_stale","object":"event","type":"checkout.session.completed","data":{"object":{"id":"cs_stale","object":"checkout.session"}}}"#;
    let stale_ts = chrono::Utc::now().timestamp() - 1000;
    let stale_header = Webhook::generate_test_header_string(raw_payload, &secret_str, Some(stale_ts));

    let req = test::TestRequest::post()
        .uri("/webhooks/v1")
        .insert_header(("Stripe-Signature", stale_header))
        .set_payload(raw_payload.as_slice())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn scenario_7_v2_thin_event_webhook_verification() {
    let env = DotEnvConfig::load();
    let secret_str = env.stripe_webhook_secret.as_str().to_string();
    let state = web::Data::new(WebhookAppState {
        stripe: env.client_for_mock("http://127.0.0.1:9"),
        webhook_secret: env.stripe_webhook_secret,
    });

    let app = test::init_service(App::new().app_data(state).configure(configure_routes)).await;

    let thin_payload = br#"{"id":"evt_v2_thin_001","object":"v2.core.event","type":"v1.billing.meter.error_report_triggered","created":"2026-11-10T12:00:00Z","related_object":{"id":"mtr_123","type":"billing.meter","url":"/v2/billing/meters/mtr_123"}}"#;
    let now = chrono::Utc::now().timestamp();
    let sig_header = Webhook::generate_test_header_string(thin_payload, &secret_str, Some(now));

    let req = test::TestRequest::post()
        .uri("/webhooks/v2-thin")
        .insert_header(("Stripe-Signature", sig_header))
        .set_payload(thin_payload.as_slice())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["received"], true);
    assert_eq!(body["thin_detected"], true);
    assert_eq!(body["event_id"], "evt_v2_thin_001");
    assert_eq!(body["related_object_id"], "mtr_123");
}
