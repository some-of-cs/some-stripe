//! # Actix-Web Example 2: Payments, Refunds, Disputes & Balance Settlement (Module 3)
//!
//! Demonstrates how to integrate **Module 3 (Payments & Settlement)** of `some-stripe`
//! inside an `actix-web` web server.
//!
//! ## Services Covered
//! - `stripe.payment_intents()`: `create`, `retrieve`, `confirm` (with PSD2 SCA 3DS2 redirect
//!   inspection), `capture`, and `cancel`
//! - `stripe.refunds()`: `create`, `retrieve`, `update`, and `cancel`
//! - `stripe.disputes()`: `retrieve`, `update` (defense evidence submission), and `close`
//! - `stripe.balance()` & `stripe.balance_transactions()`: `balance().retrieve()`,
//!   `balance_transactions().retrieve()`, and cursor-based pagination via
//!   `balance_transactions().list()`
//!
//! ## Run the Example
//! ```bash
//! STRIPE_SECRET_KEY=sk_test_123 cargo run --example actix_payments_and_settlement
//! ```

use actix_web::{App, HttpResponse, HttpServer, ResponseError, get, post, web};
use serde::{Deserialize, Serialize};
use some_stripe::{
    CancelPaymentIntentParams, CancelRefundParams, CapturePaymentIntentParams,
    ConfirmPaymentIntentParams, CreatePaymentIntentParams, CreateRefundParams, DisputeEvidence,
    SecretKey, StripeClient, StripeError, UpdateDisputeParams, UpdateRefundParams,
};
use std::fmt;

/// Shared Actix-Web error wrapper converting `StripeError` into structured JSON responses.
#[derive(Debug)]
pub struct ServiceError(pub StripeError);

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<StripeError> for ServiceError {
    fn from(err: StripeError) -> Self {
        Self(err)
    }
}

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::BadGateway().json(serde_json::json!({
            "error": self.0.to_string(),
        }))
    }
}

/// Request DTO for `POST /payments/disputes/{id}/update`.
#[derive(Debug, Deserialize, Serialize)]
pub struct SubmitDisputeEvidenceRequest {
    pub customer_name: Option<String>,
    pub product_description: Option<String>,
    pub uncategorized_text: Option<String>,
    pub submit: Option<bool>,
}

/// `POST /payments/intents` — Creates a new [`some_stripe::PaymentIntent`] using borrowed params.
#[post("/payments/intents")]
pub async fn create_payment_intent(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreatePaymentIntentParams>,
) -> Result<HttpResponse, ServiceError> {
    let intent = stripe.payment_intents().create(&params).await?;
    Ok(HttpResponse::Created().json(intent))
}

/// `GET /payments/intents/{id}` — Retrieves a [`some_stripe::PaymentIntent`] by ID.
#[get("/payments/intents/{id}")]
pub async fn retrieve_payment_intent(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, ServiceError> {
    let intent = stripe.payment_intents().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(intent))
}

/// `POST /payments/intents/{id}/confirm` — Confirms a `PaymentIntent` and surfaces any
/// PSD2 Strong Customer Authentication (3DS2) redirect URL.
#[post("/payments/intents/{id}/confirm")]
pub async fn confirm_payment_intent(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<ConfirmPaymentIntentParams>,
) -> Result<HttpResponse, ServiceError> {
    let intent = stripe
        .payment_intents()
        .confirm(id.as_str(), Some(&params))
        .await?;
    let requires_action = intent.status == "requires_action";
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "payment_intent": intent,
        "requires_3ds2_redirect": requires_action,
    })))
}

/// `POST /payments/intents/{id}/capture` — Captures an authorized `PaymentIntent`.
#[post("/payments/intents/{id}/capture")]
pub async fn capture_payment_intent(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<CapturePaymentIntentParams>,
) -> Result<HttpResponse, ServiceError> {
    let intent = stripe
        .payment_intents()
        .capture(id.as_str(), Some(&params))
        .await?;
    Ok(HttpResponse::Ok().json(intent))
}

/// `POST /payments/intents/{id}/cancel` — Cancels a `PaymentIntent`.
#[post("/payments/intents/{id}/cancel")]
pub async fn cancel_payment_intent(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<CancelPaymentIntentParams>,
) -> Result<HttpResponse, ServiceError> {
    let intent = stripe
        .payment_intents()
        .cancel(id.as_str(), Some(&params))
        .await?;
    Ok(HttpResponse::Ok().json(intent))
}

/// `POST /payments/refunds` — Issues a full or partial refund via `stripe.refunds().create()`.
#[post("/payments/refunds")]
pub async fn create_refund(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateRefundParams>,
) -> Result<HttpResponse, ServiceError> {
    let refund = stripe.refunds().create(&params).await?;
    Ok(HttpResponse::Created().json(refund))
}

/// `GET /payments/refunds/{id}` — Retrieves a refund via `stripe.refunds().retrieve()`.
#[get("/payments/refunds/{id}")]
pub async fn retrieve_refund(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, ServiceError> {
    let refund = stripe.refunds().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(refund))
}

/// `POST /payments/refunds/{id}/update` — Updates a refund's metadata via `stripe.refunds().update()`.
#[post("/payments/refunds/{id}/update")]
pub async fn update_refund(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateRefundParams>,
) -> Result<HttpResponse, ServiceError> {
    let refund = stripe.refunds().update(id.as_str(), &params).await?;
    Ok(HttpResponse::Ok().json(refund))
}

/// `POST /payments/refunds/{id}/cancel` — Cancels a refund via `stripe.refunds().cancel()`.
#[post("/payments/refunds/{id}/cancel")]
pub async fn cancel_refund(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<CancelRefundParams>,
) -> Result<HttpResponse, ServiceError> {
    let refund = stripe.refunds().cancel(id.as_str(), Some(&params)).await?;
    Ok(HttpResponse::Ok().json(refund))
}

/// `GET /payments/disputes/{id}` — Retrieves a chargeback dispute via `stripe.disputes().retrieve()`.
#[get("/payments/disputes/{id}")]
pub async fn retrieve_dispute(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, ServiceError> {
    let dispute = stripe.disputes().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(dispute))
}

/// `POST /payments/disputes/{id}/update` — Submits dispute defense evidence via `stripe.disputes().update()`.
#[post("/payments/disputes/{id}/update")]
pub async fn update_dispute(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    body: web::Json<SubmitDisputeEvidenceRequest>,
) -> Result<HttpResponse, ServiceError> {
    let req = body.into_inner();
    let params = UpdateDisputeParams {
        evidence: Some(DisputeEvidence {
            customer_name: req.customer_name,
            product_description: req.product_description,
            uncategorized_text: req.uncategorized_text,
            ..Default::default()
        }),
        metadata: None,
        submit: req.submit,
        idempotency_key: None,
    };
    let dispute = stripe.disputes().update(id.as_str(), &params).await?;
    Ok(HttpResponse::Ok().json(dispute))
}

/// `POST /payments/disputes/{id}/close` — Closes a dispute in the cardholder's favor via `stripe.disputes().close()`.
#[post("/payments/disputes/{id}/close")]
pub async fn close_dispute(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, ServiceError> {
    let dispute = stripe.disputes().close(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(dispute))
}

/// `GET /payments/balance` — Retrieves merchant available and pending funds via `stripe.balance().retrieve()`.
#[get("/payments/balance")]
pub async fn retrieve_balance(stripe: web::Data<StripeClient>) -> Result<HttpResponse, ServiceError> {
    let balance = stripe.balance().retrieve().await?;
    Ok(HttpResponse::Ok().json(balance))
}

/// `GET /payments/balance-transactions/{id}` — Retrieves an immutable ledger entry via
/// `stripe.balance_transactions().retrieve()`.
#[get("/payments/balance-transactions/{id}")]
pub async fn retrieve_balance_transaction(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, ServiceError> {
    let tx = stripe.balance_transactions().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(tx))
}

/// `GET /payments/balance-transactions` — Streams ledger entries using `Paginator<BalanceTransaction>`.
#[get("/payments/balance-transactions")]
pub async fn list_balance_transactions(
    stripe: web::Data<StripeClient>,
) -> Result<HttpResponse, ServiceError> {
    let mut paginator = stripe.balance_transactions().list(Some(10));
    let mut items = Vec::new();
    while let Some(item) = paginator.next_item().await? {
        items.push(item);
        if items.len() >= 10 {
            break;
        }
    }
    Ok(HttpResponse::Ok().json(items))
}

/// Mounts all Module 3 (Payments & Settlement) routes onto an Actix-Web [`web::ServiceConfig`].
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_payment_intent)
        .service(retrieve_payment_intent)
        .service(confirm_payment_intent)
        .service(capture_payment_intent)
        .service(cancel_payment_intent)
        .service(create_refund)
        .service(retrieve_refund)
        .service(update_refund)
        .service(cancel_refund)
        .service(retrieve_dispute)
        .service(update_dispute)
        .service(close_dispute)
        .service(retrieve_balance)
        .service(retrieve_balance_transaction)
        .service(list_balance_transactions);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let secret_key =
        std::env::var("STRIPE_SECRET_KEY").unwrap_or_else(|_| "sk_test_example_key".to_string());
    let stripe = web::Data::new(StripeClient::new(SecretKey::new(secret_key)));

    println!("Starting Actix-Web Payments & Settlement example on http://127.0.0.1:8082");
    HttpServer::new(move || {
        App::new()
            .app_data(stripe.clone())
            .configure(configure_routes)
    })
    .bind(("127.0.0.1", 8082))?
    .run()
    .await
}
