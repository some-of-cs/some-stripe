//! Integration Test Suite for Example 2: `actix_payments_and_settlement`
//!
//! Covers all 15 Actix-Web routes across `payment_intents`, `refunds`, `disputes`,
//! `balance`, and `balance_transactions`, including `.env` `STRIPE_SECRET_KEY` authentication
//! and PSD2 SCA 3DS2 redirect detection.

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/actix_web/actix_payments_and_settlement.rs"]
mod example_app;

use actix_web::{App, http::StatusCode, test, web};
use common::{DotEnvConfig, MockStripeServer, MockWireResponse};
use example_app::{SubmitDisputeEvidenceRequest, configure_routes};
use some_stripe::{
    CancelPaymentIntentParams, CancelRefundParams, CapturePaymentIntentParams,
    ConfirmPaymentIntentParams, CreatePaymentIntentParams, CreateRefundParams, UpdateRefundParams,
};

#[actix_web::test]
async fn scenario_1_payment_intents_full_lifecycle_and_3ds2_redirect() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(
            200,
            r#"{"id":"pi_test_001","object":"payment_intent","amount":5000,"currency":"usd","status":"requires_confirmation"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"pi_test_001","object":"payment_intent","amount":5000,"currency":"usd","status":"requires_confirmation"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"pi_test_001","object":"payment_intent","amount":5000,"currency":"usd","status":"requires_action"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"pi_test_001","object":"payment_intent","amount":5000,"amount_received":5000,"currency":"usd","status":"succeeded"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"pi_test_001","object":"payment_intent","amount":5000,"currency":"usd","status":"canceled"}"#,
        ),
    ])
    .await;

    let stripe = web::Data::new(env.client_for_mock(&mock.base_url));
    let app = test::init_service(App::new().app_data(stripe).configure(configure_routes)).await;

    // 1. Create PaymentIntent
    let create_req = test::TestRequest::post()
        .uri("/payments/intents")
        .set_json(&CreatePaymentIntentParams::new(5000, "usd"))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    // 2. Retrieve PaymentIntent
    let get_req = test::TestRequest::get()
        .uri("/payments/intents/pi_test_001")
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    // 3. Confirm PaymentIntent (3DS2 requires_action check)
    let confirm_req = test::TestRequest::post()
        .uri("/payments/intents/pi_test_001/confirm")
        .set_json(&ConfirmPaymentIntentParams::new())
        .to_request();
    let confirm_resp = test::call_service(&app, confirm_req).await;
    assert_eq!(confirm_resp.status(), StatusCode::OK);
    let confirm_json: serde_json::Value = test::read_body_json(confirm_resp).await;
    assert_eq!(confirm_json["requires_3ds2_redirect"], true);

    // 4. Capture PaymentIntent
    let cap_req = test::TestRequest::post()
        .uri("/payments/intents/pi_test_001/capture")
        .set_json(&CapturePaymentIntentParams {
            amount_to_capture: Some(5000),
            idempotency_key: Some("idem_cap_1".to_string()),
        })
        .to_request();
    let cap_resp = test::call_service(&app, cap_req).await;
    assert_eq!(cap_resp.status(), StatusCode::OK);

    // 5. Cancel PaymentIntent
    let cancel_req = test::TestRequest::post()
        .uri("/payments/intents/pi_test_001/cancel")
        .set_json(&CancelPaymentIntentParams::new())
        .to_request();
    let cancel_resp = test::call_service(&app, cancel_req).await;
    assert_eq!(cancel_resp.status(), StatusCode::OK);

    let captured = mock.finish().await;
    assert_eq!(captured.len(), 5);
    assert!(captured[0].request_line.starts_with("POST /v1/payment_intents "));
    assert!(
        captured[0]
            .raw_request
            .to_lowercase()
            .contains(&format!("authorization: bearer {}", env.stripe_secret_key.as_str()).to_lowercase())
    );
}

#[actix_web::test]
async fn scenario_2_refunds_and_disputes_lifecycle() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(200, r#"{"id":"re_001","object":"refund","amount":2500,"currency":"usd","status":"succeeded"}"#),
        MockWireResponse::json(200, r#"{"id":"re_001","object":"refund","amount":2500,"currency":"usd","status":"succeeded"}"#),
        MockWireResponse::json(200, r#"{"id":"re_001","object":"refund","amount":2500,"currency":"usd","status":"succeeded"}"#),
        MockWireResponse::json(200, r#"{"id":"re_001","object":"refund","amount":2500,"currency":"usd","status":"canceled"}"#),
        MockWireResponse::json(200, r#"{"id":"dp_001","object":"dispute","amount":2500,"currency":"usd","reason":"fraudulent","status":"needs_response"}"#),
        MockWireResponse::json(200, r#"{"id":"dp_001","object":"dispute","amount":2500,"currency":"usd","reason":"fraudulent","status":"under_review"}"#),
        MockWireResponse::json(200, r#"{"id":"dp_001","object":"dispute","amount":2500,"currency":"usd","reason":"fraudulent","status":"lost"}"#),
    ])
    .await;

    let stripe = web::Data::new(env.client_for_mock(&mock.base_url));
    let app = test::init_service(App::new().app_data(stripe).configure(configure_routes)).await;

    let mut create_refund = CreateRefundParams::new();
    create_refund.payment_intent = Some("pi_test_001".to_string());
    create_refund.amount = Some(2500);

    let r1 = test::call_service(
        &app,
        test::TestRequest::post().uri("/payments/refunds").set_json(&create_refund).to_request(),
    ).await;
    assert_eq!(r1.status(), StatusCode::CREATED);

    let r2 = test::call_service(
        &app,
        test::TestRequest::get().uri("/payments/refunds/re_001").to_request(),
    ).await;
    assert_eq!(r2.status(), StatusCode::OK);

    let r3 = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/payments/refunds/re_001/update")
            .set_json(&UpdateRefundParams::new())
            .to_request(),
    ).await;
    assert_eq!(r3.status(), StatusCode::OK);

    let r4 = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/payments/refunds/re_001/cancel")
            .set_json(&CancelRefundParams::new())
            .to_request(),
    ).await;
    assert_eq!(r4.status(), StatusCode::OK);

    let d1 = test::call_service(
        &app,
        test::TestRequest::get().uri("/payments/disputes/dp_001").to_request(),
    ).await;
    assert_eq!(d1.status(), StatusCode::OK);

    let d2 = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/payments/disputes/dp_001/update")
            .set_json(&SubmitDisputeEvidenceRequest {
                customer_name: Some("Ada Lovelace".to_string()),
                product_description: Some("Cloud GPU Credits".to_string()),
                uncategorized_text: Some("Signed delivery receipt attached".to_string()),
                submit: Some(true),
            })
            .to_request(),
    ).await;
    assert_eq!(d2.status(), StatusCode::OK);

    let d3 = test::call_service(
        &app,
        test::TestRequest::post().uri("/payments/disputes/dp_001/close").to_request(),
    ).await;
    assert_eq!(d3.status(), StatusCode::OK);

    let captured = mock.finish().await;
    assert_eq!(captured.len(), 7);
}

#[actix_web::test]
async fn scenario_3_balance_and_balance_transactions_pagination() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(
            200,
            r#"{"object":"balance","available":[{"amount":120000,"currency":"usd"}],"pending":[{"amount":15000,"currency":"usd"}],"livemode":false}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"txn_001","object":"balance_transaction","amount":5000,"currency":"usd","fee":175,"net":4825,"status":"available","type":"charge"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"object":"list","data":[{"id":"txn_001","object":"balance_transaction","amount":5000,"currency":"usd","fee":175,"net":4825,"status":"available","type":"charge"}],"has_more":false,"url":"/v1/balance_transactions"}"#,
        ),
    ])
    .await;

    let stripe = web::Data::new(env.client_for_mock(&mock.base_url));
    let app = test::init_service(App::new().app_data(stripe).configure(configure_routes)).await;

    let b_resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/payments/balance").to_request(),
    ).await;
    assert_eq!(b_resp.status(), StatusCode::OK);

    let tx_resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/payments/balance-transactions/txn_001").to_request(),
    ).await;
    assert_eq!(tx_resp.status(), StatusCode::OK);

    let list_resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/payments/balance-transactions").to_request(),
    ).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_json: serde_json::Value = test::read_body_json(list_resp).await;
    assert_eq!(list_json.as_array().map(Vec::len), Some(1));

    let captured = mock.finish().await;
    assert_eq!(captured.len(), 3);
}

#[actix_web::test]
async fn scenario_4_live_dotenv_balance_probe_when_enabled() {
    let env = DotEnvConfig::load();
    if !env.should_run_live() {
        return;
    }
    let stripe = web::Data::new(env.live_client());
    let app = test::init_service(App::new().app_data(stripe).configure(configure_routes)).await;

    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/payments/balance").to_request(),
    ).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
