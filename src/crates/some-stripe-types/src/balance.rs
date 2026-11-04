//! Stripe Balance and Balance Transaction ledger domain models.
//!
//! # Gate G-Law Financial Invariants
//! - All monetary balances, transaction fees, and net settlement values strictly employ
//!   ISO 4217 minor currency integer units ([`i64`]).
//! - Fractional cent representations and floating point calculations are strictly avoided
//!   to guarantee immutable accounting balance precision.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a Stripe Balance object (`balance`).
///
/// Reflects the funds in the merchant account that are either available for payout or pending.
///
/// # Example
/// ```
/// use some_stripe_types::Balance;
///
/// let json = r#"{
///     "object": "balance",
///     "available": [{"amount": 100000, "currency": "usd"}],
///     "pending": [{"amount": 50000, "currency": "usd"}],
///     "livemode": false
/// }"#;
/// let balance: Balance = serde_json::from_str(json).unwrap();
/// assert_eq!(balance.available[0].amount, 100000);
/// assert_eq!(balance.pending[0].amount, 50000);
/// assert_eq!(balance.available[0].currency, "usd");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Balance {
    /// Object type, always `balance`.
    #[serde(default = "default_balance_object")]
    pub object: String,

    /// Funds that are available to be paid out now, broken down by currency.
    pub available: Vec<BalanceAmount>,

    /// Funds that are not yet available in the balance, pending settlement.
    pub pending: Vec<BalanceAmount>,

    /// Has the value `true` if the object exists in live mode or `false` in test mode.
    pub livemode: Option<bool>,
}

fn default_balance_object() -> String {
    "balance".into()
}

/// Balance broken down by ISO 4217 minor currency unit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BalanceAmount {
    /// Balance amount in integer minor units (e.g. cents).
    pub amount: i64,

    /// Three-letter ISO currency code in lowercase.
    pub currency: String,

    /// Breakdown of balance amounts by source type (e.g. `card`, `bank_account`).
    pub source_types: Option<HashMap<String, i64>>,
}

/// Represents a Stripe BalanceTransaction object (`txn_...`).
///
/// Balance transactions represent the movement of funds into and out of your Stripe account balance.
///
/// # Example
/// ```
/// use some_stripe_types::BalanceTransaction;
///
/// let json = r#"{
///     "id": "txn_1032Hu2eZvKYlo2CEPtcnUvl",
///     "object": "balance_transaction",
///     "amount": 100,
///     "currency": "usd",
///     "net": 69,
///     "fee": 31,
///     "status": "available",
///     "type": "charge"
/// }"#;
/// let txn: BalanceTransaction = serde_json::from_str(json).unwrap();
/// assert_eq!(txn.id, "txn_1032Hu2eZvKYlo2CEPtcnUvl");
/// assert_eq!(txn.amount, 100);
/// assert_eq!(txn.net, 69);
/// assert_eq!(txn.fee, 31);
/// assert_eq!(txn.r#type, "charge");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BalanceTransaction {
    /// Unique identifier for the balance transaction (`txn_...`).
    pub id: String,

    /// Object type, always `balance_transaction`.
    #[serde(default = "default_balance_transaction_object")]
    pub object: String,

    /// Gross amount of the transaction in minor currency units (e.g. cents).
    pub amount: i64,

    /// Three-letter ISO currency code in lowercase.
    pub currency: String,

    /// Net amount of the transaction after deducting Stripe processing fees.
    pub net: i64,

    /// Processing fee charged by Stripe for this transaction in minor units.
    pub fee: i64,

    /// Detailed breakdown of fees incurred in this transaction.
    pub fee_details: Option<Vec<FeeDetail>>,

    /// Status of the balance transaction: `available` or `pending`.
    pub status: String,

    /// Transaction type (e.g. `charge`, `refund`, `adjustment`, `transfer`, `payout`).
    #[serde(rename = "type")]
    pub r#type: String,

    /// The Stripe object that generated this transaction (e.g. `ch_...`, `re_...`).
    pub source: Option<String>,

    /// An arbitrary string attached to the object (e.g. charge description).
    pub description: Option<String>,

    /// The reporting category of this transaction for balance sheet classification.
    pub reporting_category: Option<String>,

    /// Time at which the transaction was created (UNIX epoch seconds).
    pub created: Option<i64>,

    /// The date the transaction's net funds become available in the Stripe balance.
    pub available_on: Option<i64>,
}

fn default_balance_transaction_object() -> String {
    "balance_transaction".into()
}

/// Breakdown of fees incurred in a balance transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeeDetail {
    /// Fee amount in minor currency units.
    pub amount: i64,

    /// Three-letter ISO currency code.
    pub currency: String,

    /// Type of fee (e.g. `stripe_fee`, `application_fee`).
    #[serde(rename = "type")]
    pub r#type: String,

    /// An arbitrary description of the fee.
    pub description: Option<String>,

    /// Application ID if this fee is charged by an integrated platform application.
    pub application: Option<String>,
}
