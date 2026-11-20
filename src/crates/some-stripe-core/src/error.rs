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

impl StripeError {
    /// Returns `true` if this error is transient and safe to retry (HTTP 429,
    /// HTTP 5xx, connection drops, or retryable transport errors).
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Api { status, .. } => *status == 429 || (500..600).contains(status),
            Self::ConnectionError(_) => true,
            Self::Http(err) => is_reqwest_error_retryable(err),
            _ => false,
        }
    }

    /// Returns the HTTP status code associated with this error, if any.
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::Api { status, .. } => Some(*status),
            Self::Http(err) => err.status().map(|s| s.as_u16()),
            _ => None,
        }
    }

    /// Returns the Stripe-specific error code (e.g., `rate_limit`, `card_declined`), if any.
    pub fn stripe_code(&self) -> Option<&str> {
        match self {
            Self::Api { code, .. } => Some(code.as_str()),
            _ => None,
        }
    }
}

fn is_reqwest_error_retryable(err: &reqwest::Error) -> bool {
    let status_retryable = err
        .status()
        .is_some_and(|s| s.as_u16() == 429 || s.is_server_error());
    err.is_timeout() || err.is_connect() || err.is_body() || status_retryable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stripe_error_telemetry_helpers() {
        let rate_limited = StripeError::Api {
            status: 429,
            code: "rate_limit".into(),
            message: "Too many requests".into(),
            error_type: "api_error".into(),
            doc_url: None,
        };
        assert!(rate_limited.is_retryable());
        assert_eq!(rate_limited.status_code(), Some(429));
        assert_eq!(rate_limited.stripe_code(), Some("rate_limit"));

        let server_err = StripeError::Api {
            status: 503,
            code: "api_error".into(),
            message: "Service unavailable".into(),
            error_type: "api_error".into(),
            doc_url: None,
        };
        assert!(server_err.is_retryable());
        assert_eq!(server_err.status_code(), Some(503));
        assert_eq!(server_err.stripe_code(), Some("api_error"));

        let client_err = StripeError::Api {
            status: 400,
            code: "card_declined".into(),
            message: "Your card was declined".into(),
            error_type: "card_error".into(),
            doc_url: None,
        };
        assert!(!client_err.is_retryable());
        assert_eq!(client_err.status_code(), Some(400));
        assert_eq!(client_err.stripe_code(), Some("card_declined"));

        let conn_err = StripeError::ConnectionError("socket closed".into());
        assert!(conn_err.is_retryable());
        assert_eq!(conn_err.status_code(), None);
        assert_eq!(conn_err.stripe_code(), None);

        let cfg_err = StripeError::Config("invalid tls".into());
        assert!(!cfg_err.is_retryable());
        assert_eq!(cfg_err.status_code(), None);
        assert_eq!(cfg_err.stripe_code(), None);
    }
}
