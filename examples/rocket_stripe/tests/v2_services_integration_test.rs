//! Integration tests for Stripe API V2 Next-Gen Services (`/api/v2/*`).
//!
//! Scenarios covered:
//! 1. Retrieving V2 Core Events (`GET /api/v2/events/<id>`).
//! 2. Retrieving V2 Event Destinations (`GET /api/v2/destinations/<id>`).
//! 3. Retrieving Next-Gen V2 Merchant Accounts (`GET /api/v2/accounts/<id>`).
//! 4. Hydrating inbound V2 Thin Event notifications (`POST /api/v2/events/hydrate`).
//! 5. Ingesting high-throughput V2 Billing Meter Events (`POST /api/v2/billing/meter-events`).

mod common;

use common::{read_http_request, setup_mock_rocket_client, write_json_response};
use rocket::http::{ContentType, Status};
use serde_json::{Value, json};

#[tokio::test]
async fn test_v2_events_destinations_and_accounts_retrieval() {
    let (listener, client) = setup_mock_rocket_client("whsec_test").await;

    let mock_server = tokio::spawn(async move {
        // 1. GET /v2/core/events/evt_v2_rocket_1
        let (mut s1, _) = listener.accept().await.unwrap();
        let (h1, _) = read_http_request(&mut s1).await;
        assert!(h1.starts_with("GET /v2/core/events/evt_v2_rocket_1 HTTP/1.1"));
        write_json_response(
            &mut s1,
            "200 OK",
            &json!({
                "id": "evt_v2_rocket_1",
                "object": "v2.core.event",
                "type": "v1.billing.meter.error_report_triggered",
                "created": 1728345600,
                "livemode": false,
                "data": { "reason": "threshold_exceeded" }
            })
            .to_string(),
        )
        .await;

        // 2. GET /v2/core/event_destinations/edest_rocket_1
        let (mut s2, _) = listener.accept().await.unwrap();
        let (h2, _) = read_http_request(&mut s2).await;
        assert!(h2.starts_with("GET /v2/core/event_destinations/edest_rocket_1 HTTP/1.1"));
        write_json_response(
            &mut s2,
            "200 OK",
            &json!({
                "id": "edest_rocket_1",
                "object": "v2.core.event_destination",
                "status": "enabled",
                "type": "amazon_eventbridge",
                "events": ["v1.billing.meter.error_report_triggered"],
                "created": 1728345600,
                "updated": 1728345600
            })
            .to_string(),
        )
        .await;

        // 3. GET /v2/core/accounts/acct_v2_rocket_1
        let (mut s3, _) = listener.accept().await.unwrap();
        let (h3, _) = read_http_request(&mut s3).await;
        assert!(h3.starts_with("GET /v2/core/accounts/acct_v2_rocket_1 HTTP/1.1"));
        write_json_response(
            &mut s3,
            "200 OK",
            &json!({
                "id": "acct_v2_rocket_1",
                "object": "v2.core.account",
                "status": "active",
                "display_name": "Rocket V2 Merchant",
                "contact_email": "v2@rocket.example.com",
                "created": 1728345600,
                "updated": 1728345600
            })
            .to_string(),
        )
        .await;
    });

    // 1. Get V2 Event
    let ev_res = client.get("/api/v2/events/evt_v2_rocket_1").dispatch().await;
    assert_eq!(ev_res.status(), Status::Ok);
    let ev_json: Value = ev_res.into_json().await.unwrap();
    assert_eq!(ev_json["data"]["id"], "evt_v2_rocket_1");

    // 2. Get V2 Event Destination
    let dst_res = client
        .get("/api/v2/destinations/edest_rocket_1")
        .dispatch()
        .await;
    assert_eq!(dst_res.status(), Status::Ok);
    let dst_json: Value = dst_res.into_json().await.unwrap();
    assert_eq!(dst_json["data"]["id"], "edest_rocket_1");

    // 3. Get V2 Account
    let acct_res = client
        .get("/api/v2/accounts/acct_v2_rocket_1")
        .dispatch()
        .await;
    assert_eq!(acct_res.status(), Status::Ok);
    let acct_json: Value = acct_res.into_json().await.unwrap();
    assert_eq!(acct_json["data"]["display_name"], "Rocket V2 Merchant");

    mock_server.await.unwrap();
}

#[tokio::test]
async fn test_v2_thin_event_hydration_and_meter_event_ingestion() {
    let (listener, client) = setup_mock_rocket_client("whsec_test").await;
    let now = chrono::Utc::now().timestamp();

    let mock_server = tokio::spawn(async move {
        // 1. Thin Event Hydrator fetches GET /v2/core/events/evt_v2_thin_999
        let (mut s1, _) = listener.accept().await.unwrap();
        let (h1, _) = read_http_request(&mut s1).await;
        assert!(h1.starts_with("GET /v2/core/events/evt_v2_thin_999 HTTP/1.1"));
        write_json_response(
            &mut s1,
            "200 OK",
            &json!({
                "id": "evt_v2_thin_999",
                "object": "v2.core.event",
                "type": "v1.billing.meter.error_report_triggered",
                "created": now,
                "livemode": false,
                "related_object": {
                    "id": "mtr_rocket_123",
                    "type": "billing.meter",
                    "url": "https://api.stripe.com/v2/billing/meters/mtr_rocket_123"
                },
                "data": { "status": "hydrated_ok" }
            })
            .to_string(),
        )
        .await;

        // 2. POST /v2/billing/meter_events
        let (mut s2, _) = listener.accept().await.unwrap();
        let (h2, b2) = read_http_request(&mut s2).await;
        assert!(h2.starts_with("POST /v2/billing/meter_events HTTP/1.1"));
        assert!(b2.contains("\"event_name\":\"ai_inference_tokens\""));
        assert!(b2.contains("\"stripe_customer_id\":\"cus_ai_user_1\""));
        assert!(b2.contains("\"value\":\"2048\""));
        write_json_response(
            &mut s2,
            "200 OK",
            &json!({
                "object": "v2.billing.meter_event",
                "created": now,
                "livemode": false,
                "event_name": "ai_inference_tokens",
                "payload": {
                    "stripe_customer_id": "cus_ai_user_1",
                    "value": "2048"
                }
            })
            .to_string(),
        )
        .await;
    });

    // 1. Hydrate V2 Thin Event
    let hydrate_res = client
        .post("/api/v2/events/hydrate")
        .header(ContentType::JSON)
        .body(
            json!({
                "id": "evt_v2_thin_999",
                "object": "v2.core.event_notification",
                "type": "v1.billing.meter.error_report_triggered",
                "created": now,
                "livemode": false,
                "related_object": {
                    "id": "mtr_rocket_123",
                    "type": "billing.meter",
                    "url": "https://api.stripe.com/v2/billing/meters/mtr_rocket_123"
                }
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(hydrate_res.status(), Status::Ok);
    let hydrate_json: Value = hydrate_res.into_json().await.unwrap();
    assert_eq!(hydrate_json["data"]["id"], "evt_v2_thin_999");
    assert_eq!(hydrate_json["data"]["data"]["status"], "hydrated_ok");

    // 2. Record Usage Meter Event
    let meter_res = client
        .post("/api/v2/billing/meter-events")
        .header(ContentType::JSON)
        .body(
            json!({
                "event_name": "ai_inference_tokens",
                "stripe_customer_id": "cus_ai_user_1",
                "value": "2048"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(meter_res.status(), Status::Ok);
    let meter_json: Value = meter_res.into_json().await.unwrap();
    assert_eq!(meter_json["data"]["event_name"], "ai_inference_tokens");
    assert_eq!(meter_json["data"]["payload"]["value"], "2048");

    mock_server.await.unwrap();
}
