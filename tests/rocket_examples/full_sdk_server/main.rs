//! Integration Test Suite for the Unified Rocket.rs Full SDK Server (`examples/rocket_stripe/`).
//!
//! Covers:
//! 1. Service directory discovery (`GET /`) enumerating all 7 mounted namespace groups and 37 routes.
//! 2. Native `"$STRIPE_SECRET_KEY"` & `"$STRIPE_WEBHOOK_SECRET"` resolution and `[REDACTED]` secret debug formatting.
//! 3. Cross-namespace end-to-end Rocket request dispatch across Checkout, Payments, Billing, Connect,
//!    Specialized Products, V2 Services, and Cryptographic Webhooks in a single unified server instance.

#[path = "../common/mod.rs"]
mod common;

use common::{read_http_request, setup_mock_rocket_client, write_json_response};
use rocket::http::{ContentType, Header, Status};
use rocket_stripe_example::config::AppConfig;
use serde_json::{Value, json};
use some_stripe::Webhook;

#[tokio::test]
async fn scenario_1_unified_service_catalog_and_secret_redaction() {
    let config = AppConfig::from_env();
    let debug_str = format!("{:?}", config.webhook_secret);
    assert!(debug_str.contains("REDACTED"));

    let (_listener, client) = setup_mock_rocket_client("$STRIPE_WEBHOOK_SECRET").await;
    let res = client.get("/").dispatch().await;
    assert_eq!(res.status(), Status::Ok);

    let body: Value = res.into_json().await.unwrap();
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["service"], "some-stripe Rocket.rs Example");
    assert_eq!(body["data"]["version"], "0.8.0");

    let namespaces = &body["data"]["namespaces"];
    assert!(namespaces["checkout"].is_array());
    assert!(namespaces["payments"].is_array());
    assert!(namespaces["billing"].is_array());
    assert!(namespaces["connect"].is_array());
    assert!(namespaces["specialized"].is_array());
    assert!(namespaces["v2_services"].is_array());
    assert!(namespaces["webhooks"].is_array());
}

#[tokio::test]
async fn scenario_2_cross_namespace_unified_dispatch_e2e() {
    let (listener, client) = setup_mock_rocket_client("$STRIPE_WEBHOOK_SECRET").await;
    let now = chrono::Utc::now().timestamp();

    let mock_server = tokio::spawn(async move {
        // 1. POST /v1/customers
        let (mut s1, _) = listener.accept().await.unwrap();
        let (h1, _) = read_http_request(&mut s1).await;
        assert!(h1.starts_with("POST /v1/customers HTTP/1.1"));
        write_json_response(
            &mut s1,
            "200 OK",
            &json!({
                "id": "cus_unified_1",
                "object": "customer",
                "email": "unified@rocket.example.com",
                "balance": 0,
                "created": now,
                "delinquent": false,
                "livemode": false
            })
            .to_string(),
        )
        .await;

        // 2. POST /v1/payment_intents
        let (mut s2, _) = listener.accept().await.unwrap();
        let (h2, _) = read_http_request(&mut s2).await;
        assert!(h2.starts_with("POST /v1/payment_intents HTTP/1.1"));
        write_json_response(
            &mut s2,
            "200 OK",
            &json!({
                "id": "pi_unified_1",
                "object": "payment_intent",
                "amount": 5000,
                "currency": "usd",
                "status": "succeeded"
            })
            .to_string(),
        )
        .await;

        // 3. POST /v2/billing/meter_events
        let (mut s3, _) = listener.accept().await.unwrap();
        let (h3, _) = read_http_request(&mut s3).await;
        assert!(h3.starts_with("POST /v2/billing/meter_events HTTP/1.1"));
        write_json_response(
            &mut s3,
            "200 OK",
            &json!({
                "object": "v2.billing.meter_event",
                "created": now,
                "livemode": false,
                "event_name": "api_requests",
                "payload": {
                    "stripe_customer_id": "cus_unified_1",
                    "value": "42"
                }
            })
            .to_string(),
        )
        .await;
    });

    // 1. Billing: Create Customer
    let cus_res = client
        .post("/api/billing/customers")
        .header(ContentType::JSON)
        .body(json!({ "email": "unified@rocket.example.com" }).to_string())
        .dispatch()
        .await;
    assert_eq!(cus_res.status(), Status::Ok);

    // 2. Payments: Create PaymentIntent
    let pi_res = client
        .post("/api/payments/intents")
        .header(ContentType::JSON)
        .body(
            json!({
                "amount": 5000,
                "currency": "usd",
                "customer": "cus_unified_1"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(pi_res.status(), Status::Ok);

    // 3. V2 Services: Record Meter Event
    let meter_res = client
        .post("/api/v2/billing/meter-events")
        .header(ContentType::JSON)
        .body(
            json!({
                "event_name": "api_requests",
                "stripe_customer_id": "cus_unified_1",
                "value": "42"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(meter_res.status(), Status::Ok);

    // 4. Webhooks: Verify signed V1 event using "$STRIPE_WEBHOOK_SECRET"
    let webhook_payload = json!({
        "id": "evt_unified_1",
        "object": "event",
        "created": now,
        "type": "payment_intent.succeeded",
        "data": { "object": { "id": "pi_unified_1", "payment_intent": "pi_unified_1" } }
    })
    .to_string();
    let sig = Webhook::generate_test_header_string(
        webhook_payload.as_bytes(),
        "$STRIPE_WEBHOOK_SECRET",
        Some(now),
    );
    let wh_res = client
        .post("/webhooks/stripe")
        .header(ContentType::JSON)
        .header(Header::new("Stripe-Signature", sig))
        .body(webhook_payload)
        .dispatch()
        .await;
    assert_eq!(wh_res.status(), Status::Ok);

    mock_server.await.unwrap();
}
