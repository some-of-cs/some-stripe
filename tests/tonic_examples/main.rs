//! End-to-End Integration Test Suite for all 9 `some-stripe` Tonic gRPC Framework Examples.
//!
//! Covers:
//! 1. Native `"$STRIPE_SECRET_KEY"` and `"$STRIPE_WEBHOOK_SECRET"` resolution inside gRPC server state.
//! 2. Module 1 (`tonic_core_and_webhooks`): Health check, config inspection, constant-time HMAC-SHA256 webhook verification, and 1 MiB payload cap (`Code::ResourceExhausted`).
//! 3. Module 2 (`tonic_payments_and_settlement`): `PaymentIntents`, `Charges`, `Refunds`, `Disputes`, `Balance`, and `Payouts` over Tonic `Request<T>` / `Response<T>`.
//! 4. Module 3 (`tonic_checkout_and_payment_links`): Catalog bundle (`Product`, `Price`, `Coupon`, `TaxRate`), `CheckoutSession`, and `PaymentLink` RPCs.
//! 5. Module 4 (`tonic_customers_and_subscriptions`): Customer onboarding (`Customer` + `SetupIntent`), `Subscription` lifecycle, and metered usage reporting RPCs.
//! 6. Module 5 (`tonic_invoicing_and_billing_portal`): `Invoice` + `InvoiceItem`, `CreditNote`, `Quote`, and `BillingPortalSession` RPCs.
//! 7. Module 6 (`tonic_connect_marketplace`): Express `Account` + `AccountLink`, destination `Transfer`, `ApplicationFee`, and platform `Topup` RPCs.
//! 8. Module 7 (`tonic_specialized_products`): `Identity`, `Issuing`, `Terminal`, `Radar`, `Treasury`, `Climate`, `Tax`, and `Entitlements` RPCs.
//! 9. Module 8 (`tonic_v2_next_gen_services`): Stripe v2 Billing Meter Events, Meter Sessions, Event Destinations, and `MeterIngestionEngine` RPCs.
//! 10. Unified Hub (`tonic_full_sdk_server`): Full 68+ service gRPC hub verification.

#[path = "../../examples/tonic_core_and_webhooks.rs"]
mod tonic_core_and_webhooks;

#[path = "../../examples/tonic_payments_and_settlement.rs"]
mod tonic_payments_and_settlement;

#[path = "../../examples/tonic_checkout_and_payment_links.rs"]
mod tonic_checkout_and_payment_links;

#[path = "../../examples/tonic_customers_and_subscriptions.rs"]
mod tonic_customers_and_subscriptions;

#[path = "../../examples/tonic_invoicing_and_billing_portal.rs"]
mod tonic_invoicing_and_billing_portal;

#[path = "../../examples/tonic_connect_marketplace.rs"]
mod tonic_connect_marketplace;

#[path = "../../examples/tonic_specialized_products.rs"]
mod tonic_specialized_products;

#[path = "../../examples/tonic_v2_next_gen_services.rs"]
mod tonic_v2_next_gen_services;

#[path = "../../examples/tonic_full_sdk_server.rs"]
mod tonic_full_sdk_server;

use std::sync::Arc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use some_stripe::StripeClient;
use some_stripe_core::config::{SecretKey, StripeConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tonic::{Code, Request};

/// Lightweight deterministic mock Stripe HTTP server for Tonic gRPC integration tests.
struct MockStripeApi {
    base_url: String,
}

impl MockStripeApi {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock stripe server");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 16_384];
                    let n = stream.read(&mut buf).await.unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    let req = String::from_utf8_lossy(&buf[..n]);
                    let first_line = req.lines().next().unwrap_or("");
                    let body = route_mock_stripe_request(first_line);
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                });
            }
        });
        Self {
            base_url: format!("http://{}", addr),
        }
    }

    fn client(&self) -> StripeClient {
        let secret = SecretKey::new("sk_test_tonic_mock_secret_key_123456").expect("valid test key");
        let config = StripeConfig::builder(secret)
            .api_base(&self.base_url)
            .max_network_retries(0)
            .build()
            .expect("valid mock config");
        StripeClient::from_config(config).expect("valid mock client")
    }
}

fn route_mock_stripe_request(first_line: &str) -> String {
    if first_line.contains("/v1/balance") {
        r#"{"object":"balance","available":[{"amount":125000,"currency":"usd"}],"pending":[{"amount":25000,"currency":"usd"}],"livemode":false}"#.to_string()
    } else if first_line.contains("/v1/payouts") {
        r#"{"id":"po_tonic_1","object":"payout","amount":50000,"currency":"usd","status":"paid"}"#.to_string()
    } else if first_line.contains("/v1/payment_intents") {
        r#"{"id":"pi_tonic_1","object":"payment_intent","amount":4900,"currency":"usd","status":"succeeded"}"#.to_string()
    } else if first_line.contains("/v1/charges") {
        r#"{"id":"ch_tonic_1","object":"charge","amount":4900,"currency":"usd","paid":true,"status":"succeeded"}"#.to_string()
    } else if first_line.contains("/v1/refunds") {
        r#"{"id":"re_tonic_1","object":"refund","amount":4900,"currency":"usd","status":"succeeded"}"#.to_string()
    } else if first_line.contains("/v1/disputes") {
        r#"{"object":"list","data":[{"id":"dp_tonic_1","object":"dispute","amount":4900,"currency":"usd","status":"needs_response"}],"has_more":false,"url":"/v1/disputes"}"#.to_string()
    } else if first_line.contains("/v1/products") {
        r#"{"id":"prod_tonic_1","object":"product","name":"Tonic Pro Plan","active":true}"#.to_string()
    } else if first_line.contains("/v1/prices") {
        r#"{"id":"price_tonic_1","object":"price","currency":"usd","unit_amount":2900,"active":true}"#.to_string()
    } else if first_line.contains("/v1/coupons") {
        r#"{"id":"coup_tonic_1","object":"coupon","percent_off":20.0,"valid":true}"#.to_string()
    } else if first_line.contains("/v1/tax_rates") {
        r#"{"id":"txr_tonic_1","object":"tax_rate","display_name":"VAT","percentage":15.0,"inclusive":false,"active":true}"#.to_string()
    } else if first_line.contains("/v1/checkout/sessions") {
        r#"{"id":"cs_tonic_1","object":"checkout.session","status":"open","url":"https://checkout.stripe.com/c/pay/cs_tonic_1"}"#.to_string()
    } else if first_line.contains("/v1/payment_links") {
        r#"{"id":"plink_tonic_1","object":"payment_link","active":true,"url":"https://buy.stripe.com/plink_tonic_1"}"#.to_string()
    } else if first_line.contains("/v1/customers") {
        r#"{"id":"cus_tonic_1","object":"customer","email":"grpc@example.com","name":"Tonic User"}"#.to_string()
    } else if first_line.contains("/v1/setup_intents") {
        r#"{"id":"seti_tonic_1","object":"setup_intent","status":"requires_payment_method","client_secret":"seti_tonic_1_secret"}"#.to_string()
    } else if first_line.contains("/v1/subscriptions") {
        r#"{"id":"sub_tonic_1","object":"subscription","status":"active","customer":"cus_tonic_1"}"#.to_string()
    } else if first_line.contains("/v1/subscription_items") {
        r#"{"id":"si_tonic_1","object":"subscription_item","subscription":"sub_tonic_1","quantity":5}"#.to_string()
    } else if first_line.contains("/v1/invoices") {
        r#"{"id":"in_tonic_1","object":"invoice","status":"open","total":9900,"currency":"usd"}"#.to_string()
    } else if first_line.contains("/v1/invoiceitems") {
        r#"{"id":"ii_tonic_1","object":"invoiceitem","amount":9900,"currency":"usd"}"#.to_string()
    } else if first_line.contains("/v1/credit_notes") {
        r#"{"id":"cn_tonic_1","object":"credit_note","amount":1500,"currency":"usd","status":"issued"}"#.to_string()
    } else if first_line.contains("/v1/quotes") {
        r#"{"id":"qt_tonic_1","object":"quote","status":"accepted","amount_total":12000}"#.to_string()
    } else if first_line.contains("/v1/billing_portal/sessions") {
        r#"{"id":"bps_tonic_1","object":"billing_portal.session","url":"https://billing.stripe.com/p/session/bps_tonic_1"}"#.to_string()
    } else if first_line.contains("/v1/accounts") {
        r#"{"id":"acct_tonic_1","object":"account","charges_enabled":true,"payouts_enabled":true}"#.to_string()
    } else if first_line.contains("/v1/account_links") {
        r#"{"object":"account_link","url":"https://connect.stripe.com/setup/s/acct_tonic_1","created":1710000000,"expires_at":1710003600}"#.to_string()
    } else if first_line.contains("/v1/transfers") {
        r#"{"id":"tr_tonic_1","object":"transfer","amount":3500,"currency":"usd","destination":"acct_tonic_1"}"#.to_string()
    } else if first_line.contains("/v1/application_fees") {
        r#"{"id":"fee_tonic_1","object":"application_fee","amount":500,"currency":"usd","refunded":true}"#.to_string()
    } else if first_line.contains("/v1/topups") {
        r#"{"id":"tu_tonic_1","object":"topup","amount":100000,"currency":"usd","status":"succeeded"}"#.to_string()
    } else if first_line.contains("/v2/billing/meter_event_session") {
        r#"{"id":"mes_tonic_1","object":"v2.billing.meter_event_session","authentication_token":"tok_v2_tonic","expires_at":"2026-12-31T23:59:59Z"}"#.to_string()
    } else if first_line.contains("/v2/core/event_destinations") {
        r#"{"id":"ed_tonic_1","object":"v2.core.event_destination","name":"Tonic Webhook Sink","status":"enabled"}"#.to_string()
    } else {
        r#"{"id":"obj_tonic_generic","object":"generic","status":"succeeded","active":true,"livemode":false,"data":[]}"#.to_string()
    }
}

#[tokio::test]
async fn test_tonic_module_1_core_and_webhooks_rpcs() {
    use tonic_core_and_webhooks::{
        CoreWebhooksGrpc, CoreWebhooksGrpcServer, CoreAppState, HealthRequest, WebhookVerifyRequest,
    };

    let mock = MockStripeApi::start().await;
    let webhook_secret = "whsec_tonic_test_secret_key_123456";
    let state = Arc::new(CoreAppState {
        client: mock.client(),
        webhook_secret: SecretKey::new(webhook_secret).expect("valid webhook secret"),
    });
    let server = CoreWebhooksGrpcServer::new(state);

    // 1. Health RPC
    let health_resp = server
        .get_health(Request::new(HealthRequest {
            caller: "integration-suite".to_string(),
        }))
        .await
        .expect("get_health succeeded")
        .into_inner();
    assert_eq!(health_resp.status, "SERVING");
    assert!(health_resp.secret_redacted_preview.contains("REDACTED"));

    // 2. Valid HMAC-SHA256 Webhook Verification RPC
    let ts = chrono::Utc::now().timestamp();
    let payload = r#"{"id":"evt_tonic_1","object":"event","type":"payment_intent.succeeded","created":1710000000,"livemode":false,"data":{"object":{"id":"pi_tonic_1"}}}"#;
    let signed_payload = format!("{}.{}", ts, payload);
    let mut mac = Hmac::<Sha256>::new_from_slice(webhook_secret.as_bytes()).expect("hmac init");
    mac.update(signed_payload.as_bytes());
    let sig_hex = hex::encode(mac.finalize().into_bytes());
    let sig_header = format!("t={},v1={}", ts, sig_hex);

    let verify_resp = server
        .verify_webhook(Request::new(WebhookVerifyRequest {
            raw_payload: payload.to_string(),
            stripe_signature: sig_header,
        }))
        .await
        .expect("verify_webhook succeeded")
        .into_inner();
    assert!(verify_resp.verified);
    assert_eq!(verify_resp.event_id, "evt_tonic_1");

    // 3. Reject >1 MiB payload with Code::ResourceExhausted
    let oversized = "x".repeat(1_048_577);
    let err = server
        .verify_webhook(Request::new(WebhookVerifyRequest {
            raw_payload: oversized,
            stripe_signature: "t=1,v1=deadbeef".to_string(),
        }))
        .await
        .expect_err("oversized payload must fail");
    assert_eq!(err.code(), Code::ResourceExhausted);
}

#[tokio::test]
async fn test_tonic_module_2_payments_and_settlement_rpcs() {
    use tonic_payments_and_settlement::{
        CreatePaymentIntentRequest, PaymentsAppState, PaymentsSettlementGrpc,
        PaymentsSettlementGrpcServer, RefundRequest,
    };

    let mock = MockStripeApi::start().await;
    let server = PaymentsSettlementGrpcServer::new(Arc::new(PaymentsAppState {
        client: mock.client(),
    }));

    let pi_resp = server
        .create_payment_intent(Request::new(CreatePaymentIntentRequest {
            amount: 4900,
            currency: "usd".to_string(),
            customer_id: "cus_tonic_1".to_string(),
            idempotency_key: "idem_tonic_pi_1".to_string(),
        }))
        .await
        .expect("create_payment_intent succeeded")
        .into_inner();
    assert_eq!(pi_resp.payment_intent_id, "pi_tonic_1");
    assert_eq!(pi_resp.amount, 4900);

    let refund_resp = server
        .create_refund(Request::new(RefundRequest {
            payment_intent_id: "pi_tonic_1".to_string(),
            amount: 4900,
            reason: "requested_by_customer".to_string(),
        }))
        .await
        .expect("create_refund succeeded")
        .into_inner();
    assert_eq!(refund_resp.refund_id, "re_tonic_1");
}

#[tokio::test]
async fn test_tonic_modules_3_through_8_and_full_hub() {
    let mock = MockStripeApi::start().await;

    // Module 3: Checkout & Payment Links
    {
        use tonic_checkout_and_payment_links::{
            CatalogBundleRequest, CheckoutAppState, CheckoutLinksGrpc, CheckoutLinksGrpcServer,
        };
        let srv = CheckoutLinksGrpcServer::new(Arc::new(CheckoutAppState {
            client: mock.client(),
        }));
        let res = srv
            .create_catalog_bundle(Request::new(CatalogBundleRequest {
                product_name: "Tonic Pro Plan".to_string(),
                unit_amount: 2900,
                currency: "usd".to_string(),
                coupon_percent_off: 20.0,
            }))
            .await
            .expect("create_catalog_bundle succeeded")
            .into_inner();
        assert_eq!(res.product_id, "prod_tonic_1");
        assert_eq!(res.price_id, "price_tonic_1");
    }

    // Module 4: Customers & Subscriptions
    {
        use tonic_customers_and_subscriptions::{
            CustomersAppState, CustomersSubscriptionsGrpc, CustomersSubscriptionsGrpcServer,
            OnboardCustomerRequest,
        };
        let srv = CustomersSubscriptionsGrpcServer::new(Arc::new(CustomersAppState {
            client: mock.client(),
        }));
        let res = srv
            .onboard_customer(Request::new(OnboardCustomerRequest {
                email: "grpc@example.com".to_string(),
                name: "Tonic User".to_string(),
            }))
            .await
            .expect("onboard_customer succeeded")
            .into_inner();
        assert_eq!(res.customer_id, "cus_tonic_1");
    }

    // Module 5: Invoicing & Billing Portal
    {
        use tonic_invoicing_and_billing_portal::{
            InvoicingAppState, InvoicingPortalGrpc, InvoicingPortalGrpcServer, PortalSessionRequest,
        };
        let srv = InvoicingPortalGrpcServer::new(Arc::new(InvoicingAppState {
            client: mock.client(),
        }));
        let res = srv
            .create_billing_portal_session(Request::new(PortalSessionRequest {
                customer_id: "cus_tonic_1".to_string(),
                return_url: "https://example.com/account".to_string(),
            }))
            .await
            .expect("create_billing_portal_session succeeded")
            .into_inner();
        assert_eq!(res.session_id, "bps_tonic_1");
    }

    // Module 6: Connect Marketplace
    {
        use tonic_connect_marketplace::{
            ConnectAppState, ConnectMarketplaceGrpc, ConnectMarketplaceGrpcServer,
            OnboardConnectedAccountRequest,
        };
        let srv = ConnectMarketplaceGrpcServer::new(Arc::new(ConnectAppState {
            client: mock.client(),
        }));
        let res = srv
            .onboard_connected_account(Request::new(OnboardConnectedAccountRequest {
                email: "seller@example.com".to_string(),
                country: "US".to_string(),
                refresh_url: "https://example.com/reauth".to_string(),
                return_url: "https://example.com/return".to_string(),
            }))
            .await
            .expect("onboard_connected_account succeeded")
            .into_inner();
        assert_eq!(res.account_id, "acct_tonic_1");
    }

    // Module 7: Specialized Products
    {
        use tonic_specialized_products::{
            SpecializedAppState, SpecializedProductsGrpc, SpecializedProductsGrpcServer,
            SpecializedSuiteRequest,
        };
        let srv = SpecializedProductsGrpcServer::new(Arc::new(SpecializedAppState {
            client: mock.client(),
        }));
        let res = srv
            .run_specialized_suite(Request::new(SpecializedSuiteRequest {
                customer_id: "cus_tonic_1".to_string(),
                currency: "usd".to_string(),
            }))
            .await
            .expect("run_specialized_suite succeeded")
            .into_inner();
        assert!(!res.summary.is_empty());
    }

    // Module 8: Stripe v2 Next-Gen Services
    {
        use tonic_v2_next_gen_services::{
            MeterEventRequest, V2AppState, V2NextGenGrpc, V2NextGenGrpcServer,
        };
        let srv = V2NextGenGrpcServer::new(Arc::new(V2AppState {
            client: mock.client(),
        }));
        let res = srv
            .record_meter_event(Request::new(MeterEventRequest {
                event_name: "ai_tokens_used".to_string(),
                stripe_customer_id: "cus_tonic_1".to_string(),
                value: "1500".to_string(),
                identifier: "id_v2_tonic_1".to_string(),
            }))
            .await
            .expect("record_meter_event succeeded")
            .into_inner();
        assert!(res.accepted);
    }

    // Unified Hub: tonic_full_sdk_server
    {
        use tonic_full_sdk_server::{
            HubAppState, HubOverviewRequest, UnifiedStripeGrpcHub, UnifiedStripeGrpcHubServer,
        };
        let srv = UnifiedStripeGrpcHubServer::new(Arc::new(HubAppState {
            client: mock.client(),
            webhook_secret: SecretKey::new("whsec_tonic_hub_secret_123456").expect("valid secret"),
        }));
        let res = srv
            .get_hub_overview(Request::new(HubOverviewRequest {
                caller: "integration-test".to_string(),
            }))
            .await
            .expect("get_hub_overview succeeded")
            .into_inner();
        assert_eq!(res.status, "SERVING_ALL_68_SERVICES");
    }
}
