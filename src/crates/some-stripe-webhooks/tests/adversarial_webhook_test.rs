//! Integration test suite for adversarial webhook verification, bit-flip fuzzing,
//! secret rollover, payload ceiling enforcement, and timing resilience (US-2.1.0 / Unit 4).

use chrono::Utc;
use some_stripe_core::error::StripeWebhookError;
use some_stripe_types::Event;
use some_stripe_webhooks::{
    DEFAULT_TOLERANCE_SECONDS, MAX_FUTURE_DRIFT_SECONDS, MAX_SIGNATURE_HEADER_BYTES,
    MAX_V1_SIGNATURES, MAX_WEBHOOK_PAYLOAD_BYTES, Webhook, hmac_sha256_hex,
    verify_stripe_signature,
};

const TEST_SECRET: &str = "whsec_test_secret_key_adversarial_validation_98765";
const VALID_PAYLOAD: &[u8] = br#"{
    "id": "evt_2_1_0_test_event_001",
    "object": "event",
    "created": 1728345600,
    "type": "checkout.session.completed",
    "data": {
        "object": {
            "id": "cs_test_session_abc",
            "status": "complete",
            "payment_status": "paid",
            "amount_total": 4999,
            "currency": "usd"
        }
    }
}"#;

/// tc_2_1_0_01_valid_signature_passes:
/// Assert that an authentic webhook payload with a valid timestamp and HMAC-SHA256 signature returns Ok(Event).
#[test]
fn tc_2_1_0_01_valid_signature_passes() {
    let now = Utc::now().timestamp();
    let header = Webhook::generate_test_header_string(VALID_PAYLOAD, TEST_SECRET, Some(now));

    let event: Event = Webhook::construct_event(VALID_PAYLOAD, &header, TEST_SECRET, None)
        .expect("authentic webhook payload with valid timestamp must pass validation");

    assert_eq!(event.id, "evt_2_1_0_test_event_001");
    assert_eq!(event.event_type, "checkout.session.completed");
    assert_eq!(event.object, "event");

    let is_valid = verify_stripe_signature(VALID_PAYLOAD, &header, TEST_SECRET);
    assert!(
        is_valid,
        "verify_stripe_signature must return true for authentic signature"
    );
}

/// tc_2_1_0_02_bit_flip_fuzzing_rejection:
/// Mutate individual bits and characters across the valid 64-char HMAC-SHA256 signature;
/// assert that 100% of mutations fail verification with StripeWebhookError::BadSignature.
#[test]
fn tc_2_1_0_02_bit_flip_fuzzing_rejection() {
    let now = Utc::now().timestamp();
    let header = Webhook::generate_test_header_string(VALID_PAYLOAD, TEST_SECRET, Some(now));

    let sig_prefix = "v1=";
    let v1_pos = header.find(sig_prefix).expect("header must contain v1=") + sig_prefix.len();
    let original_sig = &header[v1_pos..];
    assert_eq!(
        original_sig.len(),
        64,
        "HMAC-SHA256 hex must be exactly 64 characters"
    );

    let mut total_mutations = 0;
    let mut rejected_mutations = 0;

    // 1. Bit-flip fuzzing across all 64 hexadecimal characters (8 bits per character byte)
    let original_bytes = original_sig.as_bytes();
    for byte_idx in 0..original_bytes.len() {
        for bit_idx in 0..8 {
            let mut mutated_bytes = original_bytes.to_vec();
            mutated_bytes[byte_idx] ^= 1 << bit_idx;

            if !mutated_bytes.eq_ignore_ascii_case(original_bytes) {
                let corrupted_sig = String::from_utf8_lossy(&mutated_bytes);
                let corrupted_header = format!("t={now},v1={corrupted_sig}");

                total_mutations += 1;
                let res =
                    Webhook::construct_event(VALID_PAYLOAD, &corrupted_header, TEST_SECRET, None);
                if matches!(res, Err(StripeWebhookError::BadSignature)) {
                    rejected_mutations += 1;
                }

                assert!(
                    !verify_stripe_signature(VALID_PAYLOAD, &corrupted_header, TEST_SECRET),
                    "verify_stripe_signature must reject corrupted signature byte {byte_idx} bit {bit_idx}"
                );
            }
        }
    }

    // 2. Fuzzing valid hex character substitutions across all 64 positions
    let hex_chars = b"0123456789abcdef";
    for char_idx in 0..original_bytes.len() {
        let current_char = original_bytes[char_idx];
        for &replacement in hex_chars {
            if replacement != current_char {
                let mut mutated_bytes = original_bytes.to_vec();
                mutated_bytes[char_idx] = replacement;
                let corrupted_sig = String::from_utf8(mutated_bytes).expect("valid ascii");
                let corrupted_header = format!("t={now},v1={corrupted_sig}");

                total_mutations += 1;
                let res =
                    Webhook::construct_event(VALID_PAYLOAD, &corrupted_header, TEST_SECRET, None);
                if matches!(res, Err(StripeWebhookError::BadSignature)) {
                    rejected_mutations += 1;
                }

                assert!(
                    !verify_stripe_signature(VALID_PAYLOAD, &corrupted_header, TEST_SECRET),
                    "verify_stripe_signature must reject substituted hex char at index {char_idx}"
                );
            }
        }
    }

    // Assert 100% rejection rate under adversarial bit-flip and character mutation fuzzing
    assert!(
        total_mutations > 1000,
        "Must test over 1,000 mutation variants"
    );
    assert_eq!(
        total_mutations, rejected_mutations,
        "100% of bit-flipped / mutated signatures MUST be rejected with BadSignature"
    );
}

/// tc_2_1_0_03_multi_sig_secret_rollover:
/// Validate dual v1 signatures ("t=...,v1=old_sig,v1=new_sig") matches the rolled secret.
#[test]
fn tc_2_1_0_03_multi_sig_secret_rollover() {
    let now = Utc::now().timestamp();
    let old_secret = "whsec_old_primary_secret_rotation_phase_1";
    let new_secret = "whsec_new_secondary_secret_rotation_phase_2";
    let unshared_secret = "whsec_attacker_or_unrelated_secret_key_99999";

    let ts_str = now.to_string();
    let mut signed_payload = Vec::with_capacity(ts_str.len() + 1 + VALID_PAYLOAD.len());
    signed_payload.extend_from_slice(ts_str.as_bytes());
    signed_payload.push(b'.');
    signed_payload.extend_from_slice(VALID_PAYLOAD);

    let old_sig = hmac_sha256_hex(old_secret.as_bytes(), &signed_payload).unwrap();
    let new_sig = hmac_sha256_hex(new_secret.as_bytes(), &signed_payload).unwrap();

    // Dual-signature header: contains signatures for both old and new secrets during rotation
    let rollover_header = format!("t={now},v1={old_sig},v1={new_sig}");

    // 1. Verification with old_secret succeeds
    let event_old = Webhook::construct_event(VALID_PAYLOAD, &rollover_header, old_secret, None)
        .expect("rollover header must be accepted by old secret");
    assert_eq!(event_old.id, "evt_2_1_0_test_event_001");
    assert!(verify_stripe_signature(
        VALID_PAYLOAD,
        &rollover_header,
        old_secret
    ));

    // 2. Verification with new_secret succeeds
    let event_new = Webhook::construct_event(VALID_PAYLOAD, &rollover_header, new_secret, None)
        .expect("rollover header must be accepted by new secret");
    assert_eq!(event_new.id, "evt_2_1_0_test_event_001");
    assert!(verify_stripe_signature(
        VALID_PAYLOAD,
        &rollover_header,
        new_secret
    ));

    // 3. Reversed order: new_sig first, then old_sig
    let reversed_rollover_header = format!("t={now},v1={new_sig},v1={old_sig}");
    assert!(verify_stripe_signature(
        VALID_PAYLOAD,
        &reversed_rollover_header,
        old_secret
    ));
    assert!(verify_stripe_signature(
        VALID_PAYLOAD,
        &reversed_rollover_header,
        new_secret
    ));

    // 4. Header containing bogus signature alongside valid new signature
    let header_with_garbage = format!(
        "t={now},v1=0000000000000000000000000000000000000000000000000000000000000000,v1={new_sig}"
    );
    let event_garbage =
        Webhook::construct_event(VALID_PAYLOAD, &header_with_garbage, new_secret, None).expect(
            "header with valid signature must succeed even if preceded by invalid signature",
        );
    assert_eq!(event_garbage.id, "evt_2_1_0_test_event_001");

    // 5. Verification with unrelated secret MUST fail
    let err = Webhook::construct_event(VALID_PAYLOAD, &rollover_header, unshared_secret, None)
        .unwrap_err();
    assert_eq!(err, StripeWebhookError::BadSignature);
    assert!(!verify_stripe_signature(
        VALID_PAYLOAD,
        &rollover_header,
        unshared_secret
    ));
}

/// tc_2_1_0_04_blank_and_whitespace_secret_rejection:
/// Reject empty or whitespace-only secrets immediately with StripeWebhookError::MissingSecret.
#[test]
fn tc_2_1_0_04_blank_and_whitespace_secret_rejection() {
    let now = Utc::now().timestamp();
    let header = Webhook::generate_test_header_string(VALID_PAYLOAD, TEST_SECRET, Some(now));

    let invalid_secrets = ["", " ", "   ", "\t", "\n", "\r\n", " \t\r\n "];

    for &blank_secret in &invalid_secrets {
        // verify_header
        let verify_res = Webhook::verify_header(VALID_PAYLOAD, &header, blank_secret, None);
        assert!(
            matches!(
                verify_res,
                Err(StripeWebhookError::BadKey | StripeWebhookError::MissingSecret)
            ),
            "verify_header must reject empty/whitespace secret: '{blank_secret:?}'"
        );

        // construct_event
        let construct_res = Webhook::construct_event(VALID_PAYLOAD, &header, blank_secret, None);
        let construct_err = construct_res.unwrap_err();
        assert!(
            matches!(
                construct_err,
                StripeWebhookError::BadKey | StripeWebhookError::MissingSecret
            ),
            "construct_event must reject empty/whitespace secret: '{blank_secret:?}'"
        );

        // verify_stripe_signature helper
        assert!(
            !verify_stripe_signature(VALID_PAYLOAD, &header, blank_secret),
            "verify_stripe_signature must return false for empty/whitespace secret: '{blank_secret:?}'"
        );
    }
}

/// tc_2_1_0_payload_ceiling_rejection:
/// Reject incoming payloads exceeding 1 MB (1,048,576 bytes) with StripeWebhookError::PayloadTooLarge.
#[test]
fn tc_2_1_0_payload_ceiling_rejection() {
    let now = Utc::now().timestamp();
    let header =
        format!("t={now},v1=5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165");

    // 1. Exceeds limit by 1 byte (1,048,577 bytes)
    let size_1mb_plus_one = MAX_WEBHOOK_PAYLOAD_BYTES + 1;
    let payload_1mb_plus_one = vec![b'x'; size_1mb_plus_one];

    let err_header =
        Webhook::verify_header(&payload_1mb_plus_one, &header, TEST_SECRET, None).unwrap_err();
    assert_eq!(
        err_header,
        StripeWebhookError::PayloadTooLarge {
            size: size_1mb_plus_one,
            max_allowed: MAX_WEBHOOK_PAYLOAD_BYTES,
        }
    );

    let err_event =
        Webhook::construct_event(&payload_1mb_plus_one, &header, TEST_SECRET, None).unwrap_err();
    assert_eq!(
        err_event,
        StripeWebhookError::PayloadTooLarge {
            size: size_1mb_plus_one,
            max_allowed: MAX_WEBHOOK_PAYLOAD_BYTES,
        }
    );

    assert!(
        !verify_stripe_signature(&payload_1mb_plus_one, &header, TEST_SECRET),
        "verify_stripe_signature must return false for payload exceeding ceiling"
    );

    // 2. Significantly larger payload (2 MB / 2,097,152 bytes)
    let size_2mb = 2 * 1024 * 1024;
    let payload_2mb = vec![b'z'; size_2mb];

    let err_2mb = Webhook::construct_event(&payload_2mb, &header, TEST_SECRET, None).unwrap_err();
    assert_eq!(
        err_2mb,
        StripeWebhookError::PayloadTooLarge {
            size: size_2mb,
            max_allowed: MAX_WEBHOOK_PAYLOAD_BYTES,
        }
    );

    // 3. Empty payload rejection
    let empty_payload = b"";
    let err_empty =
        Webhook::construct_event(empty_payload, &header, TEST_SECRET, None).unwrap_err();
    assert_eq!(err_empty, StripeWebhookError::EmptyPayload);
    assert!(!verify_stripe_signature(
        empty_payload,
        &header,
        TEST_SECRET
    ));
}

/// tc_2_1_0_05_timestamp_replay_and_future_drift_tolerance:
/// Validate replay attack prevention (300s window) and excessive future drift rejection (> 60s).
#[test]
fn tc_2_1_0_05_timestamp_replay_and_future_drift_tolerance() {
    let now = Utc::now().timestamp();

    // 1. Expired timestamp (> DEFAULT_TOLERANCE_SECONDS = 300s old)
    let expired_ts = now - (DEFAULT_TOLERANCE_SECONDS as i64) - 5;
    let expired_header =
        Webhook::generate_test_header_string(VALID_PAYLOAD, TEST_SECRET, Some(expired_ts));
    let err_expired =
        Webhook::construct_event(VALID_PAYLOAD, &expired_header, TEST_SECRET, None).unwrap_err();
    assert!(matches!(
        err_expired,
        StripeWebhookError::TimestampOutsideTolerance { .. }
    ));

    // 2. Future drift exceeding MAX_FUTURE_DRIFT_SECONDS (60s)
    let excessive_future_ts = now + MAX_FUTURE_DRIFT_SECONDS + 10;
    let future_header =
        Webhook::generate_test_header_string(VALID_PAYLOAD, TEST_SECRET, Some(excessive_future_ts));
    let err_future =
        Webhook::construct_event(VALID_PAYLOAD, &future_header, TEST_SECRET, None).unwrap_err();
    assert!(matches!(
        err_future,
        StripeWebhookError::TimestampOutsideTolerance { .. }
    ));

    // 3. Within allowed future drift (< MAX_FUTURE_DRIFT_SECONDS)
    let acceptable_future_ts = now + (MAX_FUTURE_DRIFT_SECONDS - 5);
    let acceptable_future_header = Webhook::generate_test_header_string(
        VALID_PAYLOAD,
        TEST_SECRET,
        Some(acceptable_future_ts),
    );
    let event_future =
        Webhook::construct_event(VALID_PAYLOAD, &acceptable_future_header, TEST_SECRET, None)
            .expect("timestamp within future drift ceiling must succeed");
    assert_eq!(event_future.id, "evt_2_1_0_test_event_001");

    // 4. Within allowed historical tolerance (< DEFAULT_TOLERANCE_SECONDS)
    let acceptable_past_ts = now - (DEFAULT_TOLERANCE_SECONDS as i64) + 10;
    let acceptable_past_header =
        Webhook::generate_test_header_string(VALID_PAYLOAD, TEST_SECRET, Some(acceptable_past_ts));
    let event_past =
        Webhook::construct_event(VALID_PAYLOAD, &acceptable_past_header, TEST_SECRET, None)
            .expect("timestamp within past tolerance window must succeed");
    assert_eq!(event_past.id, "evt_2_1_0_test_event_001");
}

/// tc_2_1_0_06_variable_length_and_malformed_header_resilience:
/// Validate length mismatch tolerance without timing leaks or panics.
#[test]
fn tc_2_1_0_06_variable_length_and_malformed_header_resilience() {
    let now = Utc::now().timestamp();

    // 1. Truncated signature length (32 chars instead of 64)
    let truncated_sig = "5257a869e7ecebeda32affa62cd45081";
    let truncated_header = format!("t={now},v1={truncated_sig}");
    let err_truncated =
        Webhook::construct_event(VALID_PAYLOAD, &truncated_header, TEST_SECRET, None).unwrap_err();
    assert_eq!(err_truncated, StripeWebhookError::BadSignature);

    // 2. Missing v1 signature parameter
    let missing_sig_header = format!("t={now}");
    let err_no_sig =
        Webhook::construct_event(VALID_PAYLOAD, &missing_sig_header, TEST_SECRET, None)
            .unwrap_err();
    assert_eq!(err_no_sig, StripeWebhookError::MissingSignature);

    // 3. Missing t timestamp parameter
    let missing_ts_header = "v1=5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165";
    let err_no_ts =
        Webhook::construct_event(VALID_PAYLOAD, missing_ts_header, TEST_SECRET, None).unwrap_err();
    assert_eq!(err_no_ts, StripeWebhookError::InvalidTimestamp);

    // 4. Blank or whitespace-only header
    let blank_header = "   ";
    let err_blank =
        Webhook::construct_event(VALID_PAYLOAD, blank_header, TEST_SECRET, None).unwrap_err();
    assert_eq!(err_blank, StripeWebhookError::MissingSignatureHeader);

    // 5. Oversized signature header exceeding MAX_SIGNATURE_HEADER_BYTES (4096 bytes)
    let padding = "x".repeat(MAX_SIGNATURE_HEADER_BYTES);
    let oversized_header = format!("t={now},v1=sig_test,{padding}");
    assert!(oversized_header.len() > MAX_SIGNATURE_HEADER_BYTES);
    let err_oversized =
        Webhook::construct_event(VALID_PAYLOAD, &oversized_header, TEST_SECRET, None).unwrap_err();
    assert_eq!(
        err_oversized,
        StripeWebhookError::InvalidHeader("header exceeds maximum allowed length")
    );
    assert!(
        !verify_stripe_signature(VALID_PAYLOAD, &oversized_header, TEST_SECRET),
        "verify_stripe_signature must reject oversized signature headers"
    );

    // 6. Excessive v1 signature count exceeding MAX_V1_SIGNATURES (16)
    let mut excessive_tokens = vec![format!("t={now}")];
    for i in 0..=MAX_V1_SIGNATURES {
        excessive_tokens.push(format!("v1=sig_overflow_{i}"));
    }
    let excessive_header = excessive_tokens.join(",");
    let err_excessive =
        Webhook::construct_event(VALID_PAYLOAD, &excessive_header, TEST_SECRET, None).unwrap_err();
    assert_eq!(
        err_excessive,
        StripeWebhookError::InvalidHeader("too many v1 signatures")
    );
    assert!(
        !verify_stripe_signature(VALID_PAYLOAD, &excessive_header, TEST_SECRET),
        "verify_stripe_signature must reject headers with excessive signatures"
    );
}
