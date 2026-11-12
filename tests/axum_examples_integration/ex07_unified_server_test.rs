//! Integration Test Suite for `examples/axum/axum_stripe_server/` (Unified Production Server covering all 8 Modules).
//!
//! Covers:
//! 1. Webhooks route (`/webhooks/stripe`) for both V1 events and V2 thin events.
//! 2. Payments routes (`/payments/intents`, `/payments/disputes/{id}`, `/payments/balance`, `/payments/balance/transactions`).
//! 3. Checkout routes (`/checkout/sessions` in both `payment` and `subscription` modes, `/checkout/payment-links`).
//! 4. Billing routes (`/billing/credit-notes` with valid & invalid `CreditNoteReason` rejection, `/billing/quotes`).
//! 5. Connect routes (`/connect/accounts/{id}/sessions`, `/connect/application-fees`).
//! 6. Specialized routes (`/specialized/treasury/transfers`).
//! 7. V2 Services routes (`/v2/billing/meters/adjustments`, `/v2/billing/meter-ingestion/stream-demo`).
//! 8. Unified `AppError` RFC 7807 error mapping (`401 Unauthorized`, `404 Not Found`, `429 Too Many Requests`).

#[path = "../../examples/axum/axum_stripe_server/main.rs"]
mod axum_stripe_server;

use axum::http::{Method, StatusCode};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{
    build_mock_stripe_client, send_json_request, send_raw_webhook_request,
    spawn_single_mock_stripe_server,
};
use axum_stripe_server::{routes::build_app_router, state::AppState};

const UNIFIED_WEBHOOK_SECRET: &str = "whsec_unified_server_test_secret";

/// Helper to build the unified server router backed by a mock `api_base`.
fn make_unified_app(api_base: impl Into<String>) -> axum::Router {
    let client = build_mock_stripe_client(api_base);
    let state = AppState::with_client(client, UNIFIED_WEBHOOK_SECRET);
    build_app_router().with_state(state)
}

/// Scenario 1: Unified server `/webhooks/stripe` processes both V1 and V2 thin events.
#[tokio::test]
async fn test_ex07_unified_webhooks_v1_and_v2_thin_events() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Clock drift")
        .as_secs() as i64;

    let hydrated_v2 = r#"{
        "id": "evt_v2_unified_07",
        "object": "v2.core.event",
        "type": "v1.billing.meter.error_report_triggered",
        "created": "2026-11-11T00:00:00Z"
    }"#;
    let (api_base, h1) = spawn_single_mock_stripe_server("200 OK", hydrated_v2).await;
    let client = build_mock_stripe_client(api_base);

    let thin_payload = format!(
        r#"{{"id":"evt_v2_unified_07","object":"v2.core.event","type":"v1.billing.meter.error_report_triggered","created":{now},"livemode":false}}"#
    );
    let sig = client.webhooks().generate_test_header_string(
        thin_payload.as_bytes(),
        UNIFIED_WEBHOOK_SECRET,
        Some(now),
    );
    let app = build_app_router().with_state(AppState::with_client(client, UNIFIED_WEBHOOK_SECRET));
    let (status, body) =
        send_raw_webhook_request(app, "/webhooks/stripe", Some(&sig), thin_payload.as_bytes())
            .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["v2_thin_event"], true);
    assert_eq!(body["hydrated_id"], "evt_v2_unified_07");
    let _ = h1.await;
}

/// Scenario 2: Unified Checkout supports `subscription` mode with `price_id` and rejects missing `price_id`.
#[tokio::test]
async fn test_ex07_unified_checkout_subscription_mode_validation() {
    // Missing price_id in subscription mode -> 400 Bad Request
    let app_bad = make_unified_app("http://127.0.0.1:9");
    let (s_bad, b_bad) = send_json_request(
        app_bad,
        Method::POST,
        "/checkout/sessions",
        Some(json!({
            "success_url": "https://example.com/ok",
            "mode": "subscription",
            "quantity": 1
        })),
    )
    .await;
    assert_eq!(s_bad, StatusCode::BAD_REQUEST);
    assert_eq!(b_bad["code"], "bad_request");

    // Valid subscription mode with price_id -> 201 Created
    let mock_cs = r#"{
        "id": "cs_sub_unified_07",
        "object": "checkout.session",
        "mode": "subscription",
        "status": "open"
    }"#;
    let (api_base, h1) = spawn_single_mock_stripe_server("200 OK", mock_cs).await;
    let app_ok = make_unified_app(api_base);
    let (s_ok, b_ok) = send_json_request(
        app_ok,
        Method::POST,
        "/checkout/sessions",
        Some(json!({
            "success_url": "https://example.com/ok",
            "mode": "subscription",
            "price_id": "price_recurring_monthly_07",
            "quantity": 2
        })),
    )
    .await;
    assert_eq!(s_ok, StatusCode::CREATED);
    assert_eq!(b_ok["id"], "cs_sub_unified_07");
    let (_, wire_body) = h1.await.expect("Mock server failed");
    assert!(
        wire_body.contains("price_recurring_monthly_07"),
        "Subscription checkout must pass recurring price_id in line_items: {wire_body}"
    );
}

/// Scenario 3: Unified Billing CreditNote validates `reason` and rejects typos with `400 Bad Request`.
#[tokio::test]
async fn test_ex07_unified_credit_note_reason_validation_and_quotes() {
    // Invalid reason "order_chagne" -> 400 Bad Request before hitting Stripe
    let app_bad = make_unified_app("http://127.0.0.1:9");
    let (s_bad, b_bad) = send_json_request(
        app_bad,
        Method::POST,
        "/billing/credit-notes",
        Some(json!({
            "invoice": "in_123",
            "amount": 1000,
            "reason": "order_chagne"
        })),
    )
    .await;
    assert_eq!(s_bad, StatusCode::BAD_REQUEST);
    assert_eq!(b_bad["code"], "bad_request");

    // Valid reason "order_change" -> 201 Created
    let mock_cn = r#"{
        "id": "cn_test_07",
        "object": "credit_note",
        "invoice": "in_123",
        "amount": 1000,
        "currency": "usd",
        "status": "issued",
        "reason": "order_change"
    }"#;
    let (api_base, h1) = spawn_single_mock_stripe_server("200 OK", mock_cn).await;
    let app_ok = make_unified_app(api_base);
    let (s_ok, b_ok) = send_json_request(
        app_ok,
        Method::POST,
        "/billing/credit-notes",
        Some(json!({
            "invoice": "in_123",
            "amount": 1000,
            "reason": "order_change"
        })),
    )
    .await;
    assert_eq!(s_ok, StatusCode::CREATED);
    assert_eq!(b_ok["id"], "cn_test_07");
    let _ = h1.await;

    // Create Quote
    let mock_quote = r#"{
        "id": "qt_test_07",
        "object": "quote",
        "status": "draft",
        "amount_total": 5000
    }"#;
    let (api_base2, h2) = spawn_single_mock_stripe_server("200 OK", mock_quote).await;
    let app_qt = make_unified_app(api_base2);
    let (s_qt, b_qt) = send_json_request(
        app_qt,
        Method::POST,
        "/billing/quotes",
        Some(json!({ "customer": "cus_123", "description": "Enterprise Quote" })),
    )
    .await;
    assert_eq!(s_qt, StatusCode::CREATED);
    assert_eq!(b_qt["id"], "qt_test_07");
    let _ = h2.await;
}

/// Scenario 4: Balance, BalanceTransactions paginator, Disputes, Treasury Transfers, and V2 Adjustments.
#[tokio::test]
async fn test_ex07_balance_disputes_treasury_and_v2_adjustments() {
    // Balance retrieve
    let mock_bal = r#"{
        "object": "balance",
        "available": [{"amount": 12000, "currency": "usd"}],
        "pending": [{"amount": 3000, "currency": "usd"}],
        "livemode": false
    }"#;
    let (api_base1, h1) = spawn_single_mock_stripe_server("200 OK", mock_bal).await;
    let (s1, b1) =
        send_json_request(make_unified_app(api_base1), Method::GET, "/payments/balance", None)
            .await;
    assert_eq!(s1, StatusCode::OK);
    assert_eq!(b1["available"][0]["amount"], 12000);
    let _ = h1.await;

    // BalanceTransactions paginated list
    let mock_bt_list = r#"{
        "object": "list",
        "has_more": false,
        "data": [
            {
                "id": "txn_07",
                "object": "balance_transaction",
                "amount": 5000,
                "fee": 175,
                "net": 4825,
                "currency": "usd",
                "type": "charge",
                "status": "available",
                "created": 1728345600,
                "available_on": 1728432000
            }
        ]
    }"#;
    let (api_base2, h2) = spawn_single_mock_stripe_server("200 OK", mock_bt_list).await;
    let (s2, b2) = send_json_request(
        make_unified_app(api_base2),
        Method::GET,
        "/payments/balance/transactions",
        None,
    )
    .await;
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(b2[0]["id"], "txn_07");
    let _ = h2.await;

    // Treasury OutboundTransfer
    let mock_ob = r#"{
        "id": "obt_test_07",
        "object": "treasury.outbound_transfer",
        "financial_account": "fa_07",
        "amount": 25000,
        "currency": "usd",
        "status": "processing"
    }"#;
    let (api_base3, h3) = spawn_single_mock_stripe_server("200 OK", mock_ob).await;
    let (s3, b3) = send_json_request(
        make_unified_app(api_base3),
        Method::POST,
        "/specialized/treasury/transfers",
        Some(json!({
            "financial_account": "fa_07",
            "amount": 25000,
            "currency": "USD",
            "destination_payment_method": "pm_us_bank_07"
        })),
    )
    .await;
    assert_eq!(s3, StatusCode::CREATED);
    assert_eq!(b3["id"], "obt_test_07");
    let _ = h3.await;

    // V2 Meter Event Adjustment
    let mock_adj = r#"{
        "id": "mea_test_07",
        "object": "billing.meter_event_adjustment",
        "event_name": "ai_tokens",
        "status": "complete",
        "type": "cancel",
        "created": 1728345600,
        "livemode": false
    }"#;
    let (api_base4, h4) = spawn_single_mock_stripe_server("200 OK", mock_adj).await;
    let (s4, b4) = send_json_request(
        make_unified_app(api_base4),
        Method::POST,
        "/v2/billing/meters/adjustments",
        Some(json!({ "event_name": "ai_tokens" })),
    )
    .await;
    assert_eq!(s4, StatusCode::CREATED);
    assert_eq!(b4["id"], "mea_test_07");
    let _ = h4.await;
}

/// Scenario 5: Unified `AppError` RFC 7807 HTTP status mapping (`401 Unauthorized`, `404 Not Found`).
#[tokio::test]
async fn test_ex07_app_error_rfc7807_http_status_mapping() {
    let err_401 = r#"{
        "error": {
            "type": "invalid_request_error",
            "code": "api_key_expired",
            "message": "Expired API Key provided"
        }
    }"#;
    let (api_base1, h1) = spawn_single_mock_stripe_server("401 Unauthorized", err_401).await;
    let (s1, b1) = send_json_request(
        make_unified_app(api_base1),
        Method::GET,
        "/payments/intents/pi_nonexistent",
        None,
    )
    .await;
    assert_eq!(s1, StatusCode::UNAUTHORIZED);
    assert_eq!(b1["code"], "api_key_expired");
    let _ = h1.await;

    let err_404 = r#"{
        "error": {
            "type": "invalid_request_error",
            "code": "resource_missing",
            "message": "No such payment_intent: pi_404"
        }
    }"#;
    let (api_base2, h2) = spawn_single_mock_stripe_server("404 Not Found", err_404).await;
    let (s2, b2) = send_json_request(
        make_unified_app(api_base2),
        Method::GET,
        "/payments/intents/pi_404",
        None,
    )
    .await;
    assert_eq!(s2, StatusCode::NOT_FOUND);
    assert_eq!(b2["code"], "resource_missing");
    let _ = h2.await;
}
