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

    /// Invalid request parameter provided prior to network transmission.
    #[error("Invalid request parameter: {0}")]
    InvalidRequest(String),

    /// An HTTP transport error.
    #[error("HTTP transport error: {0}")]
    Http(#[from] reqwest::Error),

    /// Interrupted socket read or network drop.
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// A JSON serialization or deserialization error.
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid configuration supplied to the client.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Pagination infinite loop intercepted.
    #[error("Pagination loop detected on page {page} with cursor '{cursor}'")]
    PaginationLoopDetected {
        /// Repeating cursor token.
        cursor: String,
        /// Page number on which loop was detected.
        page: usize,
    },

    /// A webhook verification or payload extraction error.
    #[error("Webhook error: {0}")]
    Webhook(#[from] StripeWebhookError),

    /// A redirect URL validation error.
    #[error("Redirect error: {0}")]
    Redirect(#[from] RedirectError),
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

    /// The webhook secret is invalid, corrupted, or consists solely of whitespace.
    #[error("Invalid or whitespace-only webhook secret")]
    BadKey,

    /// The payload slice is empty.
    #[error("Empty webhook payload")]
    EmptyPayload,

    /// Webhook payload exceeds maximum allowable memory limit (Gate G2).
    #[error("Webhook payload size {size} bytes exceeds maximum allowed limit {max_allowed} bytes")]
    PayloadTooLarge {
        /// Payload size in bytes.
        size: usize,
        /// Maximum allowed bytes (1 MB).
        max_allowed: usize,
    },

    /// The `Stripe-Signature` header is invalid, oversized, or contains excessive signatures.
    #[error("Invalid Stripe-Signature header: {0}")]
    InvalidHeader(&'static str),

    /// The `Stripe-Signature` header does not contain a valid `t=` timestamp parameter.
    #[error("Invalid timestamp in Stripe-Signature header")]
    InvalidTimestamp,

    /// No `v1=` signature schemes were found in the header.
    #[error("No v1 signature scheme found in Stripe-Signature header")]
    MissingSignature,

    /// Cryptographic HMAC-SHA256 signature mismatch (forged or tampered).
    #[error("Cryptographic signature mismatch")]
    BadSignature,

    /// The event timestamp is older than the configured tolerance window or too far in future.
    #[error(
        "Timestamp outside the tolerance zone ({timestamp} vs current {now}, tolerance {tolerance}s)"
    )]
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

/// Errors occurring during redirect URL validation (open redirect protection).
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum RedirectError {
    /// URL fails syntax parsing.
    #[error("Invalid redirect URL '{url}': {reason}")]
    InvalidUrl {
        /// The invalid URL string.
        url: String,
        /// Description of why URL parsing failed.
        reason: String,
    },

    /// URL is missing an authority/host component.
    #[error("Invalid redirect URL '{url}': missing or empty host")]
    MissingHost {
        /// The URL missing host.
        url: String,
    },

    /// URL contains embedded userinfo (credentials like `user:pass@host`).
    #[error("Invalid redirect URL '{url}': embedded userinfo credentials are not allowed")]
    EmbeddedCredentials {
        /// The URL containing credentials.
        url: String,
    },

    /// Prohibited URL scheme (e.g., `javascript:`, `data:`, `file:`, `http:` for non-localhost).
    #[error(
        "Invalid redirect URL '{url}': prohibited scheme '{scheme}'. HTTPS is required (HTTP permitted only for localhost testing)"
    )]
    ProhibitedScheme {
        /// The URL containing prohibited scheme.
        url: String,
        /// The prohibited scheme.
        scheme: String,
    },

    /// Host is not in the whitelist of allowed domains.
    #[error(
        "Redirect URL host '{host}' is not in the list of allowed domains: {allowed_domains:?}"
    )]
    DisallowedDomain {
        /// The disallowed host.
        host: String,
        /// The configured list of allowed domains.
        allowed_domains: Vec<String>,
    },
}
