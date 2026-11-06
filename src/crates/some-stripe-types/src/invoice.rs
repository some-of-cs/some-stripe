//! # Stripe Invoice Models and EU VAT / ZATCA Phase 2 Compliance (Gate G-Law)
//!
//! Strongly typed domain models for Stripe Invoices, status lifecycles,
//! EU VAT Directive 2006/112/EC sequential numbering compliance,
//! and ZATCA Phase 2 cryptographic hash-chaining.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;
use std::str::FromStr;

/// Status of a Stripe Invoice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    /// Initial draft state before finalization.
    #[default]
    Draft,
    /// Finalized invoice awaiting payment.
    Open,
    /// Fully paid invoice.
    Paid,
    /// Marked as uncollectible / bad debt.
    Uncollectible,
    /// Voided invoice.
    Void,
}

impl InvoiceStatus {
    /// Returns the Stripe API string representation.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Open => "open",
            Self::Paid => "paid",
            Self::Uncollectible => "uncollectible",
            Self::Void => "void",
        }
    }

    /// Returns `true` if in draft status.
    pub const fn is_draft(&self) -> bool {
        matches!(self, Self::Draft)
    }

    /// Returns `true` if in open status.
    pub const fn is_open(&self) -> bool {
        matches!(self, Self::Open)
    }

    /// Returns `true` if paid.
    pub const fn is_paid(&self) -> bool {
        matches!(self, Self::Paid)
    }

    /// Returns `true` if marked uncollectible.
    pub const fn is_uncollectible(&self) -> bool {
        matches!(self, Self::Uncollectible)
    }

    /// Returns `true` if voided.
    pub const fn is_void(&self) -> bool {
        matches!(self, Self::Void)
    }

    /// Validates whether a state transition to `target` is allowed by Stripe's lifecycle.
    pub fn can_transition_to(&self, target: Self) -> bool {
        match (self, target) {
            (Self::Draft, Self::Open | Self::Void) => true,
            (Self::Open, Self::Paid | Self::Uncollectible | Self::Void) => true,
            (Self::Uncollectible, Self::Paid | Self::Void) => true,
            (curr, next) if *curr == next => true,
            _ => false,
        }
    }
}

impl Deref for InvoiceStatus {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl std::fmt::Display for InvoiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for InvoiceStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Error returned when parsing an invalid invoice status string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseInvoiceStatusError(pub String);

impl std::fmt::Display for ParseInvoiceStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown InvoiceStatus: '{}'", self.0)
    }
}

impl std::error::Error for ParseInvoiceStatusError {}

impl FromStr for InvoiceStatus {
    type Err = ParseInvoiceStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "open" => Ok(Self::Open),
            "paid" => Ok(Self::Paid),
            "uncollectible" => Ok(Self::Uncollectible),
            "void" => Ok(Self::Void),
            _ => Err(ParseInvoiceStatusError(s.to_string())),
        }
    }
}

/// Payment settings configured on an [`Invoice`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoicePaymentSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_mandate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_method_options: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_method_types: Option<Vec<String>>,
}

/// Status transitions timestamp tracking on an [`Invoice`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceStatusTransitions {
    pub finalized_at: Option<i64>,
    pub marked_uncollectible_at: Option<i64>,
    pub paid_at: Option<i64>,
    pub voided_at: Option<i64>,
}

/// Represents an individual line item on an invoice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceLineItem {
    pub id: String,
    #[serde(default = "default_line_item_object")]
    pub object: String,
    pub amount: i64,
    pub currency: String,
    pub description: Option<String>,
    pub quantity: Option<i64>,
    pub proration: Option<bool>,
}

impl InvoiceLineItem {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn amount(&self) -> i64 {
        self.amount
    }
    pub fn currency(&self) -> &str {
        &self.currency
    }
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    pub fn quantity(&self) -> Option<i64> {
        self.quantity
    }
    pub fn is_proration(&self) -> bool {
        self.proration.unwrap_or(false)
    }
}

fn default_line_item_object() -> String {
    "line_item".into()
}

/// Paginated list container for invoice line items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceLineItemList {
    #[serde(default = "default_list_object")]
    pub object: String,
    pub data: Vec<InvoiceLineItem>,
    pub has_more: bool,
    pub url: String,
}

impl InvoiceLineItemList {
    pub fn as_slice(&self) -> &[InvoiceLineItem] {
        &self.data
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

fn default_list_object() -> String {
    "list".into()
}

/// ZATCA Phase 2 E-Invoicing Compliance Payload (Saudi Arabia FATOORA).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZatcaComplianceData {
    /// Monotonically increasing sequential invoice counter.
    pub invoice_counter_value: u64,
    /// Cryptographic SHA-256 hash of the previous invoice (PIH) forming an immutable hash chain.
    pub previous_invoice_hash: String,
    /// Current invoice SHA-256 cryptographic stamp / digest.
    pub cryptographic_stamp: String,
    /// TLV-encoded Base64 QR code string.
    pub qr_code_payload: String,
}

/// Represents a Stripe Invoice object.
///
/// Implements full lifecycle tracking, EU VAT sequential numbering fields,
/// and ZATCA Phase 2 compliance data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invoice {
    /// Unique identifier for the invoice (`in_...`).
    ///
    /// For upcoming invoice preview events (`invoice.upcoming`), `id` is `None` (JSON `null`).
    pub id: Option<String>,

    /// Object type, always `invoice`.
    #[serde(default = "default_invoice_object")]
    pub object: String,

    /// Final amount due at this time for this invoice in minor currency units (e.g., cents).
    pub amount_due: Option<i64>,

    /// The amount, in minor currency units, that was paid.
    pub amount_paid: Option<i64>,

    /// The difference between amount_due and amount_paid, in minor currency units.
    pub amount_remaining: Option<i64>,

    /// Total after discounts and taxes in minor currency units.
    pub total: Option<i64>,

    /// Total of all items before discounts and taxes in minor currency units.
    pub subtotal: Option<i64>,

    /// Three-letter ISO currency code in lowercase.
    pub currency: Option<String>,

    /// The ID of the customer who will be billed.
    pub customer: Option<String>,

    /// Customer email address.
    pub customer_email: Option<String>,

    /// Customer name.
    pub customer_name: Option<String>,

    /// Description of the invoice.
    pub description: Option<String>,

    /// Hosted invoice URL for customer payment.
    pub hosted_invoice_url: Option<String>,

    /// Link to download the PDF invoice.
    pub invoice_pdf: Option<String>,

    /// Individual line items on this invoice.
    pub lines: Option<InvoiceLineItemList>,

    /// Metadata key-value pairs.
    pub metadata: Option<HashMap<String, String>>,

    /// EU VAT Directive 2006/112/EC sequential number (e.g. `INV-2024-0001`).
    /// Assigned only when finalized.
    pub number: Option<String>,

    /// Whether payment was successfully made for this invoice.
    pub paid: Option<bool>,

    /// The status of the invoice: `draft`, `open`, `paid`, `uncollectible`, or `void`.
    pub status: Option<InvoiceStatus>,

    /// Timestamp status transitions.
    pub status_transitions: Option<InvoiceStatusTransitions>,

    /// Payment settings configured on this invoice.
    pub payment_settings: Option<InvoicePaymentSettings>,

    /// The ID of the subscription that this invoice was prepared for, if any.
    pub subscription: Option<String>,

    /// Start of the usage period that this invoice covers (UNIX epoch seconds).
    pub period_start: Option<i64>,

    /// End of the usage period that this invoice covers (UNIX epoch seconds).
    pub period_end: Option<i64>,

    /// Time at which the object was created (UNIX epoch seconds).
    pub created: Option<i64>,

    /// Date on which payment is due (UNIX epoch seconds).
    pub due_date: Option<i64>,

    /// ZATCA Phase 2 compliance record, if applicable.
    pub zatca_compliance: Option<ZatcaComplianceData>,
}

fn default_invoice_object() -> String {
    "invoice".into()
}

impl Invoice {
    /// Returns the invoice ID if present, or `None` if this is an upcoming invoice preview.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Returns `true` if this invoice is an upcoming preview (`id` is `None`).
    pub fn is_upcoming(&self) -> bool {
        self.id.is_none()
    }

    /// Returns a borrowed slice of invoice line items if present without heap allocations (Gate G-Borrow).
    pub fn lines_slice(&self) -> Option<&[InvoiceLineItem]> {
        self.lines.as_ref().map(|l| l.as_slice())
    }
}

/// Errors occurring during invoice statutory compliance validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvoiceComplianceError {
    /// EU VAT Directive sequential numbering gap or format violation.
    SequentialNumberingViolation(String),
    /// ZATCA Phase 2 hash-chain or cryptographic stamp tampering detected.
    ZatcaHashChainViolation(String),
}

impl std::fmt::Display for InvoiceComplianceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SequentialNumberingViolation(msg) => write!(
                f,
                "EU VAT Directive 2006/112/EC sequential numbering violation: {msg}"
            ),
            Self::ZatcaHashChainViolation(msg) => {
                write!(f, "ZATCA Phase 2 compliance violation: {msg}")
            }
        }
    }
}

impl std::error::Error for InvoiceComplianceError {}

/// Validates EU VAT Directive 2006/112/EC Article 226 sequential numbering integrity.
///
/// Ensures sequential numbers within a series increment strictly by 1 without gaps or back-dating.
pub fn validate_eu_vat_sequential_numbering(
    prev_number: Option<&str>,
    curr_number: &str,
) -> Result<(), InvoiceComplianceError> {
    let curr_trimmed = curr_number.trim();
    if curr_trimmed.is_empty() {
        return Err(InvoiceComplianceError::SequentialNumberingViolation(
            "Current invoice number cannot be blank".into(),
        ));
    }

    let Some(prev) = prev_number else {
        return Ok(());
    };

    let prev_trimmed = prev.trim();
    if prev_trimmed.is_empty() {
        return Ok(());
    }

    let split_prefix_seq = |s: &str| -> Result<(String, u64), InvoiceComplianceError> {
        let last_dash = s.rfind('-').or_else(|| s.rfind('/'));
        match last_dash {
            Some(idx) => {
                let prefix = &s[..idx];
                let num_part = &s[idx + 1..];
                let num = num_part.parse::<u64>().map_err(|_| {
                    InvoiceComplianceError::SequentialNumberingViolation(format!(
                        "Invalid sequential numeric suffix in invoice number '{s}'"
                    ))
                })?;
                Ok((prefix.to_string(), num))
            }
            None => {
                let num = s.parse::<u64>().map_err(|_| {
                    InvoiceComplianceError::SequentialNumberingViolation(format!(
                        "Invoice number '{s}' lacks numeric sequence or standard delimiter ('-', '/')"
                    ))
                })?;
                Ok((String::new(), num))
            }
        }
    };

    let (prev_prefix, prev_seq) = split_prefix_seq(prev_trimmed)?;
    let (curr_prefix, curr_seq) = split_prefix_seq(curr_trimmed)?;

    if prev_prefix != curr_prefix {
        return Err(InvoiceComplianceError::SequentialNumberingViolation(
            format!(
                "Series prefix mismatch between previous ('{prev_prefix}') and current ('{curr_prefix}')"
            ),
        ));
    }

    if curr_seq != prev_seq + 1 {
        return Err(InvoiceComplianceError::SequentialNumberingViolation(
            format!(
                "Non-sequential invoice numbering detected: previous was {prev_seq}, current is {curr_seq} (expected {})",
                prev_seq + 1
            ),
        ));
    }

    Ok(())
}

/// Validates ZATCA Phase 2 hash-chain continuity between previous invoice hash and current payload.
pub fn validate_zatca_phase2_hash_chain(
    expected_previous_hash: &str,
    actual_previous_hash_field: &str,
    current_payload: &[u8],
    expected_current_stamp: &str,
) -> Result<(), InvoiceComplianceError> {
    if expected_previous_hash != actual_previous_hash_field {
        return Err(InvoiceComplianceError::ZatcaHashChainViolation(format!(
            "Broken hash chain: expected previous hash '{expected_previous_hash}', got '{actual_previous_hash_field}'"
        )));
    }

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(current_payload);
    let hash = hasher.finalize();
    let computed_stamp: String = hash.iter().map(|b| format!("{b:02x}")).collect();

    if computed_stamp != expected_current_stamp {
        return Err(InvoiceComplianceError::ZatcaHashChainViolation(format!(
            "Cryptographic stamp mismatch: computed '{computed_stamp}', expected '{expected_current_stamp}'"
        )));
    }

    Ok(())
}
