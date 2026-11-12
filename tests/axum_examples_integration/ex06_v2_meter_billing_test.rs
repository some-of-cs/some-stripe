//! Integration Test Suite for `examples/06_axum_v2_meter_billing.rs` (Module 8: API V2 & Meter Billing).
//!
//! Covers:
//! 1. `GET /v2/events` listing API V2 core events.
//! 2. `POST /v2/destinations` creating a V2 Webhook Event Destination with URL validation.
//! 3. `POST /v2/accounts` creating a V2 Core Account.
//! 4. `POST /v2/meters` creating a V2 Billing Meter (verifying `value_settings.event_payload_key == "value"`).
//! 5. `POST /v2/meters/event` streaming usage events and `GET /v2/meters/engine-stats` verifying that
//!    the shared `MeterStreamCollector` in `V2State` increments `events_received`.

#[path = "../../examples/06_axum_v2_meter_billing.rs"]
mod ex06_axum_v2_meter_billing;

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::{
    build_mock_stripe_client, send_json_request, spawn_single_mock_stripe_server,
};
use ex06_axum_v2_meter_billing::{build_router, V2State};

/// Scenario 1, 2 & 3: V2 Core Events list, Webhook EventDestination creation, and V2 Account creation.
#[tokio::test]
async fn test_ex06_v2_events_destinations_and_accounts() {
    let mock_events = r#"{
        "data": [
            {
                "id": "evt_v2_06",
                "object": "v2.core.event",
                "type": "v1.billing.meter.error_report_triggered",
                "created": "2026-11-11T00:00:00Z"
            }
        ]
    }"#;
    let (api_base1, h1) = spawn_single_mock_stripe_server("200 OK", mock_events).await;
    let app1 = build_router(V2State::with_client(build_mock_stripe_client(api_base1)));
    let (s1, b1) = send_json_request(app1, Method::GET, "/v2/events", None).await;
    assert_eq!(s1, StatusCode::OK);
    assert_eq!(b1["data"][0]["id"], "evt_v2_06");
    let _ = h1.await;

    // Create V2 Webhook Event Destination
    let mock_dest = r#"{
        "id": "edest_test_06",
        "object": "v2.core.event_destination",
        "status": "enabled",
        "type": "webhook_endpoint",
        "events": ["v1.billing.meter.error_report_triggered"],
        "created": 1728345600,
        "webhook_endpoint": {
            "url": "https://api.example.com/v2/webhooks"
        }
    }"#;
    let (api_base2, h2) = spawn_single_mock_stripe_server("200 OK", mock_dest).await;
    let app2 = build_router(V2State::with_client(build_mock_stripe_client(api_base2)));
    let (s2, b2) = send_json_request(
        app2,
        Method::POST,
        "/v2/destinations",
        Some(json!({
            "url": "https://api.example.com/v2/webhooks",
            "events": ["v1.billing.meter.error_report_triggered"]
        })),
    )
    .await;
    assert_eq!(s2, StatusCode::CREATED);
    assert_eq!(b2["id"], "edest_test_06");
    let _ = h2.await;

    // Create V2 Account
    let mock_acct = r#"{
        "id": "acct_v2_test_06",
        "object": "v2.core.account",
        "display_name": "Acme AI SaaS"
    }"#;
    let (api_base3, h3) = spawn_single_mock_stripe_server("200 OK", mock_acct).await;
    let app3 = build_router(V2State::with_client(build_mock_stripe_client(api_base3)));
    let (s3, b3) = send_json_request(
        app3,
        Method::POST,
        "/v2/accounts",
        Some(json!({ "display_name": "Acme AI SaaS" })),
    )
    .await;
    assert_eq!(s3, StatusCode::CREATED);
    assert_eq!(b3["id"], "acct_v2_test_06");
    let _ = h3.await;
}

/// Scenario 4 & 5: V2 Billing Meter (`event_payload_key == "value"`), event streaming, and live collector telemetry.
#[tokio::test]
async fn test_ex06_v2_billing_meter_and_shared_collector_stats() {
    let mock_meter = r#"{
        "id": "mtr_test_06",
        "object": "billing.meter",
        "display_name": "AI Tokens",
        "event_name": "ai_tokens",
        "status": "active",
        "value_settings": { "event_payload_key": "value" },
        "customer_mapping": { "event_payload_key": "stripe_customer_id", "type": "by_id" },
        "aggregation_formula": "sum",
        "created": 1728345600,
        "updated": 1728345600,
        "livemode": false
    }"#;

    let (api_base1, h1) = spawn_single_mock_stripe_server("200 OK", mock_meter).await;
    let app1 = build_router(V2State::with_client(build_mock_stripe_client(api_base1)));
    let (s1, b1) = send_json_request(
        app1,
        Method::POST,
        "/v2/meters",
        Some(json!({
            "display_name": "AI Tokens",
            "event_name": "ai_tokens"
        })),
    )
    .await;
    assert_eq!(s1, StatusCode::CREATED);
    assert_eq!(b1["id"], "mtr_test_06");
    let (_, meter_wire) = h1.await.expect("Mock server failed");
    assert!(
        meter_wire.contains(""event_payload_key":"value""),
        "Meter creation must configure event_payload_key as 'value': {meter_wire}"
    );

    // Stream event and verify shared collector telemetry
    let mock_meter_evt = r#"{
        "object": "v2.billing.meter_event",
        "event_name": "ai_tokens",
        "created": 1728345600,
        "livemode": false,
        "payload": {
            "stripe_customer_id": "cus_ai_06",
            "value": "250"
        }
    }"#;
    let (api_base2, h2) = spawn_single_mock_stripe_server("200 OK", mock_meter_evt).await;
    let shared_state = V2State::with_client(build_mock_stripe_client(api_base2));

    let (s2, _) = send_json_request(
        build_router(shared_state.clone()),
        Method::POST,
        "/v2/meters/event",
        Some(json!({
            "event_name": "ai_tokens",
            "customer_id": "cus_ai_06",
            "value": "250"
        })),
    )
    .await;
    assert_eq!(s2, StatusCode::CREATED);
    let _ = h2.await;

    // Brief yield to let collector worker process the channel message
    tokio::time::sleep(std::time::Duration::from_millis(25)).await;

    let (s3, stats_body) = send_json_request(
        build_router(shared_state),
        Method::GET,
        "/v2/meters/engine-stats",
        None,
    )
    .await;
    assert_eq!(s3, StatusCode::OK);
    assert!(
        stats_body["events_received"].as_u64().unwrap_or(0) >= 1,
        "Shared collector must record streamed event: {stats_body}"
    );
}
