//! Integration Test Suite for Example 9: `tonic_full_sdk_server`
//!
//! Verifies the unified Tonic gRPC reference hub (`UnifiedStripeGrpcHub`) combining all
//! 8 modular gRPC service servers (`core_webhooks`, `payments`, `checkout`, `customers`,
//! `invoicing`, `connect`, `specialized`, and `v2`) plus `get_service_catalog`.

#[path = "../common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../../examples/tonic_full_sdk_server.rs"]
mod full_hub;

use common::{DotEnvConfig, MockStripeServer, MockWireResponse, sign_webhook_payload};
use full_hub::connect_marketplace::{ConnectMarketplaceGrpcService, ConnectRpcRequest};
use full_hub::core_and_webhooks::{
    CoreWebhooksGrpcService, HealthCheckRequest, WebhookVerifyRequest,
};
use full_hub::customers_and_subscriptions::{
    BillingRpcRequest, CustomersSubscriptionsGrpcService,
};
use full_hub::invoicing_and_billing_portal::{InvoicingPortalGrpcService, InvoicingRpcRequest};
use full_hub::payments_and_settlement::{
    JsonPayloadRequest, PaymentsSettlementGrpcService, ResourceIdRequest,
};
use full_hub::specialized_products::{SpecializedProductsGrpcService, SpecializedRpcRequest};
use full_hub::v2_next_gen_services::{V2NextGenGrpcService, V2RpcRequest};
use full_hub::{CatalogRequest, UnifiedStripeGrpcHub};
use some_stripe::{
    CreatePaymentIntentParams, CreatePortalSessionParams, CreateSubscriptionParams,
    CreateTransferParams, SubscriptionItemParams,
};
use tonic::Request;

/// Verifies `UnifiedStripeGrpcHub::get_service_catalog` and cross-suite gRPC execution across
/// payments, subscriptions, Connect transfers, invoicing, specialized tax, v2 events, and webhooks.
#[tokio::test]
async fn scenario_1_unified_grpc_hub_catalog_and_cross_suite_flows() {
    let env = DotEnvConfig::load();
    let webhook_secret_str = env.stripe_webhook_secret.as_str().to_string();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(
            200,
            r#"{"object":"balance","available":[{"amount":99000,"currency":"usd"}],"pending":[],"livemode":false}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"pi_hub_01","object":"payment_intent","amount":4900,"currency":"usd","status":"succeeded"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"sub_hub_01","object":"subscription","customer":"cus_001","status":"active"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"bps_hub_01","object":"billing_portal.session","customer":"cus_001","url":"https://billing.stripe.com/p/session/bps_hub_01"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"acct_platform_01","object":"account","charges_enabled":true}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"tr_hub_01","object":"transfer","amount":3500,"currency":"usd","destination":"acct_001"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"object":"tax.settings","status":"active"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"evt_v2_uni_01","object":"v2.core.event","type":"v1.billing.meter.error_report_triggered","created":"2026-11-11T00:00:00Z"}"#,
        ),
    ])
    .await;

    let hub = UnifiedStripeGrpcHub::with_client(
        env.client_for_mock(&mock.base_url),
        env.stripe_webhook_secret,
        env.ingestion_engine_for_mock(&mock.base_url),
    );

    // 1. Service Catalog Overview
    let catalog_resp = hub
        .get_service_catalog(Request::new(CatalogRequest {}))
        .await
        .expect("get_service_catalog")
        .into_inner();
    let catalog_json: serde_json::Value =
        serde_json::from_str(&catalog_resp.json_catalog).expect("valid catalog json");
    assert_eq!(
        catalog_json["grpc_services"].as_array().map(Vec::len),
        Some(8)
    );

    // 2. Core & Webhooks Health + Webhook Verification
    let health = hub
        .core_webhooks
        .get_health(Request::new(HealthCheckRequest {}))
        .await
        .expect("hub get_health")
        .into_inner();
    assert_eq!(health.status, "SERVING");
    assert!(health.secret_debug_redacted.contains("REDACTED"));

    let raw_event = br#"{"id":"evt_hub_01","object":"event","created":1728345600,"type":"checkout.session.completed","data":{"object":{"id":"cs_hub_01","object":"checkout.session","status":"complete","payment_status":"paid"}}}"#;
    let now = chrono::Utc::now().timestamp();
    let sig_header = sign_webhook_payload(raw_event, &webhook_secret_str, now);
    let wh_resp = hub
        .core_webhooks
        .verify_v1_webhook(Request::new(WebhookVerifyRequest {
            payload: raw_event.to_vec(),
            signature_header: sig_header,
        }))
        .await
        .expect("hub verify_v1_webhook")
        .into_inner();
    assert!(wh_resp.verified);
    assert_eq!(wh_resp.event_id, "evt_hub_01");

    // 3. Payment & Settlement Flow
    let bal_resp = hub
        .payments
        .retrieve_balance(Request::new(ResourceIdRequest {
            id: String::new(),
            limit: None,
        }))
        .await
        .expect("hub retrieve_balance")
        .into_inner();
    assert!(bal_resp.json_data.contains("99000"));

    let pi_resp = hub
        .payments
        .create_payment_intent(Request::new(JsonPayloadRequest {
            id: String::new(),
            json_params: serde_json::to_string(&CreatePaymentIntentParams::new(4900, "usd"))
                .expect("json"),
        }))
        .await
        .expect("hub create_payment_intent")
        .into_inner();
    assert!(pi_resp.json_data.contains("pi_hub_01"));

    // 4. Subscription Flow
    let sub_params =
        CreateSubscriptionParams::new("cus_001", vec![SubscriptionItemParams::new("price_1", 1)]);
    let sub_resp = hub
        .customers
        .create_subscription(Request::new(BillingRpcRequest {
            id: String::new(),
            json_params: serde_json::to_string(&sub_params).expect("json"),
        }))
        .await
        .expect("hub create_subscription")
        .into_inner();
    assert!(sub_resp.json_data.contains("sub_hub_01"));

    // 5. Invoicing & Portal Flow
    let mut portal_params = CreatePortalSessionParams::new();
    portal_params.customer = "cus_001".to_string();
    let portal_resp = hub
        .invoicing
        .create_portal_session(Request::new(InvoicingRpcRequest {
            id: String::new(),
            sub_id: String::new(),
            json_params: serde_json::to_string(&portal_params).expect("json"),
        }))
        .await
        .expect("hub create_portal_session")
        .into_inner();
    assert!(portal_resp.json_data.contains("bps_hub_01"));

    // 6. Connect Marketplace Flow
    let acct_resp = hub
        .connect
        .retrieve_current_account(Request::new(ConnectRpcRequest {
            id: String::new(),
            sub_id: String::new(),
            json_params: String::new(),
        }))
        .await
        .expect("hub retrieve_current_account")
        .into_inner();
    assert!(acct_resp.json_data.contains("acct_platform_01"));

    let tr_resp = hub
        .connect
        .create_transfer(Request::new(ConnectRpcRequest {
            id: String::new(),
            sub_id: String::new(),
            json_params: serde_json::to_string(&CreateTransferParams::new(3500, "usd", "acct_001"))
                .expect("json"),
        }))
        .await
        .expect("hub create_transfer")
        .into_inner();
    assert!(tr_resp.json_data.contains("tr_hub_01"));

    // 7. Specialized Tax Settings Flow
    let tax_resp = hub
        .specialized
        .retrieve_tax_settings(Request::new(SpecializedRpcRequest {
            id: String::new(),
            json_params: String::new(),
        }))
        .await
        .expect("hub retrieve_tax_settings")
        .into_inner();
    assert!(tax_resp.json_data.contains("tax.settings"));

    // 8. V2 Core Event Flow
    let v2_resp = hub
        .v2
        .retrieve_v2_event(Request::new(V2RpcRequest {
            id: "evt_v2_uni_01".to_string(),
            json_params: String::new(),
        }))
        .await
        .expect("hub retrieve_v2_event")
        .into_inner();
    assert!(v2_resp.json_data.contains("evt_v2_uni_01"));

    assert_eq!(mock.finish().await.len(), 8);
}
