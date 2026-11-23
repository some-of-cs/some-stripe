//! Order-agnostic raw-body multi-`v1` Stripe webhook verifier (US-9.2.0 Unit 1).
//!
//! Provides constant-time, zero-clone cryptographic verification over exact raw
//! HTTP request body bytes prior to any JSON deserialization, supporting secret
//! rotation via multiple `v1=` signatures and multiple candidate webhook secrets.
//!
//! ## Security Invariants
//!
//! - **Order-Agnostic Header Parsing**: Accepts `t=` and `v1=` in any order,
//!   rejects headers exceeding [`MAX_V1_CANDIDATES`] (16) `v1=` candidates with
//!   [`StripeWebhookError::InvalidHeader`], rejects empty `t=` with
//!   [`StripeWebhookError::InvalidTimestamp`], and explicitly ignores `v0=` and
//!   unsigned debug keys (`secret`, `destination`, `payload`, etc.).
//! - **Fail-Closed Downgrade Protection**: Rejects headers lacking any `v1=`
//!   signature (including `v0`-only payloads) with [`StripeWebhookError::MissingSignature`].
//! - **Input & Resource Bounds**: Rejects empty/blank secret lists with
//!   [`StripeWebhookError::BadKey`] and headers exceeding [`MAX_HEADER_BYTES`] (4096).
//! - **Asymmetric Replay & Drift Protection**: Enforces
//!   `now_unix_secs - tolerance_secs <= timestamp <= now_unix_secs + MAX_FUTURE_DRIFT_SECONDS`
//!   (default [`DEFAULT_TOLERANCE_SECS`] = 300s, future drift ceiling = 60s).
//! - **Raw-Body Streaming HMAC Before JSON Parsing**: Computes streaming HMAC-SHA256
//!   over exact `raw_body` bytes via [`compute_streaming_hmac_bytes`], normalizes
//!   each `v1` candidate into a fixed `[u8; 64]` lowercase ASCII stack buffer, and
//!   compares every `v1` candidate against every valid secret using [`subtle::ConstantTimeEq`]
//!   without short-circuiting.
//! - **Thin Event Detection**: After cryptographic verification succeeds, parses
//!   `raw_body` as [`serde_json::Value`] and identifies Stripe v2 thin events.

use std::fmt;

use crate::compare::ct_ascii_to_lower;
use crate::constants::{MAX_FUTURE_DRIFT_SECONDS, MAX_WEBHOOK_PAYLOAD_BYTES};
use crate::crypto::compute_streaming_hmac_bytes;
use crate::error::StripeWebhookError;
use subtle::{Choice, ConstantTimeEq};

/// Default replay and clock-drift tolerance window in seconds (5 minutes).
pub const DEFAULT_TOLERANCE_SECS: i64 = 300;

/// Maximum permitted byte length of a `Stripe-Signature` header (4 KiB).
pub const MAX_HEADER_BYTES: usize = 4096;

/// Maximum number of `v1=` signature candidates permitted in a single header.
pub const MAX_V1_CANDIDATES: usize = 16;

/// Clock and replay-window configuration for Stripe webhook verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebhookVerificationOptions {
    /// Current UNIX epoch timestamp in seconds.
    pub now_unix_secs: i64,
    /// Maximum allowed past replay window in seconds.
    pub tolerance_secs: i64,
}

impl WebhookVerificationOptions {
    /// Creates verification options using [`DEFAULT_TOLERANCE_SECS`] (300 seconds).
    #[must_use]
    pub const fn new(now_unix_secs: i64) -> Self {
        Self {
            now_unix_secs,
            tolerance_secs: DEFAULT_TOLERANCE_SECS,
        }
    }
}

/// Cryptographically verified Stripe webhook envelope and parsed JSON payload.
///
/// Implements a custom [`fmt::Debug`] that always redacts [`Self::payload`] as
/// `"[REDACTED]"` while preserving non-secret envelope metadata.
#[derive(Clone, PartialEq, Eq)]
pub struct VerifiedStripeWebhook {
    /// UNIX epoch timestamp extracted from the verified `t=` signature parameter.
    pub timestamp: i64,
    /// Stripe event identifier (`id` field in the JSON payload).
    pub event_id: String,
    /// Stripe event type (`type` field in the JSON payload).
    pub event_type: String,
    /// Indicates whether the event originated in live mode (`true`) or test mode (`false`).
    pub livemode: bool,
    /// `true` when the payload represents a Stripe v2 thin event rather than a v1 snapshot event.
    pub is_thin_event: bool,
    /// Parsed JSON payload produced only after constant-time HMAC verification succeeded.
    pub payload: serde_json::Value,
}

impl fmt::Debug for VerifiedStripeWebhook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VerifiedStripeWebhook")
            .field("timestamp", &self.timestamp)
            .field("event_id", &self.event_id)
            .field("event_type", &self.event_type)
            .field("livemode", &self.livemode)
            .field("is_thin_event", &self.is_thin_event)
            .field("payload", &"[REDACTED]")
            .finish()
    }
}

struct ParsedSignatureHeader<'a> {
    timestamp: i64,
    raw_timestamp: &'a str,
    v1_candidates: Vec<&'a str>,
}

struct HeaderAccumulator<'a> {
    raw_timestamp: Option<&'a str>,
    v1_candidates: Vec<&'a str>,
}

impl<'a> HeaderAccumulator<'a> {
    fn new() -> Self {
        Self {
            raw_timestamp: None,
            v1_candidates: Vec::new(),
        }
    }

    fn collect_segment(&mut self, segment: &'a str) -> Result<(), StripeWebhookError> {
        let Some((raw_key, raw_val)) = segment.trim().split_once('=') else {
            return Ok(());
        };
        let key = raw_key.trim();
        let value = raw_val.trim();

        match key {
            "t" => self.record_timestamp(value),
            "v1" => self.record_v1_candidate(value),
            // Explicitly ignore `v0` and unsigned debug keys (`secret`, `destination`, `payload`).
            _ => Ok(()),
        }
    }

    fn record_timestamp(&mut self, value: &'a str) -> Result<(), StripeWebhookError> {
        if value.is_empty() || self.raw_timestamp.is_some() {
            return Err(StripeWebhookError::InvalidTimestamp);
        }
        self.raw_timestamp = Some(value);
        Ok(())
    }

    fn record_v1_candidate(&mut self, value: &'a str) -> Result<(), StripeWebhookError> {
        if value.is_empty() || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(());
        }
        if self.v1_candidates.len() >= MAX_V1_CANDIDATES {
            return Err(StripeWebhookError::InvalidHeader(
                "Stripe-Signature header exceeds maximum v1 signature candidates",
            ));
        }
        self.v1_candidates.push(value);
        Ok(())
    }

    fn finish(self) -> Result<ParsedSignatureHeader<'a>, StripeWebhookError> {
        let raw_timestamp = self
            .raw_timestamp
            .ok_or(StripeWebhookError::InvalidTimestamp)?;
        let timestamp = parse_unix_timestamp(raw_timestamp)?;
        if self.v1_candidates.is_empty() {
            return Err(StripeWebhookError::MissingSignature);
        }

        Ok(ParsedSignatureHeader {
            timestamp,
            raw_timestamp,
            v1_candidates: self.v1_candidates,
        })
    }
}

/// Parses a `Stripe-Signature` header in an order-agnostic manner.
///
/// Accepts `t=` and `v1=` entries in any order, collects up to [`MAX_V1_CANDIDATES`]
/// hexadecimal `v1=` signature candidates (rejecting headers exceeding that ceiling),
/// and explicitly ignores `v0=` and unsigned debug parameters (`secret`, `destination`,
/// `payload`, etc.).
///
/// Returns `(parsed_timestamp, raw_timestamp_slice, v1_candidates)` borrowing directly
/// from `header`.
///
/// # Errors
///
/// - [`StripeWebhookError::MissingSignatureHeader`] if `header` is empty or whitespace-only.
/// - [`StripeWebhookError::InvalidHeader`] if `header` exceeds [`MAX_HEADER_BYTES`] or
///   contains more than [`MAX_V1_CANDIDATES`] `v1=` candidates.
/// - [`StripeWebhookError::InvalidTimestamp`] if `t=` is missing, empty, duplicate, or not a valid UNIX timestamp.
/// - [`StripeWebhookError::MissingSignature`] if no valid `v1=` signature is present.
///
/// # Examples
///
/// ```
/// use some_stripe_webhooks::multi_v1_verifier::parse_signature_header_order_agnostic;
///
/// let header = "v1=deadbeef,v0=ignored,secret=debug,t=1728345600,v1=cafebabe";
/// let (ts, raw_ts, sigs) = parse_signature_header_order_agnostic(header).unwrap();
/// assert_eq!(ts, 1728345600);
/// assert_eq!(raw_ts, "1728345600");
/// assert_eq!(sigs, vec!["deadbeef", "cafebabe"]);
/// ```
pub fn parse_signature_header_order_agnostic(
    header: &str,
) -> Result<(i64, &str, Vec<&str>), StripeWebhookError> {
    let parsed = parse_signature_header_internal(header)?;
    Ok((parsed.timestamp, parsed.raw_timestamp, parsed.v1_candidates))
}

/// Verifies a raw Stripe webhook body against one or more signing secrets and parses
/// the verified JSON payload.
///
/// Cryptographic verification streams `raw_timestamp` and exact `raw_body` bytes into
/// HMAC-SHA256 via [`compute_streaming_hmac_bytes`] and compares every `v1=` candidate
/// against every valid secret in constant time using [`subtle::ConstantTimeEq`] **before**
/// parsing `raw_body` as JSON.
///
/// # Errors
///
/// - [`StripeWebhookError::EmptyPayload`] if `raw_body` is empty.
/// - [`StripeWebhookError::PayloadTooLarge`] if `raw_body` exceeds [`MAX_WEBHOOK_PAYLOAD_BYTES`].
/// - [`StripeWebhookError::BadKey`] if `secrets` is empty or contains only blank strings.
/// - [`StripeWebhookError::MissingSignatureHeader`], [`StripeWebhookError::InvalidHeader`],
///   [`StripeWebhookError::InvalidTimestamp`], or [`StripeWebhookError::MissingSignature`]
///   if `signature_header` is malformed or lacks a `v1=` signature.
/// - [`StripeWebhookError::TimestampOutsideTolerance`] if `timestamp` is older than
///   `now_unix_secs - tolerance_secs` or newer than `now_unix_secs + MAX_FUTURE_DRIFT_SECONDS`.
/// - [`StripeWebhookError::BadSignature`] if no `(secret, v1_candidate)` pair matches.
/// - [`StripeWebhookError::MalformedPayload`] if the verified body is not a valid JSON object.
///
/// # Examples
///
/// ```
/// use some_stripe_webhooks::crypto::compute_streaming_hmac_bytes;
/// use some_stripe_webhooks::multi_v1_verifier::{
///     WebhookVerificationOptions, verify_stripe_raw_webhook,
/// };
///
/// let raw_body = br#"{"id":"evt_123","object":"event","type":"payment_intent.succeeded","livemode":false,"data":{"object":{"id":"pi_123"}}}"#;
/// let secret = "whsec_test_secret";
/// let ts = 1728345600_i64;
/// let sig = compute_streaming_hmac_bytes(secret.as_bytes(), b"1728345600", raw_body).unwrap();
/// let header = format!("v1={sig},t={ts}");
///
/// let verified = verify_stripe_raw_webhook(
///     raw_body,
///     &header,
///     &[secret],
///     WebhookVerificationOptions::new(ts + 10),
/// )
/// .unwrap();
///
/// assert_eq!(verified.timestamp, ts);
/// assert_eq!(verified.event_id, "evt_123");
/// assert_eq!(verified.event_type, "payment_intent.succeeded");
/// assert!(!verified.livemode);
/// assert!(!verified.is_thin_event);
/// ```
pub fn verify_stripe_raw_webhook(
    raw_body: &[u8],
    signature_header: &str,
    secrets: &[&str],
    options: WebhookVerificationOptions,
) -> Result<VerifiedStripeWebhook, StripeWebhookError> {
    validate_raw_body_bounds(raw_body)?;
    let valid_secrets = collect_valid_secrets(secrets)?;
    let parsed = parse_signature_header_internal(signature_header)?;
    check_timestamp_tolerance(parsed.timestamp, options)?;
    verify_any_secret_matches(&valid_secrets, raw_body, &parsed)?;

    parse_verified_payload(raw_body, parsed.timestamp)
}

fn validate_raw_body_bounds(raw_body: &[u8]) -> Result<(), StripeWebhookError> {
    if raw_body.is_empty() {
        return Err(StripeWebhookError::EmptyPayload);
    }
    if raw_body.len() > MAX_WEBHOOK_PAYLOAD_BYTES {
        return Err(StripeWebhookError::PayloadTooLarge {
            size: raw_body.len(),
            max_allowed: MAX_WEBHOOK_PAYLOAD_BYTES,
        });
    }
    Ok(())
}

fn parse_signature_header_internal(
    header: &str,
) -> Result<ParsedSignatureHeader<'_>, StripeWebhookError> {
    if header.trim().is_empty() {
        return Err(StripeWebhookError::MissingSignatureHeader);
    }
    if header.len() > MAX_HEADER_BYTES {
        return Err(StripeWebhookError::InvalidHeader(
            "Stripe-Signature header exceeds maximum allowed size",
        ));
    }

    let mut accumulator = HeaderAccumulator::new();
    for segment in header.split(',') {
        accumulator.collect_segment(segment)?;
    }
    accumulator.finish()
}

fn parse_unix_timestamp(raw_ts: &str) -> Result<i64, StripeWebhookError> {
    let is_canonical = !raw_ts.is_empty()
        && raw_ts.bytes().all(|b| b.is_ascii_digit())
        && (raw_ts.len() == 1 || !raw_ts.starts_with('0'));
    if !is_canonical {
        return Err(StripeWebhookError::InvalidTimestamp);
    }
    raw_ts
        .parse::<i64>()
        .map_err(|_| StripeWebhookError::InvalidTimestamp)
}

fn collect_valid_secrets<'a>(secrets: &'a [&'a str]) -> Result<Vec<&'a str>, StripeWebhookError> {
    let valid: Vec<&'a str> = secrets
        .iter()
        .copied()
        .filter(|secret| !secret.trim().is_empty())
        .collect();
    if valid.is_empty() {
        return Err(StripeWebhookError::BadKey);
    }
    Ok(valid)
}

fn check_timestamp_tolerance(
    timestamp: i64,
    options: WebhookVerificationOptions,
) -> Result<(), StripeWebhookError> {
    if options.tolerance_secs < 0 {
        return Err(StripeWebhookError::TimestampOutsideTolerance {
            timestamp,
            now: options.now_unix_secs,
            tolerance: 0,
        });
    }
    let max_tolerance = u64::try_from(options.tolerance_secs).unwrap_or(0);
    let lower_bound = options.now_unix_secs.saturating_sub(options.tolerance_secs);
    let upper_bound = options
        .now_unix_secs
        .saturating_add(MAX_FUTURE_DRIFT_SECONDS);
    if timestamp < lower_bound || timestamp > upper_bound {
        return Err(StripeWebhookError::TimestampOutsideTolerance {
            timestamp,
            now: options.now_unix_secs,
            tolerance: max_tolerance,
        });
    }
    Ok(())
}

fn ct_eq_hex_64(expected: &[u8], candidate: &[u8]) -> Choice {
    if expected.len() != 64 || candidate.len() != 64 {
        return Choice::from(0u8);
    }
    let mut normalized = [0u8; 64];
    for (dst, &src) in normalized.iter_mut().zip(candidate.iter()) {
        *dst = ct_ascii_to_lower(src);
    }
    expected.ct_eq(&normalized)
}

fn verify_secret_candidates(
    secret: &str,
    raw_body: &[u8],
    parsed: &ParsedSignatureHeader<'_>,
) -> Result<Choice, StripeWebhookError> {
    let ts_bytes = parsed.raw_timestamp.as_bytes();
    let expected = compute_streaming_hmac_bytes(secret.as_bytes(), ts_bytes, raw_body)?;
    let mut matched = Choice::from(0u8);
    for candidate in &parsed.v1_candidates {
        matched |= ct_eq_hex_64(expected.as_bytes(), candidate.as_bytes());
    }
    Ok(matched)
}

fn verify_any_secret_matches(
    valid_secrets: &[&str],
    raw_body: &[u8],
    parsed: &ParsedSignatureHeader<'_>,
) -> Result<(), StripeWebhookError> {
    let mut matched = Choice::from(0u8);
    for secret in valid_secrets {
        matched |= verify_secret_candidates(secret, raw_body, parsed)?;
    }
    if !bool::from(matched) {
        return Err(StripeWebhookError::BadSignature);
    }
    Ok(())
}

fn parse_verified_payload(
    raw_body: &[u8],
    timestamp: i64,
) -> Result<VerifiedStripeWebhook, StripeWebhookError> {
    let payload: serde_json::Value = serde_json::from_slice(raw_body)
        .map_err(|err| StripeWebhookError::MalformedPayload(err.to_string()))?;
    if !payload.is_object() {
        return Err(StripeWebhookError::MalformedPayload(
            "webhook payload must be a JSON object".to_string(),
        ));
    }

    Ok(VerifiedStripeWebhook {
        timestamp,
        event_id: extract_string_field(&payload, "id"),
        event_type: extract_string_field(&payload, "type"),
        livemode: payload
            .get("livemode")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        is_thin_event: detect_thin_event(&payload),
        payload,
    })
}

fn extract_string_field(payload: &serde_json::Value, key: &str) -> String {
    payload
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn detect_thin_event(payload: &serde_json::Value) -> bool {
    let is_v2_core_event =
        payload.get("object").and_then(serde_json::Value::as_str) == Some("v2.core.event");
    let has_related_object = payload
        .get("related_object")
        .is_some_and(|value| !value.is_null());
    let has_data = payload.get("data").is_some_and(|value| !value.is_null());

    is_v2_core_event || (has_related_object && !has_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_SECRET: &str = "whsec_unit1_test_secret";
    const SAMPLE_TS: i64 = 1_728_345_600;
    const SAMPLE_TS_STR: &str = "1728345600";
    const SNAPSHOT_BODY: &[u8] = br#"{"id":"evt_snap_001","object":"event","type":"payment_intent.succeeded","livemode":true,"data":{"object":{"id":"pi_123"}}}"#;
    const THIN_V2_BODY: &[u8] = br#"{"id":"v2_evt_001","object":"v2.core.event","type":"v1.billing.meter.error_report_triggered","livemode":false,"related_object":{"id":"mtr_123","type":"billing.meter","url":"/v2/billing/meters/mtr_123"}}"#;
    const THIN_RELATED_ONLY_BODY: &[u8] = br#"{"id":"evt_thin_002","object":"event","type":"billing.meter.updated","livemode":false,"related_object":{"id":"mtr_456","type":"billing.meter"}}"#;

    fn sign(secret: &str, body: &[u8]) -> String {
        compute_streaming_hmac_bytes(secret.as_bytes(), SAMPLE_TS_STR.as_bytes(), body)
            .expect("HMAC-SHA256 signing must succeed")
    }

    fn signed_header(secret: &str, body: &[u8]) -> String {
        format!("t={SAMPLE_TS_STR},v1={}", sign(secret, body))
    }

    fn verify_with(
        body: &[u8],
        header: &str,
        secrets: &[&str],
    ) -> Result<VerifiedStripeWebhook, StripeWebhookError> {
        verify_stripe_raw_webhook(
            body,
            header,
            secrets,
            WebhookVerificationOptions::new(SAMPLE_TS),
        )
    }

    fn verify_signed_body(body: &[u8]) -> VerifiedStripeWebhook {
        let header = signed_header(DEFAULT_SECRET, body);
        verify_with(body, &header, &[DEFAULT_SECRET]).expect("signed body must verify")
    }

    #[test]
    fn test_dual_v1_secret_rotation_succeeds() {
        let old_secret = "whsec_old_rotated_secret";
        let new_secret = "whsec_new_active_secret";
        let header = format!(
            "t={SAMPLE_TS_STR},v1={},v1={}",
            sign(old_secret, SNAPSHOT_BODY),
            sign(new_secret, SNAPSHOT_BODY)
        );

        let via_new = verify_with(SNAPSHOT_BODY, &header, &["whsec_unused", new_secret])
            .expect("active rotated secret must succeed");
        let via_old = verify_with(SNAPSHOT_BODY, &header, &[old_secret])
            .expect("previous secret must succeed during rotation");

        assert_eq!(via_new.event_id, "evt_snap_001");
        assert_eq!(via_old.event_id, "evt_snap_001");
        assert!(via_new.livemode && !via_new.is_thin_event);
    }

    #[test]
    fn test_uppercase_hex_v1_signature_verifies() {
        let sig_upper = sign(DEFAULT_SECRET, SNAPSHOT_BODY).to_ascii_uppercase();
        let header = format!("t={SAMPLE_TS_STR},v1={sig_upper}");

        let verified = verify_with(SNAPSHOT_BODY, &header, &[DEFAULT_SECRET])
            .expect("uppercase hex v1 signature must verify");
        assert_eq!(verified.event_id, "evt_snap_001");
    }

    #[test]
    fn test_empty_timestamp_parameter_rejected() {
        let sig = sign(DEFAULT_SECRET, SNAPSHOT_BODY);
        for header in [
            format!("t=,v1={sig}"),
            format!("t=,t={SAMPLE_TS_STR},v1={sig}"),
        ] {
            assert_eq!(
                parse_signature_header_order_agnostic(&header),
                Err(StripeWebhookError::InvalidTimestamp)
            );
        }
    }

    #[test]
    fn test_asymmetric_future_drift_enforced() {
        let header = signed_header(DEFAULT_SECRET, SNAPSHOT_BODY);

        let within_future = WebhookVerificationOptions::new(SAMPLE_TS - MAX_FUTURE_DRIFT_SECONDS);
        assert!(
            verify_stripe_raw_webhook(SNAPSHOT_BODY, &header, &[DEFAULT_SECRET], within_future)
                .is_ok()
        );

        let beyond_future_now = SAMPLE_TS - (MAX_FUTURE_DRIFT_SECONDS + 1);
        let beyond_future = WebhookVerificationOptions::new(beyond_future_now);
        assert_eq!(
            verify_stripe_raw_webhook(SNAPSHOT_BODY, &header, &[DEFAULT_SECRET], beyond_future),
            Err(StripeWebhookError::TimestampOutsideTolerance {
                timestamp: SAMPLE_TS,
                now: beyond_future_now,
                tolerance: 300,
            })
        );
    }

    #[test]
    fn test_order_agnostic_and_debug_keys_ignored() {
        let sig = sign(DEFAULT_SECRET, SNAPSHOT_BODY);
        let headers = [
            format!("v1={sig},t={SAMPLE_TS_STR}"),
            format!(
                "secret=leaked,destination=https://attacker.example,payload=fake,v0=deadbeef,v1={sig},t={SAMPLE_TS_STR}"
            ),
        ];

        for header in &headers {
            let (ts, raw_ts, cands) =
                parse_signature_header_order_agnostic(header).expect("header must parse");
            assert_eq!(ts, SAMPLE_TS);
            assert_eq!(raw_ts, SAMPLE_TS_STR);
            assert_eq!(cands, vec![sig.as_str()]);

            let verified = verify_with(SNAPSHOT_BODY, header, &[DEFAULT_SECRET]);
            assert_eq!(
                verified.expect("header must verify").event_id,
                "evt_snap_001"
            );
        }
    }

    #[test]
    fn test_v0_only_downgrade_rejected_fail_closed() {
        let sig = sign(DEFAULT_SECRET, SNAPSHOT_BODY);
        let header = format!("t={SAMPLE_TS_STR},v0={sig}");

        assert_eq!(
            parse_signature_header_order_agnostic(&header),
            Err(StripeWebhookError::MissingSignature)
        );
        assert_eq!(
            verify_with(SNAPSHOT_BODY, &header, &[DEFAULT_SECRET]),
            Err(StripeWebhookError::MissingSignature)
        );
    }

    #[test]
    fn test_timestamps_outside_tolerance_rejected() {
        let header = signed_header(DEFAULT_SECRET, SNAPSHOT_BODY);
        for offset in [DEFAULT_TOLERANCE_SECS + 1, -(DEFAULT_TOLERANCE_SECS + 1)] {
            let now = SAMPLE_TS + offset;
            let opts = WebhookVerificationOptions::new(now);
            let res = verify_stripe_raw_webhook(SNAPSHOT_BODY, &header, &[DEFAULT_SECRET], opts);
            let expected = StripeWebhookError::TimestampOutsideTolerance {
                timestamp: SAMPLE_TS,
                now,
                tolerance: 300,
            };
            assert_eq!(res, Err(expected));
        }
    }

    #[test]
    fn test_reserialized_whitespace_json_rejected() {
        let header = signed_header(DEFAULT_SECRET, SNAPSHOT_BODY);
        let parsed: serde_json::Value =
            serde_json::from_slice(SNAPSHOT_BODY).expect("fixture must be valid JSON");
        let reserialized =
            serde_json::to_vec_pretty(&parsed).expect("pretty serialization must succeed");

        assert_ne!(reserialized.as_slice(), SNAPSHOT_BODY);
        assert_eq!(
            verify_with(&reserialized, &header, &[DEFAULT_SECRET]),
            Err(StripeWebhookError::BadSignature)
        );
    }

    #[test]
    fn test_thin_event_vs_snapshot_event_detection() {
        let v2_thin = verify_signed_body(THIN_V2_BODY);
        let related_thin = verify_signed_body(THIN_RELATED_ONLY_BODY);
        let snapshot = verify_signed_body(SNAPSHOT_BODY);

        assert!(v2_thin.is_thin_event && v2_thin.event_id == "v2_evt_001");
        assert!(related_thin.is_thin_event && related_thin.event_id == "evt_thin_002");
        assert!(!snapshot.is_thin_event && snapshot.event_type == "payment_intent.succeeded");
    }

    #[test]
    fn test_blank_and_empty_secrets_rejected() {
        let header = format!("t={SAMPLE_TS_STR},v1={}", "a".repeat(64));

        assert_eq!(
            verify_with(SNAPSHOT_BODY, &header, &[]),
            Err(StripeWebhookError::BadKey)
        );
        assert_eq!(
            verify_with(SNAPSHOT_BODY, &header, &["", "   \t\n "]),
            Err(StripeWebhookError::BadKey)
        );
    }

    #[test]
    fn test_oversized_header_and_v1_candidate_cap() {
        let oversized = format!("t={SAMPLE_TS_STR},v1={}", "a".repeat(MAX_HEADER_BYTES));
        assert!(matches!(
            parse_signature_header_order_agnostic(&oversized),
            Err(StripeWebhookError::InvalidHeader(_))
        ));

        let mut max_parts = vec![format!("t={SAMPLE_TS_STR}")];
        for i in 0..MAX_V1_CANDIDATES {
            max_parts.push(format!("v1={i:064x}"));
        }
        let max_header = max_parts.join(",");
        let (_, _, candidates) = parse_signature_header_order_agnostic(&max_header)
            .expect("exact MAX_V1_CANDIDATES should parse");
        assert_eq!(candidates.len(), MAX_V1_CANDIDATES);

        max_parts.push(format!("v1={MAX_V1_CANDIDATES:064x}"));
        let over_header = max_parts.join(",");
        assert!(matches!(
            parse_signature_header_order_agnostic(&over_header),
            Err(StripeWebhookError::InvalidHeader(_))
        ));
    }

    #[test]
    fn test_crypto_verification_precedes_json_parsing() {
        let malformed_json = b"{not valid json";
        let bad_header = format!("t={SAMPLE_TS_STR},v1={}", "0".repeat(64));
        let valid_header = signed_header(DEFAULT_SECRET, malformed_json);

        assert_eq!(
            verify_with(malformed_json, &bad_header, &[DEFAULT_SECRET]),
            Err(StripeWebhookError::BadSignature)
        );
        assert!(matches!(
            verify_with(malformed_json, &valid_header, &[DEFAULT_SECRET]),
            Err(StripeWebhookError::MalformedPayload(_))
        ));
    }

    #[test]
    fn test_verified_webhook_debug_redacts_payload() {
        let secret_body = br#"{"id":"evt_dbg_1","object":"event","type":"payment_intent.succeeded","livemode":false,"data":{"object":{"client_secret":"pi_secret_never_log_me"}}}"#;
        let verified = verify_signed_body(secret_body);
        let rendered = format!("{verified:?}");
        assert!(rendered.contains("[REDACTED]"));
        assert!(rendered.contains("evt_dbg_1"));
        assert!(!rendered.contains("pi_secret_never_log_me"));
    }

    #[test]
    fn test_duplicate_timestamp_parameter_rejected() {
        let header = format!("t={SAMPLE_TS_STR},t=1728345601,v1={}", "a".repeat(64));
        assert_eq!(
            parse_signature_header_order_agnostic(&header),
            Err(StripeWebhookError::InvalidTimestamp)
        );
    }
}
