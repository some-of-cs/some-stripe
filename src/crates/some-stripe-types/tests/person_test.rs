//! Comprehensive unit and contract tests for Stripe KYC Person domain entities,
//! beneficial ownership verification, FinCEN CDD Rule (31 CFR § 1010.230), EU DAC7,
//! PII redaction (Gate G-Law & Gate G2), and 3-state Nullable parameter mechanics.

use some_stripe_types::customer::CustomerAddress;
use some_stripe_types::nullable::Nullable;
use some_stripe_types::person::{
    CreatePersonParams, KycComplianceError, Person, PersonDob, PersonDocumentFiles,
    PersonDocuments, PersonList, PersonRelationship, PersonVerification, PersonVerificationParams,
    UpdatePersonParams, VerificationDocument, VerificationDocumentParams,
};
use std::collections::HashMap;

/// Helper function to construct a valid base CustomerAddress for testing.
fn make_valid_address(country: &str, line1: &str, city: &str, postal: &str) -> CustomerAddress {
    CustomerAddress {
        city: Some(city.into()),
        country: Some(country.into()),
        line1: Some(line1.into()),
        line2: None,
        postal_code: Some(postal.into()),
        state: Some("CA".into()),
    }
}

/// Helper function to construct a fully compliant beneficial owner Person.
fn make_compliant_beneficial_owner() -> Person {
    Person {
        id: "person_test_cdd_001".into(),
        object: "person".into(),
        account: Some("acct_connect_platform_123".into()),
        first_name: Some("Jane".into()),
        last_name: Some("Doe".into()),
        maiden_name: None,
        email: Some("jane.doe@example.com".into()),
        phone: Some("+15555551234".into()),
        dob: Some(PersonDob::new(1985, 6, 15)),
        address: Some(make_valid_address(
            "US",
            "123 Financial Blvd",
            "San Francisco",
            "94105",
        )),
        registered_address: None,
        relationship: Some(
            PersonRelationship::new()
                .representative(true)
                .owner(true)
                .percent_ownership(35.0)
                .title("Chief Executive Officer"),
        ),
        verification: Some(PersonVerification {
            status: "verified".into(),
            details: None,
            details_code: None,
            document: Some(VerificationDocument {
                front: Some("file_front_123".into()),
                back: Some("file_back_123".into()),
                details: None,
                details_code: None,
            }),
            additional_document: None,
        }),
        documents: None,
        id_number_provided: Some(true),
        id_number_secondary_provided: Some(false),
        ssn_last_4_provided: Some(true),
        id_number: Some("123-45-6789".into()),
        id_number_secondary: None,
        ssn_last_4: Some("6789".into()),
        nationality: Some("US".into()),
        political_exposure: Some("none".into()),
        requirements: None,
        future_requirements: None,
        metadata: None,
        created: 1728345600,
    }
}

#[test]
fn test_person_deserialization_full_fixture() {
    let json = r#"{
        "id": "person_1N4m2L2eZvKYlo2C01234567",
        "object": "person",
        "account": "acct_123456789",
        "first_name": "Alexander",
        "last_name": "Hamilton",
        "maiden_name": null,
        "email": "alexander.hamilton@treasury.gov",
        "phone": "+12025550100",
        "dob": {
            "day": 11,
            "month": 1,
            "year": 1757
        },
        "address": {
            "city": "New York",
            "country": "US",
            "line1": "55 Wall Street",
            "line2": "Suite 400",
            "postal_code": "10005",
            "state": "NY"
        },
        "relationship": {
            "representative": true,
            "owner": true,
            "executive": true,
            "director": true,
            "percent_ownership": 100.0,
            "title": "Secretary of the Treasury"
        },
        "verification": {
            "status": "verified",
            "document": {
                "front": "file_front_treasury_001",
                "back": "file_back_treasury_001"
            }
        },
        "id_number_provided": true,
        "ssn_last_4_provided": true,
        "nationality": "US",
        "political_exposure": "existing",
        "created": 1728345600
    }"#;

    let person: Person = serde_json::from_str(json).expect("Failed to deserialize Person");
    assert_eq!(person.id, "person_1N4m2L2eZvKYlo2C01234567");
    assert_eq!(person.object, "person");
    assert_eq!(person.account.as_deref(), Some("acct_123456789"));
    assert_eq!(person.first_name.as_deref(), Some("Alexander"));
    assert_eq!(person.last_name.as_deref(), Some("Hamilton"));
    assert_eq!(person.dob.unwrap().day, Some(11));
    assert_eq!(person.dob.unwrap().month, Some(1));
    assert_eq!(person.dob.unwrap().year, Some(1757));

    let addr = person.address.as_ref().expect("Address missing");
    assert_eq!(addr.city.as_deref(), Some("New York"));
    assert_eq!(addr.postal_code.as_deref(), Some("10005"));

    let rel = person.relationship.as_ref().expect("Relationship missing");
    assert!(rel.is_representative());
    assert!(rel.is_owner());
    assert!(rel.is_executive());
    assert!(rel.is_director());
    assert_eq!(rel.percent_ownership, Some(100.0));
    assert!(rel.is_beneficial_owner());
    assert!(rel.meets_fincen_cdd_ownership_threshold());
    assert!(rel.is_control_person());

    let ver = person.verification.as_ref().expect("Verification missing");
    assert!(ver.is_verified());
    assert!(!ver.is_pending());
    assert_eq!(
        ver.document.as_ref().unwrap().front.as_deref(),
        Some("file_front_treasury_001")
    );
}

#[test]
fn test_person_serialization_roundtrip() {
    let person = make_compliant_beneficial_owner();
    let json = serde_json::to_string(&person).expect("Failed to serialize person");
    let deserialized: Person = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(person, deserialized);
}

// =========================================================================
// FinCEN CDD Rule (31 CFR § 1010.230) & Gate G-Law Tests
// =========================================================================

#[test]
fn test_fincen_cdd_beneficial_owner_compliance_success() {
    let person = make_compliant_beneficial_owner();
    assert!(person.validate_fincen_cdd_compliance().is_ok());
}

#[test]
fn test_fincen_cdd_missing_first_name_fail() {
    let mut person = make_compliant_beneficial_owner();
    person.first_name = None;
    let err = person.validate_fincen_cdd_compliance().unwrap_err();
    assert_eq!(
        err,
        KycComplianceError::MissingField {
            field: "first_name",
            reason: "FinCEN CDD requires full legal first name for beneficial owners and control persons",
        }
    );
}

#[test]
fn test_fincen_cdd_missing_last_name_fail() {
    let mut person = make_compliant_beneficial_owner();
    person.last_name = Some("   ".into());
    let err = person.validate_fincen_cdd_compliance().unwrap_err();
    assert_eq!(
        err,
        KycComplianceError::MissingField {
            field: "last_name",
            reason: "FinCEN CDD requires full legal last name for beneficial owners and control persons",
        }
    );
}

#[test]
fn test_fincen_cdd_missing_dob_fail() {
    let mut person = make_compliant_beneficial_owner();
    person.dob = None;
    let err = person.validate_fincen_cdd_compliance().unwrap_err();
    assert_eq!(
        err,
        KycComplianceError::MissingField {
            field: "dob",
            reason: "FinCEN CDD requires date of birth for beneficial owners and control persons",
        }
    );
}

#[test]
fn test_fincen_cdd_invalid_dob_fail() {
    let mut person = make_compliant_beneficial_owner();
    person.dob = Some(PersonDob {
        year: Some(1990),
        month: Some(14), // invalid month
        day: Some(10),
    });
    let err = person.validate_fincen_cdd_compliance().unwrap_err();
    assert_eq!(
        err,
        KycComplianceError::InvalidDateOfBirth {
            reason: "FinCEN CDD requires a valid date of birth (day, month, year)",
        }
    );
}

#[test]
fn test_fincen_cdd_missing_address_fail() {
    let mut person = make_compliant_beneficial_owner();
    person.address = None;
    let err = person.validate_fincen_cdd_compliance().unwrap_err();
    assert_eq!(
        err,
        KycComplianceError::MissingField {
            field: "address",
            reason: "FinCEN CDD requires physical residential address for beneficial owners and control persons",
        }
    );
}

#[test]
fn test_fincen_cdd_missing_address_line1_fail() {
    let mut person = make_compliant_beneficial_owner();
    person.address = Some(CustomerAddress {
        country: Some("US".into()),
        line1: None,
        ..Default::default()
    });
    let err = person.validate_fincen_cdd_compliance().unwrap_err();
    assert_eq!(
        err,
        KycComplianceError::MissingField {
            field: "address.line1",
            reason: "FinCEN CDD requires street address in residential address",
        }
    );
}

#[test]
fn test_fincen_cdd_missing_identification_fail() {
    let mut person = make_compliant_beneficial_owner();
    person.id_number_provided = Some(false);
    person.ssn_last_4_provided = Some(false);
    person.id_number = None;
    person.ssn_last_4 = None;
    person.verification = None;

    let err = person.validate_fincen_cdd_compliance().unwrap_err();
    assert_eq!(
        err,
        KycComplianceError::MissingIdentification {
            reason: "FinCEN CDD requires government identification number (SSN, passport, or national ID)",
        }
    );
}

#[test]
fn test_fincen_cdd_invalid_ownership_percentage_bounds() {
    let mut person = make_compliant_beneficial_owner();
    person.relationship = Some(PersonRelationship {
        owner: Some(true),
        percent_ownership: Some(150.0), // > 100%
        ..Default::default()
    });
    let err = person.validate_fincen_cdd_compliance().unwrap_err();
    assert!(matches!(
        err,
        KycComplianceError::InvalidOwnershipPercentage { .. }
    ));
}

#[test]
fn test_fincen_cdd_control_prong_mandate() {
    // Person with 0% ownership but designated as executive (Control Prong)
    let mut person = make_compliant_beneficial_owner();
    person.relationship = Some(PersonRelationship {
        representative: Some(false),
        owner: Some(false),
        executive: Some(true),
        director: Some(false),
        percent_ownership: Some(0.0),
        title: Some("Chief Technology Officer".into()),
    });

    // Complies because all identity information is provided
    assert!(person.validate_fincen_cdd_compliance().is_ok());

    // Fails if identity number is stripped
    person.id_number_provided = Some(false);
    person.ssn_last_4_provided = Some(false);
    person.id_number = None;
    person.ssn_last_4 = None;
    person.verification = None;
    assert!(person.validate_fincen_cdd_compliance().is_err());
}

#[test]
fn test_fincen_cdd_non_beneficial_owner_exempt() {
    // An individual with 5% ownership who is not an owner/control person is exempt
    let person = Person {
        relationship: Some(PersonRelationship {
            representative: Some(false),
            owner: Some(false),
            executive: Some(false),
            director: Some(false),
            percent_ownership: Some(5.0),
            title: Some("Minority Shareholder".into()),
        }),
        ..Default::default()
    };

    assert!(person.validate_fincen_cdd_compliance().is_ok());
}

// =========================================================================
// EU DAC7 (Directive 2021/514) / AMLD5 Compliance Tests
// =========================================================================

#[test]
fn test_dac7_compliance_success() {
    let person = Person {
        first_name: Some("Marie".into()),
        last_name: Some("Curie".into()),
        dob: Some(PersonDob::new(1975, 11, 7)),
        address: Some(make_valid_address("FR", "16 Rue Pierre", "Paris", "75005")),
        id_number_provided: Some(true),
        ..Default::default()
    };

    assert!(person.validate_dac7_compliance().is_ok());
}

#[test]
fn test_dac7_missing_address_fail() {
    let person = Person {
        first_name: Some("Marie".into()),
        last_name: Some("Curie".into()),
        dob: Some(PersonDob::new(1975, 11, 7)),
        id_number_provided: Some(true),
        ..Default::default()
    };

    let err = person.validate_dac7_compliance().unwrap_err();
    assert_eq!(
        err,
        KycComplianceError::MissingField {
            field: "address",
            reason: "EU DAC7 requires primary address for reportable sellers",
        }
    );
}

#[test]
fn test_dac7_missing_tin_identification_fail() {
    let person = Person {
        first_name: Some("Marie".into()),
        last_name: Some("Curie".into()),
        dob: Some(PersonDob::new(1975, 11, 7)),
        address: Some(make_valid_address("FR", "16 Rue Pierre", "Paris", "75005")),
        ..Default::default()
    };

    let err = person.validate_dac7_compliance().unwrap_err();
    assert_eq!(
        err,
        KycComplianceError::MissingIdentification {
            reason: "EU DAC7 requires Tax Identification Number (TIN) or national identification",
        }
    );
}

// =========================================================================
// PII Redaction & Secret Masking (Gate G-Law & Gate G2)
// =========================================================================

#[test]
fn test_pii_redaction_person_debug() {
    let person = make_compliant_beneficial_owner();
    let debug_repr = format!("{person:?}");

    // Assert that sensitive ID numbers are masked
    assert!(
        debug_repr.contains("[REDACTED]"),
        "Person Debug output must contain [REDACTED]"
    );
    assert!(
        !debug_repr.contains("123-45-6789"),
        "Raw government ID number leaked into Person Debug!"
    );
    assert!(
        !debug_repr.contains("6789"),
        "Raw SSN last 4 leaked into Person Debug!"
    );

    // Non-sensitive fields should still be present
    assert!(debug_repr.contains("Jane"));
    assert!(debug_repr.contains("Doe"));
    assert!(debug_repr.contains("San Francisco"));
}

#[test]
fn test_pii_redaction_create_params_debug() {
    let params = CreatePersonParams::new()
        .first_name("Arthur")
        .last_name("Dent")
        .id_number("999-88-7777")
        .id_number_secondary("SEC-123456")
        .ssn_last_4("7777");

    let debug_repr = format!("{params:?}");
    assert!(
        debug_repr.contains("[REDACTED]"),
        "CreatePersonParams Debug must contain [REDACTED]"
    );
    assert!(
        !debug_repr.contains("999-88-7777"),
        "Raw ID leaked in CreatePersonParams Debug!"
    );
    assert!(
        !debug_repr.contains("SEC-123456"),
        "Raw secondary ID leaked in CreatePersonParams Debug!"
    );
    assert!(
        !debug_repr.contains("7777"),
        "Raw SSN last 4 leaked in CreatePersonParams Debug!"
    );
    assert!(debug_repr.contains("Arthur"));
}

#[test]
fn test_pii_redaction_update_params_debug() {
    let params = UpdatePersonParams::new()
        .id_number("444-33-2222")
        .id_number_secondary("SEC-8888")
        .ssn_last_4("2222");

    let debug_repr = format!("{params:?}");
    assert!(
        debug_repr.contains("[REDACTED]"),
        "UpdatePersonParams Debug must contain [REDACTED]"
    );
    assert!(
        !debug_repr.contains("444-33-2222"),
        "Raw ID leaked in UpdatePersonParams Debug!"
    );
    assert!(
        !debug_repr.contains("SEC-8888"),
        "Raw secondary ID leaked in UpdatePersonParams Debug!"
    );
    assert!(
        !debug_repr.contains("2222"),
        "Raw SSN last 4 leaked in UpdatePersonParams Debug!"
    );
}

// =========================================================================
// CreatePersonParams Builder & Validation Tests
// =========================================================================

#[test]
fn test_create_person_params_builder() {
    let mut meta = HashMap::new();
    meta.insert("internal_uid".into(), "user_99".into());

    let params = CreatePersonParams::new()
        .first_name("Ada")
        .last_name("Lovelace")
        .maiden_name("Byron")
        .email("ada@analyticalengine.org")
        .phone("+442071234567")
        .dob(PersonDob::new(1985, 12, 10))
        .address(make_valid_address(
            "GB",
            "10 St James's Square",
            "London",
            "SW1Y 4LE",
        ))
        .relationship(
            PersonRelationship::new()
                .representative(true)
                .title("Inventor"),
        )
        .verification(PersonVerificationParams {
            document: Some(VerificationDocumentParams {
                front: Some("file_front_doc".into()),
                back: Some("file_back_doc".into()),
            }),
            additional_document: None,
        })
        .nationality("GB")
        .political_exposure("none")
        .metadata(meta)
        .idempotency_key("req_idem_001");

    assert!(params.validate().is_ok());
    assert_eq!(params.first_name.as_deref(), Some("Ada"));
    assert_eq!(params.last_name.as_deref(), Some("Lovelace"));
    assert_eq!(params.maiden_name.as_deref(), Some("Byron"));
    assert_eq!(params.nationality.as_deref(), Some("GB"));
    assert_eq!(params.idempotency_key.as_deref(), Some("req_idem_001"));

    // Check JSON serialization omits idempotency_key
    let json = serde_json::to_string(&params).expect("Failed to serialize CreatePersonParams");
    assert!(!json.contains("idempotency_key"));
    assert!(json.contains("Ada"));
    assert!(json.contains("Lovelace"));
}

#[test]
fn test_create_person_params_validation_failure() {
    let params = CreatePersonParams::new().relationship(
        PersonRelationship::new().percent_ownership(105.0), // out of bounds
    );
    let err = params.validate().unwrap_err();
    assert!(matches!(
        err,
        KycComplianceError::InvalidOwnershipPercentage { .. }
    ));

    let params_invalid_dob = CreatePersonParams::new().dob(PersonDob {
        year: Some(2000),
        month: Some(15), // invalid month
        day: Some(1),
    });
    let err_dob = params_invalid_dob.validate().unwrap_err();
    assert!(matches!(
        err_dob,
        KycComplianceError::InvalidDateOfBirth { .. }
    ));
}

// =========================================================================
// UpdatePersonParams 3-State Nullable Mechanics Tests
// =========================================================================

#[test]
fn test_update_person_params_nullable_semantics() {
    // 1. Omit state: empty params serialize to empty object
    let empty_params = UpdatePersonParams::default();
    assert_eq!(empty_params.first_name, Nullable::Omit);
    let json_empty = serde_json::to_string(&empty_params).expect("Serialization failed");
    assert_eq!(json_empty, "{}");

    // 2. Value state: sets concrete values
    let value_params = UpdatePersonParams::new()
        .first_name("Grace")
        .last_name("Hopper")
        .phone("+15551239876");
    assert_eq!(value_params.first_name, Nullable::Value("Grace".into()));
    let json_value = serde_json::to_string(&value_params).expect("Serialization failed");
    assert!(json_value.contains(r#""first_name":"Grace""#));
    assert!(json_value.contains(r#""last_name":"Hopper""#));
    assert!(json_value.contains(r#""phone":"+15551239876""#));
    assert!(!json_value.contains("email")); // omitted

    // 3. Null state: clears fields on remote server
    let null_params = UpdatePersonParams::new()
        .clear_first_name()
        .clear_phone()
        .clear_email()
        .clear_maiden_name();
    assert_eq!(null_params.first_name, Nullable::Null);
    let json_null = serde_json::to_string(&null_params).expect("Serialization failed");
    assert!(json_null.contains(r#""first_name":null"#));
    assert!(json_null.contains(r#""phone":null"#));
    assert!(json_null.contains(r#""email":null"#));
    assert!(json_null.contains(r#""maiden_name":null"#));
}

#[test]
fn test_update_person_params_validation() {
    let params = UpdatePersonParams::new().relationship(
        PersonRelationship::new().percent_ownership(-10.0), // out of bounds
    );
    assert!(params.validate().is_err());
}

// =========================================================================
// Verification & Documents Domain Tests
// =========================================================================

#[test]
fn test_person_verification_states() {
    let unverified = PersonVerification::default();
    assert!(unverified.is_unverified());
    assert!(!unverified.is_verified());
    assert!(!unverified.is_pending());

    let pending = PersonVerification {
        status: "pending".into(),
        ..Default::default()
    };
    assert!(pending.is_pending());
    assert!(!pending.is_verified());

    let verified = PersonVerification {
        status: "verified".into(),
        ..Default::default()
    };
    assert!(verified.is_verified());
    assert!(!verified.is_pending());
}

#[test]
fn test_person_documents_structure() {
    let docs = PersonDocuments {
        company_authorization: Some(PersonDocumentFiles {
            files: vec!["file_auth_001".into(), "file_auth_002".into()],
        }),
        passport: Some(PersonDocumentFiles {
            files: vec!["file_pass_001".into()],
        }),
        visa: None,
    };

    let json = serde_json::to_string(&docs).expect("Failed to serialize documents");
    assert!(json.contains("file_auth_001"));
    assert!(json.contains("file_pass_001"));
    assert!(!json.contains("visa"));
}

#[test]
fn test_person_list_deserialization() {
    let json = r#"{
        "object": "list",
        "data": [
            {
                "id": "person_001",
                "object": "person",
                "first_name": "Alan",
                "last_name": "Turing",
                "created": 1728345600
            },
            {
                "id": "person_002",
                "object": "person",
                "first_name": "Claude",
                "last_name": "Shannon",
                "created": 1728345700
            }
        ],
        "has_more": false,
        "url": "/v1/accounts/acct_123/persons"
    }"#;

    let list: PersonList = serde_json::from_str(json).expect("Failed to deserialize PersonList");
    assert_eq!(list.object, "list");
    assert_eq!(list.data.len(), 2);
    assert_eq!(list.data[0].id, "person_001");
    assert_eq!(list.data[0].first_name.as_deref(), Some("Alan"));
    assert_eq!(list.data[1].id, "person_002");
    assert_eq!(list.data[1].first_name.as_deref(), Some("Claude"));
    assert!(!list.has_more);
    assert_eq!(list.url, "/v1/accounts/acct_123/persons");
}

#[test]
fn test_person_dob_validity() {
    let valid_dob = PersonDob::new(1995, 2, 28);
    assert!(valid_dob.is_valid());

    let invalid_month = PersonDob::new(1995, 13, 28);
    assert!(!invalid_month.is_valid());

    let invalid_day = PersonDob::new(1995, 2, 32);
    assert!(!invalid_day.is_valid());

    let invalid_year = PersonDob::new(1850, 2, 28);
    assert!(!invalid_year.is_valid());

    let partial_dob = PersonDob {
        year: Some(1995),
        month: None,
        day: Some(15),
    };
    assert!(!partial_dob.is_valid());
}
