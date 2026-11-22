//! Order-agnostic raw-body multi-`v1` Stripe webhook verifier (US-9.2.0 Unit 1).
//!
//! Provides constant-time, zero-clone cryptographic verification over exact raw
//! HTTP request body bytes prior to any JSON deserialization, supporting secret
//! rotation via multiple `v1=` signatures and multiple candidate webhook secrets.
//!
//! ## Security Invariants
//!
//! - **Order-Agnostic Header Parsing**: Accepts `t=` and `v1=` in any order,
//!   caps collected `v1=` candidates at [`MAX_V1_CANDIDATES`] (16), and explicitly
//!   ignores `v0=` and unsigned debug keys (`secret`, `destination`, `payload`, etc.).
//! - **Fail-Closed Downgrade Protection**: Rejects headers lacking any `v1=`
//!   signature (including `v0`-only payloads) with [`StripeWebhookError::MissingSignature`].
//! - **Input & Resource Bounds**: Rejects empty/blank secret lists with
//!   [`StripeWebhookError::BadKey`] and headers exceeding [`MAX_HEADER_BYTES`] (4096).
//! - **Replay & Drift Protection**: Enforces `|now_unix_secs - timestamp| <= tolerance_secs`
//!   (default [`DEFAULT_TOLERANCE_SECS`] = 300s).
//! - **Raw-Body Streaming HMAC Before JSON Parsing**: Computes streaming HMAC-SHA256
//!   over exact `raw_body` bytes via [`compute_streaming_hmac_bytes`] and compares
//!   every `v1` candidate against every valid secret using [`subtle::ConstantTimeEq`]
//!   without short-circuiting.
//! - **Thin Event Detection**: After cryptographic verification succeeds, parses
//!   `raw_body` as [`serde_json::Value`] and identifies Stripe v2 thin events.

use crate::constants::MAX_WEBHOOK_PAYLOAD_BYTES;
use crate::crypto::compute_streaming_hmac_bytes;
use crate::error::StripeWebhookError;
use subtle::{Choice, ConstantTimeEq};

/// Default replay and clock-drift tolerance window in seconds (5 minutes).
pub const DEFAULT_TOLERANCE_SECS: i64 = 300;

/// Maximum permitted byte length of a `Stripe-Signature` header (4 KiB).
pub const MAX_HEADER_BYTES: usize = 4096;

/// Maximum number of `v1=` signature candidates collected from a single header.
pub const MAX_V1_CANDIDATES: usize = 16;

/// Cryptographically verified Stripe webhook envelope and parsed JSON payload.
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// Parses a `Stripe-Signature` header in an order-agnostic manner.
///
/// Accepts `t=` and `v1=` entries in any order, collects up to [`MAX_V1_CANDIDATES`]
/// hexadecimal `v1=` signature candidates, and explicitly ignores `v0=` and unsigned
/// debug parameters (`secret`, `destination`, `payload`, etc.).
///
/// Returns `(parsed_timestamp, raw_timestamp_slice, v1_candidates)` borrowing directly
/// from `header`.
///
/// # Errors
///
/// - [`StripeWebhookError::MissingSignatureHeader`] if `header` is empty or whitespace-only.
/// - [`StripeWebhookError::InvalidHeader`] if `header` exceeds [`MAX_HEADER_BYTES`].
/// - [`StripeWebhookError::InvalidTimestamp`] if `t=` is missing or not a valid UNIX timestamp.
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
    if header.trim().is_empty() {
        return Err(StripeWebhookError::MissingSignatureHeader);
    }
    if header.len() > MAX_HEADER_BYTES {
        return Err(StripeWebhookError::InvalidHeader(
            "Stripe-Signature header exceeds maximum allowed size",
        ));
    }

    let mut raw_timestamp: Option<&str> = None;
    let mut v1_candidates: Vec<&str> = Vec::new();
    for segment in header.split(',') {
        collect_header_segment(segment, &mut raw_timestamp, &mut v1_candidates);
    }

    let raw_ts = raw_timestamp.ok_or(StripeWebhookError::InvalidTimestamp)?;
    let timestamp = parse_unix_timestamp(raw_ts)?;
    if v1_candidates.is_empty() {
        return Err(StripeWebhookError::MissingSignature);
    }

    Ok((timestamp, raw_ts, v1_candidates))
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
/// - [`StripeWebhookError::TimestampOutsideTolerance`] if `|now_unix_secs - timestamp| > tolerance_secs`.
/// - [`StripeWebhookError::BadSignature`] if no `(secret, v1_candidate)` pair matches.
/// - [`StripeWebhookError::MalformedPayload`] if the verified body is not a valid JSON object.
///
/// # Examples
///
/// ```
/// use some_stripe_webhooks::crypto::compute_streaming_hmac_bytes;
/// use some_stripe_webhooks::multi_v1_verifier::{
///     DEFAULT_TOLERANCE_SECS, verify_stripe_raw_webhook,
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
///     ts + 10,
///     DEFAULT_TOLERANCE_SECS,
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
    now_unix_secs: i64,
    tolerance_secs: i64,
) -> Result<VerifiedStripeWebhook, StripeWebhookError> {
    if raw_body.is_empty() {
        return Err(StripeWebhookError::EmptyPayload);
    }
    if raw_body.len() > MAX_WEBHOOK_PAYLOAD_BYTES {
        return Err(StripeWebhookError::PayloadTooLarge {
            size: raw_body.len(),
            max_allowed: MAX_WEBHOOK_PAYLOAD_BYTES,
        });
    }

    let valid_secrets = collect_valid_secrets(secrets)?;
    let (timestamp, raw_timestamp, v1_candidates) =
        parse_signature_header_order_agnostic(signature_header)?;
    check_timestamp_tolerance(timestamp, now_unix_secs, tolerance_secs)?;
    verify_any_secret_matches(&valid_secrets, raw_timestamp, raw_body, &v1_candidates)?;

    parse_verified_payload(raw_body, timestamp)
}

fn collect_header_segment<'a>(
    segment: &'a str,
    raw_timestamp: &mut Option<&'a str>,
    v1_candidates: &mut Vec<&'a str>,
) {
    let Some((raw_key, raw_val)) = segment.trim().split_once('=') else {
        return;
    };
    let key = raw_key.trim();
    let value = raw_val.trim();
    if value.is_empty() {
        return;
    }

    match key {
        "t" if raw_timestamp.is_none() => *raw_timestamp = Some(value),
        "v1" if is_v1_candidate_eligible(value, v1_candidates.len()) => {
            v1_candidates.push(value);
        }
        // Explicitly ignore `v0` and unsigned debug keys (`secret`, `destination`, `payload`, etc.).
        _ => {}
    }
}

fn is_v1_candidate_eligible(value: &str, current_count: usize) -> bool {
    current_count < MAX_V1_CANDIDATES && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn parse_unix_timestamp(raw_ts: &str) -> Result<i64, StripeWebhookError> {
    if !raw_ts.bytes().all(|b| b.is_ascii_digit()) {
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
    now_unix_secs: i64,
    tolerance_secs: i64,
) -> Result<(), StripeWebhookError> {
    if tolerance_secs < 0 {
        return Err(StripeWebhookError::TimestampOutsideTolerance {
            timestamp,
            now: now_unix_secs,
            tolerance: 0,
        });
    }
    let delta = now_unix_secs.abs_diff(timestamp);
    let max_tolerance = u64::try_from(tolerance_secs).unwrap_or(0);
    if delta > max_tolerance {
        return Err(StripeWebhookError::TimestampOutsideTolerance {
            timestamp,
            now: now_unix_secs,
            tolerance: max_tolerance,
        });
    }
    Ok(())
}

fn verify_secret_candidates(
    secret: &str,
    raw_timestamp: &str,
    raw_body: &[u8],
    v1_candidates: &[&str],
) -> Result<Choice, StripeWebhookError> {
    let expected =
        compute_streaming_hmac_bytes(secret.as_bytes(), raw_timestamp.as_bytes(), raw_body)?;
    let mut matched = Choice::from(0u8);
    for candidate in v1_candidates {
        matched |= expected.as_bytes().ct_eq(candidate.as_bytes());
    }
    Ok(matched)
}

fn verify_any_secret_matches(
    valid_secrets: &[&str],
    raw_timestamp: &str,
    raw_body: &[u8],
    v1_candidates: &[&str],
) -> Result<(), StripeWebhookError> {
    let mut matched = Choice::from(0u8);
    for secret in valid_secrets {
        matched |= verify_secret_candidates(secret, raw_timestamp, raw_body, v1_candidates)?;
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

    let event_id = extract_string_field(&payload, "id");
    let event_type = extract_string_field(&payload, "type");
    let livemode = payload
        .get("livemode")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let is_thin_event = detect_thin_event(&payload);

    Ok(VerifiedStripeWebhook {
        timestamp,
        event_id,
        event_type,
        livemode,
        is_thin_event,
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

    const SAMPLE_TS: i64 = 1_728_345_600;
    const SAMPLE_TS_STR: &str = "1728345600";
    const SNAPSHOT_BODY: &[u8] = br#"{"id":"evt_snap_001","object":"event","type":"payment_intent.succeeded","livemode":true,"data":{"object":{"id":"pi_123"}}}"#;
    const THIN_V2_BODY: &[u8] = br#"{"id":"v2_evt_001","object":"v2.core.event","type":"v1.billing.meter.error_report_triggered","livemode":false,"related_object":{"id":"mtr_123","type":"billing.meter","url":"/v2/billing/meters/mtr_123"}}"#;
    const THIN_RELATED_ONLY_BODY: &[u8] = br#"{"id":"evt_thin_002","object":"event","type":"billing.meter.updated","livemode":false,"related_object":{"id":"mtr_456","type":"billing.meter"}}"#;

    fn sign(secret: &str, ts_str: &str, body: &[u8]) -> String {
        compute_streaming_hmac_bytes(secret.as_bytes(), ts_str.as_bytes(), body)
            .expect("HMAC-SHA256 signing must succeed")
    }

    #[test]
    fn test_dual_v1_secret_rotation_succeeds() {
        let old_secret = "whsec_old_rotated_secret";
        let new_secret = "whsec_new_active_secret";
        let sig_old = sign(old_secret, SAMPLE_TS_STR, SNAPSHOT_BODY);
        let sig_new = sign(new_secret, SAMPLE_TS_STR, SNAPSHOT_BODY);
        let header = format!("t={SAMPLE_TS_STR},v1={sig_old},v1={sig_new}");

        let verified_new = verify_stripe_raw_webhook(
            SNAPSHOT_BODY,
            &header,
            &["whsec_unrelated", new_secret],
            SAMPLE_TS + 5,
            DEFAULT_TOLERANCE_SECS,
        )
        .expect("dual-v1 secret rotation should accept active secret");
        assert_eq!(verified_new.timestamp, SAMPLE_TS);
        assert_eq!(verified_new.event_id, "evt_snap_001");
        assert_eq!(verified_new.event_type, "payment_intent.succeeded");
        assert!(verified_new.livemode);
        assert!(!verified_new.is_thin_event);

        let verified_old = verify_stripe_raw_webhook(
            SNAPSHOT_BODY,
            &header,
            &[old_secret],
            SAMPLE_TS + 5,
            DEFAULT_TOLERANCE_SECS,
        )
        .expect("dual-v1 secret rotation should accept previous secret during rollover");
        assert_eq!(verified_old.event_id, "evt_snap_001");
    }

    #[test]
    fn test_order_agnostic_v1_before_t_succeeds() {
        let secret = "whsec_order_test";
        let sig = sign(secret, SAMPLE_TS_STR, SNAPSHOT_BODY);
        let header = format!("v1={sig},t={SAMPLE_TS_STR}");

        let (ts, raw_ts, cands) =
            parse_signature_header_order_agnostic(&header).expect("v1 before t must parse");
        assert_eq!(ts, SAMPLE_TS);
        assert_eq!(raw_ts, SAMPLE_TS_STR);
        assert_eq!(cands, vec![sig.as_str()]);

        let verified = verify_stripe_raw_webhook(
            SNAPSHOT_BODY,
            &header,
            &[secret],
            SAMPLE_TS,
            DEFAULT_TOLERANCE_SECS,
        )
        .expect("v1 before t must verify");
        assert_eq!(verified.event_id, "evt_snap_001");
    }

    #[test]
    fn test_v0_only_downgrade_rejected_fail_closed() {
        let secret = "whsec_downgrade_test";
        let forged_v0 = sign(secret, SAMPLE_TS_STR, SNAPSHOT_BODY);
        let header = format!("t={SAMPLE_TS_STR},v0={forged_v0}");

        let parse_err = parse_signature_header_order_agnostic(&header).unwrap_err();
        assert_eq!(parse_err, StripeWebhookError::MissingSignature);

        let verify_err = verify_stripe_raw_webhook(
            SNAPSHOT_BODY,
            &header,
            &[secret],
            SAMPLE_TS,
            DEFAULT_TOLERANCE_SECS,
        )
        .unwrap_err();
        assert_eq!(verify_err, StripeWebhookError::MissingSignature);
    }

    #[test]
    fn test_unsigned_debug_entries_and_v0_ignored() {
        let secret = "whsec_debug_filter";
        let sig = sign(secret, SAMPLE_TS_STR, SNAPSHOT_BODY);
        let header = format!(
            "secret=leaked,destination=https://attacker.example,payload=fake,v0=deadbeef,v1={sig},t={SAMPLE_TS_STR}"
        );

        let (ts, raw_ts, cands) =
            parse_signature_header_order_agnostic(&header).expect("debug entries must be ignored");
        assert_eq!(ts, SAMPLE_TS);
        assert_eq!(raw_ts, SAMPLE_TS_STR);
        assert_eq!(cands, vec![sig.as_str()]);

        let verified = verify_stripe_raw_webhook(
            SNAPSHOT_BODY,
            &header,
            &[secret],
            SAMPLE_TS,
            DEFAULT_TOLERANCE_SECS,
        )
        .expect("verification should succeed while ignoring unsigned debug keys");
        assert_eq!(verified.event_id, "evt_snap_001");
    }

    #[test]
    fn test_stale_and_future_timestamps_outside_tolerance_rejected() {
        let secret = "whsec_tolerance_test";
        let sig = sign(secret, SAMPLE_TS_STR, SNAPSHOT_BODY);
        let header = format!("t={SAMPLE_TS_STR},v1={sig}");

        let stale_err = verify_stripe_raw_webhook(
            SNAPSHOT_BODY,
            &header,
            &[secret],
            SAMPLE_TS + DEFAULT_TOLERANCE_SECS + 1,
            DEFAULT_TOLERANCE_SECS,
        )
        .unwrap_err();
        assert_eq!(
            stale_err,
            StripeWebhookError::TimestampOutsideTolerance {
                timestamp: SAMPLE_TS,
                now: SAMPLE_TS + DEFAULT_TOLERANCE_SECS + 1,
                tolerance: 300,
            }
        );

        let future_err = verify_stripe_raw_webhook(
            SNAPSHOT_BODY,
            &header,
            &[secret],
            SAMPLE_TS - DEFAULT_TOLERANCE_SECS - 1,
            DEFAULT_TOLERANCE_SECS,
        )
        .unwrap_err();
        assert_eq!(
            future_err,
            StripeWebhookError::TimestampOutsideTolerance {
                timestamp: SAMPLE_TS,
                now: SAMPLE_TS - DEFAULT_TOLERANCE_SECS - 1,
                tolerance: 300,
            }
        );
    }

    #[test]
    fn test_reserialized_whitespace_json_rejected() {
        let secret = "whsec_raw_bytes_only";
        let sig = sign(secret, SAMPLE_TS_STR, SNAPSHOT_BODY);
        let header = format!("t={SAMPLE_TS_STR},v1={sig}");

        let parsed: serde_json::Value =
            serde_json::from_slice(SNAPSHOT_BODY).expect("fixture must be valid JSON");
        let reserialized =
            serde_json::to_vec_pretty(&parsed).expect("pretty serialization must succeed");
        assert_ne!(reserialized.as_slice(), SNAPSHOT_BODY);

        let err = verify_stripe_raw_webhook(
            &reserialized,
            &header,
            &[secret],
            SAMPLE_TS,
            DEFAULT_TOLERANCE_SECS,
        )
        .unwrap_err();
        assert_eq!(err, StripeWebhookError::BadSignature);
    }

    #[test]
    fn test_thin_event_vs_snapshot_event_detection() {
        let secret = "whsec_thin_detection";

        let sig_v2 = sign(secret, SAMPLE_TS_STR, THIN_V2_BODY);
        let header_v2 = format!("t={SAMPLE_TS_STR},v1={sig_v2}");
        let verified_v2 = verify_stripe_raw_webhook(
            THIN_V2_BODY,
            &header_v2,
            &[secret],
            SAMPLE_TS,
            DEFAULT_TOLERANCE_SECS,
        )
        .expect("v2.core.event thin event must verify");
        assert!(verified_v2.is_thin_event);
        assert_eq!(verified_v2.event_id, "v2_evt_001");
        assert_eq!(
            verified_v2.event_type,
            "v1.billing.meter.error_report_triggered"
        );
        assert!(!verified_v2.livemode);

        let sig_related = sign(secret, SAMPLE_TS_STR, THIN_RELATED_ONLY_BODY);
        let header_related = format!("t={SAMPLE_TS_STR},v1={sig_related}");
        let verified_related = verify_stripe_raw_webhook(
            THIN_RELATED_ONLY_BODY,
            &header_related,
            &[secret],
            SAMPLE_TS,
            DEFAULT_TOLERANCE_SECS,
        )
        .expect("related_object without data must be flagged as thin event");
        assert!(verified_related.is_thin_event);
        assert_eq!(verified_related.event_id, "evt_thin_002");

        let sig_snap = sign(secret, SAMPLE_TS_STR, SNAPSHOT_BODY);
        let header_snap = format!("t={SAMPLE_TS_STR},v1={sig_snap}");
        let verified_snap = verify_stripe_raw_webhook(
            SNAPSHOT_BODY,
            &header_snap,
            &[secret],
            SAMPLE_TS,
            DEFAULT_TOLERANCE_SECS,
        )
        .expect("snapshot event must verify");
        assert!(!verified_snap.is_thin_event);
    }

    #[test]
    fn test_blank_and_empty_secrets_rejected() {
        let header = format!("t={SAMPLE_TS_STR},v1={}", "a".repeat(64));

        assert_eq!(
            verify_stripe_raw_webhook(
                SNAPSHOT_BODY,
                &header,
                &[],
                SAMPLE_TS,
                DEFAULT_TOLERANCE_SECS,
            )
            .unwrap_err(),
            StripeWebhookError::BadKey
        );
        assert_eq!(
            verify_stripe_raw_webhook(
                SNAPSHOT_BODY,
                &header,
                &["", "   \t\n "],
                SAMPLE_TS,
                DEFAULT_TOLERANCE_SECS,
            )
            .unwrap_err(),
            StripeWebhookError::BadKey
        );
    }

    #[test]
    fn test_oversized_header_and_v1_candidate_cap() {
        let oversized = format!("t={SAMPLE_TS_STR},v1={}", "a".repeat(MAX_HEADER_BYTES));
        assert!(oversized.len() > MAX_HEADER_BYTES);
        let err = parse_signature_header_order_agnostic(&oversized).unwrap_err();
        assert!(matches!(err, StripeWebhookError::InvalidHeader(_)));

        let mut parts = vec![format!("t={SAMPLE_TS_STR}")];
        for i in 0..20 {
            parts.push(format!("v1={i:064x}"));
        }
        let many_header = parts.join(",");
        let (_, _, candidates) =
            parse_signature_header_order_agnostic(&many_header).expect("cap should parse");
        assert_eq!(candidates.len(), MAX_V1_CANDIDATES);
    }

    #[test]
    fn test_crypto_verification_precedes_json_parsing() {
        let secret = "whsec_crypto_first";
        let malformed_json = b"{not valid json";
        let wrong_sig = "0".repeat(64);
        let bad_header = format!("t={SAMPLE_TS_STR},v1={wrong_sig}");

        let bad_sig_err = verify_stripe_raw_webhook(
            malformed_json,
            &bad_header,
            &[secret],
            SAMPLE_TS,
            DEFAULT_TOLERANCE_SECS,
        )
        .unwrap_err();
        assert_eq!(bad_sig_err, StripeWebhookError::BadSignature);

        let valid_sig = sign(secret, SAMPLE_TS_STR, malformed_json);
        let valid_header = format!("t={SAMPLE_TS_STR},v1={valid_sig}");
        let malformed_err = verify_stripe_raw_webhook(
            malformed_json,
            &valid_header,
            &[secret],
            SAMPLE_TS,
            DEFAULT_TOLERANCE_SECS,
        )
        .unwrap_err();
        assert!(matches!(
            malformed_err,
            StripeWebhookError::MalformedPayload(_)
        ));
    }
}
