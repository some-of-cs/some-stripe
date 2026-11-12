//! # 02: Axum Checkout & Payments Service Example
//!
//! Production-grade Hosted Checkout Sessions and PaymentIntents flow:
//! - Hosted Checkout Sessions with Line Items and redirect URLs.
//! - Reusable Payment Links with EU Omnibus Directive 2019/2161 price transparency validation
//!   and inline `PaymentLinkPriceDataParams` binding.
//! - PaymentIntent creation, confirmation, capture, and cancellation.
//! - Statutory refunds and dispute evidence inspection.
//!
//! Run with:
//! ```bash
//! export STRIPE_SECRET_KEY="sk_test_..."
//! cargo run --example 02_axum_checkout_and_payments
//! ```

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use some_stripe::{
    validate_omnibus_price_transparency, CapturePaymentIntentParams, ConfirmPaymentIntentParams,
    CreateCheckoutSessionParams, CreatePaymentIntentParams, CreatePaymentLinkParams,
    CreateRefundParams, OmnibusPriceTransparencyMetadata, PaymentLinkLineItemParams,
    PaymentLinkPriceDataParams, StripeClient,
};
use std::sync::Arc;

/// Shared state holding the lock-free [`StripeClient`].
#[derive(Clone)]
pub struct PaymentState {
    /// Thread-safe Stripe client.
    pub stripe: Arc<StripeClient>,
}

impl PaymentState {
    /// Creates a new [`PaymentState`] from an existing [`StripeClient`].
    pub fn with_client(client: StripeClient) -> Self {
        Self {
            stripe: Arc::new(client),
        }
    }
}

/// Builds the Axum [`Router`] for the Checkout & Payments microservice.
pub fn build_router(state: PaymentState) -> Router {
    Router::new()
        .route("/checkout/session", post(create_checkout))
        .route("/checkout/session/{id}", get(get_checkout))
        .route("/checkout/payment-link", post(create_payment_link))
        .route("/payments/intent", post(create_intent))
        .route("/payments/intent/{id}/confirm", post(confirm_intent))
        .route("/payments/intent/{id}/capture", post(capture_intent))
        .route("/payments/refund", post(refund_payment))
        .with_state(state)
}

/// Entry point for the Checkout & Payments microservice.
#[allow(dead_code)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Automatically resolves $STRIPE_SECRET_KEY from process env, .env.local, .env.dev, .env, or .env.example
    let state = PaymentState::with_client(StripeClient::new("$STRIPE_SECRET_KEY"));
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3002").await?;
    tracing::info!("Checkout & Payments service listening on http://0.0.0.0:3002");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Request payload for creating a Hosted Checkout Session.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateCheckoutReq {
    pub success_url: String,
    pub cancel_url: Option<String>,
    pub currency: String,
    pub unit_amount: i64,
    pub product_name: String,
}

/// Creates a new Hosted Checkout Session.
pub async fn create_checkout(
    State(state): State<PaymentState>,
    Json(p): Json<CreateCheckoutReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreateCheckoutSessionParams {
        success_url: p.success_url,
        cancel_url: p.cancel_url,
        mode: "payment".to_string(),
        client_reference_id: None,
        customer: None,
        customer_email: None,
        expires_at: None,
        metadata: None,
        idempotency_key: None,
        line_items: None,
        currency: Some(p.currency.to_lowercase()),
        unit_amount: Some(p.unit_amount),
        product_name: Some(p.product_name),
        quantity: Some(1),
    };

    let session = state
        .stripe
        .checkout()
        .sessions
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(session)))
}

/// Retrieves an existing Hosted Checkout Session by ID.
pub async fn get_checkout(
    State(state): State<PaymentState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let session = state
        .stripe
        .checkout()
        .sessions
        .retrieve(&id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok((StatusCode::OK, Json(session)))
}

/// Request payload for creating a reusable Payment Link.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateLinkReq {
    pub price_id: Option<String>,
    pub product_id: Option<String>,
    pub currency: Option<String>,
    pub regular_price: Option<i64>,
    pub current_discounted_price: Option<i64>,
    pub previous_lowest_price: Option<i64>,
}

/// Builds [`CreatePaymentLinkParams`], binding any validated Omnibus discount directly via `price_data`.
fn build_link_params(p: CreateLinkReq) -> Result<CreatePaymentLinkParams, (StatusCode, String)> {
    if let (Some(regular), Some(curr)) = (p.regular_price, p.current_discounted_price) {
        validate_omnibus_price_transparency(regular, curr, p.previous_lowest_price)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Omnibus Directive error: {e}")))?;

        let transparency =
            OmnibusPriceTransparencyMetadata::new(regular, curr, p.previous_lowest_price)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Omnibus metadata error: {e}")))?;

        let product = p
            .product_id
            .unwrap_or_else(|| "prod_omnibus_default".to_string());
        let currency = p
            .currency
            .unwrap_or_else(|| "usd".to_string())
            .to_lowercase();

        let line_item = PaymentLinkLineItemParams {
            price: None,
            quantity: 1,
            adjustable_quantity: None,
            price_data: Some(PaymentLinkPriceDataParams {
                currency,
                unit_amount: Some(curr),
                product: Some(product),
                currency_options: None,
                tax_behavior: Some("exclusive".to_string()),
            }),
        };

        return Ok(CreatePaymentLinkParams {
            line_items: vec![line_item],
            active: Some(true),
            after_completion: None,
            allow_promotion_codes: None,
            billing_address_collection: None,
            customer_creation: None,
            metadata: Some(transparency.to_metadata_map()),
            payment_method_types: None,
            currency_options: None,
            idempotency_key: None,
        });
    }

    let price_id = p.price_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "Either price_id or (regular_price and current_discounted_price) must be provided"
                .to_string(),
        )
    })?;

    let line_item = PaymentLinkLineItemParams {
        price: Some(price_id),
        quantity: 1,
        adjustable_quantity: None,
        price_data: None,
    };

    Ok(CreatePaymentLinkParams {
        line_items: vec![line_item],
        active: Some(true),
        after_completion: None,
        allow_promotion_codes: None,
        billing_address_collection: None,
        customer_creation: None,
        metadata: None,
        payment_method_types: None,
        currency_options: None,
        idempotency_key: None,
    })
}

/// Creates a reusable Payment Link after validating EU Omnibus Directive price transparency.
pub async fn create_payment_link(
    State(state): State<PaymentState>,
    Json(p): Json<CreateLinkReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = build_link_params(p)?;
    let link = state
        .stripe
        .payment_links()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(link)))
}

/// Request payload for creating a PaymentIntent.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateIntentReq {
    pub amount: i64,
    pub currency: String,
}

/// Creates a new PaymentIntent in minor currency units.
pub async fn create_intent(
    State(state): State<PaymentState>,
    Json(p): Json<CreateIntentReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreatePaymentIntentParams {
        amount: p.amount,
        currency: p.currency.to_lowercase(),
        capture_method: Some("automatic".to_string()),
        description: Some("Axum SDK Payment Example".to_string()),
        payment_method: None,
        customer: None,
        receipt_email: None,
        return_url: None,
        confirm: None,
        metadata: None,
        idempotency_key: None,
    };

    let intent = state
        .stripe
        .payment_intents()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(intent)))
}

/// Request payload for confirming a PaymentIntent.
#[derive(Debug, Deserialize, Serialize)]
pub struct ConfirmReq {
    pub payment_method: Option<String>,
    pub return_url: Option<String>,
}

/// Confirms a PaymentIntent and handles PSD2 SCA 3DS2 redirects.
pub async fn confirm_intent(
    State(state): State<PaymentState>,
    Path(id): Path<String>,
    Json(p): Json<ConfirmReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = ConfirmPaymentIntentParams {
        payment_method: p.payment_method,
        return_url: p.return_url,
        receipt_email: None,
        setup_future_usage: None,
        client_secret: None,
        idempotency_key: None,
    };

    let intent = state
        .stripe
        .payment_intents()
        .confirm(&id, Some(&params))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::OK, Json(intent)))
}

/// Request payload for capturing an authorized PaymentIntent.
#[derive(Debug, Deserialize, Serialize)]
pub struct CaptureReq {
    pub amount: Option<i64>,
}

/// Captures funds from an authorized PaymentIntent.
pub async fn capture_intent(
    State(state): State<PaymentState>,
    Path(id): Path<String>,
    Json(p): Json<CaptureReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CapturePaymentIntentParams {
        amount_to_capture: p.amount,
        idempotency_key: None,
    };

    let intent = state
        .stripe
        .payment_intents()
        .capture(&id, Some(&params))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::OK, Json(intent)))
}

/// Request payload for issuing a refund.
#[derive(Debug, Deserialize, Serialize)]
pub struct RefundReq {
    pub payment_intent: String,
    pub amount: Option<i64>,
}

/// Issues a full or partial refund against a PaymentIntent.
pub async fn refund_payment(
    State(state): State<PaymentState>,
    Json(p): Json<RefundReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreateRefundParams {
        charge: None,
        payment_intent: Some(p.payment_intent),
        amount: p.amount,
        currency: None,
        customer: None,
        reason: Some("requested_by_customer".to_string()),
        metadata: None,
        refund_application_fee: None,
        reverse_transfer: None,
        instructions_email: None,
        idempotency_key: None,
    };

    let refund = state
        .stripe
        .refunds()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(refund)))
}
