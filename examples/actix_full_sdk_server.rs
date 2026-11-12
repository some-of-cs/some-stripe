//! # Unified Actix-Web Reference Server Covering All `some-stripe` SDK Services
//!
//! Mounts all **8 Architectural Modules (34+ SDK Resource Namespaces across API v1 & v2)**
//! of `some-stripe` into a single production-structured `actix-web` application.
//!
//! ## Mounted Service Suites
//! 1. **Core & Cryptographic Webhooks (`actix_core_and_webhooks`)** — `/api/v1/core-webhooks`
//! 2. **Payments & Settlement (`actix_payments_and_settlement`)** — `/api/v1/payments-suite`
//! 3. **Hosted Checkout & Payment Links (`actix_checkout_and_payment_links`)** — `/api/v1/checkout-suite`
//! 4. **Customers, Subscriptions, Items & Schedules (`actix_customers_and_subscriptions`)** — `/api/v1/subscriptions-suite`
//! 5. **Invoices, Credit Notes, Quotes & Billing Portal (`actix_invoicing_and_billing_portal`)** — `/api/v1/invoicing-suite`
//! 6. **Connect & Multi-Party Marketplaces (`actix_connect_marketplace`)** — `/api/v1/connect-suite`
//! 7. **Tax, Identity, Issuing, Treasury & Terminal (`actix_specialized_products`)** — `/api/v1/specialized-suite`
//! 8. **Stripe API v2 Core, Hydration & Billing Meters (`actix_v2_next_gen_services`)** — `/api/v2-suite`
//!
//! ## Run the Unified Server
//! ```bash
//! STRIPE_SECRET_KEY=sk_test_123 STRIPE_WEBHOOK_SECRET=whsec_test_123 \
//!   cargo run --example actix_full_sdk_server
//! ```

#[allow(dead_code)]
#[path = "actix_core_and_webhooks.rs"]
mod core_and_webhooks;

#[allow(dead_code)]
#[path = "actix_payments_and_settlement.rs"]
mod payments_and_settlement;

#[allow(dead_code)]
#[path = "actix_checkout_and_payment_links.rs"]
mod checkout_and_payment_links;

#[allow(dead_code)]
#[path = "actix_customers_and_subscriptions.rs"]
mod customers_and_subscriptions;

#[allow(dead_code)]
#[path = "actix_invoicing_and_billing_portal.rs"]
mod invoicing_and_billing_portal;

#[allow(dead_code)]
#[path = "actix_connect_marketplace.rs"]
mod connect_marketplace;

#[allow(dead_code)]
#[path = "actix_specialized_products.rs"]
mod specialized_products;

#[allow(dead_code)]
#[path = "actix_v2_next_gen_services.rs"]
mod v2_next_gen_services;

use actix_web::{App, HttpResponse, HttpServer, get, web};
use some_stripe::{SecretKey, StripeClient};

/// `GET /api/catalog` — Returns the complete service catalog mounted on the Actix-Web server.
#[get("/api/catalog")]
pub async fn sdk_service_catalog() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "package": "some-stripe",
        "version": "0.8.0",
        "framework": "actix-web 4",
        "suites": [
            {
                "unit": 1,
                "example": "actix_core_and_webhooks",
                "prefix": "/api/v1/core-webhooks",
                "services": ["StripeClient", "StripeConfig", "CustomErrorWorkflowBuilder", "RequestOptions", "Nullable<T>", "stripe.webhooks()", "StripeWebhookWrapper"]
            },
            {
                "unit": 2,
                "example": "actix_payments_and_settlement",
                "prefix": "/api/v1/payments-suite",
                "services": ["stripe.payment_intents()", "stripe.refunds()", "stripe.disputes()", "stripe.balance()", "stripe.balance_transactions()"]
            },
            {
                "unit": 3,
                "example": "actix_checkout_and_payment_links",
                "prefix": "/api/v1/checkout-suite",
                "services": ["stripe.checkout().sessions()", "stripe.payment_links()", "validate_omnibus_price_transparency", "validate_tax_behavior_transparency"]
            },
            {
                "unit": 4,
                "example": "actix_customers_and_subscriptions",
                "prefix": "/api/v1/subscriptions-suite",
                "services": ["stripe.customers()", "stripe.subscriptions()", "stripe.subscriptions().items()", "stripe.subscription_schedules()"]
            },
            {
                "unit": 5,
                "example": "actix_invoicing_and_billing_portal",
                "prefix": "/api/v1/invoicing-suite",
                "services": ["stripe.invoices()", "stripe.credit_notes()", "stripe.quotes()", "stripe.billing_portal()"]
            },
            {
                "unit": 6,
                "example": "actix_connect_marketplace",
                "prefix": "/api/v1/connect-suite",
                "services": ["stripe.accounts()", "stripe.account_links()", "stripe.account_sessions()", "stripe.persons()", "stripe.transfers()", "stripe.application_fees()", "stripe.payouts()"]
            },
            {
                "unit": 7,
                "example": "actix_specialized_products",
                "prefix": "/api/v1/specialized-suite",
                "services": ["stripe.tax()", "stripe.identity()", "stripe.issuing()", "stripe.treasury()", "stripe.terminal()"]
            },
            {
                "unit": 8,
                "example": "actix_v2_next_gen_services",
                "prefix": "/api/v2-suite",
                "services": ["stripe.v2().core().events()", "stripe.v2().core().hydrator()", "stripe.v2().core().event_destinations()", "stripe.v2().core().accounts()", "stripe.v2().billing().meters()", "stripe.v2().billing().meter_event_session()", "stripe.v2().billing().meter_event_stream()", "stripe.v2().billing().meter_event_adjustments()", "stripe.meter_events()", "MeterIngestionEngine"]
            }
        ]
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let env = core_and_webhooks::dotenv::ExampleDotEnv::load();
    let stripe = core_and_webhooks::build_configured_client(
        env.secret_key.as_str(),
        env.api_base.as_deref(),
    );
    let shared_stripe = web::Data::new(env.build_client());
    let webhook_state = web::Data::new(core_and_webhooks::WebhookAppState {
        stripe: env.build_client(),
        webhook_secret: env.webhook_secret.clone(),
    });
    let v2_state = web::Data::new(v2_next_gen_services::V2AppState {
        ingestion_engine: env.build_ingestion_engine(&stripe),
        stripe,
    });

    println!(
        "Starting Unified some-stripe Actix-Web Reference Server on http://127.0.0.1:8080 (.env loaded: {})",
        env.dotenv_loaded
    );
    HttpServer::new(move || {
        App::new()
            .app_data(shared_stripe.clone())
            .app_data(webhook_state.clone())
            .app_data(v2_state.clone())
            .service(sdk_service_catalog)
            .service(
                web::scope("/api/v1/core-webhooks")
                    .configure(core_and_webhooks::configure_routes),
            )
            .service(
                web::scope("/api/v1/payments-suite")
                    .configure(payments_and_settlement::configure_routes),
            )
            .service(
                web::scope("/api/v1/checkout-suite")
                    .configure(checkout_and_payment_links::configure_routes),
            )
            .service(
                web::scope("/api/v1/subscriptions-suite")
                    .configure(customers_and_subscriptions::configure_routes),
            )
            .service(
                web::scope("/api/v1/invoicing-suite")
                    .configure(invoicing_and_billing_portal::configure_routes),
            )
            .service(
                web::scope("/api/v1/connect-suite")
                    .configure(connect_marketplace::configure_routes),
            )
            .service(
                web::scope("/api/v1/specialized-suite")
                    .configure(specialized_products::configure_routes),
            )
            .service(
                web::scope("/api/v2-suite")
                    .configure(v2_next_gen_services::configure_routes),
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
