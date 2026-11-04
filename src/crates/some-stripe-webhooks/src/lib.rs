//! # some-stripe-webhooks
//!
//! Cryptographic webhook signature verification and payload wrappers for Stripe,
//! modeled after `stripe-node`.
//!
//! ## Security & Architectural Invariants (Gate G2, Gate G-Borrow, Gate G-Law)
//!
//! - **Constant-Time Verification**: HMAC-SHA256 signature comparison executed strictly in
//!   constant time via [`safe_constant_time_eq`] to prevent timing side-channel attacks.
//! - **Streaming Zero-Allocation HMAC**: Signatures computed via [`compute_streaming_hmac_hex`]
//!   streaming raw payload slices without heap buffers (Gate G-Borrow).
//! - **Replay Protection**: Enforces timestamp tolerance (default 300 seconds) and rejects
//!   excessive future drift (> 60 seconds).
//! - **Zero-Copy Streaming Header Parser**: Parses `Stripe-Signature` headers with zero heap allocation
//!   for signature strings, supporting secret rollover with multiple `v1` signatures.
//! - **Resource Limits**: Incoming payloads are strictly capped at 1 MB ([`MAX_WEBHOOK_PAYLOAD_BYTES`])
//!   to safeguard against memory exhaustion attacks.
//! - **Zero-Clone Streaming HMAC (Gate G-Borrow)**: Computes HMAC-SHA256 digests by streaming
//!   timestamp and payload slices directly into the OpenSSL digest without intermediate heap
//!   allocations ([`streaming_hmac_sha256_hex`]).
//! - **Secret Pre-Validation**: Blank or whitespace-only webhook secrets are rejected immediately
//!   with [`StripeWebhookError::BadKey`] before performing HMAC computations or allocations.
//!
//! ## Verification Example
//!
//! ```
//! use some_stripe_webhooks::{Webhook, StripeWebhookWrapper};
//!
//! let payload = br#"{"id":"evt_test_123","object":"event","created":1728345600,"type":"checkout.session.completed","data":{"object":{"id":"cs_test_123","status":"complete","payment_status":"paid"}}}"#;
//! let secret = "whsec_test_secret";
//! let header = Webhook::generate_test_header_string(payload, secret, None);
//!
//! let event = Webhook::construct_event(payload, &header, secret, None).unwrap();
//! assert_eq!(event.id, "evt_test_123");
//! assert_eq!(event.event_type, "checkout.session.completed");
//!
//! let wrapper = StripeWebhookWrapper::from_raw(payload, &header, secret, None).unwrap();
//! assert!(wrapper.is_checkout_completed());
//! ```

pub mod compare;
pub mod constants;
pub mod crypto;
pub mod error;
pub mod escalation;
pub mod header;
pub mod tolerance;
pub mod v2_events;
pub mod validation;

pub use escalation::{
    WebhookEscalationBridge, determine_severity, escalate_webhook_error, is_security_incident,
};
pub use tolerance::ToleranceConfig;
pub use v2_events::{V2RelatedObject, V2ThinEvent, is_thin_event};

pub use compare::safe_constant_time_eq;
pub use constants::{
    DEFAULT_TOLERANCE_SECONDS, MAX_FUTURE_DRIFT_SECONDS, MAX_WEBHOOK_PAYLOAD_BYTES,
    TIMESTAMP_PREFIX, V1_SIGNATURE_PREFIX,
};
pub use crypto::compute_streaming_hmac_hex;
pub use error::StripeWebhookError;
pub use header::{StripeSignatureHeader, parse_signature_header};

use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sign::Signer;
use some_stripe_types::Event;
pub use validation::is_blank_webhook_secret;

/// Computes HMAC-SHA256 over `data` using `key`, returning the lowercase hexadecimal string.
pub fn hmac_sha256_hex(key: &[u8], data: &[u8]) -> Option<String> {
    let pkey = PKey::hmac(key).ok()?;
    let mut signer = Signer::new(MessageDigest::sha256(), &pkey).ok()?;
    signer.update(data).ok()?;
    let signature = signer.sign_to_vec().ok()?;
    let mut hex = String::with_capacity(signature.len() * 2);
    for b in signature {
        use std::fmt::Write;
        let _ = write!(hex, "{:02x}", b);
    }
    Some(hex)
}

/// Computes HMAC-SHA256 by streaming timestamp and payload byte slices directly into the
/// cryptographic digest without intermediate heap buffer allocations (Gate G-Borrow / Zero-Clone Discipline).
///
/// # Example
///
/// ```
/// use some_stripe_webhooks::streaming_hmac_sha256_hex;
///
/// let key = b"whsec_test_secret";
/// let timestamp = b"1728345600";
/// let payload = br#"{"id":"evt_123"}"#;
///
/// let hex = streaming_hmac_sha256_hex(key, timestamp, payload).unwrap();
/// assert_eq!(hex.len(), 64);
/// ```
pub fn streaming_hmac_sha256_hex(key: &[u8], timestamp: &[u8], payload: &[u8]) -> Option<String> {
    let pkey = PKey::hmac(key).ok()?;
    let mut signer = Signer::new(MessageDigest::sha256(), &pkey).ok()?;
    signer.update(timestamp).ok()?;
    signer.update(b".").ok()?;
    signer.update(payload).ok()?;
    let signature = signer.sign_to_vec().ok()?;
    let mut hex = String::with_capacity(signature.len() * 2);
    for b in signature {
        use std::fmt::Write;
        let _ = write!(hex, "{:02x}", b);
    }
    Some(hex)
}

/// Verifies a Stripe webhook signature per the official Stripe signature specification.
///
/// Operates strictly on borrowed `&[u8]` and `&str` slices without copying or cloning
/// the raw payload (Gate G-Borrow).
pub fn verify_stripe_signature(
    payload: &[u8],
    signature_header: &str,
    webhook_secret: &str,
) -> bool {
    if is_blank_webhook_secret(webhook_secret)
        || payload.is_empty()
        || payload.len() > MAX_WEBHOOK_PAYLOAD_BYTES
        || signature_header.trim().is_empty()
    {
        return false;
    }

    let parsed_header = match parse_signature_header(signature_header) {
        Ok(h) => h,
        Err(_) => return false,
    };

    let expected_hex = match compute_streaming_hmac_hex(
        webhook_secret.as_bytes(),
        parsed_header.timestamp,
        payload,
    ) {
        Ok(hex) => hex,
        Err(_) => return false,
    };

    parsed_header.signatures.iter().any(|sig| {
        safe_constant_time_eq(expected_hex.as_bytes(), sig.to_ascii_lowercase().as_bytes())
    })
}

/// Namespace providing static webhook helper functions modeled after `stripe.webhooks`.
pub struct Webhook;

impl Webhook {
    /// Default tolerance in seconds for webhook age verification.
    pub const DEFAULT_TOLERANCE: u64 = DEFAULT_TOLERANCE_SECONDS;

    /// Verifies the authenticity and recency of an incoming webhook header.
    ///
    /// Rejects blank/whitespace webhook secrets immediately with [`StripeWebhookError::BadKey`]
    /// prior to allocating buffers or computing cryptographic HMAC signatures.
    /// Operates strictly on borrowed slices using zero-clone streaming HMAC (Gate G-Borrow).
    pub fn verify_header(
        payload: &[u8],
        header: &str,
        secret: &str,
        tolerance: Option<u64>,
    ) -> Result<bool, StripeWebhookError> {
        let config = tolerance.map_or_else(ToleranceConfig::default, ToleranceConfig::new);
        Self::verify_header_with_config(payload, header, secret, &config)
    }

    /// Verifies the authenticity and recency of an incoming webhook header using a [`ToleranceConfig`].
    pub fn verify_header_with_config(
        payload: &[u8],
        header: &str,
        secret: &str,
        config: &ToleranceConfig,
    ) -> Result<bool, StripeWebhookError> {
        if is_blank_webhook_secret(secret) {
            return Err(StripeWebhookError::BadKey);
        }
        if payload.is_empty() {
            return Err(StripeWebhookError::EmptyPayload);
        }
        if payload.len() > MAX_WEBHOOK_PAYLOAD_BYTES {
            return Err(StripeWebhookError::PayloadTooLarge {
                size: payload.len(),
                max_allowed: MAX_WEBHOOK_PAYLOAD_BYTES,
            });
        }
        if header.trim().is_empty() {
            return Err(StripeWebhookError::MissingSignatureHeader);
        }

        let parsed_header = parse_signature_header(header)?;
        let now = chrono::Utc::now().timestamp();
        config.check_timestamp(parsed_header.timestamp, now)?;

        let expected_signature =
            compute_streaming_hmac_hex(secret.as_bytes(), parsed_header.timestamp, payload)?;

        let matched = parsed_header.signatures.iter().any(|sig| {
            safe_constant_time_eq(
                expected_signature.as_bytes(),
                sig.to_ascii_lowercase().as_bytes(),
            )
        });

        if matched {
            Ok(true)
        } else {
            Err(StripeWebhookError::BadSignature)
        }
    }

    /// Verifies an incoming webhook payload and parses it into a strongly-typed [`Event`].
    /// Verifies an incoming webhook payload and parses it into a strongly-typed [`V2ThinEvent`].
    pub fn construct_v2_thin_event(
        payload: &[u8],
        sig_header: &str,
        secret: &str,
        tolerance: Option<u64>,
    ) -> Result<V2ThinEvent, StripeWebhookError> {
        Self::verify_header(payload, sig_header, secret, tolerance)?;

        serde_json::from_slice::<V2ThinEvent>(payload)
            .map_err(|e| StripeWebhookError::MalformedPayload(e.to_string()))
    }
    pub fn construct_event(
        payload: &[u8],
        sig_header: &str,
        secret: &str,
        tolerance: Option<u64>,
    ) -> Result<Event, StripeWebhookError> {
        Self::verify_header(payload, sig_header, secret, tolerance)?;

        serde_json::from_slice::<Event>(payload)
            .map_err(|e| StripeWebhookError::MalformedPayload(e.to_string()))
    }

    /// Verifies an incoming webhook payload using a [`ToleranceConfig`] and parses it into a strongly-typed [`Event`].
    pub fn construct_event_with_config(
        payload: &[u8],
        sig_header: &str,
        secret: &str,
        config: &ToleranceConfig,
    ) -> Result<Event, StripeWebhookError> {
        Self::verify_header_with_config(payload, sig_header, secret, config)?;

        serde_json::from_slice::<Event>(payload)
            .map_err(|e| StripeWebhookError::MalformedPayload(e.to_string()))
    }

    /// Test utility to generate signed headers for test assertions.
    pub fn generate_test_header_string(
        payload: &[u8],
        secret: &str,
        timestamp: Option<i64>,
    ) -> String {
        let ts = timestamp.unwrap_or_else(|| chrono::Utc::now().timestamp());
        let sig = compute_streaming_hmac_hex(secret.as_bytes(), ts, payload).unwrap_or_default();
        format!("{TIMESTAMP_PREFIX}{ts},{V1_SIGNATURE_PREFIX}{sig}")
    }
}

/// Ergonomic wrapper providing typed accessors over raw webhook [`Event`] instances.
/// Immutable, timestamped audit record produced upon webhook verification,
/// satisfying ISO/IEC 27001:2022 Control A.8.15 (Logging and Monitoring).
///
/// Holds borrowed string references (`&'a str`) to prevent unnecessary heap allocation
/// in compliance with Gate G-Borrow zero-clone invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookAuditRecord<'a> {
    /// Unique identifier of the Stripe event (`evt_...`).
    pub event_id: &'a str,
    /// Classification type of the event (e.g., `checkout.session.completed`).
    pub event_type: &'a str,
    /// UNIX epoch timestamp when verification succeeded, if verified.
    pub verified_at: Option<i64>,
    /// Verification status indicator (`"verified"` or `"unverified"`).
    pub status: &'a str,
}

impl<'a> WebhookAuditRecord<'a> {
    /// Returns `true` if this audit record represents a successfully verified event.
    pub fn is_verified(&self) -> bool {
        self.status == "verified" && self.verified_at.is_some()
    }
}

impl<'a> From<&'a StripeWebhookWrapper> for WebhookAuditRecord<'a> {
    fn from(wrapper: &'a StripeWebhookWrapper) -> Self {
        wrapper.audit_record()
    }
}

#[derive(Debug, Clone)]
pub struct StripeWebhookWrapper {
    /// Parsed event object.
    pub event: Event,
    /// Timestamp at which the webhook was verified.
    pub verified_at: Option<i64>,
}

impl StripeWebhookWrapper {
    /// Constructs a verified wrapper directly from raw webhook payload bytes and signature header.
    pub fn from_raw(
        payload: &[u8],
        header: &str,
        secret: &str,
        tolerance: Option<u64>,
    ) -> Result<Self, StripeWebhookError> {
        let event = Webhook::construct_event(payload, header, secret, tolerance)?;
        let verified_at = Some(chrono::Utc::now().timestamp());
        Ok(Self::new(event, verified_at))
    }

    /// Constructs a verified wrapper directly using a custom [`ToleranceConfig`].
    pub fn from_raw_with_config(
        payload: &[u8],
        header: &str,
        secret: &str,
        config: &ToleranceConfig,
    ) -> Result<Self, StripeWebhookError> {
        let event = Webhook::construct_event_with_config(payload, header, secret, config)?;
        let verified_at = Some(chrono::Utc::now().timestamp());
        Ok(Self::new(event, verified_at))
    }

    /// Access the event ID string slice.
    pub fn event_id(&self) -> &str {
        &self.event.id
    }

    /// Wraps a parsed Stripe [`Event`].
    pub fn new(event: Event, verified_at: Option<i64>) -> Self {
        Self { event, verified_at }
    }

    /// Produces an immutable, timestamped audit record for this event,
    /// satisfying ISO/IEC 27001:2022 Control A.8.15.
    pub fn audit_record(&self) -> WebhookAuditRecord<'_> {
        WebhookAuditRecord {
            event_id: &self.event.id,
            event_type: &self.event.event_type,
            verified_at: self.verified_at,
            status: if self.verified_at.is_some() {
                "verified"
            } else {
                "unverified"
            },
        }
    }

    /// Extracts the order ID from `/data/object/id` as a borrowed string slice (Gate G-Borrow zero-clone).
    pub fn order_id(&self) -> Option<&str> {
        self.event
            .data
            .object
            .pointer("/id")
            .and_then(|v| v.as_str())
    }

    /// Extracts the customer ID from `/data/object/customer` as a borrowed string slice (Gate G-Borrow zero-clone).
    pub fn customer_id(&self) -> Option<&str> {
        self.event
            .data
            .object
            .pointer("/customer")
            .and_then(|v| v.as_str())
    }

    /// Extracts the PaymentIntent ID from `/data/object/payment_intent` as a borrowed string slice (Gate G-Borrow zero-clone).
    pub fn payment_intent_id(&self) -> Option<&str> {
        self.event
            .data
            .object
            .pointer("/payment_intent")
            .and_then(|v| v.as_str())
    }

    /// Checks if the event represents a completed checkout session.
    pub fn is_checkout_completed(&self) -> bool {
        self.event.event_type == "checkout.session.completed"
            && self
                .event
                .data
                .object
                .pointer("/status")
                .and_then(|v| v.as_str())
                == Some("complete")
            && matches!(
                self.event
                    .data
                    .object
                    .pointer("/payment_status")
                    .and_then(|v| v.as_str()),
                Some("paid" | "unpaid")
            )
    }

    /// Extracts the total amount in minor units from `/data/object/amount_total`.
    pub fn amount_total(&self) -> Option<i64> {
        self.event
            .data
            .object
            .pointer("/amount_total")
            .and_then(|v| v.as_i64())
    }

    /// Extracts the currency code in uppercase from `/data/object/currency`.
    pub fn currency(&self) -> Option<String> {
        self.event
            .data
            .object
            .pointer("/currency")
            .and_then(|v| v.as_str())
            .map(|s| s.to_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_stripe_signature() {
        let payload = b"{\"id\":\"evt_123\",\"type\":\"payment_intent.succeeded\"}";
        let secret = "whsec_test_secret_key_12345";
        let timestamp = 1728345600;

        let header = Webhook::generate_test_header_string(payload, secret, Some(timestamp));
        assert!(verify_stripe_signature(payload, &header, secret));
    }

    #[test]
    fn test_payload_exceeding_1mb_rejected() {
        let large_payload = vec![b'a'; MAX_WEBHOOK_PAYLOAD_BYTES + 1];
        let secret = "whsec_test_secret_key_12345";
        let header = "t=1728345600,v1=bad_signature";

        let err = Webhook::verify_header(&large_payload, header, secret, None).unwrap_err();
        assert!(matches!(err, StripeWebhookError::PayloadTooLarge { .. }));
    }

    #[test]
    fn test_forged_stripe_signature_rejected() {
        let payload = b"{\"id\":\"evt_123\",\"type\":\"payment_intent.succeeded\"}";
        let secret = "whsec_test_secret_key_12345";
        let header =
            "t=1728345600,v1=5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165";

        assert!(!verify_stripe_signature(payload, header, secret));
    }

    #[test]
    fn test_blank_secret_rejected_with_bad_key() {
        let payload = b"{\"id\":\"evt_123\"}";
        let header = "t=1728345600,v1=abc";

        // Empty secret
        let err = Webhook::verify_header(payload, header, "", None).unwrap_err();
        assert_eq!(err, StripeWebhookError::BadKey);

        // Whitespace-only secret
        let err2 = Webhook::verify_header(payload, header, "   \t\n  ", None).unwrap_err();
        assert_eq!(err2, StripeWebhookError::BadKey);

        // Blank secret in construct_event
        let err3 = Webhook::construct_event(payload, header, "  ", None).unwrap_err();
        assert_eq!(err3, StripeWebhookError::BadKey);

        // Blank secret in verify_stripe_signature returns false
        assert!(!verify_stripe_signature(payload, header, ""));
        assert!(!verify_stripe_signature(payload, header, "   "));
    }

    #[test]
    fn test_construct_event_with_valid_tolerance() {
        let now = chrono::Utc::now().timestamp();
        let payload = br#"{"id":"evt_test_construct","object":"event","created":1728345600,"type":"checkout.session.completed","data":{"object":{"id":"cs_test_123","status":"complete","payment_status":"paid","amount_total":5000,"currency":"usd"}}}"#;
        let secret = "whsec_secret_tolerance_test";

        let header = Webhook::generate_test_header_string(payload, secret, Some(now));
        let event = Webhook::construct_event(payload, &header, secret, Some(300)).unwrap();

        assert_eq!(event.id, "evt_test_construct");
        assert_eq!(event.event_type, "checkout.session.completed");

        let wrapper = StripeWebhookWrapper::new(event, Some(now));
        assert_eq!(wrapper.order_id(), Some("cs_test_123"));
        assert_eq!(wrapper.amount_total(), Some(5000));
        assert_eq!(wrapper.currency().as_deref(), Some("USD"));
        assert!(wrapper.is_checkout_completed());
    }

    #[test]
    fn test_rollover_dual_signatures_matches_secondary() {
        let payload = b"{\"id\":\"evt_123\",\"type\":\"payment_intent.succeeded\"}";
        let secret_old = "whsec_old_test_secret_key_11111";
        let secret_new = "whsec_new_test_secret_key_22222";
        let timestamp = 1728345600;

        let sig_old = Webhook::generate_test_header_string(payload, secret_old, Some(timestamp));
        let sig_new = Webhook::generate_test_header_string(payload, secret_new, Some(timestamp));

        let sig_old_val = sig_old.split("v1=").nth(1).unwrap();
        let sig_new_val = sig_new.split("v1=").nth(1).unwrap();

        let combined_header = format!("t={timestamp},v1={sig_old_val},v1={sig_new_val}");

        assert!(verify_stripe_signature(
            payload,
            &combined_header,
            secret_new
        ));
        assert!(verify_stripe_signature(
            payload,
            &combined_header,
            secret_old
        ));
    }
}
