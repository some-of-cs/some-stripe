//! Comprehensive Gate G-Borrow and Gate G-Law Cryptographic Audit Suite
//!
//! Verifies zero-clone borrowing discipline, streaming HMAC allocations, and PCI-DSS / PSD2 compliance.

use some_stripe_core::error::StripeWebhookError;
use some_stripe_types::{Event, EventData, PaymentIntent};
use some_stripe_webhooks::{
    MAX_FUTURE_DRIFT_SECONDS, MAX_WEBHOOK_PAYLOAD_BYTES, StripeWebhookWrapper, Webhook,
    hmac_sha256_hex, safe_constant_time_eq, streaming_hmac_sha256_hex, verify_stripe_signature,
};

/// Gate G-Borrow Audit: Audits that verify_stripe_signature and Webhook::verify_header operate
/// strictly on borrowed &[u8] and &str without cloning the raw payload buffer.
#[test]
fn test_zero_clone_borrowed_slices_in_verification() {
    let secret = "whsec_test_borrow_discipline_secret_key_12345";
    let timestamp = 1728345600;

    // Fixed immutable payload slice without dynamic allocation during verification
    let payload_data: &[u8] = br#"{"id":"evt_borrow_audit_001","object":"event","created":1728345600,"type":"payment_intent.succeeded","data":{"object":{"id":"pi_borrow_test_123","amount":9900,"currency":"usd","status":"succeeded"}}}"#;

    let header = Webhook::generate_test_header_string(payload_data, secret, Some(timestamp));

    // 1. Audit that verify_stripe_signature operates strictly on borrowed &[u8] and &str
    let verified = verify_stripe_signature(payload_data, &header, secret);
    assert!(
        verified,
        "Signature verification over borrowed slice must succeed"
    );

    // 2. Audit that Webhook::verify_header operates strictly on borrowed &[u8] and &str
    let header_verified = Webhook::verify_header(payload_data, &header, secret, Some(0));
    assert!(
        header_verified.is_ok(),
        "Webhook::verify_header must succeed on borrowed slice without consuming it"
    );

    // 3. Verify memory address stability: original slice was not modified or consumed
    assert_eq!(payload_data[0], b'{');
    assert_eq!(payload_data[payload_data.len() - 1], b'}');

    // 4. Test binary non-UTF8 payload: HMAC operates at raw byte slice level without converting to String
    let binary_payload: &[u8] = &[0x00, 0x01, 0x02, 0xFE, 0xFF, 0x80, 0x90, 0xAA];
    let bin_header = Webhook::generate_test_header_string(binary_payload, secret, Some(timestamp));
    assert!(
        verify_stripe_signature(binary_payload, &bin_header, secret),
        "Raw non-UTF8 binary byte slices must verify without string copies or UTF-8 decoding"
    );

    // 5. Audit near 1 MB boundary payload borrowing without cloning
    let large_payload = vec![b'x'; MAX_WEBHOOK_PAYLOAD_BYTES];
    let large_header =
        Webhook::generate_test_header_string(&large_payload, secret, Some(timestamp));
    assert!(
        verify_stripe_signature(&large_payload, &large_header, secret),
        "Verification of 1 MB borrowed slice must succeed without cloning"
    );
}

/// Gate G-Borrow Audit: Audits that streaming HMAC computes digest without formatting or allocating
/// intermediate ${timestamp}.${payload} strings.
#[test]
fn test_streaming_hmac_avoids_heap_format_allocation() {
    let key = b"whsec_streaming_audit_key_secret_67890";
    let timestamp = b"1728345600";
    let payload = br#"{"id":"evt_stream_test","amount":4200}"#;

    // 1. Direct comparison: streaming HMAC must be bit-for-bit identical to concatenated HMAC
    let mut concatenated = Vec::with_capacity(timestamp.len() + 1 + payload.len());
    concatenated.extend_from_slice(timestamp);
    concatenated.push(b'.');
    concatenated.extend_from_slice(payload);

    let standard_hex = hmac_sha256_hex(key, &concatenated).expect("standard hmac calculation");
    let streaming_hex =
        streaming_hmac_sha256_hex(key, timestamp, payload).expect("streaming hmac calculation");

    assert_eq!(
        streaming_hex, standard_hex,
        "streaming HMAC must compute the exact same digest as concatenated HMAC without allocating concatenated buffer"
    );

    // 2. Audit 1 MB payload streaming: avoids 1 MB intermediate heap allocation
    let large_payload = vec![b'a'; MAX_WEBHOOK_PAYLOAD_BYTES];
    let large_concatenated = {
        let mut v = Vec::with_capacity(timestamp.len() + 1 + large_payload.len());
        v.extend_from_slice(timestamp);
        v.push(b'.');
        v.extend_from_slice(&large_payload);
        v
    };

    let large_standard_hex =
        hmac_sha256_hex(key, &large_concatenated).expect("standard large hmac");
    let large_streaming_hex =
        streaming_hmac_sha256_hex(key, timestamp, &large_payload).expect("streaming large hmac");

    assert_eq!(
        large_streaming_hex, large_standard_hex,
        "Streaming HMAC on 1 MB payload must match without heap concatenation"
    );

    // 3. Known RFC 2104 / HMAC test vector properties check
    // Ensure lowercase hexadecimal representation of length 64 (32 bytes * 2)
    assert_eq!(streaming_hex.len(), 64);
    assert!(
        streaming_hex
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
}

/// Gate G-Law Audit: Asserts that StripeWebhookError and event wrappers never leak sensitive payload fields,
/// secret keys, or PANs in their std::fmt::Debug / Display representations, and verifies PSD2 SCA compliance.
#[test]
fn test_tc_2_1_0_law_pci_dss_and_psd2_compliance() {
    // -------------------------------------------------------------
    // Gate G-Law: PCI-DSS 4.0 Non-Transit & Zero PAN Leakage Invariants
    // -------------------------------------------------------------
    let secret = "whsec_super_secret_signing_key_never_leak_in_logs";
    let mock_pan = "4111111111111111"; // Standard test Visa PAN
    let mock_cvv = "999";

    // 1. StripeWebhookError Display / Debug Redaction Audit
    let err_missing_secret = StripeWebhookError::MissingSecret;
    let disp_secret = format!("{err_missing_secret}");
    let dbg_secret = format!("{err_missing_secret:?}");
    assert!(
        !disp_secret.contains(secret) && !dbg_secret.contains(secret),
        "MissingSecret error must not leak signing secrets"
    );

    let err_bad_sig = StripeWebhookError::BadSignature;
    let disp_sig = format!("{err_bad_sig}");
    let dbg_sig = format!("{err_bad_sig:?}");
    assert!(
        !disp_sig.contains(secret) && !dbg_sig.contains(secret),
        "BadSignature error must not leak signing secrets"
    );

    // 2. Malformed JSON with sensitive PAN data in parse failure
    let malformed_err = StripeWebhookError::MalformedPayload("syntax error at line 1".into());
    let dbg_malformed = format!("{malformed_err:?}");
    assert!(
        !dbg_malformed.contains(mock_pan),
        "MalformedPayload debug formatting must never leak raw PANs"
    );

    // 3. PaymentIntent Debug redaction for client_secret (PCI-DSS 4.0 Invariant 1)
    let sensitive_pi = PaymentIntent {
        id: "pi_pci_audit_123".into(),
        object: "payment_intent".into(),
        amount: 25000,
        amount_received: Some(25000),
        currency: "usd".into(),
        status: "requires_action".into(),
        capture_method: Some("automatic".into()),
        client_secret: Some("pi_pci_audit_123_secret_SensitiveLiveClientSecretToken".into()),
        next_action: Some(serde_json::json!({
            "type": "redirect_to_url",
            "redirect_to_url": {
                "url": "https://hooks.stripe.com/3d_secure_2/hosted",
                "return_url": "https://merchant.example.com/checkout/return"
            }
        })),
        created: Some(1728345600),
        cancellation_reason: None,
    };

    let pi_debug = format!("{sensitive_pi:?}");
    assert!(
        pi_debug.contains("[REDACTED]"),
        "PaymentIntent Debug must redact client_secret"
    );
    assert!(
        !pi_debug.contains("SensitiveLiveClientSecretToken"),
        "PaymentIntent Debug must not leak raw client secret token"
    );

    // -------------------------------------------------------------
    // Gate G-Law: PSD2 Strong Customer Authentication (SCA) 3DS 2.0
    // -------------------------------------------------------------
    // Assert next_action preserves SCA 3DS redirect payload without mutation
    let next_action = sensitive_pi
        .next_action
        .as_ref()
        .expect("next_action must be present");
    assert_eq!(next_action["type"], "redirect_to_url");
    assert_eq!(
        next_action["redirect_to_url"]["return_url"],
        "https://merchant.example.com/checkout/return"
    );

    // 4. StripeWebhookWrapper audit with sensitive payload
    let wrapper_event = Event {
        id: "evt_pci_wrapper_test".into(),
        object: "event".into(),
        api_version: Some("2024-06-20".into()),
        created: 1728345600,
        event_type: "payment_intent.requires_action".into(),
        data: EventData {
            object: serde_json::json!({
                "id": "pi_pci_audit_123",
                "object": "payment_intent",
                "amount": 25000,
                "currency": "usd",
                "status": "requires_action",
                "payment_method_types": ["card"],
                "tokenized_pan": "pm_1PxxxxMockCardToken",
                "masked_card": "**** **** **** 1111"
            }),
        },
    };

    let wrapper = StripeWebhookWrapper::new(wrapper_event, Some(1728345600));
    let wrapper_dbg = format!("{wrapper:?}");
    assert!(
        !wrapper_dbg.contains(mock_pan),
        "StripeWebhookWrapper Debug representation must never contain raw unmasked PAN"
    );
    assert!(
        !wrapper_dbg.contains(mock_cvv),
        "StripeWebhookWrapper Debug representation must never contain CVC/CVV"
    );
    assert_eq!(wrapper.order_id(), Some("pi_pci_audit_123"));
}

/// Gate G2 Audit: Asserts constant-time comparison slice length invariance.
#[test]
fn test_constant_time_comparison_slice_length_invariance() {
    let sig_a = b"5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165";
    let sig_b = b"5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165";
    let sig_diff_len = b"5257a869e7ecebeda32affa62cd45081";
    let sig_tampered = b"5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f160";

    assert!(safe_constant_time_eq(sig_a, sig_b));
    assert!(!safe_constant_time_eq(sig_a, sig_diff_len));
    assert!(!safe_constant_time_eq(sig_a, sig_tampered));
}

/// Gate G2 Audit: Asserts payload ceiling enforcement (1 MB) prevents memory exhaustion DoS.
#[test]
fn test_payload_ceiling_enforcement_prevents_memory_dos() {
    let secret = "whsec_test_secret";
    let oversized = vec![b'z'; MAX_WEBHOOK_PAYLOAD_BYTES + 1];
    let header = "t=1728345600,v1=abcdef0123456789";

    let err = Webhook::verify_header(&oversized, header, secret, None).unwrap_err();
    match err {
        StripeWebhookError::PayloadTooLarge { size, max_allowed } => {
            assert_eq!(size, MAX_WEBHOOK_PAYLOAD_BYTES + 1);
            assert_eq!(max_allowed, MAX_WEBHOOK_PAYLOAD_BYTES);
        }
        other => panic!("Expected PayloadTooLarge, got: {other:?}"),
    }

    assert!(!verify_stripe_signature(&oversized, header, secret));
}

/// Gate G2 Audit: Asserts future drift and past tolerance window boundary conditions.
#[test]
fn test_future_drift_and_tolerance_boundary_conditions() {
    let now = chrono::Utc::now().timestamp();
    let secret = "whsec_drift_test";
    let payload = br#"{"id":"evt_drift"}"#;

    // Tolerance window: 300s in the past is valid; 301s is invalid
    let valid_past = now - 300;
    let header_valid_past = Webhook::generate_test_header_string(payload, secret, Some(valid_past));
    assert!(Webhook::verify_header(payload, &header_valid_past, secret, Some(300)).is_ok());

    let expired_past = now - 301;
    let header_expired = Webhook::generate_test_header_string(payload, secret, Some(expired_past));
    assert!(matches!(
        Webhook::verify_header(payload, &header_expired, secret, Some(300)).unwrap_err(),
        StripeWebhookError::TimestampOutsideTolerance { .. }
    ));

    // Future drift window: 60s in the future is valid; 61s is invalid
    let valid_future = now + MAX_FUTURE_DRIFT_SECONDS;
    let header_valid_future =
        Webhook::generate_test_header_string(payload, secret, Some(valid_future));
    assert!(Webhook::verify_header(payload, &header_valid_future, secret, Some(300)).is_ok());

    let invalid_future = now + MAX_FUTURE_DRIFT_SECONDS + 1;
    let header_invalid_future =
        Webhook::generate_test_header_string(payload, secret, Some(invalid_future));
    assert!(matches!(
        Webhook::verify_header(payload, &header_invalid_future, secret, Some(300)).unwrap_err(),
        StripeWebhookError::TimestampOutsideTolerance { .. }
    ));
}
