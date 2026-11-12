//! # Actix-Web Example 1: Microkernel Core, Dual-Protocol Serialization & Cryptographic Webhooks
//!
//! Demonstrates how to integrate **Module 1 (Microkernel Core Runtime)** and **Module 2
//! (Cryptographic Webhooks)** of `some-stripe` inside an `actix-web` application.
//!
//! ## Services Covered
//! - `StripeClient` & `StripeConfig`: Lock-free `Arc`-backed shared state via `web::Data<StripeClient>`
//! - `CustomErrorWorkflowBuilder`: Jittered exponential backoff & security incident escalation hooks
//! - `RequestOptions` & `Nullable<T>`: Multi-tenant `Stripe-Account`, `Idempotency-Key`, and 3-state
//!   nullability across API v1 (`serialize_v1_form`) and API v2 (`serialize_v2_json`)
//! - `stripe.webhooks()`: Constant-time HMAC-SHA256 verification (`construct_event`,
//!   `verify_header_with_config`, `construct_v2_thin_event`), `StripeWebhookWrapper::from_raw`,
//!   1 MB payload cap (`MAX_WEBHOOK_PAYLOAD_BYTES`), and `EventObject` discriminated union dispatch
//!
//! ## Run the Example
//! ```bash
//! STRIPE_SECRET_KEY=sk_test_123 STRIPE_WEBHOOK_SECRET=whsec_test_123 \
//!   cargo run --example actix_core_and_webhooks
//! ```

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, ResponseError, post, get, web};
use serde::{Deserialize, Serialize};
use some_stripe::core::patterns::{Nullable, serialize_v1_form, serialize_v2_json};
use some_stripe::core::workflow::CustomErrorWorkflowBuilder;
use some_stripe::{
    EventObject, MAX_WEBHOOK_PAYLOAD_BYTES, RequestOptions, SecretKey, StripeClient, StripeConfig,
    StripeError, StripeWebhookError, StripeWebhookWrapper, ToleranceConfig, is_thin_event,
};
use std::fmt;
use std::time::Duration;

/// Application state shared across Actix-Web worker threads with zero deep cloning.
///
/// `StripeClient` wraps `Arc<ClientInner>` and `SecretKey` implements `ZeroizeOnDrop`
/// with `[REDACTED]` `Debug` formatting.
pub struct WebhookAppState {
    pub stripe: StripeClient,
    pub webhook_secret: SecretKey,
}

/// Unified HTTP error response adapter for `StripeError` and `StripeWebhookError`.
#[derive(Debug, Serialize)]
pub struct ApiErrorResponse {
    pub error_type: &'static str,
    pub message: String,
}

/// Actix-Web compatible error wrapper with flat cyclomatic complexity (`CC <= 3`).
#[derive(Debug)]
pub enum AppError {
    Stripe(StripeError),
    Webhook(StripeWebhookError),
    MissingSignatureHeader,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stripe(err) => write!(f, "{err}"),
            Self::Webhook(err) => write!(f, "{err}"),
            Self::MissingSignatureHeader => write!(f, "Missing Stripe-Signature HTTP header"),
        }
    }
}

impl From<StripeError> for AppError {
    fn from(err: StripeError) -> Self {
        Self::Stripe(err)
    }
}

impl From<StripeWebhookError> for AppError {
    fn from(err: StripeWebhookError) -> Self {
        Self::Webhook(err)
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            Self::Stripe(err) => HttpResponse::BadGateway().json(ApiErrorResponse {
                error_type: "stripe_api_error",
                message: err.to_string(),
            }),
            Self::Webhook(err) => HttpResponse::BadRequest().json(ApiErrorResponse {
                error_type: "stripe_webhook_verification_error",
                message: err.to_string(),
            }),
            Self::MissingSignatureHeader => HttpResponse::BadRequest().json(ApiErrorResponse {
                error_type: "missing_signature_header",
                message: self.to_string(),
            }),
        }
    }
}

/// Builds a production-hardened [`StripeClient`] with custom retry policies and security hooks.
#[must_use]
pub fn build_configured_client(secret_key: &str, api_base: Option<&str>) -> StripeClient {
    let workflow = CustomErrorWorkflowBuilder::new()
        .max_retries(3)
        .initial_delay(Duration::from_millis(250))
        .on_security_incident(|err, payload_size| {
            eprintln!("SECURITY INCIDENT ({payload_size} bytes): {err:?}");
        })
        .on_api_error(|status, code, message| {
            eprintln!("Stripe API error [HTTP {status} / {code}]: {message}");
        })
        .build();

    let mut config = StripeConfig::new(SecretKey::new(secret_key))
        .with_timeout(Duration::from_secs(15))
        .with_max_retries(3)
        .with_error_workflow(workflow);

    if let Some(base) = api_base {
        config = config.with_api_base(base);
    }

    StripeClient::with_config(config)
}

/// Request payload demonstrating 3-state `Nullable<T>` dual-protocol encoding.
#[derive(Debug, Deserialize, Serialize)]
pub struct DualProtocolDemoInput {
    pub email: Option<String>,
    pub clear_description: bool,
    pub stripe_account: Option<String>,
    pub idempotency_key: Option<String>,
}

/// Wire preview struct serialized to both Stripe v1 form-urlencoded and v2 JSON.
#[derive(Serialize)]
struct WirePatchPayload<'a> {
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    email: Nullable<&'a str>,
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    description: Nullable<&'a str>,
}

/// `POST /core/dual-protocol-preview` — Demonstrates `Nullable<T>`, `serialize_v1_form`,
/// `serialize_v2_json`, and `RequestOptions` header construction without heap cloning.
#[post("/core/dual-protocol-preview")]
pub async fn preview_dual_protocol(
    body: web::Json<DualProtocolDemoInput>,
) -> Result<HttpResponse, AppError> {
    let email_nullable = match body.email.as_deref() {
        Some(value) => Nullable::Value(value),
        None => Nullable::Omit,
    };
    let description_nullable = if body.clear_description {
        Nullable::Null
    } else {
        Nullable::Omit
    };

    let patch = WirePatchPayload {
        email: email_nullable,
        description: description_nullable,
    };

    let v1_form = serialize_v1_form(&patch)?;
    let v2_json = serialize_v2_json(&patch)?;

    let mut options = RequestOptions::new();
    if let Some(acct) = body.stripe_account.as_deref() {
        options = options.stripe_account(acct);
    }
    if let Some(idem) = body.idempotency_key.as_deref() {
        options = options.idempotency_key(idem);
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "v1_form_urlencoded": v1_form,
        "v2_json_payload": v2_json,
        "stripe_account": options.stripe_account,
        "idempotency_key": options.idempotency_key,
        "max_webhook_payload_bytes": MAX_WEBHOOK_PAYLOAD_BYTES,
    })))
}

/// Extracts the `Stripe-Signature` header slice from an [`HttpRequest`].
fn extract_signature_header(req: &HttpRequest) -> Result<&str, AppError> {
    req.headers()
        .get("Stripe-Signature")
        .and_then(|val| val.to_str().ok())
        .ok_or(AppError::MissingSignatureHeader)
}

/// Classifies the deserialized [`EventObject`] discriminated union without cloning.
fn describe_event_object(object: &EventObject) -> &'static str {
    match object {
        EventObject::CheckoutSession(_) => "checkout.session",
        EventObject::PaymentIntent(_) => "payment_intent",
        EventObject::Invoice(_) => "invoice",
        EventObject::Dispute(_) => "dispute",
        EventObject::Refund(_) => "refund",
        EventObject::PaymentLink(_) => "payment_link",
        EventObject::Unknown => "unknown",
    }
}

/// `POST /webhooks/v1` — Verifies incoming Stripe API v1 webhooks in constant time
/// (`subtle::ConstantTimeEq`) with 300-second replay drift protection and 1 MB payload cap.
#[post("/webhooks/v1")]
pub async fn handle_v1_webhook(
    req: HttpRequest,
    payload: web::Bytes,
    state: web::Data<WebhookAppState>,
) -> Result<HttpResponse, AppError> {
    let sig_header = extract_signature_header(&req)?;
    let secret = state.webhook_secret.as_str();

    // Verify using StripeWebhookWrapper::from_raw (enforces 1 MB cap + 300s drift tolerance)
    let wrapper = StripeWebhookWrapper::from_raw(&payload, sig_header, secret, None)?;

    // Also demonstrate WebhooksResource::construct_event_with_config & EventObject union
    let tolerance = ToleranceConfig::default();
    let event = state
        .stripe
        .webhooks()
        .construct_event_with_config(&payload, sig_header, secret, &tolerance)?;
    let typed_object = event.data.deserialize_object().unwrap_or(EventObject::Unknown);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "received": true,
        "event_id": wrapper.event_id(),
        "event_type": wrapper.event_type(),
        "is_checkout_completed": wrapper.is_checkout_completed(),
        "object_variant": describe_event_object(&typed_object),
    })))
}

/// `POST /webhooks/v2-thin` — Probes and verifies a Stripe API v2 Thin Event notification
/// using `is_thin_event` and `stripe.webhooks().construct_v2_thin_event()`.
#[post("/webhooks/v2-thin")]
pub async fn handle_v2_thin_webhook(
    req: HttpRequest,
    payload: web::Bytes,
    state: web::Data<WebhookAppState>,
) -> Result<HttpResponse, AppError> {
    let sig_header = extract_signature_header(&req)?;
    let thin_detected = is_thin_event(&payload);
    let thin_event = state.stripe.webhooks().construct_v2_thin_event(
        &payload,
        sig_header,
        state.webhook_secret.as_str(),
        None,
    )?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "received": true,
        "thin_detected": thin_detected,
        "event_id": thin_event.id,
        "event_type": thin_event.r#type,
        "related_object_id": thin_event.related_object.as_ref().map(|r| r.id.as_str()),
    })))
}

/// `GET /health` — Reports client configuration health without exposing secret keys.
#[get("/health")]
pub async fn health(state: web::Data<WebhookAppState>) -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "actix_core_and_webhooks",
        "api_base": state.stripe.config.api_base,
        "secret_debug_redacted": format!("{:?}", state.webhook_secret),
    }))
}

/// Configures the Core & Webhooks routes on an Actix-Web [`web::ServiceConfig`].
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(health)
        .service(preview_dual_protocol)
        .service(handle_v1_webhook)
        .service(handle_v2_thin_webhook);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Automatically resolves $STRIPE_SECRET_KEY and $STRIPE_WEBHOOK_SECRET from env, .env.local, .env.dev, .env, or .env.example
    let state = web::Data::new(WebhookAppState {
        stripe: build_configured_client("$STRIPE_SECRET_KEY", None),
        webhook_secret: SecretKey::new("$STRIPE_WEBHOOK_SECRET"),
    });

    println!("Starting Actix-Web Core & Webhooks example on http://127.0.0.1:8081");
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .configure(configure_routes)
    })
    .bind(("127.0.0.1", 8081))?
    .run()
    .await
}
