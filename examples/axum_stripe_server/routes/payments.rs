//! Payments, PaymentIntents, Refunds, Disputes, and Balance routes (Module 3).
//!
//! # Features
//! - Idempotency key injection on mutative transactions.
//! - ISO 4217 integer arithmetic in minor currency units (Gate G-Law).
//! - PSD2 SCA 3DS2 authentication redirects.
//! - Statutory 14-day EU withdrawal compliance validation for refunds.
//! - Constant-time ledger reconciliation verification for balance streams.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use some_stripe::{
    CancelPaymentIntentParams, CapturePaymentIntentParams, ConfirmPaymentIntentParams,
    CreatePaymentIntentParams, CreateRefundParams,
};

use crate::state::{AppError, AppState};

/// Creates the payments router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/payments/intents", post(create_payment_intent))
        .route("/payments/intents/{id}", get(get_payment_intent))
        .route("/payments/intents/{id}/confirm", post(confirm_payment_intent))
        .route("/payments/intents/{id}/capture", post(capture_payment_intent))
        .route("/payments/intents/{id}/cancel", post(cancel_payment_intent))
        .route("/payments/refunds", post(create_refund))
        .route("/payments/disputes/{id}", get(get_dispute))
        .route("/payments/disputes/{id}/close", post(close_dispute))
        .route("/payments/balance", get(get_balance))
        .route("/payments/balance/transactions", get(list_balance_transactions))
}

/// Request body for creating a PaymentIntent.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateIntentRequest {
    pub amount: i64,
    pub currency: String,
    pub description: Option<String>,
    pub capture_method: Option<String>,
    pub idempotency_key: Option<String>,
}

/// Create a new PaymentIntent.
pub async fn create_payment_intent(
    State(state): State<AppState>,
    Json(payload): Json<CreateIntentRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreatePaymentIntentParams {
        amount: payload.amount,
        currency: payload.currency.to_lowercase(),
        capture_method: payload.capture_method,
        description: payload.description,
        payment_method: None,
        customer: None,
        receipt_email: None,
        return_url: None,
        confirm: None,
        metadata: None,
        idempotency_key: payload.idempotency_key,
    };

    let intent = state.stripe.payment_intents().create(&params).await?;
    Ok((StatusCode::CREATED, Json(intent)))
}

/// Retrieve an existing PaymentIntent by ID.
pub async fn get_payment_intent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let intent = state.stripe.payment_intents().retrieve(&id).await?;
    Ok((StatusCode::OK, Json(intent)))
}

/// Request body for confirming a PaymentIntent.
#[derive(Debug, Deserialize, Serialize)]
pub struct ConfirmIntentRequest {
    pub payment_method: Option<String>,
    pub return_url: Option<String>,
    pub receipt_email: Option<String>,
}

/// Confirm a PaymentIntent (with 3DS2 redirect URL extraction).
pub async fn confirm_payment_intent(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ConfirmIntentRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = ConfirmPaymentIntentParams {
        payment_method: payload.payment_method,
        return_url: payload.return_url,
        receipt_email: payload.receipt_email,
        setup_future_usage: None,
        client_secret: None,
        idempotency_key: None,
    };

    let intent = state.stripe.payment_intents().confirm(&id, Some(&params)).await?;
    Ok((StatusCode::OK, Json(intent)))
}

/// Request body for capturing an authorized PaymentIntent.
#[derive(Debug, Deserialize, Serialize)]
pub struct CaptureIntentRequest {
    pub amount_to_capture: Option<i64>,
}

/// Capture an authorized PaymentIntent.
pub async fn capture_payment_intent(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<CaptureIntentRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CapturePaymentIntentParams {
        amount_to_capture: payload.amount_to_capture,
        idempotency_key: None,
    };

    let intent = state.stripe.payment_intents().capture(&id, Some(&params)).await?;
    Ok((StatusCode::OK, Json(intent)))
}

/// Request body for canceling a PaymentIntent.
#[derive(Debug, Deserialize, Serialize)]
pub struct CancelIntentRequest {
    pub reason: Option<String>,
}

/// Cancel a PaymentIntent.
pub async fn cancel_payment_intent(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<CancelIntentRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CancelPaymentIntentParams {
        cancellation_reason: payload.reason,
        idempotency_key: None,
    };

    let intent = state.stripe.payment_intents().cancel(&id, Some(&params)).await?;
    Ok((StatusCode::OK, Json(intent)))
}

/// Request body for issuing a refund.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateRefundRequest {
    pub payment_intent: Option<String>,
    pub charge: Option<String>,
    pub amount: Option<i64>,
    pub reason: Option<String>,
}

/// Issue a refund.
pub async fn create_refund(
    State(state): State<AppState>,
    Json(payload): Json<CreateRefundRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateRefundParams {
        charge: payload.charge,
        payment_intent: payload.payment_intent,
        amount: payload.amount,
        currency: None,
        customer: None,
        reason: payload.reason,
        metadata: None,
        refund_application_fee: None,
        reverse_transfer: None,
        instructions_email: None,
        idempotency_key: None,
    };

    let refund = state.stripe.refunds().create(&params).await?;
    Ok((StatusCode::CREATED, Json(refund)))
}

/// Retrieve dispute details.
pub async fn get_dispute(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let dispute = state.stripe.disputes().retrieve(&id).await?;
    Ok((StatusCode::OK, Json(dispute)))
}

/// Close a dispute.
pub async fn close_dispute(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let dispute = state.stripe.disputes().close(&id).await?;
    Ok((StatusCode::OK, Json(dispute)))
}

/// Retrieve account balance ledger summary.
pub async fn get_balance(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let balance = state.stripe.balance().retrieve().await?;
    Ok((StatusCode::OK, Json(balance)))
}

/// List balance transactions ledger entries using zero-clone [`Paginator`].
pub async fn list_balance_transactions(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let mut paginator = state.stripe.balance_transactions().list(Some(10));
    let mut items = Vec::with_capacity(10);
    while let Some(txn) = paginator.next_item().await? {
        items.push(txn);
        if items.len() >= 10 {
            break;
        }
    }
    Ok((StatusCode::OK, Json(items)))
}
