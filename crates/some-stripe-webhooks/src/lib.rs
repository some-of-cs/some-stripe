//! Cryptographic webhook signature verification and payload wrappers for Stripe.

use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sign::Signer;
use some_stripe_core::error::StripeWebhookError;
use some_stripe_types::Event;
use subtle::ConstantTimeEq;

/// Default timestamp tolerance window in seconds (5 minutes), per the Stripe specification.
pub const DEFAULT_TOLERANCE_SECONDS: u64 = 300;

/// Maximum future drift tolerance in seconds (Gate G2 / replay protection).
pub const MAX_FUTURE_DRIFT_SECONDS: i64 = 60;

/// Maximum incoming webhook payload size (1 MB / Gate G2 memory exhaustion guard).
pub const MAX_WEBHOOK_PAYLOAD_BYTES: usize = 1_048_576;

/// Constant-time slice comparison safe against length mismatches (Gate G2 Invariant 1).
pub fn safe_constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

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
    if payload.is_empty()
        || payload.len() > MAX_WEBHOOK_PAYLOAD_BYTES
        || signature_header.trim().is_empty()
        || webhook_secret.trim().is_empty()
    {
        return false;
    }

    let mut timestamp: Option<&str> = None;
    let mut signatures: Vec<&str> = Vec::new();

    for item in signature_header.split(',') {
        let item = item.trim();
        if let Some(t_val) = item.strip_prefix("t=") {
            timestamp = Some(t_val.trim());
        } else if let Some(sig_val) = item.strip_prefix("v1=") {
            signatures.push(sig_val.trim());
        }
    }

    let timestamp = match timestamp {
        Some(t) if !t.is_empty() => t,
        _ => return false,
    };

    if signatures.is_empty() {
        return false;
    }

    let mut signed_payload = Vec::with_capacity(timestamp.len() + 1 + payload.len());
    signed_payload.extend_from_slice(timestamp.as_bytes());
    signed_payload.push(b'.');
    signed_payload.extend_from_slice(payload);

    let expected_hex = match hmac_sha256_hex(webhook_secret.as_bytes(), &signed_payload) {
        Some(hex) => hex,
        None => return false,
    };

    signatures.iter().any(|sig| {
        safe_constant_time_eq(expected_hex.as_bytes(), sig.to_ascii_lowercase().as_bytes())
    })
}

/// Namespace providing static webhook helper functions modeled after `stripe.webhooks`.
pub struct Webhook;

impl Webhook {
    /// Default tolerance in seconds for webhook age verification.
    pub const DEFAULT_TOLERANCE: u64 = DEFAULT_TOLERANCE_SECONDS;

    /// Verifies the authenticity and recency of an incoming webhook header.
    pub fn verify_header(
        payload: &[u8],
        header: &str,
        secret: &str,
        tolerance: Option<u64>,
    ) -> Result<bool, StripeWebhookError> {
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
        if secret.trim().is_empty() {
            return Err(StripeWebhookError::MissingSecret);
        }

        let mut timestamp: Option<i64> = None;
        let mut signatures: Vec<&str> = Vec::new();

        for item in header.split(',') {
            let item = item.trim();
            if let Some(t_val) = item.strip_prefix("t=") {
                timestamp = t_val.parse::<i64>().ok();
            } else if let Some(sig_val) = item.strip_prefix("v1=") {
                signatures.push(sig_val.trim());
            }
        }

        let timestamp = timestamp.ok_or(StripeWebhookError::InvalidTimestamp)?;
        if signatures.is_empty() {
            return Err(StripeWebhookError::MissingSignature);
        }

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

        let ts_str = timestamp.to_string();
        let mut signed_payload = Vec::with_capacity(ts_str.len() + 1 + payload.len());
        signed_payload.extend_from_slice(ts_str.as_bytes());
        signed_payload.push(b'.');
        signed_payload.extend_from_slice(payload);

        let expected_signature = hmac_sha256_hex(secret.as_bytes(), &signed_payload)
            .ok_or(StripeWebhookError::BadSignature)?;

        let matched = signatures.iter().any(|sig| {
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
        let ts_str = ts.to_string();

        let mut signed_payload = Vec::with_capacity(ts_str.len() + 1 + payload.len());
        signed_payload.extend_from_slice(ts_str.as_bytes());
        signed_payload.push(b'.');
        signed_payload.extend_from_slice(payload);

        let sig = hmac_sha256_hex(secret.as_bytes(), &signed_payload).unwrap_or_default();
        format!("t={ts},v1={sig}")
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
}
