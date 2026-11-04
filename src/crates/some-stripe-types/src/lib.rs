//! # some-stripe-types
//!
//! Strongly typed domain models, events, and request DTOs for the Stripe API,
//! modeled after `stripe-node`.
//!
//! ## Overview
//!
//! - **Domain Entities**: [`PaymentIntent`], [`PaymentIntentStatus`], [`CaptureMethod`], [`PaymentMethodOptions`], [`CheckoutSession`], [`Account`], [`Invoice`].
//! - **Event Pipeline**: [`Event`], [`EventData`], [`EventObject`].
//! - **Strongly Typed Identifiers**: [`AccountId`], [`IdempotencyKey`].
//! - **Invoice Upcoming Guard**: [`Invoice`] specifies `pub id: Option<String>` to safely accommodate
//!   `invoice.upcoming` preview events where `id` is null on Stripe wire payloads.
//! - **Discriminated Event Union**: [`EventObject`] provides seamless enum dispatch across
//!   `payment_intent`, `checkout.session`, `invoice`, and unknown types via `#[serde(other)]`.
//! - **Request Parameter Schemas & ISO 4217 Validation**: [`CreatePaymentIntentParams`],
//!   [`CapturePaymentIntentParams`], [`CancelPaymentIntentParams`], [`ConfirmPaymentIntentParams`].
//! - **PCI-DSS Compliance (Gate G-Law & G2)**: Sensitive credentials like `client_secret` in
//!   [`PaymentIntent`] and [`ConfirmPaymentIntentParams`] are masked as `[REDACTED]` in `Debug` implementations.
//!
//! ## Type Construction & Deserialization Example
//!
//! ```
//! use some_stripe_types::{AccountId, IdempotencyKey, PaymentIntent};
//!
//! let acct = AccountId::from("acct_12345");
//! assert_eq!(acct.to_string(), "acct_12345");
//!
//! let idem = IdempotencyKey::from("req_unique_999");
//! assert_eq!(idem.to_string(), "req_unique_999");
//!
//! let json = r#"{
//!     "id": "pi_3Mtw",
//!     "object": "payment_intent",
//!     "amount": 2000,
//!     "currency": "usd",
//!     "status": "requires_payment_method"
//! }"#;
//! let pi: PaymentIntent = serde_json::from_str(json).unwrap();
//! assert_eq!(pi.id, "pi_3Mtw");
//! assert_eq!(pi.amount, 2000);
//! ```
//!
//! ## Discriminated EventObject Example
//!
//! ```
//! use some_stripe_types::{EventData, EventObject};
//!
//! let data = EventData {
//!     object: serde_json::json!({
//!         "id": null,
//!         "object": "invoice",
//!         "customer": "cus_preview_123",
//!         "amount_due": 3500
//!     }),
//! };
//!
//! match data.deserialize_object().unwrap() {
//!     EventObject::Invoice(inv) => {
//!         assert!(inv.id.is_none());
//!         assert_eq!(inv.customer.as_deref(), Some("cus_preview_123"));
//!         assert_eq!(inv.amount_due, Some(3500));
//!     }
//!     _ => panic!("Expected Invoice variant"),
//! }
//! ```

use serde::{Deserialize, Serialize};

/// Strongly typed Stripe Connect Account ID (`acct_...`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId(pub String);

impl std::fmt::Display for AccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Into<String>> From<T> for AccountId {
    fn from(s: T) -> Self {
        Self(s.into())
    }
}

/// Strongly typed Stripe Idempotency Key header value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdempotencyKey(pub String);

impl std::fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Into<String>> From<T> for IdempotencyKey {
    fn from(s: T) -> Self {
        Self(s.into())
    }
}

/// Represents a Stripe Checkout Session object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutSession {
    /// Unique identifier for the checkout session (`cs_...`).
    pub id: String,

    /// Object type, always `checkout.session`.
    #[serde(default = "default_checkout_session_object")]
    pub object: String,

    /// The URL to the hosted checkout page, if available.
    pub url: Option<String>,

    /// The status of the checkout session: `open`, `complete`, or `expired`.
    pub status: Option<String>,

    /// The payment status: `paid`, `unpaid`, or `no_payment_required`.
    pub payment_status: Option<String>,

    /// The ID of the PaymentIntent created, if applicable.
    pub payment_intent: Option<String>,

    /// Total amount of items in minor units (e.g., cents).
    pub amount_total: Option<i64>,

    /// Three-letter ISO currency code in lowercase.
    pub currency: Option<String>,

    /// A unique string to reference the Checkout Session on your service.
    pub client_reference_id: Option<String>,

    /// The ID of the customer for this session.
    pub customer: Option<String>,

    /// The timestamp at which the checkout session expires.
    pub expires_at: Option<i64>,

    /// Time at which the object was created (UNIX epoch seconds).
    pub created: Option<i64>,
}

fn default_checkout_session_object() -> String {
    "checkout.session".into()
}

/// Parameters for creating a Stripe Checkout Session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCheckoutSessionParams {
    /// The URL the customer will be directed to upon successful payment.
    pub success_url: String,

    /// The URL the customer will be directed to if they decide to cancel.
    pub cancel_url: Option<String>,

    /// The mode of the checkout session (`payment`, `setup`, or `subscription`). Default: `payment`.
    #[serde(default = "default_payment_mode")]
    pub mode: String,

    /// Unique string to reference the Checkout Session on your system.
    pub client_reference_id: Option<String>,

    /// Currency code (e.g. `usd`, `eur`, `egp`).
    pub currency: String,

    /// The amount to be collected per unit in minor currency units (e.g., cents).
    pub unit_amount: i64,

    /// Name of the product or service displayed at checkout.
    pub product_name: String,

    /// Quantity of the item being purchased. Default: 1.
    #[serde(default = "default_quantity")]
    pub quantity: u32,

    /// Epoch timestamp at which the session expires (must be between 30 minutes and 24 hours).
    pub expires_at: Option<i64>,

    /// Idempotency key for safe retries.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

fn default_payment_mode() -> String {
    "payment".into()
}

fn default_quantity() -> u32 {
    1
}

/// Status of a [`PaymentIntent`].
///
/// Corresponds to the Stripe API `status` property.
///
/// # Example
/// ```
/// use some_stripe_types::PaymentIntentStatus;
/// use std::str::FromStr;
///
/// let status = PaymentIntentStatus::from_str("requires_payment_method").unwrap();
/// assert_eq!(status, PaymentIntentStatus::RequiresPaymentMethod);
/// assert_eq!(status.as_str(), "requires_payment_method");
/// assert_eq!(status.to_string(), "requires_payment_method");
/// assert_eq!(status, "requires_payment_method");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentIntentStatus {
    /// Next customer action is required to attach or select a payment method.
    RequiresPaymentMethod,
    /// The PaymentIntent is ready to be confirmed.
    RequiresConfirmation,
    /// Next customer action is required (e.g. 3D Secure authentication).
    RequiresAction,
    /// The payment is being processed by the payment network.
    Processing,
    /// The authorization succeeded and requires capture (for manual capture).
    RequiresCapture,
    /// The PaymentIntent was canceled.
    Canceled,
    /// The payment was successfully captured or authorized.
    Succeeded,
}

impl PaymentIntentStatus {
    /// Returns the Stripe API string representation of the status.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::RequiresPaymentMethod => "requires_payment_method",
            Self::RequiresConfirmation => "requires_confirmation",
            Self::RequiresAction => "requires_action",
            Self::Processing => "processing",
            Self::RequiresCapture => "requires_capture",
            Self::Canceled => "canceled",
            Self::Succeeded => "succeeded",
        }
    }
}

impl std::fmt::Display for PaymentIntentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error returned when parsing an unrecognized [`PaymentIntentStatus`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePaymentIntentStatusError(pub String);

impl std::fmt::Display for ParsePaymentIntentStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown PaymentIntentStatus: '{}'", self.0)
    }
}

impl std::error::Error for ParsePaymentIntentStatusError {}

impl std::str::FromStr for PaymentIntentStatus {
    type Err = ParsePaymentIntentStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "requires_payment_method" => Ok(Self::RequiresPaymentMethod),
            "requires_confirmation" => Ok(Self::RequiresConfirmation),
            "requires_action" => Ok(Self::RequiresAction),
            "processing" => Ok(Self::Processing),
            "requires_capture" => Ok(Self::RequiresCapture),
            "canceled" => Ok(Self::Canceled),
            "succeeded" => Ok(Self::Succeeded),
            _ => Err(ParsePaymentIntentStatusError(s.to_string())),
        }
    }
}

impl AsRef<str> for PaymentIntentStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<str> for PaymentIntentStatus {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for PaymentIntentStatus {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<PaymentIntentStatus> for str {
    fn eq(&self, other: &PaymentIntentStatus) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<PaymentIntentStatus> for &str {
    fn eq(&self, other: &PaymentIntentStatus) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<String> for PaymentIntentStatus {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<PaymentIntentStatus> for String {
    fn eq(&self, other: &PaymentIntentStatus) -> bool {
        self.as_str() == other.as_str()
    }
}

/// Capture method specifying when funds will be captured from customer's account.
///
/// # Example
/// ```
/// use some_stripe_types::CaptureMethod;
/// use std::str::FromStr;
///
/// let method = CaptureMethod::from_str("automatic").unwrap();
/// assert_eq!(method, CaptureMethod::Automatic);
/// assert_eq!(method.as_str(), "automatic");
/// assert_eq!(method.to_string(), "automatic");
/// assert_eq!(method, "automatic");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMethod {
    /// Stripe automatically captures the funds when authorized.
    Automatic,
    /// Funds are authorized on the cardholder's account and must be captured explicitly.
    Manual,
}

impl CaptureMethod {
    /// Returns the Stripe API string representation of the capture method.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::Manual => "manual",
        }
    }
}

impl std::fmt::Display for CaptureMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error returned when parsing an unrecognized [`CaptureMethod`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseCaptureMethodError(pub String);

impl std::fmt::Display for ParseCaptureMethodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown CaptureMethod: '{}'", self.0)
    }
}

impl std::error::Error for ParseCaptureMethodError {}

impl std::str::FromStr for CaptureMethod {
    type Err = ParseCaptureMethodError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "automatic" => Ok(Self::Automatic),
            "manual" => Ok(Self::Manual),
            _ => Err(ParseCaptureMethodError(s.to_string())),
        }
    }
}

impl AsRef<str> for CaptureMethod {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<str> for CaptureMethod {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for CaptureMethod {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<CaptureMethod> for str {
    fn eq(&self, other: &CaptureMethod) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<CaptureMethod> for &str {
    fn eq(&self, other: &CaptureMethod) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<String> for CaptureMethod {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<CaptureMethod> for String {
    fn eq(&self, other: &CaptureMethod) -> bool {
        self.as_str() == other.as_str()
    }
}

/// Payment method options configured on a [`PaymentIntent`].
///
/// # Example
/// ```
/// use some_stripe_types::{CardPaymentMethodOptions, CaptureMethod, PaymentMethodOptions};
///
/// let opts = PaymentMethodOptions::new().with_card(
///     CardPaymentMethodOptions::new()
///         .with_request_three_d_secure("any")
///         .with_capture_method(CaptureMethod::Manual),
/// );
/// assert!(opts.card.is_some());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentMethodOptions {
    /// Card-specific payment method configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<CardPaymentMethodOptions>,
}

impl PaymentMethodOptions {
    /// Creates a new, empty `PaymentMethodOptions`.
    pub const fn new() -> Self {
        Self { card: None }
    }

    /// Sets the card options.
    pub fn with_card(mut self, card: CardPaymentMethodOptions) -> Self {
        self.card = Some(card);
        self
    }
}

/// Options specific to card payments on a [`PaymentIntent`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPaymentMethodOptions {
    /// Controls 3D Secure authentication behavior (`automatic`, `any`, `challenge`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_three_d_secure: Option<String>,

    /// Controls capture method override specifically for card payments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_method: Option<CaptureMethod>,
}

impl CardPaymentMethodOptions {
    /// Creates a new, empty `CardPaymentMethodOptions`.
    pub const fn new() -> Self {
        Self {
            request_three_d_secure: None,
            capture_method: None,
        }
    }

    /// Sets the 3D Secure request policy (e.g., `"any"` or `"automatic"`).
    pub fn with_request_three_d_secure(mut self, policy: impl Into<String>) -> Self {
        self.request_three_d_secure = Some(policy.into());
        self
    }

    /// Sets the capture method for cards.
    pub fn with_capture_method(mut self, method: CaptureMethod) -> Self {
        self.capture_method = Some(method);
        self
    }
}

/// Represents a Stripe PaymentIntent object.
///
/// Implements custom `Debug` to mask `client_secret` to `[REDACTED]` (PCI-DSS 4.0 / Gate G2).
///
/// # Example
/// ```
/// use some_stripe_types::{CaptureMethod, PaymentIntent, PaymentIntentStatus};
///
/// let json = r#"{
///     "id": "pi_3Mtw",
///     "object": "payment_intent",
///     "amount": 2000,
///     "currency": "usd",
///     "status": "requires_payment_method",
///     "capture_method": "automatic"
/// }"#;
/// let pi: PaymentIntent = serde_json::from_str(json).unwrap();
/// assert_eq!(pi.id, "pi_3Mtw");
/// assert_eq!(pi.amount, 2000);
/// assert_eq!(pi.status, PaymentIntentStatus::RequiresPaymentMethod);
/// assert_eq!(pi.capture_method, Some(CaptureMethod::Automatic));
/// ```
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentIntent {
    /// Unique identifier for the payment intent (`pi_...`).
    pub id: String,

    /// Object type, always `payment_intent`.
    #[serde(default = "default_payment_intent_object")]
    pub object: String,

    /// Amount intended to be collected in minor currency units.
    pub amount: i64,

    /// Amount that was captured in minor units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_received: Option<i64>,

    /// Three-letter ISO currency code in lowercase.
    pub currency: String,

    /// Status of this PaymentIntent: `requires_payment_method`, `requires_confirmation`,
    /// `requires_action`, `processing`, `requires_capture`, `canceled`, or `succeeded`.
    pub status: PaymentIntentStatus,

    /// Controls when the funds will be captured from the customer's account: `automatic` or `manual`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_method: Option<CaptureMethod>,

    /// The client secret of this PaymentIntent used for client-side confirmation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,

    /// Challenge action payload for PSD2 SCA 3DS 2.0 redirects (Gate G-Law).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_action: Option<serde_json::Value>,

    /// Time at which the object was created (UNIX epoch seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,

    /// Reason for cancellation, if canceled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_reason: Option<String>,

    /// An arbitrary string attached to the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Payment-method-specific configuration for this PaymentIntent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_method_options: Option<PaymentMethodOptions>,
}

impl PaymentIntent {
    /// Constructs a new minimal `PaymentIntent`.
    pub fn new(
        id: impl Into<String>,
        amount: i64,
        currency: impl Into<String>,
        status: PaymentIntentStatus,
    ) -> Self {
        Self {
            id: id.into(),
            object: default_payment_intent_object(),
            amount,
            amount_received: None,
            currency: currency.into(),
            status,
            capture_method: None,
            client_secret: None,
            next_action: None,
            created: None,
            cancellation_reason: None,
            description: None,
            payment_method_options: None,
        }
    }
}

impl std::fmt::Debug for PaymentIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaymentIntent")
            .field("id", &self.id)
            .field("object", &self.object)
            .field("amount", &self.amount)
            .field("amount_received", &self.amount_received)
            .field("currency", &self.currency)
            .field("status", &self.status)
            .field("capture_method", &self.capture_method)
            .field(
                "client_secret",
                &self.client_secret.as_ref().map(|_| "[REDACTED]"),
            )
            .field("next_action", &self.next_action)
            .field("created", &self.created)
            .field("cancellation_reason", &self.cancellation_reason)
            .field("description", &self.description)
            .field("payment_method_options", &self.payment_method_options)
            .finish()
    }
}

fn default_payment_intent_object() -> String {
    "payment_intent".into()
}

pub mod params;
pub use params::*;

/// Represents a Stripe Account object (merchant profile).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Unique identifier for the account (`acct_...`).
    pub id: String,

    /// Object type, always `account`.
    #[serde(default = "default_account_object")]
    pub object: String,

    /// Whether the account can create live charges.
    pub charges_enabled: Option<bool>,

    /// Whether Stripe can send payouts to this account.
    pub payouts_enabled: Option<bool>,

    /// Country of the account.
    pub country: Option<String>,

    /// Default currency code for the account.
    pub default_currency: Option<String>,
}

fn default_account_object() -> String {
    "account".into()
}

/// Represents a Stripe Invoice object.
///
/// Handles `invoice.upcoming` preview events where `id` is `None` (JSON `null`).
///
/// # Example
/// ```
/// use some_stripe_types::Invoice;
///
/// let json = r#"{
///     "id": null,
///     "object": "invoice",
///     "customer": "cus_test_123",
///     "amount_due": 4500,
///     "currency": "usd"
/// }"#;
/// let invoice: Invoice = serde_json::from_str(json).unwrap();
/// assert!(invoice.id.is_none());
/// assert_eq!(invoice.amount_due, Some(4500));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    /// Unique identifier for the invoice (`in_...`).
    ///
    /// For upcoming invoice preview events (`invoice.upcoming`), `id` is `None` (JSON `null`).
    pub id: Option<String>,

    /// Object type, always `invoice`.
    #[serde(default = "default_invoice_object")]
    pub object: String,

    /// The ID of the customer who will be billed.
    pub customer: Option<String>,

    /// Final amount due at this time for this invoice in minor currency units (e.g., cents).
    pub amount_due: Option<i64>,

    /// The amount, in minor currency units, that was paid.
    pub amount_paid: Option<i64>,

    /// The difference between amount_due and amount_paid, in minor currency units.
    pub amount_remaining: Option<i64>,

    /// Total after discounts and taxes in minor currency units.
    pub total: Option<i64>,

    /// Total of all items before discounts and taxes in minor currency units.
    pub subtotal: Option<i64>,

    /// Three-letter ISO currency code in lowercase.
    pub currency: Option<String>,

    /// The status of the invoice: `draft`, `open`, `paid`, `uncollectible`, or `void`.
    pub status: Option<String>,

    /// Whether payment was successfully made for this invoice.
    pub paid: Option<bool>,

    /// The ID of the subscription that this invoice was prepared for, if any.
    pub subscription: Option<String>,

    /// Start of the usage period that this invoice covers (UNIX epoch seconds).
    pub period_start: Option<i64>,

    /// End of the usage period that this invoice covers (UNIX epoch seconds).
    pub period_end: Option<i64>,

    /// Time at which the object was created (UNIX epoch seconds).
    pub created: Option<i64>,
}

fn default_invoice_object() -> String {
    "invoice".into()
}

/// Strongly typed, discriminated union of Stripe event payload objects.
///
/// Tagged internally by the `"object"` field in JSON payloads.
/// Unrecognized or future object types cleanly fall back to [`EventObject::Unknown`]
/// without deserialization failure or panics.
///
/// # Example
/// ```
/// use some_stripe_types::EventObject;
///
/// let json = r#"{
///     "id": "pi_test_123",
///     "object": "payment_intent",
///     "amount": 2000,
///     "currency": "usd",
///     "status": "succeeded"
/// }"#;
/// let obj: EventObject = serde_json::from_str(json).unwrap();
/// match obj {
///     EventObject::PaymentIntent(pi) => assert_eq!(pi.id, "pi_test_123"),
///     _ => panic!("Expected PaymentIntent"),
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "object", rename_all = "snake_case")]
pub enum EventObject {
    /// Stripe PaymentIntent payload (`object = "payment_intent"`).
    PaymentIntent(PaymentIntent),

    /// Stripe CheckoutSession payload (`object = "checkout.session"`).
    #[serde(rename = "checkout.session")]
    CheckoutSession(CheckoutSession),

    /// Stripe Invoice payload (`object = "invoice"`), including upcoming invoices with null `id`.
    Invoice(Invoice),

    /// Unrecognized, custom, or unsupported event object types.
    #[serde(other)]
    Unknown,
}

/// Represents a Stripe Webhook Event object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique identifier for the event (`evt_...`).
    pub id: String,

    /// Object type, always `event`.
    #[serde(default = "default_event_object")]
    pub object: String,

    /// The Stripe API version by which the event was generated.
    pub api_version: Option<String>,

    /// Time at which the object was created (UNIX epoch seconds).
    pub created: i64,

    /// Description of the event (e.g. `checkout.session.completed`, `payment_intent.succeeded`).
    #[serde(rename = "type")]
    pub event_type: String,

    /// Object containing data associated with the event.
    pub data: EventData,
}

impl Event {
    /// Deserializes the inner event data payload into a strongly typed [`EventObject`] discriminated union.
    ///
    /// # Example
    /// ```
    /// use some_stripe_types::{Event, EventData, EventObject};
    ///
    /// let event = Event {
    ///     id: "evt_test".into(),
    ///     object: "event".into(),
    ///     api_version: None,
    ///     created: 1728345600,
    ///     event_type: "invoice.upcoming".into(),
    ///     data: EventData {
    ///         object: serde_json::json!({
    ///             "id": null,
    ///             "object": "invoice",
    ///             "customer": "cus_123"
    ///         }),
    ///     },
    /// };
    /// match event.deserialize_object().unwrap() {
    ///     EventObject::Invoice(inv) => assert!(inv.id.is_none()),
    ///     _ => panic!("Expected Invoice"),
    /// }
    /// ```
    pub fn deserialize_object(&self) -> Result<EventObject, serde_json::Error> {
        self.data.deserialize_object()
    }
}

fn default_event_object() -> String {
    "event".into()
}

/// Container for the data object payload of an Event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    /// Raw JSON object containing the payload (e.g. CheckoutSession, PaymentIntent, or Invoice).
    pub object: serde_json::Value,
}

impl EventData {
    /// Deserializes the inner raw JSON `object` into a strongly typed [`EventObject`] discriminated union.
    ///
    /// Handles `checkout.session`, `payment_intent`, and `invoice` (including `invoice.upcoming`
    /// where `id` is null). Unrecognized or future object types cleanly deserialize into
    /// [`EventObject::Unknown`] without error or panic.
    ///
    /// # Errors
    /// Returns `Err(serde_json::Error)` if the payload cannot be parsed as an [`EventObject`].
    ///
    /// # Example
    /// ```
    /// use some_stripe_types::{EventData, EventObject};
    ///
    /// let data = EventData {
    ///     object: serde_json::json!({
    ///         "id": null,
    ///         "object": "invoice",
    ///         "amount_due": 1200
    ///     }),
    /// };
    /// match data.deserialize_object().unwrap() {
    ///     EventObject::Invoice(inv) => {
    ///         assert!(inv.id.is_none());
    ///         assert_eq!(inv.amount_due, Some(1200));
    ///     }
    ///     _ => panic!("Expected Invoice"),
    /// }
    /// ```
    pub fn deserialize_object(&self) -> Result<EventObject, serde_json::Error> {
        EventObject::deserialize(&self.object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_intent_debug_redaction() {
        let pi = PaymentIntent {
            id: "pi_123".into(),
            object: "payment_intent".into(),
            amount: 5000,
            amount_received: Some(5000),
            currency: "usd".into(),
            status: PaymentIntentStatus::Succeeded,
            capture_method: Some(CaptureMethod::Automatic),
            client_secret: Some("pi_123_secret_xyzSuperSecretToken".into()),
            next_action: None,
            created: Some(1728345600),
            cancellation_reason: None,
            description: Some("Test payment".into()),
            payment_method_options: None,
        };

        let debug_str = format!("{pi:?}");
        assert!(debug_str.contains("[REDACTED]"));
        assert!(!debug_str.contains("xyzSuperSecretToken"));

        // Redaction verification when client_secret is None
        let pi_no_secret = PaymentIntent {
            id: "pi_123_no_secret".into(),
            object: "payment_intent".into(),
            amount: 1000,
            amount_received: None,
            currency: "usd".into(),
            status: PaymentIntentStatus::RequiresPaymentMethod,
            capture_method: None,
            client_secret: None,
            next_action: None,
            created: None,
            cancellation_reason: None,
            description: None,
            payment_method_options: None,
        };
        let debug_str_none = format!("{pi_no_secret:?}");
        assert!(debug_str_none.contains("client_secret: None"));
        assert!(!debug_str_none.contains("[REDACTED]"));
    }

    #[test]
    fn test_payment_intent_status_variants_and_parsing() {
        use std::str::FromStr;

        let pairs = [
            (
                PaymentIntentStatus::RequiresPaymentMethod,
                "requires_payment_method",
            ),
            (
                PaymentIntentStatus::RequiresConfirmation,
                "requires_confirmation",
            ),
            (PaymentIntentStatus::RequiresAction, "requires_action"),
            (PaymentIntentStatus::Processing, "processing"),
            (PaymentIntentStatus::RequiresCapture, "requires_capture"),
            (PaymentIntentStatus::Canceled, "canceled"),
            (PaymentIntentStatus::Succeeded, "succeeded"),
        ];

        for (status, expected_str) in pairs {
            // as_str, Display, AsRef
            assert_eq!(status.as_str(), expected_str);
            assert_eq!(status.to_string(), expected_str);
            assert_eq!(status.as_ref(), expected_str);

            // FromStr
            let parsed = PaymentIntentStatus::from_str(expected_str).unwrap();
            assert_eq!(parsed, status);

            // PartialEq with &str, str, and String
            assert_eq!(status, expected_str);
            assert_eq!(expected_str, status);
            let s = expected_str.to_string();
            assert_eq!(status, s);
            assert_eq!(s, status);

            // Serde roundtrip
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, format!("\"{expected_str}\""));
            let deserialized: PaymentIntentStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, status);
        }

        // Invalid status parsing
        let invalid_err = PaymentIntentStatus::from_str("unknown_status").unwrap_err();
        assert_eq!(
            invalid_err,
            ParsePaymentIntentStatusError("unknown_status".to_string())
        );
        assert!(invalid_err.to_string().contains("unknown_status"));
    }

    #[test]
    fn test_capture_method_variants_and_parsing() {
        use std::str::FromStr;

        let pairs = [
            (CaptureMethod::Automatic, "automatic"),
            (CaptureMethod::Manual, "manual"),
        ];

        for (method, expected_str) in pairs {
            assert_eq!(method.as_str(), expected_str);
            assert_eq!(method.to_string(), expected_str);
            assert_eq!(method.as_ref(), expected_str);

            let parsed = CaptureMethod::from_str(expected_str).unwrap();
            assert_eq!(parsed, method);

            assert_eq!(method, expected_str);
            assert_eq!(expected_str, method);
            let s = expected_str.to_string();
            assert_eq!(method, s);
            assert_eq!(s, method);

            let json = serde_json::to_string(&method).unwrap();
            assert_eq!(json, format!("\"{expected_str}\""));
            let deserialized: CaptureMethod = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, method);
        }

        let invalid_err = CaptureMethod::from_str("instant").unwrap_err();
        assert_eq!(invalid_err, ParseCaptureMethodError("instant".to_string()));
        assert!(invalid_err.to_string().contains("instant"));
    }

    #[test]
    fn test_payment_method_options_serialization() {
        let opts = PaymentMethodOptions::new().with_card(
            CardPaymentMethodOptions::new()
                .with_request_three_d_secure("any")
                .with_capture_method(CaptureMethod::Manual),
        );

        let json = serde_json::to_string(&opts).unwrap();
        assert!(json.contains("\"request_three_d_secure\":\"any\""));
        assert!(json.contains("\"capture_method\":\"manual\""));

        let deserialized: PaymentMethodOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, opts);

        // Empty options omit card
        let empty_opts = PaymentMethodOptions::new();
        let empty_json = serde_json::to_string(&empty_opts).unwrap();
        assert_eq!(empty_json, "{}");
    }

    #[test]
    fn test_payment_intent_full_lifecycle_model() {
        let mut pi = PaymentIntent::new(
            "pi_full_test_123",
            10000,
            "usd",
            PaymentIntentStatus::RequiresPaymentMethod,
        );
        assert_eq!(pi.id, "pi_full_test_123");
        assert_eq!(pi.object, "payment_intent");
        assert_eq!(pi.amount, 10000);
        assert_eq!(pi.currency, "usd");
        assert_eq!(pi.status, PaymentIntentStatus::RequiresPaymentMethod);
        assert!(pi.amount_received.is_none());

        // Transition status and populate options
        pi.status = PaymentIntentStatus::RequiresCapture;
        pi.capture_method = Some(CaptureMethod::Manual);
        pi.payment_method_options =
            Some(PaymentMethodOptions::new().with_card(
                CardPaymentMethodOptions::new().with_capture_method(CaptureMethod::Manual),
            ));

        let json = serde_json::to_string(&pi).unwrap();
        let roundtrip: PaymentIntent = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.status, PaymentIntentStatus::RequiresCapture);
        assert_eq!(roundtrip.capture_method, Some(CaptureMethod::Manual));
        assert_eq!(roundtrip.amount, 10000);
    }

    #[test]
    fn test_event_object_checkout_session() {
        let json = serde_json::json!({
            "id": "cs_test_session_456",
            "object": "checkout.session",
            "status": "complete",
            "payment_status": "paid",
            "amount_total": 4500,
            "currency": "usd"
        });

        let data = EventData { object: json };
        let obj = data
            .deserialize_object()
            .expect("Failed to deserialize checkout session");

        match obj {
            EventObject::CheckoutSession(cs) => {
                assert_eq!(cs.id, "cs_test_session_456");
                assert_eq!(cs.status.as_deref(), Some("complete"));
                assert_eq!(cs.payment_status.as_deref(), Some("paid"));
                assert_eq!(cs.amount_total, Some(4500));
                assert_eq!(cs.currency.as_deref(), Some("usd"));
            }
            other => panic!("Expected CheckoutSession variant, got {other:?}"),
        }
    }

    #[test]
    fn test_event_object_payment_intent() {
        let json = serde_json::json!({
            "id": "pi_test_intent_789",
            "object": "payment_intent",
            "amount": 9000,
            "currency": "eur",
            "status": "succeeded"
        });

        let data = EventData { object: json };
        let obj = data
            .deserialize_object()
            .expect("Failed to deserialize payment intent");

        match obj {
            EventObject::PaymentIntent(pi) => {
                assert_eq!(pi.id, "pi_test_intent_789");
                assert_eq!(pi.amount, 9000);
                assert_eq!(pi.currency, "eur");
                assert_eq!(pi.status, "succeeded");
            }
            other => panic!("Expected PaymentIntent variant, got {other:?}"),
        }
    }

    #[test]
    fn test_event_object_invoice_upcoming_null_id() {
        let json = serde_json::json!({
            "id": null,
            "object": "invoice",
            "customer": "cus_upcoming_user",
            "amount_due": 1999,
            "currency": "usd",
            "status": "draft"
        });

        let data = EventData { object: json };
        let obj = data
            .deserialize_object()
            .expect("Failed to deserialize invoice.upcoming with null id");

        match obj {
            EventObject::Invoice(inv) => {
                assert!(inv.id.is_none(), "Upcoming invoice id must be None");
                assert_eq!(inv.object, "invoice");
                assert_eq!(inv.customer.as_deref(), Some("cus_upcoming_user"));
                assert_eq!(inv.amount_due, Some(1999));
                assert_eq!(inv.currency.as_deref(), Some("usd"));
                assert_eq!(inv.status.as_deref(), Some("draft"));
            }
            other => panic!("Expected Invoice variant, got {other:?}"),
        }
    }

    #[test]
    fn test_event_object_invoice_with_id() {
        let json = serde_json::json!({
            "id": "in_finalized_123",
            "object": "invoice",
            "customer": "cus_customer_123",
            "amount_due": 5000,
            "amount_paid": 5000,
            "amount_remaining": 0,
            "currency": "usd",
            "status": "paid"
        });

        let data = EventData { object: json };
        let obj = data
            .deserialize_object()
            .expect("Failed to deserialize finalized invoice");

        match obj {
            EventObject::Invoice(inv) => {
                assert_eq!(inv.id.as_deref(), Some("in_finalized_123"));
                assert_eq!(inv.amount_due, Some(5000));
                assert_eq!(inv.amount_paid, Some(5000));
                assert_eq!(inv.amount_remaining, Some(0));
                assert_eq!(inv.status.as_deref(), Some("paid"));
            }
            other => panic!("Expected Invoice variant, got {other:?}"),
        }
    }

    #[test]
    fn test_event_object_unrecognized_fallback_to_unknown() {
        let json = serde_json::json!({
            "id": "ch_custom_charge_456",
            "object": "charge",
            "amount": 2500,
            "currency": "usd"
        });

        let data = EventData { object: json };
        let obj = data
            .deserialize_object()
            .expect("Should cleanly fall back to EventObject::Unknown");

        assert!(matches!(obj, EventObject::Unknown));
    }

    #[test]
    fn test_event_deserialize_object_passthrough() {
        let event = Event {
            id: "evt_123".into(),
            object: "event".into(),
            api_version: Some("2024-06-20".into()),
            created: 1728345600,
            event_type: "invoice.upcoming".into(),
            data: EventData {
                object: serde_json::json!({
                    "id": null,
                    "object": "invoice",
                    "customer": "cus_pass_777",
                    "amount_due": 1200
                }),
            },
        };

        let obj = event
            .deserialize_object()
            .expect("Event::deserialize_object failed");
        match obj {
            EventObject::Invoice(inv) => {
                assert!(inv.id.is_none());
                assert_eq!(inv.customer.as_deref(), Some("cus_pass_777"));
                assert_eq!(inv.amount_due, Some(1200));
            }
            other => panic!("Expected Invoice variant, got {other:?}"),
        }
    }

    #[test]
    fn test_create_and_confirm_payment_intent_params() {
        let create_params = CreatePaymentIntentParams {
            amount: 5000,
            currency: "usd".into(),
            capture_method: Some("manual".into()),
            description: Some("Order #1024".into()),
            payment_method: Some("pm_test_card".into()),
            confirm: Some(true),
            return_url: Some("https://example.com/complete".into()),
            idempotency_key: Some("idem_create_123".into()),
        };
        assert_eq!(create_params.amount, 5000);
        assert_eq!(create_params.currency, "usd");
        assert_eq!(create_params.confirm, Some(true));

        let confirm_params = ConfirmPaymentIntentParams {
            payment_method: Some("pm_test_456".into()),
            return_url: Some("https://example.com/return".into()),
            receipt_email: Some("billing@example.com".into()),
            setup_future_usage: Some("off_session".into()),
            idempotency_key: Some("idem_conf_789".into()),
        };
        assert_eq!(
            confirm_params.payment_method.as_deref(),
            Some("pm_test_456")
        );
        assert_eq!(
            confirm_params.setup_future_usage.as_deref(),
            Some("off_session")
        );
    }
}
