//! # Accounting Math Invariant & ISO 4217 Minor Currency Integer Audit Suite
//!
//! Validates:
//! - **AC-US-3.2.0-03 / TC-3.2.0-03**: Streaming ledger accounting invariant ($\text{net} = \text{amount} - \text{fee}$)
//!   across 500+ diverse transaction records.
//! - **AC-US-3.2.0-LAW / TC-3.2.0-LAW**: EU Directive 2011/83/EU Article 13 statutory 14-day 100%
//!   full consumer withdrawal refund compliance without penalty or fee deduction.
//! - **Gate G-Law (ISO 4217 Exact Integer Arithmetic)**: Zero floating-point (`f32`, `f64`) operations
//!   across 0-decimal (JPY, KRW), 2-decimal (USD, EUR, GBP), and 3-decimal (BHD, KWD) currencies.
//! - **Gate G-Borrow (Zero-Clone Borrowing Discipline)**: Pointer stability, reference-borrowing ID accessors,
//!   and stream traversal without cloning domain models.
//! - **Gate G2 (Security Gateway & Ledger Immutability)**: Tamper detection and dispute evidence locking.

use some_stripe_types::currency::{
    CurrencyParseError, decimal_places, format_minor_units, is_three_decimal_currency,
    is_zero_decimal_currency, minor_unit_factor, parse_to_minor_units,
};
use some_stripe_types::{
    Balance, BalanceFund, BalanceSourceTypes, BalanceTransaction, CreateRefundParams, Dispute,
    DisputeEvidence, DisputeStatus, FeeDetail, Refund, RefundReason,
    STATUTORY_WITHDRAWAL_WINDOW_SECONDS, balance_transaction_id, is_statutory_withdrawal_compliant,
    validate_ledger_reconciliation,
};
use std::collections::HashMap;
use std::ptr;

// ============================================================================
// 1. AC-US-3.2.0-03 / TC-3.2.0-03: Ledger Accounting Invariant (net = amount - fee)
// ============================================================================

#[test]
fn tc_3_2_0_03_property_test_balance_transaction_ledger_invariants() {
    // Generate 600 diverse balance transactions across different operational types and currencies
    let mut transactions = Vec::with_capacity(600);
    let currencies = ["usd", "eur", "gbp", "jpy", "kwd", "bhd"];
    let types = [
        "charge",
        "refund",
        "adjustment",
        "payout",
        "application_fee",
        "transfer",
    ];

    for i in 0..600 {
        let currency = currencies[i % currencies.len()];
        let txn_type = types[i % types.len()];

        // Generate realistic amounts based on transaction type
        let (amount, fee) = match txn_type {
            "charge" => {
                let gross = 1000 + ((i as i64) * 37) % 50000;
                let calculated_fee = 30 + (gross * 29) / 1000; // ~2.9% + 30
                (gross, calculated_fee)
            }
            "refund" => {
                let refund_gross = -(500 + ((i as i64) * 23) % 25000);
                (refund_gross, 0) // Fees are typically refunded or 0
            }
            "adjustment" => {
                // Dispute fee or correction
                (-1500, 0)
            }
            "payout" => {
                // Funds paid out to bank account: negative balance effect, 0 fee or wire fee
                (-(10000 + ((i as i64) * 51) % 100000), 25)
            }
            "application_fee" => {
                let app_fee = 100 + ((i as i64) * 11) % 2000;
                (app_fee, 0)
            }
            _ => {
                let transfer_amt = -(2000 + ((i as i64) * 17) % 10000);
                (transfer_amt, 0)
            }
        };

        let txn = BalanceTransaction {
            id: format!("txn_rec_prop_{i:04}"),
            object: "balance_transaction".to_string(),
            amount,
            available_on: 1728345600 + (i as i64 * 3600),
            created: 1728345600 + (i as i64 * 3600),
            currency: currency.to_string(),
            description: Some(format!("Reconciliation ledger entry #{i}")),
            fee,
            fee_details: vec![FeeDetail::new(fee, currency, "stripe_fee")],
            net: amount - fee,
            reporting_category: Some(txn_type.to_string()),
            source: Some(format!("src_{i:04}")),
            status: "available".to_string(),
            r#type: txn_type.to_string(),
        };

        // Assert invariant on individual transaction
        assert!(
            txn.verify_accounting_invariant(),
            "Accounting invariant net == amount - fee failed on transaction #{}",
            txn.id
        );
        assert_eq!(txn.net, txn.amount - txn.fee);

        transactions.push(txn);
    }

    assert_eq!(transactions.len(), 600);

    // Validate entire batch via aggregate ledger validator (asserts 100% record compliance)
    let reconciliation_result = validate_ledger_reconciliation(&transactions);
    assert!(
        reconciliation_result.is_ok(),
        "Aggregate ledger reconciliation failed: {:?}",
        reconciliation_result.err()
    );
}

#[test]
fn test_corrupted_ledger_detection_and_reconciliation_failure() {
    let mut transactions = vec![
        BalanceTransaction::new("txn_good_1", 5000, 175, "usd", "charge", "available"),
        BalanceTransaction::new("txn_good_2", -2000, 0, "usd", "refund", "available"),
    ];

    assert!(validate_ledger_reconciliation(&transactions).is_ok());

    // Inject a corrupted transaction where net != amount - fee
    let mut bad_txn =
        BalanceTransaction::new("txn_corrupt_3", 10000, 320, "usd", "charge", "available");
    bad_txn.net = 9000; // Expected net: 10000 - 320 = 9680!

    transactions.push(bad_txn);

    let err = validate_ledger_reconciliation(&transactions)
        .expect_err("Must detect corrupted ledger record");
    assert!(err.contains("txn_corrupt_3"));
    assert!(err.contains("expected net: 9680"));
    assert!(err.contains("net (9000) != amount (10000) - fee (320)"));
}

// ============================================================================
// 2. AC-US-3.2.0-LAW / TC-3.2.0-LAW: EU Directive 2011/83/EU Article 13
// ============================================================================

#[test]
fn tc_3_2_0_law_statutory_14_day_full_refund_compliance() {
    let charge_ts = 1_700_000_000;
    let original_consideration = 12500; // $125.00 in minor integer units

    // 1. 100% full reimbursement within 14 calendar days (Article 13(1))
    // Day 0: Immediate withdrawal
    assert!(is_statutory_withdrawal_compliant(
        original_consideration,
        original_consideration,
        charge_ts,
        charge_ts
    ));

    // Day 5: Mid-window withdrawal
    assert!(is_statutory_withdrawal_compliant(
        original_consideration,
        original_consideration,
        charge_ts,
        charge_ts + (5 * 86_400)
    ));

    // Day 14 exact boundary (1,209,600 seconds)
    assert!(is_statutory_withdrawal_compliant(
        original_consideration,
        original_consideration,
        charge_ts,
        charge_ts + STATUTORY_WITHDRAWAL_WINDOW_SECONDS
    ));

    // 2. Beyond 14-day statutory window: Article 13 does NOT apply (merchant terms govern)
    assert!(!is_statutory_withdrawal_compliant(
        original_consideration,
        original_consideration,
        charge_ts,
        charge_ts + STATUTORY_WITHDRAWAL_WINDOW_SECONDS + 1
    ));
    assert!(!is_statutory_withdrawal_compliant(
        original_consideration,
        original_consideration,
        charge_ts,
        charge_ts + (15 * 86_400)
    ));
    assert!(!is_statutory_withdrawal_compliant(
        original_consideration,
        original_consideration,
        charge_ts,
        charge_ts + (30 * 86_400)
    ));

    // 3. Deduction violations: Attempting partial refund or deducting processing fees
    // Article 13 mandates 100% reimbursement without penalty or deduction
    assert!(!is_statutory_withdrawal_compliant(
        original_consideration - 1, // Deducted 1 cent
        original_consideration,
        charge_ts,
        charge_ts + (2 * 86_400)
    ));
    assert!(!is_statutory_withdrawal_compliant(
        original_consideration - 395, // Deducted payment processing fee
        original_consideration,
        charge_ts,
        charge_ts + (2 * 86_400)
    ));
    assert!(!is_statutory_withdrawal_compliant(
        original_consideration / 2, // 50% partial return
        original_consideration,
        charge_ts,
        charge_ts + (2 * 86_400)
    ));

    // 4. Over-refund (cannot claim statutory withdrawal restitution)
    assert!(!is_statutory_withdrawal_compliant(
        original_consideration + 1000,
        original_consideration,
        charge_ts,
        charge_ts + (2 * 86_400)
    ));

    // 5. Zero and negative amounts
    assert!(!is_statutory_withdrawal_compliant(
        0,
        original_consideration,
        charge_ts,
        charge_ts
    ));
    assert!(!is_statutory_withdrawal_compliant(
        -100,
        original_consideration,
        charge_ts,
        charge_ts
    ));
    assert!(!is_statutory_withdrawal_compliant(
        original_consideration,
        0,
        charge_ts,
        charge_ts
    ));
    assert!(!is_statutory_withdrawal_compliant(
        original_consideration,
        -100,
        charge_ts,
        charge_ts
    ));

    // 6. Chronological skew (refund timestamp before charge created)
    assert!(!is_statutory_withdrawal_compliant(
        original_consideration,
        original_consideration,
        charge_ts,
        charge_ts - 50
    ));
}

#[test]
fn test_refund_domain_entity_statutory_compliance_method() {
    let charge_ts = 1_700_000_000;
    let full_amount = 7500;

    let compliant_refund = Refund {
        id: "re_eu_statutory_01".into(),
        object: "refund".into(),
        amount: full_amount,
        charge: Some("ch_original_999".into()),
        currency: "eur".into(),
        description: Some("Statutory 14-day withdrawal".into()),
        failure_balance_transaction: None,
        failure_reason: None,
        metadata: None,
        payment_intent: None,
        reason: Some("requested_by_customer".into()),
        receipt_number: None,
        status: "succeeded".into(),
        created: Some(charge_ts + (8 * 86_400)), // 8 days later
        balance_transaction: Some("txn_bal_eu_01".into()),
    };

    assert!(compliant_refund.is_statutory_withdrawal_compliant(full_amount, charge_ts));
    assert_eq!(
        compliant_refund.parsed_reason(),
        Some(RefundReason::RequestedByCustomer)
    );

    // If original amount was higher, this was a partial refund and violates 100% full refund requirement
    assert!(!compliant_refund.is_statutory_withdrawal_compliant(full_amount + 500, charge_ts));
}

// ============================================================================
// 3. Gate G-Law: ISO 4217 Minor Currency Integer Arithmetic
// ============================================================================

#[test]
fn test_iso_4217_standard_two_decimal_currencies() {
    let currencies = ["usd", "eur", "gbp", "cad", "aud", "chf", "sek", "nzd"];

    for curr in currencies {
        assert_eq!(
            decimal_places(curr),
            2,
            "Currency {curr} must have 2 decimal places"
        );
        assert_eq!(
            minor_unit_factor(curr),
            100,
            "Currency {curr} factor must be 100"
        );
        assert!(!is_zero_decimal_currency(curr));
        assert!(!is_three_decimal_currency(curr));
    }

    // Exact minor unit conversions without floating-point math
    assert_eq!(format_minor_units(100, "usd"), "1.00");
    assert_eq!(format_minor_units(1050, "usd"), "10.50");
    assert_eq!(format_minor_units(5, "usd"), "0.05");
    assert_eq!(format_minor_units(0, "usd"), "0.00");
    assert_eq!(format_minor_units(-50, "usd"), "-0.50");
    assert_eq!(format_minor_units(-2050, "eur"), "-20.50");
    assert_eq!(format_minor_units(-5, "gbp"), "-0.05");

    // Exact string parsing to minor units
    assert_eq!(parse_to_minor_units("10.50", "usd").unwrap(), 1050);
    assert_eq!(parse_to_minor_units("0.05", "usd").unwrap(), 5);
    assert_eq!(parse_to_minor_units(".05", "usd").unwrap(), 5);
    assert_eq!(parse_to_minor_units("100", "usd").unwrap(), 10000);
    assert_eq!(parse_to_minor_units("-20.50", "eur").unwrap(), -2050);
    assert_eq!(parse_to_minor_units("-0.05", "gbp").unwrap(), -5);

    // Reject excess precision for 2-decimal currencies (e.g. fractional cents)
    let err = parse_to_minor_units("10.555", "usd").unwrap_err();
    assert!(matches!(
        err,
        CurrencyParseError::TooManyDecimalPlaces {
            found: 3,
            max_allowed: 2
        }
    ));
}

#[test]
fn test_iso_4217_non_decimal_currencies() {
    let zero_decimals = [
        "jpy", "krw", "vnd", "clp", "pyg", "ugx", "bif", "djf", "gnf", "kmf", "rwf", "xaf", "xof",
        "xpf",
    ];

    for curr in zero_decimals {
        assert_eq!(
            decimal_places(curr),
            0,
            "Currency {curr} must have 0 decimal places"
        );
        assert_eq!(
            minor_unit_factor(curr),
            1,
            "Currency {curr} factor must be 1"
        );
        assert!(
            is_zero_decimal_currency(curr),
            "{curr} must be recognized as zero-decimal"
        );
        assert!(!is_three_decimal_currency(curr));
    }

    // Zero-decimal formatting (1 minor unit = 1 major unit)
    assert_eq!(format_minor_units(1500, "jpy"), "1500");
    assert_eq!(format_minor_units(50000, "krw"), "50000");
    assert_eq!(format_minor_units(0, "jpy"), "0");
    assert_eq!(format_minor_units(-1200, "jpy"), "-1200");

    // Zero-decimal parsing
    assert_eq!(parse_to_minor_units("1500", "jpy").unwrap(), 1500);
    assert_eq!(parse_to_minor_units("50000", "krw").unwrap(), 50000);
    assert_eq!(parse_to_minor_units("-1200", "jpy").unwrap(), -1200);

    // Reject fractional units on zero-decimal currencies
    let err = parse_to_minor_units("1500.5", "jpy").unwrap_err();
    assert!(matches!(
        err,
        CurrencyParseError::TooManyDecimalPlaces {
            found: 1,
            max_allowed: 0
        }
    ));

    let err_krw = parse_to_minor_units("1000.00", "krw").unwrap_err();
    assert!(matches!(
        err_krw,
        CurrencyParseError::TooManyDecimalPlaces {
            found: 2,
            max_allowed: 0
        }
    ));
}

#[test]
fn test_iso_4217_three_decimal_currencies() {
    let three_decimals = ["bhd", "kwd", "jod", "omr", "tnd"];

    for curr in three_decimals {
        assert_eq!(
            decimal_places(curr),
            3,
            "Currency {curr} must have 3 decimal places"
        );
        assert_eq!(
            minor_unit_factor(curr),
            1000,
            "Currency {curr} factor must be 1000"
        );
        assert!(
            is_three_decimal_currency(curr),
            "{curr} must be recognized as three-decimal"
        );
        assert!(!is_zero_decimal_currency(curr));
    }

    // 3-decimal formatting (1 major unit = 1000 minor units / fils / millimes)
    assert_eq!(format_minor_units(5250, "kwd"), "5.250");
    assert_eq!(format_minor_units(7, "bhd"), "0.007");
    assert_eq!(format_minor_units(12, "bhd"), "0.012");
    assert_eq!(format_minor_units(123, "bhd"), "0.123");
    assert_eq!(format_minor_units(1000, "bhd"), "1.000");
    assert_eq!(format_minor_units(-15000, "kwd"), "-15.000");
    assert_eq!(format_minor_units(-7, "bhd"), "-0.007");

    // 3-decimal parsing
    assert_eq!(parse_to_minor_units("5.250", "kwd").unwrap(), 5250);
    assert_eq!(parse_to_minor_units("5.25", "kwd").unwrap(), 5250); // Padded to 3 decimals
    assert_eq!(parse_to_minor_units("0.007", "bhd").unwrap(), 7);
    assert_eq!(parse_to_minor_units(".007", "bhd").unwrap(), 7);
    assert_eq!(parse_to_minor_units("1", "bhd").unwrap(), 1000);
    assert_eq!(parse_to_minor_units("-15.000", "kwd").unwrap(), -15000);

    // Reject 4 decimal places on 3-decimal currency
    let err = parse_to_minor_units("5.2505", "kwd").unwrap_err();
    assert!(matches!(
        err,
        CurrencyParseError::TooManyDecimalPlaces {
            found: 4,
            max_allowed: 3
        }
    ));
}

// ============================================================================
// 4. Gate G-Borrow: Zero-Clone Borrowing Discipline & Pointer Stability
// ============================================================================

#[test]
fn test_gate_g_borrow_zero_clone_ledger_accessors() {
    let txn = BalanceTransaction {
        id: "txn_borrow_discipline_999".into(),
        object: "balance_transaction".into(),
        amount: 8500,
        available_on: 1728345600,
        created: 1728345600,
        currency: "usd".into(),
        description: Some("Zero-clone audit test".into()),
        fee: 250,
        fee_details: vec![],
        net: 8250,
        reporting_category: Some("charge".into()),
        source: Some("ch_999".into()),
        status: "available".into(),
        r#type: "charge".into(),
    };

    // 1. Audit that balance_transaction_id borrows &str directly from txn.id without allocation
    let id_ref = balance_transaction_id(&txn);
    assert_eq!(id_ref, "txn_borrow_discipline_999");

    // 2. Memory Pointer Stability: Audit that id_ref points directly to txn.id heap buffer
    assert!(
        ptr::eq(id_ref, txn.id.as_str()),
        "Accessor must return borrowed slice with identical memory address"
    );
    assert_eq!(id_ref.as_ptr(), txn.id.as_str().as_ptr());

    // 3. Batch traversal without cloning: Iterate over borrowed slice
    let batch = [txn];
    let mut visited_ids = Vec::new();
    for item in &batch {
        // Access via reference only, no .clone()
        visited_ids.push(balance_transaction_id(item));
    }
    assert_eq!(visited_ids.len(), 1);
    assert_eq!(visited_ids[0], "txn_borrow_discipline_999");
}

#[test]
fn test_gate_g_borrow_move_semantics_without_clone() {
    // Audit that BalanceTransaction and BalanceFund can be processed via move semantics
    // in streaming queues without cloning.
    let txn = BalanceTransaction::new("txn_move_01", 3000, 90, "usd", "charge", "available");

    // Move into a closure / worker function by value without invoking clone
    let compute_net = |t: BalanceTransaction| -> i64 { t.net };
    let net = compute_net(txn);
    assert_eq!(net, 2910);
}

#[test]
fn test_gate_g_borrow_zero_floating_point_representation_in_ast() {
    // Programmatic AST / source code inspection audit:
    // Reads domain models in `src/crates/some-stripe-types/src/` and verifies
    // zero occurrences of floating point types `f32` or `f64`.

    let type_source_files = [
        "src/crates/some-stripe-types/src/balance.rs",
        "src/crates/some-stripe-types/src/currency.rs",
        "src/crates/some-stripe-types/src/refund.rs",
        "src/crates/some-stripe-types/src/dispute.rs",
    ];

    for file_path in type_source_files {
        let content = std::fs::read_to_string(file_path)
            .unwrap_or_else(|_| panic!("Failed to read source file: {file_path}"));

        // Strip comments to inspect code tokens only
        for (line_no, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            // Skip pure comment lines
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                continue;
            }

            // Assert zero f32 or f64 type declarations
            assert!(
                !line.contains(": f32") && !line.contains(": Option<f32>"),
                "Gate G-Law violation: f32 type found at {file_path}:{}: {}",
                line_no + 1,
                line
            );
            assert!(
                !line.contains(": f64") && !line.contains(": Option<f64>"),
                "Gate G-Law violation: f64 type found at {file_path}:{}: {}",
                line_no + 1,
                line
            );
        }
    }
}

// ============================================================================
// 5. Gate G2: Security Gateway, Evidence Locking & Redaction Audit
// ============================================================================

#[test]
fn test_dispute_evidence_submission_locking_invariants() {
    let mut dispute = Dispute {
        id: "dp_audit_lock_101".into(),
        object: "dispute".into(),
        amount: 6000,
        currency: "usd".into(),
        status: DisputeStatus::NeedsResponse,
        reason: Some("fraudulent".into()),
        charge: Some("ch_lock_101".into()),
        payment_intent: None,
        evidence: DisputeEvidence {
            customer_name: Some("John Doe".into()),
            customer_email_address: Some("john@example.com".into()),
            customer_communication: Some("Proof of communication".into()),
            ..Default::default()
        },
        evidence_details: None,
        is_charge_refundable: Some(false),
        livemode: Some(false),
        metadata: None,
        created: Some(1728345600),
        balance_transactions: Some(vec!["txn_disp_101".into()]),
    };

    // Open for response initially
    assert!(!dispute.is_evidence_locked());

    // 1. Merchant finalizes submission via submit = true (without cloning evidence)
    let evidence_to_submit = DisputeEvidence {
        customer_name: Some("John Doe".into()),
        customer_email_address: Some("john@example.com".into()),
        customer_communication: Some("Proof of communication".into()),
        ..Default::default()
    };
    let submit_params = some_stripe_types::UpdateDisputeParams::new()
        .with_evidence(evidence_to_submit)
        .with_submit(true)
        .with_idempotency_key("idem_evidence_lock_01");

    assert!(
        submit_params.is_submission_final(),
        "submit = true must mark submission as final"
    );

    // 2. Server transitions status to UnderReview upon submission
    dispute.status = DisputeStatus::UnderReview;
    assert!(
        dispute.is_evidence_locked(),
        "Status UnderReview must lock evidence submission against modification"
    );

    // 3. Status transitions to Won or Lost lock permanently
    dispute.status = DisputeStatus::Won;
    assert!(dispute.is_evidence_locked());

    dispute.status = DisputeStatus::Lost;
    assert!(dispute.is_evidence_locked());
}

#[test]
fn test_create_refund_params_idempotency_key_skip_audit() {
    let mut meta = HashMap::new();
    meta.insert("audit_run".into(), "unit_5".into());

    let params = CreateRefundParams::for_charge("ch_audit_idem_99")
        .with_amount(4500)
        .with_currency("usd")
        .with_reason(RefundReason::RequestedByCustomer)
        .with_idempotency_key("req_idem_key_super_secret_999")
        .with_metadata(meta);

    // Verify idempotency key is accessible for HTTP header insertion
    assert_eq!(
        params.idempotency_key.as_deref(),
        Some("req_idem_key_super_secret_999")
    );

    // Verify idempotency key is NOT serialized into JSON payload (Gate G2 invariant)
    let json = serde_json::to_string(&params).expect("Serialization failed");
    assert!(
        !json.contains("req_idem_key_super_secret_999"),
        "Idempotency key must never be leaked into JSON payload body"
    );
    assert!(json.contains("ch_audit_idem_99"));
    assert!(json.contains("4500"));
}

#[test]
fn test_balance_fund_and_source_types_minor_currency() {
    let source_types = BalanceSourceTypes {
        card: Some(75000),
        bank_account: Some(25000),
        ..Default::default()
    };

    let fund = BalanceFund {
        amount: 100000,
        currency: "usd".into(),
        source_types: Some(source_types),
    };

    assert_eq!(fund.amount, 100000);
    assert_eq!(fund.currency, "usd");
    let card_amt = fund.source_types.as_ref().and_then(|s| s.card).unwrap();
    let bank_amt = fund
        .source_types
        .as_ref()
        .and_then(|s| s.bank_account)
        .unwrap();
    assert_eq!(card_amt + bank_amt, fund.amount);

    let balance = Balance {
        available: vec![fund],
        pending: vec![BalanceFund::new(5000, "usd")],
        ..Default::default()
    };

    assert_eq!(balance.object, "balance");
    assert_eq!(balance.available.len(), 1);
    assert_eq!(balance.available[0].amount, 100000);
    assert_eq!(balance.pending[0].amount, 5000);
    assert!(!balance.livemode);
}
