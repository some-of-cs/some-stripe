//! Integration Test Suite for Example 2: `tonic_payments_and_settlement`
//!
//! Covers all 15 Tonic gRPC methods across `payment_intents`, `refunds`, `disputes`,
//! `balance`, and `balance_transactions`, including `.env` `STRIPE_SECRET_KEY` authentication,
//! PSD2 SCA 3DS2 redirect detection, and `Code::InvalidArgument` / `Code::Unavailable` error handling.

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/tonic_grpc/tonic_payments_and_settlement.rs"]
mod example_grpc;

use common::{DotEnvConfig, MockStripeServer, MockWireResponse};
use example_grpc::{
    JsonPayloadRequest, PaymentsSettlementGrpcServer, PaymentsSettlementGrpcService,
    ResourceIdRequest, SubmitDisputeEvidenceInput,
};
use some_stripe::{
    CancelPaymentIntentParams, CancelRefundParams, CapturePaymentIntentParams,
    ConfirmPaymentIntentParams, CreatePaymentIntentParams, CreateRefundParams, UpdateRefundParams,
};
use tonic::{Code, Request};

/// Verifies `PaymentIntent` create, retrieve, confirm (PSD2 3DS2 redirect detection),
/// capture, cancel, and `.env` Bearer header wire propagation.
#[tokio::test]
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

    let server = PaymentsSettlementGrpcServer {
        stripe: env.client_for_mock(&mock.base_url),
    };

    // 1. Create PaymentIntent
    let create_json = serde_json::to_string(&CreatePaymentIntentParams::new(5000, "usd"))
        .expect("serialize CreatePaymentIntentParams");
    let create_resp = server
        .create_payment_intent(Request::new(JsonPayloadRequest {
            id: String::new(),
            json_params: create_json,
        }))
        .await
        .expect("create_payment_intent should succeed")
        .into_inner();
    assert!(create_resp.json_data.contains("pi_test_001"));
    assert!(!create_resp.requires_3ds2_redirect);

    // 2. Retrieve PaymentIntent
    let get_resp = server
        .retrieve_payment_intent(Request::new(ResourceIdRequest {
            id: "pi_test_001".to_string(),
            limit: None,
        }))
        .await
        .expect("retrieve_payment_intent should succeed")
        .into_inner();
    assert!(get_resp.json_data.contains("requires_confirmation"));

    // 3. Confirm PaymentIntent (3DS2 requires_action check)
    let confirm_json = serde_json::to_string(&ConfirmPaymentIntentParams::new())
        .expect("serialize ConfirmPaymentIntentParams");
    let confirm_resp = server
        .confirm_payment_intent(Request::new(JsonPayloadRequest {
            id: "pi_test_001".to_string(),
            json_params: confirm_json,
        }))
        .await
        .expect("confirm_payment_intent should succeed")
        .into_inner();
    assert!(confirm_resp.requires_3ds2_redirect);

    // 4. Capture PaymentIntent
    let cap_json = serde_json::to_string(&CapturePaymentIntentParams {
        amount_to_capture: Some(5000),
        idempotency_key: Some("idem_cap_1".to_string()),
    })
    .expect("serialize CapturePaymentIntentParams");
    let cap_resp = server
        .capture_payment_intent(Request::new(JsonPayloadRequest {
            id: "pi_test_001".to_string(),
            json_params: cap_json,
        }))
        .await
        .expect("capture_payment_intent should succeed")
        .into_inner();
    assert!(cap_resp.json_data.contains("succeeded"));

    // 5. Cancel PaymentIntent
    let cancel_json = serde_json::to_string(&CancelPaymentIntentParams::new())
        .expect("serialize CancelPaymentIntentParams");
    let cancel_resp = server
        .cancel_payment_intent(Request::new(JsonPayloadRequest {
            id: "pi_test_001".to_string(),
            json_params: cancel_json,
        }))
        .await
        .expect("cancel_payment_intent should succeed")
        .into_inner();
    assert!(cancel_resp.json_data.contains("canceled"));

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

/// Verifies `Refund` and `Dispute` gRPC lifecycle methods.
#[tokio::test]
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

    let server = PaymentsSettlementGrpcServer {
        stripe: env.client_for_mock(&mock.base_url),
    };

    let mut create_refund = CreateRefundParams::new();
    create_refund.payment_intent = Some("pi_test_001".to_string());
    create_refund.amount = Some(2500);

    let r1 = server
        .create_refund(Request::new(JsonPayloadRequest {
            id: String::new(),
            json_params: serde_json::to_string(&create_refund).expect("json"),
        }))
        .await
        .expect("create_refund")
        .into_inner();
    assert!(r1.json_data.contains("re_001"));

    let r2 = server
        .retrieve_refund(Request::new(ResourceIdRequest {
            id: "re_001".to_string(),
            limit: None,
        }))
        .await
        .expect("retrieve_refund")
        .into_inner();
    assert!(r2.json_data.contains("re_001"));

    let r3 = server
        .update_refund(Request::new(JsonPayloadRequest {
            id: "re_001".to_string(),
            json_params: serde_json::to_string(&UpdateRefundParams::new()).expect("json"),
        }))
        .await
        .expect("update_refund")
        .into_inner();
    assert!(r3.json_data.contains("re_001"));

    let r4 = server
        .cancel_refund(Request::new(JsonPayloadRequest {
            id: "re_001".to_string(),
            json_params: serde_json::to_string(&CancelRefundParams::new()).expect("json"),
        }))
        .await
        .expect("cancel_refund")
        .into_inner();
    assert!(r4.json_data.contains("canceled"));

    let d1 = server
        .retrieve_dispute(Request::new(ResourceIdRequest {
            id: "dp_001".to_string(),
            limit: None,
        }))
        .await
        .expect("retrieve_dispute")
        .into_inner();
    assert!(d1.json_data.contains("needs_response"));

    let d2 = server
        .update_dispute(Request::new(JsonPayloadRequest {
            id: "dp_001".to_string(),
            json_params: serde_json::to_string(&SubmitDisputeEvidenceInput {
                customer_name: Some("Ada Lovelace".to_string()),
                product_description: Some("Cloud GPU Credits".to_string()),
                uncategorized_text: Some("Signed delivery receipt attached".to_string()),
                submit: Some(true),
            })
            .expect("json"),
        }))
        .await
        .expect("update_dispute")
        .into_inner();
    assert!(d2.json_data.contains("under_review"));

    let d3 = server
        .close_dispute(Request::new(ResourceIdRequest {
            id: "dp_001".to_string(),
            limit: None,
        }))
        .await
        .expect("close_dispute")
        .into_inner();
    assert!(d3.json_data.contains("lost"));

    assert_eq!(mock.finish().await.len(), 7);
}

/// Verifies `retrieve_balance`, `retrieve_balance_transaction`, `list_balance_transactions`,
/// and invalid JSON / upstream HTTP error handling (`Code::InvalidArgument` & `Code::Unavailable`).
#[tokio::test]
async fn scenario_3_balance_transactions_and_error_handling() {
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
        MockWireResponse::stripe_error(401, "invalid_api_key", "Invalid API Key provided"),
    ])
    .await;

    let server = PaymentsSettlementGrpcServer {
        stripe: env.client_for_mock(&mock.base_url),
    };

    let b_resp = server
        .retrieve_balance(Request::new(ResourceIdRequest {
            id: String::new(),
            limit: None,
        }))
        .await
        .expect("retrieve_balance")
        .into_inner();
    assert!(b_resp.json_data.contains("120000"));

    let tx_resp = server
        .retrieve_balance_transaction(Request::new(ResourceIdRequest {
            id: "txn_001".to_string(),
            limit: None,
        }))
        .await
        .expect("retrieve_balance_transaction")
        .into_inner();
    assert!(tx_resp.json_data.contains("4825"));

    let list_resp = server
        .list_balance_transactions(Request::new(ResourceIdRequest {
            id: String::new(),
            limit: Some(5),
        }))
        .await
        .expect("list_balance_transactions")
        .into_inner();
    let parsed_list: serde_json::Value =
        serde_json::from_str(&list_resp.json_data).expect("valid list json");
    assert_eq!(parsed_list.as_array().map(Vec::len), Some(1));

    // Invalid JSON input -> Code::InvalidArgument
    let bad_json_err = server
        .create_payment_intent(Request::new(JsonPayloadRequest {
            id: String::new(),
            json_params: "{malformed_json".to_string(),
        }))
        .await
        .expect_err("malformed json must return InvalidArgument");
    assert_eq!(bad_json_err.code(), Code::InvalidArgument);

    // Simulated 401 upstream error -> Code::Unavailable
    let api_err = server
        .retrieve_balance(Request::new(ResourceIdRequest {
            id: String::new(),
            limit: None,
        }))
        .await
        .expect_err("401 upstream response must fail");
    assert_eq!(api_err.code(), Code::Unavailable);

    assert_eq!(mock.finish().await.len(), 4);
}

/// Optional live `.env` balance probe when `STRIPE_RUN_LIVE_TESTS=1`.
#[tokio::test]
async fn scenario_4_live_dotenv_balance_probe_when_enabled() {
    let env = DotEnvConfig::load();
    if !env.should_run_live() {
        return;
    }
    let server = PaymentsSettlementGrpcServer::from_env();
    let resp = server
        .retrieve_balance(Request::new(ResourceIdRequest {
            id: String::new(),
            limit: None,
        }))
        .await
        .expect("live retrieve_balance should succeed")
        .into_inner();
    assert!(resp.json_data.contains("available"));
}
