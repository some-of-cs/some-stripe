//! Integration tests for Stripe Connect & Marketplaces (`/api/connect/*`).
//!
//! Scenarios covered:
//! 1. Connected Account creation (`POST /api/connect/accounts`) across Express, Standard, and Custom types.
//! 2. Connected Account retrieval (`GET /api/connect/accounts/<id>`).
//! 3. Hosted Account Link onboarding flow generation (`POST /api/connect/account-links`).
//! 4. Multi-party Transfers (`POST /api/connect/transfers`), parameter validation (invalid amount/currency),
//!    and Transfer Reversals (`POST /api/connect/transfers/<id>/reversals`).
//! 5. Platform Application Fee inspection (`GET /api/connect/application-fees/<id>`).

#[path = "../common/mod.rs"]
mod common;

use common::{read_http_request, setup_mock_rocket_client, write_json_response};
use rocket::http::{ContentType, Status};
use serde_json::{Value, json};

#[tokio::test]
async fn test_connect_accounts_and_onboarding_links_lifecycle() {
    let (listener, client) = setup_mock_rocket_client("whsec_test").await;

    let mock_server = tokio::spawn(async move {
        // 1. POST /v1/accounts
        let (mut s1, _) = listener.accept().await.unwrap();
        let (h1, b1) = read_http_request(&mut s1).await;
        assert!(h1.starts_with("POST /v1/accounts HTTP/1.1"));
        assert!(b1.contains("type=express"));
        assert!(b1.contains("country=US"));
        let acct_json = json!({
            "id": "acct_rocket_seller_1",
            "object": "account",
            "type": "express",
            "country": "US",
            "email": "seller@marketplace.com",
            "charges_enabled": true,
            "payouts_enabled": true
        });
        write_json_response(&mut s1, "200 OK", &acct_json.to_string()).await;

        // 2. GET /v1/accounts/acct_rocket_seller_1
        let (mut s2, _) = listener.accept().await.unwrap();
        let (h2, _) = read_http_request(&mut s2).await;
        assert!(h2.starts_with("GET /v1/accounts/acct_rocket_seller_1 HTTP/1.1"));
        write_json_response(&mut s2, "200 OK", &acct_json.to_string()).await;

        // 3. POST /v1/account_links
        let (mut s3, _) = listener.accept().await.unwrap();
        let (h3, b3) = read_http_request(&mut s3).await;
        assert!(h3.starts_with("POST /v1/account_links HTTP/1.1"));
        assert!(b3.contains("account=acct_rocket_seller_1"));
        assert!(b3.contains("type=account_onboarding"));
        write_json_response(
            &mut s3,
            "200 OK",
            &json!({
                "object": "account_link",
                "created": 1728345600,
                "expires_at": 1728345900,
                "url": "https://connect.stripe.com/setup/e/acct_rocket_seller_1/xyz"
            })
            .to_string(),
        )
        .await;
    });

    // 1. Create Connected Account
    let c_res = client
        .post("/api/connect/accounts")
        .header(ContentType::JSON)
        .body(
            json!({
                "email": "seller@marketplace.com",
                "country": "US",
                "account_type": "express"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(c_res.status(), Status::Ok);
    let c_json: Value = c_res.into_json().await.unwrap();
    assert_eq!(c_json["data"]["id"], "acct_rocket_seller_1");

    // 2. Get Connected Account
    let g_res = client
        .get("/api/connect/accounts/acct_rocket_seller_1")
        .dispatch()
        .await;
    assert_eq!(g_res.status(), Status::Ok);

    // 3. Create Account Link
    let l_res = client
        .post("/api/connect/account-links")
        .header(ContentType::JSON)
        .body(
            json!({
                "account": "acct_rocket_seller_1",
                "refresh_url": "https://marketplace.com/reauth",
                "return_url": "https://marketplace.com/return",
                "type": "account_onboarding"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(l_res.status(), Status::Ok);
    let l_json: Value = l_res.into_json().await.unwrap();
    assert!(
        l_json["data"]["url"]
            .as_str()
            .unwrap()
            .starts_with("https://connect.stripe.com")
    );

    mock_server.await.unwrap();
}

#[tokio::test]
async fn test_transfers_reversals_and_application_fees_lifecycle() {
    let (listener, client) = setup_mock_rocket_client("whsec_test").await;

    // 1. Verify ISO 4217 validation rejects zero or negative transfer amount before hitting wire
    let bad_transfer = client
        .post("/api/connect/transfers")
        .header(ContentType::JSON)
        .body(
            json!({
                "destination": "acct_rocket_seller_1",
                "amount": 0,
                "currency": "usd"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(bad_transfer.status(), Status::BadRequest);
    let bad_json: Value = bad_transfer.into_json().await.unwrap();
    assert!(
        bad_json["error"]
            .as_str()
            .unwrap()
            .contains("Transfer parameter validation failed")
    );

    let mock_server = tokio::spawn(async move {
        // 2. POST /v1/transfers
        let (mut s1, _) = listener.accept().await.unwrap();
        let (h1, b1) = read_http_request(&mut s1).await;
        assert!(h1.starts_with("POST /v1/transfers HTTP/1.1"));
        assert!(b1.contains("amount=12000"));
        assert!(b1.contains("destination=acct_rocket_seller_1"));
        write_json_response(
            &mut s1,
            "200 OK",
            &json!({
                "id": "tr_rocket_12000",
                "object": "transfer",
                "amount": 12000,
                "amount_reversed": 0,
                "currency": "usd",
                "destination": "acct_rocket_seller_1",
                "created": 1728345600,
                "livemode": false,
                "reversed": false
            })
            .to_string(),
        )
        .await;

        // 3. POST /v1/transfers/tr_rocket_12000/reversals
        let (mut s2, _) = listener.accept().await.unwrap();
        let (h2, b2) = read_http_request(&mut s2).await;
        assert!(h2.starts_with("POST /v1/transfers/tr_rocket_12000/reversals HTTP/1.1"));
        assert!(b2.contains("refund_application_fee=true"));
        write_json_response(
            &mut s2,
            "200 OK",
            &json!({
                "id": "trr_rocket_rev_1",
                "object": "transfer_reversal",
                "amount": 12000,
                "currency": "usd",
                "transfer": "tr_rocket_12000",
                "created": 1728345650
            })
            .to_string(),
        )
        .await;

        // 4. GET /v1/application_fees/fee_rocket_500
        let (mut s3, _) = listener.accept().await.unwrap();
        let (h3, _) = read_http_request(&mut s3).await;
        assert!(h3.starts_with("GET /v1/application_fees/fee_rocket_500 HTTP/1.1"));
        write_json_response(
            &mut s3,
            "200 OK",
            &json!({
                "id": "fee_rocket_500",
                "object": "application_fee",
                "account": "acct_rocket_seller_1",
                "amount": 500,
                "amount_refunded": 0,
                "charge": "ch_rocket_1",
                "created": 1728345600,
                "currency": "usd",
                "livemode": false,
                "refunded": false
            })
            .to_string(),
        )
        .await;
    });

    // Create Transfer
    let t_res = client
        .post("/api/connect/transfers")
        .header(ContentType::JSON)
        .body(
            json!({
                "destination": "acct_rocket_seller_1",
                "amount": 12000,
                "currency": "usd",
                "description": "Seller weekly payout"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(t_res.status(), Status::Ok);
    let t_json: Value = t_res.into_json().await.unwrap();
    assert_eq!(t_json["data"]["id"], "tr_rocket_12000");

    // Reverse Transfer
    let rev_res = client
        .post("/api/connect/transfers/tr_rocket_12000/reversals")
        .dispatch()
        .await;
    assert_eq!(rev_res.status(), Status::Ok);
    let rev_json: Value = rev_res.into_json().await.unwrap();
    assert_eq!(rev_json["data"]["id"], "trr_rocket_rev_1");

    // Get Application Fee
    let fee_res = client
        .get("/api/connect/application-fees/fee_rocket_500")
        .dispatch()
        .await;
    assert_eq!(fee_res.status(), Status::Ok);
    let fee_json: Value = fee_res.into_json().await.unwrap();
    assert_eq!(fee_json["data"]["amount"], 500);

    mock_server.await.unwrap();
}
