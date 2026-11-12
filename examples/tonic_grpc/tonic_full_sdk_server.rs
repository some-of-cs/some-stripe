//! # Unified Tonic gRPC Reference Hub Covering All `some-stripe` SDK Services
//!
//! Combines all **8 Tonic gRPC Service Suites (34+ SDK Resource Namespaces across API v1 & v2)**
//! into a single unified gRPC hub (`UnifiedStripeGrpcHub`), initialized via
//! `StripeClient::new("$STRIPE_SECRET_KEY")` and `SecretKey::new("$STRIPE_WEBHOOK_SECRET")`.
//!
//! ## Included gRPC Service Modules
//! 1. [`core_and_webhooks`] (`CoreWebhooksGrpcServer`)
//! 2. [`payments_and_settlement`] (`PaymentsSettlementGrpcServer`)
//! 3. [`checkout_and_payment_links`] (`CheckoutLinksGrpcServer`)
//! 4. [`customers_and_subscriptions`] (`CustomersSubscriptionsGrpcServer`)
//! 5. [`invoicing_and_billing_portal`] (`InvoicingPortalGrpcServer`)
//! 6. [`connect_marketplace`] (`ConnectMarketplaceGrpcServer`)
//! 7. [`specialized_products`] (`SpecializedProductsGrpcServer`)
//! 8. [`v2_next_gen_services`] (`V2NextGenGrpcServer`)
//!
//! ## Run the Unified gRPC Hub
//! ```bash
//! cargo run --example tonic_full_sdk_server
//! ```

#[allow(dead_code)]
#[path = "tonic_core_and_webhooks.rs"]
pub mod core_and_webhooks;

#[allow(dead_code)]
#[path = "tonic_payments_and_settlement.rs"]
pub mod payments_and_settlement;

#[allow(dead_code)]
#[path = "tonic_checkout_and_payment_links.rs"]
pub mod checkout_and_payment_links;

#[allow(dead_code)]
#[path = "tonic_customers_and_subscriptions.rs"]
pub mod customers_and_subscriptions;

#[allow(dead_code)]
#[path = "tonic_invoicing_and_billing_portal.rs"]
pub mod invoicing_and_billing_portal;

#[allow(dead_code)]
#[path = "tonic_connect_marketplace.rs"]
pub mod connect_marketplace;

#[allow(dead_code)]
#[path = "tonic_specialized_products.rs"]
pub mod specialized_products;

#[allow(dead_code)]
#[path = "tonic_v2_next_gen_services.rs"]
pub mod v2_next_gen_services;

use prost::Message;
use some_stripe::{MeterIngestionEngine, SecretKey, StripeClient};
use tonic::{Request, Response, Status};

#[derive(Clone, PartialEq, Message)]
pub struct CatalogRequest {}

#[derive(Clone, PartialEq, Message)]
pub struct CatalogResponse {
    #[prost(string, tag = "1")]
    pub json_catalog: String,
}

/// Unified Tonic gRPC hub holding all 8 modular gRPC service servers over a shared `StripeClient`.
#[derive(Clone)]
pub struct UnifiedStripeGrpcHub {
    pub core_webhooks: core_and_webhooks::CoreWebhooksGrpcServer,
    pub payments: payments_and_settlement::PaymentsSettlementGrpcServer,
    pub checkout: checkout_and_payment_links::CheckoutLinksGrpcServer,
    pub customers: customers_and_subscriptions::CustomersSubscriptionsGrpcServer,
    pub invoicing: invoicing_and_billing_portal::InvoicingPortalGrpcServer,
    pub connect: connect_marketplace::ConnectMarketplaceGrpcServer,
    pub specialized: specialized_products::SpecializedProductsGrpcServer,
    pub v2: v2_next_gen_services::V2NextGenGrpcServer,
}

impl UnifiedStripeGrpcHub {
    /// Initializes all 8 Tonic gRPC services from `"$STRIPE_SECRET_KEY"` and `"$STRIPE_WEBHOOK_SECRET"`.
    #[must_use]
    pub fn from_env() -> Self {
        let stripe = StripeClient::new("$STRIPE_SECRET_KEY");
        let webhook_secret = SecretKey::new("$STRIPE_WEBHOOK_SECRET");
        let ingestion_engine = stripe.v2().billing().create_ingestion_engine();
        Self::with_client(stripe, webhook_secret, ingestion_engine)
    }

    /// Initializes all 8 Tonic gRPC services from a provided [`StripeClient`].
    #[must_use]
    pub fn with_client(
        stripe: StripeClient,
        webhook_secret: SecretKey,
        ingestion_engine: MeterIngestionEngine,
    ) -> Self {
        Self {
            core_webhooks: core_and_webhooks::CoreWebhooksGrpcServer {
                stripe: stripe.clone(),
                webhook_secret,
            },
            payments: payments_and_settlement::PaymentsSettlementGrpcServer {
                stripe: stripe.clone(),
            },
            checkout: checkout_and_payment_links::CheckoutLinksGrpcServer {
                stripe: stripe.clone(),
            },
            customers: customers_and_subscriptions::CustomersSubscriptionsGrpcServer {
                stripe: stripe.clone(),
            },
            invoicing: invoicing_and_billing_portal::InvoicingPortalGrpcServer {
                stripe: stripe.clone(),
            },
            connect: connect_marketplace::ConnectMarketplaceGrpcServer {
                stripe: stripe.clone(),
            },
            specialized: specialized_products::SpecializedProductsGrpcServer {
                stripe: stripe.clone(),
            },
            v2: v2_next_gen_services::V2NextGenGrpcServer {
                stripe,
                ingestion_engine,
            },
        }
    }

    /// Returns the full gRPC service catalog across all 8 modules.
    pub async fn get_service_catalog(
        &self,
        _request: Request<CatalogRequest>,
    ) -> Result<Response<CatalogResponse>, Status> {
        let catalog = serde_json::json!({
            "package": "some-stripe",
            "version": "0.8.0",
            "framework": "tonic gRPC + prost",
            "proto_schema": "examples/proto/some_stripe.proto",
            "grpc_services": [
                { "unit": 1, "service": "CoreWebhooksGrpcService", "example": "tonic_core_and_webhooks" },
                { "unit": 2, "service": "PaymentsSettlementGrpcService", "example": "tonic_payments_and_settlement" },
                { "unit": 3, "service": "CheckoutLinksGrpcService", "example": "tonic_checkout_and_payment_links" },
                { "unit": 4, "service": "CustomersSubscriptionsGrpcService", "example": "tonic_customers_and_subscriptions" },
                { "unit": 5, "service": "InvoicingPortalGrpcService", "example": "tonic_invoicing_and_billing_portal" },
                { "unit": 6, "service": "ConnectMarketplaceGrpcService", "example": "tonic_connect_marketplace" },
                { "unit": 7, "service": "SpecializedProductsGrpcService", "example": "tonic_specialized_products" },
                { "unit": 8, "service": "V2NextGenGrpcService", "example": "tonic_v2_next_gen_services" }
            ]
        });
        Ok(Response::new(CatalogResponse {
            json_catalog: catalog.to_string(),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hub = UnifiedStripeGrpcHub::from_env();
    let catalog = hub
        .get_service_catalog(Request::new(CatalogRequest {}))
        .await?;
    println!(
        "Unified Tonic gRPC Hub initialized with 8 service suites:
{}",
        catalog.into_inner().json_catalog
    );
    Ok(())
}
