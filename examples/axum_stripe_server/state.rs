//! Application state and unified error handling for the Axum Stripe integration.
//!
//! Uses `some-stripe`'s native `$VAR_NAME` secret resolution (`StripeClient::new("$STRIPE_SECRET_KEY")`
//! and `SecretKey::new("$STRIPE_WEBHOOK_SECRET")`) introduced in PR #150, which automatically resolves
//! secrets from the process environment or `.env.local`, `.env.dev`, `.env`, `.env.example`, etc.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use some_stripe::{
    MeterStreamCollector, SecretKey, StripeClient, StripeError, StripeWebhookError,
};
use std::sync::Arc;
use std::time::Duration;

/// Global shared application state for the Axum web server.
///
/// Designed with lock-free concurrency and zero-clone discipline (Gate G-Borrow).
/// Holding `Arc<StripeClient>` and [`SecretKey`] (`ZeroizeOnDrop` + `[REDACTED]` `Debug`)
/// allows handlers to cheaply share references across async task workers without deep heap allocations.
#[derive(Clone)]
pub struct AppState {
    /// Thread-safe Stripe SDK client instance.
    pub stripe: Arc<StripeClient>,
    /// Webhook endpoint signing secret (`SecretKey` with automatic `$VAR_NAME` resolution & zeroization on drop).
    pub webhook_secret: SecretKey,
    /// Shared active V2 meter stream collector for live telemetry and buffered ingestion.
    pub meter_collector: Arc<MeterStreamCollector>,
}

impl AppState {
    /// Creates a new `AppState` from provided secret keys or `$VAR_NAME` references
    /// (e.g. `AppState::new("$STRIPE_SECRET_KEY", "$STRIPE_WEBHOOK_SECRET")`).
    pub fn new(secret_key: impl Into<SecretKey>, webhook_secret: impl Into<SecretKey>) -> Self {
        Self::with_client(StripeClient::new(secret_key), webhook_secret)
    }

    /// Creates a new `AppState` by automatically resolving `"$STRIPE_SECRET_KEY"` and
    /// `"$STRIPE_WEBHOOK_SECRET"` from process environment or `.env.local`, `.env.dev`, `.env`, `.env.example`.
    pub fn from_env() -> Self {
        Self::new("$STRIPE_SECRET_KEY", "$STRIPE_WEBHOOK_SECRET")
    }

    /// Creates a new `AppState` from an existing [`StripeClient`] and webhook [`SecretKey`]
    /// (useful for custom `StripeConfig` or wire-mock testing).
    pub fn with_client(client: StripeClient, webhook_secret: impl Into<SecretKey>) -> Self {
        let engine = client.v2().billing().create_ingestion_engine();
        let collector = engine.create_stream_collector(1000, Duration::from_millis(500));
        Self {
            stripe: Arc::new(client),
            webhook_secret: webhook_secret.into(),
            meter_collector: Arc::new(collector),
        }
    }
}

/// Standardized API error response format adhering to RFC 7807 problem details.
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Machine-readable error code or category.
    pub code: String,
    /// Human-readable explanation of the error.
    pub message: String,
    /// Detailed diagnostic type.
    pub r#type: String,
}

/// Unified error taxonomy for the Axum Stripe service layer.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Standard Stripe API error returned by the SDK transport.
    #[error("Stripe API error: {0}")]
    Stripe(#[from] StripeError),

    /// Webhook verification or cryptographic signature failure.
    #[error("Stripe webhook verification error: {0}")]
    Webhook(#[from] StripeWebhookError),

    /// Missing or invalid HTTP headers or request parameters.
    #[error("Invalid or missing request parameter: {0}")]
    BadRequest(String),

    /// Internal application configuration or processing fault.
    #[error("Internal server error: {0}")]
    Internal(String),
}

impl AppError {
    /// Maps a [`StripeError`] into an HTTP status code and structured error payload.
    fn map_stripe_error(err: &StripeError) -> (StatusCode, String, String, String) {
        match err {
            StripeError::InvalidRequest(message) => (
                StatusCode::BAD_REQUEST,
                "invalid_request".to_string(),
                message.clone(),
                "invalid_request_error".to_string(),
            ),
            StripeError::Api {
                status,
                code,
                message,
                error_type,
                ..
            } => (
                StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_GATEWAY),
                code.clone(),
                message.clone(),
                error_type.clone(),
            ),
            StripeError::Webhook(webhook_err) => (
                StatusCode::BAD_REQUEST,
                "webhook_verification_failed".to_string(),
                webhook_err.to_string(),
                "invalid_request_error".to_string(),
            ),
            other => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "stripe_internal_error".to_string(),
                other.to_string(),
                "api_error".to_string(),
            ),
        }
    }

    /// Resolves this [`AppError`] into its HTTP status code and `(code, message, type)` tuple.
    fn into_status_and_parts(&self) -> (StatusCode, String, String, String) {
        match self {
            AppError::Webhook(err) => (
                StatusCode::BAD_REQUEST,
                "webhook_verification_failed".to_string(),
                err.to_string(),
                "invalid_request_error".to_string(),
            ),
            AppError::Stripe(stripe_err) => Self::map_stripe_error(stripe_err),
            AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                "bad_request".to_string(),
                msg.clone(),
                "invalid_request_error".to_string(),
            ),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error".to_string(),
                msg.clone(),
                "api_error".to_string(),
            ),
        }
    }
}

impl IntoResponse for AppError {
    /// Converts [`AppError`] into a standardized JSON HTTP response.
    fn into_response(self) -> Response {
        let (status, code, message, error_type) = self.into_status_and_parts();
        let body = Json(ErrorResponse {
            code,
            message,
            r#type: error_type,
        });
        (status, body).into_response()
    }
}
