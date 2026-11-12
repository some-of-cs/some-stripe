//! Hosted Checkout Sessions and Payment Links routes (Module 4).
//!
//! # Features
//! - Hosted Checkout Sessions supporting both one-time `payment` mode (inline `unit_amount`)
//!   and recurring `subscription` mode (via catalog `price_id`).
//! - Reusable Payment Links with EU Omnibus Directive 2019/2161 price transparency validation
//!   and inline `PaymentLinkPriceDataParams` binding so the charged amount is guaranteed to match
//!   the validated discounted amount.
//! - Expiring pending sessions and retrieving session states.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use some_stripe::{
    validate_omnibus_price_transparency, CreateCheckoutSessionParams, CreatePaymentLinkParams,
    LineItemParams, OmnibusPriceTransparencyMetadata, PaymentLinkLineItemParams,
    PaymentLinkPriceDataParams,
};

use crate::state::{AppError, AppState};

/// Creates the checkout router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/checkout/sessions", post(create_checkout_session))
        .route("/checkout/sessions/{id}", get(get_checkout_session))
        .route("/checkout/sessions/{id}/expire", post(expire_checkout_session))
        .route("/checkout/payment-links", post(create_payment_link))
        .route("/checkout/payment-links/{id}", get(get_payment_link))
}

/// Request body for creating a CheckoutSession.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateCheckoutRequest {
    pub success_url: String,
    pub cancel_url: Option<String>,
    pub mode: Option<String>,
    pub client_reference_id: Option<String>,
    pub currency: Option<String>,
    pub unit_amount: Option<i64>,
    pub product_name: Option<String>,
    /// Recurring or catalog Price ID (`price_...`), required when `mode == "subscription"`.
    pub price_id: Option<String>,
    pub quantity: u32,
    pub idempotency_key: Option<String>,
}

/// Builds mode-specific [`CreateCheckoutSessionParams`], ensuring subscription mode uses a recurring Price ID.
fn build_checkout_session_params(
    payload: CreateCheckoutRequest,
) -> Result<CreateCheckoutSessionParams, AppError> {
    let mode = payload.mode.unwrap_or_else(|| "payment".to_string());
    if mode == "subscription" {
        let price_id = payload.price_id.ok_or_else(|| {
            AppError::BadRequest(
                "price_id is required when checkout mode is 'subscription'".to_string(),
            )
        })?;
        let line_item = LineItemParams {
            price: Some(price_id),
            quantity: Some(payload.quantity),
            price_data: None,
        };
        return Ok(CreateCheckoutSessionParams {
            success_url: payload.success_url,
            cancel_url: payload.cancel_url,
            mode,
            client_reference_id: payload.client_reference_id,
            customer: None,
            customer_email: None,
            expires_at: None,
            metadata: None,
            idempotency_key: payload.idempotency_key,
            line_items: Some(vec![line_item]),
            currency: None,
            unit_amount: None,
            product_name: None,
            quantity: None,
        });
    }

    let currency = payload
        .currency
        .ok_or_else(|| AppError::BadRequest("currency is required for payment mode".to_string()))?;
    let unit_amount = payload.unit_amount.ok_or_else(|| {
        AppError::BadRequest("unit_amount is required for payment mode".to_string())
    })?;
    let product_name = payload.product_name.ok_or_else(|| {
        AppError::BadRequest("product_name is required for payment mode".to_string())
    })?;

    Ok(CreateCheckoutSessionParams {
        success_url: payload.success_url,
        cancel_url: payload.cancel_url,
        mode,
        client_reference_id: payload.client_reference_id,
        customer: None,
        customer_email: None,
        expires_at: None,
        metadata: None,
        idempotency_key: payload.idempotency_key,
        line_items: None,
        currency: Some(currency.to_lowercase()),
        unit_amount: Some(unit_amount),
        product_name: Some(product_name),
        quantity: Some(payload.quantity),
    })
}

/// Create a new hosted Checkout Session.
pub async fn create_checkout_session(
    State(state): State<AppState>,
    Json(payload): Json<CreateCheckoutRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = build_checkout_session_params(payload)?;
    let session = state.stripe.checkout().sessions.create(&params).await?;
    Ok((StatusCode::CREATED, Json(session)))
}

/// Retrieve an existing Checkout Session by ID.
pub async fn get_checkout_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let session = state.stripe.checkout().sessions.retrieve(&id).await?;
    Ok((StatusCode::OK, Json(session)))
}

/// Expire a Checkout Session.
pub async fn expire_checkout_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let session = state.stripe.checkout().sessions.expire(&id).await?;
    Ok((StatusCode::OK, Json(session)))
}

/// Request body for creating a reusable Payment Link.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreatePaymentLinkRequest {
    /// Catalog Price ID (`price_...`) when no promotional Omnibus discount is applied.
    pub price_id: Option<String>,
    /// Product ID (`prod_...`) used to bind inline `price_data` when `current_discounted_price` is specified.
    pub product_id: Option<String>,
    /// Three-letter ISO 4217 currency code (default `"usd"`).
    pub currency: Option<String>,
    pub quantity: u32,
    /// Regular baseline price in minor units for EU Omnibus Directive 2019/2161 validation.
    pub regular_price: Option<i64>,
    /// Current announced discounted price in minor units.
    pub current_discounted_price: Option<i64>,
    /// Prior 30-day lowest price in minor units.
    pub previous_lowest_price: Option<i64>,
}

/// Builds the [`CreatePaymentLinkParams`] ensuring discounted amounts are directly bound via `price_data`.
fn build_payment_link_params(
    payload: CreatePaymentLinkRequest,
) -> Result<CreatePaymentLinkParams, AppError> {
    if let (Some(regular), Some(curr)) = (payload.regular_price, payload.current_discounted_price) {
        validate_omnibus_price_transparency(regular, curr, payload.previous_lowest_price)
            .map_err(|e| AppError::BadRequest(format!("EU Omnibus Directive violation: {e}")))?;

        let transparency =
            OmnibusPriceTransparencyMetadata::new(regular, curr, payload.previous_lowest_price)
                .map_err(|e| AppError::BadRequest(format!("EU Omnibus metadata error: {e}")))?;

        let product = payload
            .product_id
            .unwrap_or_else(|| "prod_omnibus_default".to_string());
        let currency = payload
            .currency
            .unwrap_or_else(|| "usd".to_string())
            .to_lowercase();

        let line_item = PaymentLinkLineItemParams {
            price: None,
            quantity: payload.quantity,
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

    let price_id = payload.price_id.ok_or_else(|| {
        AppError::BadRequest(
            "Either price_id or (regular_price and current_discounted_price) must be provided"
                .to_string(),
        )
    })?;

    let line_item = PaymentLinkLineItemParams {
        price: Some(price_id),
        quantity: payload.quantity,
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

/// Create a reusable Payment Link with statutory EU Omnibus price compliance validation.
pub async fn create_payment_link(
    State(state): State<AppState>,
    Json(payload): Json<CreatePaymentLinkRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = build_payment_link_params(payload)?;
    let link = state.stripe.payment_links().create(&params).await?;
    Ok((StatusCode::CREATED, Json(link)))
}

/// Retrieve a Payment Link by ID.
pub async fn get_payment_link(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let link = state.stripe.payment_links().retrieve(&id).await?;
    Ok((StatusCode::OK, Json(link)))
}
