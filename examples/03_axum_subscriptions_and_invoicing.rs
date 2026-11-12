//! # 03: Axum Subscriptions & Invoicing Service Example
//!
//! SaaS Recurring Billing, Customer Vaulting, and Invoicing microservice:
//! - Customer creation with GDPR Art. 15 (portability) and Art. 17 (erasure).
//! - Subscriptions with trial periods and multi-item tiers.
//! - Invoicing lifecycle (create, finalize, pay).
//! - Stripe Customer Portal for self-service subscription management.
//!
//! Run with:
//! ```bash
//! export STRIPE_SECRET_KEY="sk_test_..."
//! cargo run --example 03_axum_subscriptions_and_invoicing
//! ```

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use some_stripe::{
    CreateCustomerParams, CreateInvoiceParams, CreatePortalSessionParams,
    CreateSubscriptionParams, StripeClient, SubscriptionItemParams,
};
use std::sync::Arc;

/// Shared state holding the lock-free [`StripeClient`].
#[derive(Clone)]
pub struct BillingState {
    /// Thread-safe Stripe client.
    pub stripe: Arc<StripeClient>,
}

impl BillingState {
    /// Creates a new [`BillingState`] from an existing [`StripeClient`].
    pub fn with_client(client: StripeClient) -> Self {
        Self {
            stripe: Arc::new(client),
        }
    }
}

/// Builds the Axum [`Router`] for the Subscriptions & Invoicing microservice.
pub fn build_router(state: BillingState) -> Router {
    Router::new()
        .route("/customers", post(create_customer))
        .route("/customers/{id}", get(get_customer))
        .route("/customers/{id}", delete(delete_customer))
        .route("/subscriptions", post(create_subscription))
        .route("/subscriptions/{id}", delete(cancel_subscription))
        .route("/invoices", post(create_invoice))
        .route("/invoices/{id}/pay", post(pay_invoice))
        .route("/portal/session", post(create_portal))
        .with_state(state)
}

/// Entry point for the Subscriptions & Invoicing microservice.
#[allow(dead_code)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Automatically resolves $STRIPE_SECRET_KEY from process env, .env.local, .env.dev, .env, or .env.example
    let state = BillingState::with_client(StripeClient::new("$STRIPE_SECRET_KEY"));
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3003").await?;
    tracing::info!("Subscriptions & Invoicing service listening on http://0.0.0.0:3003");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Request payload for creating a Customer.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateCustomerReq {
    pub email: String,
    pub name: String,
}

/// Vaults a new Customer in Stripe.
pub async fn create_customer(
    State(state): State<BillingState>,
    Json(p): Json<CreateCustomerReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreateCustomerParams {
        address: None,
        balance: None,
        description: Some("Axum SaaS Subscriber".to_string()),
        email: Some(p.email),
        invoice_settings: None,
        metadata: None,
        name: Some(p.name),
        phone: None,
        payment_method: None,
        shipping: None,
        currency: None,
        invoice_prefix: None,
        tax_exempt: None,
        tax_id_data: None,
        idempotency_key: None,
    };

    let cust = state
        .stripe
        .customers()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(cust)))
}

/// Retrieves a Customer record (fulfills GDPR Article 15 Data Portability).
pub async fn get_customer(
    State(state): State<BillingState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cust = state
        .stripe
        .customers()
        .retrieve(&id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok((StatusCode::OK, Json(cust)))
}

/// Deletes a Customer record (fulfills GDPR Article 17 Right-to-Erasure).
pub async fn delete_customer(
    State(state): State<BillingState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cust = state
        .stripe
        .customers()
        .delete(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::OK, Json(cust)))
}

/// Request payload for creating a Subscription.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateSubReq {
    pub customer: String,
    pub price_id: String,
}

/// Creates a new recurring Subscription with a 14-day trial.
pub async fn create_subscription(
    State(state): State<BillingState>,
    Json(p): Json<CreateSubReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let item = SubscriptionItemParams {
        price: Some(p.price_id),
        quantity: Some(1),
        id: None,
        deleted: None,
        metadata: None,
        proration_behavior: None,
        billing_thresholds: None,
        tax_rates: None,
    };

    let params = CreateSubscriptionParams {
        customer: p.customer,
        items: vec![item],
        cancel_at_period_end: None,
        currency: None,
        default_payment_method: None,
        description: None,
        metadata: None,
        payment_behavior: None,
        payment_settings: None,
        proration_behavior: None,
        trial_end: None,
        trial_from_plan: None,
        trial_period_days: Some(14),
        collection_method: None,
        days_until_due: None,
        expand: None,
        idempotency_key: None,
    };

    let sub = state
        .stripe
        .subscriptions()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(sub)))
}

/// Cancels an active Subscription.
pub async fn cancel_subscription(
    State(state): State<BillingState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let sub = state
        .stripe
        .subscriptions()
        .cancel(&id, None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::OK, Json(sub)))
}

/// Request payload for creating a draft Invoice.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateInvoiceReq {
    pub customer: String,
}

/// Creates a draft Invoice for a Customer.
pub async fn create_invoice(
    State(state): State<BillingState>,
    Json(p): Json<CreateInvoiceReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreateInvoiceParams {
        customer: p.customer,
        auto_advance: Some(true),
        collection_method: Some("charge_automatically".to_string()),
        currency: None,
        description: Some("Monthly Usage Invoice".to_string()),
        due_date: None,
        days_until_due: None,
        metadata: None,
        payment_settings: None,
        subscription: None,
        idempotency_key: None,
    };

    let inv = state
        .stripe
        .invoices()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(inv)))
}

/// Pays an open Invoice.
pub async fn pay_invoice(
    State(state): State<BillingState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let inv = state
        .stripe
        .invoices()
        .pay(&id, None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::OK, Json(inv)))
}

/// Request payload for creating a Customer Portal Session.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreatePortalReq {
    pub customer: String,
    pub return_url: String,
}

/// Generates a Customer Portal session for self-service billing management.
pub async fn create_portal(
    State(state): State<BillingState>,
    Json(p): Json<CreatePortalReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreatePortalSessionParams {
        customer: p.customer,
        configuration: None,
        on_behalf_of: None,
        return_url: Some(p.return_url),
        idempotency_key: None,
    };

    let sess = state
        .stripe
        .billing_portal()
        .create_session(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(sess)))
}
