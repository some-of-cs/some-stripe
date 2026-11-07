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

/// Query parameters for fetching an upcoming invoice preview (`GET /v1/invoices/upcoming`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceUpcomingParams {
    /// The ID of the customer whose upcoming invoice will be previewed (`cus_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,

    /// The ID of the subscription for which you want to preview the upcoming invoice (`sub_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription: Option<String>,

    /// List of subscription items to adjust in the preview simulation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription_items: Option<Vec<InvoiceUpcomingSubscriptionItem>>,

    /// The date at which the proration is calculated (Unix epoch seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription_proration_date: Option<i64>,

    /// Determines how prorations are calculated when previewing changes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription_proration_behavior: Option<String>,

    /// Currency code if customer is not specified or for currency selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    /// The identifier of the schedule associated with the subscription (`sub_sched_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,

    /// Additional invoice items to simulate on the preview.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice_items: Option<Vec<InvoiceUpcomingInvoiceItemParams>>,
}

impl InvoiceUpcomingParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn customer(mut self, customer: impl Into<String>) -> Self {
        self.customer = Some(customer.into());
        self
    }

    pub fn subscription(mut self, subscription: impl Into<String>) -> Self {
        self.subscription = Some(subscription.into());
        self
    }

    pub fn currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = Some(currency.into().to_ascii_lowercase());
        self
    }

    pub fn subscription_proration_date(mut self, date: i64) -> Self {
        self.subscription_proration_date = Some(date);
        self
    }

    pub fn subscription_proration_behavior(mut self, behavior: impl Into<String>) -> Self {
        self.subscription_proration_behavior = Some(behavior.into());
        self
    }

    pub fn schedule(mut self, schedule: impl Into<String>) -> Self {
        self.schedule = Some(schedule.into());
        self
    }

    pub fn add_subscription_item(mut self, item: InvoiceUpcomingSubscriptionItem) -> Self {
        self.subscription_items
            .get_or_insert_with(Vec::new)
            .push(item);
        self
    }

    pub fn add_invoice_item(mut self, item: InvoiceUpcomingInvoiceItemParams) -> Self {
        self.invoice_items.get_or_insert_with(Vec::new).push(item);
        self
    }
}

/// Simulated subscription item adjustment for [`InvoiceUpcomingParams`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceUpcomingSubscriptionItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
}

impl InvoiceUpcomingSubscriptionItem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn price(mut self, price: impl Into<String>) -> Self {
        self.price = Some(price.into());
        self
    }

    pub fn quantity(mut self, quantity: i64) -> Self {
        self.quantity = Some(quantity);
        self
    }

    pub fn deleted(mut self, deleted: bool) -> Self {
        self.deleted = Some(deleted);
        self
    }
}

/// Simulated invoice item parameter for [`InvoiceUpcomingParams`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceUpcomingInvoiceItemParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i64>,
}

/// Parameter definition for an individual line item added to an invoice.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceLineItemParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_rates: Option<Vec<String>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl InvoiceLineItemParams {
    pub fn from_price(price: impl Into<String>, quantity: i64) -> Self {
        Self {
            price: Some(price.into()),
            quantity: Some(quantity),
            ..Default::default()
        }
    }

    pub fn from_amount(
        amount: i64,
        currency: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            amount: Some(amount),
            currency: Some(currency.into().to_ascii_lowercase()),
            description: Some(description.into()),
            quantity: Some(1),
            ..Default::default()
        }
    }

    pub fn price(mut self, price: impl Into<String>) -> Self {
        self.price = Some(price.into());
        self
    }

    pub fn amount(mut self, amount: i64) -> Self {
        self.amount = Some(amount);
        self
    }

    pub fn currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = Some(currency.into().to_ascii_lowercase());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn quantity(mut self, quantity: i64) -> Self {
        self.quantity = Some(quantity);
        self
    }

    pub fn add_tax_rate(mut self, tax_rate_id: impl Into<String>) -> Self {
        self.tax_rates
            .get_or_insert_with(Vec::new)
            .push(tax_rate_id.into());
        self
    }

    pub fn metadata<K: Into<String>, V: Into<String>>(
        mut self,
        iter: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        self.metadata = Some(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
        self
    }
}

/// Parameters for dynamically adding lines to an invoice (`POST /v1/invoices/{id}/add_lines`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddInvoiceLinesParams {
    pub lines: Vec<InvoiceLineItemParams>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice: Option<String>,

    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl AddInvoiceLinesParams {
    pub fn new(lines: Vec<InvoiceLineItemParams>) -> Self {
        Self {
            lines,
            invoice: None,
            idempotency_key: None,
        }
    }

    pub fn add_line(mut self, line: InvoiceLineItemParams) -> Self {
        self.lines.push(line);
        self
    }

    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for updating a specific invoice line item (`POST /v1/invoices/{id}/lines/{line_id}`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateInvoiceLineParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_rates: Option<Vec<String>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl UpdateInvoiceLineParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn amount(mut self, amount: i64) -> Self {
        self.amount = Some(amount);
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn quantity(mut self, quantity: i64) -> Self {
        self.quantity = Some(quantity);
        self
    }

    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Specification of a line item to remove from an invoice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceLineRemovalItem {
    pub id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behavior: Option<String>,
}

impl InvoiceLineRemovalItem {
    pub fn delete(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            behavior: Some("delete".into()),
        }
    }

    pub fn unassign(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            behavior: Some("unassign".into()),
        }
    }
}

/// Parameters for removing lines from an invoice (`POST /v1/invoices/{id}/remove_lines`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveInvoiceLinesParams {
    pub lines: Vec<InvoiceLineRemovalItem>,

    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl RemoveInvoiceLinesParams {
    pub fn from_ids<I, S>(ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            lines: ids
                .into_iter()
                .map(InvoiceLineRemovalItem::delete)
                .collect(),
            idempotency_key: None,
        }
    }

    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Query parameters for listing line items of an Invoice (`GET /v1/invoices/{id}/lines`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListInvoiceLineItemsParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListInvoiceLineItemsParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn starting_after(mut self, cursor: impl Into<String>) -> Self {
        self.starting_after = Some(cursor.into());
        self
    }

    pub fn ending_before(mut self, cursor: impl Into<String>) -> Self {
        self.ending_before = Some(cursor.into());
        self
    }
}

impl From<Option<u32>> for ListInvoiceLineItemsParams {
    fn from(limit: Option<u32>) -> Self {
        Self {
            limit,
            starting_after: None,
            ending_before: None,
        }
    }
}

impl From<u32> for ListInvoiceLineItemsParams {
    fn from(limit: u32) -> Self {
        Self {
            limit: Some(limit),
            starting_after: None,
            ending_before: None,
        }
    }
}
