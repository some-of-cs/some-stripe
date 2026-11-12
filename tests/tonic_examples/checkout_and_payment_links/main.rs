//! Integration Test Suite for Example 3: `tonic_checkout_and_payment_links`
//!
//! Covers Hosted Checkout Sessions, open-redirect domain allowlist enforcement,
//! fulfillment readiness checks (`is_fulfillment_ready`), Payment Links CRUD, and
//! EU Omnibus Directive 2019/2161 Article 6a promotional price transparency validation.

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/tonic_checkout_and_payment_links.rs"]
mod example_grpc;

use common::{DotEnvConfig, MockStripeServer, MockWireResponse};
use example_grpc::{
    CheckoutIdRequest, CheckoutJsonRequest, CheckoutLinksGrpcServer, CheckoutLinksGrpcService,
    OmnibusPromotionInput,
};
use some_stripe::{
    CreateCheckoutSessionParams, CreatePaymentLinkParams, PaymentLinkLineItemParams, TaxBehavior,
    UpdatePaymentLinkParams,
};
use tonic::{Code, Request};

/// Verifies `CheckoutSession` create, retrieve (`is_fulfillment_ready`), expire, and line items.
#[tokio::test]
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

    let server = CheckoutLinksGrpcServer {
        stripe: env.client_for_mock(&mock.base_url),
    };

    let mut create_params = CreateCheckoutSessionParams::new("https://example.com/return");
    create_params.mode = "payment".to_string();
    create_params.currency = Some("usd".to_string());
    create_params.unit_amount = Some(5000);
    create_params.product_name = Some("Pro Plan".to_string());
    create_params.quantity = Some(1);

    let r1 = server
        .create_checkout_session(Request::new(CheckoutJsonRequest {
            id: String::new(),
            json_params: serde_json::to_string(&create_params).expect("json"),
        }))
        .await
        .expect("create_checkout_session")
        .into_inner();
    assert!(!r1.is_fulfillment_ready);
    assert!(r1.json_data.contains("cs_test_101"));

    let r2 = server
        .retrieve_checkout_session(Request::new(CheckoutIdRequest {
            id: "cs_test_101".to_string(),
        }))
        .await
        .expect("retrieve_checkout_session")
        .into_inner();
    assert!(r2.is_fulfillment_ready);

    let r3 = server
        .expire_checkout_session(Request::new(CheckoutIdRequest {
            id: "cs_test_101".to_string(),
        }))
        .await
        .expect("expire_checkout_session")
        .into_inner();
    assert!(r3.json_data.contains("expired"));

    let r4 = server
        .list_checkout_session_line_items(Request::new(CheckoutIdRequest {
            id: "cs_test_101".to_string(),
        }))
        .await
        .expect("list_checkout_session_line_items")
        .into_inner();
    assert!(r4.json_data.contains("li_001"));

    assert_eq!(mock.finish().await.len(), 4);
}

/// Verifies `create_checkout_session` rejects untrusted open-redirect URLs with `Code::InvalidArgument`.
#[tokio::test]
async fn scenario_2_checkout_session_rejects_untrusted_open_redirect_domain() {
    let env = DotEnvConfig::load();
    let server = CheckoutLinksGrpcServer {
        stripe: env.client_for_mock("http://127.0.0.1:9"),
    };

    let bad_params = CreateCheckoutSessionParams::new("https://phishing.attacker.invalid/steal");
    let err = server
        .create_checkout_session(Request::new(CheckoutJsonRequest {
            id: String::new(),
            json_params: serde_json::to_string(&bad_params).expect("json"),
        }))
        .await
        .expect_err("untrusted redirect domain must fail");
    assert_eq!(err.code(), Code::InvalidArgument);
}

/// Verifies `PaymentLink` create, retrieve, update, and line-item pagination RPCs.
#[tokio::test]
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

    let server = CheckoutLinksGrpcServer {
        stripe: env.client_for_mock(&mock.base_url),
    };

    let create_link =
        CreatePaymentLinkParams::new(vec![PaymentLinkLineItemParams::new("price_123", 1)]);
    let r1 = server
        .create_payment_link(Request::new(CheckoutJsonRequest {
            id: String::new(),
            json_params: serde_json::to_string(&create_link).expect("json"),
        }))
        .await
        .expect("create_payment_link")
        .into_inner();
    assert!(r1.json_data.contains("plink_001"));

    let r2 = server
        .retrieve_payment_link(Request::new(CheckoutIdRequest {
            id: "plink_001".to_string(),
        }))
        .await
        .expect("retrieve_payment_link")
        .into_inner();
    assert!(r2.json_data.contains("plink_001"));

    let mut update_link = UpdatePaymentLinkParams::new();
    update_link.active = Some(false);
    let r3 = server
        .update_payment_link(Request::new(CheckoutJsonRequest {
            id: "plink_001".to_string(),
            json_params: serde_json::to_string(&update_link).expect("json"),
        }))
        .await
        .expect("update_payment_link")
        .into_inner();
    assert!(r3.json_data.contains(r#""active":false"#));

    let r4 = server
        .list_payment_link_line_items(Request::new(CheckoutIdRequest {
            id: "plink_001".to_string(),
        }))
        .await
        .expect("list_payment_link_line_items")
        .into_inner();
    assert!(r4.json_data.contains("li_plink_1"));

    assert_eq!(mock.finish().await.len(), 4);
}

/// Verifies `validate_omnibus_promotion` for EU Omnibus Directive 2019/2161 Article 6a compliance.
#[tokio::test]
async fn scenario_4_eu_omnibus_directive_price_and_tax_transparency_validation() {
    let env = DotEnvConfig::load();
    let server = CheckoutLinksGrpcServer {
        stripe: env.client_for_mock("http://127.0.0.1:9"),
    };

    let valid_promo = OmnibusPromotionInput {
        regular_price: 10_000,
        discounted_price: 7_500,
        prior_30d_lowest_price: Some(10_000),
        tax_behavior: TaxBehavior::Inclusive,
    };
    let ok_resp = server
        .validate_omnibus_promotion(Request::new(CheckoutJsonRequest {
            id: String::new(),
            json_params: serde_json::to_string(&valid_promo).expect("json"),
        }))
        .await
        .expect("valid omnibus promotion should pass")
        .into_inner();
    assert!(ok_resp.is_fulfillment_ready);
    assert!(ok_resp.json_data.contains(r#""compliant":true"#));

    let invalid_promo = OmnibusPromotionInput {
        regular_price: 10_000,
        discounted_price: 7_500,
        prior_30d_lowest_price: None,
        tax_behavior: TaxBehavior::Inclusive,
    };
    let err = server
        .validate_omnibus_promotion(Request::new(CheckoutJsonRequest {
            id: String::new(),
            json_params: serde_json::to_string(&invalid_promo).expect("json"),
        }))
        .await
        .expect_err("missing 30d prior lowest price must fail");
    assert_eq!(err.code(), Code::InvalidArgument);
}
