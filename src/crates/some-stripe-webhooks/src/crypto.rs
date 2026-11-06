//! Streaming HMAC-SHA256 signature computation engine.
//!
//! Enforces Gate G-Borrow Invariant: eliminates intermediate heap allocations
//! for payload concatenation by streaming timestamp bytes, separator, and
//! webhook payload directly into the pure-Rust HMAC-SHA256 digest engine.

use hmac::digest::KeyInit;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use some_stripe_core::error::StripeWebhookError;

type HmacSha256 = Hmac<Sha256>;

/// Computes an HMAC-SHA256 signature in a zero-allocation streaming manner over verbatim timestamp bytes (Gate G-Borrow & G2).
///
/// Feeds `timestamp` bytes verbatim, `b"."`, and raw `payload` bytes sequentially into the
/// pure-Rust HMAC-SHA256 [`Mac`].
///
/// Returns the lowercase 64-character hexadecimal signature string.
///
/// # Errors
///
/// Returns [`StripeWebhookError::BadKey`] if key initialization or HMAC creation fails.
///
/// # Examples
///
/// ```
/// use some_stripe_webhooks::crypto::compute_streaming_hmac_bytes;
///
/// let secret = b"whsec_test_secret";
/// let timestamp = b"1728345600";
/// let payload = b"{\"id\":\"evt_123\"}";
///
/// let sig = compute_streaming_hmac_bytes(secret, timestamp, payload).unwrap();
/// assert_eq!(sig.len(), 64);
/// ```
pub fn compute_streaming_hmac_bytes(
    key: &[u8],
    timestamp: &[u8],
    payload: &[u8],
) -> Result<String, StripeWebhookError> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| StripeWebhookError::BadKey)?;

    mac.update(timestamp);
    mac.update(b".");
    mac.update(payload);

    let signature = mac.finalize().into_bytes();

    let mut hex = String::with_capacity(signature.len() * 2);
    for b in signature {
        use std::fmt::Write;
        let _ = write!(hex, "{:02x}", b);
    }

    Ok(hex)
}

/// Computes an HMAC-SHA256 signature in a zero-allocation streaming manner (Gate G-Borrow & G2).
///
/// Rather than allocating an intermediate heap buffer for `${timestamp}.${payload}`
/// (which for large webhooks would clone up to 1 MB of memory), this function converts
/// `timestamp` to its decimal ASCII representation and delegates to [`compute_streaming_hmac_bytes`].
///
/// Returns the lowercase 64-character hexadecimal signature string.
///
/// # Errors
///
/// Returns [`StripeWebhookError::BadKey`] if key initialization or HMAC creation fails.
///
/// # Examples
///
/// ```
/// use some_stripe_webhooks::crypto::compute_streaming_hmac_hex;
///
/// let secret = b"whsec_test_secret";
/// let timestamp = 1728345600;
/// let payload = b"{\"id\":\"evt_123\"}";
///
/// let sig = compute_streaming_hmac_hex(secret, timestamp, payload).unwrap();
/// assert_eq!(sig.len(), 64);
/// ```
pub fn compute_streaming_hmac_hex(
    key: &[u8],
    timestamp: i64,
    payload: &[u8],
) -> Result<String, StripeWebhookError> {
    compute_streaming_hmac_bytes(key, timestamp.to_string().as_bytes(), payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hmac_sha256_hex;

    #[test]
    fn test_streaming_matches_monolithic_hmac() {
        let key = b"whsec_test_secret_key_12345";
        let timestamp = 1728345600;
        let payload = br#"{"id":"evt_123","type":"payment_intent.succeeded"}"#;

        // Monolithic: allocate timestamp + . + payload
        let mut monolithic_input = Vec::new();
        monolithic_input.extend_from_slice(timestamp.to_string().as_bytes());
        monolithic_input.push(b'.');
        monolithic_input.extend_from_slice(payload);
        let expected = hmac_sha256_hex(key, &monolithic_input).unwrap();

        // Streaming: zero intermediate allocation
        let streamed = compute_streaming_hmac_hex(key, timestamp, payload).unwrap();

        assert_eq!(streamed, expected);
    }

    #[test]
    fn test_streaming_hmac_stripe_official_vector() {
        let secret = b"whsec_test_secret";
        let timestamp = 1492774577;
        let payload = br#"{"id": "evt_test_webhook"}"#;

        let sig = compute_streaming_hmac_hex(secret, timestamp, payload).unwrap();

        assert_eq!(sig.len(), 64);
        assert!(
            sig.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );

        let mut expected_payload = Vec::new();
        expected_payload.extend_from_slice(timestamp.to_string().as_bytes());
        expected_payload.push(b'.');
        expected_payload.extend_from_slice(payload);
        let reference = hmac_sha256_hex(secret, &expected_payload).unwrap();
        assert_eq!(sig, reference);
    }

    #[test]
    fn test_streaming_hmac_large_payload() {
        let key = b"whsec_large_payload_secret";
        let timestamp = 1728345600;
        let large_payload = vec![b'x'; 524_288];

        let sig = compute_streaming_hmac_hex(key, timestamp, &large_payload).unwrap();
        assert_eq!(sig.len(), 64);
        assert!(
            sig.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn test_streaming_hmac_bytes_verbatim() {
        let key = b"whsec_test_secret";
        let timestamp_verbatim = b"01728345600";
        let payload = b"{\"id\":\"evt_test\"}";

        let sig = compute_streaming_hmac_bytes(key, timestamp_verbatim, payload).unwrap();
        assert_eq!(sig.len(), 64);

        let mut expected = Vec::new();
        expected.extend_from_slice(timestamp_verbatim);
        expected.push(b'.');
        expected.extend_from_slice(payload);
        let ref_sig = hmac_sha256_hex(key, &expected).unwrap();
        assert_eq!(sig, ref_sig);

        // Ensure non-canonical timestamp produces a different signature than canonical decimal
        let canonical_sig = compute_streaming_hmac_hex(key, 1728345600, payload).unwrap();
        assert_ne!(sig, canonical_sig);
    }
}
