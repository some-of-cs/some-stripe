//! Integration Tests for Settlement Math, P2B Compliance, and Zero-Clone Borrowing Audit

use some_stripe_types::settlement::{
    P2BComplianceError, P2BFeeSchedule, SettlementSplitError,
    validate_eu_p2b_transparency_disclosure, validate_settlement_split,
};

#[test]
fn test_settlement_split_valid_scenarios() {
    // Exact split equal to charge
    assert!(validate_settlement_split(1500, 500, 2000).is_ok());
    // Transfer only (no fee)
    assert!(validate_settlement_split(2000, 0, 2000).is_ok());
    // Zero transfer and zero fee
    assert!(validate_settlement_split(0, 0, 0).is_ok());
}

#[test]
fn test_settlement_split_exceeds_total() {
    let err = validate_settlement_split(1600, 500, 2000).unwrap_err();
    match err {
        SettlementSplitError::SplitExceedsTotal {
            transfer,
            fee,
            sum,
            total,
        } => {
            assert_eq!(transfer, 1600);
            assert_eq!(fee, 500);
            assert_eq!(sum, 2100);
            assert_eq!(total, 2000);
        }
        _ => panic!("Expected SplitExceedsTotal error"),
    }
}

#[test]
fn test_settlement_split_negative_amounts() {
    assert_eq!(
        validate_settlement_split(-1, 500, 2000).unwrap_err(),
        SettlementSplitError::NegativeTransferAmount(-1)
    );
    assert_eq!(
        validate_settlement_split(1500, -5, 2000).unwrap_err(),
        SettlementSplitError::NegativeApplicationFeeAmount(-5)
    );
    assert_eq!(
        validate_settlement_split(1500, 500, -10).unwrap_err(),
        SettlementSplitError::NegativeTotalChargeAmount(-10)
    );
}

#[test]
fn test_settlement_split_overflow() {
    let err = validate_settlement_split(i64::MAX, 1, 2000).unwrap_err();
    assert_eq!(err, SettlementSplitError::ArithmeticOverflow);
}

#[test]
fn test_eu_p2b_transparency_disclosure_valid() {
    let schedule = P2BFeeSchedule {
        id: "p2b_sch_ok".into(),
        seller_fee_disclosure_explicit: true,
        dispute_cost_allocation_clear: true,
        settlement_timeframe_days: 2,
        change_notice_period_days: 15,
    };
    assert!(validate_eu_p2b_transparency_disclosure(&schedule).is_ok());
}

#[test]
fn test_eu_p2b_violations() {
    let mut schedule = P2BFeeSchedule {
        id: "p2b_sch_err".into(),
        seller_fee_disclosure_explicit: false,
        dispute_cost_allocation_clear: true,
        settlement_timeframe_days: 2,
        change_notice_period_days: 15,
    };

    // Missing seller fee disclosure
    assert_eq!(
        validate_eu_p2b_transparency_disclosure(&schedule).unwrap_err(),
        P2BComplianceError::MissingSellerFeeDisclosure
    );

    schedule.seller_fee_disclosure_explicit = true;
    schedule.dispute_cost_allocation_clear = false;
    // Unclear dispute cost allocation
    assert_eq!(
        validate_eu_p2b_transparency_disclosure(&schedule).unwrap_err(),
        P2BComplianceError::UnclearDisputeCostAllocation
    );

    schedule.dispute_cost_allocation_clear = true;
    schedule.settlement_timeframe_days = 31;
    // Settlement timeframe too long
    assert_eq!(
        validate_eu_p2b_transparency_disclosure(&schedule).unwrap_err(),
        P2BComplianceError::SettlementTimeframeTooLong { days: 31 }
    );

    schedule.settlement_timeframe_days = 3;
    schedule.change_notice_period_days = 10;
    // Notice period too short
    assert_eq!(
        validate_eu_p2b_transparency_disclosure(&schedule).unwrap_err(),
        P2BComplianceError::NoticePeriodTooShort {
            provided_days: 10,
            required_minimum: 15
        }
    );
}

#[test]
fn test_parameter_zero_clone_borrowing_audit() {
    // Verify that validate_eu_p2b_transparency_disclosure takes &P2BFeeSchedule by reference (zero-clone)
    let schedule = P2BFeeSchedule {
        id: "p2b_borrow".into(),
        seller_fee_disclosure_explicit: true,
        dispute_cost_allocation_clear: true,
        settlement_timeframe_days: 5,
        change_notice_period_days: 30,
    };

    let schedule_ref: &P2BFeeSchedule = &schedule;
    let res = validate_eu_p2b_transparency_disclosure(schedule_ref);
    assert!(res.is_ok());
    // schedule is still fully owned and usable here without cloning
    assert_eq!(schedule.id, "p2b_borrow");
}
