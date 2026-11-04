//! Stripe Balance and BalanceTransaction Domain Models & Immutable Ledger Reconciliation.
//!
//! This module implements domain models for Stripe balance inquiries and balance transaction
//! ledger streams, adhering to:
//! - **Gate G-Law**: ISO 4217 minor currency integer units (strictly `i64`, zero floats).
//! - **Gate G-Borrow**: Zero-clone borrowing discipline with reference accessors and stream traversal.
//! - **Gate G2**: Immutable audit ledger reconciliation enforcing $\text{net} = \text{amount} - \text{fee}$.
//! - **Gate G-Docs**: Comprehensive rustdoc documentation with verified doc-tests.

use serde::{Deserialize, Serialize};

/// Breakdown of balance funds by payment source type (e.g. card, bank transfer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BalanceSourceTypes {
    /// Amount sourced from card payments in minor currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<i64>,

    /// Amount sourced from bank accounts in minor currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bank_account: Option<i64>,

    /// Amount sourced from FPX payments in minor currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fpx: Option<i64>,
}

/// Represents funds in a single currency within a Stripe balance.
///
/// Complies with **Gate G-Law** (ISO 4217 minor integer units, zero floating-point arithmetic).
///
/// # Example
/// ```
/// use some_stripe_types::BalanceFund;
///
/// let fund = BalanceFund::new(100_000, "usd");
/// assert_eq!(fund.amount, 100_000);
/// assert_eq!(fund.currency, "usd");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceFund {
    /// Balance amount in minor currency integer units (e.g., cents for USD, pence for GBP).
    pub amount: i64,

    /// Three-letter ISO 4217 currency code in lowercase (e.g., "usd", "eur").
    pub currency: String,

    /// Breakdown of funds by source type, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_types: Option<BalanceSourceTypes>,
}

impl BalanceFund {
    /// Creates a new `BalanceFund` with the specified minor currency integer unit amount and currency code.
    pub fn new(amount: i64, currency: impl Into<String>) -> Self {
        Self {
            amount,
            currency: currency.into(),
            source_types: None,
        }
    }
}

/// Represents a Stripe Balance object.
///
/// Contains funds that are currently available to be paid out, as well as funds
/// that have been earned but are pending availability.
///
/// # Example
/// ```
/// use some_stripe_types::{Balance, BalanceFund};
///
/// let json = r#"{
///     "object": "balance",
///     "available": [{"amount": 50000, "currency": "usd"}],
///     "pending": [{"amount": 12000, "currency": "usd"}],
///     "livemode": false
/// }"#;
/// let balance: Balance = serde_json::from_str(json).unwrap();
/// assert_eq!(balance.available.len(), 1);
/// assert_eq!(balance.available[0].amount, 50000);
/// assert_eq!(balance.pending[0].amount, 12000);
/// assert!(!balance.livemode);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Balance {
    /// Object type, always `balance`.
    #[serde(default = "default_balance_object")]
    pub object: String,

    /// Funds that are currently available to be paid out.
    #[serde(default)]
    pub available: Vec<BalanceFund>,

    /// Funds that are not yet available in the balance.
    #[serde(default)]
    pub pending: Vec<BalanceFund>,

    /// Funds that are held in reserve for Stripe Connect platforms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connect_reserved: Option<Vec<BalanceFund>>,

    /// Funds that can be paid out immediately using Instant Payouts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instant_available: Option<Vec<BalanceFund>>,

    /// Has the value `true` if the object exists in live mode or `false` in test mode.
    #[serde(default)]
    pub livemode: bool,
}

fn default_balance_object() -> String {
    "balance".into()
}

impl Default for Balance {
    fn default() -> Self {
        Self {
            object: default_balance_object(),
            available: Vec::new(),
            pending: Vec::new(),
            connect_reserved: None,
            instant_available: None,
            livemode: false,
        }
    }
}

/// Breakdown of each fee applied to a balance transaction.
///
/// Complies with **Gate G-Law** (ISO 4217 minor currency integer units).
///
/// # Example
/// ```
/// use some_stripe_types::FeeDetail;
///
/// let fee = FeeDetail::new(29, "usd", "stripe_fee");
/// assert_eq!(fee.amount, 29);
/// assert_eq!(fee.currency, "usd");
/// assert_eq!(fee.r#type, "stripe_fee");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeDetail {
    /// Amount of the fee in minor currency integer units.
    pub amount: i64,

    /// Three-letter ISO 4217 currency code in lowercase.
    pub currency: String,

    /// An arbitrary string attached to the fee object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Type of the fee, e.g. `stripe_fee`, `application_fee`, `tax`.
    pub r#type: String,

    /// ID of the Connect application that earned the fee, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application: Option<String>,
}

impl FeeDetail {
    /// Creates a new `FeeDetail`.
    pub fn new(amount: i64, currency: impl Into<String>, fee_type: impl Into<String>) -> Self {
        Self {
            amount,
            currency: currency.into(),
            description: None,
            r#type: fee_type.into(),
            application: None,
        }
    }
}

/// Represents an immutable Stripe BalanceTransaction ledger entry.
///
/// Balance transactions represent changes to your Stripe balance (charges, refunds, fees, disputes).
/// Every balance transaction must adhere to the mathematical double-entry accounting invariant:
///
/// $$ \text{net} = \text{amount} - \text{fee} $$
///
/// Complies with **Gate G-Law** (ISO 4217 integer arithmetic, zero float rounding errors)
/// and **Gate G2** (immutable audit ledger entry).
///
/// # Example
/// ```
/// use some_stripe_types::BalanceTransaction;
///
/// let json = r#"{
///     "id": "txn_1001",
///     "object": "balance_transaction",
///     "amount": 1000,
///     "available_on": 1728345600,
///     "created": 1728345600,
///     "currency": "usd",
///     "fee": 59,
///     "fee_details": [],
///     "net": 941,
///     "status": "available",
///     "type": "charge"
/// }"#;
/// let txn: BalanceTransaction = serde_json::from_str(json).unwrap();
/// assert_eq!(txn.id, "txn_1001");
/// assert_eq!(txn.net, 941);
/// assert!(txn.verify_accounting_invariant());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceTransaction {
    /// Unique identifier for the balance transaction (`txn_...`).
    pub id: String,

    /// Object type, always `balance_transaction`.
    #[serde(default = "default_balance_transaction_object")]
    pub object: String,

    /// Gross amount of the transaction in minor currency units (e.g., cents).
    /// Positive for incoming funds (charges), negative for outgoing funds (refunds, payout).
    pub amount: i64,

    /// The date the transaction's net funds will become available in the Stripe balance (UNIX epoch seconds).
    pub available_on: i64,

    /// Time at which the object was created (UNIX epoch seconds).
    pub created: i64,

    /// Three-letter ISO 4217 currency code in lowercase (e.g. `usd`, `eur`).
    pub currency: String,

    /// An arbitrary string attached to the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Fees paid for this transaction in minor currency units.
    pub fee: i64,

    /// Detailed breakdown of fees incurred in this transaction.
    #[serde(default)]
    pub fee_details: Vec<FeeDetail>,

    /// Net amount resulting from this transaction in minor currency units (`amount - fee`).
    pub net: i64,

    /// High-level categorization of the transaction for reporting (e.g. `charge`, `refund`, `dispute`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reporting_category: Option<String>,

    /// The Stripe object ID that caused this balance transaction (e.g. `ch_...`, `re_...`, `dp_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// The transaction status: `available` or `pending`.
    pub status: String,

    /// Transaction type (e.g. `charge`, `refund`, `adjustment`, `application_fee`, `transfer`, `payout`).
    pub r#type: String,
}

fn default_balance_transaction_object() -> String {
    "balance_transaction".into()
}

impl BalanceTransaction {
    /// Creates a new `BalanceTransaction` ensuring the ledger math invariant (`net = amount - fee`) is preserved.
    pub fn new(
        id: impl Into<String>,
        amount: i64,
        fee: i64,
        currency: impl Into<String>,
        txn_type: impl Into<String>,
        status: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            object: default_balance_transaction_object(),
            amount,
            available_on: 0,
            created: 0,
            currency: currency.into(),
            description: None,
            fee,
            fee_details: Vec::new(),
            net: amount - fee,
            reporting_category: None,
            source: None,
            status: status.into(),
            r#type: txn_type.into(),
        }
    }

    /// Verifies the fundamental double-entry ledger accounting invariant:
    /// `net == amount - fee`.
    ///
    /// # Mathematical Invariant
    /// In an immutable financial ledger, the net funds deposited into or withdrawn from
    /// an account must equal the gross amount minus all fees.
    ///
    /// $$ \text{net} = \text{amount} - \text{fee} $$
    ///
    /// If `self.net != self.amount - self.fee`, this function returns `false`, signaling
    /// a ledger invariant violation or arithmetic corruption.
    ///
    /// # Example
    /// ```
    /// use some_stripe_types::BalanceTransaction;
    ///
    /// let txn = BalanceTransaction {
    ///     id: "txn_1001".into(),
    ///     object: "balance_transaction".into(),
    ///     amount: 5000,
    ///     available_on: 1728345600,
    ///     created: 1728345600,
    ///     currency: "usd".into(),
    ///     description: Some("Charge deposit".into()),
    ///     fee: 175,
    ///     fee_details: vec![],
    ///     net: 4825,
    ///     reporting_category: Some("charge".into()),
    ///     source: Some("ch_1001".into()),
    ///     status: "available".into(),
    ///     r#type: "charge".into(),
    /// };
    /// assert!(txn.verify_accounting_invariant());
    /// ```
    #[inline]
    pub fn verify_accounting_invariant(&self) -> bool {
        self.net == self.amount - self.fee
    }
}

/// Validates an entire batch or ledger stream of balance transactions against the
/// immutable accounting invariant `net == amount - fee`.
///
/// Checks both each individual transaction invariant and the overall aggregate sums
/// across all transactions using 128-bit integer arithmetic to prevent overflow.
///
/// # Errors
/// Returns an `Err(String)` containing descriptive diagnostic details if any transaction
/// or the overall ledger aggregate violates the invariant.
///
/// # Example
/// ```
/// use some_stripe_types::{BalanceTransaction, validate_ledger_reconciliation};
///
/// let txns = vec![
///     BalanceTransaction {
///         id: "txn_1".into(),
///         object: "balance_transaction".into(),
///         amount: 1000,
///         available_on: 1728345600,
///         created: 1728345600,
///         currency: "usd".into(),
///         description: None,
///         fee: 30,
///         fee_details: vec![],
///         net: 970,
///         reporting_category: None,
///         source: None,
///         status: "available".into(),
///         r#type: "charge".into(),
///     },
///     BalanceTransaction {
///         id: "txn_2".into(),
///         object: "balance_transaction".into(),
///         amount: -1000,
///         available_on: 1728345600,
///         created: 1728345600,
///         currency: "usd".into(),
///         description: None,
///         fee: 0,
///         fee_details: vec![],
///         net: -1000,
///         reporting_category: None,
///         source: None,
///         status: "available".into(),
///         r#type: "refund".into(),
///     },
/// ];
/// assert!(validate_ledger_reconciliation(&txns).is_ok());
/// ```
pub fn validate_ledger_reconciliation(txns: &[BalanceTransaction]) -> Result<(), String> {
    let mut total_amount: i128 = 0;
    let mut total_fee: i128 = 0;
    let mut total_net: i128 = 0;

    for (index, txn) in txns.iter().enumerate() {
        if !txn.verify_accounting_invariant() {
            return Err(format!(
                "Ledger reconciliation invariant violation at index {} [id: '{}']: net ({}) != amount ({}) - fee ({}) [expected net: {}]",
                index,
                txn.id,
                txn.net,
                txn.amount,
                txn.fee,
                txn.amount - txn.fee
            ));
        }
        total_amount += txn.amount as i128;
        total_fee += txn.fee as i128;
        total_net += txn.net as i128;
    }

    if total_net != total_amount - total_fee {
        return Err(format!(
            "Aggregate ledger reconciliation invariant violation: total_net ({}) != total_amount ({}) - total_fee ({})",
            total_net, total_amount, total_fee
        ));
    }

    Ok(())
}

/// Helper ID extractor function for seamless integration with zero-clone stream pagination
/// (`some_stripe_core::patterns::pagination::Paginator`).
///
/// Conforms strictly to the `fn(&T) -> &str` function pointer signature required by `Paginator::new`.
///
/// # Example
/// ```
/// use some_stripe_types::{BalanceTransaction, balance_transaction_id};
///
/// let txn = BalanceTransaction {
///     id: "txn_stream_99".into(),
///     object: "balance_transaction".into(),
///     amount: 2500,
///     available_on: 1728345600,
///     created: 1728345600,
///     currency: "usd".into(),
///     description: None,
///     fee: 75,
///     fee_details: vec![],
///     net: 2425,
///     reporting_category: None,
///     source: None,
///     status: "available".into(),
///     r#type: "charge".into(),
/// };
/// assert_eq!(balance_transaction_id(&txn), "txn_stream_99");
/// ```
#[inline]
pub fn balance_transaction_id(txn: &BalanceTransaction) -> &str {
    &txn.id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_transaction_accounting_invariant_valid() {
        let txn = BalanceTransaction {
            id: "txn_test_1".into(),
            object: "balance_transaction".into(),
            amount: 10000,
            available_on: 1728345600,
            created: 1728345600,
            currency: "usd".into(),
            description: Some("Order #1001 payment".into()),
            fee: 320,
            fee_details: vec![FeeDetail::new(320, "usd", "stripe_fee")],
            net: 9680,
            reporting_category: Some("charge".into()),
            source: Some("ch_test_1".into()),
            status: "available".into(),
            r#type: "charge".into(),
        };

        assert!(txn.verify_accounting_invariant());
        assert_eq!(balance_transaction_id(&txn), "txn_test_1");
    }

    #[test]
    fn test_balance_transaction_accounting_invariant_invalid() {
        let txn = BalanceTransaction {
            id: "txn_corrupted".into(),
            object: "balance_transaction".into(),
            amount: 10000,
            available_on: 1728345600,
            created: 1728345600,
            currency: "usd".into(),
            description: None,
            fee: 320,
            fee_details: vec![],
            net: 9000, // Should be 9680!
            reporting_category: None,
            source: None,
            status: "available".into(),
            r#type: "charge".into(),
        };

        assert!(!txn.verify_accounting_invariant());
        let err = validate_ledger_reconciliation(&[txn]).unwrap_err();
        assert!(err.contains("txn_corrupted"));
        assert!(err.contains("expected net: 9680"));
    }

    #[test]
    fn test_balance_transaction_refund_invariant() {
        // Refund: amount is negative, fee is refunded or 0, net is negative
        let refund_txn = BalanceTransaction {
            id: "txn_refund_1".into(),
            object: "balance_transaction".into(),
            amount: -5000,
            available_on: 1728345600,
            created: 1728345600,
            currency: "eur".into(),
            description: Some("Customer return".into()),
            fee: 0,
            fee_details: vec![],
            net: -5000,
            reporting_category: Some("refund".into()),
            source: Some("re_test_1".into()),
            status: "available".into(),
            r#type: "refund".into(),
        };

        assert!(refund_txn.verify_accounting_invariant());
        assert!(validate_ledger_reconciliation(&[refund_txn]).is_ok());
    }

    #[test]
    fn test_balance_transaction_dispute_adjustment_invariant() {
        // Dispute fee: amount is 0 or negative, dispute fee is deducted
        let dispute_txn = BalanceTransaction {
            id: "txn_dispute_fee_1".into(),
            object: "balance_transaction".into(),
            amount: -1500, // $15 dispute fee
            available_on: 1728345600,
            created: 1728345600,
            currency: "usd".into(),
            description: Some("Dispute fee".into()),
            fee: 0,
            fee_details: vec![],
            net: -1500,
            reporting_category: Some("dispute".into()),
            source: Some("dp_test_1".into()),
            status: "available".into(),
            r#type: "adjustment".into(),
        };

        assert!(dispute_txn.verify_accounting_invariant());
        assert!(validate_ledger_reconciliation(&[dispute_txn]).is_ok());
    }

    #[test]
    fn test_balance_serde_roundtrip() {
        let json = serde_json::json!({
            "object": "balance",
            "available": [
                {
                    "amount": 250000,
                    "currency": "usd",
                    "source_types": {
                        "card": 200000,
                        "bank_account": 50000
                    }
                }
            ],
            "pending": [
                {
                    "amount": 10000,
                    "currency": "usd"
                }
            ],
            "connect_reserved": [
                {
                    "amount": 5000,
                    "currency": "usd"
                }
            ],
            "instant_available": [
                {
                    "amount": 150000,
                    "currency": "usd"
                }
            ],
            "livemode": true
        });

        let balance: Balance = serde_json::from_value(json).expect("Failed to deserialize balance");
        assert_eq!(balance.available.len(), 1);
        assert_eq!(balance.available[0].amount, 250000);
        assert_eq!(balance.available[0].currency, "usd");
        assert_eq!(
            balance.available[0]
                .source_types
                .as_ref()
                .and_then(|s| s.card),
            Some(200000)
        );
        assert!(balance.livemode);

        let serialized = serde_json::to_string(&balance).expect("Serialization failed");
        let parsed_back: Balance =
            serde_json::from_str(&serialized).expect("Deserialization back failed");
        assert_eq!(balance, parsed_back);
    }
}
