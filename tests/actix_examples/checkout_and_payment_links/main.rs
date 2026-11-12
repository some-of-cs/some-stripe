//! Integration Test Suite for Example 3: `actix_checkout_and_payment_links`
//!
//! Covers Hosted Checkout Sessions, open-redirect domain allowlist enforcement,
//! fulfillment readiness checks, Payment Links CRUD, and EU Omnibus Directive 2019/2161
//! Article 6a promotional price transparency validation.

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/actix_checkout_and_payment_links.rs"]
mod example_app;

use actix_web::{App, http::StatusCode, test, web};
use common::{DotEnvConfig, MockStripeServer, MockWireResponse};
use example_app::{OmnibusPromotionCheckRequest, configure_routes};
use some_stripe::{
    CreateCheckoutSessionParams, CreatePaymentLinkParams, PaymentLinkLineItemParams, TaxBehavior,
    UpdatePaymentLinkParams,
};

#[actix_web::test]
async fn scenario_1_checkout_session_lifecycle_and_fulfillment_readiness() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(
            200,
            r#"{"id":"cs_test_101","object":"checkout.session","status":"open","payment_status":"unpaid","url":"https://checkout.stripe.com/c/pay/cs_test_101"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"cs_test_101","object":"checkout.session","status":"complete","payment_status":"paid"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"cs_test_101","object":"checkout.session","status":"expired","payment_status":"unpaid"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"object":"list","data":[{"id":"li_001","object":"item","amount_total":5000,"currency":"usd","description":"Pro Plan","quantity":1}],"has_more":false,"url":"/v1/checkout/sessions/cs_test_101/line_items"}"#,
        ),
    ])
    .await;

    let stripe = web::Data::new(env.client_for_mock(&mock.base_url));
    let app = test::init_service(App::new().app_data(stripe).configure(configure_routes)).await;

    let mut create_params = CreateCheckoutSessionParams::new("https://example.com/return");
    create_params.mode = "payment".to_string();
    create_params.currency = Some("usd".to_string());
    create_params.unit_amount = Some(5000);
    create_params.product_name = Some("Pro Plan".to_string());
    create_params.quantity = Some(1);

    let r1 = test::call_service(
        &app,
        test::TestRequest::post().uri("/checkout/sessions").set_json(&create_params).to_request(),
    ).await;
    assert_eq!(r1.status(), StatusCode::CREATED);

    let r2 = test::call_service(
        &app,
        test::TestRequest::get().uri("/checkout/sessions/cs_test_101").to_request(),
    ).await;
    assert_eq!(r2.status(), StatusCode::OK);
    let r2_json: serde_json::Value = test::read_body_json(r2).await;
    assert_eq!(r2_json["is_fulfillment_ready"], true);

    let r3 = test::call_service(
        &app,
        test::TestRequest::post().uri("/checkout/sessions/cs_test_101/expire").to_request(),
    ).await;
    assert_eq!(r3.status(), StatusCode::OK);

    let r4 = test::call_service(
        &app,
        test::TestRequest::get().uri("/checkout/sessions/cs_test_101/line-items").to_request(),
    ).await;
    assert_eq!(r4.status(), StatusCode::OK);

    let captured = mock.finish().await;
    assert_eq!(captured.len(), 4);
}

#[actix_web::test]
async fn scenario_2_checkout_session_rejects_untrusted_open_redirect_domain() {
    let env = DotEnvConfig::load();
    let stripe = web::Data::new(env.client_for_mock("http://127.0.0.1:9"));
    let app = test::init_service(App::new().app_data(stripe).configure(configure_routes)).await;

    let bad_params = CreateCheckoutSessionParams::new("https://phishing.attacker.invalid/steal");
    let resp = test::call_service(
        &app,
        test::TestRequest::post().uri("/checkout/sessions").set_json(&bad_params).to_request(),
    ).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn scenario_3_payment_links_crud_and_line_items() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(
            200,
            r#"{"id":"plink_001","object":"payment_link","active":true,"url":"https://buy.stripe.com/test_001"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"plink_001","object":"payment_link","active":true,"url":"https://buy.stripe.com/test_001"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"plink_001","object":"payment_link","active":false,"url":"https://buy.stripe.com/test_001"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"object":"list","data":[{"id":"li_plink_1","object":"item","amount_total":2000,"currency":"usd","description":"Starter","quantity":1}],"has_more":false,"url":"/v1/payment_links/plink_001/line_items"}"#,
        ),
    ])
    .await;

    let stripe = web::Data::new(env.client_for_mock(&mock.base_url));
    let app = test::init_service(App::new().app_data(stripe).configure(configure_routes)).await;

    let create_link = CreatePaymentLinkParams::new(vec![PaymentLinkLineItemParams::new("price_123", 1)]);
    let r1 = test::call_service(
        &app,
        test::TestRequest::post().uri("/payment-links").set_json(&create_link).to_request(),
    ).await;
    assert_eq!(r1.status(), StatusCode::CREATED);

    let r2 = test::call_service(
        &app,
        test::TestRequest::get().uri("/payment-links/plink_001").to_request(),
    ).await;
    assert_eq!(r2.status(), StatusCode::OK);

    let mut update_link = UpdatePaymentLinkParams::new();
    update_link.active = Some(false);
    let r3 = test::call_service(
        &app,
        test::TestRequest::post().uri("/payment-links/plink_001").set_json(&update_link).to_request(),
    ).await;
    assert_eq!(r3.status(), StatusCode::OK);

    let r4 = test::call_service(
        &app,
        test::TestRequest::get().uri("/payment-links/plink_001/line-items").to_request(),
    ).await;
    assert_eq!(r4.status(), StatusCode::OK);

    let captured = mock.finish().await;
    assert_eq!(captured.len(), 4);
}

#[actix_web::test]
async fn scenario_4_eu_omnibus_directive_price_and_tax_transparency_validation() {
    let env = DotEnvConfig::load();
    let stripe = web::Data::new(env.client_for_mock("http://127.0.0.1:9"));
    let app = test::init_service(App::new().app_data(stripe).configure(configure_routes)).await;

    // Valid promotion
    let valid_promo = OmnibusPromotionCheckRequest {
        regular_price: 10000,
        discounted_price: 7500,
        prior_30d_lowest_price: Some(10000),
        tax_behavior: TaxBehavior::Inclusive,
    };
    let ok_resp = test::call_service(
        &app,
        test::TestRequest::post().uri("/checkout/validate-omnibus").set_json(&valid_promo).to_request(),
    ).await;
    assert_eq!(ok_resp.status(), StatusCode::OK);

    // Invalid promotion (missing 30-day prior lowest price)
    let invalid_promo = OmnibusPromotionCheckRequest {
        regular_price: 10000,
        discounted_price: 7500,
        prior_30d_lowest_price: None,
        tax_behavior: TaxBehavior::Inclusive,
    };
    let bad_resp = test::call_service(
        &app,
        test::TestRequest::post().uri("/checkout/validate-omnibus").set_json(&invalid_promo).to_request(),
    ).await;
    assert_eq!(bad_resp.status(), StatusCode::BAD_REQUEST);
}
