//! Request parameter types and list envelope for Stripe Invoices.

use crate::{Invoice, InvoicePaymentSettings, InvoiceStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parameters for creating a new draft invoice (`POST /v1/invoices`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateInvoiceParams {
    /// The ID of the customer who will be billed.
    pub customer: String,
    /// Controls whether Stripe will automatically advance and finalize the invoice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_advance: Option<bool>,
    /// Either `charge_automatically` or `send_invoice`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_method: Option<String>,
    /// Three-letter ISO currency code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// An arbitrary string attached to the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The date on which payment is due (UNIX timestamp).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_date: Option<i64>,
    /// The number of days from when the invoice is advanced until it is due.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub days_until_due: Option<u32>,
    /// Key-value metadata pairs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Payment settings for this invoice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_settings: Option<InvoicePaymentSettings>,
    /// The ID of the subscription that this invoice was prepared for, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription: Option<String>,
    /// Unique idempotency key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

impl CreateInvoiceParams {
    pub fn new(customer: impl Into<String>) -> Self {
        Self {
            customer: customer.into(),
            ..Default::default()
        }
    }

    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = Some(currency.into());
        self
    }

    pub fn with_collection_method(mut self, method: impl Into<String>) -> Self {
        self.collection_method = Some(method.into());
        self
    }

    pub fn with_auto_advance(mut self, auto: bool) -> Self {
        self.auto_advance = Some(auto);
        self
    }

    pub fn with_due_date(mut self, due_date: i64) -> Self {
        self.due_date = Some(due_date);
        self
    }

    pub fn with_days_until_due(mut self, days: u32) -> Self {
        self.days_until_due = Some(days);
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for updating an existing invoice (`POST /v1/invoices/{id}`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateInvoiceParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_advance: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_date: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_settings: Option<InvoicePaymentSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

impl UpdateInvoiceParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_auto_advance(mut self, auto: bool) -> Self {
        self.auto_advance = Some(auto);
        self
    }

    pub fn with_due_date(mut self, due_date: i64) -> Self {
        self.due_date = Some(due_date);
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for finalizing a draft invoice (`POST /v1/invoices/{id}/finalize`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalizeInvoiceParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_advance: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

impl FinalizeInvoiceParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_auto_advance(mut self, auto: bool) -> Self {
        self.auto_advance = Some(auto);
        self
    }

    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for paying an invoice (`POST /v1/invoices/{id}/pay`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayInvoiceParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forgive: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mandate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub off_session: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paid_out_of_band: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

impl PayInvoiceParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_paid_out_of_band(mut self, paid: bool) -> Self {
        self.paid_out_of_band = Some(paid);
        self
    }

    pub fn with_payment_method(mut self, pm: impl Into<String>) -> Self {
        self.payment_method = Some(pm.into());
        self
    }

    pub fn with_forgive(mut self, forgive: bool) -> Self {
        self.forgive = Some(forgive);
        self
    }

    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for voiding an invoice (`POST /v1/invoices/{id}/void`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoidInvoiceParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

impl VoidInvoiceParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for listing invoices (`GET /v1/invoices`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListInvoicesParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<InvoiceStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListInvoicesParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_customer(mut self, customer: impl Into<String>) -> Self {
        self.customer = Some(customer.into());
        self
    }

    pub fn with_status(mut self, status: InvoiceStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_starting_after(mut self, cursor: impl Into<String>) -> Self {
        self.starting_after = Some(cursor.into());
        self
    }
}

/// Paginated list of invoices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceList {
    #[serde(default = "default_list_object")]
    pub object: String,
    pub data: Vec<Invoice>,
    pub has_more: bool,
    pub url: String,
}

fn default_list_object() -> String {
    "list".into()
}
