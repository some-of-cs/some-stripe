//! Integration Test Suite for Example 8: `actix_v2_next_gen_services`
//!
//! Covers all 30 Actix-Web routes across Stripe API v2 Core (`events`, `hydrator`,
//! `event_destinations`, `accounts` & tokens) and API v2 Billing (`meters`,
//! `meter_event_session`, `meter_event_stream`, `meter_event_adjustments`, `meter_events`,
//! and `MeterIngestionEngine`).

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/actix_web/actix_v2_next_gen_services.rs"]
mod example_app;

use actix_web::{App, http::StatusCode, test, web};
use common::{DotEnvConfig, MockStripeServer, MockWireResponse};
use example_app::{V2AppState, configure_routes};
use some_stripe::{
    AggregationFormula, CreateBillingMeterParams, CreateEventDestinationParams,
    CreateMeterEventAdjustmentParams, CreateMeterEventParams, CreateMeterEventSessionParams,
    CreateMeterEventStreamParams, CreateV2AccountParams, CreateV2AccountTokenParams,
    CustomerMappingSettings, EventDestinationType, MeterEventPayload, UpdateBillingMeterParams,
    UpdateEventDestinationParams, UpdateV2AccountParams, ValueSettings,
};

#[actix_web::test]
async fn scenario_1_v2_core_events_hydrator_destinations_and_accounts() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(200, r#"{"id":"evt_v2_001","object":"v2.core.event","type":"v1.billing.meter.error_report_triggered","created":"2026-11-11T00:00:00Z"}"#),
        MockWireResponse::json(200, r#"{"data":[{"id":"evt_v2_001","object":"v2.core.event","type":"v1.billing.meter.error_report_triggered","created":"2026-11-11T00:00:00Z"}]}"#),
        MockWireResponse::json(200, r#"{"id":"evt_v2_hydrated","object":"v2.core.event","type":"v1.billing.meter.error_report_triggered","created":"2026-11-11T00:00:00Z"}"#),
        MockWireResponse::json(200, r#"{"id":"ed_001","object":"v2.core.event_destination","name":"Prod Webhook","status":"enabled"}"#),
        MockWireResponse::json(200, r#"{"id":"ed_001","object":"v2.core.event_destination","name":"Prod Webhook","status":"enabled"}"#),
        MockWireResponse::json(200, r#"{"id":"ed_001","object":"v2.core.event_destination","name":"Prod Webhook","status":"enabled"}"#),
        MockWireResponse::json(200, r#"{"id":"ed_001","object":"v2.core.event_destination","name":"Prod Webhook","status":"disabled"}"#),
        MockWireResponse::json(200, r#"{"data":[{"id":"ed_001","object":"v2.core.event_destination","name":"Prod Webhook","status":"enabled"}]}"#),
        MockWireResponse::json(200, r#"{"id":"ed_001","object":"v2.core.event_destination","name":"Prod Webhook","status":"enabled"}"#),
        MockWireResponse::json(200, r#"{"id":"ed_001","object":"v2.core.event_destination","name":"Prod Webhook","status":"disabled"}"#),
        MockWireResponse::json(200, r#"{"id":"ed_001","object":"v2.core.event_destination","name":"Prod Webhook","status":"enabled"}"#),
        MockWireResponse::json(200, r#"{"id":"acct_v2_001","object":"v2.core.account","display_name":"Acme V2"}"#),
        MockWireResponse::json(200, r#"{"id":"acct_v2_001","object":"v2.core.account","display_name":"Acme V2"}"#),
        MockWireResponse::json(200, r#"{"id":"acct_v2_001","object":"v2.core.account","display_name":"Acme V2 Updated"}"#),
        MockWireResponse::json(200, r#"{"id":"acct_v2_001","object":"v2.core.account","display_name":"Acme V2"}"#),
        MockWireResponse::json(200, r#"{"data":[{"id":"acct_v2_001","object":"v2.core.account","display_name":"Acme V2"}]}"#),
        MockWireResponse::json(200, r#"{"object":"v2.core.account_token","token":"st_v2_secret_token_999","expires_at":1795000000}"#),
    ])
    .await;

    let state = web::Data::new(V2AppState {
        stripe: env.client_for_mock(&mock.base_url),
        ingestion_engine: env.ingestion_engine_for_mock(&mock.base_url),
    });
    let app = test::init_service(App::new().app_data(state).configure(configure_routes)).await;

    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/v2/core/events/evt_v2_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/v2/core/events").to_request()).await.status(), StatusCode::OK);

    // Hydrate a fresh thin event notification
    let now_rfc3339 = chrono::Utc::now().to_rfc3339();
    let thin_json = format!(
        r#"{{"id":"evt_v2_hydrated","object":"v2.core.event","type":"v1.billing.meter.error_report_triggered","created":"{now_rfc3339}"}}"#
    );
    assert_eq!(
        test::call_service(&app, test::TestRequest::post().uri("/v2/core/hydrate").set_payload(thin_json).to_request()).await.status(),
        StatusCode::OK
    );

    let dest_create = CreateEventDestinationParams::new(
        EventDestinationType::WebhookEndpoint,
        vec!["v1.billing.meter.error_report_triggered".to_string()],
    );
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/v2/core/event-destinations").set_json(&dest_create).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/v2/core/event-destinations/ed_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/v2/core/event-destinations/ed_001").set_json(&UpdateEventDestinationParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::delete().uri("/v2/core/event-destinations/ed_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/v2/core/event-destinations").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/v2/core/event-destinations/ed_001/enable").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/v2/core/event-destinations/ed_001/disable").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/v2/core/event-destinations/ed_001/ping").to_request()).await.status(), StatusCode::OK);

    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/v2/core/accounts").set_json(&CreateV2AccountParams::new()).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/v2/core/accounts/acct_v2_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/v2/core/accounts/acct_v2_001").set_json(&UpdateV2AccountParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::delete().uri("/v2/core/accounts/acct_v2_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/v2/core/accounts").to_request()).await.status(), StatusCode::OK);

    let tok_resp = test::call_service(&app, test::TestRequest::post().uri("/v2/core/accounts/acct_v2_001/tokens").set_json(&CreateV2AccountTokenParams::new()).to_request()).await;
    assert_eq!(tok_resp.status(), StatusCode::CREATED);
    let tok_body: serde_json::Value = test::read_body_json(tok_resp).await;
    assert!(tok_body["token_debug_redacted"].as_str().unwrap_or_default().contains("REDACTED"));

    assert_eq!(mock.finish().await.len(), 17);
}

#[actix_web::test]
async fn scenario_2_v2_billing_meters_sessions_streams_adjustments_and_ingestion_engine() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(200, r#"{"id":"mtr_001","object":"billing.meter","display_name":"LLM Tokens","event_name":"llm_tokens","status":"active"}"#),
        MockWireResponse::json(200, r#"{"id":"mtr_001","object":"billing.meter","display_name":"LLM Tokens","event_name":"llm_tokens","status":"active"}"#),
        MockWireResponse::json(200, r#"{"id":"mtr_001","object":"billing.meter","display_name":"LLM Tokens v2","event_name":"llm_tokens","status":"active"}"#),
        MockWireResponse::json(200, r#"{"id":"mtr_001","object":"billing.meter","display_name":"LLM Tokens v2","event_name":"llm_tokens","status":"inactive"}"#),
        MockWireResponse::json(200, r#"{"id":"mtr_001","object":"billing.meter","display_name":"LLM Tokens v2","event_name":"llm_tokens","status":"active"}"#),
        MockWireResponse::json(200, r#"{"data":[{"id":"mtr_001","object":"billing.meter","display_name":"LLM Tokens v2","event_name":"llm_tokens","status":"active"}]}"#),
        MockWireResponse::json(200, r#"{"id":"mes_001","object":"billing.meter_event_session","authentication_token":"mes_secret_bearer_token","created":1728000000,"expires_at":1999999999}"#),
        MockWireResponse::json(200, r#"{"object":"v2.billing.meter_event","event_name":"llm_tokens","identifier":"evt_sync_1"}"#),
        MockWireResponse::json(202, r#"{"status":"accepted","events_ingested":1}"#),
        MockWireResponse::json(200, r#"{"id":"mes_engine","object":"billing.meter_event_session","authentication_token":"mes_engine_bearer","created":1728000000,"expires_at":1999999999}"#),
        MockWireResponse::json(202, r#"{"status":"accepted","events_ingested":1}"#),
        MockWireResponse::json(200, r#"{"id":"mea_001","object":"billing.meter_event_adjustment","event_name":"llm_tokens","status":"complete","type":"cancel"}"#),
        MockWireResponse::json(200, r#"{"id":"mea_001","object":"billing.meter_event_adjustment","event_name":"llm_tokens","status":"complete","type":"cancel"}"#),
        MockWireResponse::json(200, r#"{"data":[{"id":"mea_001","object":"billing.meter_event_adjustment","event_name":"llm_tokens","status":"complete","type":"cancel"}]}"#),
    ])
    .await;

    let state = web::Data::new(V2AppState {
        stripe: env.client_for_mock(&mock.base_url),
        ingestion_engine: env.ingestion_engine_for_mock(&mock.base_url),
    });
    let app = test::init_service(App::new().app_data(state).configure(configure_routes)).await;

    let create_meter = CreateBillingMeterParams::new(
        "LLM Tokens",
        "llm_tokens",
        AggregationFormula::Sum,
        ValueSettings::new("value"),
        CustomerMappingSettings::by_id("stripe_customer_id"),
    );
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/v2/billing/meters").set_json(&create_meter).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/v2/billing/meters/mtr_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/v2/billing/meters/mtr_001").set_json(&UpdateBillingMeterParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/v2/billing/meters/mtr_001/deactivate").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/v2/billing/meters/mtr_001/reactivate").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/v2/billing/meters").to_request()).await.status(), StatusCode::OK);

    let sess_resp = test::call_service(&app, test::TestRequest::post().uri("/v2/billing/meter-event-sessions").set_json(&CreateMeterEventSessionParams::new()).to_request()).await;
    assert_eq!(sess_resp.status(), StatusCode::CREATED);
    let sess_body: serde_json::Value = test::read_body_json(sess_resp).await;
    assert!(sess_body["token_redacted"].as_str().unwrap_or_default().contains("REDACTED"));

    let sync_evt = CreateMeterEventParams::new("llm_tokens", "cus_001", "1500");
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/v2/billing/meter-events").set_json(&sync_evt).to_request()).await.status(), StatusCode::CREATED);

    let stream_batch = CreateMeterEventStreamParams::new(vec![MeterEventPayload::new("llm_tokens", "cus_001", "2500")]);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/v2/billing/meter-event-stream").set_json(&stream_batch).to_request()).await.status(), StatusCode::ACCEPTED);

    let engine_evt = MeterEventPayload::new("llm_tokens", "cus_001", "500").with_identifier("evt_dedup_001");
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/v2/billing/ingestion-engine/enqueue").set_json(&engine_evt).to_request()).await.status(), StatusCode::ACCEPTED);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/v2/billing/ingestion-engine/flush").to_request()).await.status(), StatusCode::OK);

    let adj_create = CreateMeterEventAdjustmentParams::new("llm_tokens", "cancel");
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/v2/billing/meter-event-adjustments").set_json(&adj_create).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/v2/billing/meter-event-adjustments/mea_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/v2/billing/meter-event-adjustments").to_request()).await.status(), StatusCode::OK);

    assert_eq!(mock.finish().await.len(), 14);
}
