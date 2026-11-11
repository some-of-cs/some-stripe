//! # Actix-Web Example 3: Hosted Checkout Sessions & Payment Links (Module 4)
//!
//! Demonstrates how to integrate **Module 4 (Hosted Checkout & Payment Links)** of `some-stripe`
//! inside an `actix-web` application.
//!
//! ## Services Covered
//! - `stripe.checkout().sessions()`: `create`, `retrieve`, `expire`, `list_line_items_default`,
//!   `is_fulfillment_ready()`, and `validate_redirect_url()`
//! - `stripe.payment_links()` (and `stripe.checkout().payment_links()`): `create`, `retrieve`,
//!   `update`, and `list_line_items_default()`
//! - EU Omnibus Directive 2019/2161 Article 6a price reduction transparency
//!   (`validate_omnibus_price_transparency`) and tax transparency (`validate_tax_behavior_transparency`)
//!
//! ## Run the Example
//! ```bash
//! STRIPE_SECRET_KEY=sk_test_123 cargo run --example actix_checkout_and_payment_links
//! ```

use actix_web::{App, HttpResponse, HttpServer, ResponseError, get, post, web};
use serde::{Deserialize, Serialize};
use some_stripe::{
    CreateCheckoutSessionParams, CreatePaymentLinkParams, SecretKey, StripeClient, StripeError,
    TaxBehavior, UpdatePaymentLinkParams, is_fulfillment_ready,
    validate_omnibus_price_transparency, validate_redirect_url, validate_tax_behavior_transparency,
};
use std::fmt;

/// Error wrapper mapping `StripeError` and domain validation failures to HTTP responses.
#[derive(Debug)]
pub struct CheckoutApiError(pub StripeError);

impl fmt::Display for CheckoutApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<StripeError> for CheckoutApiError {
    fn from(err: StripeError) -> Self {
        Self(err)
    }
}

impl ResponseError for CheckoutApiError {
    fn error_response(&self) -> HttpResponse {
        match &self.0 {
            StripeError::InvalidRequest(msg) => HttpResponse::BadRequest().json(serde_json::json!({
                "error_type": "invalid_request_error",
                "message": msg,
            })),
            other => HttpResponse::BadGateway().json(serde_json::json!({
                "error_type": "stripe_error",
                "message": other.to_string(),
            })),
        }
    }
}

/// Request payload for validating EU Omnibus Directive 2019/2161 promotional pricing.
#[derive(Debug, Deserialize, Serialize)]
pub struct OmnibusPromotionCheckRequest {
    pub regular_price: i64,
    pub discounted_price: i64,
    pub prior_30d_lowest_price: Option<i64>,
    pub tax_behavior: TaxBehavior,
}

/// `POST /checkout/sessions` — Validates redirect URLs against an allowlist and creates a
/// Stripe Hosted Checkout Session via `stripe.checkout().sessions().create()`.
#[post("/checkout/sessions")]
pub async fn create_checkout_session(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateCheckoutSessionParams>,
) -> Result<HttpResponse, CheckoutApiError> {
    let allowed_domains = ["example.com", "checkout.example.com"];
    validate_redirect_url(&params.success_url, &allowed_domains)
        .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

    let session = stripe.checkout().sessions().create(&params).await?;
    Ok(HttpResponse::Created().json(session))
}

/// `GET /checkout/sessions/{id}` — Retrieves a Checkout Session and checks fulfillment readiness
/// (`is_fulfillment_ready`).
#[get("/checkout/sessions/{id}")]
pub async fn retrieve_checkout_session(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, CheckoutApiError> {
    let session = stripe.checkout().sessions().retrieve(id.as_str()).await?;
    let ready_to_fulfill = is_fulfillment_ready(&session);
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "session": session,
        "is_fulfillment_ready": ready_to_fulfill,
    })))
}

/// `POST /checkout/sessions/{id}/expire` — Expires an open Checkout Session via
/// `stripe.checkout().sessions().expire()`.
#[post("/checkout/sessions/{id}/expire")]
pub async fn expire_checkout_session(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, CheckoutApiError> {
    let session = stripe.checkout().sessions().expire(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(session))
}

/// `GET /checkout/sessions/{id}/line-items` — Streams line items for a Checkout Session using
/// `stripe.checkout().sessions().list_line_items_default()`.
#[get("/checkout/sessions/{id}/line-items")]
pub async fn list_checkout_session_line_items(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, CheckoutApiError> {
    let mut paginator = stripe
        .checkout()
        .sessions()
        .list_line_items_default(id.as_str());
    let mut items = Vec::new();
    while let Some(item) = paginator.next_item().await? {
        items.push(item);
        if items.len() >= 25 {
            break;
        }
    }
    Ok(HttpResponse::Ok().json(items))
}

/// `POST /payment-links` — Creates a shareable Stripe Payment Link via `stripe.payment_links().create()`.
#[post("/payment-links")]
pub async fn create_payment_link(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreatePaymentLinkParams>,
) -> Result<HttpResponse, CheckoutApiError> {
    let link = stripe.payment_links().create(&params).await?;
    Ok(HttpResponse::Created().json(link))
}

/// `GET /payment-links/{id}` — Retrieves a Payment Link via `stripe.payment_links().retrieve()`.
#[get("/payment-links/{id}")]
pub async fn retrieve_payment_link(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, CheckoutApiError> {
    let link = stripe.payment_links().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(link))
}

/// `POST /payment-links/{id}` — Updates a Payment Link (e.g. deactivating it) via
/// `stripe.payment_links().update()`.
#[post("/payment-links/{id}")]
pub async fn update_payment_link(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdatePaymentLinkParams>,
) -> Result<HttpResponse, CheckoutApiError> {
    let link = stripe.payment_links().update(id.as_str(), &params).await?;
    Ok(HttpResponse::Ok().json(link))
}

/// `GET /payment-links/{id}/line-items` — Streams line items for a Payment Link via
/// `stripe.payment_links().list_line_items_default()`.
#[get("/payment-links/{id}/line-items")]
pub async fn list_payment_link_line_items(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, CheckoutApiError> {
    let mut paginator = stripe
        .payment_links()
        .list_line_items_default(id.as_str());
    let mut items = Vec::new();
    while let Some(item) = paginator.next_item().await? {
        items.push(item);
        if items.len() >= 25 {
            break;
        }
    }
    Ok(HttpResponse::Ok().json(items))
}

/// `POST /checkout/validate-omnibus` — Validates EU Omnibus Directive 2019/2161 price reduction
/// and tax transparency rules before launching a promotion.
#[post("/checkout/validate-omnibus")]
pub async fn validate_omnibus_promotion(
    body: web::Json<OmnibusPromotionCheckRequest>,
) -> Result<HttpResponse, CheckoutApiError> {
    validate_omnibus_price_transparency(
        body.regular_price,
        body.discounted_price,
        body.prior_30d_lowest_price,
    )?;
    validate_tax_behavior_transparency(body.tax_behavior)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "compliant": true,
        "regular_price": body.regular_price,
        "discounted_price": body.discounted_price,
        "prior_30d_lowest_price": body.prior_30d_lowest_price,
    })))
}

/// Mounts all Module 4 (Hosted Checkout & Payment Links) routes onto an Actix-Web [`web::ServiceConfig`].
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_checkout_session)
        .service(retrieve_checkout_session)
        .service(expire_checkout_session)
        .service(list_checkout_session_line_items)
        .service(create_payment_link)
        .service(retrieve_payment_link)
        .service(update_payment_link)
        .service(list_payment_link_line_items)
        .service(validate_omnibus_promotion);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let secret_key =
        std::env::var("STRIPE_SECRET_KEY").unwrap_or_else(|_| "sk_test_example_key".to_string());
    let stripe = web::Data::new(StripeClient::new(SecretKey::new(secret_key)));

    println!("Starting Actix-Web Checkout & Payment Links example on http://127.0.0.1:8083");
    HttpServer::new(move || {
        App::new()
            .app_data(stripe.clone())
            .configure(configure_routes)
    })
    .bind(("127.0.0.1", 8083))?
    .run()
    .await
}
