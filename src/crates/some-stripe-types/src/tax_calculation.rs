//! # Stripe Tax Calculation Types & Domain Models
//!
//! Strongly typed domain models, request parameters, and statutory validation helpers for the
//! Stripe Tax Calculations API (`POST /v1/tax/calculations`, `GET /v1/tax/calculations/{id}`),
//! adhering to ISO 4217 integer minor currency units, EU VAT Directive 2006/112/EC Article 196,
//! and Rust zero-clone discipline (Gate G-Borrow).
//!
//! # Architecture & Statutory Compliance:
//! - **Gate G-Borrow (Zero-Clone Discipline)**: Parameter borrowing by reference (`&CreateTaxCalculationParams`),
//!   zero deep `.clone()` calls on entities in request pipelines, and borrowed string slices for accessors.
//! - **Gate G-Law (EU VAT Directive 2006/112/EC Article 196)**: [`validate_eu_vat_reverse_charge`] verifies
//!   that cross-border B2B transactions carry a zero tax rate, valid VAT identification, and explicit reverse charge designation.
//! - **Multi-Jurisdiction Apportionment**: Itemized tax breakdowns across country, state, county, city, and district levels.
//! - **Gate G-Complexity**: Cyclomatic complexity <= 6 across all methods and validation functions.
//! - **Gate G-Docs**: Runnable doc-tests verified by `cargo test --doc`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// Default object string for [`TaxCalculation`].
fn default_tax_calculation_object() -> String {
    "tax.calculation".to_string()
}

/// Default object string for [`TaxCalculationLineItem`].
fn default_line_item_object() -> String {
    "tax.calculation_line_item".to_string()
}

/// Default object string for [`TaxCalculationLineItemList`].
fn default_list_object() -> String {
    "list".to_string()
}

/// Calculation mode for a [`TaxCalculation`].
///
/// Indicates whether standard calculation rules, B2B reverse charge rules,
/// exemption rules, or deferred rules apply.
///
/// # Example
/// ```
/// use some_stripe_types::TaxCalculationMode;
/// use std::str::FromStr;
///
/// let mode = TaxCalculationMode::ReverseCharge;
/// assert_eq!(mode.as_str(), "reverse_charge");
/// assert!(mode.is_reverse_charge());
///
/// let parsed = TaxCalculationMode::from_str("standard").unwrap();
/// assert_eq!(parsed, TaxCalculationMode::Standard);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaxCalculationMode {
    /// Standard calculation mode where tax is evaluated based on destination or origin.
    #[default]
    Standard,
    /// Cross-border B2B reverse charge mode where tax liability is shifted to customer (0% rate).
    ReverseCharge,
    /// Customer tax-exempt mode.
    Exempt,
    /// Deferred tax determination mode.
    Deferred,
}

impl TaxCalculationMode {
    /// Returns the static string representation of this calculation mode.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::ReverseCharge => "reverse_charge",
            Self::Exempt => "exempt",
            Self::Deferred => "deferred",
        }
    }

    /// Returns `true` if this mode represents a B2B reverse charge transaction.
    #[must_use]
    pub const fn is_reverse_charge(&self) -> bool {
        matches!(self, Self::ReverseCharge)
    }

    /// Returns `true` if this mode represents an exempt transaction.
    #[must_use]
    pub const fn is_exempt(&self) -> bool {
        matches!(self, Self::Exempt)
    }
}

impl std::fmt::Display for TaxCalculationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for TaxCalculationMode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for TaxCalculationMode {
    type Err = ParseTaxCalculationModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "standard" => Ok(Self::Standard),
            "reverse_charge" => Ok(Self::ReverseCharge),
            "exempt" => Ok(Self::Exempt),
            "deferred" => Ok(Self::Deferred),
            _ => Err(ParseTaxCalculationModeError(s.to_string())),
        }
    }
}

/// Error returned when parsing an unrecognized [`TaxCalculationMode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseTaxCalculationModeError(pub String);

impl std::fmt::Display for ParseTaxCalculationModeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown TaxCalculationMode: '{}'", self.0)
    }
}

impl std::error::Error for ParseTaxCalculationModeError {}

/// Lifecycle status of a [`TaxCalculation`].
///
/// Calculations remain active for 48 hours after creation, after which they expire
/// and can no longer be committed to official tax reporting ledgers via transactions.
///
/// # Example
/// ```
/// use some_stripe_types::TaxCalculationStatus;
/// use std::str::FromStr;
///
/// let status = TaxCalculationStatus::Active;
/// assert_eq!(status.as_str(), "active");
/// assert!(status.is_active());
///
/// let parsed = TaxCalculationStatus::from_str("expired").unwrap();
/// assert_eq!(parsed, TaxCalculationStatus::Expired);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaxCalculationStatus {
    /// Calculation is active and eligible to be committed into a transaction (within 48 hours).
    #[default]
    Active,
    /// Calculation has expired (> 48 hours) and can no longer be committed.
    Expired,
    /// Calculation has already been committed to an official tax ledger transaction.
    Committed,
    /// Calculation is pending determination.
    Pending,
}

impl TaxCalculationStatus {
    /// Returns the static string representation of this status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Committed => "committed",
            Self::Pending => "pending",
        }
    }

    /// Returns `true` if the calculation is currently active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns `true` if the calculation has expired.
    #[must_use]
    pub const fn is_expired(&self) -> bool {
        matches!(self, Self::Expired)
    }

    /// Returns `true` if the calculation has been committed.
    #[must_use]
    pub const fn is_committed(&self) -> bool {
        matches!(self, Self::Committed)
    }
}

impl std::fmt::Display for TaxCalculationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for TaxCalculationStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for TaxCalculationStatus {
    type Err = ParseTaxCalculationStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "expired" => Ok(Self::Expired),
            "committed" => Ok(Self::Committed),
            "pending" => Ok(Self::Pending),
            _ => Err(ParseTaxCalculationStatusError(s.to_string())),
        }
    }
}

/// Error returned when parsing an unrecognized [`TaxCalculationStatus`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseTaxCalculationStatusError(pub String);

impl std::fmt::Display for ParseTaxCalculationStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown TaxCalculationStatus: '{}'", self.0)
    }
}

impl std::error::Error for ParseTaxCalculationStatusError {}

/// Customer tax address used for tax jurisdiction determination.
///
/// # Example
/// ```
/// use some_stripe_types::CustomerTaxAddress;
///
/// let addr = CustomerTaxAddress::new("US")
///     .state("CA")
///     .postal_code("94103")
///     .city("San Francisco");
/// assert_eq!(addr.country.as_str(), "US");
/// assert_eq!(addr.state.as_deref(), Some("CA"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CustomerTaxAddress {
    /// City, district, suburb, town, or village.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,

    /// Two-letter country code (ISO 3166-1 alpha-2).
    pub country: String,

    /// Address line 1 (e.g., street, PO Box, or company name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line1: Option<String>,

    /// Address line 2 (e.g., apartment, suite, unit, or building).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line2: Option<String>,

    /// ZIP or postal code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,

    /// State, county, province, or region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

impl CustomerTaxAddress {
    /// Creates a new `CustomerTaxAddress` with the required ISO 3166-1 alpha-2 country code.
    pub fn new(country: impl Into<String>) -> Self {
        Self {
            country: country.into(),
            city: None,
            line1: None,
            line2: None,
            postal_code: None,
            state: None,
        }
    }

    /// Sets the state, county, province, or region.
    pub fn state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    /// Sets the postal code or ZIP code.
    pub fn postal_code(mut self, postal_code: impl Into<String>) -> Self {
        self.postal_code = Some(postal_code.into());
        self
    }

    /// Sets the city or municipality.
    pub fn city(mut self, city: impl Into<String>) -> Self {
        self.city = Some(city.into());
        self
    }

    /// Sets line 1 of the address.
    pub fn line1(mut self, line1: impl Into<String>) -> Self {
        self.line1 = Some(line1.into());
        self
    }

    /// Sets line 2 of the address.
    pub fn line2(mut self, line2: impl Into<String>) -> Self {
        self.line2 = Some(line2.into());
        self
    }
}

impl From<crate::customer::CustomerAddress> for CustomerTaxAddress {
    fn from(a: crate::customer::CustomerAddress) -> Self {
        Self {
            city: a.city,
            country: a.country.unwrap_or_default(),
            line1: a.line1,
            line2: a.line2,
            postal_code: a.postal_code,
            state: a.state,
        }
    }
}

impl From<CustomerTaxAddress> for crate::customer::CustomerAddress {
    fn from(a: CustomerTaxAddress) -> Self {
        Self {
            city: a.city,
            country: Some(a.country),
            line1: a.line1,
            line2: a.line2,
            postal_code: a.postal_code,
            state: a.state,
        }
    }
}

/// Customer tax identification number details for tax calculation requests.
///
/// # Example
/// ```
/// use some_stripe_types::TaxCalculationCustomerTaxId;
///
/// let id = TaxCalculationCustomerTaxId::new("eu_vat", "DE123456789");
/// assert_eq!(id.tax_id_type.as_str(), "eu_vat");
/// assert_eq!(id.value.as_str(), "DE123456789");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaxCalculationCustomerTaxId {
    /// Type of tax ID (e.g., `eu_vat`, `us_ein`, `gb_vat`).
    #[serde(rename = "type")]
    pub tax_id_type: String,

    /// Value of the tax ID.
    pub value: String,
}

impl TaxCalculationCustomerTaxId {
    /// Creates a new `TaxCalculationCustomerTaxId`.
    pub fn new(tax_id_type: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            tax_id_type: tax_id_type.into(),
            value: value.into(),
        }
    }
}

/// Customer tax details supplied to determine tax liability, jurisdiction, and reverse charge applicability.
///
/// # Example
/// ```
/// use some_stripe_types::{CustomerTaxAddress, CustomerTaxDetails};
///
/// let details = CustomerTaxDetails::new()
///     .address(CustomerTaxAddress::new("US").state("CA").postal_code("94103"))
///     .address_source("shipping");
/// assert_eq!(details.address_source.as_deref(), Some("shipping"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CustomerTaxDetails {
    /// Customer's physical postal address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<CustomerTaxAddress>,

    /// Source of address: `"shipping"` or `"billing"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_source: Option<String>,

    /// Customer's IP address used for geolocation fallback when physical address is omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,

    /// Customer tax IDs (e.g., EU VAT IDs) for B2B reverse charge eligibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_ids: Option<Vec<TaxCalculationCustomerTaxId>>,

    /// Manual taxability override (`"none"`, `"customer_exempt"`, `"reverse_charge"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taxability_override: Option<String>,
}

impl CustomerTaxDetails {
    /// Creates an empty `CustomerTaxDetails`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the customer's postal address.
    pub fn address(mut self, addr: CustomerTaxAddress) -> Self {
        self.address = Some(addr);
        self
    }

    /// Sets the address source (`"shipping"` or `"billing"`).
    pub fn address_source(mut self, source: impl Into<String>) -> Self {
        self.address_source = Some(source.into());
        self
    }

    /// Sets the customer's IP address.
    pub fn ip_address(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    /// Appends a tax ID to the customer's tax ID list.
    pub fn tax_id(mut self, tax_id_type: impl Into<String>, value: impl Into<String>) -> Self {
        let list = self.tax_ids.get_or_insert_with(Vec::new);
        list.push(TaxCalculationCustomerTaxId::new(tax_id_type, value));
        self
    }

    /// Sets the manual taxability override (`"none"`, `"customer_exempt"`, `"reverse_charge"`).
    pub fn taxability_override(mut self, override_val: impl Into<String>) -> Self {
        self.taxability_override = Some(override_val.into());
        self
    }
}

/// Governing tax jurisdiction authority details imposing the tax.
///
/// # Example
/// ```
/// use some_stripe_types::TaxJurisdiction;
///
/// let jurisdiction = TaxJurisdiction {
///     country: "US".into(),
///     state: Some("CA".into()),
///     county: Some("San Francisco".into()),
///     city: None,
///     district: None,
///     level: "county".into(),
///     display_name: "San Francisco County Sales Tax".into(),
/// };
/// assert_eq!(jurisdiction.level.as_str(), "county");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaxJurisdiction {
    /// Two-letter country code (ISO 3166-1 alpha-2).
    pub country: String,

    /// Two-letter subdivision code (e.g., `"CA"`, `"NY"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,

    /// County name if applicable (e.g., `"Los Angeles"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub county: Option<String>,

    /// City name if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,

    /// District or special taxing district name if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub district: Option<String>,

    /// Jurisdictional tier level (e.g., `"country"`, `"state"`, `"county"`, `"city"`, `"district"`).
    pub level: String,

    /// Human-readable display name of the jurisdiction.
    pub display_name: String,
}

/// Granular details of the tax rate applied in a calculation breakdown.
///
/// # Example
/// ```
/// use some_stripe_types::TaxRateDetails;
///
/// let rate = TaxRateDetails {
///     country: Some("US".into()),
///     state: Some("CA".into()),
///     percentage_decimal: "7.25".into(),
///     tax_type: Some("sales_tax".into()),
///     rate_type: Some("percentage".into()),
///     display_name: Some("California State Sales Tax".into()),
///     taxability_reason: Some("standard_rated".into()),
/// };
/// assert_eq!(rate.percentage_decimal.as_str(), "7.25");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaxRateDetails {
    /// Two-letter country code (ISO 3166-1 alpha-2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Two-letter state or province code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,

    /// Rate percentage formatted as a decimal string (e.g., `"7.25"`, `"0.0"`, `"20.0"`).
    pub percentage_decimal: String,

    /// Tax type (e.g., `"sales_tax"`, `"vat"`, `"gst"`, `"amusement_tax"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_type: Option<String>,

    /// Rate type (e.g., `"percentage"`, `"flat_amount"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_type: Option<String>,

    /// Human-readable tax rate display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Taxability reason for this rate (e.g., `"standard_rated"`, `"customer_exempt"`, `"reverse_charge"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taxability_reason: Option<String>,
}

/// Itemized breakdown of taxes calculated per jurisdiction.
///
/// # Example
/// ```
/// use some_stripe_types::{TaxBreakdown, TaxJurisdiction};
///
/// let breakdown = TaxBreakdown {
///     amount: 725,
///     inclusive: false,
///     jurisdiction: TaxJurisdiction {
///         country: "US".into(),
///         state: Some("CA".into()),
///         level: "state".into(),
///         display_name: "California".into(),
///         ..Default::default()
///     },
///     taxable_amount: Some(10000),
///     ..Default::default()
/// };
/// assert_eq!(breakdown.amount, 725);
/// assert!(!breakdown.inclusive);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaxBreakdown {
    /// Tax amount collected for this breakdown in minor currency units (e.g., cents).
    pub amount: i64,

    /// Whether the tax amount is included in the item price (`true`) or added on top (`false`).
    pub inclusive: bool,

    /// The jurisdiction levying this tax.
    pub jurisdiction: TaxJurisdiction,

    /// Rate details applied by this jurisdiction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_rate_details: Option<TaxRateDetails>,

    /// Reason explaining why tax was calculated or exempt (e.g., `"standard_rated"`, `"reverse_charge"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taxability_reason: Option<String>,

    /// Portion of the line item base amount subject to this tax rate in minor currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taxable_amount: Option<i64>,

    /// Sourcing rule applied to determine jurisdiction: `"destination"` or `"origin"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sourcing: Option<String>,
}

impl TaxBreakdown {
    /// Returns `true` if this breakdown entry represents a B2B reverse charge.
    #[must_use]
    pub fn is_reverse_charge(&self) -> bool {
        self.taxability_reason
            .as_deref()
            .is_some_and(|r| r.eq_ignore_ascii_case("reverse_charge"))
            || self
                .tax_rate_details
                .as_ref()
                .and_then(|d| d.taxability_reason.as_deref())
                .is_some_and(|r| r.eq_ignore_ascii_case("reverse_charge"))
    }
}

/// Summary of amount and tax metrics for a line item or calculation.
///
/// # Example
/// ```
/// use some_stripe_types::TaxAmountDetails;
///
/// let details = TaxAmountDetails {
///     amount_tax: 850,
///     tax_amount_exclusive: 850,
///     tax_amount_inclusive: 0,
///     taxable_amount: Some(10000),
///     tax_rate_details: None,
/// };
/// assert_eq!(details.amount_tax, 850);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaxAmountDetails {
    /// Total tax amount in minor currency units.
    pub amount_tax: i64,

    /// Exclusive tax portion in minor currency units.
    #[serde(default)]
    pub tax_amount_exclusive: i64,

    /// Inclusive tax portion in minor currency units.
    #[serde(default)]
    pub tax_amount_inclusive: i64,

    /// Total taxable base amount subject to tax in minor currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taxable_amount: Option<i64>,

    /// Optional rate details associated with this amount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_rate_details: Option<TaxRateDetails>,
}

/// Itemized line item in a Stripe [`TaxCalculation`].
///
/// # Example
/// ```
/// use some_stripe_types::TaxCalculationLineItem;
///
/// let json = r#"{
///     "id": "tax_li_123",
///     "object": "tax.calculation_line_item",
///     "amount": 10000,
///     "amount_tax": 725,
///     "reference": "item_1"
/// }"#;
/// let item: TaxCalculationLineItem = serde_json::from_str(json).unwrap();
/// assert_eq!(item.id.as_str(), "tax_li_123");
/// assert_eq!(item.amount, 10000);
/// assert_eq!(item.amount_tax, 725);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxCalculationLineItem {
    /// Unique identifier for the calculation line item (`tax.calculation_line_item`).
    pub id: String,

    /// String representing the object's type, always `tax.calculation_line_item`.
    #[serde(default = "default_line_item_object")]
    pub object: String,

    /// The line item base amount in minor currency units (e.g., cents).
    pub amount: i64,

    /// The calculated tax amount for this line item in minor currency units.
    pub amount_tax: i64,

    /// Unique reference string for this line item.
    pub reference: String,

    /// Specifies whether price is inclusive or exclusive of tax (`"exclusive"`, `"inclusive"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_behavior: Option<String>,

    /// The Stripe Tax Code for this line item (e.g., `"txcd_10000000"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_code: Option<String>,

    /// Stripe product ID associated with this line item if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,

    /// Quantity of items.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i64>,

    /// Itemized breakdown of taxes calculated for this specific line item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_breakdown: Option<Vec<TaxBreakdown>>,

    /// Summary of amount and tax metrics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_details: Option<TaxAmountDetails>,

    /// Metadata key-value pairs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl TaxCalculationLineItem {
    /// Returns the line item's unique identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the line item reference.
    #[must_use]
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// Returns the itemized tax breakdowns as a borrowed slice.
    #[must_use]
    pub fn tax_breakdown(&self) -> Option<&[TaxBreakdown]> {
        self.tax_breakdown.as_deref()
    }
}

/// Key extractor function for [`TaxCalculationLineItem`] in paginated streams.
pub fn tax_calculation_line_item_id(item: &TaxCalculationLineItem) -> &str {
    item.id.as_str()
}

/// Paginated list of [`TaxCalculationLineItem`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaxCalculationLineItemList {
    /// String representing the object's type, always `list`.
    #[serde(default = "default_list_object")]
    pub object: String,

    /// List of line items.
    pub data: Vec<TaxCalculationLineItem>,

    /// True if there are more items to paginate through.
    pub has_more: bool,

    /// URL to fetch next page.
    pub url: String,
}

/// Shipping cost details and associated taxes in a [`TaxCalculation`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaxShippingCost {
    /// The shipping cost amount in minor currency units.
    pub amount: i64,

    /// The calculated tax on shipping in minor currency units.
    pub amount_tax: i64,

    /// Shipping tax code if specified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_code: Option<String>,

    /// Tax behavior: `"inclusive"` or `"exclusive"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_behavior: Option<String>,

    /// Itemized tax breakdown for shipping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_breakdown: Option<Vec<TaxBreakdown>>,
}

/// Real-time Stripe Tax Calculation object (`tax.calculation`).
///
/// Generated via `POST /v1/tax/calculations` or retrieved via `GET /v1/tax/calculations/{id}`.
///
/// # Example
/// ```
/// use some_stripe_types::{CustomerTaxAddress, CustomerTaxDetails, TaxCalculation};
///
/// let json = r#"{
///     "id": "taxcalc_12345",
///     "object": "tax.calculation",
///     "amount_total": 10725,
///     "currency": "usd",
///     "customer_details": {
///         "address": {
///             "country": "US",
///             "state": "CA",
///             "postal_code": "94103"
///         },
///         "address_source": "shipping"
///     },
///     "tax_amount_exclusive": 725,
///     "tax_amount_inclusive": 0,
///     "tax_breakdown": []
/// }"#;
/// let calc: TaxCalculation = serde_json::from_str(json).unwrap();
/// assert_eq!(calc.id.as_str(), "taxcalc_12345");
/// assert_eq!(calc.amount_total, 10725);
/// assert_eq!(calc.tax_amount_exclusive, 725);
/// assert_eq!(calc.total_tax(), 725);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxCalculation {
    /// Unique identifier for the tax calculation (`taxcalc_...`).
    pub id: String,

    /// Object type, always `tax.calculation`.
    #[serde(default = "default_tax_calculation_object")]
    pub object: String,

    /// Total amount after taxes in minor currency units (e.g., cents).
    pub amount_total: i64,

    /// Three-letter ISO currency code (lowercase, e.g., `"usd"`, `"eur"`).
    pub currency: String,

    /// Stripe Customer ID if associated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,

    /// Customer details used to compute taxes.
    pub customer_details: CustomerTaxDetails,

    /// Timestamp at which the calculation expires (48 hours after creation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,

    /// Expanded line items associated with this calculation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_items: Option<TaxCalculationLineItemList>,

    /// Has the value `true` if livemode, `false` if testmode.
    #[serde(default)]
    pub livemode: bool,

    /// Shipping cost details and associated taxes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping_cost: Option<TaxShippingCost>,

    /// The total amount of tax to be collected on top of prices in minor currency units.
    pub tax_amount_exclusive: i64,

    /// The total amount of tax included in prices in minor currency units.
    pub tax_amount_inclusive: i64,

    /// Itemized breakdown of taxes calculated across all line items and shipping.
    #[serde(default)]
    pub tax_breakdown: Vec<TaxBreakdown>,

    /// Timestamp of when the calculation was evaluated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_date: Option<i64>,

    /// Status of the calculation (`active`, `expired`, `committed`, `pending`).
    #[serde(default)]
    pub status: TaxCalculationStatus,

    /// Calculation mode (`standard`, `reverse_charge`, `exempt`, `deferred`).
    #[serde(default)]
    pub calculation_mode: TaxCalculationMode,
}

impl TaxCalculation {
    /// Returns the unique calculation identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the total tax amount (exclusive + inclusive) in minor currency units.
    #[must_use]
    pub fn total_tax(&self) -> i64 {
        self.tax_amount_exclusive + self.tax_amount_inclusive
    }

    /// Returns `true` if this calculation represents a B2B reverse charge transaction.
    #[must_use]
    pub fn is_reverse_charge(&self) -> bool {
        self.calculation_mode.is_reverse_charge()
            || self
                .customer_details
                .taxability_override
                .as_deref()
                .is_some_and(|o| o.eq_ignore_ascii_case("reverse_charge"))
            || self.tax_breakdown.iter().any(|b| b.is_reverse_charge())
    }

    /// Returns `true` if the calculation has expired based on current timestamp or status.
    #[must_use]
    pub fn is_expired(&self, current_timestamp: i64) -> bool {
        if self.status.is_expired() {
            return true;
        }
        match self.expires_at {
            Some(exp) => current_timestamp >= exp,
            None => false,
        }
    }

    /// Returns a borrowed slice of the expanded line items, if present.
    #[must_use]
    pub fn line_items(&self) -> Option<&[TaxCalculationLineItem]> {
        self.line_items.as_ref().map(|l| l.data.as_slice())
    }

    /// Returns a borrowed slice of the tax breakdown.
    #[must_use]
    pub fn tax_breakdown(&self) -> &[TaxBreakdown] {
        &self.tax_breakdown
    }
}

/// Parameters for a line item in a tax calculation request.
///
/// # Example
/// ```
/// use some_stripe_types::TaxLineItemParam;
///
/// let item = TaxLineItemParam::new(10000, "item_1")
///     .tax_code("txcd_10000000")
///     .quantity(1);
/// assert_eq!(item.amount, 10000);
/// assert_eq!(item.reference.as_str(), "item_1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaxLineItemParam {
    /// Amount in minor currency units (e.g., cents for USD).
    pub amount: i64,

    /// Unique identifier for this line item in the calculation.
    pub reference: String,

    /// Quantity of items being purchased.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i64>,

    /// Stripe Tax Code (e.g., `"txcd_10000000"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_code: Option<String>,

    /// Specifies whether amount includes tax (`"exclusive"` or `"inclusive"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_behavior: Option<String>,

    /// Stripe product ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,

    /// Key-value metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl TaxLineItemParam {
    /// Creates a new `TaxLineItemParam` with amount and unique reference.
    pub fn new(amount: i64, reference: impl Into<String>) -> Self {
        Self {
            amount,
            reference: reference.into(),
            quantity: None,
            tax_code: None,
            tax_behavior: None,
            product: None,
            metadata: None,
        }
    }

    /// Sets the quantity.
    pub fn quantity(mut self, qty: i64) -> Self {
        self.quantity = Some(qty);
        self
    }

    /// Sets the Stripe Tax Code.
    pub fn tax_code(mut self, code: impl Into<String>) -> Self {
        self.tax_code = Some(code.into());
        self
    }

    /// Sets the tax behavior (`"inclusive"` or `"exclusive"`).
    pub fn tax_behavior(mut self, behavior: impl Into<String>) -> Self {
        self.tax_behavior = Some(behavior.into());
        self
    }

    /// Sets the product ID.
    pub fn product(mut self, prod: impl Into<String>) -> Self {
        self.product = Some(prod.into());
        self
    }
}

/// Shipping cost parameters in a tax calculation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaxShippingCostParam {
    /// The shipping cost amount in minor currency units.
    pub amount: i64,

    /// Stripe Tax Code for shipping if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_code: Option<String>,

    /// Tax behavior: `"inclusive"` or `"exclusive"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_behavior: Option<String>,
}

impl TaxShippingCostParam {
    /// Creates a new `TaxShippingCostParam`.
    pub fn new(amount: i64) -> Self {
        Self {
            amount,
            tax_code: None,
            tax_behavior: None,
        }
    }

    /// Sets the shipping tax code.
    pub fn tax_code(mut self, code: impl Into<String>) -> Self {
        self.tax_code = Some(code.into());
        self
    }

    /// Sets the tax behavior (`"inclusive"` or `"exclusive"`).
    pub fn tax_behavior(mut self, behavior: impl Into<String>) -> Self {
        self.tax_behavior = Some(behavior.into());
        self
    }
}

/// Parameters for creating a new Stripe Tax Calculation (`POST /v1/tax/calculations`).
///
/// # Example
/// ```
/// use some_stripe_types::{
///     CreateTaxCalculationParams, CustomerTaxAddress, CustomerTaxDetails, TaxLineItemParam,
/// };
///
/// let params = CreateTaxCalculationParams::new("usd")
///     .customer_details(
///         CustomerTaxDetails::new()
///             .address(CustomerTaxAddress::new("US").state("CA").postal_code("94103"))
///             .address_source("shipping"),
///     )
///     .line_item(TaxLineItemParam::new(10000, "item_1").tax_code("txcd_10000000"));
///
/// assert_eq!(params.currency.as_str(), "usd");
/// assert_eq!(params.line_items.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CreateTaxCalculationParams {
    /// Three-letter ISO currency code.
    pub currency: String,

    /// Customer details including postal address, IP address, and tax IDs.
    pub customer_details: CustomerTaxDetails,

    /// Array of line items being purchased.
    pub line_items: Vec<TaxLineItemParam>,

    /// Existing customer ID if associated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,

    /// Timestamp for tax calculation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_date: Option<i64>,

    /// Shipping cost parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping_cost: Option<TaxShippingCostParam>,

    /// Fields to expand on response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expand: Option<Vec<String>>,

    /// Client-side idempotency key (Gate G2).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateTaxCalculationParams {
    /// Creates a new `CreateTaxCalculationParams` with the specified currency.
    pub fn new(currency: impl Into<String>) -> Self {
        Self {
            currency: currency.into(),
            customer_details: CustomerTaxDetails::default(),
            line_items: Vec::new(),
            customer: None,
            tax_date: None,
            shipping_cost: None,
            expand: None,
            idempotency_key: None,
        }
    }

    /// Sets the customer details.
    pub fn customer_details(mut self, details: CustomerTaxDetails) -> Self {
        self.customer_details = details;
        self
    }

    /// Adds a line item to the calculation request.
    pub fn line_item(mut self, item: TaxLineItemParam) -> Self {
        self.line_items.push(item);
        self
    }

    /// Sets the customer ID.
    pub fn customer(mut self, cus: impl Into<String>) -> Self {
        self.customer = Some(cus.into());
        self
    }

    /// Sets the tax date timestamp.
    pub fn tax_date(mut self, date: i64) -> Self {
        self.tax_date = Some(date);
        self
    }

    /// Sets the shipping cost.
    pub fn shipping_cost(mut self, cost: TaxShippingCostParam) -> Self {
        self.shipping_cost = Some(cost);
        self
    }

    /// Adds an expansion field.
    pub fn expand(mut self, field: impl Into<String>) -> Self {
        let list = self.expand.get_or_insert_with(Vec::new);
        list.push(field.into());
        self
    }

    /// Sets the client-supplied idempotency key.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for listing line items of a tax calculation (`GET /v1/tax/calculations/{id}/line_items`).
///
/// # Example
/// ```
/// use some_stripe_types::ListTaxCalculationLineItemsParams;
///
/// let params = ListTaxCalculationLineItemsParams::new()
///     .limit(20)
///     .starting_after("tax_li_123");
/// assert_eq!(params.limit, Some(20));
/// assert_eq!(params.starting_after.as_deref(), Some("tax_li_123"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ListTaxCalculationLineItemsParams {
    /// Limit on number of line items to return (between 1 and 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Cursor for pagination: fetch items starting after this ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// Cursor for pagination: fetch items ending before this ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListTaxCalculationLineItemsParams {
    /// Creates an empty `ListTaxCalculationLineItemsParams`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the pagination page size limit.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the `starting_after` cursor.
    pub fn starting_after(mut self, id: impl Into<String>) -> Self {
        self.starting_after = Some(id.into());
        self
    }

    /// Sets the `ending_before` cursor.
    pub fn ending_before(mut self, id: impl Into<String>) -> Self {
        self.ending_before = Some(id.into());
        self
    }
}

/// Statutory compliance error for EU VAT Directive 2006/112/EC Article 196 (Reverse Charge Mechanism).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EuVatReverseChargeError {
    /// Non-zero tax amount charged when reverse charge applies.
    #[error("Article 196 statutory violation: tax amount must be zero for reverse charge, got {0}")]
    NonZeroTaxAmount(i64),

    /// Missing customer tax ID (VAT identification number).
    #[error(
        "Article 196 statutory violation: customer VAT identification number required for B2B reverse charge"
    )]
    MissingCustomerVatId,

    /// Missing reverse charge designation.
    #[error(
        "Article 196 statutory violation: calculation or breakdown must record 'reverse_charge'"
    )]
    MissingReverseChargeDesignation,

    /// Domestic supply not eligible for cross-border EU reverse charge under Article 196.
    #[error(
        "Article 196 statutory violation: supplier and customer must be in different EU member states"
    )]
    DomesticTransactionNotEligible,
}

/// Statutory compliance validator for EU VAT Directive 2006/112/EC Article 196 (Reverse Charge Mechanism).
///
/// Under Directive 2006/112/EC Art. 196, for cross-border B2B supplies of services/goods within
/// the European Union, the person liable to pay VAT is the taxable recipient (customer).
/// Therefore, the calculation must satisfy:
/// 1. **Zero Tax Amount**: `total_tax()` == 0.
/// 2. **Cross-Border**: Customer country differs from supplier country.
/// 3. **VAT ID Recording**: Customer details record a valid VAT identification number.
/// 4. **Reverse Charge Designation**: Mode, override, or tax breakdown explicitly designates reverse charge.
///
/// # Example
/// ```
/// use some_stripe_types::{
///     CustomerTaxAddress, CustomerTaxDetails, TaxBreakdown, TaxCalculation,
///     TaxCalculationCustomerTaxId, TaxCalculationMode, TaxJurisdiction,
///     validate_eu_vat_reverse_charge,
/// };
///
/// let calculation = TaxCalculation {
///     id: "taxcalc_eu_b2b".into(),
///     object: "tax.calculation".into(),
///     amount_total: 50000,
///     currency: "eur".into(),
///     customer: None,
///     customer_details: CustomerTaxDetails::new()
///         .address(CustomerTaxAddress::new("FR").city("Paris"))
///         .address_source("shipping")
///         .tax_id("eu_vat", "FR12345678901")
///         .taxability_override("reverse_charge"),
///     expires_at: Some(1730000000),
///     line_items: None,
///     livemode: false,
///     shipping_cost: None,
///     tax_amount_exclusive: 0,
///     tax_amount_inclusive: 0,
///     tax_breakdown: vec![TaxBreakdown {
///         amount: 0,
///         inclusive: false,
///         jurisdiction: TaxJurisdiction {
///             country: "FR".into(),
///             level: "country".into(),
///             display_name: "France".into(),
///             ..Default::default()
///         },
///         taxability_reason: Some("reverse_charge".into()),
///         taxable_amount: Some(50000),
///         ..Default::default()
///     }],
///     tax_date: Some(1728345600),
///     status: Default::default(),
///     calculation_mode: TaxCalculationMode::ReverseCharge,
/// };
///
/// // Supplier in Ireland (IE), customer in France (FR) with valid FR VAT ID:
/// assert!(validate_eu_vat_reverse_charge(&calculation, "IE").is_ok());
/// ```
pub fn validate_eu_vat_reverse_charge(
    calculation: &TaxCalculation,
    supplier_country: &str,
) -> Result<(), EuVatReverseChargeError> {
    if calculation.total_tax() != 0 {
        return Err(EuVatReverseChargeError::NonZeroTaxAmount(
            calculation.total_tax(),
        ));
    }

    let customer_country = calculation
        .customer_details
        .address
        .as_ref()
        .map(|a| a.country.as_str())
        .unwrap_or("");

    if !customer_country.is_empty() && customer_country.eq_ignore_ascii_case(supplier_country) {
        return Err(EuVatReverseChargeError::DomesticTransactionNotEligible);
    }

    let has_vat_id = calculation
        .customer_details
        .tax_ids
        .as_deref()
        .is_some_and(|ids| !ids.is_empty() && ids.iter().any(|id| !id.value.is_empty()));

    if !has_vat_id {
        return Err(EuVatReverseChargeError::MissingCustomerVatId);
    }

    let is_designated = calculation.calculation_mode.is_reverse_charge()
        || calculation
            .customer_details
            .taxability_override
            .as_deref()
            .is_some_and(|o| o.eq_ignore_ascii_case("reverse_charge"))
        || calculation
            .tax_breakdown
            .iter()
            .any(|b| b.is_reverse_charge());

    if !is_designated {
        return Err(EuVatReverseChargeError::MissingReverseChargeDesignation);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tax_calculation_mode_conversions() {
        assert_eq!(TaxCalculationMode::Standard.as_str(), "standard");
        assert_eq!(TaxCalculationMode::ReverseCharge.as_str(), "reverse_charge");
        assert_eq!(TaxCalculationMode::Exempt.as_str(), "exempt");
        assert_eq!(TaxCalculationMode::Deferred.as_str(), "deferred");

        assert!(TaxCalculationMode::ReverseCharge.is_reverse_charge());
        assert!(!TaxCalculationMode::Standard.is_reverse_charge());
        assert!(TaxCalculationMode::Exempt.is_exempt());

        let parsed: TaxCalculationMode = "reverse_charge".parse().unwrap();
        assert_eq!(parsed, TaxCalculationMode::ReverseCharge);
        assert!("invalid_mode".parse::<TaxCalculationMode>().is_err());
    }

    #[test]
    fn test_tax_calculation_status_conversions() {
        assert_eq!(TaxCalculationStatus::Active.as_str(), "active");
        assert_eq!(TaxCalculationStatus::Expired.as_str(), "expired");
        assert_eq!(TaxCalculationStatus::Committed.as_str(), "committed");
        assert_eq!(TaxCalculationStatus::Pending.as_str(), "pending");

        assert!(TaxCalculationStatus::Active.is_active());
        assert!(TaxCalculationStatus::Expired.is_expired());
        assert!(TaxCalculationStatus::Committed.is_committed());

        let parsed: TaxCalculationStatus = "active".parse().unwrap();
        assert_eq!(parsed, TaxCalculationStatus::Active);
        assert!("unknown".parse::<TaxCalculationStatus>().is_err());
    }

    #[test]
    fn test_us_multi_jurisdiction_tax_breakdown() {
        let json = r#"{
            "id": "taxcalc_us_ca_sf",
            "object": "tax.calculation",
            "amount_total": 10863,
            "currency": "usd",
            "customer_details": {
                "address": {
                    "city": "San Francisco",
                    "country": "US",
                    "line1": "123 Market St",
                    "postal_code": "94103",
                    "state": "CA"
                },
                "address_source": "shipping"
            },
            "tax_amount_exclusive": 863,
            "tax_amount_inclusive": 0,
            "tax_breakdown": [
                {
                    "amount": 600,
                    "inclusive": false,
                    "jurisdiction": {
                        "country": "US",
                        "state": "CA",
                        "level": "state",
                        "display_name": "California State Tax"
                    },
                    "tax_rate_details": {
                        "country": "US",
                        "state": "CA",
                        "percentage_decimal": "6.0",
                        "tax_type": "sales_tax",
                        "rate_type": "percentage"
                    },
                    "taxable_amount": 10000,
                    "sourcing": "destination"
                },
                {
                    "amount": 125,
                    "inclusive": false,
                    "jurisdiction": {
                        "country": "US",
                        "state": "CA",
                        "county": "San Francisco",
                        "level": "county",
                        "display_name": "San Francisco County Tax"
                    },
                    "tax_rate_details": {
                        "percentage_decimal": "1.25",
                        "tax_type": "sales_tax"
                    },
                    "taxable_amount": 10000,
                    "sourcing": "destination"
                },
                {
                    "amount": 138,
                    "inclusive": false,
                    "jurisdiction": {
                        "country": "US",
                        "state": "CA",
                        "district": "BART District",
                        "level": "district",
                        "display_name": "Bay Area Rapid Transit District"
                    },
                    "tax_rate_details": {
                        "percentage_decimal": "1.375",
                        "tax_type": "sales_tax"
                    },
                    "taxable_amount": 10000,
                    "sourcing": "destination"
                }
            ],
            "tax_date": 1728345600,
            "status": "active",
            "calculation_mode": "standard"
        }"#;

        let calc: TaxCalculation = serde_json::from_str(json).unwrap();
        assert_eq!(calc.id, "taxcalc_us_ca_sf");
        assert_eq!(calc.amount_total, 10863);
        assert_eq!(calc.tax_amount_exclusive, 863);
        assert_eq!(calc.tax_breakdown.len(), 3);

        let levels: Vec<&str> = calc
            .tax_breakdown
            .iter()
            .map(|b| b.jurisdiction.level.as_str())
            .collect();
        assert_eq!(levels, vec!["state", "county", "district"]);

        let total_itemized: i64 = calc.tax_breakdown.iter().map(|b| b.amount).sum();
        assert_eq!(total_itemized, 863);
        assert_eq!(calc.total_tax(), 863);
    }

    #[test]
    fn test_eu_vat_reverse_charge_directive_2006_112_ec_art_196() {
        let mut calc = TaxCalculation {
            id: "taxcalc_eu_b2b_reverse".into(),
            object: "tax.calculation".into(),
            amount_total: 100000,
            currency: "eur".into(),
            customer: None,
            customer_details: CustomerTaxDetails::new()
                .address(CustomerTaxAddress::new("DE").city("Berlin"))
                .address_source("shipping")
                .tax_id("eu_vat", "DE987654321")
                .taxability_override("reverse_charge"),
            expires_at: Some(1730000000),
            line_items: None,
            livemode: false,
            shipping_cost: None,
            tax_amount_exclusive: 0,
            tax_amount_inclusive: 0,
            tax_breakdown: vec![TaxBreakdown {
                amount: 0,
                inclusive: false,
                jurisdiction: TaxJurisdiction {
                    country: "DE".into(),
                    level: "country".into(),
                    display_name: "Germany".into(),
                    ..Default::default()
                },
                tax_rate_details: Some(TaxRateDetails {
                    country: Some("DE".into()),
                    percentage_decimal: "0.0".into(),
                    tax_type: Some("vat".into()),
                    rate_type: Some("percentage".into()),
                    taxability_reason: Some("reverse_charge".into()),
                    ..Default::default()
                }),
                taxability_reason: Some("reverse_charge".into()),
                taxable_amount: Some(100000),
                sourcing: Some("destination".into()),
            }],
            tax_date: Some(1728345600),
            status: TaxCalculationStatus::Active,
            calculation_mode: TaxCalculationMode::ReverseCharge,
        };

        // Supplier in France (FR), customer in Germany (DE) with DE VAT ID -> Valid Art 196 reverse charge
        assert!(validate_eu_vat_reverse_charge(&calc, "FR").is_ok());

        // Statutory failure 1: Domestic transaction (Supplier DE, Customer DE) cannot use cross-border Art 196
        let domestic_res = validate_eu_vat_reverse_charge(&calc, "DE");
        assert_eq!(
            domestic_res.unwrap_err(),
            EuVatReverseChargeError::DomesticTransactionNotEligible
        );

        // Statutory failure 2: Non-zero tax charged under reverse charge
        calc.tax_amount_exclusive = 19000;
        let nonzero_res = validate_eu_vat_reverse_charge(&calc, "FR");
        assert_eq!(
            nonzero_res.unwrap_err(),
            EuVatReverseChargeError::NonZeroTaxAmount(19000)
        );
        calc.tax_amount_exclusive = 0;

        // Statutory failure 3: Missing VAT ID
        calc.customer_details.tax_ids = None;
        let missing_id_res = validate_eu_vat_reverse_charge(&calc, "FR");
        assert_eq!(
            missing_id_res.unwrap_err(),
            EuVatReverseChargeError::MissingCustomerVatId
        );
    }

    #[test]
    fn test_create_tax_calculation_params_builder() {
        let params = CreateTaxCalculationParams::new("usd")
            .customer("cus_test_123")
            .customer_details(
                CustomerTaxDetails::new()
                    .address(
                        CustomerTaxAddress::new("US")
                            .state("CA")
                            .postal_code("94103")
                            .city("San Francisco"),
                    )
                    .address_source("shipping"),
            )
            .line_item(
                TaxLineItemParam::new(5000, "item_ref_1")
                    .quantity(2)
                    .tax_code("txcd_10000000")
                    .tax_behavior("exclusive"),
            )
            .shipping_cost(TaxShippingCostParam::new(500).tax_behavior("exclusive"))
            .idempotency_key("req_unique_tax_calc_1");

        assert_eq!(params.currency, "usd");
        assert_eq!(params.customer.as_deref(), Some("cus_test_123"));
        assert_eq!(params.line_items.len(), 1);
        assert_eq!(params.line_items[0].amount, 5000);
        assert_eq!(
            params.idempotency_key.as_deref(),
            Some("req_unique_tax_calc_1")
        );
    }

    #[test]
    fn test_line_item_key_extractor() {
        let item = TaxCalculationLineItem {
            id: "tax_li_999".into(),
            object: "tax.calculation_line_item".into(),
            amount: 2500,
            amount_tax: 200,
            reference: "ref_1".into(),
            tax_behavior: Some("exclusive".into()),
            tax_code: None,
            product: None,
            quantity: Some(1),
            tax_breakdown: None,
            amount_details: None,
            metadata: None,
        };
        assert_eq!(tax_calculation_line_item_id(&item), "tax_li_999");
    }
}
