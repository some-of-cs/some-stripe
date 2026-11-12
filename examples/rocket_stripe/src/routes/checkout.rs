//! Hosted Checkout and Payment Links routes (Module 4).
//!
//! Provides endpoints to:
//! - Create and manage hosted Stripe Checkout Sessions (`/api/checkout/sessions`).
//! - Create and inspect EU Omnibus Directive compliant Payment Links (`/api/checkout/payment-links`).

use rocket::serde::json::Json;
use rocket::State;
use some_stripe::{
    validate_omnibus_price_transparency, CheckoutSession, CreateCheckoutSessionParams,
    CreatePaymentLinkParams, PaymentLink, PaymentLinkLineItemParams,
};

use crate::config::AppConfig;
use crate::models::{
    ApiResponse, CreateCheckoutRequest, CreatePaymentLinkRequest, ErrorResponse,
};

/// Initiates a hosted Stripe Checkout Session.
///
/// Route: `POST /api/checkout/sessions`
#[rocket::post("/checkout/sessions", data = "<req>")]
pub async fn create_checkout_session(
    config: &State<AppConfig>,
    req: Json<CreateCheckoutRequest>,
) -> Result<Json<ApiResponse<CheckoutSession>>, ErrorResponse> {
    let success_url = req
        .success_url
        .clone()
        .unwrap_or_else(|| config.default_success_url.clone());
    let cancel_url = req
        .cancel_url
        .clone()
        .or_else(|| Some(config.default_cancel_url.clone()));

    let params = CreateCheckoutSessionParams {
        success_url,
        cancel_url,
        mode: "payment".to_string(),
        currency: Some(req.currency.clone()),
        unit_amount: Some(req.unit_amount),
        product_name: Some(req.product_name.clone()),
        quantity: Some(req.quantity),
        ..Default::default()
    };

    let session = config
        .client
        .checkout()
        .sessions()
        .create(&params)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to create checkout session: {e}")))?;

    Ok(Json(ApiResponse::new(session)))
}

/// Retrieves an existing Stripe Checkout Session by ID.
///
/// Route: `GET /api/checkout/sessions/<id>`
#[rocket::get("/checkout/sessions/<id>")]
pub async fn get_checkout_session(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<CheckoutSession>>, ErrorResponse> {
    let session = config
        .client
        .checkout()
        .sessions()
        .retrieve(id)
        .await
        .map_err(|e| ErrorResponse::not_found(format!("Checkout session '{id}' not found: {e}")))?;

    Ok(Json(ApiResponse::new(session)))
}

/// Expires an active Stripe Checkout Session before customer completion.
///
/// Route: `POST /api/checkout/sessions/<id>/expire`
#[rocket::post("/checkout/sessions/<id>/expire")]
pub async fn expire_checkout_session(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<CheckoutSession>>, ErrorResponse> {
    let session = config
        .client
        .checkout()
        .sessions()
        .expire(id)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to expire session '{id}': {e}")))?;

    Ok(Json(ApiResponse::new(session)))
}

/// Creates a Payment Link with statutory EU Omnibus Directive price transparency validation.
///
/// Route: `POST /api/checkout/payment-links`
#[rocket::post("/checkout/payment-links", data = "<req>")]
pub async fn create_payment_link(
    config: &State<AppConfig>,
    req: Json<CreatePaymentLinkRequest>,
) -> Result<Json<ApiResponse<PaymentLink>>, ErrorResponse> {
    if let (Some(regular), Some(discounted)) = (req.regular_price, req.announced_discounted_price) {
        validate_omnibus_price_transparency(regular, discounted, req.prior_30d_lowest_price)
            .map_err(|e| ErrorResponse::bad_request(format!("Omnibus compliance violation: {e}")))?;
    }

    let line_items = req
        .line_items
        .iter()
        .map(|item| PaymentLinkLineItemParams {
            price: item.price.clone(),
            quantity: item.quantity,
            adjustable_quantity: None,
        })
        .collect();

    let params = CreatePaymentLinkParams {
        line_items,
        active: Some(true),
        after_completion: None,
        allow_promotion_codes: Some(true),
        automatic_tax: None,
        billing_address_collection: None,
        currency: None,
        customer_creation: None,
        idempotency_key: None,
        metadata: None,
        payment_method_types: None,
        shipping_address_collection: None,
        submit_type: None,
        tax_id_collection: None,
    };

    let link = config
        .client
        .payment_links()
        .create(&params)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to create payment link: {e}")))?;

    Ok(Json(ApiResponse::new(link)))
}

/// Retrieves a Payment Link by ID.
///
/// Route: `GET /api/checkout/payment-links/<id>`
#[rocket::get("/checkout/payment-links/<id>")]
pub async fn get_payment_link(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<PaymentLink>>, ErrorResponse> {
    let link = config
        .client
        .payment_links()
        .retrieve(id)
        .await
        .map_err(|e| ErrorResponse::not_found(format!("Payment link '{id}' not found: {e}")))?;

    Ok(Json(ApiResponse::new(link)))
}
