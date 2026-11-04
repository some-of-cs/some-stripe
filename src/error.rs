//! Error types for the Stripe API client and webhook verification.

use thiserror::Error;

/// Main error type returned by Stripe API operations.
#[derive(Debug, Error)]
pub enum StripeError {
    /// An error returned by the Stripe REST API.
    #[error("Stripe API error (HTTP {status}): {message} [{error_type}:{code}]")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Error code (e.g., `resource_missing`, `parameter_invalid_empty`).
        code: String,
        /// Human-readable error message.
        message: String,
        /// Error classification (e.g., `invalid_request_error`, `api_error`).
        error_type: String,
        /// URL with additional documentation, if provided by Stripe.
        doc_url: Option<String>,
    },

    /// An HTTP transport error.
    #[error("HTTP transport error: {0}")]
    Http(#[from] reqwest::Error),

    /// A JSON serialization or deserialization error.
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid configuration supplied to the client.
    #[error("Configuration error: {0}")]
    Config(String),

    /// A webhook verification or payload extraction error.
    #[error("Webhook error: {0}")]
    Webhook(#[from] StripeWebhookError),
}

/// Errors occurring during webhook signature verification and event parsing.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum StripeWebhookError {
    /// The `Stripe-Signature` header is missing or empty.
    #[error("Missing or empty Stripe-Signature header")]
    MissingSignatureHeader,

    /// The webhook secret is missing or empty.
    #[error("Missing or empty webhook secret")]
    MissingSecret,

    /// The payload slice is empty.
    #[error("Empty webhook payload")]
    EmptyPayload,

    /// The `Stripe-Signature` header does not contain a valid `t=` timestamp parameter.
    #[error("Invalid timestamp in Stripe-Signature header")]
    InvalidTimestamp,

    /// No `v1=` signature schemes were found in the header.
    #[error("No v1 signature scheme found in Stripe-Signature header")]
    MissingSignature,

    /// Cryptographic HMAC-SHA256 signature mismatch (forged or tampered).
    #[error("Cryptographic signature mismatch")]
    BadSignature,

    /// The event timestamp is older than the configured tolerance window.
    #[error("Timestamp outside the tolerance zone ({timestamp} vs current {now}, tolerance {tolerance}s)")]
    TimestampOutsideTolerance {
        /// The timestamp on the event.
        timestamp: i64,
        /// Current UNIX epoch timestamp.
        now: i64,
        /// Permitted tolerance window in seconds.
        tolerance: u64,
    },

    /// The JSON body could not be parsed into a Stripe `Event`.
    #[error("Failed to parse Stripe webhook JSON: {0}")]
    MalformedPayload(String),
}
