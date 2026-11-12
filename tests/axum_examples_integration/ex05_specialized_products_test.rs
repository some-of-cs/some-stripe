//! Integration Test Suite for `examples/axum/05_axum_specialized_products.rs` (Module 7: Specialized Products).
//!
//! Covers:
//! 1. `POST /tax/calculate` & `POST /tax/commit` (Stripe Tax calculations and immutable transaction commit).
//! 2. `POST /identity/verify` & `GET /identity/session/{id}` (Stripe Identity KYC verification sessions).
//! 3. `POST /issuing/cardholder` (verifying `individual[first_name]` & `individual[last_name]` for active cards)
//!    and `POST /issuing/card` (Issuing virtual card activation).
//! 4. `POST /treasury/account` (Treasury financial account creation).
//! 5. `POST /terminal/token` (Terminal Smart Reader connection token minting).

#[path = "../../examples/axum/05_axum_specialized_products.rs"]
mod ex05_axum_specialized_products;

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::{
    build_mock_stripe_client, send_json_request, spawn_single_mock_stripe_server,
};
use ex05_axum_specialized_products::{build_router, SpecializedState};

/// Scenario 1 & 2: Stripe Tax calculation/commit and Identity KYC verification session lifecycle.
#[tokio::test]
async fn test_ex05_tax_and_identity_verification_lifecycle() {
    let mock_tax_calc = r#"{
        "id": "taxcalc_test_05",
        "object": "tax.calculation",
        "amount_total": 10825,
        "currency": "usd",
        "tax_amount_exclusive": 825
    }"#;

    let (api_base1, h1) = spawn_single_mock_stripe_server("200 OK", mock_tax_calc).await;
    let app1 = build_router(SpecializedState::with_client(build_mock_stripe_client(
        api_base1,
    )));
    let (s1, b1) = send_json_request(
        app1,
        Method::POST,
        "/tax/calculate",
        Some(json!({
            "postal_code": "94103",
            "country": "US",
            "amount": 10000
        })),
    )
    .await;
    assert_eq!(s1, StatusCode::CREATED);
    assert_eq!(b1["id"], "taxcalc_test_05");
    let _ = h1.await;

    let mock_tax_tx = r#"{
        "id": "tax_tx_test_05",
        "object": "tax.transaction",
        "currency": "usd",
        "reference": "order_tax_tx_1"
    }"#;
    let (api_base2, h2) = spawn_single_mock_stripe_server("200 OK", mock_tax_tx).await;
    let app2 = build_router(SpecializedState::with_client(build_mock_stripe_client(
        api_base2,
    )));
    let (s2, b2) = send_json_request(
        app2,
        Method::POST,
        "/tax/commit",
        Some(json!({ "calculation_id": "taxcalc_test_05" })),
    )
    .await;
    assert_eq!(s2, StatusCode::CREATED);
    assert_eq!(b2["id"], "tax_tx_test_05");
    let _ = h2.await;

    // Identity VerificationSession create & retrieve
    let mock_id_sess = r#"{
        "id": "vs_test_05",
        "object": "identity.verification_session",
        "status": "requires_input",
        "type": "document"
    }"#;
    let (api_base3, h3) = spawn_single_mock_stripe_server("200 OK", mock_id_sess).await;
    let app3 = build_router(SpecializedState::with_client(build_mock_stripe_client(
        api_base3,
    )));
    let (s3, b3) = send_json_request(app3, Method::POST, "/identity/verify", None).await;
    assert_eq!(s3, StatusCode::CREATED);
    assert_eq!(b3["id"], "vs_test_05");
    let _ = h3.await;

    let (api_base4, h4) = spawn_single_mock_stripe_server("200 OK", mock_id_sess).await;
    let app4 = build_router(SpecializedState::with_client(build_mock_stripe_client(
        api_base4,
    )));
    let (s4, b4) =
        send_json_request(app4, Method::GET, "/identity/session/vs_test_05", None).await;
    assert_eq!(s4, StatusCode::OK);
    assert_eq!(b4["id"], "vs_test_05");
    let _ = h4.await;
}

/// Scenario 3, 4 & 5: Issuing Cardholder (with `individual` first/last name) + Active Card, Treasury, and Terminal.
#[tokio::test]
async fn test_ex05_issuing_treasury_and_terminal() {
    let mock_holder = r#"{
        "id": "ich_test_05",
        "object": "issuing.cardholder",
        "name": "Alan Turing",
        "status": "active",
        "type": "individual"
    }"#;

    let (api_base1, h1) = spawn_single_mock_stripe_server("200 OK", mock_holder).await;
    let app1 = build_router(SpecializedState::with_client(build_mock_stripe_client(
        api_base1,
    )));
    let (s1, b1) = send_json_request(
        app1,
        Method::POST,
        "/issuing/cardholder",
        Some(json!({
            "name": "Alan Turing",
            "first_name": "Alan",
            "last_name": "Turing",
            "email": "alan@example.com",
            "line1": "10 Downing St",
            "city": "London",
            "postal_code": "SW1A 2AA",
            "country": "GB"
        })),
    )
    .await;
    assert_eq!(s1, StatusCode::CREATED);
    assert_eq!(b1["id"], "ich_test_05");
    let (_, holder_wire) = h1.await.expect("Mock server failed");
    assert!(
        holder_wire.contains("first_name%5D=Alan") || holder_wire.contains("first_name]=Alan"),
        "Cardholder wire body must include individual[first_name]: {holder_wire}"
    );
    assert!(
        holder_wire.contains("last_name%5D=Turing") || holder_wire.contains("last_name]=Turing"),
        "Cardholder wire body must include individual[last_name]: {holder_wire}"
    );

    // Issue Virtual Card
    let mock_card = r#"{
        "id": "ic_test_05",
        "object": "issuing.card",
        "currency": "usd",
        "status": "active",
        "type": "virtual",
        "last4": "4242",
        "exp_month": 12,
        "exp_year": 2029
    }"#;
    let (api_base2, h2) = spawn_single_mock_stripe_server("200 OK", mock_card).await;
    let app2 = build_router(SpecializedState::with_client(build_mock_stripe_client(
        api_base2,
    )));
    let (s2, b2) = send_json_request(
        app2,
        Method::POST,
        "/issuing/card",
        Some(json!({ "cardholder": "ich_test_05" })),
    )
    .await;
    assert_eq!(s2, StatusCode::CREATED);
    assert_eq!(b2["id"], "ic_test_05");
    let _ = h2.await;

    // Treasury FinancialAccount
    let mock_fa = r#"{
        "id": "fa_test_05",
        "object": "treasury.financial_account",
        "supported_currencies": ["usd"],
        "status": "open"
    }"#;
    let (api_base3, h3) = spawn_single_mock_stripe_server("200 OK", mock_fa).await;
    let app3 = build_router(SpecializedState::with_client(build_mock_stripe_client(
        api_base3,
    )));
    let (s3, b3) = send_json_request(app3, Method::POST, "/treasury/account", None).await;
    assert_eq!(s3, StatusCode::CREATED);
    assert_eq!(b3["id"], "fa_test_05");
    let _ = h3.await;

    // Terminal ConnectionToken
    let mock_tok = r#"{
        "object": "terminal.connection_token",
        "secret": "pst_test_secret_token_05"
    }"#;
    let (api_base4, h4) = spawn_single_mock_stripe_server("200 OK", mock_tok).await;
    let app4 = build_router(SpecializedState::with_client(build_mock_stripe_client(
        api_base4,
    )));
    let (s4, b4) = send_json_request(app4, Method::POST, "/terminal/token", None).await;
    assert_eq!(s4, StatusCode::CREATED);
    assert_eq!(b4["object"], "terminal.connection_token");
    let _ = h4.await;
}
