//! # Stripe Credit Note Types & Invoice Balance Deductions
//!
//! Strongly typed domain models, parameters, and balance deduction invariants for Stripe Credit Notes,
//! adhering to ISO 4217 integer minor currency units, immutable historical invoice records,
//! and Rust zero-clone discipline (Gate G-Borrow).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// Standard Stripe credit note reasons.
///
/// Reason for issuing a credit note against an invoice.
///
/// # Example
/// ```
/// use some_stripe_types::CreditNoteReason;
/// use std::str::FromStr;
///
/// let reason = CreditNoteReason::OrderChange;
/// assert_eq!(reason.as_str(), "order_change");
/// assert_eq!(reason.to_string(), "order_change");
///
/// let parsed = CreditNoteReason::from_str("fraudulent").unwrap();
/// assert_eq!(parsed, CreditNoteReason::Fraudulent);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CreditNoteReason {
    /// Charge was a duplicate.
    Duplicate,
    /// Fraudulent activity.
    Fraudulent,
    /// Customer requested order modification or cancellation.
    #[default]
    OrderChange,
    /// Product or service was unsatisfactory.
    ProductUnsatisfactory,
}

impl CreditNoteReason {
    /// Returns the static string slice representation for this reason code.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Duplicate => "duplicate",
            Self::Fraudulent => "fraudulent",
            Self::OrderChange => "order_change",
            Self::ProductUnsatisfactory => "product_unsatisfactory",
        }
    }
}

impl std::fmt::Display for CreditNoteReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for CreditNoteReason {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for CreditNoteReason {
    type Err = ParseCreditNoteReasonError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "duplicate" => Ok(Self::Duplicate),
            "fraudulent" => Ok(Self::Fraudulent),
            "order_change" => Ok(Self::OrderChange),
            "product_unsatisfactory" => Ok(Self::ProductUnsatisfactory),
            _ => Err(ParseCreditNoteReasonError(s.to_string())),
        }
    }
}

/// Error returned when parsing an unrecognized [`CreditNoteReason`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseCreditNoteReasonError(pub String);

impl std::fmt::Display for ParseCreditNoteReasonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown CreditNoteReason: '{}'", self.0)
    }
}

impl std::error::Error for ParseCreditNoteReasonError {}

/// Status of a [`CreditNote`].
///
/// Indicates whether the credit note is currently active (`issued`) or has been voided (`void`).
///
/// # Example
/// ```
/// use some_stripe_types::CreditNoteStatus;
/// use std::str::FromStr;
///
/// let status = CreditNoteStatus::Issued;
/// assert_eq!(status.as_str(), "issued");
/// assert!(status.is_issued());
///
/// let parsed = CreditNoteStatus::from_str("void").unwrap();
/// assert_eq!(parsed, CreditNoteStatus::Void);
/// assert!(parsed.is_void());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CreditNoteStatus {
    /// The credit note has been issued and applied against the invoice.
    #[default]
    Issued,
    /// The credit note was voided and its adjustments reversed.
    Void,
}

impl CreditNoteStatus {
    /// Returns the static string representation of this status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Issued => "issued",
            Self::Void => "void",
        }
    }

    /// Returns `true` if the status is [`CreditNoteStatus::Issued`].
    #[must_use]
    pub fn is_issued(&self) -> bool {
        matches!(self, Self::Issued)
    }

    /// Returns `true` if the status is [`CreditNoteStatus::Void`].
    #[must_use]
    pub fn is_void(&self) -> bool {
        matches!(self, Self::Void)
    }
}

impl std::fmt::Display for CreditNoteStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for CreditNoteStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for CreditNoteStatus {
    type Err = ParseCreditNoteStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "issued" => Ok(Self::Issued),
            "void" => Ok(Self::Void),
            _ => Err(ParseCreditNoteStatusError(s.to_string())),
        }
    }
}

/// Error returned when parsing an unrecognized [`CreditNoteStatus`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseCreditNoteStatusError(pub String);

impl std::fmt::Display for ParseCreditNoteStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown CreditNoteStatus: '{}'", self.0)
    }
}

impl std::error::Error for ParseCreditNoteStatusError {}

/// Type of a credit note indicating whether it was created pre-payment or post-payment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreditNoteType {
    /// Pre-payment credit note adjusting an unpaid invoice before collection.
    PrePayment,
    /// Post-payment credit note issuing credit balance or refund against a paid invoice.
    PostPayment,
}

impl CreditNoteType {
    /// Returns the static string representation of this credit note type.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PrePayment => "pre_payment",
            Self::PostPayment => "post_payment",
        }
    }
}

impl std::fmt::Display for CreditNoteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for CreditNoteType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Represents an individual credited line item within a [`CreditNote`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreditNoteLineItem {
    /// Unique identifier for this line item (`cnli_...`).
    pub id: String,

    /// Object type, always `credit_note_line_item`.
    #[serde(default = "default_credit_note_line_item_object")]
    pub object: String,

    /// The integer amount in minor currency units credited for this line item.
    pub amount: i64,

    /// Description of the line item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The amount of discount calculated for this line item in minor units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discount_amount: Option<i64>,

    /// ID of the invoice line item being credited (`il_...`), if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice_line_item: Option<String>,

    /// Quantity credited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i64>,

    /// Unit amount in minor units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_amount: Option<i64>,

    /// Type of line item: `invoice_line_item` or `custom_line_item`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

fn default_credit_note_line_item_object() -> String {
    "credit_note_line_item".into()
}

/// Represents a Stripe Credit Note object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreditNote {
    /// Unique identifier for the credit note (`cn_...`).
    pub id: String,

    /// Object type, always `credit_note`.
    #[serde(default = "default_credit_note_object")]
    pub object: String,

    /// Total amount credited in minor currency units (e.g. cents) conforming to ISO 4217.
    pub amount: i64,

    /// Amount in minor currency units that is applied against customer's credit balance.
    #[serde(default)]
    pub credit_amount: i64,

    /// Three-letter ISO currency code in lowercase (e.g., `usd`, `eur`).
    pub currency: String,

    /// The ID of the customer associated with this credit note (`cus_...`).
    pub customer: String,

    /// Reference to customer balance transaction created by this credit note (`cbt_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer_balance_transaction: Option<String>,

    /// The amount of discount calculated for this credit note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discount_amount: Option<i64>,

    /// The ID of the invoice credited by this credit note (`in_...`).
    pub invoice: String,

    /// Line items included in the credit note.
    #[serde(default)]
    pub lines: Vec<CreditNoteLineItem>,

    /// Has the value `true` if the object exists in live mode or `false` if in test mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub livemode: Option<bool>,

    /// Customer-facing note or memo explaining the credit note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    /// Set of key-value pairs attached to the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// A unique number identifying this credit note (e.g. `CN-0001`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,

    /// Amount in minor currency units that was credited outside of Stripe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_of_band_amount: Option<i64>,

    /// The link to download the PDF representation of the credit note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdf: Option<String>,

    /// Reason for credit note issuance (`duplicate`, `fraudulent`, `order_change`, `product_unsatisfactory`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<CreditNoteReason>,

    /// ID of the refund created by this credit note (`re_...`), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refund: Option<String>,

    /// Status of the credit note (`issued` or `void`).
    pub status: CreditNoteStatus,

    /// Subtotal of the credit note before discounts and taxes in minor units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtotal: Option<i64>,

    /// Total amount of the credit note in minor units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,

    /// Credit note type: `pre_payment` or `post_payment`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<CreditNoteType>,

    /// Time at which the credit note was voided (UNIX epoch seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voided_at: Option<i64>,

    /// Time at which the credit note was created (UNIX epoch seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
}

fn default_credit_note_object() -> String {
    "credit_note".into()
}

impl CreditNote {
    /// Returns `true` if this credit note has status `issued`.
    #[must_use]
    pub fn is_issued(&self) -> bool {
        self.status.is_issued()
    }

    /// Returns `true` if this credit note has status `void`.
    #[must_use]
    pub fn is_void(&self) -> bool {
        self.status.is_void()
    }

    /// Validates that credit note balance deduction does not exceed the paid amount of the invoice,
    /// safeguarding historical immutable invoice ledger records.
    pub fn validate_balance_deduction(&self, invoice_paid_amount: i64) -> Result<(), &'static str> {
        if self.amount <= 0 {
            return Err("Credit note amount must be strictly positive");
        }
        if self.amount > invoice_paid_amount {
            return Err("Credit note amount cannot exceed total invoice paid amount");
        }
        Ok(())
    }
}

/// Line item parameter for credit note creation and preview requests.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreditNoteLineItemParams {
    /// Type of line item: `invoice_line_item` or `custom_line_item`.
    pub r#type: String,

    /// ID of the invoice line item being credited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice_line_item: Option<String>,

    /// Amount to credit in minor integer units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i64>,

    /// Description of custom line item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Quantity to credit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i64>,

    /// Unit amount in minor units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_amount: Option<i64>,
}

/// Parameters for issuing a new [`CreditNote`] (`POST /v1/credit_notes`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateCreditNoteParams {
    /// The ID of the invoice to credit (`in_...`).
    pub invoice: String,

    /// The integer amount in minor currency units representing the total amount of the credit note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i64>,

    /// Amount in minor currency units to apply to customer balance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credit_amount: Option<i64>,

    /// Amount in minor currency units to refund to original payment source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refund_amount: Option<i64>,

    /// Amount credited out-of-band.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_of_band_amount: Option<i64>,

    /// Reason for credit note issuance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<CreditNoteReason>,

    /// Internal customer-facing memo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    /// Specific line items to credit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<CreditNoteLineItemParams>>,

    /// Key-value metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Unique idempotency key for safe execution (Gate G2).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateCreditNoteParams {
    #[must_use]
    pub fn for_invoice(invoice: impl Into<String>) -> Self {
        Self {
            invoice: invoice.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn with_amount(mut self, amount: i64) -> Self {
        self.amount = Some(amount);
        self
    }

    #[must_use]
    pub fn with_credit_amount(mut self, credit_amount: i64) -> Self {
        self.credit_amount = Some(credit_amount);
        self
    }

    #[must_use]
    pub fn with_refund_amount(mut self, refund_amount: i64) -> Self {
        self.refund_amount = Some(refund_amount);
        self
    }

    #[must_use]
    pub fn with_reason(mut self, reason: CreditNoteReason) -> Self {
        self.reason = Some(reason);
        self
    }

    #[must_use]
    pub fn with_memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = Some(memo.into());
        self
    }

    #[must_use]
    pub fn with_lines(mut self, lines: Vec<CreditNoteLineItemParams>) -> Self {
        self.lines = Some(lines);
        self
    }

    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for updating an existing [`CreditNote`] (`POST /v1/credit_notes/{id}`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCreditNoteParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl UpdateCreditNoteParams {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = Some(memo.into());
        self
    }

    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for voiding a [`CreditNote`] (`POST /v1/credit_notes/{id}/void`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoidCreditNoteParams {
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl VoidCreditNoteParams {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for previewing a [`CreditNote`] (`GET /v1/credit_notes/preview`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewCreditNoteParams {
    pub invoice: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credit_amount: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refund_amount: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_of_band_amount: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<CreditNoteReason>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<CreditNoteLineItemParams>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl PreviewCreditNoteParams {
    #[must_use]
    pub fn for_invoice(invoice: impl Into<String>) -> Self {
        Self {
            invoice: invoice.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn with_amount(mut self, amount: i64) -> Self {
        self.amount = Some(amount);
        self
    }

    #[must_use]
    pub fn with_credit_amount(mut self, credit_amount: i64) -> Self {
        self.credit_amount = Some(credit_amount);
        self
    }

    #[must_use]
    pub fn with_refund_amount(mut self, refund_amount: i64) -> Self {
        self.refund_amount = Some(refund_amount);
        self
    }

    #[must_use]
    pub fn with_reason(mut self, reason: CreditNoteReason) -> Self {
        self.reason = Some(reason);
        self
    }

    #[must_use]
    pub fn with_memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = Some(memo.into());
        self
    }

    #[must_use]
    pub fn with_lines(mut self, lines: Vec<CreditNoteLineItemParams>) -> Self {
        self.lines = Some(lines);
        self
    }
}

/// Parameters for listing credit notes (`GET /v1/credit_notes`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListCreditNotesParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

/// Paginated list of [`CreditNote`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreditNoteList {
    pub object: String,
    pub data: Vec<CreditNote>,
    pub has_more: bool,
    pub url: String,
}
