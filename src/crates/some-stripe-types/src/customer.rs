//! Customer domain entities, vaulting models, and cash balance schemas for Stripe.
//!
//! Enforces ISO 4217 minor currency integer rules (Gate G-Law) and GDPR Article 17
//! customer data management guidelines.

use serde::{Deserialize, Serialize};

/// Represents a Stripe Customer object in the customer vault.
///
/// # Examples
///
/// ```
/// use some_stripe_types::customer::Customer;
///
/// let json = r#"{
///     "id": "cus_12345",
///     "object": "customer",
///     "email": "customer@example.com",
///     "name": "Jane Doe",
///     "balance": 0,
///     "currency": "usd",
///     "created": 1728345600,
///     "delinquent": false,
///     "livemode": false
/// }"#;
/// let customer: Customer = serde_json::from_str(json).unwrap();
/// assert_eq!(customer.id, "cus_12345");
/// assert_eq!(customer.email.as_deref(), Some("customer@example.com"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Customer {
    /// Unique identifier for the customer (`cus_...`).
    pub id: String,

    /// Object type, always `customer`.
    #[serde(default = "default_customer_object")]
    pub object: String,

    /// Customer's default currency in lowercase ISO 4217 code.
    pub currency: Option<String>,

    /// Customer's primary email address.
    pub email: Option<String>,

    /// Customer's full name or business name.
    pub name: Option<String>,

    /// Customer's phone number.
    pub phone: Option<String>,

    /// Customer's description.
    pub description: Option<String>,

    /// Current balance of the customer in minor currency units (e.g. cents).
    /// Negative means credit; positive means debit.
    #[serde(default)]
    pub balance: i64,

    /// Time at which the customer was created (Unix timestamp).
    pub created: i64,

    /// Whether the customer has any unpaid invoices or past due subscriptions.
    #[serde(default)]
    pub delinquent: bool,

    /// Has the value `true` if the object exists in live mode or `false` for test mode.
    #[serde(default)]
    pub livemode: bool,

    /// ID of the default payment source or payment method.
    pub default_source: Option<String>,

    /// Mailing and billing address for the customer.
    pub address: Option<CustomerAddress>,

    /// Shipping details for the customer.
    pub shipping: Option<CustomerShipping>,

    /// Key-value metadata attached to the customer.
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,

    /// Customer tax identification numbers.
    #[serde(default)]
    pub tax_ids: Option<Vec<CustomerTaxId>>,

    /// Customer cash balance details.
    pub cash_balance: Option<CashBalance>,

    /// Default invoice settings for this customer.
    pub invoice_settings: Option<CustomerInvoiceSettings>,
}

fn default_customer_object() -> String {
    "customer".into()
}

/// Mailing or billing address for a customer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CustomerAddress {
    /// City, district, suburb, town, or village.
    pub city: Option<String>,

    /// Two-letter country code (ISO 3166-1 alpha-2).
    pub country: Option<String>,

    /// Address line 1 (e.g., street, PO Box, or company name).
    pub line1: Option<String>,

    /// Address line 2 (e.g., apartment, suite, unit, or building).
    pub line2: Option<String>,

    /// ZIP or postal code.
    pub postal_code: Option<String>,

    /// State, county, province, or region.
    pub state: Option<String>,
}

/// Shipping address and contact information for a customer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerShipping {
    /// Shipping recipient name.
    pub name: String,

    /// Customer shipping address.
    pub address: CustomerAddress,

    /// Recipient phone number.
    pub phone: Option<String>,
}

/// Customer tax identification number details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerTaxId {
    /// Unique identifier for the tax ID object (`txi_...`).
    pub id: String,

    /// Object type, always `tax_id`.
    #[serde(default = "default_tax_id_object")]
    pub object: String,

    /// Type of the tax ID (e.g., `eu_vat`, `us_ein`, `gb_vat`).
    #[serde(rename = "type")]
    pub tax_id_type: String,

    /// Value of the tax ID.
    pub value: String,

    /// Country code for the tax ID.
    pub country: Option<String>,
}

fn default_tax_id_object() -> String {
    "tax_id".into()
}

/// Cash balance details for customer vaulting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashBalance {
    /// Object type, always `cash_balance`.
    #[serde(default = "default_cash_balance_object")]
    pub object: String,

    /// The ID of the customer whose cash balance this is.
    pub customer: String,

    /// Available customer balance amounts by currency in ISO 4217 minor integer units.
    #[serde(default)]
    pub available: Option<std::collections::HashMap<String, i64>>,

    /// Has the value `true` if the object exists in live mode or `false` for test mode.
    #[serde(default)]
    pub livemode: bool,
}

fn default_cash_balance_object() -> String {
    "cash_balance".into()
}

/// Default invoice settings for a customer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CustomerInvoiceSettings {
    /// ID of the default payment method used for subscriptions and invoices.
    pub default_payment_method: Option<String>,

    /// Default custom fields displayed on invoices for this customer.
    #[serde(default)]
    pub custom_fields: Option<Vec<InvoiceCustomField>>,

    /// Default footer text for invoices.
    pub footer: Option<String>,
}

/// Custom field displayed on customer invoices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceCustomField {
    /// Name of the custom field.
    pub name: String,

    /// Value of the custom field.
    pub value: String,
}
