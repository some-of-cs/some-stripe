//! Integration Test Suite for Example 4: `tonic_customers_and_subscriptions`
//!
//! Covers all 30 Tonic gRPC methods across `customers`, `subscriptions`, `subscription_items`,
//! and `subscription_schedules`, including GDPR Art. 17 customer erasure, cash balance,
//! pause/resume collection, proration previews, metered usage records, and multi-phase schedules.

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/tonic_grpc/tonic_customers_and_subscriptions.rs"]
mod example_grpc;

use common::{DotEnvConfig, MockStripeServer, MockWireResponse};
use example_grpc::{
    BillingRpcRequest, CustomersSubscriptionsGrpcServer, CustomersSubscriptionsGrpcService,
};
use some_stripe::{
    CancelSubscriptionParams, CancelSubscriptionScheduleParams, CreateCustomerParams,
    CreateSubscriptionItemParams, CreateSubscriptionParams, CreateSubscriptionScheduleParams,
    CreateUsageRecordParams, ListCustomersParams, ListSubscriptionItemsParams,
    ListSubscriptionSchedulesParams, ListSubscriptionsParams, PauseCollectionParams,
    PreviewProrationParams, ReleaseSubscriptionScheduleParams, ResumeSubscriptionParams,
    SearchCustomersParams, SearchSubscriptionsParams, SubscriptionItemParams,
    UpdateCustomerCashBalanceParams, UpdateCustomerParams, UpdateSubscriptionItemParams,
    UpdateSubscriptionParams, UpdateSubscriptionScheduleParams,
};
use tonic::{Code, Request};

/// Verifies `Customer` CRUD, GDPR Art. 17 deletion, list, search, and cash balance RPCs.
#[tokio::test]
async fn scenario_1_customer_vault_crud_search_and_cash_balance() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(200, r#"{"id":"cus_001","object":"customer","email":"ada@example.com"}"#),
        MockWireResponse::json(200, r#"{"id":"cus_001","object":"customer","email":"ada@example.com"}"#),
        MockWireResponse::json(200, r#"{"id":"cus_001","object":"customer","email":null}"#),
        MockWireResponse::json(200, r#"{"id":"cus_001","object":"customer","deleted":true}"#),
        MockWireResponse::json(200, r#"{"object":"list","data":[{"id":"cus_001","object":"customer"}],"has_more":false,"url":"/v1/customers"}"#),
        MockWireResponse::json(200, r#"{"object":"search_result","data":[{"id":"cus_001","object":"customer"}],"has_more":false,"url":"/v1/customers/search"}"#),
        MockWireResponse::json(200, r#"{"object":"cash_balance","livemode":false}"#),
        MockWireResponse::json(200, r#"{"object":"cash_balance","livemode":false}"#),
    ])
    .await;

    let server = CustomersSubscriptionsGrpcServer {
        stripe: env.client_for_mock(&mock.base_url),
    };

    assert!(server.create_customer(Request::new(BillingRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&CreateCustomerParams::new()).expect("json"),
    })).await.is_ok());

    assert!(server.retrieve_customer(Request::new(BillingRpcRequest {
        id: "cus_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());

    assert!(server.update_customer(Request::new(BillingRpcRequest {
        id: "cus_001".to_string(),
        json_params: serde_json::to_string(&UpdateCustomerParams::new()).expect("json"),
    })).await.is_ok());

    assert!(server.delete_customer(Request::new(BillingRpcRequest {
        id: "cus_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());

    assert!(server.list_customers(Request::new(BillingRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&ListCustomersParams::new()).expect("json"),
    })).await.is_ok());

    assert!(server.search_customers(Request::new(BillingRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&SearchCustomersParams::new("email:'ada@example.com'")).expect("json"),
    })).await.is_ok());

    assert!(server.retrieve_customer_cash_balance(Request::new(BillingRpcRequest {
        id: "cus_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());

    assert!(server.update_customer_cash_balance(Request::new(BillingRpcRequest {
        id: "cus_001".to_string(),
        json_params: serde_json::to_string(&UpdateCustomerCashBalanceParams::new()).expect("json"),
    })).await.is_ok());

    assert_eq!(mock.finish().await.len(), 8);
}

/// Verifies `Subscription`, `SubscriptionItem`, `UsageRecord`, and `SubscriptionSchedule` RPCs.
#[tokio::test]
async fn scenario_2_subscriptions_items_usage_records_and_schedules() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(200, r#"{"id":"sub_001","object":"subscription","customer":"cus_001","status":"active"}"#),
        MockWireResponse::json(200, r#"{"id":"sub_001","object":"subscription","customer":"cus_001","status":"active"}"#),
        MockWireResponse::json(200, r#"{"id":"sub_001","object":"subscription","customer":"cus_001","status":"active"}"#),
        MockWireResponse::json(200, r#"{"id":"sub_001","object":"subscription","customer":"cus_001","status":"canceled"}"#),
        MockWireResponse::json(200, r#"{"id":"sub_001","object":"subscription","customer":"cus_001","status":"active"}"#),
        MockWireResponse::json(200, r#"{"id":"sub_001","object":"subscription","customer":"cus_001","status":"paused"}"#),
        MockWireResponse::json(200, r#"{"id":"sub_001","object":"subscription","customer":"cus_001","status":"active"}"#),
        MockWireResponse::json(200, r#"{"object":"invoice","amount_due":1500,"currency":"usd"}"#),
        MockWireResponse::json(200, r#"{"object":"search_result","data":[{"id":"sub_001","object":"subscription","customer":"cus_001","status":"active"}],"has_more":false,"url":"/v1/subscriptions/search"}"#),
        MockWireResponse::json(200, r#"{"object":"list","data":[{"id":"sub_001","object":"subscription","customer":"cus_001","status":"active"}],"has_more":false,"url":"/v1/subscriptions"}"#),
        MockWireResponse::json(200, r#"{"id":"si_001","object":"subscription_item","subscription":"sub_001","quantity":2}"#),
        MockWireResponse::json(200, r#"{"id":"si_001","object":"subscription_item","subscription":"sub_001","quantity":2}"#),
        MockWireResponse::json(200, r#"{"id":"si_001","object":"subscription_item","subscription":"sub_001","quantity":5}"#),
        MockWireResponse::json(200, r#"{"id":"si_001","object":"subscription_item","deleted":true}"#),
        MockWireResponse::json(200, r#"{"object":"list","data":[{"id":"si_001","object":"subscription_item","subscription":"sub_001"}],"has_more":false,"url":"/v1/subscription_items"}"#),
        MockWireResponse::json(200, r#"{"id":"mbur_001","object":"usage_record","quantity":100,"subscription_item":"si_001"}"#),
        MockWireResponse::json(200, r#"{"id":"sub_sched_001","object":"subscription_schedule","status":"not_started"}"#),
        MockWireResponse::json(200, r#"{"id":"sub_sched_001","object":"subscription_schedule","status":"active"}"#),
        MockWireResponse::json(200, r#"{"id":"sub_sched_001","object":"subscription_schedule","status":"active"}"#),
        MockWireResponse::json(200, r#"{"id":"sub_sched_001","object":"subscription_schedule","status":"released"}"#),
        MockWireResponse::json(200, r#"{"id":"sub_sched_001","object":"subscription_schedule","status":"canceled"}"#),
        MockWireResponse::json(200, r#"{"object":"list","data":[{"id":"sub_sched_001","object":"subscription_schedule","status":"active"}],"has_more":false,"url":"/v1/subscription_schedules"}"#),
    ])
    .await;

    let server = CustomersSubscriptionsGrpcServer {
        stripe: env.client_for_mock(&mock.base_url),
    };

    let create_sub = CreateSubscriptionParams::new("cus_001", vec![SubscriptionItemParams::new("price_123", 1)]);
    assert!(server.create_subscription(Request::new(BillingRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&create_sub).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_subscription(Request::new(BillingRpcRequest {
        id: "sub_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_subscription(Request::new(BillingRpcRequest {
        id: "sub_001".to_string(),
        json_params: serde_json::to_string(&UpdateSubscriptionParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.cancel_subscription(Request::new(BillingRpcRequest {
        id: "sub_001".to_string(),
        json_params: serde_json::to_string(&CancelSubscriptionParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.resume_subscription(Request::new(BillingRpcRequest {
        id: "sub_001".to_string(),
        json_params: serde_json::to_string(&ResumeSubscriptionParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.pause_subscription_collection(Request::new(BillingRpcRequest {
        id: "sub_001".to_string(),
        json_params: serde_json::to_string(&PauseCollectionParams::new("void")).expect("json"),
    })).await.is_ok());
    assert!(server.resume_subscription_collection(Request::new(BillingRpcRequest {
        id: "sub_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.preview_subscription_proration(Request::new(BillingRpcRequest {
        id: "sub_001".to_string(),
        json_params: serde_json::to_string(&PreviewProrationParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.search_subscriptions(Request::new(BillingRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&SearchSubscriptionsParams::new("status:'active'")).expect("json"),
    })).await.is_ok());
    assert!(server.list_subscriptions(Request::new(BillingRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&ListSubscriptionsParams::new()).expect("json"),
    })).await.is_ok());

    assert!(server.create_subscription_item(Request::new(BillingRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&CreateSubscriptionItemParams::new("sub_001", "price_123")).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_subscription_item(Request::new(BillingRpcRequest {
        id: "si_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_subscription_item(Request::new(BillingRpcRequest {
        id: "si_001".to_string(),
        json_params: serde_json::to_string(&UpdateSubscriptionItemParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.delete_subscription_item(Request::new(BillingRpcRequest {
        id: "si_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.list_subscription_items(Request::new(BillingRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&ListSubscriptionItemsParams::new("sub_001")).expect("json"),
    })).await.is_ok());
    assert!(server.create_usage_record(Request::new(BillingRpcRequest {
        id: "si_001".to_string(),
        json_params: serde_json::to_string(&CreateUsageRecordParams::new(100)).expect("json"),
    })).await.is_ok());

    let mut create_sched = CreateSubscriptionScheduleParams::new();
    create_sched.customer = Some("cus_001".to_string());
    assert!(server.create_subscription_schedule(Request::new(BillingRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&create_sched).expect("json"),
    })).await.is_ok());
    assert!(server.retrieve_subscription_schedule(Request::new(BillingRpcRequest {
        id: "sub_sched_001".to_string(),
        json_params: String::new(),
    })).await.is_ok());
    assert!(server.update_subscription_schedule(Request::new(BillingRpcRequest {
        id: "sub_sched_001".to_string(),
        json_params: serde_json::to_string(&UpdateSubscriptionScheduleParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.release_subscription_schedule(Request::new(BillingRpcRequest {
        id: "sub_sched_001".to_string(),
        json_params: serde_json::to_string(&ReleaseSubscriptionScheduleParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.cancel_subscription_schedule(Request::new(BillingRpcRequest {
        id: "sub_sched_001".to_string(),
        json_params: serde_json::to_string(&CancelSubscriptionScheduleParams::new()).expect("json"),
    })).await.is_ok());
    assert!(server.list_subscription_schedules(Request::new(BillingRpcRequest {
        id: String::new(),
        json_params: serde_json::to_string(&ListSubscriptionSchedulesParams::new()).expect("json"),
    })).await.is_ok());

    let invalid_err = server
        .create_subscription(Request::new(BillingRpcRequest {
            id: String::new(),
            json_params: "{invalid".to_string(),
        }))
        .await
        .expect_err("invalid json must return Code::InvalidArgument");
    assert_eq!(invalid_err.code(), Code::InvalidArgument);

    assert_eq!(mock.finish().await.len(), 22);
}
