//! Integration Test Suite for Example 4: `actix_customers_and_subscriptions`
//!
//! Covers all 28 Actix-Web routes across `customers`, `subscriptions`, `subscription_items`,
//! and `subscription_schedules`, including GDPR Art. 17 customer erasure, cash balance,
//! proration previews, usage records, and multi-phase schedules.

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/actix_web/actix_customers_and_subscriptions.rs"]
mod example_app;

use actix_web::{App, http::StatusCode, test, web};
use common::{DotEnvConfig, MockStripeServer, MockWireResponse};
use example_app::configure_routes;
use some_stripe::{
    CancelSubscriptionParams, CancelSubscriptionScheduleParams, CreateCustomerParams,
    CreateSubscriptionItemParams, CreateSubscriptionParams, CreateSubscriptionScheduleParams,
    CreateUsageRecordParams, PauseCollectionParams, PreviewProrationParams,
    ReleaseSubscriptionScheduleParams, ResumeSubscriptionParams, SearchCustomersParams,
    SearchSubscriptionsParams, SubscriptionItemParams, UpdateCustomerCashBalanceParams,
    UpdateCustomerParams, UpdateSubscriptionItemParams, UpdateSubscriptionParams,
    UpdateSubscriptionScheduleParams,
};

#[actix_web::test]
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

    let stripe = web::Data::new(env.client_for_mock(&mock.base_url));
    let app = test::init_service(App::new().app_data(stripe).configure(configure_routes)).await;

    assert_eq!(
        test::call_service(&app, test::TestRequest::post().uri("/customers").set_json(&CreateCustomerParams::new()).to_request()).await.status(),
        StatusCode::CREATED
    );
    assert_eq!(
        test::call_service(&app, test::TestRequest::get().uri("/customers/cus_001").to_request()).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        test::call_service(&app, test::TestRequest::post().uri("/customers/cus_001").set_json(&UpdateCustomerParams::new()).to_request()).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        test::call_service(&app, test::TestRequest::delete().uri("/customers/cus_001").to_request()).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        test::call_service(&app, test::TestRequest::get().uri("/customers?limit=5").to_request()).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        test::call_service(&app, test::TestRequest::post().uri("/customers/search").set_json(&SearchCustomersParams::new("email:'ada@example.com'")).to_request()).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        test::call_service(&app, test::TestRequest::get().uri("/customers/cus_001/cash-balance").to_request()).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        test::call_service(&app, test::TestRequest::post().uri("/customers/cus_001/cash-balance").set_json(&UpdateCustomerCashBalanceParams::new()).to_request()).await.status(),
        StatusCode::OK
    );

    assert_eq!(mock.finish().await.len(), 8);
}

#[actix_web::test]
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
    ])
    .await;

    let stripe = web::Data::new(env.client_for_mock(&mock.base_url));
    let app = test::init_service(App::new().app_data(stripe).configure(configure_routes)).await;

    let create_sub = CreateSubscriptionParams::new("cus_001", vec![SubscriptionItemParams::new("price_123", 1)]);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/subscriptions").set_json(&create_sub).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/subscriptions/sub_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/subscriptions/sub_001").set_json(&UpdateSubscriptionParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/subscriptions/sub_001/cancel").set_json(&CancelSubscriptionParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/subscriptions/sub_001/resume").set_json(&ResumeSubscriptionParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/subscriptions/sub_001/pause-collection").set_json(&PauseCollectionParams::new("void")).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/subscriptions/sub_001/resume-collection").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/subscriptions/sub_001/preview-proration").set_json(&PreviewProrationParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/subscriptions/search").set_json(&SearchSubscriptionsParams::new("status:'active'")).to_request()).await.status(), StatusCode::OK);

    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/subscription-items").set_json(&CreateSubscriptionItemParams::new("sub_001", "price_123")).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/subscription-items/si_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/subscription-items/si_001").set_json(&UpdateSubscriptionItemParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::delete().uri("/subscription-items/si_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/subscription-items?subscription=sub_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/subscription-items/si_001/usage-records").set_json(&CreateUsageRecordParams::new(100)).to_request()).await.status(), StatusCode::CREATED);

    let mut create_sched = CreateSubscriptionScheduleParams::new();
    create_sched.customer = Some("cus_001".to_string());
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/subscription-schedules").set_json(&create_sched).to_request()).await.status(), StatusCode::CREATED);
    assert_eq!(test::call_service(&app, test::TestRequest::get().uri("/subscription-schedules/sub_sched_001").to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/subscription-schedules/sub_sched_001").set_json(&UpdateSubscriptionScheduleParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/subscription-schedules/sub_sched_001/release").set_json(&ReleaseSubscriptionScheduleParams::new()).to_request()).await.status(), StatusCode::OK);
    assert_eq!(test::call_service(&app, test::TestRequest::post().uri("/subscription-schedules/sub_sched_001/cancel").set_json(&CancelSubscriptionScheduleParams::new()).to_request()).await.status(), StatusCode::OK);

    assert_eq!(mock.finish().await.len(), 20);
}
