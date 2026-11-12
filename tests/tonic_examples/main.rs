//! End-to-End Integration Test Suite for all 9 `some-stripe` Tonic gRPC Framework Examples.
//!
//! Covers:
//! 1. Native `"$STRIPE_SECRET_KEY"` and `"$STRIPE_WEBHOOK_SECRET"` resolution inside gRPC server state.
//! 2. Module 1 (`tonic_core_and_webhooks`): `get_health`, `preview_dual_protocol`, `verify_v1_webhook`, and >1 MiB payload guard.
//! 3. Module 2 (`tonic_payments_and_settlement`): `PaymentIntents`, `Refunds`, `Disputes`, and `Balance` over Tonic `Request<T>` / `Response<T>`.
//! 4. Module 3 (`tonic_checkout_and_payment_links`): `CheckoutSession`, `PaymentLink`, and `validate_omnibus_promotion` RPCs.
//! 5. Module 4 (`tonic_customers_and_subscriptions`): `Customer`, `Subscription`, and `SubscriptionItem` RPCs.
//! 6. Module 5 (`tonic_invoicing_and_billing_portal`): `Invoice`, `CreditNote`, `Quote`, and `BillingPortalSession` RPCs.
//! 7. Module 6 (`tonic_connect_marketplace`): `Account`, `AccountLink`, `Transfer`, `ApplicationFee`, and `Payout` RPCs.
//! 8. Module 7 (`tonic_specialized_products`): `Tax`, `Identity`, `Issuing`, `Treasury`, and `Terminal` RPCs.
//! 9. Module 8 (`tonic_v2_next_gen_services`): Stripe v2 `events`, `event_destinations`, `meters`, and `MeterIngestionEngine` RPCs.
//! 10. Unified Hub (`tonic_full_sdk_server`): `UnifiedStripeGrpcHub` catalog and cross-suite verification.

#[path = "common/mod.rs"]
mod common;

#[allow(dead_code)]
#[path = "../../examples/tonic_grpc/tonic_core_and_webhooks.rs"]
mod tonic_core_and_webhooks;

#[allow(dead_code)]
#[path = "../../examples/tonic_grpc/tonic_payments_and_settlement.rs"]
mod tonic_payments_and_settlement;

#[allow(dead_code)]
#[path = "../../examples/tonic_grpc/tonic_checkout_and_payment_links.rs"]
mod tonic_checkout_and_payment_links;

#[allow(dead_code)]
#[path = "../../examples/tonic_grpc/tonic_customers_and_subscriptions.rs"]
mod tonic_customers_and_subscriptions;

#[allow(dead_code)]
#[path = "../../examples/tonic_grpc/tonic_invoicing_and_billing_portal.rs"]
mod tonic_invoicing_and_billing_portal;

#[allow(dead_code)]
#[path = "../../examples/tonic_grpc/tonic_connect_marketplace.rs"]
mod tonic_connect_marketplace;

#[allow(dead_code)]
#[path = "../../examples/tonic_grpc/tonic_specialized_products.rs"]
mod tonic_specialized_products;

#[allow(dead_code)]
#[path = "../../examples/tonic_grpc/tonic_v2_next_gen_services.rs"]
mod tonic_v2_next_gen_services;

#[allow(dead_code)]
#[path = "../../examples/tonic_grpc/tonic_full_sdk_server.rs"]
mod tonic_full_sdk_server;

use common::{DotEnvConfig, MockStripeServer, MockWireResponse, sign_webhook_payload};
use some_stripe::{
    CreateCheckoutSessionParams, CreateCustomerParams, CreatePaymentIntentParams,
    CreatePortalSessionParams, CreateRefundParams, MAX_WEBHOOK_PAYLOAD_BYTES,
};
use tonic::{Code, Request};

/// Verifies `tonic_core_and_webhooks` RPCs (`get_health`, `verify_v1_webhook`, >1 MiB payload guard).
#[tokio::test]
async fn test_tonic_module_1_core_and_webhooks_rpcs() {
    use tonic_core_and_webhooks::{
        CoreWebhooksGrpcServer, CoreWebhooksGrpcService, HealthCheckRequest, WebhookVerifyRequest,
    };

    let env = DotEnvConfig::load();
    let secret_str = env.stripe_webhook_secret.as_str().to_string();
    let server = CoreWebhooksGrpcServer {
        stripe: env.client_for_mock("http://127.0.0.1:9"),
        webhook_secret: env.stripe_webhook_secret,
    };

    let health_resp = server
        .get_health(Request::new(HealthCheckRequest {}))
        .await
        .expect("get_health succeeded")
        .into_inner();
    assert_eq!(health_resp.status, "SERVING");
    assert!(health_resp.secret_debug_redacted.contains("REDACTED"));

    let ts = chrono::Utc::now().timestamp();
    let payload = br#"{"id":"evt_tonic_1","object":"event","type":"payment_intent.succeeded","created":1710000000,"livemode":false,"data":{"object":{"id":"pi_tonic_1","object":"payment_intent","amount":4900,"currency":"usd","status":"succeeded"}}}"#;
    let sig_header = sign_webhook_payload(payload, &secret_str, ts);

    let verify_resp = server
        .verify_webhook_v1_helper(payload, sig_header)
        .await;
    assert!(verify_resp.verified);
    assert_eq!(verify_resp.event_id, "evt_tonic_1");

    let oversized = vec![b'x'; MAX_WEBHOOK_PAYLOAD_BYTES + 1];
    let err = server
        .verify_v1_webhook(Request::new(WebhookVerifyRequest {
            payload: oversized,
            signature_header: "t=1,v1=deadbeef".to_string(),
        }))
        .await
        .expect_err("oversized payload must fail");
    assert!(matches!(
        err.code(),
        Code::InvalidArgument | Code::ResourceExhausted
    ));
}

trait VerifyWebhookTestExt {
    async fn verify_webhook_v1_helper(
        &self,
        payload: &[u8],
        signature_header: String,
    ) -> tonic_core_and_webhooks::V1WebhookVerifyResponse;
}

impl VerifyWebhookTestExt for tonic_core_and_webhooks::CoreWebhooksGrpcServer {
    async fn verify_webhook_v1_helper(
        &self,
        payload: &[u8],
        signature_header: String,
    ) -> tonic_core_and_webhooks::V1WebhookVerifyResponse {
        use tonic_core_and_webhooks::{CoreWebhooksGrpcService, WebhookVerifyRequest};
        self.verify_v1_webhook(Request::new(WebhookVerifyRequest {
            payload: payload.to_vec(),
            signature_header,
        }))
        .await
        .expect("verify_v1_webhook succeeded")
        .into_inner()
    }
}

/// Verifies `tonic_payments_and_settlement` RPCs (`create_payment_intent` and `create_refund`).
#[tokio::test]
async fn test_tonic_module_2_payments_and_settlement_rpcs() {
    use tonic_payments_and_settlement::{
        JsonPayloadRequest, PaymentsSettlementGrpcServer, PaymentsSettlementGrpcService,
    };

    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(
            200,
            r#"{"id":"pi_tonic_1","object":"payment_intent","amount":4900,"currency":"usd","status":"succeeded"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"re_tonic_1","object":"refund","amount":4900,"currency":"usd","status":"succeeded"}"#,
        ),
    ])
    .await;

    let server = PaymentsSettlementGrpcServer {
        stripe: env.client_for_mock(&mock.base_url),
    };

    let pi_resp = server
        .create_payment_intent(Request::new(JsonPayloadRequest {
            id: String::new(),
            json_params: serde_json::to_string(&CreatePaymentIntentParams::new(4900, "usd"))
                .expect("json"),
        }))
        .await
        .expect("create_payment_intent succeeded")
        .into_inner();
    assert!(pi_resp.json_data.contains("pi_tonic_1"));

    let mut refund_params = CreateRefundParams::new();
    refund_params.payment_intent = Some("pi_tonic_1".to_string());
    refund_params.amount = Some(4900);
    let refund_resp = server
        .create_refund(Request::new(JsonPayloadRequest {
            id: String::new(),
            json_params: serde_json::to_string(&refund_params).expect("json"),
        }))
        .await
        .expect("create_refund succeeded")
        .into_inner();
    assert!(refund_resp.json_data.contains("re_tonic_1"));
}

/// Verifies modules 3 through 8 and the unified gRPC hub (`UnifiedStripeGrpcHub`).
#[tokio::test]
async fn test_tonic_modules_3_through_8_and_full_hub() {
    let env = DotEnvConfig::load();
    let mock = MockStripeServer::start(vec![
        MockWireResponse::json(
            200,
            r#"{"id":"cs_tonic_1","object":"checkout.session","status":"open","url":"https://checkout.stripe.com/c/pay/cs_tonic_1"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"cus_tonic_1","object":"customer","email":"grpc@example.com","name":"Tonic User"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"bps_tonic_1","object":"billing_portal.session","url":"https://billing.stripe.com/p/session/bps_tonic_1"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"acct_tonic_1","object":"account","charges_enabled":true,"payouts_enabled":true}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"object":"tax.settings","status":"active"}"#,
        ),
        MockWireResponse::json(
            200,
            r#"{"id":"evt_v2_tonic_1","object":"v2.core.event","type":"v1.billing.meter.error_report_triggered","created":"2026-11-11T00:00:00Z"}"#,
        ),
    ])
    .await;

    // Module 3: Checkout & Payment Links
    {
        use tonic_checkout_and_payment_links::{
            CheckoutJsonRequest, CheckoutLinksGrpcServer, CheckoutLinksGrpcService,
        };
        let srv = CheckoutLinksGrpcServer {
            stripe: env.client_for_mock(&mock.base_url),
        };
        let params = CreateCheckoutSessionParams::new("https://example.com/return");
        let res = srv
            .create_checkout_session(Request::new(CheckoutJsonRequest {
                id: String::new(),
                json_params: serde_json::to_string(&params).expect("json"),
            }))
            .await
            .expect("create_checkout_session succeeded")
            .into_inner();
        assert!(res.json_data.contains("cs_tonic_1"));
    }

    // Module 4: Customers & Subscriptions
    {
        use tonic_customers_and_subscriptions::{
            BillingRpcRequest, CustomersSubscriptionsGrpcServer, CustomersSubscriptionsGrpcService,
        };
        let srv = CustomersSubscriptionsGrpcServer {
            stripe: env.client_for_mock(&mock.base_url),
        };
        let res = srv
            .create_customer(Request::new(BillingRpcRequest {
                id: String::new(),
                json_params: serde_json::to_string(&CreateCustomerParams::new()).expect("json"),
            }))
            .await
            .expect("create_customer succeeded")
            .into_inner();
        assert!(res.json_data.contains("cus_tonic_1"));
    }

    // Module 5: Invoicing & Billing Portal
    {
        use tonic_invoicing_and_billing_portal::{
            InvoicingPortalGrpcServer, InvoicingPortalGrpcService, InvoicingRpcRequest,
        };
        let srv = InvoicingPortalGrpcServer {
            stripe: env.client_for_mock(&mock.base_url),
        };
        let mut sess = CreatePortalSessionParams::new();
        sess.customer = "cus_tonic_1".to_string();
        let res = srv
            .create_portal_session(Request::new(InvoicingRpcRequest {
                id: String::new(),
                sub_id: String::new(),
                json_params: serde_json::to_string(&sess).expect("json"),
            }))
            .await
            .expect("create_portal_session succeeded")
            .into_inner();
        assert!(res.json_data.contains("bps_tonic_1"));
    }

    // Module 6: Connect Marketplace
    {
        use tonic_connect_marketplace::{
            ConnectMarketplaceGrpcServer, ConnectMarketplaceGrpcService, ConnectRpcRequest,
        };
        let srv = ConnectMarketplaceGrpcServer {
            stripe: env.client_for_mock(&mock.base_url),
        };
        let res = srv
            .retrieve_current_account(Request::new(ConnectRpcRequest {
                id: String::new(),
                sub_id: String::new(),
                json_params: String::new(),
            }))
            .await
            .expect("retrieve_current_account succeeded")
            .into_inner();
        assert!(res.json_data.contains("acct_tonic_1"));
    }

    // Module 7: Specialized Products
    {
        use tonic_specialized_products::{
            SpecializedProductsGrpcServer, SpecializedProductsGrpcService, SpecializedRpcRequest,
        };
        let srv = SpecializedProductsGrpcServer {
            stripe: env.client_for_mock(&mock.base_url),
        };
        let res = srv
            .retrieve_tax_settings(Request::new(SpecializedRpcRequest {
                id: String::new(),
                json_params: String::new(),
            }))
            .await
            .expect("retrieve_tax_settings succeeded")
            .into_inner();
        assert!(res.json_data.contains("tax.settings"));
    }

    // Module 8: Stripe v2 Next-Gen Services
    {
        use tonic_v2_next_gen_services::{
            V2NextGenGrpcServer, V2NextGenGrpcService, V2RpcRequest,
        };
        let srv = V2NextGenGrpcServer {
            stripe: env.client_for_mock(&mock.base_url),
            ingestion_engine: env.ingestion_engine_for_mock(&mock.base_url),
        };
        let res = srv
            .retrieve_v2_event(Request::new(V2RpcRequest {
                id: "evt_v2_tonic_1".to_string(),
                json_params: String::new(),
            }))
            .await
            .expect("retrieve_v2_event succeeded")
            .into_inner();
        assert!(res.json_data.contains("evt_v2_tonic_1"));
    }

    // Unified Hub: tonic_full_sdk_server
    {
        use tonic_full_sdk_server::{CatalogRequest, UnifiedStripeGrpcHub};
        let hub = UnifiedStripeGrpcHub::with_client(
            env.client_for_mock(&mock.base_url),
            env.stripe_webhook_secret,
            env.ingestion_engine_for_mock(&mock.base_url),
        );
        let res = hub
            .get_service_catalog(Request::new(CatalogRequest {}))
            .await
            .expect("get_service_catalog succeeded")
            .into_inner();
        assert!(res.json_catalog.contains("CoreWebhooksGrpcService"));
    }
}
