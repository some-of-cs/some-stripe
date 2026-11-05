//! # Stripe PaymentLink Types, Request Parameters & State Machine
//!
//! Strongly typed domain models, request parameters, status state machine,
//! and multi-currency pricing schemas for Stripe Payment Links.
//!
//! A Payment Link represents a hosted payment page that a customer can use to purchase items,
//! sell products or subscriptions, and accept donations.
//!
//! Under EU Omnibus Directive 2019/2161 and ISO 4217 integer arithmetic (Gate G-Law),
//! payment links manage shareable checkout URLs with transparent pricing and instant state transitions.

use crate::validate_currency;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// Default object type name for a Stripe Payment Link (`"payment_link"`).
#[must_use]
pub fn default_payment_link_object() -> String {
    "payment_link".into()
}

/// Strongly typed Stripe Payment Link ID (`plink_...`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaymentLinkId(pub String);

impl std::fmt::Display for PaymentLinkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Into<String>> From<T> for PaymentLinkId {
    fn from(s: T) -> Self {
        Self(s.into())
    }
}

impl AsRef<str> for PaymentLinkId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Status of a [`PaymentLink`].
///
/// Corresponds to the lifecycle state of a Stripe Payment Link,
/// representing whether the shareable URL is currently accepting payments.
///
/// In the Stripe API wire format, status is represented by the boolean `active` field.
/// This enum provides a strongly typed state machine for transitions between `Active` and `Inactive`.
///
/// # Example
/// ```
/// use some_stripe_types::PaymentLinkStatus;
/// use std::str::FromStr;
///
/// let status = PaymentLinkStatus::Active;
/// assert!(status.is_active());
/// assert_eq!(status.as_str(), "active");
///
/// let toggled = status.toggle();
/// assert!(!toggled.is_active());
/// assert_eq!(toggled, PaymentLinkStatus::Inactive);
///
/// let parsed = PaymentLinkStatus::from_str("active").unwrap();
/// assert_eq!(parsed, PaymentLinkStatus::Active);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentLinkStatus {
    /// The payment link is active and open to customer checkouts (`active = true`).
    Active,
    /// The payment link is deactivated / inactive and rejects customer checkouts (`active = false`).
    Inactive,
}

impl PaymentLinkStatus {
    /// Returns the static string representation of the status (`"active"` or `"inactive"`).
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
        }
    }

    /// Returns `true` if the status is [`PaymentLinkStatus::Active`].
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns `true` if the status is [`PaymentLinkStatus::Inactive`].
    #[must_use]
    pub const fn is_inactive(&self) -> bool {
        matches!(self, Self::Inactive)
    }

    /// Validates whether a state transition from `self` to `target` is allowed.
    ///
    /// Payment Links support bidirectional transitions between `Active` and `Inactive` at any time.
    #[must_use]
    pub const fn can_transition_to(&self, _target: Self) -> bool {
        true
    }

    /// Transitions to the specified `target` status, returning the new status.
    #[must_use]
    pub const fn transition_to(&self, target: Self) -> Self {
        target
    }

    /// Toggles the status between `Active` and `Inactive`.
    #[must_use]
    pub const fn toggle(self) -> Self {
        match self {
            Self::Active => Self::Inactive,
            Self::Inactive => Self::Active,
        }
    }

    /// Transitions to `Active`.
    #[must_use]
    pub const fn activate(self) -> Self {
        Self::Active
    }

    /// Transitions to `Inactive`.
    #[must_use]
    pub const fn deactivate(self) -> Self {
        Self::Inactive
    }

    /// Parses a string into [`PaymentLinkStatus`].
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, ParsePaymentLinkStatusError> {
        <Self as std::str::FromStr>::from_str(s)
    }
}

impl std::fmt::Display for PaymentLinkStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for PaymentLinkStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<bool> for PaymentLinkStatus {
    fn from(active: bool) -> Self {
        if active { Self::Active } else { Self::Inactive }
    }
}

impl From<PaymentLinkStatus> for bool {
    fn from(status: PaymentLinkStatus) -> Self {
        status.is_active()
    }
}

impl FromStr for PaymentLinkStatus {
    type Err = ParsePaymentLinkStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" | "true" => Ok(Self::Active),
            "inactive" | "false" => Ok(Self::Inactive),
            _ => Err(ParsePaymentLinkStatusError(s.to_string())),
        }
    }
}

/// Error returned when parsing an unrecognized payment link status string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePaymentLinkStatusError(pub String);

impl std::fmt::Display for ParsePaymentLinkStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown Stripe PaymentLink status: '{}'", self.0)
    }
}

impl std::error::Error for ParsePaymentLinkStatusError {}

impl PartialEq<&str> for PaymentLinkStatus {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<PaymentLinkStatus> for &str {
    fn eq(&self, other: &PaymentLinkStatus) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<str> for PaymentLinkStatus {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<PaymentLinkStatus> for str {
    fn eq(&self, other: &PaymentLinkStatus) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<String> for PaymentLinkStatus {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<PaymentLinkStatus> for String {
    fn eq(&self, other: &PaymentLinkStatus) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<bool> for PaymentLinkStatus {
    fn eq(&self, other: &bool) -> bool {
        self.is_active() == *other
    }
}

impl PartialEq<PaymentLinkStatus> for bool {
    fn eq(&self, other: &PaymentLinkStatus) -> bool {
        *self == other.is_active()
    }
}

impl From<PaymentLinkStatus> for &'static str {
    fn from(status: PaymentLinkStatus) -> Self {
        status.as_str()
    }
}

impl From<PaymentLinkStatus> for String {
    fn from(status: PaymentLinkStatus) -> Self {
        status.as_str().to_string()
    }
}

/// Represents a Stripe PaymentLink domain entity.
///
/// A Payment Link is a shareable URL that lets customers purchase products or services
/// without requiring a custom checkout integration.
///
/// # Example
/// ```
/// use some_stripe_types::{PaymentLink, PaymentLinkStatus};
///
/// let json = r#"{
///     "id": "plink_1N42iV2eZvKYlo2CUbdf5V4L",
///     "object": "payment_link",
///     "active": true,
///     "url": "https://buy.stripe.com/test_1N42iV2eZvKYlo2CUbdf5V4L"
/// }"#;
/// let link: PaymentLink = serde_json::from_str(json).unwrap();
/// assert_eq!(link.id, "plink_1N42iV2eZvKYlo2CUbdf5V4L");
/// assert!(link.is_active());
/// assert_eq!(link.status(), PaymentLinkStatus::Active);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentLink {
    /// Unique identifier for the payment link (`plink_...`).
    pub id: String,

    /// Object type, always `payment_link`.
    #[serde(default = "default_payment_link_object")]
    pub object: String,

    /// Whether the payment link's URL is active and accepting payments.
    pub active: bool,

    /// The public URL that can be shared with customers (e.g. `https://buy.stripe.com/...`).
    pub url: String,

    /// Three-letter ISO currency code in lowercase, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// The list of payment method types that customers can use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_method_types: Option<Vec<String>>,

    /// Whether promotion codes are allowed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_promotion_codes: Option<bool>,

    /// Configuration for collecting the customer's billing address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing_address_collection: Option<String>,

    /// Configuration for collecting shipping address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping_address_collection: Option<serde_json::Value>,

    /// Configuration for customer creation during checkout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer_creation: Option<String>,

    /// Configuration for post-payment redirect or confirmation behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_completion: Option<PaymentLinkAfterCompletionParams>,

    /// Has the value `true` if the object exists in live mode or `false` if in test mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub livemode: Option<bool>,
}

impl Default for PaymentLink {
    fn default() -> Self {
        Self {
            id: String::new(),
            object: default_payment_link_object(),
            active: true,
            url: String::new(),
            currency: None,
            metadata: None,
            payment_method_types: None,
            allow_promotion_codes: None,
            billing_address_collection: None,
            shipping_address_collection: None,
            customer_creation: None,
            after_completion: None,
            livemode: None,
        }
    }
}

impl PaymentLink {
    /// Constructs a minimal valid [`PaymentLink`].
    pub fn new(id: impl Into<String>, url: impl Into<String>, active: bool) -> Self {
        Self {
            id: id.into(),
            object: default_payment_link_object(),
            active,
            url: url.into(),
            currency: None,
            metadata: None,
            payment_method_types: None,
            allow_promotion_codes: None,
            billing_address_collection: None,
            shipping_address_collection: None,
            customer_creation: None,
            after_completion: None,
            livemode: None,
        }
    }

    /// Returns `true` if the payment link is currently active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.active
    }

    /// Returns the strongly typed [`PaymentLinkStatus`] of this payment link.
    #[must_use]
    pub const fn status(&self) -> PaymentLinkStatus {
        if self.active {
            PaymentLinkStatus::Active
        } else {
            PaymentLinkStatus::Inactive
        }
    }

    /// Sets the payment link to active (`active = true`).
    ///
    /// Returns the new status ([`PaymentLinkStatus::Active`]).
    pub fn activate(&mut self) -> PaymentLinkStatus {
        self.active = true;
        PaymentLinkStatus::Active
    }

    /// Sets the payment link to inactive / deactivated (`active = false`).
    ///
    /// Returns the new status ([`PaymentLinkStatus::Inactive`]).
    pub fn deactivate(&mut self) -> PaymentLinkStatus {
        self.active = false;
        PaymentLinkStatus::Inactive
    }

    /// Toggles the active status of the payment link.
    ///
    /// Returns the new status after the toggle.
    pub fn toggle_active(&mut self) -> PaymentLinkStatus {
        self.active = !self.active;
        self.status()
    }

    /// Transitions the payment link to the specified target status.
    ///
    /// Returns the new status.
    pub fn transition_to(&mut self, target: PaymentLinkStatus) -> PaymentLinkStatus {
        self.active = target.is_active();
        target
    }

    /// Fluent builder method to set the `currency`.
    #[must_use]
    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = Some(currency.into());
        self
    }

    /// Fluent builder method to set the `metadata`.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Fluent builder method to set the `payment_method_types`.
    #[must_use]
    pub fn with_payment_method_types(mut self, types: Vec<String>) -> Self {
        self.payment_method_types = Some(types);
        self
    }

    /// Fluent builder method to set `allow_promotion_codes`.
    #[must_use]
    pub fn with_allow_promotion_codes(mut self, allow: bool) -> Self {
        self.allow_promotion_codes = Some(allow);
        self
    }

    /// Fluent builder method to set `billing_address_collection`.
    #[must_use]
    pub fn with_billing_address_collection(mut self, collection: impl Into<String>) -> Self {
        self.billing_address_collection = Some(collection.into());
        self
    }

    /// Fluent builder method to set `shipping_address_collection`.
    #[must_use]
    pub fn with_shipping_address_collection(mut self, collection: serde_json::Value) -> Self {
        self.shipping_address_collection = Some(collection);
        self
    }

    /// Fluent builder method to set `customer_creation`.
    #[must_use]
    pub fn with_customer_creation(mut self, customer_creation: impl Into<String>) -> Self {
        self.customer_creation = Some(customer_creation.into());
        self
    }

    /// Fluent builder method to set `after_completion`.
    #[must_use]
    pub fn with_after_completion(
        mut self,
        after_completion: PaymentLinkAfterCompletionParams,
    ) -> Self {
        self.after_completion = Some(after_completion);
        self
    }

    /// Fluent builder method to set `livemode`.
    #[must_use]
    pub fn with_livemode(mut self, livemode: bool) -> Self {
        self.livemode = Some(livemode);
        self
    }

    /// Returns the payment link ID as a borrowed string slice (`&str`).
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the public checkout URL as a borrowed string slice (`&str`).
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the currency as an optional borrowed string slice (`Option<&str>`).
    #[must_use]
    pub fn currency(&self) -> Option<&str> {
        self.currency.as_deref()
    }

    /// Queries the metadata map and returns a borrowed string slice without map cloning.
    #[must_use]
    pub fn metadata_get(&self, key: &str) -> Option<&str> {
        self.metadata
            .as_ref()
            .and_then(|m| m.get(key))
            .map(|s| s.as_str())
    }
}

// ============================================================================
// Request Parameter Schemas (US-4.2.0 Unit 2)
// ============================================================================

/// Parameters for creating a Stripe Payment Link (`POST /v1/payment_links`).
///
/// Supports multi-line items, customer adjustable quantities, post-completion redirects,
/// custom hosted confirmation messages, multi-currency pricing, and metadata.
///
/// # Example
/// ```
/// use some_stripe_types::{
///     CreatePaymentLinkParams, PaymentLinkLineItemParams,
///     PaymentLinkAfterCompletionParams,
/// };
///
/// let params = CreatePaymentLinkParams::new(vec![
///     PaymentLinkLineItemParams::new("price_1N42iV2eZvKYlo2CUbdf5V4L", 1),
/// ])
/// .active(true)
/// .after_completion(PaymentLinkAfterCompletionParams::redirect("https://example.com/success"))
/// .allow_promotion_codes(true);
///
/// assert!(params.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePaymentLinkParams {
    /// The line items representing products or prices to be purchased.
    pub line_items: Vec<PaymentLinkLineItemParams>,

    /// Whether the payment link is active upon creation. Default: `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,

    /// Behavior after the payment link customer completes the purchase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_completion: Option<PaymentLinkAfterCompletionParams>,

    /// Whether promotion codes are allowed on this payment link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_promotion_codes: Option<bool>,

    /// Configuration for collecting the customer's billing address (`auto` or `required`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing_address_collection: Option<String>,

    /// Configuration for customer creation (`always` or `if_required`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer_creation: Option<String>,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// The list of payment method types (e.g. `card`, `ideal`) that customers can use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_method_types: Option<Vec<String>>,

    /// Multi-currency pricing options and text presentation overrides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency_options: Option<HashMap<String, PaymentLinkCurrencyOptionsParams>>,

    /// Optional idempotency key for safe retries (omitted from serialization body).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreatePaymentLinkParams {
    /// Constructs a new [`CreatePaymentLinkParams`] with the required line items.
    pub fn new(line_items: Vec<PaymentLinkLineItemParams>) -> Self {
        Self {
            line_items,
            active: None,
            after_completion: None,
            allow_promotion_codes: None,
            billing_address_collection: None,
            customer_creation: None,
            metadata: None,
            payment_method_types: None,
            currency_options: None,
            idempotency_key: None,
        }
    }

    /// Fluent builder method to set whether the payment link is active.
    #[must_use]
    pub fn active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }

    /// Fluent builder method to set the post-completion action.
    #[must_use]
    pub fn after_completion(mut self, after_completion: PaymentLinkAfterCompletionParams) -> Self {
        self.after_completion = Some(after_completion);
        self
    }

    /// Fluent builder method to set `allow_promotion_codes`.
    #[must_use]
    pub fn allow_promotion_codes(mut self, allow: bool) -> Self {
        self.allow_promotion_codes = Some(allow);
        self
    }

    /// Fluent builder method to set `billing_address_collection`.
    #[must_use]
    pub fn billing_address_collection(mut self, collection: impl Into<String>) -> Self {
        self.billing_address_collection = Some(collection.into());
        self
    }

    /// Fluent builder method to set `customer_creation`.
    #[must_use]
    pub fn customer_creation(mut self, creation: impl Into<String>) -> Self {
        self.customer_creation = Some(creation.into());
        self
    }

    /// Fluent builder method to set `metadata`.
    #[must_use]
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Fluent builder method to append a single key-value metadata entry.
    #[must_use]
    pub fn metadata_entry(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let map = self.metadata.get_or_insert_with(HashMap::new);
        map.insert(key.into(), value.into());
        self
    }

    /// Fluent builder method to set `payment_method_types`.
    #[must_use]
    pub fn payment_method_types(mut self, types: Vec<String>) -> Self {
        self.payment_method_types = Some(types);
        self
    }

    /// Fluent builder method to set `currency_options`.
    #[must_use]
    pub fn currency_options(
        mut self,
        options: HashMap<String, PaymentLinkCurrencyOptionsParams>,
    ) -> Self {
        self.currency_options = Some(options);
        self
    }

    /// Fluent builder method to set the `idempotency_key`.
    #[must_use]
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates parameters against Stripe API constraints and Gate G-Law invariants.
    pub fn validate(&self) -> Result<(), PaymentLinkValidationError> {
        if self.line_items.is_empty() {
            return Err(PaymentLinkValidationError::EmptyLineItems);
        }
        for item in &self.line_items {
            item.validate()?;
        }
        if let Some(ref after) = self.after_completion {
            after.validate()?;
        }
        if let Some(ref opts) = self.currency_options {
            for currency in opts.keys() {
                validate_currency(currency).map_err(|_| {
                    PaymentLinkValidationError::InvalidCurrency {
                        currency: currency.clone(),
                    }
                })?;
            }
        }
        Ok(())
    }
}

/// Parameters for updating an existing Stripe Payment Link (`POST /v1/payment_links/:id`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UpdatePaymentLinkParams {
    /// Whether the payment link's URL is active. Set to `false` to deactivate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,

    /// Configuration for post-payment redirect or confirmation behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_completion: Option<PaymentLinkAfterCompletionParams>,

    /// Whether promotion codes are allowed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_promotion_codes: Option<bool>,

    /// Configuration for collecting the customer's billing address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing_address_collection: Option<String>,

    /// Configuration for customer creation during checkout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer_creation: Option<String>,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// The list of payment method types that customers can use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_method_types: Option<Vec<String>>,

    /// Configuration for multi-currency presentation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency_options: Option<HashMap<String, PaymentLinkCurrencyOptionsParams>>,

    /// Idempotency key for safe retries (omitted from serialization body).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl UpdatePaymentLinkParams {
    /// Constructs empty [`UpdatePaymentLinkParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Fluent builder method to set `active`.
    #[must_use]
    pub fn active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }

    /// Fluent builder method to set `after_completion`.
    #[must_use]
    pub fn after_completion(mut self, after_completion: PaymentLinkAfterCompletionParams) -> Self {
        self.after_completion = Some(after_completion);
        self
    }

    /// Fluent builder method to set `allow_promotion_codes`.
    #[must_use]
    pub fn allow_promotion_codes(mut self, allow: bool) -> Self {
        self.allow_promotion_codes = Some(allow);
        self
    }

    /// Fluent builder method to set `billing_address_collection`.
    #[must_use]
    pub fn billing_address_collection(mut self, collection: impl Into<String>) -> Self {
        self.billing_address_collection = Some(collection.into());
        self
    }

    /// Fluent builder method to set `customer_creation`.
    #[must_use]
    pub fn customer_creation(mut self, creation: impl Into<String>) -> Self {
        self.customer_creation = Some(creation.into());
        self
    }

    /// Fluent builder method to set `metadata`.
    #[must_use]
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Fluent builder method to append a metadata key-value pair.
    #[must_use]
    pub fn metadata_entry(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let map = self.metadata.get_or_insert_with(HashMap::new);
        map.insert(key.into(), value.into());
        self
    }

    /// Fluent builder method to set `payment_method_types`.
    #[must_use]
    pub fn payment_method_types(mut self, types: Vec<String>) -> Self {
        self.payment_method_types = Some(types);
        self
    }

    /// Fluent builder method to set `currency_options`.
    #[must_use]
    pub fn currency_options(
        mut self,
        options: HashMap<String, PaymentLinkCurrencyOptionsParams>,
    ) -> Self {
        self.currency_options = Some(options);
        self
    }

    /// Fluent builder method to set the `idempotency_key`.
    #[must_use]
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates the parameters.
    pub fn validate(&self) -> Result<(), PaymentLinkValidationError> {
        if let Some(ref after) = self.after_completion {
            after.validate()?;
        }
        if let Some(ref opts) = self.currency_options {
            for currency in opts.keys() {
                validate_currency(currency).map_err(|_| {
                    PaymentLinkValidationError::InvalidCurrency {
                        currency: currency.clone(),
                    }
                })?;
            }
        }
        Ok(())
    }
}

/// Line item parameter for creating or presenting a Payment Link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentLinkLineItemParams {
    /// The ID of the Price object or multi-currency price (`price_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,

    /// The quantity of units to purchase. Default: 1.
    pub quantity: u32,

    /// Configuration for customer-adjustable quantities on the checkout page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjustable_quantity: Option<AdjustableQuantityParams>,

    /// Optional inline price data with multi-currency pricing options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_data: Option<PaymentLinkPriceDataParams>,
}

impl PaymentLinkLineItemParams {
    /// Constructs a [`PaymentLinkLineItemParams`] with a price ID and quantity.
    pub fn new(price: impl Into<String>, quantity: u32) -> Self {
        Self {
            price: Some(price.into()),
            quantity,
            adjustable_quantity: None,
            price_data: None,
        }
    }

    /// Constructs a [`PaymentLinkLineItemParams`] with inline price data.
    pub fn with_price_data(price_data: PaymentLinkPriceDataParams, quantity: u32) -> Self {
        Self {
            price: None,
            quantity,
            adjustable_quantity: None,
            price_data: Some(price_data),
        }
    }

    /// Fluent builder method to attach adjustable quantity parameters.
    #[must_use]
    pub fn adjustable_quantity(mut self, adjustable: AdjustableQuantityParams) -> Self {
        self.adjustable_quantity = Some(adjustable);
        self
    }

    /// Validates the line item parameter.
    pub fn validate(&self) -> Result<(), PaymentLinkValidationError> {
        if self.quantity == 0 {
            return Err(PaymentLinkValidationError::InvalidQuantity { quantity: 0 });
        }
        if self.price.is_none() && self.price_data.is_none() {
            return Err(PaymentLinkValidationError::MissingPriceOrPriceData);
        }
        if let Some(ref adj) = self.adjustable_quantity {
            adj.validate()?;
        }
        if let Some(ref pd) = self.price_data {
            pd.validate()?;
        }
        Ok(())
    }
}

/// Configuration for customer-adjustable quantities on Stripe Payment Links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AdjustableQuantityParams {
    /// Whether the quantity can be adjusted by the customer on the checkout page.
    pub enabled: bool,

    /// The minimum quantity the customer can purchase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<u32>,

    /// The maximum quantity the customer can purchase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<u32>,
}

impl AdjustableQuantityParams {
    /// Constructs an [`AdjustableQuantityParams`] with the enabled flag.
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            minimum: None,
            maximum: None,
        }
    }

    /// Constructs an [`AdjustableQuantityParams`] with specific minimum and maximum limits.
    pub fn with_limits(minimum: u32, maximum: u32) -> Self {
        Self {
            enabled: true,
            minimum: Some(minimum),
            maximum: Some(maximum),
        }
    }

    /// Validates the adjustable quantity parameters.
    pub fn validate(&self) -> Result<(), PaymentLinkValidationError> {
        match (self.minimum, self.maximum) {
            (Some(min), Some(max)) if min > max => Err(
                PaymentLinkValidationError::InvalidAdjustableQuantityLimits {
                    minimum: min,
                    maximum: max,
                },
            ),
            _ => Ok(()),
        }
    }
}

/// Post-completion behavior configuration for a Payment Link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentLinkAfterCompletionParams {
    /// The type of post-completion action (`redirect` or `hosted_confirmation`).
    #[serde(rename = "type")]
    pub r#type: String,

    /// Configuration for redirecting the customer to your website after payment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redirect: Option<PaymentLinkRedirectParams>,

    /// Configuration for displaying a hosted confirmation page after payment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosted_confirmation: Option<HostedConfirmationParams>,
}

impl PaymentLinkAfterCompletionParams {
    /// Constructs a redirect post-completion action with the given URL.
    pub fn redirect(url: impl Into<String>) -> Self {
        Self {
            r#type: "redirect".into(),
            redirect: Some(PaymentLinkRedirectParams { url: url.into() }),
            hosted_confirmation: None,
        }
    }

    /// Constructs a hosted confirmation post-completion action with an optional custom message.
    pub fn hosted_confirmation(custom_message: Option<String>) -> Self {
        Self {
            r#type: "hosted_confirmation".into(),
            redirect: None,
            hosted_confirmation: Some(HostedConfirmationParams { custom_message }),
        }
    }

    /// Validates after_completion parameters.
    pub fn validate(&self) -> Result<(), PaymentLinkValidationError> {
        match self.r#type.as_str() {
            "redirect" => {
                let Some(ref r) = self.redirect else {
                    return Err(PaymentLinkValidationError::InvalidAfterCompletion {
                        reason: "Redirect parameters required when type is 'redirect'".into(),
                    });
                };
                if r.url.trim().is_empty() {
                    return Err(PaymentLinkValidationError::InvalidAfterCompletion {
                        reason: "Redirect URL must not be empty".into(),
                    });
                }
            }
            "hosted_confirmation" => {
                // hosted_confirmation is valid even if hosted_confirmation is None
            }
            other => {
                return Err(PaymentLinkValidationError::InvalidAfterCompletion {
                    reason: format!(
                        "Unknown after_completion type '{other}', expected 'redirect' or 'hosted_confirmation'"
                    ),
                });
            }
        }
        Ok(())
    }
}

/// Redirect parameters for post-completion flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentLinkRedirectParams {
    /// The URL the customer will be directed to after the purchase is complete.
    pub url: String,
}

impl PaymentLinkRedirectParams {
    /// Constructs a new [`PaymentLinkRedirectParams`] with the target URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

/// Hosted confirmation page parameters for post-completion flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HostedConfirmationParams {
    /// A custom message to display to the customer after the purchase is complete.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_message: Option<String>,
}

impl HostedConfirmationParams {
    /// Constructs a new [`HostedConfirmationParams`].
    pub fn new(custom_message: Option<String>) -> Self {
        Self { custom_message }
    }
}

// ============================================================================
// Multi-Currency Pricing Schemas (Gate G-Law & ISO 4217)
// ============================================================================

/// Inline price data for creating prices or line items with multi-currency presentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentLinkPriceDataParams {
    /// Three-letter ISO currency code, in lowercase (must be a valid ISO 4217 code).
    pub currency: String,

    /// The unit amount in minor currency units (e.g., 1000 for $10.00 USD, 900 for €9.00 EUR).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_amount: Option<i64>,

    /// Product ID that this price is associated with (`prod_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,

    /// Multi-currency pricing options (Gate G-Law & ISO 4217).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency_options: Option<HashMap<String, CurrencyOptionParams>>,

    /// Tax behavior (`inclusive`, `exclusive`, or `unspecified`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_behavior: Option<String>,
}

impl PaymentLinkPriceDataParams {
    /// Constructs price data with base currency and integer unit amount.
    pub fn new(currency: impl Into<String>, unit_amount: i64) -> Self {
        Self {
            currency: currency.into(),
            unit_amount: Some(unit_amount),
            product: None,
            currency_options: None,
            tax_behavior: None,
        }
    }

    /// Fluent builder method to attach product ID.
    #[must_use]
    pub fn product(mut self, product: impl Into<String>) -> Self {
        self.product = Some(product.into());
        self
    }

    /// Fluent builder method to attach multi-currency options.
    #[must_use]
    pub fn currency_options(mut self, options: HashMap<String, CurrencyOptionParams>) -> Self {
        self.currency_options = Some(options);
        self
    }

    /// Fluent builder method to append a single currency option.
    #[must_use]
    pub fn currency_option(
        mut self,
        currency: impl Into<String>,
        option: CurrencyOptionParams,
    ) -> Self {
        let map = self.currency_options.get_or_insert_with(HashMap::new);
        map.insert(currency.into(), option);
        self
    }

    /// Fluent builder method to attach tax behavior (`inclusive`, `exclusive`, or `unspecified`).
    #[must_use]
    pub fn tax_behavior(mut self, tax_behavior: impl Into<String>) -> Self {
        self.tax_behavior = Some(tax_behavior.into());
        self
    }

    /// Validates the price data and multi-currency options.
    pub fn validate(&self) -> Result<(), PaymentLinkValidationError> {
        validate_currency(&self.currency).map_err(|_| {
            PaymentLinkValidationError::InvalidCurrency {
                currency: self.currency.clone(),
            }
        })?;
        if let Some(amt) = self.unit_amount.filter(|&a| a <= 0) {
            return Err(PaymentLinkValidationError::InvalidAmount { amount: amt });
        }
        if let Some(ref opts) = self.currency_options {
            for (curr, opt) in opts {
                validate_currency(curr).map_err(|_| {
                    PaymentLinkValidationError::InvalidCurrency {
                        currency: curr.clone(),
                    }
                })?;
                if let Some(amt) = opt.unit_amount.filter(|&a| a <= 0) {
                    return Err(PaymentLinkValidationError::InvalidAmount { amount: amt });
                }
                if let Some(ref custom) = opt.custom_unit_amount {
                    match (custom.minimum, custom.maximum) {
                        (Some(min), Some(max)) if min > max => {
                            return Err(PaymentLinkValidationError::InvalidParameter {
                                field: "custom_unit_amount",
                                reason: format!(
                                    "Minimum amount {min} cannot exceed maximum amount {max}"
                                ),
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }
}

/// Multi-currency pricing options for a specific currency (ISO 4217).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CurrencyOptionParams {
    /// The unit amount in minor currency units (e.g., cents, pence).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_amount: Option<i64>,

    /// Tax behavior (`inclusive`, `exclusive`, or `unspecified`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_behavior: Option<String>,

    /// Configuration for customer-adjustable amounts in this currency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_unit_amount: Option<CustomUnitAmountParams>,
}

impl CurrencyOptionParams {
    /// Constructs [`CurrencyOptionParams`] with the unit amount.
    pub fn with_amount(unit_amount: i64) -> Self {
        Self {
            unit_amount: Some(unit_amount),
            tax_behavior: None,
            custom_unit_amount: None,
        }
    }
}

/// Configuration for customer-adjustable / donation amounts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CustomUnitAmountParams {
    /// Whether custom unit amounts are enabled for this currency.
    pub enabled: bool,

    /// The minimum unit amount in minor units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i64>,

    /// The maximum unit amount in minor units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i64>,

    /// The preset / default unit amount in minor units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<i64>,
}

impl CustomUnitAmountParams {
    /// Constructs [`CustomUnitAmountParams`] with enabled flag.
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            minimum: None,
            maximum: None,
            preset: None,
        }
    }
}

/// Presentation parameters for a specific currency on a Payment Link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PaymentLinkCurrencyOptionsParams {
    /// Custom text position parameters for this currency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_text: Option<PaymentLinkCustomTextParams>,
}

impl PaymentLinkCurrencyOptionsParams {
    /// Constructs [`PaymentLinkCurrencyOptionsParams`].
    pub fn new() -> Self {
        Self::default()
    }
}

/// Custom text position parameters for a Payment Link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PaymentLinkCustomTextParams {
    /// Custom message for the shipping address collection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping_address: Option<PaymentLinkCustomTextMessageParams>,

    /// Custom message for the submit button.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submit: Option<PaymentLinkCustomTextMessageParams>,
}

/// Custom text message parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PaymentLinkCustomTextMessageParams {
    /// The custom message string.
    pub message: String,
}

impl PaymentLinkCustomTextMessageParams {
    /// Constructs a [`PaymentLinkCustomTextMessageParams`] with the message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

// ============================================================================
// Validation Error Types (Gate G-Law & ISO 4217)
// ============================================================================

/// Validation error for Payment Link request parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentLinkValidationError {
    /// Line items list is empty; at least one line item is required.
    EmptyLineItems,
    /// Invalid line item quantity (must be >= 1).
    InvalidQuantity {
        /// The invalid quantity supplied.
        quantity: u32,
    },
    /// Neither price ID nor inline price data was supplied.
    MissingPriceOrPriceData,
    /// Adjustable quantity minimum exceeds maximum limit.
    InvalidAdjustableQuantityLimits {
        /// Minimum limit.
        minimum: u32,
        /// Maximum limit.
        maximum: u32,
    },
    /// Invalid post-completion action configuration.
    InvalidAfterCompletion {
        /// Detailed description of the error.
        reason: String,
    },
    /// The specified currency is invalid (must be 3 ASCII alphabetic characters).
    InvalidCurrency {
        /// The invalid currency string.
        currency: String,
    },
    /// Monetary amount is non-positive; must be integer minor currency units (> 0).
    InvalidAmount {
        /// The invalid amount.
        amount: i64,
    },
    /// A general parameter validation failure.
    InvalidParameter {
        /// Field name.
        field: &'static str,
        /// Description of the error.
        reason: String,
    },
}

impl std::fmt::Display for PaymentLinkValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyLineItems => write!(f, "PaymentLink line_items cannot be empty"),
            Self::InvalidQuantity { quantity } => {
                write!(
                    f,
                    "PaymentLink line item quantity must be >= 1, got: {quantity}"
                )
            }
            Self::MissingPriceOrPriceData => {
                write!(
                    f,
                    "PaymentLink line item requires either 'price' or 'price_data'"
                )
            }
            Self::InvalidAdjustableQuantityLimits { minimum, maximum } => {
                write!(
                    f,
                    "PaymentLink adjustable_quantity minimum ({minimum}) cannot exceed maximum ({maximum})"
                )
            }
            Self::InvalidAfterCompletion { reason } => {
                write!(
                    f,
                    "Invalid PaymentLink after_completion configuration: {reason}"
                )
            }
            Self::InvalidCurrency { currency } => {
                write!(
                    f,
                    "PaymentLink currency must be a valid 3-letter ISO 4217 code (got: '{currency}')"
                )
            }
            Self::InvalidAmount { amount } => {
                write!(
                    f,
                    "PaymentLink amount must be a positive integer minor currency unit (> 0), got: {amount}"
                )
            }
            Self::InvalidParameter { field, reason } => {
                write!(f, "Invalid PaymentLink parameter '{field}': {reason}")
            }
        }
    }
}

impl std::error::Error for PaymentLinkValidationError {}

// ============================================================================
// LineItem Domain Entity & Paginator Extractor (US-4.2.0 Unit 3)
// ============================================================================

/// Returns the default object type string for LineItem (`"item"`).
#[must_use]
pub fn default_line_item_object() -> String {
    "item".into()
}

/// Returns the default object type string for LineItemPrice (`"price"`).
#[must_use]
pub fn default_price_object() -> String {
    "price".into()
}

/// Price object associated with a [`LineItem`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineItemPrice {
    /// Unique identifier for the price (`price_...`).
    pub id: String,

    /// Object type, always `price`.
    #[serde(default = "default_price_object")]
    pub object: String,

    /// Whether the price is currently active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,

    /// Three-letter ISO currency code in lowercase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    /// The unit amount in minor currency units (e.g., cents).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_amount: Option<i64>,

    /// The ID of the product this price belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
}

impl LineItemPrice {
    /// Creates a minimal [`LineItemPrice`].
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            object: default_price_object(),
            active: None,
            currency: None,
            unit_amount: None,
            product: None,
        }
    }

    /// Fluent builder method to attach `unit_amount` and `currency`.
    #[must_use]
    pub fn with_amount(mut self, unit_amount: i64, currency: impl Into<String>) -> Self {
        self.unit_amount = Some(unit_amount);
        self.currency = Some(currency.into());
        self
    }
}

/// Represents a Stripe LineItem object (`item`).
///
/// A LineItem represents an item being purchased via a Payment Link or Checkout Session.
///
/// # Example
/// ```
/// use some_stripe_types::{LineItem, line_item_id};
///
/// let item = LineItem::new("li_test_123", 2500, 2500, "usd")
///     .with_description("E-book Download")
///     .with_quantity(1);
/// assert_eq!(item.id, "li_test_123");
/// assert_eq!(line_item_id(&item), "li_test_123");
/// assert_eq!(item.amount_total, 2500);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineItem {
    /// Unique identifier for the line item (`li_...`).
    pub id: String,

    /// Object type, always `item`.
    #[serde(default = "default_line_item_object")]
    pub object: String,

    /// An arbitrary string attached to the object, often useful for displaying to users.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Total amount of the line item in minor units (e.g., cents) after discounts and taxes.
    pub amount_total: i64,

    /// Total of the line item before discounts or taxes in minor units (e.g., cents).
    pub amount_subtotal: i64,

    /// The amount of tax calculated for this line item in minor units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_tax: Option<i64>,

    /// The amount of discount applied to this line item in minor units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_discount: Option<i64>,

    /// Three-letter ISO currency code in lowercase (e.g., `usd`, `eur`).
    pub currency: String,

    /// The quantity of the item purchased.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<u32>,

    /// The price used for this line item, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<LineItemPrice>,
}

impl LineItem {
    /// Creates a new `LineItem` with mandatory amount and currency fields.
    pub fn new(
        id: impl Into<String>,
        amount_subtotal: i64,
        amount_total: i64,
        currency: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            object: default_line_item_object(),
            description: None,
            amount_total,
            amount_subtotal,
            amount_tax: None,
            amount_discount: None,
            currency: currency.into(),
            quantity: None,
            price: None,
        }
    }

    /// Sets the optional description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the optional quantity.
    #[must_use]
    pub fn with_quantity(mut self, quantity: u32) -> Self {
        self.quantity = Some(quantity);
        self
    }

    /// Sets the optional price.
    #[must_use]
    pub fn with_price(mut self, price: LineItemPrice) -> Self {
        self.price = Some(price);
        self
    }
}

/// Extracts the line item ID from a [`LineItem`].
///
/// Conforms strictly to the `fn(&T) -> &str` function pointer signature required by `Paginator::new`.
#[must_use]
pub fn line_item_id(item: &LineItem) -> &str {
    &item.id
}

/// Parameters for listing line items of a payment link (`GET /v1/payment_links/{id}/line_items`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListLineItemsParams {
    /// A limit on the number of objects to be returned (1 to 100). Default: 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// A cursor for use in pagination: an object ID defining your place in the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// A cursor for use in backward pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListLineItemsParams {
    /// Constructs a new, default [`ListLineItemsParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum number of items to return (clamped to 100).
    #[must_use]
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit.clamp(1, 100));
        self
    }

    /// Fetches the page of objects immediately following this object ID.
    #[must_use]
    pub fn starting_after(mut self, id: impl Into<String>) -> Self {
        self.starting_after = Some(id.into());
        self
    }

    /// Fetches the page of objects immediately preceding this object ID.
    #[must_use]
    pub fn ending_before(mut self, id: impl Into<String>) -> Self {
        self.ending_before = Some(id.into());
        self
    }
}

// ============================================================================
// Unit Tests (PaymentLink & Params)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_link_status_state_machine() {
        let status = PaymentLinkStatus::Active;
        assert!(status.is_active());
        assert!(!status.is_inactive());
        assert_eq!(status.as_str(), "active");

        let deactivated = status.deactivate();
        assert!(!deactivated.is_active());
        assert!(deactivated.is_inactive());
        assert_eq!(deactivated.as_str(), "inactive");

        let toggled = deactivated.toggle();
        assert!(toggled.is_active());

        let from_bool = PaymentLinkStatus::from(false);
        assert_eq!(from_bool, PaymentLinkStatus::Inactive);

        let parsed: PaymentLinkStatus = "active".parse().unwrap();
        assert_eq!(parsed, PaymentLinkStatus::Active);

        let parsed_false: PaymentLinkStatus = "false".parse().unwrap();
        assert_eq!(parsed_false, PaymentLinkStatus::Inactive);

        assert!("invalid".parse::<PaymentLinkStatus>().is_err());
    }

    #[test]
    fn test_payment_link_entity_serde_and_mutation() {
        let mut link = PaymentLink::new("plink_test_123", "https://buy.stripe.com/test_123", true)
            .with_currency("usd")
            .with_allow_promotion_codes(true);

        assert_eq!(link.status(), PaymentLinkStatus::Active);
        link.deactivate();
        assert_eq!(link.status(), PaymentLinkStatus::Inactive);
        assert!(!link.is_active());

        let json = serde_json::to_string(&link).expect("Serialize");
        let deserialized: PaymentLink = serde_json::from_str(&json).expect("Deserialize");
        assert_eq!(deserialized.id, "plink_test_123");
        assert!(!deserialized.active);
        assert_eq!(deserialized.currency.as_deref(), Some("usd"));
        assert_eq!(deserialized.allow_promotion_codes, Some(true));
    }

    #[test]
    fn test_create_params_builder_and_validation() {
        let params = CreatePaymentLinkParams::new(vec![
            PaymentLinkLineItemParams::new("price_item_1", 2)
                .adjustable_quantity(AdjustableQuantityParams::with_limits(1, 10)),
        ])
        .active(true)
        .after_completion(PaymentLinkAfterCompletionParams::redirect(
            "https://example.com/thanks",
        ))
        .allow_promotion_codes(true)
        .billing_address_collection("auto")
        .customer_creation("if_required")
        .metadata_entry("order_id", "9944")
        .payment_method_types(vec!["card".into(), "ideal".into()])
        .idempotency_key("idem_test_abc");

        assert!(params.validate().is_ok());
        assert_eq!(params.line_items.len(), 1);
        assert_eq!(params.line_items[0].quantity, 2);
        assert_eq!(params.idempotency_key.as_deref(), Some("idem_test_abc"));

        // Serde must omit idempotency_key
        let json = serde_json::to_string(&params).expect("Serialize");
        assert!(!json.contains("idem_test_abc"));
        assert!(!json.contains("idempotency_key"));
        assert!(json.contains("price_item_1"));
        assert!(json.contains("https://example.com/thanks"));
        assert!(json.contains(r#""type":"redirect""#));
    }

    #[test]
    fn test_create_params_validation_failures() {
        // Empty line items
        let empty = CreatePaymentLinkParams::new(vec![]);
        assert_eq!(
            empty.validate(),
            Err(PaymentLinkValidationError::EmptyLineItems)
        );

        // Quantity 0
        let zero_qty = CreatePaymentLinkParams::new(vec![PaymentLinkLineItemParams {
            price: Some("price_1".into()),
            quantity: 0,
            adjustable_quantity: None,
            price_data: None,
        }]);
        assert_eq!(
            zero_qty.validate(),
            Err(PaymentLinkValidationError::InvalidQuantity { quantity: 0 })
        );

        // Inverted adjustable limits
        let inv_limits = CreatePaymentLinkParams::new(vec![
            PaymentLinkLineItemParams::new("price_1", 1)
                .adjustable_quantity(AdjustableQuantityParams::with_limits(10, 2)),
        ]);
        assert_eq!(
            inv_limits.validate(),
            Err(
                PaymentLinkValidationError::InvalidAdjustableQuantityLimits {
                    minimum: 10,
                    maximum: 2,
                }
            )
        );
    }

    #[test]
    fn test_multi_currency_price_data() {
        let price_data = PaymentLinkPriceDataParams::new("usd", 1000)
            .currency_option("eur", CurrencyOptionParams::with_amount(900))
            .currency_option("gbp", CurrencyOptionParams::with_amount(800));

        assert!(price_data.validate().is_ok());

        let item = PaymentLinkLineItemParams::with_price_data(price_data, 1);
        let params = CreatePaymentLinkParams::new(vec![item]);
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_update_params_builder() {
        let update = UpdatePaymentLinkParams::new()
            .active(false)
            .allow_promotion_codes(false)
            .metadata_entry("status", "closed")
            .idempotency_key("idem_upd_123");

        assert_eq!(update.active, Some(false));
        assert_eq!(update.allow_promotion_codes, Some(false));
        assert_eq!(
            update.metadata.as_ref().unwrap().get("status").unwrap(),
            "closed"
        );
        assert_eq!(update.idempotency_key.as_deref(), Some("idem_upd_123"));
        assert!(update.validate().is_ok());

        let json = serde_json::to_string(&update).expect("Serialize");
        assert!(!json.contains("idem_upd_123"));
        assert!(json.contains(r#""active":false"#));
    }

    #[test]
    fn test_line_item_serde_and_extractor() {
        let price = LineItemPrice::new("price_test_999").with_amount(2500, "usd");
        let item = LineItem::new("li_test_999", 2500, 2500, "usd")
            .with_description("Premium Course")
            .with_quantity(2)
            .with_price(price);

        assert_eq!(line_item_id(&item), "li_test_999");
        assert_eq!(item.amount_total, 2500);
        assert_eq!(item.quantity, Some(2));
        assert_eq!(item.price.as_ref().unwrap().id, "price_test_999");

        let json = serde_json::to_string(&item).expect("Serialize");
        let deserialized: LineItem = serde_json::from_str(&json).expect("Deserialize");
        assert_eq!(deserialized.id, "li_test_999");
        assert_eq!(line_item_id(&deserialized), "li_test_999");
        assert_eq!(deserialized.amount_total, 2500);
        assert_eq!(deserialized.currency, "usd");
    }

    #[test]
    fn test_list_line_items_params() {
        let params = ListLineItemsParams::new()
            .limit(25)
            .starting_after("li_prev_123")
            .ending_before("li_next_456");

        assert_eq!(params.limit, Some(25));
        assert_eq!(params.starting_after.as_deref(), Some("li_prev_123"));
        assert_eq!(params.ending_before.as_deref(), Some("li_next_456"));

        let json = serde_json::to_string(&params).expect("Serialize");
        assert!(json.contains(r#""limit":25"#));
        assert!(json.contains(r#""starting_after":"li_prev_123""#));
    }
}
