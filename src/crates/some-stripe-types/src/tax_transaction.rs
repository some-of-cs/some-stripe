//! # Stripe Tax Transaction Types & Domain Invariants
//!
//! Strongly typed domain models, parameters, and statutory ledger invariants for Stripe Tax Transactions,
//! adhering to the 48-hour calculation expiration lifecycle and immutable double-entry reversal balance rules.
//!
//! ## Overview
//!
//! - **Domain Entities**: [`TaxTransaction`], [`TaxTransactionLineItem`], [`TaxTransactionType`],
//!   [`ReversalDetails`], [`TaxShippingCost`], [`TaxCustomerDetails`].
//! - **Request Parameters**: [`CreateTaxTransactionFromCalculationParams`],
//!   [`CreateTaxTransactionReversalParams`], [`ListTaxTransactionLineItemsParams`].
//! - **Calculation Expiration Invariant**: Tax calculations expire after 48 hours (172,800 seconds).
//!   [`is_tax_calculation_valid`] enforces this lifecycle constraint before committing to the ledger.
//! - **Reversal Balance Adjustments**: [`calculate_reversal_balance_adjustment`] calculates
//!   exact integer ledger adjustments when reversing transactions without floating-point inaccuracies.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// The expiration duration for Stripe Tax calculations: 48 hours in seconds (172,800 seconds).
///
/// Under Stripe Tax domain rules, calculations must be committed into transactions
/// within 48 hours of calculation creation.
pub const TAX_CALCULATION_EXPIRATION_SECONDS: i64 = 48 * 3600;

/// The type of a tax transaction (`transaction` or `reversal`).
///
/// # Example
/// ```
/// use some_stripe_types::TaxTransactionType;
/// use std::str::FromStr;
///
/// let tx_type = TaxTransactionType::Transaction;
/// assert_eq!(tx_type.as_str(), "transaction");
/// assert_eq!(tx_type.to_string(), "transaction");
///
/// let parsed = TaxTransactionType::from_str("reversal").unwrap();
/// assert_eq!(parsed, TaxTransactionType::Reversal);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaxTransactionType {
    /// A standard tax transaction committing tax liabilities.
    Transaction,
    /// A reversal tax transaction adjusting or refunding tax liabilities.
    Reversal,
}

impl TaxTransactionType {
    /// Returns the static string slice representation for this transaction type.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Transaction => "transaction",
            Self::Reversal => "reversal",
        }
    }
}

impl std::fmt::Display for TaxTransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for TaxTransactionType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Error returned when parsing an invalid [`TaxTransactionType`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseTaxTransactionTypeError(pub String);

impl std::fmt::Display for ParseTaxTransactionTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid tax transaction type: '{}'", self.0)
    }
}

impl std::error::Error for ParseTaxTransactionTypeError {}

impl FromStr for TaxTransactionType {
    type Err = ParseTaxTransactionTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "transaction" => Ok(Self::Transaction),
            "reversal" => Ok(Self::Reversal),
            _ => Err(ParseTaxTransactionTypeError(s.to_string())),
        }
    }
}

/// The mode of a tax transaction reversal: full or partial.
///
/// # Example
/// ```
/// use some_stripe_types::ReversalMode;
///
/// let mode = ReversalMode::Full;
/// assert_eq!(mode.as_str(), "full");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReversalMode {
    /// Fully reverses the original transaction and all associated line items.
    Full,
    /// Partially reverses specific line item amounts or a flat amount.
    Partial,
}

impl ReversalMode {
    /// Returns the static string slice representation for this reversal mode.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Partial => "partial",
        }
    }
}

impl std::fmt::Display for ReversalMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for ReversalMode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Details of the original transaction when this tax transaction represents a reversal.
///
/// # Example
/// ```
/// use some_stripe_types::ReversalDetails;
///
/// let rev = ReversalDetails {
///     original_transaction: Some("tax_tx_original_123".into()),
///     mode: Some("full".into()),
/// };
/// assert_eq!(rev.original_transaction.as_deref(), Some("tax_tx_original_123"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReversalDetails {
    /// The ID of the original Tax Transaction that was reversed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_transaction: Option<String>,
    /// The mode of the reversal: `full` or `partial`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

pub use crate::tax_calculation::TaxShippingCost;

/// Customer tax-relevant details associated with a tax transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaxCustomerDetails {
    /// The customer's postal address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<crate::Address>,
    /// The source of the address (`billing` or `shipping`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_source: Option<String>,
    /// The customer's IP address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    /// Customer taxability override (`customer_exempt`, `none`, `reverse_charge`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taxability_override: Option<String>,
}

fn default_tax_transaction_line_item_object() -> String {
    "tax.transaction_line_item".into()
}

/// A line item within a Stripe Tax Transaction (`tax.transaction_line_item`).
///
/// # Example
/// ```
/// use some_stripe_types::TaxTransactionLineItem;
///
/// let item = TaxTransactionLineItem {
///     id: "tax_li_123".into(),
///     object: "tax.transaction_line_item".into(),
///     amount: 10000,
///     amount_tax: 825,
///     livemode: false,
///     metadata: None,
///     product: Some("prod_sample".into()),
///     quantity: Some(1),
///     reference: Some("sku_shoe_001".into()),
///     reversal: None,
///     tax_behavior: Some("exclusive".into()),
///     tax_code: Some("txcd_99999999".into()),
///     item_type: Some("transaction".into()),
/// };
/// assert_eq!(item.id, "tax_li_123");
/// assert_eq!(item.amount, 10000);
/// assert_eq!(item.amount_tax, 825);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxTransactionLineItem {
    /// Unique identifier for the tax transaction line item (e.g. `tax_li_123`).
    pub id: String,
    /// Object type descriptor. Always `"tax.transaction_line_item"`.
    #[serde(default = "default_tax_transaction_line_item_object")]
    pub object: String,
    /// The line item amount in minor integer currency units.
    pub amount: i64,
    /// The calculated tax amount in minor integer currency units.
    pub amount_tax: i64,
    /// Whether this line item exists in live mode or test mode.
    #[serde(default)]
    pub livemode: bool,
    /// Set of key-value pairs attached to the line item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Stripe Product ID associated with this line item, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
    /// Number of units of the item being purchased.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i64>,
    /// Your own reference identifier for this line item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Reversal details if this line item was part of a reversal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reversal: Option<ReversalDetails>,
    /// Specifies whether the tax is `"inclusive"` or `"exclusive"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_behavior: Option<String>,
    /// The Stripe Tax Code used to categorize this item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_code: Option<String>,
    /// Line item type (e.g. `"transaction"` or `"reversal"`).
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub item_type: Option<String>,
}

/// Helper function extracting the unique ID reference from a [`TaxTransactionLineItem`].
///
/// Conforms to the `fn(&T) -> &str` function pointer signature required by `Paginator::new`.
#[must_use]
pub fn tax_transaction_line_item_id(item: &TaxTransactionLineItem) -> &str {
    &item.id
}

fn default_tax_transaction_object() -> String {
    "tax.transaction".into()
}

/// Represents a Stripe Tax Transaction (`tax.transaction`).
///
/// Tax Transactions record official tax liabilities on Stripe's reporting ledgers
/// after a payment is collected against a Tax Calculation, or record adjustments
/// and reversals when refunds occur.
///
/// # Example
/// ```
/// use some_stripe_types::{TaxTransaction, TaxTransactionType};
///
/// let tx = TaxTransaction {
///     id: "tax_tx_123".into(),
///     object: "tax.transaction".into(),
///     created: 1_700_000_000,
///     currency: "usd".into(),
///     customer: Some("cus_123".into()),
///     customer_details: None,
///     livemode: false,
///     metadata: None,
///     posted_at: 1_700_000_000,
///     reference: "order_12345".into(),
///     reversal: None,
///     shipping_cost: None,
///     tax_date: 1_700_000_000,
///     transaction_type: TaxTransactionType::Transaction,
/// };
/// assert_eq!(tx.id, "tax_tx_123");
/// assert!(tx.is_standard_transaction());
/// assert!(!tx.is_reversal());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxTransaction {
    /// Unique identifier for the tax transaction (e.g. `tax_tx_123`).
    pub id: String,
    /// Object type descriptor. Always `"tax.transaction"`.
    #[serde(default = "default_tax_transaction_object")]
    pub object: String,
    /// Time at which the transaction was created (UNIX timestamp).
    pub created: i64,
    /// Three-letter ISO currency code, in lowercase.
    pub currency: String,
    /// The customer ID if associated with a Stripe customer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,
    /// Customer tax-relevant details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer_details: Option<TaxCustomerDetails>,
    /// Whether the transaction exists in live mode or test mode.
    #[serde(default)]
    pub livemode: bool,
    /// Set of key-value pairs attached to the transaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Timestamp when the tax liability was posted.
    pub posted_at: i64,
    /// Unique merchant reference for the transaction.
    pub reference: String,
    /// Reversal details if this transaction is a reversal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reversal: Option<ReversalDetails>,
    /// Shipping cost details for the transaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping_cost: Option<TaxShippingCost>,
    /// Timestamp of the tax calculation using rules in effect at that time.
    pub tax_date: i64,
    /// The type of transaction (`transaction` or `reversal`).
    #[serde(rename = "type")]
    pub transaction_type: TaxTransactionType,
}

impl TaxTransaction {
    /// Checks whether this transaction represents a reversal.
    #[must_use]
    pub fn is_reversal(&self) -> bool {
        self.transaction_type == TaxTransactionType::Reversal
    }

    /// Checks whether this transaction represents a standard tax liability transaction.
    #[must_use]
    pub fn is_standard_transaction(&self) -> bool {
        self.transaction_type == TaxTransactionType::Transaction
    }
}

/// Validates whether a tax calculation is within its 48-hour expiration window.
///
/// In accordance with Stripe Tax domain invariants, tax calculations expire 48 hours
/// (172,800 seconds) after generation. Committing an expired calculation to the ledger
/// will be rejected by Stripe Tax.
///
/// # Arguments
/// * `calculation_timestamp` - The UNIX timestamp when the calculation was created.
/// * `current_timestamp` - The UNIX timestamp when transaction creation is attempted.
///
/// # Returns
/// `true` if `current_timestamp >= calculation_timestamp` and
/// `current_timestamp - calculation_timestamp <= TAX_CALCULATION_EXPIRATION_SECONDS`.
///
/// # Example
/// ```
/// use some_stripe_types::{TAX_CALCULATION_EXPIRATION_SECONDS, is_tax_calculation_valid};
///
/// let created = 1_700_000_000;
/// // Valid within 48 hours
/// assert!(is_tax_calculation_valid(created, created + 24 * 3600));
/// // Exactly at 48 hours boundary
/// assert!(is_tax_calculation_valid(created, created + TAX_CALCULATION_EXPIRATION_SECONDS));
/// // Expired after 48 hours
/// assert!(!is_tax_calculation_valid(created, created + TAX_CALCULATION_EXPIRATION_SECONDS + 1));
/// // Invalid if current timestamp antedates calculation timestamp
/// assert!(!is_tax_calculation_valid(created, created - 100));
/// ```
#[must_use]
pub fn is_tax_calculation_valid(calculation_timestamp: i64, current_timestamp: i64) -> bool {
    if current_timestamp < calculation_timestamp {
        return false;
    }
    current_timestamp - calculation_timestamp <= TAX_CALCULATION_EXPIRATION_SECONDS
}

/// Evaluates whether a tax calculation has expired given its creation timestamp.
///
/// # Example
/// ```
/// use some_stripe_types::is_tax_calculation_expired;
///
/// let created = 1_700_000_000;
/// assert!(!is_tax_calculation_expired(created, created + 3600));
/// assert!(is_tax_calculation_expired(created, created + 200_000));
/// ```
#[must_use]
pub fn is_tax_calculation_expired(calculation_timestamp: i64, current_timestamp: i64) -> bool {
    !is_tax_calculation_valid(calculation_timestamp, current_timestamp)
}

/// Summary of a tax ledger balance adjustment resulting from a transaction reversal.
///
/// Implements exact integer arithmetic conforming to ISO 4217 minor currency units (Gate G-Law).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReversalBalanceAdjustment {
    /// Original gross transaction amount in minor currency units.
    pub original_amount: i64,
    /// Original tax liability in minor currency units.
    pub original_tax: i64,
    /// Amount refunded/reversed in minor currency units.
    pub reversed_amount: i64,
    /// Tax refunded/reversed in minor currency units.
    pub reversed_tax: i64,
    /// Net remaining transaction amount after reversal adjustment.
    pub remaining_amount: i64,
    /// Net remaining tax liability after reversal adjustment.
    pub remaining_tax: i64,
}

/// Error returned when reversal balance adjustment invariants are violated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReversalBalanceError {
    /// Reversal amount exceeds original gross amount.
    AmountExceedsOriginal { original: i64, reversed: i64 },
    /// Reversal tax liability exceeds original tax liability.
    TaxExceedsOriginal { original: i64, reversed: i64 },
    /// Negative original amount or tax value.
    InvalidOriginalAmounts,
    /// Negative reversal magnitude.
    InvalidReversalMagnitude,
}

impl std::fmt::Display for ReversalBalanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AmountExceedsOriginal { original, reversed } => {
                write!(
                    f,
                    "Reversal amount ({reversed}) exceeds original amount ({original})"
                )
            }
            Self::TaxExceedsOriginal { original, reversed } => {
                write!(
                    f,
                    "Reversal tax ({reversed}) exceeds original tax ({original})"
                )
            }
            Self::InvalidOriginalAmounts => {
                f.write_str("Original amount and tax must be non-negative")
            }
            Self::InvalidReversalMagnitude => {
                f.write_str("Reversal amount and tax magnitude must be non-negative")
            }
        }
    }
}

impl std::error::Error for ReversalBalanceError {}

/// Computes and validates a tax ledger balance adjustment for a reversal against an original transaction.
///
/// Enforces immutable double-entry ledger invariants:
/// 1. `original_amount >= 0` and `original_tax >= 0`.
/// 2. `reversed_amount >= 0` and `reversed_tax >= 0` (normalized magnitude).
/// 3. `reversed_amount <= original_amount` and `reversed_tax <= original_tax`.
/// 4. $\text{remaining\_amount} = \text{original\_amount} - \text{reversed\_amount}$.
/// 5. $\text{remaining\_tax} = \text{original\_tax} - \text{reversed\_tax}$.
///
/// # Arguments
/// * `original_amount` - The gross transaction amount in minor currency units.
/// * `original_tax` - The original tax liability in minor currency units.
/// * `reversed_amount` - The magnitude of the gross amount reversed in minor currency units.
/// * `reversed_tax` - The magnitude of the tax reversed in minor currency units.
///
/// # Example
/// ```
/// use some_stripe_types::calculate_reversal_balance_adjustment;
///
/// let adj = calculate_reversal_balance_adjustment(10000, 825, 4000, 330).unwrap();
/// assert_eq!(adj.remaining_amount, 6000);
/// assert_eq!(adj.remaining_tax, 495);
/// ```
pub fn calculate_reversal_balance_adjustment(
    original_amount: i64,
    original_tax: i64,
    reversed_amount: i64,
    reversed_tax: i64,
) -> Result<ReversalBalanceAdjustment, ReversalBalanceError> {
    if original_amount < 0 || original_tax < 0 {
        return Err(ReversalBalanceError::InvalidOriginalAmounts);
    }
    if reversed_amount < 0 || reversed_tax < 0 {
        return Err(ReversalBalanceError::InvalidReversalMagnitude);
    }
    if reversed_amount > original_amount {
        return Err(ReversalBalanceError::AmountExceedsOriginal {
            original: original_amount,
            reversed: reversed_amount,
        });
    }
    if reversed_tax > original_tax {
        return Err(ReversalBalanceError::TaxExceedsOriginal {
            original: original_tax,
            reversed: reversed_tax,
        });
    }

    Ok(ReversalBalanceAdjustment {
        original_amount,
        original_tax,
        reversed_amount,
        reversed_tax,
        remaining_amount: original_amount - reversed_amount,
        remaining_tax: original_tax - reversed_tax,
    })
}

/// Parameters for creating a tax transaction from a calculation (`POST /v1/tax/transactions/create_from_calculation`).
///
/// # Example
/// ```
/// use some_stripe_types::CreateTaxTransactionFromCalculationParams;
///
/// let params = CreateTaxTransactionFromCalculationParams::new("taxcalc_123", "order_ref_456")
///     .with_posted_at(1_700_000_000)
///     .with_idempotency_key("tx_idem_999");
///
/// assert_eq!(params.calculation, "taxcalc_123");
/// assert_eq!(params.reference, "order_ref_456");
/// assert_eq!(params.posted_at, Some(1_700_000_000));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTaxTransactionFromCalculationParams {
    /// The ID of the Tax Calculation object (e.g. `taxcalc_123`).
    pub calculation: String,
    /// A unique custom order or sale identifier.
    pub reference: String,
    /// The Unix timestamp representing when the tax liability was assumed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub posted_at: Option<i64>,
    /// Key-value pairs for storing custom metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Unique idempotency key for safe retries (Gate G2). Not serialized in request body.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateTaxTransactionFromCalculationParams {
    /// Creates a new `CreateTaxTransactionFromCalculationParams` with the required calculation and reference.
    #[must_use]
    pub fn new(calculation: impl Into<String>, reference: impl Into<String>) -> Self {
        Self {
            calculation: calculation.into(),
            reference: reference.into(),
            posted_at: None,
            metadata: None,
            idempotency_key: None,
        }
    }

    /// Sets the timestamp when the tax liability is assumed.
    #[must_use]
    pub fn with_posted_at(mut self, posted_at: i64) -> Self {
        self.posted_at = Some(posted_at);
        self
    }

    /// Sets metadata key-value pairs.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets the idempotency key for safe retries (Gate G2).
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for reversing shipping cost in a tax transaction reversal.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReversalShippingCostParams {
    /// The amount of shipping to reverse in minor integer currency units.
    pub amount: i64,
    /// The tax amount of shipping to reverse in minor integer currency units.
    pub amount_tax: i64,
}

/// Parameters for a line item reversal within a partial tax transaction reversal.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReversalLineItemParams {
    /// The amount to reverse in minor integer currency units.
    pub amount: i64,
    /// The tax amount to reverse in minor integer currency units.
    pub amount_tax: i64,
    /// The ID of the original line item to reverse.
    pub original_line_item: String,
    /// Unique reference identifier for this line item reversal.
    pub reference: String,
    /// Set of key-value pairs attached to the line item reversal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

/// Parameters for creating a tax transaction reversal (`POST /v1/tax/transactions/create_reversal`).
///
/// # Example
/// ```
/// use some_stripe_types::CreateTaxTransactionReversalParams;
///
/// // Full reversal
/// let full = CreateTaxTransactionReversalParams::full("tax_tx_123", "refund_ref_456")
///     .with_idempotency_key("rev_idem_001");
/// assert_eq!(full.mode, "full");
/// assert_eq!(full.original_transaction, "tax_tx_123");
///
/// // Partial reversal with flat amount
/// let partial = CreateTaxTransactionReversalParams::partial("tax_tx_123", "refund_ref_457")
///     .with_flat_amount(2500);
/// assert_eq!(partial.mode, "partial");
/// assert_eq!(partial.flat_amount, Some(2500));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTaxTransactionReversalParams {
    /// The ID of the original Tax Transaction to reverse.
    pub original_transaction: String,
    /// The mode of the reversal: `"full"` or `"partial"`.
    pub mode: String,
    /// A unique custom identifier for this reversal transaction.
    pub reference: String,
    /// A flat amount to reverse when `mode = "partial"` without itemized line items.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flat_amount: Option<i64>,
    /// Line items to partially reverse.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_items: Option<Vec<ReversalLineItemParams>>,
    /// Shipping cost reversal details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping_cost: Option<ReversalShippingCostParams>,
    /// Key-value pairs for storing custom metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Unique idempotency key for safe retries (Gate G2). Not serialized in request body.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateTaxTransactionReversalParams {
    /// Constructs a full transaction reversal parameter set.
    #[must_use]
    pub fn full(original_transaction: impl Into<String>, reference: impl Into<String>) -> Self {
        Self {
            original_transaction: original_transaction.into(),
            mode: "full".into(),
            reference: reference.into(),
            flat_amount: None,
            line_items: None,
            shipping_cost: None,
            metadata: None,
            idempotency_key: None,
        }
    }

    /// Constructs a partial transaction reversal parameter set.
    #[must_use]
    pub fn partial(original_transaction: impl Into<String>, reference: impl Into<String>) -> Self {
        Self {
            original_transaction: original_transaction.into(),
            mode: "partial".into(),
            reference: reference.into(),
            flat_amount: None,
            line_items: None,
            shipping_cost: None,
            metadata: None,
            idempotency_key: None,
        }
    }

    /// Sets a flat reversal amount in minor integer currency units.
    #[must_use]
    pub fn with_flat_amount(mut self, amount: i64) -> Self {
        self.flat_amount = Some(amount);
        self
    }

    /// Sets itemized line item reversal parameters.
    #[must_use]
    pub fn with_line_items(mut self, items: Vec<ReversalLineItemParams>) -> Self {
        self.line_items = Some(items);
        self
    }

    /// Sets shipping cost reversal parameters.
    #[must_use]
    pub fn with_shipping_cost(mut self, shipping: ReversalShippingCostParams) -> Self {
        self.shipping_cost = Some(shipping);
        self
    }

    /// Sets metadata key-value pairs.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets the idempotency key for safe retries (Gate G2).
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for listing line items of a tax transaction (`GET /v1/tax/transactions/{id}/line_items`).
///
/// # Example
/// ```
/// use some_stripe_types::ListTaxTransactionLineItemsParams;
///
/// let params = ListTaxTransactionLineItemsParams::new()
///     .with_limit(25)
///     .with_starting_after("tax_li_prev");
///
/// assert_eq!(params.limit, Some(25));
/// assert_eq!(params.starting_after.as_deref(), Some("tax_li_prev"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListTaxTransactionLineItemsParams {
    /// Limit on the number of objects to be returned (between 1 and 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Cursor for pagination: fetch items created after this ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,
    /// Cursor for pagination: fetch items created before this ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListTaxTransactionLineItemsParams {
    /// Creates a new default `ListTaxTransactionLineItemsParams`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum page limit (between 1 and 100).
    #[must_use]
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the cursor to fetch items created after the specified ID.
    #[must_use]
    pub fn with_starting_after(mut self, id: impl Into<String>) -> Self {
        self.starting_after = Some(id.into());
        self
    }

    /// Sets the cursor to fetch items created before the specified ID.
    #[must_use]
    pub fn with_ending_before(mut self, id: impl Into<String>) -> Self {
        self.ending_before = Some(id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tax_calculation_expiration_invariants() {
        let base_ts = 1_700_000_000;

        // Fresh calculation (10 seconds later)
        assert!(is_tax_calculation_valid(base_ts, base_ts + 10));
        assert!(!is_tax_calculation_expired(base_ts, base_ts + 10));

        // Exactly at 48-hour expiration threshold
        let exact_threshold = base_ts + TAX_CALCULATION_EXPIRATION_SECONDS;
        assert!(is_tax_calculation_valid(base_ts, exact_threshold));
        assert!(!is_tax_calculation_expired(base_ts, exact_threshold));

        // 1 second past 48-hour expiration threshold
        let expired_ts = exact_threshold + 1;
        assert!(!is_tax_calculation_valid(base_ts, expired_ts));
        assert!(is_tax_calculation_expired(base_ts, expired_ts));

        // In the future relative to current time (invalid antedated creation)
        assert!(!is_tax_calculation_valid(base_ts, base_ts - 50));
        assert!(is_tax_calculation_expired(base_ts, base_ts - 50));
    }

    #[test]
    fn test_reversal_balance_adjustment_full() {
        let original_gross = 12500; // $125.00
        let original_tax = 1031; // $10.31

        // Full reversal
        let adj = calculate_reversal_balance_adjustment(
            original_gross,
            original_tax,
            original_gross,
            original_tax,
        )
        .expect("Full reversal adjustment must succeed");

        assert_eq!(adj.original_amount, original_gross);
        assert_eq!(adj.original_tax, original_tax);
        assert_eq!(adj.reversed_amount, original_gross);
        assert_eq!(adj.reversed_tax, original_tax);
        assert_eq!(adj.remaining_amount, 0);
        assert_eq!(adj.remaining_tax, 0);
    }

    #[test]
    fn test_reversal_balance_adjustment_partial() {
        let original_gross = 10000;
        let original_tax = 825;

        // Partial reversal: $40.00 gross, $3.30 tax
        let adj = calculate_reversal_balance_adjustment(original_gross, original_tax, 4000, 330)
            .expect("Partial reversal adjustment must succeed");

        assert_eq!(adj.original_amount, 10000);
        assert_eq!(adj.original_tax, 825);
        assert_eq!(adj.reversed_amount, 4000);
        assert_eq!(adj.reversed_tax, 330);
        assert_eq!(adj.remaining_amount, 6000);
        assert_eq!(adj.remaining_tax, 495);
    }

    #[test]
    fn test_reversal_balance_adjustment_errors() {
        // Reversed amount exceeds original
        let err = calculate_reversal_balance_adjustment(1000, 100, 1500, 100)
            .expect_err("Must error when reversal amount exceeds original");
        assert!(matches!(
            err,
            ReversalBalanceError::AmountExceedsOriginal { .. }
        ));

        // Reversed tax exceeds original
        let err = calculate_reversal_balance_adjustment(1000, 100, 500, 150)
            .expect_err("Must error when reversal tax exceeds original");
        assert!(matches!(
            err,
            ReversalBalanceError::TaxExceedsOriginal { .. }
        ));

        // Negative original amounts
        let err = calculate_reversal_balance_adjustment(-1000, 100, 500, 50)
            .expect_err("Must error on negative original amount");
        assert_eq!(err, ReversalBalanceError::InvalidOriginalAmounts);

        // Negative reversal magnitudes
        let err = calculate_reversal_balance_adjustment(1000, 100, -500, 50)
            .expect_err("Must error on negative reversal amount");
        assert_eq!(err, ReversalBalanceError::InvalidReversalMagnitude);
    }

    #[test]
    fn test_tax_transaction_serde_wire_roundtrip() {
        let json = serde_json::json!({
            "id": "tax_tx_sample_123",
            "object": "tax.transaction",
            "created": 1735689600,
            "currency": "usd",
            "customer": "cus_tax_client_99",
            "customer_details": {
                "address": {
                    "city": "San Francisco",
                    "country": "US",
                    "line1": "123 Market St",
                    "postal_code": "94103",
                    "state": "CA"
                },
                "address_source": "shipping",
                "ip_address": "192.0.2.1",
                "taxability_override": "none"
            },
            "livemode": false,
            "metadata": {
                "order_id": "ORD-2026-99"
            },
            "posted_at": 1735689600,
            "reference": "ref_order_99",
            "reversal": null,
            "shipping_cost": {
                "amount": 500,
                "amount_tax": 45,
                "tax_behavior": "exclusive",
                "tax_code": "txcd_92010001"
            },
            "tax_date": 1735689600,
            "type": "transaction"
        });

        let tx: TaxTransaction = serde_json::from_value(json).expect("Deserialization failed");
        assert_eq!(tx.id, "tax_tx_sample_123");
        assert_eq!(tx.object, "tax.transaction");
        assert_eq!(tx.currency, "usd");
        assert_eq!(tx.customer.as_deref(), Some("cus_tax_client_99"));
        assert!(tx.is_standard_transaction());
        assert!(!tx.is_reversal());

        let shipping = tx.shipping_cost.as_ref().expect("Shipping cost missing");
        assert_eq!(shipping.amount, 500);
        assert_eq!(shipping.amount_tax, 45);

        let cust_details = tx
            .customer_details
            .as_ref()
            .expect("Customer details missing");
        assert_eq!(cust_details.address_source.as_deref(), Some("shipping"));
        let addr = cust_details.address.as_ref().expect("Address missing");
        assert_eq!(addr.state.as_deref(), Some("CA"));
        assert_eq!(addr.postal_code.as_deref(), Some("94103"));
    }

    #[test]
    fn test_tax_transaction_reversal_serde_roundtrip() {
        let json = serde_json::json!({
            "id": "tax_tx_rev_456",
            "object": "tax.transaction",
            "created": 1735690000,
            "currency": "usd",
            "customer": null,
            "customer_details": null,
            "livemode": false,
            "metadata": null,
            "posted_at": 1735690000,
            "reference": "ref_order_99_refund",
            "reversal": {
                "original_transaction": "tax_tx_sample_123",
                "mode": "full"
            },
            "shipping_cost": null,
            "tax_date": 1735690000,
            "type": "reversal"
        });

        let tx: TaxTransaction = serde_json::from_value(json).expect("Deserialization failed");
        assert_eq!(tx.id, "tax_tx_rev_456");
        assert!(tx.is_reversal());
        assert!(!tx.is_standard_transaction());

        let rev = tx.reversal.as_ref().expect("Reversal details missing");
        assert_eq!(
            rev.original_transaction.as_deref(),
            Some("tax_tx_sample_123")
        );
        assert_eq!(rev.mode.as_deref(), Some("full"));
    }

    #[test]
    fn test_tax_transaction_line_item_serde() {
        let json = serde_json::json!({
            "id": "tax_li_789",
            "object": "tax.transaction_line_item",
            "amount": 10000,
            "amount_tax": 825,
            "livemode": false,
            "metadata": { "sku": "SKU-SHOES-1" },
            "product": "prod_123",
            "quantity": 2,
            "reference": "line_item_ref_1",
            "reversal": null,
            "tax_behavior": "exclusive",
            "tax_code": "txcd_99999999",
            "type": "transaction"
        });

        let item: TaxTransactionLineItem =
            serde_json::from_value(json).expect("Deserialization failed");
        assert_eq!(item.id, "tax_li_789");
        assert_eq!(item.amount, 10000);
        assert_eq!(item.amount_tax, 825);
        assert_eq!(tax_transaction_line_item_id(&item), "tax_li_789");
    }
}
