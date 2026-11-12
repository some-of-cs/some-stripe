//! Integration tests for PaymentIntents, Refunds, Disputes, and Balance Ledger (`/api/payments/*`).
//!
//! Scenarios covered:
//! 1. PaymentIntent full lifecycle: create (`POST /api/payments/intents`), retrieve (`GET`),
//!    confirm (`POST /confirm`), capture (`POST /capture`), and cancel (`POST /cancel`).
//! 2. Refunds creation (`POST /api/payments/refunds`) and retrieval (`GET /api/payments/refunds/<id>`).
//! 3. Disputes retrieval (`GET /api/payments/disputes/<id>`) and closure (`POST /api/payments/disputes/<id>/close`).
//! 4. Balance ledger (`GET /api/payments/balance`) and Balance Transaction (`GET /api/payments/balance/transactions/<id>`).
//! 5. Optional live `.env` sandbox verification when `STRIPE_SECRET_KEY` is provided in `.env`.

#[path = "../common/mod.rs"]
mod common;

use common::{
    read_http_request, setup_live_env_rocket_client, setup_mock_rocket_client, write_json_response,
};
use rocket::http::{ContentType, Status};
use serde_json::{Value, json};

#[tokio::test]
async fn test_payment_intent_full_lifecycle() {
    let (listener, client) = setup_mock_rocket_client("whsec_test").await;

    let mock_server = tokio::spawn(async move {
        // 1. Create PaymentIntent
        let (mut s1, _) = listener.accept().await.unwrap();
        let (h1, b1) = read_http_request(&mut s1).await;
        assert!(h1.starts_with("POST /v1/payment_intents HTTP/1.1"));
        assert!(b1.contains("amount=7500"));
        assert!(b1.contains("currency=usd"));
        assert!(b1.contains("capture_method=manual"));
        write_json_response(
            &mut s1,
            "200 OK",
            &json!({
                "id": "pi_rocket_7500",
                "object": "payment_intent",
                "amount": 7500,
                "currency": "usd",
                "status": "requires_payment_method"
            })
            .to_string(),
        )
        .await;

        // 2. Retrieve PaymentIntent
        let (mut s2, _) = listener.accept().await.unwrap();
        let (h2, _) = read_http_request(&mut s2).await;
        assert!(h2.starts_with("GET /v1/payment_intents/pi_rocket_7500 HTTP/1.1"));
        write_json_response(
            &mut s2,
            "200 OK",
            &json!({
                "id": "pi_rocket_7500",
                "object": "payment_intent",
                "amount": 7500,
                "currency": "usd",
                "status": "requires_confirmation"
            })
            .to_string(),
        )
        .await;

        // 3. Confirm PaymentIntent
        let (mut s3, _) = listener.accept().await.unwrap();
        let (h3, b3) = read_http_request(&mut s3).await;
        assert!(h3.starts_with("POST /v1/payment_intents/pi_rocket_7500/confirm HTTP/1.1"));
        assert!(b3.contains("payment_method=pm_card_visa"));
        write_json_response(
            &mut s3,
            "200 OK",
            &json!({
                "id": "pi_rocket_7500",
                "object": "payment_intent",
                "amount": 7500,
                "currency": "usd",
                "status": "requires_capture"
            })
            .to_string(),
        )
        .await;

        // 4. Capture PaymentIntent
        let (mut s4, _) = listener.accept().await.unwrap();
        let (h4, _) = read_http_request(&mut s4).await;
        assert!(h4.starts_with("POST /v1/payment_intents/pi_rocket_7500/capture HTTP/1.1"));
        write_json_response(
            &mut s4,
            "200 OK",
            &json!({
                "id": "pi_rocket_7500",
                "object": "payment_intent",
                "amount": 7500,
                "amount_received": 7500,
                "currency": "usd",
                "status": "succeeded"
            })
            .to_string(),
        )
        .await;

        // 5. Cancel another PaymentIntent
        let (mut s5, _) = listener.accept().await.unwrap();
        let (h5, b5) = read_http_request(&mut s5).await;
        assert!(h5.starts_with("POST /v1/payment_intents/pi_rocket_cancel/cancel HTTP/1.1"));
        assert!(b5.contains("cancellation_reason=requested_by_customer"));
        write_json_response(
            &mut s5,
            "200 OK",
            &json!({
                "id": "pi_rocket_cancel",
                "object": "payment_intent",
                "amount": 3000,
                "currency": "usd",
                "status": "canceled"
            })
            .to_string(),
        )
        .await;
    });

    // Step 1: Create
    let res1 = client
        .post("/api/payments/intents")
        .header(ContentType::JSON)
        .body(
            json!({
                "amount": 7500,
                "currency": "usd",
                "capture_method": "manual",
                "description": "Manual auth order"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(res1.status(), Status::Ok);
    let j1: Value = res1.into_json().await.unwrap();
    assert_eq!(j1["data"]["id"], "pi_rocket_7500");

    // Step 2: Retrieve
    let res2 = client
        .get("/api/payments/intents/pi_rocket_7500")
        .dispatch()
        .await;
    assert_eq!(res2.status(), Status::Ok);

    // Step 3: Confirm
    let res3 = client
        .post("/api/payments/intents/pi_rocket_7500/confirm")
        .header(ContentType::JSON)
        .body(
            json!({
                "payment_method": "pm_card_visa",
                "return_url": "https://example.com/3ds-return"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(res3.status(), Status::Ok);
    let j3: Value = res3.into_json().await.unwrap();
    assert_eq!(j3["data"]["status"], "requires_capture");

    // Step 4: Capture
    let res4 = client
        .post("/api/payments/intents/pi_rocket_7500/capture")
        .dispatch()
        .await;
    assert_eq!(res4.status(), Status::Ok);
    let j4: Value = res4.into_json().await.unwrap();
    assert_eq!(j4["data"]["status"], "succeeded");

    // Step 5: Cancel
    let res5 = client
        .post("/api/payments/intents/pi_rocket_cancel/cancel")
        .dispatch()
        .await;
    assert_eq!(res5.status(), Status::Ok);
    let j5: Value = res5.into_json().await.unwrap();
    assert_eq!(j5["data"]["status"], "canceled");

    mock_server.await.unwrap();
}

#[tokio::test]
async fn test_refunds_disputes_and_balance_ledger_endpoints() {
    let (listener, client) = setup_mock_rocket_client("whsec_test").await;

    let mock_server = tokio::spawn(async move {
        // 1. POST /v1/refunds
        let (mut s1, _) = listener.accept().await.unwrap();
        let (h1, b1) = read_http_request(&mut s1).await;
        assert!(h1.starts_with("POST /v1/refunds HTTP/1.1"));
        assert!(b1.contains("payment_intent=pi_rocket_7500"));
        assert!(b1.contains("amount=2500"));
        let refund_body = json!({
            "id": "re_rocket_2500",
            "object": "refund",
            "amount": 2500,
            "currency": "usd",
            "payment_intent": "pi_rocket_7500",
            "status": "succeeded"
        });
        write_json_response(&mut s1, "200 OK", &refund_body.to_string()).await;

        // 2. GET /v1/refunds/re_rocket_2500
        let (mut s2, _) = listener.accept().await.unwrap();
        let (h2, _) = read_http_request(&mut s2).await;
        assert!(h2.starts_with("GET /v1/refunds/re_rocket_2500 HTTP/1.1"));
        write_json_response(&mut s2, "200 OK", &refund_body.to_string()).await;

        // 3. GET /v1/disputes/dp_rocket_1
        let (mut s3, _) = listener.accept().await.unwrap();
        let (h3, _) = read_http_request(&mut s3).await;
        assert!(h3.starts_with("GET /v1/disputes/dp_rocket_1 HTTP/1.1"));
        write_json_response(
            &mut s3,
            "200 OK",
            &json!({
                "id": "dp_rocket_1",
                "object": "dispute",
                "amount": 5000,
                "currency": "usd",
                "reason": "fraudulent",
                "status": "needs_response"
            })
            .to_string(),
        )
        .await;

        // 4. POST /v1/disputes/dp_rocket_1/close
        let (mut s4, _) = listener.accept().await.unwrap();
        let (h4, _) = read_http_request(&mut s4).await;
        assert!(h4.starts_with("POST /v1/disputes/dp_rocket_1/close HTTP/1.1"));
        write_json_response(
            &mut s4,
            "200 OK",
            &json!({
                "id": "dp_rocket_1",
                "object": "dispute",
                "amount": 5000,
                "currency": "usd",
                "reason": "fraudulent",
                "status": "lost"
            })
            .to_string(),
        )
        .await;

        // 5. GET /v1/balance
        let (mut s5, _) = listener.accept().await.unwrap();
        let (h5, _) = read_http_request(&mut s5).await;
        assert!(h5.starts_with("GET /v1/balance HTTP/1.1"));
        write_json_response(
            &mut s5,
            "200 OK",
            &json!({
                "object": "balance",
                "available": [{ "amount": 125000, "currency": "usd" }],
                "pending": [{ "amount": 15000, "currency": "usd" }],
                "livemode": false
            })
            .to_string(),
        )
        .await;

        // 6. GET /v1/balance_transactions/txn_rocket_1
        let (mut s6, _) = listener.accept().await.unwrap();
        let (h6, _) = read_http_request(&mut s6).await;
        assert!(h6.starts_with("GET /v1/balance_transactions/txn_rocket_1 HTTP/1.1"));
        write_json_response(
            &mut s6,
            "200 OK",
            &json!({
                "id": "txn_rocket_1",
                "object": "balance_transaction",
                "amount": 10000,
                "fee": 320,
                "net": 9680,
                "currency": "usd",
                "type": "charge",
                "status": "available"
            })
            .to_string(),
        )
        .await;
    });

    // Refund create & get
    let ref_create = client
        .post("/api/payments/refunds")
        .header(ContentType::JSON)
        .body(
            json!({
                "payment_intent": "pi_rocket_7500",
                "amount": 2500,
                "reason": "requested_by_customer"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(ref_create.status(), Status::Ok);

    let ref_get = client
        .get("/api/payments/refunds/re_rocket_2500")
        .dispatch()
        .await;
    assert_eq!(ref_get.status(), Status::Ok);

    // Dispute get & close
    let dp_get = client
        .get("/api/payments/disputes/dp_rocket_1")
        .dispatch()
        .await;
    assert_eq!(dp_get.status(), Status::Ok);

    let dp_close = client
        .post("/api/payments/disputes/dp_rocket_1/close")
        .dispatch()
        .await;
    assert_eq!(dp_close.status(), Status::Ok);
    let dp_json: Value = dp_close.into_json().await.unwrap();
    assert_eq!(dp_json["data"]["status"], "lost");

    // Balance & BalanceTransaction
    let bal_res = client.get("/api/payments/balance").dispatch().await;
    assert_eq!(bal_res.status(), Status::Ok);
    let bal_json: Value = bal_res.into_json().await.unwrap();
    assert_eq!(bal_json["data"]["available"][0]["amount"], 125000);

    let txn_res = client
        .get("/api/payments/balance/transactions/txn_rocket_1")
        .dispatch()
        .await;
    assert_eq!(txn_res.status(), Status::Ok);
    let txn_json: Value = txn_res.into_json().await.unwrap();
    assert_eq!(txn_json["data"]["net"], 9680);

    mock_server.await.unwrap();
}

#[tokio::test]
async fn test_live_env_payments_and_balance_if_configured() {
    let Some(client) = setup_live_env_rocket_client().await else {
        return;
    };

    // 1. Live Balance check
    let bal_res = client.get("/api/payments/balance").dispatch().await;
    assert_eq!(bal_res.status(), Status::Ok);

    // 2. Live PaymentIntent create & cancel
    let pi_res = client
        .post("/api/payments/intents")
        .header(ContentType::JSON)
        .body(
            json!({
                "amount": 2000,
                "currency": "usd",
                "description": "Rocket live .env test"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(pi_res.status(), Status::Ok);
    let pi_json: Value = pi_res.into_json().await.unwrap();
    let pi_id = pi_json["data"]["id"].as_str().unwrap();

    let cancel_res = client
        .post(format!("/api/payments/intents/{pi_id}/cancel"))
        .dispatch()
        .await;
    assert_eq!(cancel_res.status(), Status::Ok);
}
