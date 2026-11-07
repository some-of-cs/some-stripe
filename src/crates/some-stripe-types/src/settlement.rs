//! # Settlement Math and EU P2B Compliance Verification
//!
//! Provides strict mathematical validation for split settlements, application fees,
//! and EU Platform-to-Business (P2B) Regulation (EU) 2019/1150 transparency disclosures.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors arising from split settlement calculations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SettlementSplitError {
    #[error("Transfer amount ({0}) cannot be negative")]
    NegativeTransferAmount(i64),

    #[error("Application fee amount ({0}) cannot be negative")]
    NegativeApplicationFeeAmount(i64),

    #[error("Total charge amount ({0}) cannot be negative")]
    NegativeTotalChargeAmount(i64),

    #[error("Split sum (transfer {transfer} + fee {fee} = {sum}) exceeds total charge amount ({total})")]
    SplitExceedsTotal {
        transfer: i64,
        fee: i64,
        sum: i64,
        total: i64,
    },

    #[error("Integer overflow encountered during split settlement arithmetic")]
    ArithmeticOverflow,
}

/// Validates that split transfer and application fee amounts do not exceed the total charge amount,
/// and that all amounts are valid non-negative ISO 4217 minor integers.
///
/// # Examples
///
/// ```
/// use some_stripe_types::settlement::validate_settlement_split;
///
/// // Transfer 1500 + fee 500 <= total 2000 is valid
/// assert!(validate_settlement_split(1500, 500, 2000).is_ok());
///
/// // Sum exceeds total is an error
/// assert!(validate_settlement_split(1600, 500, 2000).is_err());
///
/// // Negative amounts are rejected
/// assert!(validate_settlement_split(-100, 500, 2000).is_err());
/// ```
pub fn validate_settlement_split(
    transfer_amount: i64,
    application_fee_amount: i64,
    total_charge_amount: i64,
) -> Result<(), SettlementSplitError> {
    if transfer_amount < 0 {
        return Err(SettlementSplitError::NegativeTransferAmount(transfer_amount));
    }
    if application_fee_amount < 0 {
        return Err(SettlementSplitError::NegativeApplicationFeeAmount(application_fee_amount));
    }
    if total_charge_amount < 0 {
        return Err(SettlementSplitError::NegativeTotalChargeAmount(total_charge_amount));
    }

    let sum = transfer_amount
        .checked_add(application_fee_amount)
        .ok_or(SettlementSplitError::ArithmeticOverflow)?;

    if sum > total_charge_amount {
        return Err(SettlementSplitError::SplitExceedsTotal {
            transfer: transfer_amount,
            fee: application_fee_amount,
            sum,
            total: total_charge_amount,
        });
    }

    Ok(())
}

/// EU Platform-to-Business (P2B) fee schedule and transparency disclosure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct P2BFeeSchedule {
    /// Identifier for the fee schedule.
    pub id: String,
    /// Commercial seller fee disclosure present and explicit.
    pub seller_fee_disclosure_explicit: bool,
    /// Dispute and chargeback cost allocation clearly stated.
    pub dispute_cost_allocation_clear: bool,
    /// Settlement timeframe clarity in business days.
    pub settlement_timeframe_days: u32,
    /// Notice period (days) prior to any fee schedule changes (EU P2B Art. 3).
    pub change_notice_period_days: u32,
}

/// Errors arising from EU P2B compliance verification.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum P2BComplianceError {
    #[error("EU P2B Violation: Commercial seller fees must be explicitly disclosed (Regulation (EU) 2019/1150 Art. 3)")]
    MissingSellerFeeDisclosure,

    #[error("EU P2B Violation: Dispute and chargeback cost allocations must be clearly defined")]
    UnclearDisputeCostAllocation,

    #[error("EU P2B Violation: Settlement timeframe cannot exceed 30 business days")]
    SettlementTimeframeTooLong { days: u32 },

    #[error("EU P2B Violation: Fee change notice period must be at least 15 days (Art. 3(2))")]
    NoticePeriodTooShort { provided_days: u32, required_minimum: u32 },
}

/// Validates EU Platform-to-Business (P2B) Regulation (EU) 2019/1150 transparency disclosure requirements.
///
/// # Examples
///
/// ```
/// use some_stripe_types::settlement::{P2BFeeSchedule, validate_eu_p2b_transparency_disclosure};
///
/// let schedule = P2BFeeSchedule {
///     id: "p2b_sch_1".into(),
///     seller_fee_disclosure_explicit: true,
///     dispute_cost_allocation_clear: true,
///     settlement_timeframe_days: 3,
///     change_notice_period_days: 15,
/// };
///
/// assert!(validate_eu_p2b_transparency_disclosure(&schedule).is_ok());
/// ```
pub fn validate_eu_p2b_transparency_disclosure(
    fee_schedule: &P2BFeeSchedule,
) -> Result<(), P2BComplianceError> {
    if !fee_schedule.seller_fee_disclosure_explicit {
        return Err(P2BComplianceError::MissingSellerFeeDisclosure);
    }
    if !fee_schedule.dispute_cost_allocation_clear {
        return Err(P2BComplianceError::UnclearDisputeCostAllocation);
    }
    if fee_schedule.settlement_timeframe_days > 30 {
        return Err(P2BComplianceError::SettlementTimeframeTooLong {
            days: fee_schedule.settlement_timeframe_days,
        });
    }
    if fee_schedule.change_notice_period_days < 15 {
        return Err(P2BComplianceError::NoticePeriodTooShort {
            provided_days: fee_schedule.change_notice_period_days,
            required_minimum: 15,
        });
    }

    Ok(())
}
