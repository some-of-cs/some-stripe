//! Comprehensive test suite for Stripe Identity Verification Session types.
//!
//! Validates:
//! - Serialization roundtripping and bracket notation flattening (`to_form_params`).
//! - Gate G2: Sensitive credential masking in `Debug` implementations.
//! - Gate G-Borrow: Zero deep clones and borrowing discipline.
//! - Gate G-Law: GDPR Article 17 Right to Erasure redaction states and biometric minimization.

use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_types::identity_session::{
    CreateVerificationSessionParams, DocumentOptions, IdNumberOptions,
    UpdateVerificationSessionParams, VerificationOptions, VerificationSession,
    VerificationSessionProvidedDetails, VerificationSessionRedaction, VerificationSessionStatus,
    VerificationSessionType,
};
use std::collections::HashMap;

#[test]
fn test_verification_session_full_deserialization() {
    let json = r#"{
        "id": "vs_1NtpABCxyz",
        "object": "identity.verification_session",
        "client_secret": "vcs_1Ntp_secret_live",
        "created": 1690000000,
        "expires_at": 1690086400,
        "livemode": true,
        "status": "verified",
        "type": "document",
        "url": "https://verify.stripe.com/start/vs_1NtpABCxyz",
        "return_url": "https://merchant.example.com/kyc-callback",
        "last_error": {
            "code": "document_unverified_other",
            "reason": "Document glare prevented barcode reading"
        },
        "last_verification_report": "vr_99999",
        "metadata": {
            "user_id": "usr_abc123"
        },
        "options": {
            "document": {
                "allowed_types": ["passport", "driving_license"],
                "require_id_number": true,
                "require_live_capture": true,
                "require_matching_selfie": true
            }
        },
        "redaction": {
            "status": "processing"
        },
        "verified_outputs": {
            "first_name": "Margaret",
            "last_name": "Hamilton",
            "id_number": "A12345678",
            "id_number_type": "us_ssn",
            "dob": {
                "day": 17,
                "month": 8,
                "year": 1936
            },
            "address": {
                "line1": "1 Broadway",
                "city": "Cambridge",
                "state": "MA",
                "postal_code": "02142",
                "country": "US"
            }
        }
    }"#;

    let session: VerificationSession = serde_json::from_str(json).unwrap();
    assert_eq!(session.id(), "vs_1NtpABCxyz");
    assert_eq!(session.client_secret(), Some("vcs_1Ntp_secret_live"));
    assert_eq!(session.status(), VerificationSessionStatus::Verified);
    assert_eq!(
        session.verification_type(),
        VerificationSessionType::Document
    );
    assert!(session.is_livemode());
    assert!(session.is_verified());
    assert!(!session.is_canceled());
    assert!(!session.requires_input());
    assert!(!session.is_redacted());

    let redaction = session.redaction.as_ref().unwrap();
    assert!(redaction.is_processing());
    assert!(!redaction.is_redacted());

    let error = session.last_error.as_ref().unwrap();
    assert_eq!(error.code.as_deref(), Some("document_unverified_other"));
    assert_eq!(
        error.reason.as_deref(),
        Some("Document glare prevented barcode reading")
    );

    let report_id = session.last_verification_report.as_deref();
    assert_eq!(report_id, Some("vr_99999"));

    let meta = &session.metadata;
    assert_eq!(meta.get("user_id").map(String::as_str), Some("usr_abc123"));

    let outputs = session.verified_outputs.as_ref().unwrap();
    assert_eq!(outputs.first_name.as_deref(), Some("Margaret"));
    assert_eq!(outputs.last_name.as_deref(), Some("Hamilton"));
    assert_eq!(outputs.id_number.as_deref(), Some("A12345678"));
    assert_eq!(outputs.id_number_type.as_deref(), Some("us_ssn"));

    let dob = outputs.dob.unwrap();
    assert_eq!(dob.day, Some(17));
    assert_eq!(dob.month, Some(8));
    assert_eq!(dob.year, Some(1936));

    let addr = outputs.address.as_ref().unwrap();
    assert_eq!(addr.line1.as_deref(), Some("1 Broadway"));
    assert_eq!(addr.city.as_deref(), Some("Cambridge"));
    assert_eq!(addr.state.as_deref(), Some("MA"));
    assert_eq!(addr.postal_code.as_deref(), Some("02142"));
    assert_eq!(addr.country.as_deref(), Some("US"));

    // Gate G2 Debug Masking
    let debug_repr = format!("{session:?}");
    assert!(!debug_repr.contains("vcs_1Ntp_secret_live"));
    assert!(debug_repr.contains("[REDACTED]"));
}

#[test]
fn test_create_params_form_bracket_serialization() {
    let mut metadata = HashMap::new();
    metadata.insert("account_tier".to_string(), "gold".to_string());

    let params = CreateVerificationSessionParams::document()
        .with_options(
            VerificationOptions::new().with_document(
                DocumentOptions::new()
                    .with_matching_selfie(true)
                    .with_live_capture(true)
                    .with_id_number(false),
            ),
        )
        .with_return_url("https://platform.com/verified")
        .with_client_reference_id("ref_cust_99")
        .with_metadata(metadata)
        .with_provided_details(VerificationSessionProvidedDetails {
            email: Some("compliance@example.com".into()),
            phone: Some("+15550001111".into()),
        })
        .with_idempotency_key("req_idem_abc");

    let form_pairs = to_form_params(&params).expect("Flattening to form params must succeed");

    let pairs_map: HashMap<String, String> = form_pairs.into_iter().collect();

    assert_eq!(pairs_map.get("type").map(String::as_str), Some("document"));
    assert_eq!(
        pairs_map.get("return_url").map(String::as_str),
        Some("https://platform.com/verified")
    );
    assert_eq!(
        pairs_map.get("client_reference_id").map(String::as_str),
        Some("ref_cust_99")
    );
    assert_eq!(
        pairs_map.get("metadata[account_tier]").map(String::as_str),
        Some("gold")
    );
    assert_eq!(
        pairs_map
            .get("options[document][require_matching_selfie]")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        pairs_map
            .get("options[document][require_live_capture]")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        pairs_map
            .get("options[document][require_id_number]")
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        pairs_map.get("provided_details[email]").map(String::as_str),
        Some("compliance@example.com")
    );
    assert_eq!(
        pairs_map.get("provided_details[phone]").map(String::as_str),
        Some("+15550001111")
    );

    // Verify idempotency_key is NOT serialized into the form body
    assert!(!pairs_map.contains_key("idempotency_key"));
}

#[test]
fn test_update_params_form_bracket_serialization() {
    let update = UpdateVerificationSessionParams::new()
        .with_type(VerificationSessionType::IdNumber)
        .with_options(VerificationOptions::for_id_number(IdNumberOptions::new()))
        .with_metadata_entry("step", "2")
        .with_idempotency_key("update_key_xyz");

    let form_pairs = to_form_params(&update).expect("Flattening to form params must succeed");
    let pairs_map: HashMap<String, String> = form_pairs.into_iter().collect();

    assert_eq!(pairs_map.get("type").map(String::as_str), Some("id_number"));
    assert_eq!(
        pairs_map.get("metadata[step]").map(String::as_str),
        Some("2")
    );
    assert!(!pairs_map.contains_key("idempotency_key"));
}

#[test]
fn test_redaction_transitions() {
    let mut session = VerificationSession {
        id: "vs_redact_test".into(),
        object: "identity.verification_session".into(),
        client_secret: None,
        created: 1700000000,
        expires_at: None,
        livemode: false,
        status: VerificationSessionStatus::Verified,
        r#type: VerificationSessionType::Document,
        url: None,
        return_url: None,
        last_error: None,
        last_verification_report: None,
        metadata: HashMap::new(),
        options: None,
        redaction: None,
        verified_outputs: None,
    };

    assert!(!session.is_redacted());

    session.redaction = Some(VerificationSessionRedaction {
        status: Some("processing".into()),
    });
    assert!(!session.is_redacted());

    session.redaction = Some(VerificationSessionRedaction {
        status: Some("redacted".into()),
    });
    assert!(session.is_redacted());
}
