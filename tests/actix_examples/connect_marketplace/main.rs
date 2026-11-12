//! Integration Test Suite for Example 6: `actix_connect_marketplace`
//!
//! Covers all 31 Actix-Web routes across `accounts`, `account_links`, `account_sessions`,
//! `persons`, `transfers`, `application_fees`, and `payouts`.

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/actix_web/actix_connect_marketplace.rs"]
mod example_app;

use actix_web::{App, http::StatusCode, test, web};
use common::{DotEnvConfig, MockStripeServer, MockWireResponse};
use example_app::configure_routes;
use some_stripe::{
    AccountLinkType, AccountSessionComponents, CreateAccountLinkParams, CreateAccountParams,
    CreateAccountSessionParams, CreateFeeRefundParams, CreatePayoutParams, CreatePersonParams,
    CreateTransferParams, CreateTransferReversalParams, RejectAccountParams, UpdateAccountParams,
    UpdatePayoutParams, UpdatePersonParams, UpdateTransferParams,
};

#[actix_web::test]
async fn scenario_1_connect_accounts_links_sessions_and_persons() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(200, r#"{"id":"acct_001","object":"account","charges_enabled":true}"#),
        MockWireResponse::json(200, r#"{"id":"acct_platform","object":"account","charges_enabled":true}"#),
        MockWireResponse::json(200, r#"{"id":"acct_001","object":"account","charges_enabled":true}"#),
        MockWireResponse::json(200, r#"{"id":"acct_001","object":"account","charges_enabled":true}"#),
        MockWireResponse::json(200, r#"{"id":"acct_001","object":"account","deleted":true}"#),
        MockWireResponse::json(200, r#"{"id":"acct_001","object":"account","charges_enabled":false}"#),
        MockWireResponse::json(200, r#"{"object":"list","data":[{"id":"acct_001","object":"account"}],"has_more":false,"url":"/v1/accounts"}"#),
        MockWireResponse::json(200, r#"{"object":"account_link","created":1728000000,"expires_at":1728000300,"url":"https://connect.stripe.com/setup/s/acct_001"}"#),
        MockWireResponse::json(200, r#"{"object":"account_session","account":"acct_001","client_secret":"accs_secret_123","expires_at":1728000300,"livemode":false}"#),
        MockWireResponse::json(200, r#"{"id":"person_001","object":"person","account":"acct_001","first_name":"Grace"}"#),
        MockWireResponse::json(200, r#"{"id":"person_001","object":"person","account":"acct_001","first_name":"Grace"}"#),
        MockWireResponse::json(200, r#"{"id":"person_001","object":"person","account":"acct_001","first_name":"Grace"}"#),
        MockWireResponse::json(200, r#"{"id":"person_001","object":"person","deleted":true}"#),
        MockWireResponse::json(200, r#"{"object":"list","data":[{"id":"person_001","object":"person","account":"acct_001"}],"has_more":false,"url":"/v1/accounts/acct_001/persons"}"#),
    ])
    .await;

    let stripe = web::Data::new(env.client_for_mock(&mock.base_url));
    let app = test::init_service(App::new().app_data(stripe).configure(configure_routes)).await;

    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/connect/accounts").set_json(&CreateAccountParams::new()).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/connect/accounts/me").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/connect/accounts/acct_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/connect/accounts/acct_001").set_json(&UpdateAccountParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::delete().uri("/connect/accounts/acct_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/connect/accounts/acct_001/reject").set_json(&RejectAccountParams::new("fraud")).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/connect/accounts?limit=5").to_request()).await.status(), StatusCode::OK);

    let mut link_params = CreateAccountLinkParams::new("acct_001", AccountLinkType::AccountOnboarding);
    link_params.refresh_url = Some("https://example.com/reauth".to_string());
    link_params.return_url = Some("https://example.com/return".to_string());
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/connect/account-links").set_json(&link_params).to_request()).await.status(), StatusCode::CREATED);

    let sess_params = CreateAccountSessionParams::new("acct_001", AccountSessionComponents::new());
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/connect/account-sessions").set_json(&sess_params).to_request()).await.status(), StatusCode::CREATED);

    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/connect/accounts/acct_001/persons").set_json(&CreatePersonParams::new()).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/connect/accounts/acct_001/persons/person_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/connect/accounts/acct_001/persons/person_001").set_json(&UpdatePersonParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::delete().uri("/connect/accounts/acct_001/persons/person_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/connect/accounts/acct_001/persons?limit=5").to_request()).await.status(), StatusCode::OK);

    assert_eq!(mock.finish().await.len(), 14);
}

#[actix_web::test]
async fn scenario_2_transfers_application_fees_and_payouts() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(200, r#"{"id":"tr_001","object":"transfer","amount":4000,"currency":"usd","destination":"acct_001"}"#),
        MockWireResponse::json(200, r#"{"id":"tr_001","object":"transfer","amount":4000,"currency":"usd","destination":"acct_001"}"#),
        MockWireResponse::json(200, r#"{"id":"tr_001","object":"transfer","amount":4000,"currency":"usd","destination":"acct_001"}"#),
        MockWireResponse::json(200, r#"{"id":"trr_001","object":"transfer_reversal","amount":1000,"currency":"usd","transfer":"tr_001"}"#),
        MockWireResponse::json(200, r#"{"object":"list","data":[{"id":"tr_001","object":"transfer","amount":4000,"currency":"usd","destination":"acct_001"}],"has_more":false,"url":"/v1/transfers"}"#),
        MockWireResponse::json(200, r#"{"object":"list","data":[{"id":"trr_001","object":"transfer_reversal","amount":1000,"currency":"usd","transfer":"tr_001"}],"has_more":false,"url":"/v1/transfers/tr_001/reversals"}"#),
        MockWireResponse::json(200, r#"{"id":"fee_001","object":"application_fee","amount":500,"currency":"usd"}"#),
        MockWireResponse::json(200, r#"{"object":"list","data":[{"id":"fee_001","object":"application_fee","amount":500,"currency":"usd"}],"has_more":false,"url":"/v1/application_fees"}"#),
        MockWireResponse::json(200, r#"{"id":"fr_001","object":"fee_refund","amount":250,"currency":"usd","fee":"fee_001"}"#),
        MockWireResponse::json(200, r#"{"id":"fr_001","object":"fee_refund","amount":250,"currency":"usd","fee":"fee_001"}"#),
        MockWireResponse::json(200, r#"{"object":"list","data":[{"id":"fr_001","object":"fee_refund","amount":250,"currency":"usd","fee":"fee_001"}],"has_more":false,"url":"/v1/application_fees/fee_001/refunds"}"#),
        MockWireResponse::json(200, r#"{"id":"po_001","object":"payout","amount":10000,"currency":"usd","status":"pending"}"#),
        MockWireResponse::json(200, r#"{"id":"po_001","object":"payout","amount":10000,"currency":"usd","status":"pending"}"#),
        MockWireResponse::json(200, r#"{"id":"po_001","object":"payout","amount":10000,"currency":"usd","status":"pending"}"#),
        MockWireResponse::json(200, r#"{"id":"po_001","object":"payout","amount":10000,"currency":"usd","status":"canceled"}"#),
        MockWireResponse::json(200, r#"{"id":"po_002","object":"payout","amount":10000,"currency":"usd","status":"paid"}"#),
        MockWireResponse::json(200, r#"{"object":"list","data":[{"id":"po_001","object":"payout","amount":10000,"currency":"usd","status":"pending"}],"has_more":false,"url":"/v1/payouts"}"#),
    ])
    .await;

    let stripe = web::Data::new(env.client_for_mock(&mock.base_url));
    let app = test::init_service(App::new().app_data(stripe).configure(configure_routes)).await;

    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/connect/transfers").set_json(&CreateTransferParams::new(4000, "usd", "acct_001")).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/connect/transfers/tr_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/connect/transfers/tr_001").set_json(&UpdateTransferParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/connect/transfers/tr_001/reverse").set_json(&CreateTransferReversalParams::new()).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/connect/transfers?limit=5").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/connect/transfers/tr_001/reversals?limit=5").to_request()).await.status(), StatusCode::OK);

    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/connect/application-fees/fee_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/connect/application-fees?limit=5").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/connect/application-fees/fee_001/refunds").set_json(&CreateFeeRefundParams::new()).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/connect/application-fees/fee_001/refunds/fr_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/connect/application-fees/fee_001/refunds?limit=5").to_request()).await.status(), StatusCode::OK);

    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/connect/payouts").set_json(&CreatePayoutParams::new(10000, "usd")).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/connect/payouts/po_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/connect/payouts/po_001").set_json(&UpdatePayoutParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/connect/payouts/po_001/cancel").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/connect/payouts/po_001/reverse").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/connect/payouts?limit=5").to_request()).await.status(), StatusCode::OK);

    assert_eq!(mock.finish().await.len(), 17);
}
