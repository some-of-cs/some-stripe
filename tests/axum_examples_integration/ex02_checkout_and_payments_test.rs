//! Integration Test Suite for `examples/02_axum_checkout_and_payments.rs` (Modules 3 & 4).
//!
//! Covers:
//! 1. `POST /checkout/session` & `GET /checkout/session/{id}` lifecycle.
//! 2. `POST /checkout/payment-link` with EU Omnibus Directive 2019/2161 compliant discount
//!    (verifying inline `price_data[unit_amount]` matches `current_discounted_price`).
//! 3. `POST /checkout/payment-link` rejecting deceptive discounts (`400 Bad Request`) where
//!    discounted price exceeds the 30-day lowest price.
//! 4. `POST /payments/intent`, `POST /payments/intent/{id}/confirm`, `POST /payments/intent/{id}/capture`.
//! 5. `POST /payments/refund` statutory refund creation.
//! 6. Live `.env` Stripe API verification when `STRIPE_SECRET_KEY` is provided in `.env`.

#[path = "../../examples/02_axum_checkout_and_payments.rs"]
mod ex02_axum_checkout_and_payments;

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::{
    build_live_stripe_client, build_mock_stripe_client, send_json_request,
    spawn_single_mock_stripe_server,
};
use ex02_axum_checkout_and_payments::{build_router, PaymentState};

/// Scenario 1: Create and retrieve a Hosted Checkout Session via Axum routes.
#[tokio::test]
async fn test_ex02_create_and_get_checkout_session() {
    let mock_session = r#"{
        "id": "cs_test_axum_02",
        "object": "checkout.session",
        "mode": "payment",
        "status": "open",
        "url": "https://checkout.stripe.com/c/pay/cs_test_axum_02",
        "currency": "usd",
        "amount_total": 4900
    }"#;

    let (api_base, handle) = spawn_single_mock_stripe_server("200 OK", mock_session).await;
    let app = build_router(PaymentState::with_client(build_mock_stripe_client(api_base)));

    let (status, body) = send_json_request(
        app,
        Method::POST,
        "/checkout/session",
        Some(json!({
            "success_url": "https://example.com/success",
            "cancel_url": "https://example.com/cancel",
            "currency": "USD",
            "unit_amount": 4900,
            "product_name": "Rust SDK Pro License"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["id"], "cs_test_axum_02");
    assert_eq!(body["amount_total"], 4900);

    let (req_headers, req_body) = handle.await.expect("Mock server failed");
    assert!(req_headers.starts_with("POST /v1/checkout/sessions HTTP/1.1"));
    assert!(req_body.contains("unit_amount=4900"));

    // Retrieve session
    let (api_base2, handle2) = spawn_single_mock_stripe_server("200 OK", mock_session).await;
    let app2 = build_router(PaymentState::with_client(build_mock_stripe_client(api_base2)));
    let (status2, body2) =
        send_json_request(app2, Method::GET, "/checkout/session/cs_test_axum_02", None).await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(body2["id"], "cs_test_axum_02");
    let (req_headers2, _) = handle2.await.expect("Mock server failed");
    assert!(req_headers2.starts_with("GET /v1/checkout/sessions/cs_test_axum_02 HTTP/1.1"));
}

/// Scenario 2 & 3: Payment Link creation with EU Omnibus Directive price transparency & inline `price_data`.
#[tokio::test]
async fn test_ex02_payment_link_omnibus_compliant_and_deceptive_rejection() {
    let mock_link = r#"{
        "id": "plink_test_omnibus_02",
        "object": "payment_link",
        "active": true,
        "url": "https://buy.stripe.com/test_omnibus_02"
    }"#;

    let (api_base, handle) = spawn_single_mock_stripe_server("200 OK", mock_link).await;
    let app = build_router(PaymentState::with_client(build_mock_stripe_client(
        api_base.clone(),
    )));

    // Compliant Omnibus discount: regular=10000, discounted=7500, prior_30d_lowest=8000
    let (status, body) = send_json_request(
        app,
        Method::POST,
        "/checkout/payment-link",
        Some(json!({
            "product_id": "prod_eu_saas",
            "currency": "eur",
            "regular_price": 10000,
            "current_discounted_price": 7500,
            "previous_lowest_price": 8000
        })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["id"], "plink_test_omnibus_02");

    let (_, wire_body) = handle.await.expect("Mock server failed");
    assert!(
        wire_body.contains("unit_amount%5D=7500") || wire_body.contains("unit_amount]=7500"),
        "Wire body must bind discounted amount 7500 in price_data: {wire_body}"
    );

    // Deceptive Omnibus discount: discounted=9000 > prior_30d_lowest=8000 -> 400 Bad Request
    let app_reject = build_router(PaymentState::with_client(build_mock_stripe_client(api_base)));
    let (status_err, _) = send_json_request(
        app_reject,
        Method::POST,
        "/checkout/payment-link",
        Some(json!({
            "product_id": "prod_eu_saas",
            "currency": "eur",
            "regular_price": 12000,
            "current_discounted_price": 9000,
            "previous_lowest_price": 8000
        })),
    )
    .await;
    assert_eq!(status_err, StatusCode::BAD_REQUEST);
}

/// Scenario 4 & 5: PaymentIntent create, confirm, capture, and Refund creation.
#[tokio::test]
async fn test_ex02_payment_intent_lifecycle_and_refund() {
    let mock_pi = r#"{
        "id": "pi_test_02",
        "object": "payment_intent",
        "amount": 3500,
        "currency": "usd",
        "status": "requires_capture"
    }"#;

    // Create Intent
    let (api_base1, h1) = spawn_single_mock_stripe_server("200 OK", mock_pi).await;
    let app1 = build_router(PaymentState::with_client(build_mock_stripe_client(api_base1)));
    let (s1, b1) = send_json_request(
        app1,
        Method::POST,
        "/payments/intent",
        Some(json!({ "amount": 3500, "currency": "USD" })),
    )
    .await;
    assert_eq!(s1, StatusCode::CREATED);
    assert_eq!(b1["id"], "pi_test_02");
    let _ = h1.await;

    // Confirm Intent
    let (api_base2, h2) = spawn_single_mock_stripe_server("200 OK", mock_pi).await;
    let app2 = build_router(PaymentState::with_client(build_mock_stripe_client(api_base2)));
    let (s2, _) = send_json_request(
        app2,
        Method::POST,
        "/payments/intent/pi_test_02/confirm",
        Some(json!({
            "payment_method": "pm_card_visa",
            "return_url": "https://example.com/3ds-return"
        })),
    )
    .await;
    assert_eq!(s2, StatusCode::OK);
    let _ = h2.await;

    // Capture Intent
    let (api_base3, h3) = spawn_single_mock_stripe_server("200 OK", mock_pi).await;
    let app3 = build_router(PaymentState::with_client(build_mock_stripe_client(api_base3)));
    let (s3, _) = send_json_request(
        app3,
        Method::POST,
        "/payments/intent/pi_test_02/capture",
        Some(json!({ "amount": 3500 })),
    )
    .await;
    assert_eq!(s3, StatusCode::OK);
    let _ = h3.await;

    // Refund Payment
    let mock_refund = r#"{
        "id": "re_test_02",
        "object": "refund",
        "amount": 1500,
        "currency": "usd",
        "payment_intent": "pi_test_02",
        "status": "succeeded"
    }"#;
    let (api_base4, h4) = spawn_single_mock_stripe_server("200 OK", mock_refund).await;
    let app4 = build_router(PaymentState::with_client(build_mock_stripe_client(api_base4)));
    let (s4, b4) = send_json_request(
        app4,
        Method::POST,
        "/payments/refund",
        Some(json!({ "payment_intent": "pi_test_02", "amount": 1500 })),
    )
    .await;
    assert_eq!(s4, StatusCode::CREATED);
    assert_eq!(b4["id"], "re_test_02");
    let _ = h4.await;
}

/// Scenario 6: Live `.env` Stripe API test (executes automatically when `STRIPE_SECRET_KEY` is set in `.env`).
#[tokio::test]
async fn test_ex02_live_env_payment_intent_and_checkout() {
    let Some(live_client) = build_live_stripe_client() else {
        return;
    };
    let app = build_router(PaymentState::with_client(live_client));
    let (status, body) = send_json_request(
        app,
        Method::POST,
        "/payments/intent",
        Some(json!({ "amount": 1099, "currency": "usd" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "Live Stripe PaymentIntent creation failed: {body}");
    assert!(body["id"].as_str().unwrap_or_default().starts_with("pi_"));
}
