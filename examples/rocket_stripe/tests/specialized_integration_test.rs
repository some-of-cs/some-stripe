//! Integration tests for Specialized Product Namespaces (`/api/tax`, `/api/identity`, `/api/issuing`, `/api/treasury`, `/api/terminal`).
//!
//! Scenarios covered:
//! 1. Stripe Tax settings retrieval (`GET /api/tax/settings`).
//! 2. Stripe Identity Verification Session creation (`POST /api/identity/verification-sessions`)
//!    and retrieval (`GET /api/identity/verification-sessions/<id>`).
//! 3. Stripe Issuing virtual card issuance (`POST /api/issuing/cards/<cardholder_id>`).
//! 4. Stripe Treasury commercial financial account opening (`POST /api/treasury/financial-accounts`).
//! 5. Stripe Terminal POS reader connection token generation (`POST /api/terminal/connection-tokens`).
//! 6. Optional live `.env` sandbox verification when `STRIPE_SECRET_KEY` is provided in `.env`.

mod common;

use common::{
    read_http_request, setup_live_env_rocket_client, setup_mock_rocket_client, write_json_response,
};
use rocket::http::Status;
use serde_json::{Value, json};

#[tokio::test]
async fn test_tax_and_identity_verification_endpoints() {
    let (listener, client) = setup_mock_rocket_client("whsec_test").await;

    let mock_server = tokio::spawn(async move {
        // 1. GET /v1/tax/settings
        let (mut s1, _) = listener.accept().await.unwrap();
        let (h1, _) = read_http_request(&mut s1).await;
        assert!(h1.starts_with("GET /v1/tax/settings HTTP/1.1"));
        write_json_response(
            &mut s1,
            "200 OK",
            &json!({
                "object": "tax.settings",
                "status": "active",
                "defaults": {
                    "tax_behavior": "exclusive",
                    "tax_code": "txcd_10000000"
                },
                "head_office": null,
                "status_details": null,
                "livemode": false
            })
            .to_string(),
        )
        .await;

        // 2. POST /v1/identity/verification_sessions
        let (mut s2, _) = listener.accept().await.unwrap();
        let (h2, b2) = read_http_request(&mut s2).await;
        assert!(h2.starts_with("POST /v1/identity/verification_sessions HTTP/1.1"));
        assert!(b2.contains("type=document"));
        let vs_json = json!({
            "id": "vs_rocket_kyc_1",
            "object": "identity.verification_session",
            "client_secret": "vs_secret_masked",
            "created": 1728345600,
            "livemode": false,
            "status": "requires_input",
            "type": "document",
            "url": "https://verify.stripe.com/start/vs_rocket_kyc_1"
        });
        write_json_response(&mut s2, "200 OK", &vs_json.to_string()).await;

        // 3. GET /v1/identity/verification_sessions/vs_rocket_kyc_1
        let (mut s3, _) = listener.accept().await.unwrap();
        let (h3, _) = read_http_request(&mut s3).await;
        assert!(
            h3.starts_with("GET /v1/identity/verification_sessions/vs_rocket_kyc_1 HTTP/1.1")
        );
        write_json_response(&mut s3, "200 OK", &vs_json.to_string()).await;
    });

    // 1. Tax Settings
    let tax_res = client.get("/api/tax/settings").dispatch().await;
    assert_eq!(tax_res.status(), Status::Ok);
    let tax_json: Value = tax_res.into_json().await.unwrap();
    assert_eq!(tax_json["data"]["status"], "active");

    // 2. Create Identity Verification Session
    let vs_create = client
        .post("/api/identity/verification-sessions")
        .dispatch()
        .await;
    assert_eq!(vs_create.status(), Status::Ok);
    let vs_json: Value = vs_create.into_json().await.unwrap();
    assert_eq!(vs_json["data"]["id"], "vs_rocket_kyc_1");

    // 3. Retrieve Identity Verification Session
    let vs_get = client
        .get("/api/identity/verification-sessions/vs_rocket_kyc_1")
        .dispatch()
        .await;
    assert_eq!(vs_get.status(), Status::Ok);

    mock_server.await.unwrap();
}

#[tokio::test]
async fn test_issuing_treasury_and_terminal_endpoints() {
    let (listener, client) = setup_mock_rocket_client("whsec_test").await;

    let mock_server = tokio::spawn(async move {
        // 1. POST /v1/issuing/cards
        let (mut s1, _) = listener.accept().await.unwrap();
        let (h1, b1) = read_http_request(&mut s1).await;
        assert!(h1.starts_with("POST /v1/issuing/cards HTTP/1.1"));
        assert!(b1.contains("cardholder=ich_rocket_holder_1"));
        assert!(b1.contains("type=virtual"));
        write_json_response(
            &mut s1,
            "200 OK",
            &json!({
                "id": "ic_rocket_card_1",
                "object": "issuing.card",
                "brand": "Visa",
                "cardholder": "ich_rocket_holder_1",
                "created": 1728345600,
                "currency": "usd",
                "exp_month": 12,
                "exp_year": 2028,
                "last4": "4242",
                "livemode": false,
                "status": "active",
                "type": "virtual"
            })
            .to_string(),
        )
        .await;

        // 2. POST /v1/treasury/financial_accounts
        let (mut s2, _) = listener.accept().await.unwrap();
        let (h2, b2) = read_http_request(&mut s2).await;
        assert!(h2.starts_with("POST /v1/treasury/financial_accounts HTTP/1.1"));
        assert!(
            b2.contains("supported_currencies%5B0%5D=usd")
                || b2.contains("supported_currencies[0]=usd")
        );
        write_json_response(
            &mut s2,
            "200 OK",
            &json!({
                "id": "fa_rocket_treasury_1",
                "object": "treasury.financial_account",
                "active_features": ["deposit_insurance"],
                "balance": {
                    "cash": { "usd": 250000 },
                    "inbound_pending": { "usd": 0 },
                    "outbound_pending": { "usd": 0 }
                },
                "country": "US",
                "created": 1728345600,
                "financial_addresses": [],
                "livemode": false,
                "status": "open",
                "supported_currencies": ["usd"]
            })
            .to_string(),
        )
        .await;

        // 3. POST /v1/terminal/connection_tokens
        let (mut s3, _) = listener.accept().await.unwrap();
        let (h3, _) = read_http_request(&mut s3).await;
        assert!(h3.starts_with("POST /v1/terminal/connection_tokens HTTP/1.1"));
        write_json_response(
            &mut s3,
            "200 OK",
            &json!({
                "object": "terminal.connection_token",
                "secret": "pst_test_rocket_reader_secret"
            })
            .to_string(),
        )
        .await;
    });

    // 1. Issue virtual card
    let card_res = client
        .post("/api/issuing/cards/ich_rocket_holder_1")
        .dispatch()
        .await;
    assert_eq!(card_res.status(), Status::Ok);
    let card_json: Value = card_res.into_json().await.unwrap();
    assert_eq!(card_json["data"]["id"], "ic_rocket_card_1");
    assert_eq!(card_json["data"]["last4"], "4242");

    // 2. Open Treasury Financial Account
    let fa_res = client
        .post("/api/treasury/financial-accounts")
        .dispatch()
        .await;
    assert_eq!(fa_res.status(), Status::Ok);
    let fa_json: Value = fa_res.into_json().await.unwrap();
    assert_eq!(fa_json["data"]["id"], "fa_rocket_treasury_1");

    // 3. Create Terminal POS connection token
    let term_res = client
        .post("/api/terminal/connection-tokens")
        .dispatch()
        .await;
    assert_eq!(term_res.status(), Status::Ok);
    let term_json: Value = term_res.into_json().await.unwrap();
    assert_eq!(term_json["data"]["object"], "terminal.connection_token");

    mock_server.await.unwrap();
}

#[tokio::test]
async fn test_live_env_terminal_connection_token_if_configured() {
    let Some(client) = setup_live_env_rocket_client().await else {
        return;
    };

    let term_res = client
        .post("/api/terminal/connection-tokens")
        .dispatch()
        .await;
    assert_eq!(term_res.status(), Status::Ok);
    let term_json: Value = term_res.into_json().await.unwrap();
    assert_eq!(term_json["data"]["object"], "terminal.connection_token");
}
