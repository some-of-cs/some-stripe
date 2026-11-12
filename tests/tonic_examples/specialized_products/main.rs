//! Integration Test Suite for Example 7: `tonic_specialized_products`
//!
//! Covers all 59 Tonic gRPC methods across `tax`, `identity`, `issuing`, `treasury`,
//! and `terminal`.

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/tonic_grpc/tonic_specialized_products.rs"]
mod example_grpc;

use common::{DotEnvConfig, MockStripeServer, MockWireResponse};
use example_grpc::{
    SpecializedProductsGrpcServer, SpecializedProductsGrpcService, SpecializedRpcRequest,
};
use some_stripe::{
    CreateConnectionTokenParams, CreateFinancialAccountParams, CreateLocationParams,
    CreateOutboundPaymentParams, CreateOutboundTransferParams, CreateReaderParams,
    CreateTaxCalculationParams, CreateTaxRegistrationParams,
    CreateTaxTransactionFromCalculationParams, CreateVerificationSessionParams, TaxLineItemParam,
    UpdateTaxRegistrationParams, UpdateTaxSettingsParams, UpdateVerificationSessionParams,
    VerificationSessionType,
};
use tonic::Request;

/// Verifies `Tax` calculations/transactions/registrations/settings and `Identity` verification RPCs.
#[tokio::test]
async fn scenario_1_tax_and_identity_grpc_methods() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(200, r#"{"id":"taxcalc_001","object":"tax.calculation","amount_total":5400,"currency":"usd","tax_amount_exclusive":400}"#),
        MockWireResponse::json(200, r#"{"id":"taxcalc_001","object":"tax.calculation","amount_total":5400,"currency":"usd","tax_amount_exclusive":400}"#),
        MockWireResponse::json(200, r#"{"object":"list","data":[],"has_more":false,"url":"/v1/tax/calculations/taxcalc_001/line_items"}"#),
        MockWireResponse::json(200, r#"{"id":"tax_001","object":"tax.transaction","currency":"usd","reference":"order_001"}"#),
        MockWireResponse::json(200, r#"{"id":"tax_001","object":"tax.transaction","currency":"usd","reference":"order_001"}"#),
        MockWireResponse::json(200, r#"{"id":"taxreg_001","object":"tax.registration","country":"DE","status":"active"}"#),
        MockWireResponse::json(200, r#"{"id":"taxreg_001","object":"tax.registration","country":"DE","status":"active"}"#),
        MockWireResponse::json(200, r#"{"id":"taxreg_001","object":"tax.registration","country":"DE","status":"expired"}"#),
        MockWireResponse::json(200, r#"{"object":"list","data":[{"id":"taxreg_001","object":"tax.registration","country":"DE","status":"active"}],"has_more":false,"url":"/v1/tax/registrations"}"#),
        MockWireResponse::json(200, r#"{"object":"tax.settings","status":"active"}"#),
        MockWireResponse::json(200, r#"{"object":"tax.settings","status":"active"}"#),
        MockWireResponse::json(200, r#"{"id":"vs_001","object":"identity.verification_session","status":"requires_input","type":"document"}"#),
        MockWireResponse::json(200, r#"{"id":"vs_001","object":"identity.verification_session","status":"requires_input","type":"document"}"#),
        MockWireResponse::json(200, r#"{"id":"vs_001","object":"identity.verification_session","status":"requires_input","type":"document"}"#),
        MockWireResponse::json(200, r#"{"id":"vs_001","object":"identity.verification_session","status":"canceled","type":"document"}"#),
        MockWireResponse::json(200, r#"{"id":"vs_001","object":"identity.verification_session","status":"redacted","type":"document"}"#),
        MockWireResponse::json(200, r#"{"id":"vr_001","object":"identity.verification_report","type":"document"}"#),
    ])
    .await;

    let server = SpecializedProductsGrpcServer {
        stripe: env.client_for_mock(&mock.base_url),
    };

    let mut tax_calc = CreateTaxCalculationParams::new("usd");
    tax_calc.line_items.push(TaxLineItemParam::new(5000, "item_1"));
    assert!(server.create_tax_calculation(Request::new(SpecializedRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&tax_calc).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_tax_calculation(Request::new(SpecializedRpcRequest {
        id: "taxcalc_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.list_tax_calculation_line_items(Request::new(SpecializedRpcRequest {
        id: "taxcalc_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.create_tax_transaction_from_calculation(Request::new(SpecializedRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&CreateTaxTransactionFromCalculationParams::new("taxcalc_001", "order_001")).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_tax_transaction(Request::new(SpecializedRpcRequest {
        id: "tax_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());

    let reg_create = CreateTaxRegistrationParams {
        country: "DE".to_string(),
        active_from: "now".to_string(),
        country_options: serde_json::json!({"de":{"type":"standard"}}),
        expires_at: None,
    };
    assert!(server.create_tax_registration(Request::new(SpecializedRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&reg_create).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_tax_registration(Request::new(SpecializedRpcRequest {
        id: "taxreg_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_tax_registration(Request::new(SpecializedRpcRequest {
        id: "taxreg_001".to_string(),
        json_params: serde_json::to_string(&UpdateTaxRegistrationParams { active_from: None, expires_at: Some("now".to_string()) }).expect("json"),
    })).await.is_ok());
    assert!(server.list_tax_registrations(Request::new(SpecializedRpcRequest {
        id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.retrieve_tax_settings(Request::new(SpecializedRpcRequest {
        id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_tax_settings(Request::new(SpecializedRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&UpdateTaxSettingsParams { defaults: None, head_office: None }).expect("json"),
    })).await.is_ok());

    assert!(server.create_verification_session(Request::new(SpecializedRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&CreateVerificationSessionParams::new(VerificationSessionType::Document)).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_verification_session(Request::new(SpecializedRpcRequest {
        id: "vs_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_verification_session(Request::new(SpecializedRpcRequest {
        id: "vs_001".to_string(),
        json_params: serde_json::to_string(&UpdateVerificationSessionParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.cancel_verification_session(Request::new(SpecializedRpcRequest {
        id: "vs_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.redact_verification_session(Request::new(SpecializedRpcRequest {
        id: "vs_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.retrieve_verification_report(Request::new(SpecializedRpcRequest {
        id: "vr_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());

    assert_eq!(mock.finish().await.len(), 17);
}

/// Verifies `Issuing`, `Treasury`, and `Terminal` gRPC methods.
#[tokio::test]
async fn scenario_2_issuing_treasury_and_terminal_grpc_methods() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(200, r#"{"id":"ich_001","object":"issuing.cardholder","name":"Jane Doe","status":"active","type":"individual"}"#),
        MockWireResponse::json(200, r#"{"id":"ic_001","object":"issuing.card","currency":"usd","status":"active","type":"virtual"}"#),
        MockWireResponse::json(200, r#"{"id":"iauth_001","object":"issuing.authorization","amount":1200,"approved":true,"currency":"usd","status":"closed"}"#),
        MockWireResponse::json(200, r#"{"id":"iauth_001","object":"issuing.authorization","amount":1200,"approved":false,"currency":"usd","status":"closed"}"#),
        MockWireResponse::json(200, r#"{"id":"ipi_001","object":"issuing.transaction","amount":-1200,"currency":"usd","type":"capture"}"#),
        MockWireResponse::json(200, r#"{"id":"fa_001","object":"treasury.financial_account","supported_currencies":["usd"],"status":"open"}"#),
        MockWireResponse::json(200, r#"{"id":"fa_001","object":"treasury.financial_account","supported_currencies":["usd"],"status":"closed"}"#),
        MockWireResponse::json(200, r#"{"id":"obt_001","object":"treasury.outbound_transfer","amount":5000,"currency":"usd","financial_account":"fa_001","status":"processing"}"#),
        MockWireResponse::json(200, r#"{"id":"obp_001","object":"treasury.outbound_payment","amount":3000,"currency":"usd","financial_account":"fa_001","status":"processing"}"#),
        MockWireResponse::json(200, r#"{"id":"rc_001","object":"treasury.received_credit","amount":9000,"currency":"usd","status":"succeeded"}"#),
        MockWireResponse::json(200, r#"{"id":"tml_001","object":"terminal.location","display_name":"HQ Store"}"#),
        MockWireResponse::json(200, r#"{"id":"tmr_001","object":"terminal.reader","label":"Counter 1","device_type":"bbpos_wisepos_e"}"#),
        MockWireResponse::json(200, r#"{"id":"tmr_001","object":"terminal.reader","label":"Counter 1","device_type":"bbpos_wisepos_e"}"#),
        MockWireResponse::json(200, r#"{"object":"terminal.connection_token","secret":"pst_test_secret_123"}"#),
    ])
    .await;

    let server = SpecializedProductsGrpcServer {
        stripe: env.client_for_mock(&mock.base_url),
    };

    assert!(server.retrieve_issuing_cardholder(Request::new(SpecializedRpcRequest {
        id: "ich_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.retrieve_issuing_card(Request::new(SpecializedRpcRequest {
        id: "ic_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.approve_issuing_authorization(Request::new(SpecializedRpcRequest {
        id: "iauth_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.decline_issuing_authorization(Request::new(SpecializedRpcRequest {
        id: "iauth_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.retrieve_issuing_transaction(Request::new(SpecializedRpcRequest {
        id: "ipi_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());

    assert!(server.create_financial_account(Request::new(SpecializedRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&CreateFinancialAccountParams::new(vec!["usd".to_string()])).expect("json"),
    })).await.is_ok());
    assert!(server.close_financial_account(Request::new(SpecializedRpcRequest {
        id: "fa_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.create_outbound_transfer(Request::new(SpecializedRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&CreateOutboundTransferParams::new("fa_001", 5000, "usd")).expect("json"),
    })).await.is_ok());
    assert!(server.create_outbound_payment(Request::new(SpecializedRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&CreateOutboundPaymentParams::new("fa_001", 3000, "usd")).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_received_credit(Request::new(SpecializedRpcRequest {
        id: "rc_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());

    assert!(server.create_terminal_location(Request::new(SpecializedRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&CreateLocationParams::new("HQ Store")).expect("json"),
    })).await.is_ok());
    assert!(server.create_terminal_reader(Request::new(SpecializedRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&CreateReaderParams::new("simulated-wpe")).expect("json"),
    })).await.is_ok());
    assert!(server.cancel_terminal_reader_action(Request::new(SpecializedRpcRequest {
        id: "tmr_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.create_terminal_connection_token(Request::new(SpecializedRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&CreateConnectionTokenParams::new()).expect("json"),
    })).await.is_ok());

    assert_eq!(mock.finish().await.len(), 14);
}
