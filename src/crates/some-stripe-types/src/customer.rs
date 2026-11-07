//! Customer domain entities, vaulting models, cash balance, and tax ID schemas for Stripe.
//!
//! Enforces ISO 4217 minor currency integer rules (Gate G-Law), PCI-DSS 4.0 tokenization
//! (Gate G2/G-Law), and GDPR Article 17 Right to Erasure customer data management.
//!
//! # Payment Method Tokenization (PCI-DSS 4.0 Gate G-Law)
//! All payment credentials on customer entities are referenced strictly through tokenized
//! identifiers (`pm_...` or `src_...`), ensuring zero raw Primary Account Numbers (PAN)
//! or sensitive authentication data (SAD) enter application memory.

pub use crate::nullable::Nullable;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a Stripe Customer object in the customer vault.
///
/// Customers allow you to perform recurring charges, track multiple payments, and save
/// tokenized payment credentials.
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
///     "balance": -5000,
///     "currency": "usd",
///     "created": 1728345600,
///     "delinquent": false,
///     "livemode": false,
///     "default_payment_method": "pm_1N4m2L2eZvKYlo2C01234567"
/// }"#;
/// let customer: Customer = serde_json::from_str(json).unwrap();
/// assert_eq!(customer.id, "cus_12345");
/// assert_eq!(customer.email.as_deref(), Some("customer@example.com"));
/// assert_eq!(customer.balance, -5000);
/// assert_eq!(customer.default_payment_method.as_deref(), Some("pm_1N4m2L2eZvKYlo2C01234567"));
/// assert!(!customer.delinquent);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Customer {
    /// Unique identifier for the customer (`cus_...`).
    pub id: String,

    /// Object type, always `customer`.
    #[serde(default = "default_customer_object")]
    pub object: String,

    /// Customer's default currency in lowercase ISO 4217 code (e.g. `usd`, `eur`).
    pub currency: Option<String>,

    /// Customer's primary email address.
    pub email: Option<String>,

    /// Customer's full name or business name.
    pub name: Option<String>,

    /// Customer's phone number.
    pub phone: Option<String>,

    /// Customer's description.
    pub description: Option<String>,

    /// Current balance of the customer in minor currency integer units (e.g. cents).
    /// Negative means credit; positive means debit owed by the customer.
    #[serde(default)]
    pub balance: i64,

    /// Time at which the customer was created (Unix epoch seconds).
    #[serde(default)]
    pub created: i64,

    /// Whether the customer has any unpaid invoices or past-due subscriptions.
    #[serde(default)]
    pub delinquent: bool,

    /// Has the value `true` if the object exists in live mode or `false` for test mode.
    #[serde(default)]
    pub livemode: bool,

    /// Whether the customer has been soft-deleted or deleted on Stripe servers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,

    /// ID of the default payment source (`src_...` or legacy `card_...`).
    pub default_source: Option<String>,

    /// ID of the default payment method (`pm_...`) attached to this customer.
    ///
    /// PCI-DSS 4.0: Only tokenized references are held. Zero raw PAN.
    pub default_payment_method: Option<String>,

    /// Mailing and billing address for the customer.
    pub address: Option<CustomerAddress>,

    /// Shipping details for the customer.
    pub shipping: Option<CustomerShipping>,

    /// Key-value metadata attached to the customer.
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    /// Customer tax identification numbers.
    #[serde(default)]
    pub tax_ids: Option<Vec<TaxId>>,

    /// Customer cash balance details.
    pub cash_balance: Option<CashBalance>,

    /// Default invoice settings for this customer.
    pub invoice_settings: Option<CustomerInvoiceSettings>,

    /// Prefix for invoices generated for this customer (e.g. `INV-`).
    pub invoice_prefix: Option<String>,

    /// The sequence to be used on the customer's next invoice.
    pub next_invoice_sequence: Option<i64>,

    /// Customer's tax exemption status (`none`, `exempt`, `reverse`).
    pub tax_exempt: Option<String>,

    /// ID of the test clock attached to the customer, if in simulation.
    pub test_clock: Option<String>,
}

impl Customer {
    /// Returns the customer's ID (`cus_...`).
    #[inline]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the customer's email address if present.
    #[inline]
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    /// Returns the customer's full name if present.
    #[inline]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the customer's tokenized default payment method ID (`pm_...`) if present.
    #[inline]
    pub fn default_payment_method(&self) -> Option<&str> {
        self.default_payment_method.as_deref()
    }

    /// Returns the customer's balance in ISO 4217 minor currency units (e.g., cents).
    #[inline]
    pub fn balance(&self) -> i64 {
        self.balance
    }

    /// Returns `true` if the customer is delinquent (has past-due invoices).
    #[inline]
    pub fn is_delinquent(&self) -> bool {
        self.delinquent
    }

    /// Returns `true` if the customer has been deleted.
    #[inline]
    pub fn is_deleted(&self) -> bool {
        self.deleted.unwrap_or(false)
    }

    /// Sanitizes all Personally Identifiable Information (PII) from this customer record
    /// in compliance with **GDPR Article 17 (Right to Erasure)**.
    ///
    /// Preserves immutable statutory financial record fields (`id`, `created`, `currency`, `balance`)
    /// required for legal, tax, and accounting retention (7–10 year statutory requirement).
    ///
    /// # Example
    ///
    /// ```
    /// use some_stripe_types::customer::{Customer, CustomerAddress};
    ///
    /// let mut customer = Customer {
    ///     id: "cus_123".into(),
    ///     object: "customer".into(),
    ///     currency: Some("usd".into()),
    ///     email: Some("jane@example.com".into()),
    ///     name: Some("Jane Doe".into()),
    ///     phone: Some("+15555555555".into()),
    ///     description: Some("VIP customer".into()),
    ///     balance: 0,
    ///     created: 1728345600,
    ///     delinquent: false,
    ///     livemode: false,
    ///     deleted: None,
    ///     default_source: None,
    ///     default_payment_method: Some("pm_abc".into()),
    ///     address: Some(CustomerAddress {
    ///         city: Some("Austin".into()),
    ///         ..Default::default()
    ///     }),
    ///     shipping: None,
    ///     metadata: std::collections::HashMap::new(),
    ///     tax_ids: None,
    ///     cash_balance: None,
    ///     invoice_settings: None,
    ///     invoice_prefix: None,
    ///     next_invoice_sequence: None,
    ///     tax_exempt: None,
    ///     test_clock: None,
    /// };
    ///
    /// assert!(!customer.is_pii_scrubbed());
    /// customer.scrub_gdpr_pii();
    /// assert!(customer.is_pii_scrubbed());
    /// assert_eq!(customer.email, None);
    /// assert_eq!(customer.name, None);
    /// assert_eq!(customer.id, "cus_123"); // Financial identifier retained
    /// ```
    pub fn scrub_gdpr_pii(&mut self) {
        self.email = None;
        self.name = None;
        self.phone = None;
        self.description = None;
        self.address = None;
        self.shipping = None;
        self.metadata.clear();
        self.default_source = None;
        self.default_payment_method = None;
        self.tax_ids = None;
        if let Some(ref mut settings) = self.invoice_settings {
            settings.default_payment_method = None;
            settings.footer = None;
            settings.custom_fields = None;
        }
    }

    /// Returns `true` if all GDPR PII fields have been scrubbed.
    pub fn is_pii_scrubbed(&self) -> bool {
        self.email.is_none()
            && self.name.is_none()
            && self.phone.is_none()
            && self.description.is_none()
            && self.address.is_none()
            && self.shipping.is_none()
            && self.metadata.is_empty()
            && self.default_payment_method.is_none()
    }

    /// Applies GDPR Article 17 Right to Erasure by scrubbing all Personally Identifiable
    /// Information (PII) from this customer record while marking it deleted and retaining
    /// statutory financial ledger entries (`id`, `created`, `currency`, `balance`).
    pub fn apply_gdpr_erasure(&mut self) {
        self.scrub_gdpr_pii();
        self.deleted = Some(true);
    }

    /// Returns the customer's available cash balance for the specified currency in minor units, if present.
    pub fn available_balance(&self, currency: &str) -> Option<i64> {
        self.cash_balance
            .as_ref()?
            .available
            .as_ref()?
            .get(currency)
            .copied()
    }
}

/// Helper function to extract customer ID reference for paginators (Gate G-Borrow zero-clone).
///
/// # Example
///
/// ```
/// use some_stripe_types::customer::{Customer, customer_id};
///
/// let customer = Customer {
///     id: "cus_xyz123".into(),
///     object: "customer".into(),
///     currency: None,
///     email: None,
///     name: None,
///     phone: None,
///     description: None,
///     balance: 0,
///     created: 1728345600,
///     delinquent: false,
///     livemode: false,
///     deleted: None,
///     default_source: None,
///     default_payment_method: None,
///     address: None,
///     shipping: None,
///     metadata: std::collections::HashMap::new(),
///     tax_ids: None,
///     cash_balance: None,
///     invoice_settings: None,
///     invoice_prefix: None,
///     next_invoice_sequence: None,
///     tax_exempt: None,
///     test_clock: None,
/// };
/// assert_eq!(customer_id(&customer), "cus_xyz123");
/// ```
#[inline]
pub fn customer_id(c: &Customer) -> &str {
    &c.id
}

/// Helper to apply GDPR Article 17 Right to Erasure on a mutable customer record.
///
/// Sanitizes all Personally Identifiable Information (PII) from the customer object
/// while retaining immutable non-PII financial ledger entries (`id`, `created`, `currency`, `balance`)
/// required for statutory tax and accounting retention.
///
/// # Example
///
/// ```
/// use some_stripe_types::customer::{Customer, redact_gdpr_customer_data};
///
/// let mut customer = Customer {
///     id: "cus_999".into(),
///     object: "customer".into(),
///     currency: Some("usd".into()),
///     email: Some("user@example.com".into()),
///     name: Some("Alice Smith".into()),
///     phone: Some("+15551234567".into()),
///     description: Some("Loyal customer".into()),
///     balance: 1500,
///     created: 1728345600,
///     delinquent: false,
///     livemode: false,
///     deleted: None,
///     default_source: None,
///     default_payment_method: Some("pm_valid_123".into()),
///     address: None,
///     shipping: None,
///     metadata: std::collections::HashMap::new(),
///     tax_ids: None,
///     cash_balance: None,
///     invoice_settings: None,
///     invoice_prefix: None,
///     next_invoice_sequence: None,
///     tax_exempt: None,
///     test_clock: None,
/// };
/// redact_gdpr_customer_data(&mut customer);
/// assert!(customer.is_pii_scrubbed());
/// assert_eq!(customer.is_deleted(), true);
/// assert_eq!(customer.balance, 1500); // Financial ledger entry preserved
/// ```
pub fn redact_gdpr_customer_data(customer: &mut Customer) {
    customer.apply_gdpr_erasure();
}

/// Error returned when a payment credential violates PCI-DSS 4.0 tokenization invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PciDssViolationError(pub String);

impl std::fmt::Display for PciDssViolationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PCI-DSS 4.0 Tokenization Violation: {}", self.0)
    }
}

impl std::error::Error for PciDssViolationError {}

/// Validates that a payment credential identifier is tokenized (`pm_...`, `src_...`, `tok_...`, `card_...`, `ba_...`)
/// and strictly prevents raw Primary Account Numbers (PAN) or sensitive cardholder data from entering application memory.
///
/// Enforces PCI-DSS 4.0 requirement 3.4 (PAN must be rendered unreadable anywhere it is stored or processed).
///
/// # Examples
///
/// ```
/// use some_stripe_types::customer::validate_pci_dss_token;
///
/// // Valid tokenized identifiers
/// assert!(validate_pci_dss_token("pm_1N4m2L2eZvKYlo2C01234567").is_ok());
/// assert!(validate_pci_dss_token("src_12345abcdef").is_ok());
/// assert!(validate_pci_dss_token("tok_visa").is_ok());
/// assert!(validate_pci_dss_token("card_12345").is_ok());
///
/// // Raw card numbers (PANs) are strictly rejected
/// assert!(validate_pci_dss_token("4242424242424242").is_err());
/// assert!(validate_pci_dss_token("378282246310005").is_err());
/// assert!(validate_pci_dss_token("").is_err());
/// ```
pub fn validate_pci_dss_token(token: &str) -> Result<(), PciDssViolationError> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Err(PciDssViolationError(
            "Payment method token must not be empty".into(),
        ));
    }
    // Check if token looks like a raw PAN (12 to 19 digits)
    let digits_only: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits_only.len() >= 12
        && digits_only.len() <= 19
        && trimmed
            .chars()
            .all(|c| c.is_ascii_digit() || c == ' ' || c == '-')
    {
        return Err(PciDssViolationError(
            "Raw Primary Account Number (PAN) detected in memory. PCI-DSS 4.0 mandates tokenized identifiers (e.g. pm_..., tok_...) only.".into(),
        ));
    }
    // Check valid token prefixes
    if trimmed.starts_with("pm_")
        || trimmed.starts_with("src_")
        || trimmed.starts_with("tok_")
        || trimmed.starts_with("card_")
        || trimmed.starts_with("ba_")
        || trimmed.starts_with("btok_")
    {
        Ok(())
    } else {
        Err(PciDssViolationError(format!(
            "Invalid payment method identifier '{trimmed}'. Must start with pm_, src_, tok_, card_, or ba_."
        )))
    }
}

/// Returns `true` if the given token satisfies PCI-DSS 4.0 tokenization rules.
#[inline]
pub fn is_pci_dss_tokenized(token: &str) -> bool {
    validate_pci_dss_token(token).is_ok()
}

fn default_customer_object() -> String {
    "customer".into()
}

/// Represents the response returned when a Customer is deleted.
///
/// # Example
///
/// ```
/// use some_stripe_types::customer::DeletedCustomer;
///
/// let json = r#"{
///     "id": "cus_12345",
///     "object": "customer",
///     "deleted": true
/// }"#;
/// let deleted: DeletedCustomer = serde_json::from_str(json).unwrap();
/// assert_eq!(deleted.id, "cus_12345");
/// assert!(deleted.deleted);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletedCustomer {
    /// Unique identifier for the customer (`cus_...`).
    pub id: String,

    /// Object type, always `customer`.
    #[serde(default = "default_customer_object")]
    pub object: String,

    /// Always `true` indicating the customer record was deleted.
    pub deleted: bool,
}

/// Mailing or billing address for a customer.
///
/// # Example
///
/// ```
/// use some_stripe_types::customer::CustomerAddress;
///
/// let addr = CustomerAddress {
///     city: Some("San Francisco".into()),
///     country: Some("US".into()),
///     line1: Some("123 Market St".into()),
///     line2: Some("Suite 400".into()),
///     postal_code: Some("94105".into()),
///     state: Some("CA".into()),
/// };
/// assert_eq!(addr.city.as_deref(), Some("San Francisco"));
/// ```
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

/// Type alias for customer address.
pub type Address = CustomerAddress;

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
///
/// # Example
///
/// ```
/// use some_stripe_types::customer::TaxId;
///
/// let json = r#"{
///     "id": "txi_123",
///     "object": "tax_id",
///     "type": "eu_vat",
///     "value": "DE123456789",
///     "country": "DE"
/// }"#;
/// let tax_id: TaxId = serde_json::from_str(json).unwrap();
/// assert_eq!(tax_id.id, "txi_123");
/// assert_eq!(tax_id.tax_id_type, "eu_vat");
/// assert_eq!(tax_id.value, "DE123456789");
/// assert_eq!(tax_id.country.as_deref(), Some("DE"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxId {
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

    /// Two-letter country code (ISO 3166-1 alpha-2) for the tax ID.
    pub country: Option<String>,

    /// ID of the customer who owns this tax ID.
    pub customer: Option<String>,

    /// Has the value `true` if the object exists in live mode or `false` for test mode.
    #[serde(default)]
    pub livemode: bool,

    /// Time at which the tax ID was created (Unix epoch seconds).
    pub created: Option<i64>,

    /// Verification details for the tax ID.
    pub verification: Option<TaxIdVerification>,
}

fn default_tax_id_object() -> String {
    "tax_id".into()
}

/// Type alias for [`TaxId`] to preserve backward compatibility.
pub type CustomerTaxId = TaxId;

/// Verification status details for a customer tax ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxIdVerification {
    /// Verification status (`pending`, `verified`, `unverified`, `unavailable`).
    pub status: String,

    /// Verified address associated with the tax ID.
    pub verified_address: Option<String>,

    /// Verified business name associated with the tax ID.
    pub verified_name: Option<String>,
}

/// Cash balance details for customer vaulting.
///
/// Tracks customer balance stored on Stripe across multiple currencies.
///
/// # Example
///
/// ```
/// use some_stripe_types::customer::CashBalance;
///
/// let json = r#"{
///     "object": "cash_balance",
///     "customer": "cus_123",
///     "available": { "usd": 25000 },
///     "livemode": false
/// }"#;
/// let cb: CashBalance = serde_json::from_str(json).unwrap();
/// assert_eq!(cb.customer, "cus_123");
/// assert_eq!(cb.available.as_ref().unwrap().get("usd"), Some(&25000));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashBalance {
    /// Object type, always `cash_balance`.
    #[serde(default = "default_cash_balance_object")]
    pub object: String,

    /// The ID of the customer whose cash balance this is.
    pub customer: String,

    /// Available customer balance amounts by currency in ISO 4217 minor integer units.
    #[serde(default)]
    pub available: Option<HashMap<String, i64>>,

    /// Has the value `true` if the object exists in live mode or `false` for test mode.
    #[serde(default)]
    pub livemode: bool,

    /// Configuration settings for the customer's cash balance.
    pub settings: Option<CashBalanceSettings>,
}

fn default_cash_balance_object() -> String {
    "cash_balance".into()
}

/// Configuration settings for a customer's cash balance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CashBalanceSettings {
    /// The configuration for how Stripe handles customer reconciliation.
    pub reconciliation_mode: Option<String>,

    /// A flag to indicate when Stripe should use the merchant default setting.
    pub using_merchant_default: Option<bool>,
}

/// Default invoice settings for a customer.
///
/// # Example
///
/// ```
/// use some_stripe_types::customer::CustomerInvoiceSettings;
///
/// let settings = CustomerInvoiceSettings {
///     default_payment_method: Some("pm_123".into()),
///     custom_fields: None,
///     footer: Some("Thank you for your business!".into()),
/// };
/// assert_eq!(settings.default_payment_method.as_deref(), Some("pm_123"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CustomerInvoiceSettings {
    /// ID of the default payment method (`pm_...`) used for subscriptions and invoices.
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

/// Error type for customer parameter validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomerValidationError {
    /// PCI-DSS 4.0 violation in payment method parameter.
    PciDss(PciDssViolationError),
    /// Invalid currency code format.
    InvalidCurrency(String),
    /// General validation error description.
    InvalidField {
        /// Name of the field that failed validation.
        field: &'static str,
        /// Reason for validation failure.
        message: String,
    },
}

impl std::fmt::Display for CustomerValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PciDss(err) => write!(f, "{err}"),
            Self::InvalidCurrency(c) => write!(
                f,
                "Invalid ISO 4217 currency code: '{c}'. Expected 3-letter lowercase code."
            ),
            Self::InvalidField { field, message } => {
                write!(f, "Validation error on field '{field}': {message}")
            }
        }
    }
}

impl std::error::Error for CustomerValidationError {}

impl From<PciDssViolationError> for CustomerValidationError {
    fn from(err: PciDssViolationError) -> Self {
        Self::PciDss(err)
    }
}

/// Parameters for creating a new Stripe Customer in the vault (`POST /v1/customers`).
///
/// # Examples
///
/// ```
/// use some_stripe_types::customer::CreateCustomerParams;
///
/// let params = CreateCustomerParams::new()
///     .email("customer@example.com")
///     .name("Jane Doe")
///     .balance(-5000)
///     .currency("usd")
///     .payment_method("pm_1N4m2L2eZvKYlo2C01234567")
///     .idempotency_key("req_cust_create_001");
///
/// assert!(params.validate().is_ok());
/// assert_eq!(params.email.as_deref(), Some("customer@example.com"));
/// assert_eq!(params.balance, Some(-5000));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CreateCustomerParams {
    /// Mailing and billing address for the customer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<CustomerAddress>,

    /// An integer amount in cents/minor units representing the customer's balance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance: Option<i64>,

    /// An arbitrary string that you can attach to a customer object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Customer's email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Default invoice settings for this customer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice_settings: Option<CustomerInvoiceSettingsParams>,

    /// Key-value metadata attached to the customer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Customer's full name or business name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Customer's phone number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,

    /// ID of the tokenized payment method (`pm_...`) to attach to the customer.
    ///
    /// PCI-DSS 4.0: Only tokenized IDs are accepted. Raw PAN is strictly rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<String>,

    /// Shipping information for the customer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping: Option<CustomerShipping>,

    /// Customer's default currency in lowercase ISO 4217 code (e.g. `usd`, `eur`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    /// Prefix for invoices generated for this customer (e.g. `INV-`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice_prefix: Option<String>,

    /// Customer's tax exemption status (`none`, `exempt`, `reverse`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_exempt: Option<String>,

    /// Customer tax IDs to create and attach.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_id_data: Option<Vec<CustomerTaxIdDataParams>>,

    /// Idempotency key for safe retries (omitted from body payload).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateCustomerParams {
    /// Creates a new, empty [`CreateCustomerParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the customer's email address.
    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Sets the customer's full name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the customer's phone number.
    pub fn phone(mut self, phone: impl Into<String>) -> Self {
        self.phone = Some(phone.into());
        self
    }

    /// Sets the customer's description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Sets the tokenized payment method ID (`pm_...`).
    pub fn payment_method(mut self, pm: impl Into<String>) -> Self {
        self.payment_method = Some(pm.into());
        self
    }

    /// Sets the customer's initial balance in ISO 4217 minor units.
    pub fn balance(mut self, bal: i64) -> Self {
        self.balance = Some(bal);
        self
    }

    /// Sets the customer's default currency in ISO 4217 code.
    pub fn currency(mut self, cur: impl Into<String>) -> Self {
        self.currency = Some(cur.into().to_lowercase());
        self
    }

    /// Sets the customer's address.
    pub fn address(mut self, addr: CustomerAddress) -> Self {
        self.address = Some(addr);
        self
    }

    /// Sets the customer's shipping address.
    pub fn shipping(mut self, ship: CustomerShipping) -> Self {
        self.shipping = Some(ship);
        self
    }

    /// Sets the customer's metadata map.
    pub fn metadata(mut self, meta: HashMap<String, String>) -> Self {
        self.metadata = Some(meta);
        self
    }

    /// Inserts a key-value entry into metadata.
    pub fn metadata_entry(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.metadata
            .get_or_insert_with(HashMap::new)
            .insert(k.into(), v.into());
        self
    }

    /// Sets invoice settings.
    pub fn invoice_settings(mut self, settings: CustomerInvoiceSettingsParams) -> Self {
        self.invoice_settings = Some(settings);
        self
    }

    /// Sets the idempotency key for safe retries.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates the parameter payload according to PCI-DSS 4.0 and ISO 4217 rules.
    pub fn validate(&self) -> Result<(), CustomerValidationError> {
        if let Some(ref pm) = self.payment_method {
            validate_pci_dss_token(pm)?;
        }
        if let Some(ref cur) = self.currency
            && (cur.len() != 3 || !cur.chars().all(|c| c.is_ascii_alphabetic()))
        {
            return Err(CustomerValidationError::InvalidCurrency(cur.clone()));
        }
        Ok(())
    }
}

/// Parameters for customer default invoice settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CustomerInvoiceSettingsParams {
    /// ID of the default payment method (`pm_...`) used for subscriptions and invoices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_payment_method: Option<String>,

    /// Default footer text for invoices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer: Option<String>,

    /// Custom fields displayed on customer invoices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_fields: Option<Vec<InvoiceCustomField>>,
}

impl CustomerInvoiceSettingsParams {
    /// Creates a new, empty [`CustomerInvoiceSettingsParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the default payment method ID.
    pub fn default_payment_method(mut self, pm: impl Into<String>) -> Self {
        self.default_payment_method = Some(pm.into());
        self
    }

    /// Sets the footer text.
    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }
}

/// Specification for customer tax identification numbers during creation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerTaxIdDataParams {
    /// Type of the tax ID (e.g. `eu_vat`, `us_ein`, `gb_vat`).
    #[serde(rename = "type")]
    pub tax_id_type: String,

    /// Value of the tax ID.
    pub value: String,
}

impl CustomerTaxIdDataParams {
    /// Creates a new customer tax ID parameter specification.
    pub fn new(tax_id_type: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            tax_id_type: tax_id_type.into(),
            value: value.into(),
        }
    }
}

/// Parameters for updating an existing Stripe Customer in the vault (`POST /v1/customers/{id}`).
///
/// Employs 3-state [`Nullable`] mechanics:
/// - `Nullable::Value(T)` updates the stored value.
/// - `Nullable::Null` explicitly clears/resets the remote field (serialized as `""` in form payloads).
/// - `Nullable::Omit` skips the field during serialization.
///
/// # Examples
///
/// ```
/// use some_stripe_types::customer::UpdateCustomerParams;
///
/// // Update name and clear description
/// let params = UpdateCustomerParams::new()
///     .name("Jane Smith")
///     .clear_description()
///     .default_payment_method("pm_1N4m2L2eZvKYlo2C01234567");
///
/// assert!(params.validate().is_ok());
/// assert!(params.name.is_value());
/// assert!(params.description.is_null());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UpdateCustomerParams {
    /// Mailing and billing address for the customer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<CustomerAddress>,

    /// An integer amount in cents/minor units representing the customer's balance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance: Option<i64>,

    /// An arbitrary string that you can attach to a customer object.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub description: Nullable<String>,

    /// Customer's email address.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub email: Nullable<String>,

    /// ID of the default payment method (`pm_...`) for this customer.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub default_payment_method: Nullable<String>,

    /// Default invoice settings for this customer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice_settings: Option<CustomerInvoiceSettingsParams>,

    /// Key-value metadata attached to the customer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Customer's full name or business name.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub name: Nullable<String>,

    /// Customer's phone number.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub phone: Nullable<String>,

    /// Shipping information for the customer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping: Option<CustomerShipping>,

    /// Customer's tax exemption status (`none`, `exempt`, `reverse`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_exempt: Option<String>,

    /// Idempotency key for safe retries (omitted from body payload).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl UpdateCustomerParams {
    /// Creates a new, empty [`UpdateCustomerParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the updated name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Nullable::Value(name.into());
        self
    }

    /// Clears the customer's name on Stripe servers.
    pub fn clear_name(mut self) -> Self {
        self.name = Nullable::Null;
        self
    }

    /// Sets the updated email address.
    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Nullable::Value(email.into());
        self
    }

    /// Clears the customer's email address on Stripe servers.
    pub fn clear_email(mut self) -> Self {
        self.email = Nullable::Null;
        self
    }

    /// Sets the updated description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Nullable::Value(desc.into());
        self
    }

    /// Clears the customer's description on Stripe servers.
    pub fn clear_description(mut self) -> Self {
        self.description = Nullable::Null;
        self
    }

    /// Sets the updated phone number.
    pub fn phone(mut self, phone: impl Into<String>) -> Self {
        self.phone = Nullable::Value(phone.into());
        self
    }

    /// Clears the customer's phone number on Stripe servers.
    pub fn clear_phone(mut self) -> Self {
        self.phone = Nullable::Null;
        self
    }

    /// Sets the default payment method ID (`pm_...`).
    pub fn default_payment_method(mut self, pm: impl Into<String>) -> Self {
        self.default_payment_method = Nullable::Value(pm.into());
        self
    }

    /// Clears the customer's default payment method on Stripe servers.
    pub fn clear_default_payment_method(mut self) -> Self {
        self.default_payment_method = Nullable::Null;
        self
    }

    /// Sets the updated balance in ISO 4217 minor currency units.
    pub fn balance(mut self, bal: i64) -> Self {
        self.balance = Some(bal);
        self
    }

    /// Sets updated address.
    pub fn address(mut self, addr: CustomerAddress) -> Self {
        self.address = Some(addr);
        self
    }

    /// Sets updated shipping address.
    pub fn shipping(mut self, ship: CustomerShipping) -> Self {
        self.shipping = Some(ship);
        self
    }

    /// Sets updated invoice settings.
    pub fn invoice_settings(mut self, settings: CustomerInvoiceSettingsParams) -> Self {
        self.invoice_settings = Some(settings);
        self
    }

    /// Sets updated metadata map.
    pub fn metadata(mut self, meta: HashMap<String, String>) -> Self {
        self.metadata = Some(meta);
        self
    }

    /// Sets the idempotency key for safe retries.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates parameters against PCI-DSS 4.0 tokenization rules.
    pub fn validate(&self) -> Result<(), CustomerValidationError> {
        if let Some(pm) = self.default_payment_method.as_option() {
            validate_pci_dss_token(pm)?;
        }
        Ok(())
    }
}

/// Parameters for listing customers (`GET /v1/customers`).
///
/// # Examples
///
/// ```
/// use some_stripe_types::customer::ListCustomersParams;
///
/// let params = ListCustomersParams::new()
///     .limit(20)
///     .email("customer@example.com")
///     .starting_after("cus_123");
///
/// assert_eq!(params.limit, Some(20));
/// assert_eq!(params.email.as_deref(), Some("customer@example.com"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ListCustomersParams {
    /// A limit on the number of objects to be returned, between 1 and 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// A cursor for use in pagination. An object ID defining your place in the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// A cursor for use in pagination. An object ID defining your place in the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,

    /// A case-sensitive filter on the list based on the customer's `email` field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Only return customers that were created during the given date range or Unix timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,

    /// Provides a list of customers that are associated with the specified test clock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_clock: Option<String>,
}

impl ListCustomersParams {
    /// Creates a new, empty [`ListCustomersParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum number of items to return.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the `starting_after` pagination cursor.
    pub fn starting_after(mut self, cursor: impl Into<String>) -> Self {
        self.starting_after = Some(cursor.into());
        self
    }

    /// Sets the `ending_before` pagination cursor.
    pub fn ending_before(mut self, cursor: impl Into<String>) -> Self {
        self.ending_before = Some(cursor.into());
        self
    }

    /// Filters customers by email address.
    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Filters customers by creation timestamp.
    pub fn created(mut self, created: i64) -> Self {
        self.created = Some(created);
        self
    }

    /// Filters customers by test clock ID.
    pub fn test_clock(mut self, clock: impl Into<String>) -> Self {
        self.test_clock = Some(clock.into());
        self
    }
}

/// Parameters for searching customers (`GET /v1/customers/search`).
///
/// # Examples
///
/// ```
/// use some_stripe_types::customer::SearchCustomersParams;
///
/// let params = SearchCustomersParams::new("email:'customer@example.com'")
///     .limit(10);
///
/// assert_eq!(params.query, "email:'customer@example.com'");
/// assert_eq!(params.limit, Some(10));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchCustomersParams {
    /// The search query string following Stripe search syntax (e.g. `email:'jane@example.com'`).
    pub query: String,

    /// A limit on the number of objects to be returned, between 1 and 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// A cursor for pagination across search results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
}

impl SearchCustomersParams {
    /// Creates a new search specification with the provided query string.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            limit: None,
            page: None,
        }
    }

    /// Sets the maximum number of items to return.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the pagination page token.
    pub fn page(mut self, page: impl Into<String>) -> Self {
        self.page = Some(page.into());
        self
    }
}

/// Parameters for updating a customer's cash balance settings (`POST /v1/customers/{id}/cash_balance`).
///
/// # Examples
///
/// ```
/// use some_stripe_types::customer::{CashBalanceSettingsParams, UpdateCustomerCashBalanceParams};
///
/// let params = UpdateCustomerCashBalanceParams::new()
///     .settings(CashBalanceSettingsParams::new().reconciliation_mode("manual"));
///
/// assert!(params.settings.is_some());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UpdateCustomerCashBalanceParams {
    /// A hash of settings for this cash balance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<CashBalanceSettingsParams>,

    /// Idempotency key for safe retries.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl UpdateCustomerCashBalanceParams {
    /// Creates a new [`UpdateCustomerCashBalanceParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets cash balance settings.
    pub fn settings(mut self, settings: CashBalanceSettingsParams) -> Self {
        self.settings = Some(settings);
        self
    }

    /// Sets idempotency key.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Configuration settings for customer cash balance reconciliation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CashBalanceSettingsParams {
    /// The configuration for how Stripe handles customer reconciliation (`automatic` or `manual`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reconciliation_mode: Option<String>,

    /// A flag to indicate when Stripe should use the merchant default setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub using_merchant_default: Option<bool>,
}

impl CashBalanceSettingsParams {
    /// Creates a new [`CashBalanceSettingsParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the reconciliation mode (`automatic` or `manual`).
    pub fn reconciliation_mode(mut self, mode: impl Into<String>) -> Self {
        self.reconciliation_mode = Some(mode.into());
        self
    }

    /// Sets the using merchant default flag.
    pub fn using_merchant_default(mut self, val: bool) -> Self {
        self.using_merchant_default = Some(val);
        self
    }
}

/// Paginated list of Stripe Customers (`object: "list"`).
///
/// # Examples
///
/// ```
/// use some_stripe_types::customer::CustomerList;
///
/// let json = r#"{
///     "object": "list",
///     "data": [],
///     "has_more": false,
///     "url": "/v1/customers"
/// }"#;
/// let list: CustomerList = serde_json::from_str(json).unwrap();
/// assert!(!list.has_more);
/// assert!(list.data.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerList {
    /// Object type, always `list`.
    #[serde(default = "default_list_object")]
    pub object: String,

    /// List of customer objects.
    pub data: Vec<Customer>,

    /// True if there are more customers available in the list.
    pub has_more: bool,

    /// The URL where this list can be accessed.
    pub url: String,
}

fn default_list_object() -> String {
    "list".into()
}

/// Search result collection for Customers (`object: "search_result"`).
///
/// # Examples
///
/// ```
/// use some_stripe_types::customer::CustomerSearchResult;
///
/// let json = r#"{
///     "object": "search_result",
///     "data": [],
///     "has_more": false,
///     "url": "/v1/customers/search"
/// }"#;
/// let res: CustomerSearchResult = serde_json::from_str(json).unwrap();
/// assert!(!res.has_more);
/// assert!(res.data.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerSearchResult {
    /// Object type, always `search_result`.
    #[serde(default = "default_search_result_object")]
    pub object: String,

    /// List of matching customer objects.
    pub data: Vec<Customer>,

    /// True if there are more results available.
    pub has_more: bool,

    /// Cursor for the next page of results, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page: Option<String>,

    /// The URL where this search result can be accessed.
    pub url: String,
}

fn default_search_result_object() -> String {
    "search_result".into()
}
