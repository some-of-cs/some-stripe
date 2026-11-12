//! Integration Test Suite for `examples/03_axum_subscriptions_and_invoicing.rs` (Module 5: Billing & GDPR).
//!
//! Covers:
//! 1. `POST /customers`, `GET /customers/{id}` (GDPR Art. 15 portability), and `DELETE /customers/{id}` (GDPR Art. 17 erasure).
//! 2. `POST /subscriptions` (14-day trial period) and `DELETE /subscriptions/{id}`.
//! 3. `POST /invoices` and `POST /invoices/{id}/pay`.
//! 4. `POST /portal/session` self-service Customer Portal session minting.
//! 5. Live `.env` Stripe API Customer create/retrieve/delete cycle when `STRIPE_SECRET_KEY` is set.

#[path = "../../examples/03_axum_subscriptions_and_invoicing.rs"]
mod ex03_axum_subscriptions_and_invoicing;

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::{
    build_live_stripe_client, build_mock_stripe_client, send_json_request,
    spawn_single_mock_stripe_server,
};
use ex03_axum_subscriptions_and_invoicing::{build_router, BillingState};

/// Scenario 1: Customer creation, GDPR Art. 15 retrieval, and GDPR Art. 17 deletion.
#[tokio::test]
async fn test_ex03_customer_vault_and_gdpr_lifecycle() {
    let mock_customer = r#"{
        "id": "cus_test_gdpr_03",
        "object": "customer",
        "email": "subscriber@example.com",
        "name": "Ada Lovelace"
    }"#;

    // Create Customer
    let (api_base1, h1) = spawn_single_mock_stripe_server("200 OK", mock_customer).await;
    let app1 = build_router(BillingState::with_client(build_mock_stripe_client(api_base1)));
    let (s1, b1) = send_json_request(
        app1,
        Method::POST,
        "/customers",
        Some(json!({ "email": "subscriber@example.com", "name": "Ada Lovelace" })),
    )
    .await;
    assert_eq!(s1, StatusCode::CREATED);
    assert_eq!(b1["id"], "cus_test_gdpr_03");
    let _ = h1.await;

    // Retrieve Customer (GDPR Art. 15)
    let (api_base2, h2) = spawn_single_mock_stripe_server("200 OK", mock_customer).await;
    let app2 = build_router(BillingState::with_client(build_mock_stripe_client(api_base2)));
    let (s2, b2) =
        send_json_request(app2, Method::GET, "/customers/cus_test_gdpr_03", None).await;
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(b2["email"], "subscriber@example.com");
    let _ = h2.await;

    // Delete Customer (GDPR Art. 17)
    let mock_deleted = r#"{
        "id": "cus_test_gdpr_03",
        "object": "customer",
        "deleted": true
    }"#;
    let (api_base3, h3) = spawn_single_mock_stripe_server("200 OK", mock_deleted).await;
    let app3 = build_router(BillingState::with_client(build_mock_stripe_client(api_base3)));
    let (s3, b3) =
        send_json_request(app3, Method::DELETE, "/customers/cus_test_gdpr_03", None).await;
    assert_eq!(s3, StatusCode::OK);
    assert_eq!(b3["deleted"], true);
    let _ = h3.await;
}

/// Scenario 2, 3 & 4: Subscriptions, Invoices, and Billing Portal sessions.
#[tokio::test]
async fn test_ex03_subscriptions_invoices_and_portal() {
    let mock_sub = r#"{
        "id": "sub_test_03",
        "object": "subscription",
        "customer": "cus_test_gdpr_03",
        "status": "trialing"
    }"#;

    let (api_base1, h1) = spawn_single_mock_stripe_server("200 OK", mock_sub).await;
    let app1 = build_router(BillingState::with_client(build_mock_stripe_client(api_base1)));
    let (s1, b1) = send_json_request(
        app1,
        Method::POST,
        "/subscriptions",
        Some(json!({ "customer": "cus_test_gdpr_03", "price_id": "price_pro_tier" })),
    )
    .await;
    assert_eq!(s1, StatusCode::CREATED);
    assert_eq!(b1["id"], "sub_test_03");
    let (_, wire_sub) = h1.await.expect("Mock server failed");
    assert!(wire_sub.contains("trial_period_days=14"));

    // Cancel subscription
    let (api_base2, h2) = spawn_single_mock_stripe_server("200 OK", mock_sub).await;
    let app2 = build_router(BillingState::with_client(build_mock_stripe_client(api_base2)));
    let (s2, _) =
        send_json_request(app2, Method::DELETE, "/subscriptions/sub_test_03", None).await;
    assert_eq!(s2, StatusCode::OK);
    let _ = h2.await;

    // Create & Pay Invoice
    let mock_inv = r#"{
        "id": "in_test_03",
        "object": "invoice",
        "customer": "cus_test_gdpr_03",
        "status": "paid",
        "amount_due": 2900
    }"#;
    let (api_base3, h3) = spawn_single_mock_stripe_server("200 OK", mock_inv).await;
    let app3 = build_router(BillingState::with_client(build_mock_stripe_client(api_base3)));
    let (s3, b3) = send_json_request(
        app3,
        Method::POST,
        "/invoices",
        Some(json!({ "customer": "cus_test_gdpr_03" })),
    )
    .await;
    assert_eq!(s3, StatusCode::CREATED);
    assert_eq!(b3["id"], "in_test_03");
    let _ = h3.await;

    let (api_base4, h4) = spawn_single_mock_stripe_server("200 OK", mock_inv).await;
    let app4 = build_router(BillingState::with_client(build_mock_stripe_client(api_base4)));
    let (s4, _) =
        send_json_request(app4, Method::POST, "/invoices/in_test_03/pay", None).await;
    assert_eq!(s4, StatusCode::OK);
    let _ = h4.await;

    // Create Billing Portal session
    let mock_portal = r#"{
        "id": "bps_test_03",
        "object": "billing_portal.session",
        "customer": "cus_test_gdpr_03",
        "url": "https://billing.stripe.com/p/session/bps_test_03"
    }"#;
    let (api_base5, h5) = spawn_single_mock_stripe_server("200 OK", mock_portal).await;
    let app5 = build_router(BillingState::with_client(build_mock_stripe_client(api_base5)));
    let (s5, b5) = send_json_request(
        app5,
        Method::POST,
        "/portal/session",
        Some(json!({
            "customer": "cus_test_gdpr_03",
            "return_url": "https://example.com/billing"
        })),
    )
    .await;
    assert_eq!(s5, StatusCode::CREATED);
    assert_eq!(b5["id"], "bps_test_03");
    let _ = h5.await;
}

/// Scenario 5: Live `.env` Stripe API Customer create & GDPR delete roundtrip.
#[tokio::test]
async fn test_ex03_live_env_customer_roundtrip() {
    let Some(live_client) = build_live_stripe_client() else {
        return;
    };
    let app = build_router(BillingState::with_client(live_client.clone()));
    let (status, created) = send_json_request(
        app,
        Method::POST,
        "/customers",
        Some(json!({
            "email": "axum-integration-test@example.com",
            "name": "Axum Integration Tester"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let cus_id = created["id"].as_str().expect("Missing customer id");

    let app_del = build_router(BillingState::with_client(live_client));
    let (del_status, del_body) =
        send_json_request(app_del, Method::DELETE, &format!("/customers/{cus_id}"), None).await;
    assert_eq!(del_status, StatusCode::OK);
    assert_eq!(del_body["deleted"], true);
}
