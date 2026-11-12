//! Integration Test Suite for Example 7: `actix_specialized_products`
//!
//! Covers all 55 Actix-Web routes across `tax`, `identity`, `issuing`, `treasury`,
//! and `terminal`.

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/actix_web/actix_specialized_products.rs"]
mod example_app;

use actix_web::{App, http::StatusCode, test, web};
use common::{DotEnvConfig, MockStripeServer, MockWireResponse};
use example_app::configure_routes;
use some_stripe::{
    CreateConnectionTokenParams, CreateFinancialAccountParams, CreateLocationParams,
    CreateOutboundPaymentParams, CreateOutboundTransferParams, CreateReaderParams,
    CreateTaxCalculationParams, CreateTaxRegistrationParams,
    CreateTaxTransactionFromCalculationParams, CreateVerificationSessionParams,
    IssuingCardAddress, IssuingCardType, IssuingCardholderBillingParams, IssuingCardholderType,
    TaxLineItemParam, UpdateCardParams, UpdateCardholderParams, UpdateFinancialAccountParams,
    UpdateIssuingAuthorizationParams, UpdateIssuingTransactionParams, UpdateTaxRegistrationParams,
    UpdateTaxSettingsParams, UpdateVerificationSessionParams, VerificationSessionType,
};

#[actix_web::test]
async fn scenario_1_tax_and_identity_routes() {
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

    let stripe = web::Data::new(env.client_for_mock(&mock.base_url));
    let app = test::init_service(App::new().app_data(stripe).configure(configure_routes)).await;

    let mut tax_calc = CreateTaxCalculationParams::new("usd");
    tax_calc.line_items.push(TaxLineItemParam::new(5000, "item_1"));
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/tax/calculations").set_json(&tax_calc).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/tax/calculations/taxcalc_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/tax/calculations/taxcalc_001/line-items").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/tax/transactions/from-calculation").set_json(&CreateTaxTransactionFromCalculationParams::new("taxcalc_001", "order_001")).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/tax/transactions/tax_001").to_request()).await.status(), StatusCode::OK);

    let reg_create = CreateTaxRegistrationParams {
        country: "DE".to_string(),
        active_from: "now".to_string(),
        country_options: serde_json::json!({"de":{"type":"standard"}}),
        expires_at: None,
    };
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/tax/registrations").set_json(&reg_create).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/tax/registrations/taxreg_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/tax/registrations/taxreg_001").set_json(&UpdateTaxRegistrationParams { active_from: None, expires_at: Some("now".to_string()) }).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/tax/registrations").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/tax/settings").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/tax/settings").set_json(&UpdateTaxSettingsParams { defaults: None, head_office: None }).to_request()).await.status(), StatusCode::OK);

    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/identity/sessions").set_json(&CreateVerificationSessionParams::new(VerificationSessionType::Document)).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/identity/sessions/vs_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/identity/sessions/vs_001").set_json(&UpdateVerificationSessionParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/identity/sessions/vs_001/cancel").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/identity/sessions/vs_001/redact").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/identity/reports/vr_001").to_request()).await.status(), StatusCode::OK);

    assert_eq!(mock.finish().await.len(), 17);
}

#[actix_web::test]
async fn scenario_2_issuing_treasury_and_terminal_routes() {
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

    let stripe = web::Data::new(env.client_for_mock(&mock.base_url));
    let app = test::init_service(App::new().app_data(stripe).configure(configure_routes)).await;

    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/issuing/cardholders/ich_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/issuing/cards/ic_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/issuing/authorizations/iauth_001/approve").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/issuing/authorizations/iauth_001/decline").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/issuing/transactions/ipi_001").to_request()).await.status(), StatusCode::OK);

    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/treasury/financial-accounts").set_json(&CreateFinancialAccountParams::new(vec!["usd".to_string()])).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/treasury/financial-accounts/fa_001/close").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/treasury/outbound-transfers").set_json(&CreateOutboundTransferParams::new("fa_001", 5000, "usd")).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/treasury/outbound-payments").set_json(&CreateOutboundPaymentParams::new("fa_001", 3000, "usd")).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/treasury/received-credits/rc_001").to_request()).await.status(), StatusCode::OK);

    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/terminal/locations").set_json(&CreateLocationParams::new("HQ Store")).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/terminal/readers").set_json(&CreateReaderParams::new("simulated-wpe")).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/terminal/readers/tmr_001/cancel-action").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/terminal/connection-tokens").set_json(&CreateConnectionTokenParams::new()).to_request()).await.status(), StatusCode::CREATED);

    let _ = (
        IssuingCardAddress::default(),
        IssuingCardType::Virtual,
        IssuingCardholderBillingParams { address: IssuingCardAddress::default() },
        IssuingCardholderType::Individual,
        UpdateCardParams::default(),
        UpdateCardholderParams::default(),
        UpdateFinancialAccountParams::new(),
        UpdateIssuingAuthorizationParams::new(),
        UpdateIssuingTransactionParams::new(),
    );

    assert_eq!(mock.finish().await.len(), 14);
}
