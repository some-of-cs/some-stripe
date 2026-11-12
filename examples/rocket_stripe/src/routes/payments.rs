//! Payments, Refunds, Disputes, and Balance Ledger routes (Module 3).
//!
//! Exposes endpoints for:
//! - PaymentIntent creation, confirmation, capture, and cancellation.
//! - Issuing refunds and inspecting disputes.
//! - Querying the Stripe cash and balance transaction ledgers.

use rocket::serde::json::Json;
use rocket::State;
use some_stripe::{
    Balance, BalanceTransaction, CancelPaymentIntentParams, CapturePaymentIntentParams,
    ConfirmPaymentIntentParams, CreatePaymentIntentParams, CreateRefundParams, Dispute,
    PaymentIntent, Refund,
};

use crate::config::AppConfig;
use crate::models::{
    ApiResponse, ConfirmPaymentIntentRequest, CreatePaymentIntentRequest, ErrorResponse,
};

/// Creates a new Stripe PaymentIntent for online payment authorization.
///
/// Route: `POST /api/payments/intents`
#[rocket::post("/payments/intents", data = "<req>")]
pub async fn create_payment_intent(
    config: &State<AppConfig>,
    req: Json<CreatePaymentIntentRequest>,
) -> Result<Json<ApiResponse<PaymentIntent>>, ErrorResponse> {
    let params = CreatePaymentIntentParams {
        amount: req.amount,
        currency: req.currency.clone(),
        capture_method: req.capture_method.clone(),
        description: req.description.clone(),
        customer: req.customer.clone(),
        payment_method: None,
        receipt_email: None,
        return_url: None,
        confirm: None,
        metadata: None,
        idempotency_key: None,
    };

    let pi = config
        .client
        .payment_intents()
        .create(&params)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to create PaymentIntent: {e}")))?;

    Ok(Json(ApiResponse::new(pi)))
}

/// Retrieves a PaymentIntent by ID.
///
/// Route: `GET /api/payments/intents/<id>`
#[rocket::get("/payments/intents/<id>")]
pub async fn get_payment_intent(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<PaymentIntent>>, ErrorResponse> {
    let pi = config
        .client
        .payment_intents()
        .retrieve(id)
        .await
        .map_err(|e| ErrorResponse::not_found(format!("PaymentIntent '{id}' not found: {e}")))?;

    Ok(Json(ApiResponse::new(pi)))
}

/// Confirms an existing PaymentIntent with a payment method and redirect return URL.
///
/// Route: `POST /api/payments/intents/<id>/confirm`
#[rocket::post("/payments/intents/<id>/confirm", data = "<req>")]
pub async fn confirm_payment_intent(
    config: &State<AppConfig>,
    id: &str,
    req: Json<ConfirmPaymentIntentRequest>,
) -> Result<Json<ApiResponse<PaymentIntent>>, ErrorResponse> {
    let params = ConfirmPaymentIntentParams {
        payment_method: Some(req.payment_method.clone()),
        return_url: req.return_url.clone(),
        receipt_email: None,
        idempotency_key: None,
    };

    let pi = config
        .client
        .payment_intents()
        .confirm(id, Some(&params))
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to confirm PaymentIntent: {e}")))?;

    Ok(Json(ApiResponse::new(pi)))
}

/// Captures authorized funds for a PaymentIntent with manual capture method.
///
/// Route: `POST /api/payments/intents/<id>/capture`
#[rocket::post("/payments/intents/<id>/capture")]
pub async fn capture_payment_intent(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<PaymentIntent>>, ErrorResponse> {
    let params = CapturePaymentIntentParams {
        amount_to_capture: None,
        idempotency_key: None,
    };

    let pi = config
        .client
        .payment_intents()
        .capture(id, Some(&params))
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to capture PaymentIntent: {e}")))?;

    Ok(Json(ApiResponse::new(pi)))
}

/// Cancels an uncaptured PaymentIntent.
///
/// Route: `POST /api/payments/intents/<id>/cancel`
#[rocket::post("/payments/intents/<id>/cancel")]
pub async fn cancel_payment_intent(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<PaymentIntent>>, ErrorResponse> {
    let params = CancelPaymentIntentParams {
        cancellation_reason: Some("requested_by_customer".to_string()),
        idempotency_key: None,
    };

    let pi = config
        .client
        .payment_intents()
        .cancel(id, Some(&params))
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to cancel PaymentIntent: {e}")))?;

    Ok(Json(ApiResponse::new(pi)))
}

/// Issues a full or partial refund for a PaymentIntent or Charge.
///
/// Route: `POST /api/payments/refunds`
#[rocket::post("/payments/refunds", data = "<req>")]
pub async fn create_refund(
    config: &State<AppConfig>,
    req: Json<CreateRefundParams>,
) -> Result<Json<ApiResponse<Refund>>, ErrorResponse> {
    let refund = config
        .client
        .refunds()
        .create(&req.into_inner())
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to issue refund: {e}")))?;

    Ok(Json(ApiResponse::new(refund)))
}

/// Retrieves a Refund by ID.
///
/// Route: `GET /api/payments/refunds/<id>`
#[rocket::get("/payments/refunds/<id>")]
pub async fn get_refund(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<Refund>>, ErrorResponse> {
    let refund = config
        .client
        .refunds()
        .retrieve(id)
        .await
        .map_err(|e| ErrorResponse::not_found(format!("Refund '{id}' not found: {e}")))?;

    Ok(Json(ApiResponse::new(refund)))
}

/// Retrieves a Dispute by ID.
///
/// Route: `GET /api/payments/disputes/<id>`
#[rocket::get("/payments/disputes/<id>")]
pub async fn get_dispute(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<Dispute>>, ErrorResponse> {
    let dispute = config
        .client
        .disputes()
        .retrieve(id)
        .await
        .map_err(|e| ErrorResponse::not_found(format!("Dispute '{id}' not found: {e}")))?;

    Ok(Json(ApiResponse::new(dispute)))
}

/// Closes a Dispute in agreement with the issuer.
///
/// Route: `POST /api/payments/disputes/<id>/close`
#[rocket::post("/payments/disputes/<id>/close")]
pub async fn close_dispute(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<Dispute>>, ErrorResponse> {
    let dispute = config
        .client
        .disputes()
        .close(id)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to close dispute '{id}': {e}")))?;

    Ok(Json(ApiResponse::new(dispute)))
}

/// Retrieves the current account's Stripe Balance ledger.
///
/// Route: `GET /api/payments/balance`
#[rocket::get("/payments/balance")]
pub async fn get_balance(
    config: &State<AppConfig>,
) -> Result<Json<ApiResponse<Balance>>, ErrorResponse> {
    let balance = config
        .client
        .balance()
        .retrieve()
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to retrieve balance: {e}")))?;

    Ok(Json(ApiResponse::new(balance)))
}

/// Retrieves a specific Balance Transaction by ID.
///
/// Route: `GET /api/payments/balance/transactions/<id>`
#[rocket::get("/payments/balance/transactions/<id>")]
pub async fn get_balance_transaction(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<BalanceTransaction>>, ErrorResponse> {
    let txn = config
        .client
        .balance_transactions()
        .retrieve(id)
        .await
        .map_err(|e| ErrorResponse::not_found(format!("BalanceTransaction '{id}' not found: {e}")))?;

    Ok(Json(ApiResponse::new(txn)))
}
