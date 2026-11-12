//! Integration Test Suite for Example 9: `actix_full_sdk_server`
//!
//! Verifies the unified Actix-Web reference server mounting all 8 SDK service suites
//! (`/api/v1/core-webhooks`, `/api/v1/payments-suite`, `/api/v1/checkout-suite`,
//! `/api/v1/subscriptions-suite`, `/api/v1/invoicing-suite`, `/api/v1/connect-suite`,
//! `/api/v1/specialized-suite`, and `/api/v2-suite`) and the `/api/catalog` endpoint.

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/actix_full_sdk_server.rs"]
mod full_server;

use actix_web::{App, http::StatusCode, test, web};
use common::{DotEnvConfig, MockStripeServer, MockWireResponse};

#[actix_web::test]
async fn scenario_1_unified_server_catalog_and_cross_suite_routing() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(
            200,
            r#"{"object":"balance","available":[{"amount":99000,"currency":"usd"}],"pending":[],"livemode":false}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"acct_platform_01","object":"account","charges_enabled":true}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"evt_v2_uni_01","object":"v2.core.event","type":"v1.billing.meter.error_report_triggered","created":"2026-11-11T00:00:00Z"}"#,
        ),
    ])
    .await;

    let stripe = env.client_for_mock(&mock.base_url);
    let shared_stripe = web::Data::new(stripe.clone());
    let webhook_state = web::Data::new(full_server::core_and_webhooks::WebhookAppState {
        stripe: env.client_for_mock(&mock.base_url),
        webhook_secret: env.stripe_webhook_secret,
    });
    let v2_state = web::Data::new(full_server::v2_next_gen_services::V2AppState {
        stripe,
        ingestion_engine: env.ingestion_engine_for_mock(&mock.base_url),
    });

    let app = test::init_service(
        App::new()
            .app_data(shared_stripe)
            .app_data(webhook_state)
            .app_data(v2_state)
            .service(full_server::sdk_service_catalog)
            .service(
                web::scope("/api/v1/core-webhooks")
                    .configure(full_server::core_and_webhooks::configure_routes),
            )
            .service(
                web::scope("/api/v1/payments-suite")
                    .configure(full_server::payments_and_settlement::configure_routes),
            )
            .service(
                web::scope("/api/v1/checkout-suite")
                    .configure(full_server::checkout_and_payment_links::configure_routes),
            )
            .service(
                web::scope("/api/v1/subscriptions-suite")
                    .configure(full_server::customers_and_subscriptions::configure_routes),
            )
            .service(
                web::scope("/api/v1/invoicing-suite")
                    .configure(full_server::invoicing_and_billing_portal::configure_routes),
            )
            .service(
                web::scope("/api/v1/connect-suite")
                    .configure(full_server::connect_marketplace::configure_routes),
            )
            .service(
                web::scope("/api/v1/specialized-suite")
                    .configure(full_server::specialized_products::configure_routes),
            )
            .service(
                web::scope("/api/v2-suite")
                    .configure(full_server::v2_next_gen_services::configure_routes),
            ),
    )
    .await;

    // 1. Check /api/catalog
    let cat_resp = test::call_service(&app, test::TestRequest::get().uri("/api/catalog").to_request()).await;
    assert_eq!(cat_resp.status(), StatusCode::OK);
    let cat_json: serde_json::Value = test::read_body_json(cat_resp).await;
    assert_eq!(cat_json["suites"].as_array().map(Vec::len), Some(8));

    // 2. Check /api/v1/core-webhooks/health
    let health_resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/api/v1/core-webhooks/health").to_request(),
    ).await;
    assert_eq!(health_resp.status(), StatusCode::OK);

    // 3. Check /api/v1/payments-suite/payments/balance
    let bal_resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/api/v1/payments-suite/payments/balance").to_request(),
    ).await;
    assert_eq!(bal_resp.status(), StatusCode::OK);

    // 4. Check /api/v1/connect-suite/connect/accounts/me
    let acct_resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/api/v1/connect-suite/connect/accounts/me").to_request(),
    ).await;
    assert_eq!(acct_resp.status(), StatusCode::OK);

    // 5. Check /api/v2-suite/v2/core/events/evt_v2_uni_01
    let v2_resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/api/v2-suite/v2/core/events/evt_v2_uni_01").to_request(),
    ).await;
    assert_eq!(v2_resp.status(), StatusCode::OK);

    assert_eq!(mock.finish().await.len(), 3);
}
