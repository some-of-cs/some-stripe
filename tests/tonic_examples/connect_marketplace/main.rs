//! Integration Test Suite for Example 6: `tonic_connect_marketplace`
//!
//! Covers all 31 Tonic gRPC methods across `accounts`, `account_links`, `account_sessions`,
//! `persons`, `transfers`, `application_fees`, and `payouts`.

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/tonic_grpc/tonic_connect_marketplace.rs"]
mod example_grpc;

use common::{DotEnvConfig, MockStripeServer, MockWireResponse};
use example_grpc::{
    ConnectMarketplaceGrpcServer, ConnectMarketplaceGrpcService, ConnectRpcRequest,
};
use some_stripe::{
    AccountLinkType, AccountSessionComponents, CreateAccountLinkParams, CreateAccountParams,
    CreateAccountSessionParams, CreateFeeRefundParams, CreatePayoutParams, CreatePersonParams,
    CreateTransferParams, CreateTransferReversalParams, ListAccountsParams, ListPersonsParams,
    RejectAccountParams, UpdateAccountParams, UpdatePayoutParams, UpdatePersonParams,
    UpdateTransferParams,
};
use tonic::Request;

/// Verifies connected `Account` CRUD, `AccountLink`, `AccountSession`, and KYC `Person` RPCs.
#[tokio::test]
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

    let server = ConnectMarketplaceGrpcServer {
        stripe: env.client_for_mock(&mock.base_url),
    };

    assert!(server.create_account(Request::new(ConnectRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&CreateAccountParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_current_account(Request::new(ConnectRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.retrieve_account_by_id(Request::new(ConnectRpcRequest {
        id: "acct_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_account(Request::new(ConnectRpcRequest {
        id: "acct_001".to_string(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&UpdateAccountParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.delete_account(Request::new(ConnectRpcRequest {
        id: "acct_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.reject_account(Request::new(ConnectRpcRequest {
        id: "acct_001".to_string(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&RejectAccountParams::new("fraud")).expect("json"),
    })).await.is_ok());
    assert!(server.list_accounts(Request::new(ConnectRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&ListAccountsParams::new()).expect("json"),
    })).await.is_ok());

    let mut link_params = CreateAccountLinkParams::new("acct_001", AccountLinkType::AccountOnboarding);
    link_params.refresh_url = Some("https://example.com/reauth".to_string());
    link_params.return_url = Some("https://example.com/return".to_string());
    assert!(server.create_account_link(Request::new(ConnectRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&link_params).expect("json"),
    })).await.is_ok());

    let sess_params = CreateAccountSessionParams::new("acct_001", AccountSessionComponents::new());
    assert!(server.create_account_session(Request::new(ConnectRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&sess_params).expect("json"),
    })).await.is_ok());

    assert!(server.create_person(Request::new(ConnectRpcRequest {
        id: "acct_001".to_string(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&CreatePersonParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_person(Request::new(ConnectRpcRequest {
        id: "acct_001".to_string(),
        sub_id: "person_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_person(Request::new(ConnectRpcRequest {
        id: "acct_001".to_string(),
        sub_id: "person_001".to_string(),
        json_params: serde_json::to_string(&UpdatePersonParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.delete_person(Request::new(ConnectRpcRequest {
        id: "acct_001".to_string(),
        sub_id: "person_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.list_persons(Request::new(ConnectRpcRequest {
        id: "acct_001".to_string(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&ListPersonsParams::new()).expect("json"),
    })).await.is_ok());

    assert_eq!(mock.finish().await.len(), 14);
}

/// Verifies destination `Transfer`, `ApplicationFee`, and platform `Payout` RPCs.
#[tokio::test]
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

    let server = ConnectMarketplaceGrpcServer {
        stripe: env.client_for_mock(&mock.base_url),
    };

    assert!(server.create_transfer(Request::new(ConnectRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&CreateTransferParams::new(4000, "usd", "acct_001")).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_transfer(Request::new(ConnectRpcRequest {
        id: "tr_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_transfer(Request::new(ConnectRpcRequest {
        id: "tr_001".to_string(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&UpdateTransferParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.reverse_transfer(Request::new(ConnectRpcRequest {
        id: "tr_001".to_string(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&CreateTransferReversalParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.list_transfers(Request::new(ConnectRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.list_transfer_reversals(Request::new(ConnectRpcRequest {
        id: "tr_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());

    assert!(server.retrieve_application_fee(Request::new(ConnectRpcRequest {
        id: "fee_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.list_application_fees(Request::new(ConnectRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.create_application_fee_refund(Request::new(ConnectRpcRequest {
        id: "fee_001".to_string(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&CreateFeeRefundParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_application_fee_refund(Request::new(ConnectRpcRequest {
        id: "fee_001".to_string(),
        sub_id: "fr_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.list_application_fee_refunds(Request::new(ConnectRpcRequest {
        id: "fee_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());

    assert!(server.create_payout(Request::new(ConnectRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&CreatePayoutParams::new(10000, "usd")).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_payout(Request::new(ConnectRpcRequest {
        id: "po_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_payout(Request::new(ConnectRpcRequest {
        id: "po_001".to_string(),
        sub_id: String::new(),
        json_params: serde_json::to_string(&UpdatePayoutParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.cancel_payout(Request::new(ConnectRpcRequest {
        id: "po_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.reverse_payout(Request::new(ConnectRpcRequest {
        id: "po_001".to_string(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.list_payouts(Request::new(ConnectRpcRequest {
        id: String::new(),
        sub_id: String::new(),
        json_params: String::new(),
    })).await.is_ok());

    assert_eq!(mock.finish().await.len(), 17);
}
