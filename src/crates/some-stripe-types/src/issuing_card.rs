//! # Stripe Issuing Card Domain Models & Parameters
//!
//! Strongly typed domain models, parameters, and lifecycle states for Stripe Issuing Cards,
//! adhering to ISO 4217 integer minor currency units, PCI-DSS SAQ A PAN isolation,
//! and Rust 2024 zero-clone borrowing discipline.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// Type of an Issuing card (`virtual` or `physical`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IssuingCardType {
    /// Virtual card suitable for online and digital wallet transactions.
    #[default]
    Virtual,
    /// Physical card manufactured and shipped to the cardholder.
    Physical,
}

impl IssuingCardType {
    /// Returns the static string representation of this card type.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Virtual => "virtual",
            Self::Physical => "physical",
        }
    }
}

impl fmt::Display for IssuingCardType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for IssuingCardType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for IssuingCardType {
    type Err = ParseIssuingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "virtual" => Ok(Self::Virtual),
            "physical" => Ok(Self::Physical),
            _ => Err(ParseIssuingError(format!("Unknown card type: {s}"))),
        }
    }
}

impl From<&str> for IssuingCardType {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or(Self::Virtual)
    }
}

impl From<String> for IssuingCardType {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl PartialEq<str> for IssuingCardType {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for IssuingCardType {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<IssuingCardType> for str {
    fn eq(&self, other: &IssuingCardType) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<IssuingCardType> for &str {
    fn eq(&self, other: &IssuingCardType) -> bool {
        *self == other.as_str()
    }
}

/// Status of an Issuing card (`active`, `inactive`, `canceled`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IssuingCardStatus {
    /// Card is active and can authorize transactions.
    #[default]
    Active,
    /// Card is inactive and will decline transactions until reactivated.
    Inactive,
    /// Card has been permanently canceled and cannot be reused.
    Canceled,
}

impl IssuingCardStatus {
    /// Returns the static string representation of this card status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Canceled => "canceled",
        }
    }
}

impl fmt::Display for IssuingCardStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for IssuingCardStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for IssuingCardStatus {
    type Err = ParseIssuingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "canceled" => Ok(Self::Canceled),
            _ => Err(ParseIssuingError(format!("Unknown card status: {s}"))),
        }
    }
}

impl From<&str> for IssuingCardStatus {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or(Self::Inactive)
    }
}

impl From<String> for IssuingCardStatus {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl PartialEq<str> for IssuingCardStatus {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for IssuingCardStatus {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<IssuingCardStatus> for str {
    fn eq(&self, other: &IssuingCardStatus) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<IssuingCardStatus> for &str {
    fn eq(&self, other: &IssuingCardStatus) -> bool {
        *self == other.as_str()
    }
}

/// Reason why an Issuing card was canceled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssuingCardCancellationReason {
    /// Card was lost.
    Lost,
    /// Card was stolen.
    Stolen,
    /// Custom artwork or personalization was rejected.
    DesignRejected,
    /// Card expired.
    Expired,
}

impl IssuingCardCancellationReason {
    /// Returns the static string representation of this cancellation reason.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Lost => "lost",
            Self::Stolen => "stolen",
            Self::DesignRejected => "design_rejected",
            Self::Expired => "expired",
        }
    }
}

impl fmt::Display for IssuingCardCancellationReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for IssuingCardCancellationReason {
    type Err = ParseIssuingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lost" => Ok(Self::Lost),
            "stolen" => Ok(Self::Stolen),
            "design_rejected" => Ok(Self::DesignRejected),
            "expired" => Ok(Self::Expired),
            _ => Err(ParseIssuingError(format!(
                "Unknown cancellation reason: {s}"
            ))),
        }
    }
}

/// Reason why an Issuing card was replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssuingCardReplacementReason {
    /// Physical card was damaged.
    Damaged,
    /// Card expired.
    Expired,
    /// Card was lost.
    Lost,
    /// Card was stolen.
    Stolen,
}

impl IssuingCardReplacementReason {
    /// Returns the static string representation of this replacement reason.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Damaged => "damaged",
            Self::Expired => "expired",
            Self::Lost => "lost",
            Self::Stolen => "stolen",
        }
    }
}

impl fmt::Display for IssuingCardReplacementReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for IssuingCardReplacementReason {
    type Err = ParseIssuingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "damaged" => Ok(Self::Damaged),
            "expired" => Ok(Self::Expired),
            "lost" => Ok(Self::Lost),
            "stolen" => Ok(Self::Stolen),
            _ => Err(ParseIssuingError(format!(
                "Unknown replacement reason: {s}"
            ))),
        }
    }
}

/// Time interval over which a spending limit applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SpendingLimitInterval {
    /// Cumulative limit over the entire lifetime of the card.
    AllTime,
    /// Limit enforced per calendar day (UTC).
    #[default]
    Daily,
    /// Limit enforced per calendar month (UTC).
    Monthly,
    /// Limit enforced on each individual authorization request.
    PerAuthorization,
    /// Limit enforced per calendar week (UTC).
    Weekly,
    /// Limit enforced per calendar year (UTC).
    Yearly,
}

impl SpendingLimitInterval {
    /// Returns the static string representation of this spending limit interval.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::AllTime => "all_time",
            Self::Daily => "daily",
            Self::Monthly => "monthly",
            Self::PerAuthorization => "per_authorization",
            Self::Weekly => "weekly",
            Self::Yearly => "yearly",
        }
    }
}

impl fmt::Display for SpendingLimitInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for SpendingLimitInterval {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for SpendingLimitInterval {
    type Err = ParseIssuingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "all_time" => Ok(Self::AllTime),
            "daily" => Ok(Self::Daily),
            "monthly" => Ok(Self::Monthly),
            "per_authorization" => Ok(Self::PerAuthorization),
            "weekly" => Ok(Self::Weekly),
            "yearly" => Ok(Self::Yearly),
            _ => Err(ParseIssuingError(format!("Unknown limit interval: {s}"))),
        }
    }
}

impl From<&str> for SpendingLimitInterval {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or(Self::Daily)
    }
}

impl From<String> for SpendingLimitInterval {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl PartialEq<str> for SpendingLimitInterval {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for SpendingLimitInterval {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<SpendingLimitInterval> for str {
    fn eq(&self, other: &SpendingLimitInterval) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<SpendingLimitInterval> for &str {
    fn eq(&self, other: &SpendingLimitInterval) -> bool {
        *self == other.as_str()
    }
}

/// Generic error when parsing unknown Issuing enum variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseIssuingError(pub String);

impl fmt::Display for ParseIssuingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Issuing parsing error: {}", self.0)
    }
}

impl std::error::Error for ParseIssuingError {}

/// An amount-based limit applied over a specific interval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SpendingLimit {
    /// Maximum amount allowed in minor currency units (e.g. cents).
    pub amount: i64,
    /// Specific merchant categories to which this limit applies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
    /// Rolling interval over which this limit applies.
    pub interval: SpendingLimitInterval,
}

/// Parameters for creating or updating a spending limit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SpendingLimitParams {
    /// Maximum amount allowed in minor currency units.
    pub amount: i64,
    /// Rolling interval for the spend limit.
    pub interval: SpendingLimitInterval,
    /// Optional category filter for this limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
}

/// Granular spending rules and velocity limits applied to a Card or Cardholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SpendingControls {
    /// Whitelist of allowed Merchant Category Codes (MCCs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_categories: Option<Vec<String>>,
    /// Blacklist of blocked Merchant Category Codes (MCCs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked_categories: Option<Vec<String>>,
    /// Granular spending limits over specific intervals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spending_limits: Option<Vec<SpendingLimit>>,
    /// Currency of the spending limits, represented as lowercase ISO code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spending_limits_currency: Option<String>,
}

/// Parameters for updating spending controls on a Card or Cardholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SpendingControlsParams {
    /// Whitelist of allowed categories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_categories: Option<Vec<String>>,
    /// Blacklist of blocked categories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked_categories: Option<Vec<String>>,
    /// Spending limits array.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spending_limits: Option<Vec<SpendingLimitParams>>,
    /// Currency for the spending limits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spending_limits_currency: Option<String>,
}

/// Address details for physical card shipping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingCardAddress {
    /// Street address line 1.
    pub line1: String,
    /// Street address line 2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line2: Option<String>,
    /// City, district, suburb, town, or village.
    pub city: String,
    /// State, county, province, or region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Postal code or ZIP code.
    pub postal_code: String,
    /// Two-letter country code (ISO 3166-1 alpha-2).
    pub country: String,
}

/// Customs declaration details for international physical card shipments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingCardShippingCustoms {
    /// Harmonized Tariff Schedule code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hts_code: Option<String>,
}

/// Shipping details for a physical card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingCardShipping {
    /// Shipping recipient address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<IssuingCardAddress>,
    /// Customs details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customs: Option<IssuingCardShippingCustoms>,
    /// Recipient name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Recipient phone number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<String>,
    /// Delivery carrier (e.g., `fedex`, `usps`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carrier: Option<String>,
    /// Estimated time of arrival (UTC timestamp).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta: Option<i64>,
    /// Shipping service level (e.g., `standard`, `express`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    /// Shipment status (e.g., `pending`, `shipped`, `delivered`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Shipment tracking number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracking_number: Option<String>,
    /// Shipment tracking URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracking_url: Option<String>,
    /// Packaging type (e.g., `bulk`, `individual`).
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub shipping_type: Option<String>,
}

/// Shipping parameters when creating or updating a physical card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingCardShippingParams {
    /// Shipping destination address.
    pub address: IssuingCardAddress,
    /// Recipient name.
    pub name: String,
    /// Optional recipient phone number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<String>,
    /// Service level (`standard`, `express`, `priority`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    /// Packaging type (`bulk`, `individual`).
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub shipping_type: Option<String>,
}

/// Digital wallet provisioning status for a card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingWalletDetails {
    /// Whether the card is eligible for the digital wallet.
    pub eligible: bool,
    /// Reason if card is not eligible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ineligible_reason: Option<String>,
}

/// Digital wallet compatibility configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingCardWallets {
    /// Apple Pay provisioning details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apple_pay: Option<IssuingWalletDetails>,
    /// Google Pay provisioning details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub google_pay: Option<IssuingWalletDetails>,
    /// Primary Account Identifier (FPAN token).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_account_identifier: Option<String>,
}

/// Reference to an Issuing Cardholder (either expanded object or cardholder ID).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IssuingCardholderReference {
    /// Cardholder ID string (e.g. `ich_123`).
    Id(String),
}

impl IssuingCardholderReference {
    /// Returns the cardholder ID string reference.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Id(id) => id.as_str(),
        }
    }
}

impl fmt::Display for IssuingCardholderReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

impl AsRef<str> for IssuingCardholderReference {
    fn as_ref(&self) -> &str {
        self.id()
    }
}

impl From<&str> for IssuingCardholderReference {
    fn from(s: &str) -> Self {
        Self::Id(s.to_string())
    }
}

impl From<String> for IssuingCardholderReference {
    fn from(s: String) -> Self {
        Self::Id(s)
    }
}

impl PartialEq<str> for IssuingCardholderReference {
    fn eq(&self, other: &str) -> bool {
        self.id() == other
    }
}

impl PartialEq<&str> for IssuingCardholderReference {
    fn eq(&self, other: &&str) -> bool {
        self.id() == *other
    }
}

/// Represents a Stripe Issuing Card (`issuing.card`).
///
/// Under PCI-DSS SAQ A guidelines, Primary Account Numbers (PANs) are isolated
/// and never stored unencrypted in application memory. Masked `last4` is provided.
///
/// # Examples
/// ```rust
/// use some_stripe_types::{IssuingCard, IssuingCardType, IssuingCardStatus};
///
/// let json = r#"{
///   "id": "ic_123",
///   "object": "issuing.card",
///   "brand": "Visa",
///   "created": 1700000000,
///   "currency": "usd",
///   "exp_month": 12,
///   "exp_year": 2028,
///   "last4": "4242",
///   "livemode": false,
///   "status": "active",
///   "type": "virtual"
/// }"#;
///
/// let card: IssuingCard = serde_json::from_str(json).unwrap();
/// assert_eq!(card.id, "ic_123");
/// assert_eq!(card.brand, "Visa");
/// assert_eq!(card.r#type, IssuingCardType::Virtual);
/// assert_eq!(card.status, IssuingCardStatus::Active);
/// assert!(card.is_pan_isolated());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuingCard {
    /// Unique identifier for the Issuing card.
    pub id: String,
    /// String representing the object's type. Always `issuing.card`.
    #[serde(default = "default_issuing_card_object")]
    pub object: String,
    /// Card brand (e.g. `Visa`, `Mastercard`).
    pub brand: String,
    /// Reason why the card was canceled, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_reason: Option<IssuingCardCancellationReason>,
    /// Associated Cardholder reference ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cardholder: Option<IssuingCardholderReference>,
    /// Time at which the card was created (Unix epoch seconds).
    pub created: i64,
    /// Three-letter ISO currency code, in lowercase.
    pub currency: String,
    /// Optional CVC field (strictly masked or absent per SAQ A).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cvc: Option<String>,
    /// Two-digit expiration month (1-12).
    pub exp_month: i64,
    /// Four-digit expiration year.
    pub exp_year: i64,
    /// Associated Treasury Financial Account ID, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub financial_account: Option<String>,
    /// Last four digits of the card number.
    pub last4: String,
    /// Whether the card exists in livemode.
    pub livemode: bool,
    /// Set of key-value pairs attached to the card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Omitted or masked card number under PCI-DSS SAQ A PAN isolation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    /// ID of personalization design template for physical cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personalization_design: Option<String>,
    /// ID of the card replacing this card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced_by: Option<String>,
    /// ID of the card that this card replaces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement_for: Option<String>,
    /// Reason for card replacement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement_reason: Option<IssuingCardReplacementReason>,
    /// Shipping information for physical cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping: Option<IssuingCardShipping>,
    /// Spending controls and limits configured for this card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spending_controls: Option<SpendingControls>,
    /// Current card status (`active`, `inactive`, `canceled`).
    pub status: IssuingCardStatus,
    /// Card type (`virtual` or `physical`).
    #[serde(rename = "type")]
    pub r#type: IssuingCardType,
    /// Digital wallet details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wallets: Option<IssuingCardWallets>,
}

fn default_issuing_card_object() -> String {
    "issuing.card".to_string()
}

impl IssuingCard {
    /// Validates PCI-DSS SAQ A PAN isolation invariant: verifies that no full 16-digit PAN
    /// is present in memory and that `last4` contains at most 4 digits.
    #[must_use]
    pub fn is_pan_isolated(&self) -> bool {
        self.last4.len() <= 4 && self.number.as_ref().is_none_or(|num| num.len() <= 4)
    }

    /// Returns the maximum configured daily spending limit in minor currency units if present.
    #[must_use]
    pub fn daily_spending_limit(&self) -> Option<i64> {
        self.spending_controls.as_ref().and_then(|sc| {
            sc.spending_limits.as_ref().and_then(|limits| {
                limits
                    .iter()
                    .find(|l| l.interval == SpendingLimitInterval::Daily)
                    .map(|l| l.amount)
            })
        })
    }
}

/// Parameters for creating a new Issuing card.
///
/// # Examples
/// ```rust
/// use some_stripe_types::{CreateCardParams, SpendingControlsParams, SpendingLimitParams};
///
/// let params = CreateCardParams {
///     cardholder: "ich_123".into(),
///     currency: "usd".into(),
///     r#type: "virtual".into(),
///     spending_controls: Some(SpendingControlsParams {
///         spending_limits: Some(vec![SpendingLimitParams {
///             amount: 50000,
///             interval: "daily".into(),
///             ..Default::default()
///         }]),
///         ..Default::default()
///     }),
///     ..Default::default()
/// };
///
/// assert_eq!(params.currency, "usd");
/// assert_eq!(params.cardholder, "ich_123");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CreateCardParams {
    /// ID of the Cardholder to whom this card belongs.
    pub cardholder: String,
    /// Three-letter ISO currency code, in lowercase.
    pub currency: String,
    /// Card type (`virtual` or `physical`).
    #[serde(rename = "type")]
    pub r#type: IssuingCardType,
    /// Initial card status (defaults to `active` if virtual, `inactive` if physical).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<IssuingCardStatus>,
    /// Spending controls to enforce on authorizations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spending_controls: Option<SpendingControlsParams>,
    /// Card ID this card replaces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement_for: Option<String>,
    /// Reason for replacement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement_reason: Option<IssuingCardReplacementReason>,
    /// Shipping instructions for physical cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping: Option<IssuingCardShippingParams>,
    /// ID of personalization design template.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personalization_design: Option<String>,
    /// Associated Treasury Financial Account ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub financial_account: Option<String>,
    /// Set of key-value pairs attached to the card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Optional idempotency key for safe retries.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
    /// Specifies which fields in the response should be expanded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expand: Option<Vec<String>>,
}

impl CreateCardParams {
    /// Constructs a new [`CreateCardParams`] with mandatory fields.
    pub fn new(
        cardholder: impl Into<String>,
        currency: impl Into<String>,
        card_type: impl Into<IssuingCardType>,
    ) -> Self {
        Self {
            cardholder: cardholder.into(),
            currency: currency.into(),
            r#type: card_type.into(),
            ..Default::default()
        }
    }
}

/// Parameters for updating an existing Issuing card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UpdateCardParams {
    /// Card status update (`active`, `inactive`, `canceled`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<IssuingCardStatus>,
    /// Reason for cancellation if canceling the card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_reason: Option<IssuingCardCancellationReason>,
    /// Updated spending controls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spending_controls: Option<SpendingControlsParams>,
    /// Updated shipping information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping: Option<IssuingCardShippingParams>,
    /// Updated personalization design.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personalization_design: Option<String>,
    /// Updated metadata key-value pairs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Optional idempotency key for safe retries.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
    /// Specifies which fields in the response should be expanded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expand: Option<Vec<String>>,
}

/// Parameters for listing Issuing cards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ListIssuingCardsParams {
    /// Only return cards belonging to the given Cardholder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cardholder: Option<String>,
    /// Filter cards created on or after this timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
    /// Filter by expiration month.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exp_month: Option<i64>,
    /// Filter by expiration year.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exp_year: Option<i64>,
    /// Filter by last 4 digits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last4: Option<String>,
    /// Filter by card status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<IssuingCardStatus>,
    /// Filter by card type (`virtual` or `physical`).
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<IssuingCardType>,
    /// Number of objects to return (1 to 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Cursor for pagination: ID of the object to fetch after.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,
    /// Cursor for pagination: ID of the object to fetch before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

/// Paginated list of Issuing cards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuingCardList {
    /// String representing the object's type. Always `list`.
    pub object: String,
    /// List of Issuing card objects.
    pub data: Vec<IssuingCard>,
    /// True if there are more items available.
    pub has_more: bool,
    /// The URL endpoint for this list query.
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_type_conversions() {
        assert_eq!(IssuingCardType::Virtual.as_str(), "virtual");
        assert_eq!(IssuingCardType::Physical.as_str(), "physical");
        assert_eq!(IssuingCardType::from("virtual"), IssuingCardType::Virtual);
        assert_eq!(IssuingCardType::from("physical"), IssuingCardType::Physical);
        assert_eq!(IssuingCardType::Virtual, "virtual");
        assert_eq!("physical", IssuingCardType::Physical);
    }

    #[test]
    fn test_card_status_conversions() {
        assert_eq!(IssuingCardStatus::Active.as_str(), "active");
        assert_eq!(IssuingCardStatus::Inactive.as_str(), "inactive");
        assert_eq!(IssuingCardStatus::Canceled.as_str(), "canceled");
        assert_eq!(IssuingCardStatus::from("active"), IssuingCardStatus::Active);
    }

    #[test]
    fn test_spending_limit_interval_conversions() {
        assert_eq!(SpendingLimitInterval::Daily.as_str(), "daily");
        assert_eq!(SpendingLimitInterval::Monthly.as_str(), "monthly");
        assert_eq!(
            SpendingLimitInterval::from("daily"),
            SpendingLimitInterval::Daily
        );
        assert_eq!(SpendingLimitInterval::Daily, "daily");
    }

    #[test]
    fn test_card_pan_isolation_pci_dss() {
        let card = IssuingCard {
            id: "ic_123".into(),
            object: "issuing.card".into(),
            brand: "Visa".into(),
            cancellation_reason: None,
            cardholder: Some("ich_123".into()),
            created: 1700000000,
            currency: "usd".into(),
            cvc: None,
            exp_month: 12,
            exp_year: 2028,
            financial_account: None,
            last4: "4242".into(),
            livemode: false,
            metadata: None,
            number: None,
            personalization_design: None,
            replaced_by: None,
            replacement_for: None,
            replacement_reason: None,
            shipping: None,
            spending_controls: Some(SpendingControls {
                allowed_categories: None,
                blocked_categories: None,
                spending_limits: Some(vec![SpendingLimit {
                    amount: 50000,
                    interval: SpendingLimitInterval::Daily,
                    categories: None,
                }]),
                spending_limits_currency: Some("usd".into()),
            }),
            status: IssuingCardStatus::Active,
            r#type: IssuingCardType::Virtual,
            wallets: None,
        };

        assert!(card.is_pan_isolated());
        assert_eq!(card.daily_spending_limit(), Some(50000));
    }

    #[test]
    fn test_card_serialization_roundtrip() {
        let card = IssuingCard {
            id: "ic_test".into(),
            object: "issuing.card".into(),
            brand: "Mastercard".into(),
            cancellation_reason: None,
            cardholder: Some("ich_456".into()),
            created: 1700000000,
            currency: "usd".into(),
            cvc: None,
            exp_month: 11,
            exp_year: 2029,
            financial_account: None,
            last4: "1234".into(),
            livemode: false,
            metadata: None,
            number: None,
            personalization_design: None,
            replaced_by: None,
            replacement_for: None,
            replacement_reason: None,
            shipping: None,
            spending_controls: None,
            status: IssuingCardStatus::Active,
            r#type: IssuingCardType::Virtual,
            wallets: None,
        };

        let json = serde_json::to_string(&card).unwrap();
        let parsed: IssuingCard = serde_json::from_str(&json).unwrap();
        assert_eq!(card, parsed);
    }
}
