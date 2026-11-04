//! Zero-copy streaming parser for Stripe webhook signature headers (`Stripe-Signature`).
//!
//! # Specification & Security Invariants
//!
//! Stripe delivers webhook signatures via the `Stripe-Signature` HTTP header in a comma-delimited
//! key-value format:
//!
//! ```text
//! Stripe-Signature: t=1492774577,v1=5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165,v1=...
//! ```
//!
//! - **Zero Allocation & Zero-Copy Borrowing (Gate G-Borrow)**: The parser yields [`StripeSignatureHeader`]
//!   with borrowed slices (`&'a str`) pointing directly into the original header slice without heap-allocating
//!   individual signature strings.
//! - **Secret Rollover Support**: Collects all `v1=` signature instances, enabling seamless key rotation
//!   where webhooks are signed simultaneously with old and newly rotated secrets.
//! - **Strict Timestamp Validation (Gate G2)**: Extracts the Unix timestamp integer from `t=`. Rejects
//!   missing, non-numeric, or malformed timestamps with [`StripeWebhookError::InvalidTimestamp`].
//! - **Header Integrity (Gate G2)**: Rejects empty or whitespace-only headers with
//!   [`StripeWebhookError::MissingSignatureHeader`] and headers lacking any `v1` signature with
//!   [`StripeWebhookError::MissingSignature`].

use some_stripe_core::error::StripeWebhookError;

/// Parsed representation of a `Stripe-Signature` HTTP header.
///
/// Holds the Unix timestamp (in seconds) and zero-copy references to all `v1` signature hex strings
/// extracted from the header.
///
/// # Examples
///
/// ```
/// use some_stripe_webhooks::StripeSignatureHeader;
///
/// let header = StripeSignatureHeader {
///     timestamp: 1728345600,
///     signatures: vec!["sig_v1_primary", "sig_v1_rolling"],
/// };
///
/// assert_eq!(header.timestamp, 1728345600);
/// assert_eq!(header.signatures.len(), 2);
/// assert!(header.has_signatures());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StripeSignatureHeader<'a> {
    /// Unix timestamp in seconds extracted from `t=<timestamp>`.
    pub timestamp: i64,
    /// Zero-copy borrowed slices of all `v1` signatures present in the header.
    pub signatures: Vec<&'a str>,
}

impl<'a> StripeSignatureHeader<'a> {
    /// Constructs a new [`StripeSignatureHeader`] with the given timestamp and signatures.
    #[inline]
    pub const fn new(timestamp: i64, signatures: Vec<&'a str>) -> Self {
        Self {
            timestamp,
            signatures,
        }
    }

    /// Returns `true` if one or more `v1` signatures are present.
    #[inline]
    pub fn has_signatures(&self) -> bool {
        !self.signatures.is_empty()
    }
}

/// Parses a raw `Stripe-Signature` header value into a structured [`StripeSignatureHeader`].
///
/// This parser scans comma-separated key-value tokens zero-copy without allocating
/// new strings. It collects all `v1` signatures to support key rollover and secret rotation.
///
/// # Errors
///
/// - [`StripeWebhookError::MissingSignatureHeader`]: If `header` is empty or consists solely of whitespace.
/// - [`StripeWebhookError::InvalidTimestamp`]: If `t=` is absent or cannot be parsed as a 64-bit signed integer.
/// - [`StripeWebhookError::MissingSignature`]: If no valid, non-empty `v1` signature tokens exist in the header.
///
/// # Examples
///
/// Single signature:
/// ```
/// use some_stripe_webhooks::parse_signature_header;
///
/// let raw = "t=1728345600,v1=5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165";
/// let parsed = parse_signature_header(raw).unwrap();
///
/// assert_eq!(parsed.timestamp, 1728345600);
/// assert_eq!(parsed.signatures, vec!["5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165"]);
/// ```
///
/// Dual signatures during secret rollover:
/// ```
/// use some_stripe_webhooks::parse_signature_header;
///
/// let raw = "t=1728345600,v1=sig_current,v1=sig_rolling";
/// let parsed = parse_signature_header(raw).unwrap();
///
/// assert_eq!(parsed.timestamp, 1728345600);
/// assert_eq!(parsed.signatures.len(), 2);
/// assert_eq!(parsed.signatures[0], "sig_current");
/// assert_eq!(parsed.signatures[1], "sig_rolling");
/// ```
pub fn parse_signature_header<'a>(
    header: &'a str,
) -> Result<StripeSignatureHeader<'a>, StripeWebhookError> {
    if header.trim().is_empty() {
        return Err(StripeWebhookError::MissingSignatureHeader);
    }

    let mut timestamp: Option<i64> = None;
    let mut signatures: Vec<&'a str> = Vec::with_capacity(2);

    for token in header.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }

        if let Some(t_val) = token.strip_prefix("t=") {
            let t_str = t_val.trim();
            match t_str.parse::<i64>() {
                Ok(ts) => timestamp = Some(ts),
                Err(_) => return Err(StripeWebhookError::InvalidTimestamp),
            }
        } else if let Some(sig_val) = token.strip_prefix("v1=") {
            let sig = sig_val.trim();
            if !sig.is_empty() {
                signatures.push(sig);
            }
        }
    }

    let timestamp = timestamp.ok_or(StripeWebhookError::InvalidTimestamp)?;
    if signatures.is_empty() {
        return Err(StripeWebhookError::MissingSignature);
    }

    Ok(StripeSignatureHeader {
        timestamp,
        signatures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_v1_signature() {
        let raw =
            "t=1728345600,v1=5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165";
        let parsed = parse_signature_header(raw).expect("should parse valid header");

        assert_eq!(parsed.timestamp, 1728345600);
        assert_eq!(
            parsed.signatures,
            vec!["5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165"]
        );
        assert!(parsed.has_signatures());
    }

    #[test]
    fn test_dual_v1_signatures_rollover() {
        let raw = "t=1728345600,v1=sig_alpha,v1=sig_beta";
        let parsed = parse_signature_header(raw).expect("should parse multi-sig header");

        assert_eq!(parsed.timestamp, 1728345600);
        assert_eq!(parsed.signatures.len(), 2);
        assert_eq!(parsed.signatures, vec!["sig_alpha", "sig_beta"]);
    }

    #[test]
    fn test_whitespace_and_mixed_delimiters() {
        let raw = "  t=1728345600 ,  v1=sig_alpha  , v1=sig_beta  ";
        let parsed = parse_signature_header(raw).expect("should tolerate whitespace around tokens");

        assert_eq!(parsed.timestamp, 1728345600);
        assert_eq!(parsed.signatures, vec!["sig_alpha", "sig_beta"]);
    }

    #[test]
    fn test_malformed_timestamp_returns_invalid_timestamp() {
        let cases = [
            "t=not_a_number,v1=sig",
            "t=123.456,v1=sig",
            "t=,v1=sig",
            "t=  ,v1=sig",
            "t=9999999999999999999999999999999999999999999999,v1=sig",
        ];

        for case in cases {
            let err =
                parse_signature_header(case).expect_err(&format!("expected error for {case}"));
            assert_eq!(err, StripeWebhookError::InvalidTimestamp);
        }
    }

    #[test]
    fn test_missing_timestamp_returns_invalid_timestamp() {
        let raw = "v1=5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165";
        let err = parse_signature_header(raw).expect_err("should fail without timestamp");
        assert_eq!(err, StripeWebhookError::InvalidTimestamp);
    }

    #[test]
    fn test_missing_v1_signatures_returns_missing_signature() {
        let cases = [
            "t=1728345600",
            "t=1728345600,v0=legacy_signature",
            "t=1728345600,v1=",
            "t=1728345600,v1=   ",
            "t=1728345600,v2=future_sig",
        ];

        for case in cases {
            let err =
                parse_signature_header(case).expect_err(&format!("expected error for {case}"));
            assert_eq!(err, StripeWebhookError::MissingSignature);
        }
    }

    #[test]
    fn test_empty_or_whitespace_header_returns_missing_signature_header() {
        let cases = ["", "   ", "\t\n\r  "];

        for case in cases {
            let err = parse_signature_header(case).expect_err("expected error for empty header");
            assert_eq!(err, StripeWebhookError::MissingSignatureHeader);
        }
    }

    #[test]
    fn test_ignores_non_v1_schemes_alongside_v1() {
        let raw = "t=1728345600,v0=legacy_scheme,v1=valid_v1_sig,v2=future_scheme";
        let parsed =
            parse_signature_header(raw).expect("should parse valid header with extra schemes");

        assert_eq!(parsed.timestamp, 1728345600);
        assert_eq!(parsed.signatures, vec!["valid_v1_sig"]);
    }

    #[test]
    fn test_zero_copy_borrowing_invariants() {
        let raw = String::from("t=1728345600,v1=borrowed_sig");
        let parsed = parse_signature_header(&raw).unwrap();

        // Verify pointers point directly inside the raw String memory buffer (no allocation)
        let raw_ptr_start = raw.as_ptr() as usize;
        let raw_ptr_end = raw_ptr_start + raw.len();
        let sig_ptr = parsed.signatures[0].as_ptr() as usize;

        assert!(
            sig_ptr >= raw_ptr_start && sig_ptr < raw_ptr_end,
            "Signature slice must be borrowed directly from the input header buffer"
        );
    }
}
