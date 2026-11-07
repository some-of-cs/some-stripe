//! Request parameter schemas for Stripe Quotes API operations.

use crate::QuoteStatus;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Line item parameter for creating or updating a quote.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteLineItemParams {
    /// The ID of the price to associate with this line item (`price_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,

    /// The quantity of products being quoted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i64>,
}

impl QuoteLineItemParams {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_price(mut self, price: impl Into<String>) -> Self {
        self.price = Some(price.into());
        self
    }

    #[must_use]
    pub fn with_quantity(mut self, quantity: i64) -> Self {
        self.quantity = Some(quantity);
        self
    }
}

/// Parameters for creating a new [`Quote`] (`POST /v1/quotes`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateQuoteParams {
    /// The customer for which this quote is created (`cus_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,

    /// A list of line items on the quote.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_items: Option<Vec<QuoteLineItemParams>>,

    /// Description of the quote.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The UNIX timestamp at which the quote expires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,

    /// Footer content displayed on the quote PDF.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer: Option<String>,

    /// Header content displayed on the quote PDF.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,

    /// Key-value metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Unique idempotency key for safe execution (Gate G2).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateQuoteParams {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_customer(mut self, customer: impl Into<String>) -> Self {
        self.customer = Some(customer.into());
        self
    }

    #[must_use]
    pub fn with_line_items(mut self, items: Vec<QuoteLineItemParams>) -> Self {
        self.line_items = Some(items);
        self
    }

    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    #[must_use]
    pub fn with_expires_at(mut self, expires_at: i64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    #[must_use]
    pub fn with_footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    #[must_use]
    pub fn with_header(mut self, header: impl Into<String>) -> Self {
        self.header = Some(header.into());
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

/// Parameters for updating an existing [`Quote`] (`POST /v1/quotes/{id}`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateQuoteParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_items: Option<Vec<QuoteLineItemParams>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl UpdateQuoteParams {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    #[must_use]
    pub fn with_expires_at(mut self, expires_at: i64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    #[must_use]
    pub fn with_footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    #[must_use]
    pub fn with_header(mut self, header: impl Into<String>) -> Self {
        self.header = Some(header.into());
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

/// Parameters for finalizing a [`Quote`] (`POST /v1/quotes/{id}/finalize`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalizeQuoteParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,

    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl FinalizeQuoteParams {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_expires_at(mut self, expires_at: i64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for accepting a [`Quote`] (`POST /v1/quotes/{id}/accept`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptQuoteParams {
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl AcceptQuoteParams {
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

/// Parameters for canceling a [`Quote`] (`POST /v1/quotes/{id}/cancel`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelQuoteParams {
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CancelQuoteParams {
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

/// Parameters for listing quotes (`GET /v1/quotes`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListQuotesParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<QuoteStatus>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}
