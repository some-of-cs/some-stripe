//! Unit tests for Treasury Outbound Transfers, Outbound Payments, and Received Credits.
//!
//! Validates:
//! - Domain entity serialization/deserialization.
//! - Parameter validation invariants (amount, currency, account ID).
//! - Statutory balance sufficiency verification (AC-US-7.2.0-02).
//! - Bank Secrecy Act (BSA) / AML thresholds (Gate G-Law).
//! - PCI-DSS 4.0 zero PAN leakage protection (Gate G2).
//! - Velocity limits and tracking.

use some_stripe_types::{
    BsaAmlClassification, CreateOutboundTransferParams, EventObject, ListOutboundPaymentsParams,
    ListOutboundTransfersParams, ListReceivedCreditsParams, OutboundPayment, OutboundPaymentList,
    OutboundPaymentStatus, OutboundTransfer, OutboundTransferList, OutboundTransferStatus,
    ReceivedCredit, ReceivedCreditList, ReceivedCreditStatus, TreasuryTransferValidationError,
    TreasuryVelocityTracker, classify_bsa_aml_transfer, detect_pan_leakage,
    verify_sufficient_balance,
};
use std::collections::HashMap;
use std::str::FromStr;

#[test]
fn test_outbound_transfer_serde() {
    let json_data = r#"{
        "id": "obt_12345",
        "object": "treasury.outbound_transfer",
        "amount": 250000,
        "cancelable": true,
        "created": 1728345600,
        "currency": "usd",
        "description": "Vendor invoice payout",
        "destination_payment_method": "pm_bank_wire_123",
        "destination_payment_method_details": {
            "type": "us_bank_account",
            "us_bank_account": {
                "account_holder_name": "Acme Corp",
                "bank_name": "Silicon Valley Bank",
                "last4": "6789",
                "routing_number": "121141976"
            }
        },
        "expected_arrival_date": 1728432000,
        "financial_account": "fa_12345",
        "hosted_regulatory_receipt_url": "https://receipts.stripe.com/treasury/obt_12345",
        "livemode": false,
        "status": "posted",
        "status_transitions": {
            "posted_at": 1728345700
        },
        "tracking_details": {
            "type": "ach",
            "ach": {
                "trace_number": "021000021234567"
            }
        },
        "transaction": "trxn_998877"
    }"#;

    let transfer: OutboundTransfer = serde_json::from_str(json_data).unwrap();
    assert_eq!(transfer.id, "obt_12345");
    assert_eq!(transfer.object, "treasury.outbound_transfer");
    assert_eq!(transfer.amount, 250000);
    assert!(transfer.cancelable);
    assert_eq!(transfer.currency, "usd");
    assert_eq!(transfer.status, OutboundTransferStatus::Posted);
    assert_eq!(transfer.financial_account, "fa_12345");
    assert_eq!(
        transfer.destination_payment_method.as_deref(),
        Some("pm_bank_wire_123")
    );

    let details = transfer
        .destination_payment_method_details
        .as_ref()
        .unwrap();
    assert_eq!(details.type_, "us_bank_account");
    let bank = details.us_bank_account.as_ref().unwrap();
    assert_eq!(bank.last4.as_deref(), Some("6789"));
    assert_eq!(bank.bank_name.as_deref(), Some("Silicon Valley Bank"));

    let tracking = transfer.tracking_details.as_ref().unwrap();
    assert_eq!(tracking.type_, "ach");
    assert_eq!(
        tracking.ach.as_ref().unwrap().trace_number,
        "021000021234567"
    );

    // Re-serialize and verify roundtrip
    let serialized = serde_json::to_string(&transfer).unwrap();
    let deserialized: OutboundTransfer = serde_json::from_str(&serialized).unwrap();
    assert_eq!(transfer, deserialized);
}

#[test]
fn test_outbound_payment_serde() {
    let json_data = r#"{
        "id": "obp_98765",
        "object": "treasury.outbound_payment",
        "amount": 75000,
        "cancelable": false,
        "created": 1728346000,
        "currency": "usd",
        "customer": "cus_vendor_111",
        "description": "Consulting services",
        "destination_payment_method": "pm_ach_external",
        "expected_arrival_date": 1728432400,
        "financial_account": "fa_main_treasury",
        "livemode": false,
        "status": "processing",
        "status_transitions": {
            "posted_at": null
        },
        "end_user_details": {
            "ip_address": "192.0.2.1",
            "present": true
        }
    }"#;

    let payment: OutboundPayment = serde_json::from_str(json_data).unwrap();
    assert_eq!(payment.id, "obp_98765");
    assert_eq!(payment.object, "treasury.outbound_payment");
    assert_eq!(payment.amount, 75000);
    assert!(!payment.cancelable);
    assert_eq!(payment.status, OutboundPaymentStatus::Processing);
    assert_eq!(payment.customer.as_deref(), Some("cus_vendor_111"));

    let end_user = payment.end_user_details.as_ref().unwrap();
    assert_eq!(end_user.ip_address.as_deref(), Some("192.0.2.1"));
    assert!(end_user.present);

    let serialized = serde_json::to_string(&payment).unwrap();
    let deserialized: OutboundPayment = serde_json::from_str(&serialized).unwrap();
    assert_eq!(payment, deserialized);
}

#[test]
fn test_received_credit_serde() {
    let json_data = r#"{
        "id": "rc_554433",
        "object": "treasury.received_credit",
        "amount": 1000000,
        "created": 1728340000,
        "currency": "usd",
        "description": "Wire transfer deposit",
        "financial_account": "fa_deposit_account",
        "livemode": false,
        "network": "us_domestic_wire",
        "status": "succeeded",
        "transaction": "trxn_credit_1"
    }"#;

    let credit: ReceivedCredit = serde_json::from_str(json_data).unwrap();
    assert_eq!(credit.id, "rc_554433");
    assert_eq!(credit.object, "treasury.received_credit");
    assert_eq!(credit.amount, 1000000);
    assert_eq!(credit.network, "us_domestic_wire");
    assert_eq!(credit.status, ReceivedCreditStatus::Succeeded);
    assert_eq!(credit.transaction.as_deref(), Some("trxn_credit_1"));

    let serialized = serde_json::to_string(&credit).unwrap();
    let deserialized: ReceivedCredit = serde_json::from_str(&serialized).unwrap();
    assert_eq!(credit, deserialized);
}

#[test]
fn test_status_enums_and_display() {
    // OutboundTransferStatus
    assert_eq!(OutboundTransferStatus::Posted.as_str(), "posted");
    assert_eq!(OutboundTransferStatus::Processing.as_str(), "processing");
    assert_eq!(OutboundTransferStatus::Canceled.as_str(), "canceled");
    assert_eq!(OutboundTransferStatus::Failed.as_str(), "failed");
    assert_eq!(OutboundTransferStatus::Returned.as_str(), "returned");
    assert_eq!(OutboundTransferStatus::Posted.to_string(), "posted");
    assert_eq!(
        OutboundTransferStatus::from_str("posted").unwrap(),
        OutboundTransferStatus::Posted
    );
    assert_eq!(
        OutboundTransferStatus::from_str("canceled").unwrap(),
        OutboundTransferStatus::Canceled
    );
    assert!(OutboundTransferStatus::from_str("unknown_state").is_err());
    assert_eq!(OutboundTransferStatus::Posted, "posted");

    // OutboundPaymentStatus
    assert_eq!(OutboundPaymentStatus::Posted.as_str(), "posted");
    assert_eq!(OutboundPaymentStatus::Processing.as_str(), "processing");
    assert_eq!(OutboundPaymentStatus::Canceled.as_str(), "canceled");
    assert_eq!(OutboundPaymentStatus::Failed.as_str(), "failed");
    assert_eq!(OutboundPaymentStatus::Returned.as_str(), "returned");
    assert_eq!(OutboundPaymentStatus::Posted.to_string(), "posted");
    assert_eq!(
        OutboundPaymentStatus::from_str("posted").unwrap(),
        OutboundPaymentStatus::Posted
    );
    assert!(OutboundPaymentStatus::from_str("invalid").is_err());

    // ReceivedCreditStatus
    assert_eq!(ReceivedCreditStatus::Succeeded.as_str(), "succeeded");
    assert_eq!(ReceivedCreditStatus::Failed.as_str(), "failed");
    assert_eq!(ReceivedCreditStatus::Succeeded.to_string(), "succeeded");
    assert_eq!(
        ReceivedCreditStatus::from_str("succeeded").unwrap(),
        ReceivedCreditStatus::Succeeded
    );
    assert_eq!(
        ReceivedCreditStatus::from_str("failed").unwrap(),
        ReceivedCreditStatus::Failed
    );
    assert!(ReceivedCreditStatus::from_str("pending").is_err());
}

#[test]
fn test_create_outbound_transfer_params_validation_success() {
    let params = CreateOutboundTransferParams::new("fa_test123", 250000, "usd")
        .with_destination_payment_method("pm_bank_wire_123")
        .with_description("Legitimate business payout")
        .with_statement_descriptor("ACME CORP WIRE")
        .with_idempotency_key("idem_obt_123");

    assert!(params.validate().is_ok());
    assert_eq!(params.financial_account, "fa_test123");
    assert_eq!(params.amount, 250000);
    assert_eq!(params.currency, "usd");
    assert_eq!(
        params.destination_payment_method.as_deref(),
        Some("pm_bank_wire_123")
    );
}

#[test]
fn test_create_outbound_transfer_params_validation_failures() {
    // Non-positive amount
    let bad_amount = CreateOutboundTransferParams::new("fa_123", 0, "usd");
    assert!(matches!(
        bad_amount.validate(),
        Err(TreasuryTransferValidationError::InvalidAmount { amount: 0 })
    ));

    let negative_amount = CreateOutboundTransferParams::new("fa_123", -500, "usd");
    assert!(matches!(
        negative_amount.validate(),
        Err(TreasuryTransferValidationError::InvalidAmount { amount: -500 })
    ));

    // Invalid currency
    let uppercase_cur = CreateOutboundTransferParams::new("fa_123", 1000, "USD");
    assert!(matches!(
        uppercase_cur.validate(),
        Err(TreasuryTransferValidationError::InvalidCurrency { .. })
    ));

    let long_cur = CreateOutboundTransferParams::new("fa_123", 1000, "usdollar");
    assert!(matches!(
        long_cur.validate(),
        Err(TreasuryTransferValidationError::InvalidCurrency { .. })
    ));

    // Invalid financial account
    let invalid_fa = CreateOutboundTransferParams::new("acct_not_fa", 1000, "usd");
    assert!(matches!(
        invalid_fa.validate(),
        Err(TreasuryTransferValidationError::InvalidFinancialAccount { .. })
    ));

    let empty_fa = CreateOutboundTransferParams::new("", 1000, "usd");
    assert!(matches!(
        empty_fa.validate(),
        Err(TreasuryTransferValidationError::InvalidFinancialAccount { .. })
    ));
}

#[test]
fn test_balance_verification_insufficient_funds() {
    // Scenario [AC-US-7.2.0-02]: Available $10,000.00 (1,000,000 cents), requested $2,500.00 (250,000 cents)
    let available_balance = 1_000_000;
    let transfer_amount = 250_000;

    let params = CreateOutboundTransferParams::new("fa_123", transfer_amount, "usd");
    assert!(params.verify_balance(available_balance).is_ok());

    // Excess amount
    let excessive_amount = 1_500_000;
    let excessive_params = CreateOutboundTransferParams::new("fa_123", excessive_amount, "usd");
    let err = excessive_params
        .verify_balance(available_balance)
        .unwrap_err();
    assert_eq!(
        err,
        TreasuryTransferValidationError::InsufficientFunds {
            available: 1_000_000,
            required: 1_500_000,
        }
    );

    // Direct helper
    assert!(verify_sufficient_balance(5000, 5000).is_ok());
    assert!(verify_sufficient_balance(5000, 5001).is_err());
    assert!(verify_sufficient_balance(5000, 0).is_err());
}

#[test]
fn test_zero_pan_leakage_detection() {
    // Valid Visa test card passing Luhn mod 10
    let valid_luhn_pan = "4532015112830366";
    assert!(detect_pan_leakage(valid_luhn_pan));
    assert!(detect_pan_leakage(&format!(
        "Payment for card {valid_luhn_pan}"
    )));

    // Hyphenated format
    let hyphenated_pan = "4532-0151-1283-0366";
    assert!(detect_pan_leakage(hyphenated_pan));

    // Space separated format
    let spaced_pan = "4532 0151 1283 0366";
    assert!(detect_pan_leakage(spaced_pan));

    // Safe numbers should not trigger PAN detection
    assert!(!detect_pan_leakage("Invoice #12345678"));
    assert!(!detect_pan_leakage("Order ref: PO-998877"));
    assert!(!detect_pan_leakage("Payment date 2026-11-07"));
    assert!(!detect_pan_leakage("Routing 121141976 and last4 6789"));

    // Validation rejection in CreateOutboundTransferParams
    let leaking_desc_params = CreateOutboundTransferParams::new("fa_123", 5000, "usd")
        .with_description(format!("Send to card {valid_luhn_pan}"));
    assert!(matches!(
        leaking_desc_params.validate(),
        Err(TreasuryTransferValidationError::PanLeakageDetected {
            field: "description"
        })
    ));

    let leaking_statement_params = CreateOutboundTransferParams::new("fa_123", 5000, "usd")
        .with_statement_descriptor(valid_luhn_pan);
    assert!(matches!(
        leaking_statement_params.validate(),
        Err(TreasuryTransferValidationError::PanLeakageDetected {
            field: "statement_descriptor"
        })
    ));

    // Validation rejection in metadata
    let mut meta = HashMap::new();
    meta.insert("card_num".to_string(), valid_luhn_pan.to_string());
    let mut leaking_meta_params = CreateOutboundTransferParams::new("fa_123", 5000, "usd");
    leaking_meta_params.metadata = Some(meta);
    assert!(matches!(
        leaking_meta_params.validate(),
        Err(TreasuryTransferValidationError::PanLeakageDetected { field: "metadata" })
    ));
}

#[test]
fn test_bsa_aml_classification() {
    // Under $3,000.00 (< 300,000 cents) -> Standard
    assert_eq!(
        classify_bsa_aml_transfer(100_000),
        BsaAmlClassification::Standard
    );
    assert_eq!(
        classify_bsa_aml_transfer(299_999),
        BsaAmlClassification::Standard
    );

    // $3,000.00 to $9,999.99 (300,000 to 999,999 cents) -> TravelRuleReportable
    assert_eq!(
        classify_bsa_aml_transfer(300_000),
        BsaAmlClassification::TravelRuleReportable
    );
    assert_eq!(
        classify_bsa_aml_transfer(500_000),
        BsaAmlClassification::TravelRuleReportable
    );
    assert_eq!(
        classify_bsa_aml_transfer(999_999),
        BsaAmlClassification::TravelRuleReportable
    );

    // >= $10,000.00 (>= 1,000,000 cents) -> CtrReportable
    assert_eq!(
        classify_bsa_aml_transfer(1_000_000),
        BsaAmlClassification::CtrReportable
    );
    assert_eq!(
        classify_bsa_aml_transfer(5_000_000),
        BsaAmlClassification::CtrReportable
    );
}

#[test]
fn test_velocity_tracker_volume_limit() {
    let now = 1728345600;
    let max_volume = 1_000_000; // $10,000 limit
    let max_count = 10;
    let window = 86_400;

    let mut tracker = TreasuryVelocityTracker::new(max_volume, max_count, window, now);

    // Transfer 1: $6,000 -> OK
    assert!(tracker.record_transfer(600_000, now).is_ok());
    assert_eq!(tracker.current_volume_cents, 600_000);
    assert_eq!(tracker.current_transaction_count, 1);

    // Transfer 2: $3,000 -> OK (total $9,000)
    assert!(tracker.record_transfer(300_000, now + 100).is_ok());
    assert_eq!(tracker.current_volume_cents, 900_000);
    assert_eq!(tracker.current_transaction_count, 2);

    // Transfer 3: $2,000 -> Exceeds limit ($11,000 > $10,000)
    let err = tracker.record_transfer(200_000, now + 200).unwrap_err();
    assert!(matches!(
        err,
        TreasuryTransferValidationError::VelocityLimitExceeded { .. }
    ));
    assert_eq!(tracker.current_volume_cents, 900_000);
    assert_eq!(tracker.current_transaction_count, 2);
}

#[test]
fn test_velocity_tracker_transaction_count_limit() {
    let now = 1728345600;
    let max_volume = 10_000_000;
    let max_count = 3;
    let window = 86_400;

    let mut tracker = TreasuryVelocityTracker::new(max_volume, max_count, window, now);

    assert!(tracker.record_transfer(1000, now).is_ok());
    assert!(tracker.record_transfer(1000, now + 10).is_ok());
    assert!(tracker.record_transfer(1000, now + 20).is_ok());

    // 4th transaction exceeds count cap
    let err = tracker.record_transfer(1000, now + 30).unwrap_err();
    assert!(matches!(
        err,
        TreasuryTransferValidationError::VelocityLimitExceeded { .. }
    ));
}

#[test]
fn test_velocity_tracker_window_refresh() {
    let now = 1728345600;
    let max_volume = 1_000_000;
    let max_count = 5;
    let window = 86_400;

    let mut tracker = TreasuryVelocityTracker::new(max_volume, max_count, window, now);
    assert!(tracker.record_transfer(900_000, now).is_ok());
    assert_eq!(tracker.current_volume_cents, 900_000);

    // Advance beyond 24 hours
    let next_day = now + 86_401;
    assert!(tracker.record_transfer(900_000, next_day).is_ok());
    assert_eq!(tracker.current_volume_cents, 900_000);
    assert_eq!(tracker.current_transaction_count, 1);
}

#[test]
fn test_event_object_discriminator() {
    let transfer_json = serde_json::json!({
        "id": "obt_discrim_1",
        "object": "treasury.outbound_transfer",
        "amount": 5000,
        "cancelable": false,
        "created": 1728345600,
        "currency": "usd",
        "expected_arrival_date": 1728432000,
        "financial_account": "fa_test",
        "livemode": false,
        "status": "posted",
        "status_transitions": {}
    });

    let obj: EventObject = serde_json::from_value(transfer_json).unwrap();
    match obj {
        EventObject::TreasuryOutboundTransfer(t) => {
            assert_eq!(t.id, "obt_discrim_1");
            assert_eq!(t.amount, 5000);
            assert_eq!(t.status, OutboundTransferStatus::Posted);
        }
        _ => panic!("Expected EventObject::TreasuryOutboundTransfer"),
    }

    let payment_json = serde_json::json!({
        "id": "obp_discrim_2",
        "object": "treasury.outbound_payment",
        "amount": 12000,
        "cancelable": true,
        "created": 1728345600,
        "currency": "usd",
        "expected_arrival_date": 1728432000,
        "financial_account": "fa_test",
        "livemode": false,
        "status": "processing",
        "status_transitions": {}
    });

    let obj2: EventObject = serde_json::from_value(payment_json).unwrap();
    match obj2 {
        EventObject::TreasuryOutboundPayment(p) => {
            assert_eq!(p.id, "obp_discrim_2");
            assert_eq!(p.amount, 12000);
            assert_eq!(p.status, OutboundPaymentStatus::Processing);
        }
        _ => panic!("Expected EventObject::TreasuryOutboundPayment"),
    }

    let credit_json = serde_json::json!({
        "id": "rc_discrim_3",
        "object": "treasury.received_credit",
        "amount": 99000,
        "created": 1728345600,
        "currency": "usd",
        "livemode": false,
        "network": "ach",
        "status": "succeeded"
    });

    let obj3: EventObject = serde_json::from_value(credit_json).unwrap();
    match obj3 {
        EventObject::TreasuryReceivedCredit(c) => {
            assert_eq!(c.id, "rc_discrim_3");
            assert_eq!(c.amount, 99000);
            assert_eq!(c.status, ReceivedCreditStatus::Succeeded);
        }
        _ => panic!("Expected EventObject::TreasuryReceivedCredit"),
    }
}

#[test]
fn test_list_params_and_collections() {
    let list_transfers = ListOutboundTransfersParams::new("fa_target")
        .with_status(OutboundTransferStatus::Posted)
        .with_limit(25);
    assert_eq!(list_transfers.financial_account, "fa_target");
    assert_eq!(list_transfers.status, Some(OutboundTransferStatus::Posted));
    assert_eq!(list_transfers.limit, Some(25));

    let list_payments = ListOutboundPaymentsParams::new("fa_target")
        .with_status(OutboundPaymentStatus::Processing)
        .with_limit(10);
    assert_eq!(list_payments.financial_account, "fa_target");
    assert_eq!(
        list_payments.status,
        Some(OutboundPaymentStatus::Processing)
    );
    assert_eq!(list_payments.limit, Some(10));

    let list_credits = ListReceivedCreditsParams::new("fa_target")
        .with_status(ReceivedCreditStatus::Succeeded)
        .with_limit(50);
    assert_eq!(list_credits.financial_account, "fa_target");
    assert_eq!(list_credits.status, Some(ReceivedCreditStatus::Succeeded));
    assert_eq!(list_credits.limit, Some(50));

    let transfer_list_json = r#"{
        "object": "list",
        "data": [],
        "has_more": false,
        "url": "/v1/treasury/outbound_transfers"
    }"#;
    let t_list: OutboundTransferList = serde_json::from_str(transfer_list_json).unwrap();
    assert_eq!(t_list.object, "list");
    assert!(t_list.data.is_empty());

    let payment_list_json = r#"{
        "object": "list",
        "data": [],
        "has_more": false,
        "url": "/v1/treasury/outbound_payments"
    }"#;
    let p_list: OutboundPaymentList = serde_json::from_str(payment_list_json).unwrap();
    assert_eq!(p_list.object, "list");

    let credit_list_json = r#"{
        "object": "list",
        "data": [],
        "has_more": false,
        "url": "/v1/treasury/received_credits"
    }"#;
    let c_list: ReceivedCreditList = serde_json::from_str(credit_list_json).unwrap();
    assert_eq!(c_list.object, "list");
}
