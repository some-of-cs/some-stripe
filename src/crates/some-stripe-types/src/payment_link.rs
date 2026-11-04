//! # Stripe PaymentLink Types & State Machine
//!
//! Strongly typed domain models, status state machine, and helper predicates for Stripe Payment Links.
//!
//! A Payment Link represents a payment page that a customer can use to purchase items, sell products
//! or subscriptions, and accept donations.
//!
//! Under EU Omnibus Directive 2019/2161 and ISO 4217 integer arithmetic (Gate G-Law),
//! payment links manage shareable checkout URLs with transparent pricing and instant state transitions.

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

/// Represents a Stripe PaymentLink object.
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
    pub after_completion: Option<serde_json::Value>,

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
    pub fn with_after_completion(mut self, after_completion: serde_json::Value) -> Self {
        self.after_completion = Some(after_completion);
        self
    }

    /// Fluent builder method to set `livemode`.
    #[must_use]
    pub fn with_livemode(mut self, livemode: bool) -> Self {
        self.livemode = Some(livemode);
        self
    }
}
