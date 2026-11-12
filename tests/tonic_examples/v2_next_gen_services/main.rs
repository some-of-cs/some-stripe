//! Integration Test Suite for Example 8: `tonic_v2_next_gen_services`
//!
//! Covers all 31 Tonic gRPC methods across Stripe API v2 Core (`events`, `hydrator`,
//! `event_destinations`, `accounts` & tokens) and API v2 Billing (`meters`,
//! `meter_event_session`, `meter_event_stream`, `meter_event_adjustments`, `meter_events`,
//! and `MeterIngestionEngine`).

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/tonic_v2_next_gen_services.rs"]
mod example_grpc;

use common::{DotEnvConfig, MockStripeServer, MockWireResponse};
use example_grpc::{V2NextGenGrpcServer, V2NextGenGrpcService, V2RpcRequest};
use some_stripe::{
    AggregationFormula, CreateBillingMeterParams, CreateEventDestinationParams,
    CreateMeterEventAdjustmentParams, CreateMeterEventParams, CreateMeterEventSessionParams,
    CreateMeterEventStreamParams, CreateV2AccountParams, CreateV2AccountTokenParams,
    CustomerMappingSettings, EventDestinationType, ListBillingMetersParams,
    ListMeterEventAdjustmentsParams, MeterEventPayload, UpdateBillingMeterParams,
    UpdateEventDestinationParams, UpdateV2AccountParams, ValueSettings,
};
use tonic::Request;

/// Verifies Stripe API v2 Core `events`, `hydrate_v2_thin_event`, `event_destinations`, and `accounts` RPCs.
#[tokio::test]
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

    let server = V2NextGenGrpcServer {
        stripe: env.client_for_mock(&mock.base_url),
        ingestion_engine: env.ingestion_engine_for_mock(&mock.base_url),
    };

    assert!(server.retrieve_v2_event(Request::new(V2RpcRequest {
        id: "evt_v2_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.list_v2_events(Request::new(V2RpcRequest {
        id: String::new(),
        json_params: String::new(),
    })).await.is_ok());

    let now_rfc3339 = chrono::Utc::now().to_rfc3339();
    let thin_json = format!(
        r#"{{"id":"evt_v2_hydrated","object":"v2.core.event","type":"v1.billing.meter.error_report_triggered","created":"{now_rfc3339}"}}"#
    );
    assert!(server.hydrate_v2_thin_event(Request::new(V2RpcRequest {
        id: String::new(),
        json_params: thin_json,
    })).await.is_ok());

    let dest_create = CreateEventDestinationParams::new(
        EventDestinationType::WebhookEndpoint,
        vec!["v1.billing.meter.error_report_triggered".to_string()],
    );
    assert!(server.create_event_destination(Request::new(V2RpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&dest_create).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_event_destination(Request::new(V2RpcRequest {
        id: "ed_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_event_destination(Request::new(V2RpcRequest {
        id: "ed_001".to_string(),
        json_params: serde_json::to_string(&UpdateEventDestinationParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.delete_event_destination(Request::new(V2RpcRequest {
        id: "ed_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.list_event_destinations(Request::new(V2RpcRequest {
        id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.enable_event_destination(Request::new(V2RpcRequest {
        id: "ed_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.disable_event_destination(Request::new(V2RpcRequest {
        id: "ed_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.ping_event_destination(Request::new(V2RpcRequest {
        id: "ed_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());

    assert!(server.create_v2_account(Request::new(V2RpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&CreateV2AccountParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_v2_account(Request::new(V2RpcRequest {
        id: "acct_v2_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_v2_account(Request::new(V2RpcRequest {
        id: "acct_v2_001".to_string(),
        json_params: serde_json::to_string(&UpdateV2AccountParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.delete_v2_account(Request::new(V2RpcRequest {
        id: "acct_v2_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.list_v2_accounts(Request::new(V2RpcRequest {
        id: String::new(),
        json_params: String::new(),
    })).await.is_ok());

    let tok_resp = server
        .create_v2_account_token(Request::new(V2RpcRequest {
            id: "acct_v2_001".to_string(),
            json_params: serde_json::to_string(&CreateV2AccountTokenParams::new()).expect("json"),
        }))
        .await
        .expect("create_v2_account_token")
        .into_inner();
    assert!(tok_resp.json_data.contains("REDACTED"));
    assert!(!tok_resp.json_data.contains("st_v2_secret_token_999"));

    assert_eq!(mock.finish().await.len(), 17);
}

/// Verifies Stripe API v2 Billing `meters`, `meter_event_session`, `meter_event_stream`,
/// `MeterIngestionEngine` (`enqueue_meter_event` & `flush_ingestion_engine`), and adjustments.
#[tokio::test]
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

    let server = V2NextGenGrpcServer {
        stripe: env.client_for_mock(&mock.base_url),
        ingestion_engine: env.ingestion_engine_for_mock(&mock.base_url),
    };

    let create_meter = CreateBillingMeterParams::new(
        "LLM Tokens",
        "llm_tokens",
        AggregationFormula::Sum,
        ValueSettings::new("value"),
        CustomerMappingSettings::by_id("stripe_customer_id"),
    );
    assert!(server.create_billing_meter(Request::new(V2RpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&create_meter).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_billing_meter(Request::new(V2RpcRequest {
        id: "mtr_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_billing_meter(Request::new(V2RpcRequest {
        id: "mtr_001".to_string(),
        json_params: serde_json::to_string(&UpdateBillingMeterParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.deactivate_billing_meter(Request::new(V2RpcRequest {
        id: "mtr_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.reactivate_billing_meter(Request::new(V2RpcRequest {
        id: "mtr_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.list_billing_meters(Request::new(V2RpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&ListBillingMetersParams::new()).expect("json"),
    })).await.is_ok());

    let sess_resp = server
        .create_meter_event_session(Request::new(V2RpcRequest {
            id: String::new(),
            json_params: serde_json::to_string(&CreateMeterEventSessionParams::new()).expect("json"),
        }))
        .await
        .expect("create_meter_event_session")
        .into_inner();
    assert!(sess_resp.json_data.contains("REDACTED"));
    assert!(!sess_resp.json_data.contains("mes_secret_bearer_token"));

    let sync_evt = CreateMeterEventParams::new("llm_tokens", "cus_001", "1500");
    assert!(server.create_synchronous_meter_event(Request::new(V2RpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&sync_evt).expect("json"),
    })).await.is_ok());

    let stream_batch = CreateMeterEventStreamParams::new(vec![MeterEventPayload::new("llm_tokens", "cus_001", "2500")]);
    assert!(server.create_meter_event_stream_batch(Request::new(V2RpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&stream_batch).expect("json"),
    })).await.is_ok());

    let engine_evt = MeterEventPayload::new("llm_tokens", "cus_001", "500").with_identifier("evt_dedup_001");
    let enqueue_resp = server
        .enqueue_meter_event(Request::new(V2RpcRequest {
            id: String::new(),
            json_params: serde_json::to_string(&engine_evt).expect("json"),
        }))
        .await
        .expect("enqueue_meter_event")
        .into_inner();
    assert!(enqueue_resp.json_data.contains(r#""pending_events":1"#));

    let flush_resp = server
        .flush_ingestion_engine(Request::new(V2RpcRequest {
            id: String::new(),
            json_params: String::new(),
        }))
        .await
        .expect("flush_ingestion_engine")
        .into_inner();
    assert!(flush_resp.json_data.contains(r#""flushed":1"#));

    let adj_create = CreateMeterEventAdjustmentParams::new("llm_tokens", "cancel");
    assert!(server.create_meter_event_adjustment(Request::new(V2RpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&adj_create).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_meter_event_adjustment(Request::new(V2RpcRequest {
        id: "mea_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.list_meter_event_adjustments(Request::new(V2RpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&ListMeterEventAdjustmentsParams::new()).expect("json"),
    })).await.is_ok());

    assert_eq!(mock.finish().await.len(), 14);
}
