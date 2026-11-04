//! # some-stripe-webhooks
//!
//! Cryptographic webhook signature verification and payload wrappers for Stripe,
//! modeled after `stripe-node`.
//!
//! ## Security & Architectural Invariants (Gate G2)
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
pub mod header;
pub mod validation;

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

/// Verifies a Stripe webhook signature per the official Stripe signature specification.
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
    pub fn verify_header(
        payload: &[u8],
        header: &str,
        secret: &str,
        tolerance: Option<u64>,
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
        let timestamp = parsed_header.timestamp;

        let tolerance_secs = tolerance.unwrap_or(Self::DEFAULT_TOLERANCE);
        if tolerance_secs > 0 {
            let now = chrono::Utc::now().timestamp();
            // Check past tolerance AND future drift bound (Gate G2)
            if timestamp < now - (tolerance_secs as i64)
                || timestamp > now + MAX_FUTURE_DRIFT_SECONDS
            {
                return Err(StripeWebhookError::TimestampOutsideTolerance {
                    timestamp,
                    now,
                    tolerance: tolerance_secs,
                });
            }
        }

        let expected_signature = compute_streaming_hmac_hex(secret.as_bytes(), timestamp, payload)?;

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

    /// Access the event ID string slice.
    pub fn event_id(&self) -> &str {
        &self.event.id
    }

    /// Wraps a parsed Stripe [`Event`].
    pub fn new(event: Event, verified_at: Option<i64>) -> Self {
        Self { event, verified_at }
    }

    /// Extracts the order ID from `/data/object/id`.
    pub fn order_id(&self) -> Option<String> {
        self.event
            .data
            .object
            .pointer("/id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
    }

    /// Extracts the customer ID from `/data/object/customer`.
    pub fn customer_id(&self) -> Option<String> {
        self.event
            .data
            .object
            .pointer("/customer")
            .and_then(|v| v.as_str())
            .map(str::to_string)
    }

    /// Extracts the PaymentIntent ID from `/data/object/payment_intent`.
    pub fn payment_intent_id(&self) -> Option<String> {
        self.event
            .data
            .object
            .pointer("/payment_intent")
            .and_then(|v| v.as_str())
            .map(str::to_string)
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
        assert_eq!(wrapper.order_id().as_deref(), Some("cs_test_123"));
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
