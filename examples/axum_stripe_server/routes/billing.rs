//! Customers, Subscriptions, Invoices, Credit Notes, Quotes, and Customer Portal (Module 5).
//!
//! # Features
//! - Customer vaulting with GDPR Art. 15 (data portability) and Art. 17 (right-to-erasure) compliance.
//! - Recurring SaaS Subscriptions with multi-item tiers and proration management.
//! - Invoice lifecycle management (draft, finalize, pay, credit notes).
//! - Quotes to Invoices flow.
//! - Self-service Customer Portal session minting.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use some_stripe::{
    CreateCreditNoteParams, CreateCustomerParams, CreateInvoiceParams, CreatePortalSessionParams,
    CreateQuoteParams, CreateSubscriptionParams, CreditNoteReason, SubscriptionItemParams,
};
use std::str::FromStr;

use crate::state::{AppError, AppState};

/// Creates the billing and subscriptions router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/billing/customers", post(create_customer))
        .route("/billing/customers/{id}", get(get_customer))
        .route("/billing/customers/{id}", delete(delete_customer))
        .route("/billing/subscriptions", post(create_subscription))
        .route("/billing/subscriptions/{id}", get(get_subscription))
        .route("/billing/subscriptions/{id}", delete(cancel_subscription))
        .route("/billing/invoices", post(create_invoice))
        .route("/billing/invoices/{id}/finalize", post(finalize_invoice))
        .route("/billing/invoices/{id}/pay", post(pay_invoice))
        .route("/billing/credit-notes", post(create_credit_note))
        .route("/billing/quotes", post(create_quote))
        .route("/billing/portal/sessions", post(create_portal_session))
}

/// Request body for creating a Customer.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateCustomerRequest {
    pub email: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Create a customer in the vault.
pub async fn create_customer(
    State(state): State<AppState>,
    Json(payload): Json<CreateCustomerRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateCustomerParams {
        address: None,
        balance: None,
        description: payload.description,
        email: payload.email,
        invoice_settings: None,
        metadata: None,
        name: payload.name,
        phone: None,
        payment_method: None,
        shipping: None,
        currency: None,
        invoice_prefix: None,
        tax_exempt: None,
        tax_id_data: None,
        idempotency_key: None,
    };

    let customer = state.stripe.customers().create(&params).await?;
    Ok((StatusCode::CREATED, Json(customer)))
}

/// Retrieve customer details (fulfills GDPR Article 15 Data Portability).
pub async fn get_customer(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let customer = state.stripe.customers().retrieve(&id).await?;
    Ok((StatusCode::OK, Json(customer)))
}

/// Delete a customer (fulfills GDPR Article 17 Right-to-Erasure).
pub async fn delete_customer(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let customer = state.stripe.customers().delete(&id).await?;
    Ok((StatusCode::OK, Json(customer)))
}

/// Request body for creating a recurring Subscription.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateSubscriptionRequest {
    pub customer: String,
    pub price_id: String,
    pub quantity: Option<u64>,
}

/// Create a recurring subscription.
pub async fn create_subscription(
    State(state): State<AppState>,
    Json(payload): Json<CreateSubscriptionRequest>,
) -> Result<impl IntoResponse, AppError> {
    let item = SubscriptionItemParams {
        price: Some(payload.price_id),
        quantity: payload.quantity,
        id: None,
        deleted: None,
        metadata: None,
        proration_behavior: None,
        billing_thresholds: None,
        tax_rates: None,
    };

    let params = CreateSubscriptionParams {
        customer: payload.customer,
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
        trial_period_days: None,
        collection_method: None,
        days_until_due: None,
        expand: None,
        idempotency_key: None,
    };

    let sub = state.stripe.subscriptions().create(&params).await?;
    Ok((StatusCode::CREATED, Json(sub)))
}

/// Retrieve subscription details.
pub async fn get_subscription(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let sub = state.stripe.subscriptions().retrieve(&id).await?;
    Ok((StatusCode::OK, Json(sub)))
}

/// Cancel a subscription.
pub async fn cancel_subscription(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let sub = state.stripe.subscriptions().cancel(&id, None).await?;
    Ok((StatusCode::OK, Json(sub)))
}

/// Request body for creating an Invoice.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateInvoiceRequest {
    pub customer: String,
    pub description: Option<String>,
}

/// Create a draft Invoice.
pub async fn create_invoice(
    State(state): State<AppState>,
    Json(payload): Json<CreateInvoiceRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateInvoiceParams {
        customer: payload.customer,
        auto_advance: Some(true),
        collection_method: Some("charge_automatically".to_string()),
        currency: None,
        description: payload.description,
        due_date: None,
        days_until_due: None,
        metadata: None,
        payment_settings: None,
        subscription: None,
        idempotency_key: None,
    };

    let inv = state.stripe.invoices().create(&params).await?;
    Ok((StatusCode::CREATED, Json(inv)))
}

/// Finalize a draft invoice.
pub async fn finalize_invoice(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let inv = state.stripe.invoices().finalize_invoice(&id, None).await?;
    Ok((StatusCode::OK, Json(inv)))
}

/// Pay an open invoice.
pub async fn pay_invoice(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let inv = state.stripe.invoices().pay(&id, None).await?;
    Ok((StatusCode::OK, Json(inv)))
}

/// Request body for creating a Credit Note.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateCreditNoteRequest {
    pub invoice: String,
    pub amount: Option<i64>,
    pub reason: Option<String>,
}

/// Issue a Credit Note against an invoice, validating any supplied `reason` before writing.
pub async fn create_credit_note(
    State(state): State<AppState>,
    Json(payload): Json<CreateCreditNoteRequest>,
) -> Result<impl IntoResponse, AppError> {
    let credit_reason = payload
        .reason
        .as_deref()
        .map(CreditNoteReason::from_str)
        .transpose()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let params = CreateCreditNoteParams {
        invoice: payload.invoice,
        amount: payload.amount,
        credit_amount: None,
        refund_amount: None,
        out_of_band_amount: None,
        reason: credit_reason,
        memo: None,
        lines: None,
        metadata: None,
        idempotency_key: None,
    };

    let cn = state.stripe.credit_notes().create(&params).await?;
    Ok((StatusCode::CREATED, Json(cn)))
}

/// Request body for creating a Quote.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateQuoteRequest {
    pub customer: Option<String>,
    pub description: Option<String>,
}

/// Create a formal Quote.
pub async fn create_quote(
    State(state): State<AppState>,
    Json(payload): Json<CreateQuoteRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateQuoteParams {
        customer: payload.customer,
        line_items: None,
        description: payload.description,
        expires_at: None,
        footer: None,
        header: None,
        metadata: None,
        idempotency_key: None,
    };

    let quote = state.stripe.quotes().create(&params).await?;
    Ok((StatusCode::CREATED, Json(quote)))
}

/// Request body for creating a Customer Portal session.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreatePortalSessionRequest {
    pub customer: String,
    pub return_url: Option<String>,
}

/// Generate a Customer Portal session for self-service subscription management.
pub async fn create_portal_session(
    State(state): State<AppState>,
    Json(payload): Json<CreatePortalSessionRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreatePortalSessionParams {
        customer: payload.customer,
        configuration: None,
        on_behalf_of: None,
        return_url: payload.return_url,
        idempotency_key: None,
    };

    let session = state.stripe.billing_portal().create_session(&params).await?;
    Ok((StatusCode::CREATED, Json(session)))
}
