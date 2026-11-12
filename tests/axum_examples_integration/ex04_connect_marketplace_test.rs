//! Integration Test Suite for `examples/axum/04_axum_connect_marketplace.rs` (Module 6: Connect & Marketplaces).
//!
//! Covers:
//! 1. `POST /connect/account` Express merchant account creation.
//! 2. `POST /connect/account/{id}/link` hosted AccountLink onboarding generation (`AccountLinkType::AccountOnboarding`).
//! 3. `POST /connect/account/{id}/person` KYC Person onboarding with `Stripe-Account` multi-tenant header verification.
//! 4. `POST /connect/transfer` split payment transfer to connected merchant.
//! 5. `POST /connect/payout` merchant bank payout creation.

#[path = "../../examples/axum/04_axum_connect_marketplace.rs"]
mod ex04_axum_connect_marketplace;

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::{
    build_mock_stripe_client, send_json_request, spawn_single_mock_stripe_server,
};
use ex04_axum_connect_marketplace::{build_router, ConnectState};

/// Scenario 1 & 2: Connected Account creation and Hosted Onboarding AccountLink generation.
#[tokio::test]
async fn test_ex04_create_merchant_account_and_onboarding_link() {
    let mock_account = r#"{
        "id": "acct_test_merchant_04",
        "object": "account",
        "type": "express",
        "country": "US",
        "email": "merchant@example.com"
    }"#;

    let (api_base1, h1) = spawn_single_mock_stripe_server("200 OK", mock_account).await;
    let app1 = build_router(ConnectState::with_client(build_mock_stripe_client(api_base1)));
    let (s1, b1) = send_json_request(
        app1,
        Method::POST,
        "/connect/account",
        Some(json!({ "email": "merchant@example.com", "country": "US" })),
    )
    .await;
    assert_eq!(s1, StatusCode::CREATED);
    assert_eq!(b1["id"], "acct_test_merchant_04");
    let _ = h1.await;

    let mock_link = r#"{
        "object": "account_link",
        "created": 1728345600,
        "expires_at": 1728349200,
        "url": "https://connect.stripe.com/setup/s/acct_test_merchant_04_token"
    }"#;
    let (api_base2, h2) = spawn_single_mock_stripe_server("200 OK", mock_link).await;
    let app2 = build_router(ConnectState::with_client(build_mock_stripe_client(api_base2)));
    let (s2, b2) = send_json_request(
        app2,
        Method::POST,
        "/connect/account/acct_test_merchant_04/link",
        Some(json!({
            "refresh_url": "https://example.com/reauth",
            "return_url": "https://example.com/return"
        })),
    )
    .await;
    assert_eq!(s2, StatusCode::CREATED);
    assert!(b2["url"]
        .as_str()
        .unwrap_or_default()
        .contains("connect.stripe.com"));
    let (_, wire_link) = h2.await.expect("Mock server failed");
    assert!(wire_link.contains("type=account_onboarding"));
}

/// Scenario 3, 4 & 5: KYC Person onboarding (`Stripe-Account` header), Split Transfer, and Payout.
#[tokio::test]
async fn test_ex04_kyc_person_transfer_and_payout() {
    let mock_person = r#"{
        "id": "person_test_04",
        "object": "person",
        "account": "acct_test_merchant_04",
        "first_name": "Grace",
        "last_name": "Hopper"
    }"#;

    let (api_base1, h1) = spawn_single_mock_stripe_server("200 OK", mock_person).await;
    let app1 = build_router(ConnectState::with_client(build_mock_stripe_client(api_base1)));
    let (s1, b1) = send_json_request(
        app1,
        Method::POST,
        "/connect/account/acct_test_merchant_04/person",
        Some(json!({
            "first_name": "Grace",
            "last_name": "Hopper",
            "email": "grace@example.com"
        })),
    )
    .await;
    assert_eq!(s1, StatusCode::CREATED);
    assert_eq!(b1["id"], "person_test_04");
    let (req_headers, _) = h1.await.expect("Mock server failed");
    assert!(req_headers.starts_with("POST /v1/accounts/acct_test_merchant_04/persons HTTP/1.1"));
    assert!(req_headers
        .to_ascii_lowercase()
        .contains("stripe-account: acct_test_merchant_04"));

    // Split Transfer
    let mock_transfer = r#"{
        "id": "tr_test_04",
        "object": "transfer",
        "amount": 4000,
        "currency": "usd",
        "destination": "acct_test_merchant_04"
    }"#;
    let (api_base2, h2) = spawn_single_mock_stripe_server("200 OK", mock_transfer).await;
    let app2 = build_router(ConnectState::with_client(build_mock_stripe_client(api_base2)));
    let (s2, b2) = send_json_request(
        app2,
        Method::POST,
        "/connect/transfer",
        Some(json!({
            "destination": "acct_test_merchant_04",
            "amount": 4000,
            "currency": "USD"
        })),
    )
    .await;
    assert_eq!(s2, StatusCode::CREATED);
    assert_eq!(b2["id"], "tr_test_04");
    let _ = h2.await;

    // Payout
    let mock_payout = r#"{
        "id": "po_test_04",
        "object": "payout",
        "amount": 4000,
        "currency": "usd",
        "status": "paid"
    }"#;
    let (api_base3, h3) = spawn_single_mock_stripe_server("200 OK", mock_payout).await;
    let app3 = build_router(ConnectState::with_client(build_mock_stripe_client(api_base3)));
    let (s3, b3) = send_json_request(
        app3,
        Method::POST,
        "/connect/payout",
        Some(json!({ "amount": 4000, "currency": "USD" })),
    )
    .await;
    assert_eq!(s3, StatusCode::CREATED);
    assert_eq!(b3["id"], "po_test_04");
    let _ = h3.await;
}
