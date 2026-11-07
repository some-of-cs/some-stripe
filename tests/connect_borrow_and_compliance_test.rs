//! Comprehensive Integration Tests for Connect Borrowing Audit, Multi-Tenant Isolation,
//! and FinCEN CDD / EU DAC7 Legal Compliance (Gate G0, G-Borrow, G2, G-Law, G1, G4, G-Docs).

use some_stripe::StripeClient;
use some_stripe_core::client::RequestOptions;
use some_stripe_types::Account;
use some_stripe_types::connect_compliance::{
    DAC7_ACTIVITY_COUNT_THRESHOLD, DAC7_CONSIDERATION_THRESHOLD_CENTS, Dac7ActivitySummary,
    Dac7ComplianceError, Dac7ReportingStatus, Dac7SellerProfile, FinCenBeneficialOwner,
    FinCenCddError, FinCenCddRequirements, MultiTenantContext, MultiTenantError,
    audit_connect_entity_borrow, evaluate_dac7_reporting_status, validate_connect_account_id,
    validate_fincen_cdd,
};

// ============================================================================
// 1. FinCEN Customer Due Diligence (CDD) Compliance Tests (31 CFR § 1010.230)
// ============================================================================

#[test]
fn test_fincen_cdd_valid_company_with_25_pct_owner_and_control() {
    let cdd = FinCenCddRequirements {
        account_id: "acct_1032D82eZvKYlo2C".into(),
        business_type: "company".into(),
        beneficial_owners: vec![
            FinCenBeneficialOwner {
                id: "person_owner_1".into(),
                first_name: "Alice".into(),
                last_name: "Smith".into(),
                date_of_birth: "1985-04-12".into(),
                equity_percentage: 40.0,
                is_control_person: true,
                has_verified_tax_id: true,
                country: "US".into(),
            },
            FinCenBeneficialOwner {
                id: "person_owner_2".into(),
                first_name: "Bob".into(),
                last_name: "Jones".into(),
                date_of_birth: "1990-08-23".into(),
                equity_percentage: 30.0,
                is_control_person: false,
                has_verified_tax_id: true,
                country: "US".into(),
            },
        ],
    };

    assert!(validate_fincen_cdd(&cdd).is_ok());
}

#[test]
fn test_fincen_cdd_missing_control_person() {
    let cdd = FinCenCddRequirements {
        account_id: "acct_1032D82eZvKYlo2C".into(),
        business_type: "company".into(),
        beneficial_owners: vec![FinCenBeneficialOwner {
            id: "person_owner_1".into(),
            first_name: "Alice".into(),
            last_name: "Smith".into(),
            date_of_birth: "1985-04-12".into(),
            equity_percentage: 50.0,
            is_control_person: false, // No control person!
            has_verified_tax_id: true,
            country: "US".into(),
        }],
    };

    let err = validate_fincen_cdd(&cdd).unwrap_err();
    assert_eq!(err, FinCenCddError::MissingControlPerson);
}

#[test]
fn test_fincen_cdd_25_pct_owner_unverified_tax_id_rejected() {
    let cdd = FinCenCddRequirements {
        account_id: "acct_1032D82eZvKYlo2C".into(),
        business_type: "company".into(),
        beneficial_owners: vec![FinCenBeneficialOwner {
            id: "person_unverified_major".into(),
            first_name: "Charlie".into(),
            last_name: "Brown".into(),
            date_of_birth: "1978-01-15".into(),
            equity_percentage: 25.0, // Exactly 25% threshold
            is_control_person: true,
            has_verified_tax_id: false, // Unverified TIN!
            country: "US".into(),
        }],
    };

    let err = validate_fincen_cdd(&cdd).unwrap_err();
    assert_eq!(
        err,
        FinCenCddError::MissingTaxIdVerification {
            owner_id: "person_unverified_major".into(),
            equity: 25.0,
        }
    );
}

#[test]
fn test_fincen_cdd_under_25_pct_owner_unverified_tax_id_permitted() {
    let cdd = FinCenCddRequirements {
        account_id: "acct_1032D82eZvKYlo2C".into(),
        business_type: "company".into(),
        beneficial_owners: vec![
            FinCenBeneficialOwner {
                id: "person_ctrl".into(),
                first_name: "Diana".into(),
                last_name: "Prince".into(),
                date_of_birth: "1982-11-05".into(),
                equity_percentage: 50.0,
                is_control_person: true,
                has_verified_tax_id: true,
                country: "US".into(),
            },
            FinCenBeneficialOwner {
                id: "person_passive_minority".into(),
                first_name: "Eve".into(),
                last_name: "Polastri".into(),
                date_of_birth: "1992-06-30".into(),
                equity_percentage: 15.0, // Below 25% threshold
                is_control_person: false,
                has_verified_tax_id: false, // Allowed for <25% passive owners
                country: "US".into(),
            },
        ],
    };

    // Valid because minority owner < 25% does not trigger the equity ownership verification prong
    assert!(validate_fincen_cdd(&cdd).is_ok());
}

#[test]
fn test_fincen_cdd_negative_equity_rejected() {
    let cdd = FinCenCddRequirements {
        account_id: "acct_1032D82eZvKYlo2C".into(),
        business_type: "company".into(),
        beneficial_owners: vec![FinCenBeneficialOwner {
            id: "person_bad_math".into(),
            first_name: "Frank".into(),
            last_name: "Castle".into(),
            date_of_birth: "1975-02-18".into(),
            equity_percentage: -5.0,
            is_control_person: true,
            has_verified_tax_id: true,
            country: "US".into(),
        }],
    };

    let err = validate_fincen_cdd(&cdd).unwrap_err();
    assert_eq!(
        err,
        FinCenCddError::NegativeEquityPercentage {
            owner_id: "person_bad_math".into(),
            equity: -5.0,
        }
    );
}

#[test]
fn test_fincen_cdd_equity_overflow_rejected() {
    let cdd = FinCenCddRequirements {
        account_id: "acct_1032D82eZvKYlo2C".into(),
        business_type: "company".into(),
        beneficial_owners: vec![
            FinCenBeneficialOwner {
                id: "person_1".into(),
                first_name: "Owner1".into(),
                last_name: "One".into(),
                date_of_birth: "1980-01-01".into(),
                equity_percentage: 60.0,
                is_control_person: true,
                has_verified_tax_id: true,
                country: "US".into(),
            },
            FinCenBeneficialOwner {
                id: "person_2".into(),
                first_name: "Owner2".into(),
                last_name: "Two".into(),
                date_of_birth: "1982-02-02".into(),
                equity_percentage: 45.0, // 60 + 45 = 105% > 100%
                is_control_person: false,
                has_verified_tax_id: true,
                country: "US".into(),
            },
        ],
    };

    let err = validate_fincen_cdd(&cdd).unwrap_err();
    match err {
        FinCenCddError::EquityPercentageOverflow { total_equity } => {
            assert!((total_equity - 105.0).abs() < 1e-4);
        }
        _ => panic!("Expected EquityPercentageOverflow"),
    }
}

#[test]
fn test_fincen_cdd_invalid_date_of_birth_rejected() {
    let cdd = FinCenCddRequirements {
        account_id: "acct_1032D82eZvKYlo2C".into(),
        business_type: "company".into(),
        beneficial_owners: vec![FinCenBeneficialOwner {
            id: "person_bad_dob".into(),
            first_name: "Grace".into(),
            last_name: "Hopper".into(),
            date_of_birth: "1906/12/09".into(), // Bad delimiter (slash instead of dash)
            equity_percentage: 100.0,
            is_control_person: true,
            has_verified_tax_id: true,
            country: "US".into(),
        }],
    };

    let err = validate_fincen_cdd(&cdd).unwrap_err();
    assert_eq!(
        err,
        FinCenCddError::InvalidDateOfBirth {
            owner_id: "person_bad_dob".into(),
            dob: "1906/12/09".into(),
        }
    );
}

#[test]
fn test_fincen_cdd_individual_account_exempt() {
    let cdd = FinCenCddRequirements {
        account_id: "acct_1032D82eZvKYlo2C".into(),
        business_type: "individual".into(),
        beneficial_owners: vec![], // Empty is legal for sole proprietors
    };

    assert!(validate_fincen_cdd(&cdd).is_ok());
}

#[test]
fn test_fincen_cdd_empty_account_id_rejected() {
    let cdd = FinCenCddRequirements {
        account_id: "".into(),
        business_type: "company".into(),
        beneficial_owners: vec![],
    };

    assert_eq!(
        validate_fincen_cdd(&cdd).unwrap_err(),
        FinCenCddError::EmptyAccountId
    );
}

// ============================================================================
// 2. EU DAC7 Platform Tax Reporting Tests (Directive (EU) 2021/514)
// ============================================================================

#[test]
fn test_dac7_exempt_low_volume_seller() {
    let seller = Dac7SellerProfile {
        seller_id: "acct_exempt_seller".into(),
        legal_name: "Small Craftsman".into(),
        tax_identification_number: None, // Missing TIN allowed for low volume
        country: "FR".into(),
        iban: None, // Missing IBAN allowed for low volume
    };

    let activities = vec![Dac7ActivitySummary {
        reporting_year: 2026,
        quarter: 1,
        number_of_activities: 12,
        total_consideration_cents: 120_000, // €1,200 <= €2,000 threshold
        fees_withheld_cents: 10_000,
        taxes_withheld_cents: 0,
    }];

    let status = evaluate_dac7_reporting_status(&seller, &activities).unwrap();
    assert_eq!(
        status,
        Dac7ReportingStatus::ExemptLowVolume {
            total_activities: 12,
            total_consideration_cents: 120_000,
        }
    );
}

#[test]
fn test_dac7_reportable_due_to_activities_threshold() {
    let seller = Dac7SellerProfile {
        seller_id: "acct_high_activity".into(),
        legal_name: "Frequent Seller SAS".into(),
        tax_identification_number: Some("FR12345678901".into()),
        country: "FR".into(),
        iban: Some("FR7630006000011234567890189".into()),
    };

    let activities = vec![Dac7ActivitySummary {
        reporting_year: 2026,
        quarter: 2,
        number_of_activities: DAC7_ACTIVITY_COUNT_THRESHOLD, // Exactly 30 activities
        total_consideration_cents: 50_000,                   // €500 < €2,000 threshold
        fees_withheld_cents: 5_000,
        taxes_withheld_cents: 0,
    }];

    let status = evaluate_dac7_reporting_status(&seller, &activities).unwrap();
    assert_eq!(
        status,
        Dac7ReportingStatus::ReportableSeller {
            total_activities: 30,
            net_consideration_cents: 45_000,
            tax_identifier: "FR12345678901".into(),
            financial_identifier: "FR7630006000011234567890189".into(),
        }
    );
}

#[test]
fn test_dac7_reportable_due_to_consideration_threshold() {
    let seller = Dac7SellerProfile {
        seller_id: "acct_high_value".into(),
        legal_name: "Luxury Vintage SRL".into(),
        tax_identification_number: Some("IT12345678901".into()),
        country: "IT".into(),
        iban: Some("IT60X0542811101000000123456".into()),
    };

    let activities = vec![Dac7ActivitySummary {
        reporting_year: 2026,
        quarter: 3,
        number_of_activities: 2, // Low activity
        total_consideration_cents: DAC7_CONSIDERATION_THRESHOLD_CENTS + 1, // €2,000.01 > €2,000
        fees_withheld_cents: 20_000,
        taxes_withheld_cents: 0,
    }];

    let status = evaluate_dac7_reporting_status(&seller, &activities).unwrap();
    assert_eq!(
        status,
        Dac7ReportingStatus::ReportableSeller {
            total_activities: 2,
            net_consideration_cents: 180_001,
            tax_identifier: "IT12345678901".into(),
            financial_identifier: "IT60X0542811101000000123456".into(),
        }
    );
}

#[test]
fn test_dac7_reportable_missing_tax_identifier_fails() {
    let seller = Dac7SellerProfile {
        seller_id: "acct_missing_tin".into(),
        legal_name: "Missing TIN Co".into(),
        tax_identification_number: None, // Missing!
        country: "ES".into(),
        iban: Some("ES9121000418450200051332".into()),
    };

    let activities = vec![Dac7ActivitySummary {
        reporting_year: 2026,
        quarter: 1,
        number_of_activities: 40,
        total_consideration_cents: 300_000,
        fees_withheld_cents: 30_000,
        taxes_withheld_cents: 0,
    }];

    let err = evaluate_dac7_reporting_status(&seller, &activities).unwrap_err();
    assert_eq!(
        err,
        Dac7ComplianceError::MissingTaxIdentifier {
            seller_id: "acct_missing_tin".into(),
        }
    );
}

#[test]
fn test_dac7_reportable_missing_iban_fails() {
    let seller = Dac7SellerProfile {
        seller_id: "acct_missing_iban".into(),
        legal_name: "Missing IBAN BV".into(),
        tax_identification_number: Some("NL123456789B01".into()),
        country: "NL".into(),
        iban: None, // Missing!
    };

    let activities = vec![Dac7ActivitySummary {
        reporting_year: 2026,
        quarter: 1,
        number_of_activities: 50,
        total_consideration_cents: 400_000,
        fees_withheld_cents: 40_000,
        taxes_withheld_cents: 0,
    }];

    let err = evaluate_dac7_reporting_status(&seller, &activities).unwrap_err();
    assert_eq!(
        err,
        Dac7ComplianceError::MissingFinancialIdentifier {
            seller_id: "acct_missing_iban".into(),
        }
    );
}

#[test]
fn test_dac7_negative_consideration_fails() {
    let seller = Dac7SellerProfile {
        seller_id: "acct_seller".into(),
        legal_name: "Test Seller".into(),
        tax_identification_number: Some("TIN".into()),
        country: "DE".into(),
        iban: Some("IBAN".into()),
    };

    let activities = vec![Dac7ActivitySummary {
        reporting_year: 2026,
        quarter: 1,
        number_of_activities: 10,
        total_consideration_cents: -500,
        fees_withheld_cents: 0,
        taxes_withheld_cents: 0,
    }];

    assert_eq!(
        evaluate_dac7_reporting_status(&seller, &activities).unwrap_err(),
        Dac7ComplianceError::NegativeConsideration { cents: -500 }
    );
}

#[test]
fn test_dac7_negative_fees_fails() {
    let seller = Dac7SellerProfile {
        seller_id: "acct_seller".into(),
        legal_name: "Test Seller".into(),
        tax_identification_number: Some("TIN".into()),
        country: "DE".into(),
        iban: Some("IBAN".into()),
    };

    let activities = vec![Dac7ActivitySummary {
        reporting_year: 2026,
        quarter: 1,
        number_of_activities: 10,
        total_consideration_cents: 500,
        fees_withheld_cents: -50,
        taxes_withheld_cents: 0,
    }];

    assert_eq!(
        evaluate_dac7_reporting_status(&seller, &activities).unwrap_err(),
        Dac7ComplianceError::NegativeFees { cents: -50 }
    );
}

#[test]
fn test_dac7_fees_exceed_consideration_fails() {
    let seller = Dac7SellerProfile {
        seller_id: "acct_seller".into(),
        legal_name: "Test Seller".into(),
        tax_identification_number: Some("TIN".into()),
        country: "DE".into(),
        iban: Some("IBAN".into()),
    };

    let activities = vec![Dac7ActivitySummary {
        reporting_year: 2026,
        quarter: 1,
        number_of_activities: 10,
        total_consideration_cents: 1_000,
        fees_withheld_cents: 1_500, // Fees > gross consideration
        taxes_withheld_cents: 0,
    }];

    assert_eq!(
        evaluate_dac7_reporting_status(&seller, &activities).unwrap_err(),
        Dac7ComplianceError::FeesExceedConsideration {
            fees: 1_500,
            consideration: 1_000,
        }
    );
}

#[test]
fn test_dac7_invalid_quarter_fails() {
    let seller = Dac7SellerProfile {
        seller_id: "acct_seller".into(),
        legal_name: "Test Seller".into(),
        tax_identification_number: Some("TIN".into()),
        country: "DE".into(),
        iban: Some("IBAN".into()),
    };

    let activities = vec![Dac7ActivitySummary {
        reporting_year: 2026,
        quarter: 5, // Invalid quarter
        number_of_activities: 10,
        total_consideration_cents: 1_000,
        fees_withheld_cents: 100,
        taxes_withheld_cents: 0,
    }];

    assert_eq!(
        evaluate_dac7_reporting_status(&seller, &activities).unwrap_err(),
        Dac7ComplianceError::InvalidQuarter { quarter: 5 }
    );
}

#[test]
fn test_dac7_multi_quarter_aggregation() {
    let seller = Dac7SellerProfile {
        seller_id: "acct_agg_seller".into(),
        legal_name: "Multi-Quarter Seller".into(),
        tax_identification_number: Some("IE1234567T".into()),
        country: "IE".into(),
        iban: Some("IE29AIBK93115212345678".into()),
    };

    let activities = vec![
        Dac7ActivitySummary {
            reporting_year: 2026,
            quarter: 1,
            number_of_activities: 10,
            total_consideration_cents: 100_000,
            fees_withheld_cents: 10_000,
            taxes_withheld_cents: 0,
        },
        Dac7ActivitySummary {
            reporting_year: 2026,
            quarter: 2,
            number_of_activities: 10,
            total_consideration_cents: 100_000,
            fees_withheld_cents: 10_000,
            taxes_withheld_cents: 0,
        },
        Dac7ActivitySummary {
            reporting_year: 2026,
            quarter: 3,
            number_of_activities: 15,
            total_consideration_cents: 150_000,
            fees_withheld_cents: 15_000,
            taxes_withheld_cents: 0,
        },
    ];

    let status = evaluate_dac7_reporting_status(&seller, &activities).unwrap();
    match status {
        Dac7ReportingStatus::ReportableSeller {
            total_activities,
            net_consideration_cents,
            tax_identifier,
            financial_identifier,
        } => {
            assert_eq!(total_activities, 35);
            assert_eq!(net_consideration_cents, 315_000); // 350,000 - 35,000
            assert_eq!(tax_identifier, "IE1234567T");
            assert_eq!(financial_identifier, "IE29AIBK93115212345678");
        }
        Dac7ReportingStatus::ExemptLowVolume { .. } => panic!("Expected reportable seller"),
    }
}

// ============================================================================
// 3. Connect Multi-Tenant Isolation Tests (Stripe-Account)
// ============================================================================

#[test]
fn test_multi_tenant_id_validation_rules() {
    assert!(validate_connect_account_id("acct_1032D82eZvKYlo2C").is_ok());
    assert!(validate_connect_account_id("acct_1H2y3z4w5v6u7t8s").is_ok());

    assert_eq!(
        validate_connect_account_id("").unwrap_err(),
        MultiTenantError::EmptyAccountId
    );
    assert_eq!(
        validate_connect_account_id("cus_1234567890abcdef").unwrap_err(),
        MultiTenantError::InvalidPrefix("cus_1234567890abcdef".into())
    );
    assert_eq!(
        validate_connect_account_id("acct_short").unwrap_err(),
        MultiTenantError::TooShort("acct_short".into())
    );
    assert_eq!(
        validate_connect_account_id("acct_bad#characters!").unwrap_err(),
        MultiTenantError::InvalidCharacters("acct_bad#characters!".into())
    );
}

#[test]
fn test_multi_tenant_context_header_matching_constant_time() {
    let ctx = MultiTenantContext::new("acct_1032D82eZvKYlo2C").unwrap();
    assert_eq!(ctx.account_id(), "acct_1032D82eZvKYlo2C");

    // Exact match
    assert!(ctx.matches_header("acct_1032D82eZvKYlo2C"));

    // Mismatched accounts
    assert!(!ctx.matches_header("acct_9999D82eZvKYlo2C"));
    assert!(!ctx.matches_header("acct_1032D82eZvKYlo2"));
    assert!(!ctx.matches_header(""));
}

#[test]
fn test_multi_tenant_boundary_isolation_and_assertions() {
    let ctx_a = MultiTenantContext::new("acct_1032D82eZvKYlo2C").unwrap();
    let ctx_b = MultiTenantContext::new("acct_9999D82eZvKYlo2C").unwrap();

    assert!(ctx_a.is_isolated_from(ctx_b.account_id()));
    assert!(ctx_b.is_isolated_from(ctx_a.account_id()));
    assert!(!ctx_a.is_isolated_from(ctx_a.account_id()));

    assert!(
        ctx_a
            .assert_tenant_boundary("acct_1032D82eZvKYlo2C")
            .is_ok()
    );
    let mismatch = ctx_a
        .assert_tenant_boundary("acct_9999D82eZvKYlo2C")
        .unwrap_err();
    assert_eq!(
        mismatch,
        MultiTenantError::TenantMismatch {
            expected: "acct_1032D82eZvKYlo2C".into(),
            found: "acct_9999D82eZvKYlo2C".into(),
        }
    );
}

#[test]
fn test_stripe_client_multi_tenant_header_injection() {
    let client = StripeClient::new("sk_test_mock_connect_key");

    let opts_tenant_a = RequestOptions::default().stripe_account("acct_tenant_a_12345");
    assert_eq!(
        opts_tenant_a.stripe_account.as_deref(),
        Some("acct_tenant_a_12345")
    );

    let opts_tenant_b = RequestOptions::default().stripe_account("acct_tenant_b_67890");
    assert_eq!(
        opts_tenant_b.stripe_account.as_deref(),
        Some("acct_tenant_b_67890")
    );

    // Multi-tenant boundary between requests: client remains immutable
    let accounts_resource = client.accounts();
    assert_eq!(
        accounts_resource.client().config.api_base,
        "https://api.stripe.com"
    );
}

// ============================================================================
// 4. Zero-Clone Borrowing Audit on Connect Domain Entities (Gate G-Borrow)
// ============================================================================

#[test]
fn test_zero_clone_borrowing_audit_across_connect_entities() {
    // Audit Account borrowing
    let account = Account {
        id: "acct_1032D82eZvKYlo2C".into(),
        object: "account".into(),
        charges_enabled: Some(true),
        payouts_enabled: Some(true),
        country: Some("US".into()),
        default_currency: Some("usd".into()),
        ..Default::default()
    };

    let acct_ref: &Account = &account;
    let inspected_id = audit_connect_entity_borrow(acct_ref, |a| a.id.as_str());
    assert_eq!(inspected_id, "acct_1032D82eZvKYlo2C");
    // Original account still accessible without move or clone
    assert_eq!(account.id, "acct_1032D82eZvKYlo2C");

    // Audit FinCenBeneficialOwner borrowing
    let owner = FinCenBeneficialOwner {
        id: "owner_borrow_audit".into(),
        first_name: "Margaret".into(),
        last_name: "Hamilton".into(),
        date_of_birth: "1936-08-17".into(),
        equity_percentage: 100.0,
        is_control_person: true,
        has_verified_tax_id: true,
        country: "US".into(),
    };

    let owner_ref: &FinCenBeneficialOwner = &owner;
    let is_ctrl = audit_connect_entity_borrow(owner_ref, |o| o.is_control_person());
    assert!(is_ctrl);
    assert_eq!(owner.first_name(), "Margaret");

    // Audit Dac7SellerProfile borrowing
    let seller = Dac7SellerProfile {
        seller_id: "seller_borrow_audit".into(),
        legal_name: "Apollo Navigation Systems".into(),
        tax_identification_number: Some("US123456789".into()),
        country: "US".into(),
        iban: Some("US89000000000000000000".into()),
    };

    let seller_ref: &Dac7SellerProfile = &seller;
    let tax_id = audit_connect_entity_borrow(seller_ref, |s| s.tax_identification_number());
    assert_eq!(tax_id, Some("US123456789"));
    assert_eq!(seller.legal_name(), "Apollo Navigation Systems");

    // Audit MultiTenantContext zero-copy lifetime
    let raw_id = String::from("acct_1032D82eZvKYlo2C");
    {
        let ctx = MultiTenantContext::new(&raw_id).unwrap();
        assert_eq!(ctx.account_id(), "acct_1032D82eZvKYlo2C");
        assert!(ctx.matches_header("acct_1032D82eZvKYlo2C"));
    }
    // raw_id was borrowed, not consumed
    assert_eq!(raw_id, "acct_1032D82eZvKYlo2C");
}
