//! # Stripe Quote Types & Status Transitions
//!
//! Strongly typed domain models, line items, and lifecycle state machines for Stripe Quotes,
//! adhering to ISO 4217 integer currency arithmetic and Rust zero-clone borrowing discipline.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// Status of a [`Quote`].
///
/// Indicates whether the quote is currently a draft, open for acceptance, accepted, or canceled.
///
/// # Example
/// ```
/// use some_stripe_types::QuoteStatus;
/// use std::str::FromStr;
///
/// let status = QuoteStatus::Draft;
/// assert_eq!(status.as_str(), "draft");
/// assert!(status.is_draft());
///
/// let parsed = QuoteStatus::from_str("open").unwrap();
/// assert_eq!(parsed, QuoteStatus::Open);
/// assert!(parsed.is_open());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QuoteStatus {
    /// The quote is in draft mode and can be edited.
    #[default]
    Draft,
    /// The quote has been finalized and is open for customer review/acceptance.
    Open,
    /// The quote has been accepted by the customer.
    Accepted,
    /// The quote has been canceled.
    Canceled,
}

impl QuoteStatus {
    /// Returns the static string representation of this quote status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Open => "open",
            Self::Accepted => "accepted",
            Self::Canceled => "canceled",
        }
    }

    /// Returns `true` if the status is [`QuoteStatus::Draft`].
    #[must_use]
    pub fn is_draft(&self) -> bool {
        matches!(self, Self::Draft)
    }

    /// Returns `true` if the status is [`QuoteStatus::Open`].
    #[must_use]
    pub fn is_open(&self) -> bool {
        matches!(self, Self::Open)
    }

    /// Returns `true` if the status is [`QuoteStatus::Accepted`].
    #[must_use]
    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted)
    }

    /// Returns `true` if the status is [`QuoteStatus::Canceled`].
    #[must_use]
    pub fn is_canceled(&self) -> bool {
        matches!(self, Self::Canceled)
    }
}

impl std::fmt::Display for QuoteStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for QuoteStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for QuoteStatus {
    type Err = ParseQuoteStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "open" => Ok(Self::Open),
            "accepted" => Ok(Self::Accepted),
            "canceled" => Ok(Self::Canceled),
            _ => Err(ParseQuoteStatusError(s.to_string())),
        }
    }
}

/// Error returned when parsing an unrecognized [`QuoteStatus`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseQuoteStatusError(pub String);

impl std::fmt::Display for ParseQuoteStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown QuoteStatus: '{}'", self.0)
    }
}

impl std::error::Error for ParseQuoteStatusError {}

/// State machine transitions validator for Stripe Quotes.
pub struct QuoteStatusTransitions;

impl QuoteStatusTransitions {
    /// Validates if transition from `current` to `target` is legally permitted.
    #[must_use]
    pub fn can_transition(current: QuoteStatus, target: QuoteStatus) -> bool {
        matches!(
            (current, target),
            (QuoteStatus::Draft, QuoteStatus::Open)
                | (QuoteStatus::Draft, QuoteStatus::Canceled)
                | (QuoteStatus::Open, QuoteStatus::Accepted)
                | (QuoteStatus::Open, QuoteStatus::Canceled)
        )
    }
}

/// Timestamps for status transitions of a [`Quote`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteStatusTransitionsTimestamps {
    /// The time at which the quote was accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_at: Option<i64>,

    /// The time at which the quote was canceled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canceled_at: Option<i64>,

    /// The time at which the quote was finalized.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finalized_at: Option<i64>,
}

/// Represents an individual line item within a [`Quote`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteLineItem {
    /// Unique identifier for this line item (`item_...` or `il_...`).
    pub id: String,

    /// Object type, always `item`.
    #[serde(default = "default_quote_line_item_object")]
    pub object: String,

    /// Total amount in minor currency units.
    pub amount_total: i64,

    /// Description of the line item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// ID of the price attached to this item (`price_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,

    /// Quantity of the item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i64>,
}

fn default_quote_line_item_object() -> String {
    "item".into()
}

/// Represents a Stripe Quote object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quote {
    /// Unique identifier for the quote (`qt_...`).
    pub id: String,

    /// Object type, always `quote`.
    #[serde(default = "default_quote_object")]
    pub object: String,

    /// Total subtotal amount in minor currency units before tax/discounts.
    pub amount_subtotal: i64,

    /// Total amount in minor currency units after discounts and taxes.
    pub amount_total: i64,

    /// Either `charge_automatically` or `send_invoice`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_method: Option<String>,

    /// Three-letter ISO currency code in lowercase (e.g., `usd`, `eur`).
    pub currency: String,

    /// The ID of the customer associated with this quote (`cus_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,

    /// Description of the quote.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The UNIX timestamp at which this quote expires.
    pub expires_at: i64,

    /// Footer content displayed on the quote PDF.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer: Option<String>,

    /// Header content displayed on the quote PDF.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,

    /// The ID of the invoice created by this quote (`in_...`), if accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice: Option<String>,

    /// Line items associated with this quote.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_items: Option<Vec<QuoteLineItem>>,

    /// Set of key-value pairs attached to the quote.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// A unique number identifying this quote (e.g. `QT-0001`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,

    /// Link to download the PDF representation of the quote.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdf: Option<String>,

    /// Status of the quote (`draft`, `open`, `accepted`, `canceled`).
    pub status: QuoteStatus,

    /// Details of status transitions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_transitions: Option<QuoteStatusTransitionsTimestamps>,

    /// Time at which the object was created (UNIX epoch seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
}

fn default_quote_object() -> String {
    "quote".into()
}

impl Quote {
    /// Returns `true` if this quote is currently a draft.
    #[must_use]
    pub fn is_draft(&self) -> bool {
        self.status.is_draft()
    }

    /// Returns `true` if this quote is currently open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.status.is_open()
    }

    /// Returns `true` if this quote has been accepted.
    #[must_use]
    pub fn is_accepted(&self) -> bool {
        self.status.is_accepted()
    }

    /// Returns `true` if this quote has been canceled.
    #[must_use]
    pub fn is_canceled(&self) -> bool {
        self.status.is_canceled()
    }
}

/// Paginated list of [`Quote`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteList {
    pub object: String,
    pub data: Vec<Quote>,
    pub has_more: bool,
    pub url: String,
}
