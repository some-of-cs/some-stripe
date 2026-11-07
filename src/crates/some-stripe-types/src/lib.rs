//! # some-stripe-types
//!
//! Strongly typed domain models, events, and request DTOs for the Stripe API,
//! modeled after `stripe-node`.
//!
//! ## Overview
//!
//! - **Domain Entities**: [`PaymentIntent`], [`PaymentIntentStatus`], [`CaptureMethod`], [`PaymentMethodOptions`], [`CheckoutSession`], [`LineItem`], [`LineItemList`], [`Account`], [`AccountLink`], [`AccountSession`], [`Invoice`], [`Dispute`], [`Refund`], [`Balance`], [`BalanceTransaction`], [`PaymentLink`], [`PaymentLinkStatus`].
//! - **ISO 4217 Integer Arithmetic (Gate G-Law)**: [`currency`] module provides zero-float exact integer arithmetic across non-decimal, 2-decimal, and 3-decimal currencies.
//! - **Event Pipeline**: [`Event`], [`EventData`], [`EventObject`].
//! - **Statutory Legal Compliance**: [`is_statutory_withdrawal_compliant`] enforces EU Directive 2011/83/EU Article 13.
//! - **Immutable Ledger Reconciliation**: [`validate_ledger_reconciliation`] verifies $\text{net} = \text{amount} - \text{fee}$ across batches and streams.
//! - **Strongly Typed Identifiers**: [`AccountId`], [`IdempotencyKey`], [`DisputeId`], [`PaymentLinkId`].
//! - **Dispute & Evidence Management**: [`Dispute`], [`DisputeEvidence`], [`DisputeEvidenceDetails`],
//!   [`DisputeStatus`], and [`UpdateDisputeParams`] with submission locking invariants.
//! - **Invoice Upcoming Guard**: [`Invoice`] specifies `pub id: Option<String>` to safely accommodate
//!   `invoice.upcoming` preview events where `id` is null on Stripe wire payloads.
//! - **Discriminated Event Union**: [`EventObject`] provides seamless enum dispatch across
//!   `payment_intent`, `checkout.session`, `invoice`, `dispute`, `refund`, `payment_link`, and unknown types via `#[serde(other)]`.
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

pub mod next_action;

pub use next_action::{NextAction, NextActionType, RedirectToUrl};
use serde::{Deserialize, Serialize};

pub mod balance;
pub use balance::*;
pub mod currency;
pub use currency::*;
pub mod dispute;
pub use dispute::*;
pub mod refund;
pub use refund::*;
pub mod checkout;
pub use checkout::*;
pub mod payment_link;
pub use payment_link::*;
pub mod omnibus;
pub use omnibus::*;
pub mod billing_portal;
pub use billing_portal::*;
pub mod customer;
pub use customer::*;
pub mod subscription;
#[allow(unused_imports)]
pub use subscription::*;
pub mod fee;
pub use fee::*;
pub mod fee_params;
#[allow(unused_imports)]
pub use fee_params::*;
pub mod nullable;
pub use nullable::*;
pub mod account_link;
pub use account_link::*;
pub mod account_session;
pub use account_session::*;
pub mod person;
pub use person::*;
pub mod transfer;
pub use transfer::*;
pub mod transfer_params;
pub use transfer_params::*;
pub mod settlement;
pub use settlement::*;
pub mod identity_session;
pub use identity_session::*;
pub mod identity_report;
pub use identity_report::*;
pub mod tax_calculation;
pub use tax_calculation::*;
pub mod tax_settings;
pub use tax_settings::*;
pub mod tax_transaction;
pub use tax_calculation::TaxShippingCost;
pub use tax_transaction::*;
pub mod issuing_card;
pub use issuing_card::*;
pub mod issuing_cardholder;
pub use issuing_cardholder::*;
pub mod issuing_authorization;
pub use issuing_authorization::*;
pub mod issuing_transaction;
pub use issuing_transaction::*;
pub mod treasury_financial_account;
pub use treasury_financial_account::*;
pub mod treasury_transfer;
pub use treasury_transfer::*;
pub mod terminal;
pub use terminal::*;
pub mod v2_destination;
pub use v2_destination::*;

pub mod v2 {
    pub mod core {
        pub use crate::v2_destination::*;
    }
}

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

    /// Returns the strongly typed 3D Secure / PSD2 [`NextAction`] payload, if present and valid.
    ///
    /// # Example
    /// ```
    /// use some_stripe_types::{NextActionType, PaymentIntent};
    ///
    /// let json = r#"{
    ///     "id": "pi_3Mtw",
    ///     "object": "payment_intent",
    ///     "amount": 2000,
    ///     "currency": "usd",
    ///     "status": "requires_action",
    ///     "next_action": {
    ///         "type": "redirect_to_url",
    ///         "redirect_to_url": {
    ///             "url": "https://hooks.stripe.com/3d_secure_2/hosted",
    ///             "return_url": "https://example.com/return"
    ///         }
    ///     }
    /// }"#;
    /// let pi: PaymentIntent = serde_json::from_str(json).unwrap();
    /// let action = pi.next_action().expect("next_action must be present");
    /// assert_eq!(action.r#type, NextActionType::RedirectToUrl);
    /// assert_eq!(action.redirect_url(), Some("https://hooks.stripe.com/3d_secure_2/hosted"));
    /// ```
    pub fn next_action(&self) -> Option<NextAction> {
        self.parse_next_action().ok().flatten()
    }

    /// Parses the raw `next_action` JSON payload into a strongly-typed [`NextAction`].
    ///
    /// # Errors
    /// Returns `Err(serde_json::Error)` if `next_action` is present but does not match
    /// the expected [`NextAction`] schema.
    pub fn parse_next_action(&self) -> Result<Option<NextAction>, serde_json::Error> {
        match &self.next_action {
            Some(value) => NextAction::deserialize(value).map(Some),
            None => Ok(None),
        }
    }

    /// Sets the strongly typed [`NextAction`] payload.
    pub fn set_next_action(
        &mut self,
        next_action: Option<NextAction>,
    ) -> Result<(), serde_json::Error> {
        self.next_action = match next_action {
            Some(action) => Some(serde_json::to_value(action)?),
            None => None,
        };
        Ok(())
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

impl PaymentIntent {
    /// Returns `true` if the payment succeeded (`status == PaymentIntentStatus::Succeeded`).
    pub fn is_succeeded(&self) -> bool {
        self.status == PaymentIntentStatus::Succeeded
    }

    /// Returns `true` if the payment has been authorized and requires capture (`status == PaymentIntentStatus::RequiresCapture`).
    pub fn requires_capture(&self) -> bool {
        self.status == PaymentIntentStatus::RequiresCapture
    }

    /// Returns `true` if customer authentication action is required (`status == PaymentIntentStatus::RequiresAction`).
    pub fn requires_action(&self) -> bool {
        self.status == PaymentIntentStatus::RequiresAction
    }

    /// Returns `true` if the payment was canceled (`status == PaymentIntentStatus::Canceled`).
    pub fn is_canceled(&self) -> bool {
        self.status == PaymentIntentStatus::Canceled
    }

    /// Returns the PSD2 3D Secure redirect URL, if present in `next_action`.
    ///
    /// Extracts the URL by reference (`Option<&str>`) without heap cloning (Gate G-Borrow).
    ///
    /// # Example
    /// ```
    /// use some_stripe_types::PaymentIntent;
    ///
    /// let json = r#"{
    ///     "id": "pi_3Mtw",
    ///     "object": "payment_intent",
    ///     "amount": 2000,
    ///     "currency": "usd",
    ///     "status": "requires_action",
    ///     "next_action": {
    ///         "type": "redirect_to_url",
    ///         "redirect_to_url": {
    ///             "url": "https://hooks.stripe.com/redirect/authenticate/src_123"
    ///         }
    ///     }
    /// }"#;
    /// let pi: PaymentIntent = serde_json::from_str(json).unwrap();
    /// assert_eq!(
    ///     pi.redirect_url(),
    ///     Some("https://hooks.stripe.com/redirect/authenticate/src_123")
    /// );
    /// ```
    pub fn redirect_url(&self) -> Option<&str> {
        self.next_action
            .as_ref()?
            .get("redirect_to_url")?
            .get("url")?
            .as_str()
    }
}

fn default_payment_intent_object() -> String {
    "payment_intent".into()
}

pub mod params;
pub use params::*;

pub mod account;
pub use account::*;

pub mod invoice;
pub use invoice::*;
pub mod invoice_params;
pub use invoice_params::*;
pub mod credit_note;
pub use credit_note::*;
pub mod quote;
pub use quote::*;
pub mod quote_params;
pub use quote_params::*;
pub mod connect_compliance;
pub use connect_compliance::*;
pub mod payout;
pub use payout::*;
pub mod v2_events;
pub use v2_events::*;

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
#[allow(clippy::large_enum_variant)]
pub enum EventObject {
    /// Stripe PaymentIntent payload (`object = "payment_intent"`).
    PaymentIntent(PaymentIntent),

    /// Stripe CheckoutSession payload (`object = "checkout.session"`).
    #[serde(rename = "checkout.session")]
    CheckoutSession(CheckoutSession),

    /// Stripe Invoice payload (`object = "invoice"`), including upcoming invoices with null `id`.
    Invoice(Invoice),

    /// Stripe Dispute payload (`object = "dispute"`).
    Dispute(Dispute),

    /// Stripe Refund payload (`object = "refund"`).
    Refund(Refund),

    /// Stripe PaymentLink payload (`object = "payment_link"`).
    #[serde(rename = "payment_link")]
    PaymentLink(PaymentLink),

    /// Stripe CreditNote payload (`object = "credit_note"`).
    #[serde(rename = "credit_note")]
    CreditNote(CreditNote),

    /// Stripe Quote payload (`object = "quote"`).
    #[serde(rename = "quote")]
    Quote(Quote),

    /// Stripe PortalSession payload (`object = "billing_portal.session"`).
    #[serde(rename = "billing_portal.session")]
    PortalSession(PortalSession),

    /// Stripe PortalConfiguration payload (`object = "billing_portal.configuration"`).
    #[serde(rename = "billing_portal.configuration")]
    PortalConfiguration(PortalConfiguration),

    /// Stripe Customer payload (`object = "customer"`).
    Customer(Customer),

    /// Stripe Account payload (`object = "account"`).
    Account(Account),

    /// Stripe Transfer payload (`object = "transfer"`).
    Transfer(Transfer),

    /// Stripe TransferReversal payload (`object = "transfer_reversal"`).
    #[serde(rename = "transfer_reversal")]
    TransferReversal(TransferReversal),

    /// Stripe ApplicationFee payload (`object = "application_fee"`).
    ApplicationFee(ApplicationFee),

    /// Stripe FeeRefund payload (`object = "fee_refund"`).
    FeeRefund(FeeRefund),

    /// Stripe Payout payload (`object = "payout"`).
    Payout(Payout),

    /// Stripe Identity VerificationReport payload (`object = "identity.verification_report"`).
    #[serde(rename = "identity.verification_report")]
    VerificationReport(VerificationReport),

    /// Stripe TaxCalculation payload (`object = "tax.calculation"`).
    #[serde(rename = "tax.calculation")]
    TaxCalculation(TaxCalculation),

    /// Stripe TaxTransaction payload (`object = "tax.transaction"`).
    #[serde(rename = "tax.transaction")]
    TaxTransaction(TaxTransaction),

    /// Stripe Issuing Card payload (`object = "issuing.card"`).
    #[serde(rename = "issuing.card")]
    IssuingCard(IssuingCard),

    /// Stripe Issuing Cardholder payload (`object = "issuing.cardholder"`).
    #[serde(rename = "issuing.cardholder")]
    IssuingCardholder(IssuingCardholder),

    /// Stripe Issuing Authorization payload (`object = "issuing.authorization"`).
    #[serde(rename = "issuing.authorization")]
    IssuingAuthorization(IssuingAuthorization),

    /// Stripe Issuing Transaction payload (`object = "issuing.transaction"`).
    #[serde(rename = "issuing.transaction")]
    IssuingTransaction(IssuingTransaction),

    /// Stripe Treasury FinancialAccount payload (`object = "treasury.financial_account"`).
    #[serde(rename = "treasury.financial_account")]
    TreasuryFinancialAccount(FinancialAccount),

    /// Stripe Treasury OutboundTransfer payload (`object = "treasury.outbound_transfer"`).
    #[serde(rename = "treasury.outbound_transfer")]
    TreasuryOutboundTransfer(OutboundTransfer),

    /// Stripe Treasury OutboundPayment payload (`object = "treasury.outbound_payment"`).
    #[serde(rename = "treasury.outbound_payment")]
    TreasuryOutboundPayment(OutboundPayment),

    /// Stripe Treasury ReceivedCredit payload (`object = "treasury.received_credit"`).
    #[serde(rename = "treasury.received_credit")]
    TreasuryReceivedCredit(ReceivedCredit),

    /// Stripe Terminal Reader payload (`object = "terminal.reader"`).
    #[serde(rename = "terminal.reader")]
    TerminalReader(TerminalReader),

    /// Stripe Terminal Location payload (`object = "terminal.location"`).
    #[serde(rename = "terminal.location")]
    TerminalLocation(TerminalLocation),

    /// Stripe Terminal ConnectionToken payload (`object = "terminal.connection_token"`).
    #[serde(rename = "terminal.connection_token")]
    TerminalConnectionToken(ConnectionToken),

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
    fn test_event_object_dispute() {
        let json = serde_json::json!({
            "id": "dp_event_test_1",
            "object": "dispute",
            "amount": 3500,
            "currency": "usd",
            "status": "needs_response",
            "reason": "fraudulent"
        });

        let data = EventData { object: json };
        let obj = data
            .deserialize_object()
            .expect("Failed to deserialize dispute event");

        match obj {
            EventObject::Dispute(dp) => {
                assert_eq!(dp.id, "dp_event_test_1");
                assert_eq!(dp.amount, 3500);
                assert_eq!(dp.status, DisputeStatus::NeedsResponse);
                assert!(!dp.is_evidence_locked());
            }
            other => panic!("Expected Dispute variant, got {other:?}"),
        }
    }

    #[test]
    fn test_event_object_refund() {
        let json = serde_json::json!({
            "id": "re_evt_refund_101",
            "object": "refund",
            "amount": 3500,
            "currency": "usd",
            "status": "succeeded",
            "charge": "ch_parent_123"
        });

        let data = EventData { object: json };
        let obj = data
            .deserialize_object()
            .expect("Failed to deserialize refund event");

        match obj {
            EventObject::Refund(r) => {
                assert_eq!(r.id, "re_evt_refund_101");
                assert_eq!(r.amount, 3500);
                assert_eq!(r.currency, "usd");
                assert_eq!(r.status, "succeeded");
                assert_eq!(r.charge.as_deref(), Some("ch_parent_123"));
            }
            other => panic!("Expected Refund variant, got {other:?}"),
        }
    }

    #[test]
    fn test_payment_intent_methods_and_redirect_url() {
        let json = serde_json::json!({
            "id": "pi_test_3ds_challenge",
            "object": "payment_intent",
            "amount": 10000,
            "currency": "eur",
            "status": "requires_action",
            "client_secret": "pi_secret_top_secret_code",
            "next_action": {
                "type": "redirect_to_url",
                "redirect_to_url": {
                    "url": "https://hooks.stripe.com/redirect/authenticate/src_test"
                }
            }
        });

        let pi: PaymentIntent = serde_json::from_value(json).unwrap();
        assert!(pi.requires_action());
        assert!(!pi.is_succeeded());
        assert!(!pi.requires_capture());
        assert!(!pi.is_canceled());
        assert_eq!(
            pi.redirect_url(),
            Some("https://hooks.stripe.com/redirect/authenticate/src_test")
        );

        let debug_str = format!("{pi:?}");
        assert!(debug_str.contains("[REDACTED]"));
        assert!(!debug_str.contains("top_secret_code"));
    }

    #[test]
    fn test_create_and_confirm_payment_intent_params() {
        let create_params = CreatePaymentIntentParams {
            amount: 5000,
            currency: "usd".into(),
            capture_method: Some("manual".into()),
            description: Some("Order #1024".into()),
            payment_method: Some("pm_test_card".into()),
            customer: None,
            receipt_email: None,
            confirm: Some(true),
            return_url: Some("https://example.com/complete".into()),
            metadata: None,
            idempotency_key: Some("idem_create_123".into()),
        };
        assert_eq!(create_params.amount, 5000);
        assert_eq!(create_params.currency, "usd");
        assert_eq!(create_params.confirm, Some(true));

        let confirm_params = ConfirmPaymentIntentParams {
            client_secret: None,
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

    #[test]
    fn test_event_object_payment_link() {
        let json = serde_json::json!({
            "id": "plink_evt_test_101",
            "object": "payment_link",
            "active": true,
            "url": "https://buy.stripe.com/test_evt_101",
            "currency": "usd",
            "livemode": false
        });

        let data = EventData { object: json };
        let obj = data
            .deserialize_object()
            .expect("Failed to deserialize payment_link event");

        match obj {
            EventObject::PaymentLink(link) => {
                assert_eq!(link.id, "plink_evt_test_101");
                assert_eq!(link.object, "payment_link");
                assert!(link.is_active());
                assert_eq!(link.url, "https://buy.stripe.com/test_evt_101");
                assert_eq!(link.currency.as_deref(), Some("usd"));
                assert_eq!(link.livemode, Some(false));
            }
            other => panic!("Expected PaymentLink variant, got {other:?}"),
        }
    }

    #[test]
    fn test_event_object_credit_note() {
        let json = serde_json::json!({
            "id": "cn_evt_test_202",
            "object": "credit_note",
            "amount": 1500,
            "currency": "usd",
            "customer": "cus_test_123",
            "invoice": "in_test_456",
            "status": "issued"
        });

        let data = EventData { object: json };
        let obj = data
            .deserialize_object()
            .expect("Failed to deserialize credit_note event");

        match obj {
            EventObject::CreditNote(cn) => {
                assert_eq!(cn.id, "cn_evt_test_202");
                assert_eq!(cn.object, "credit_note");
                assert_eq!(cn.amount, 1500);
                assert_eq!(cn.currency, "usd");
                assert_eq!(cn.customer, "cus_test_123");
                assert_eq!(cn.invoice, "in_test_456");
                assert!(cn.is_issued());
                assert!(!cn.is_void());
            }
            other => panic!("Expected CreditNote variant, got {other:?}"),
        }
    }

    #[test]
    fn test_event_object_quote() {
        let json = serde_json::json!({
            "id": "qt_evt_test_303",
            "object": "quote",
            "amount_subtotal": 5000,
            "amount_total": 5000,
            "currency": "usd",
            "customer": "cus_quote_cust",
            "expires_at": 1735689600,
            "status": "open"
        });

        let data = EventData { object: json };
        let obj = data
            .deserialize_object()
            .expect("Failed to deserialize quote event");

        match obj {
            EventObject::Quote(qt) => {
                assert_eq!(qt.id, "qt_evt_test_303");
                assert_eq!(qt.object, "quote");
                assert_eq!(qt.amount_total, 5000);
                assert_eq!(qt.currency, "usd");
                assert_eq!(qt.customer.as_deref(), Some("cus_quote_cust"));
                assert!(qt.is_open());
                assert!(!qt.is_accepted());
            }
            other => panic!("Expected Quote variant, got {other:?}"),
        }
    }

    #[test]
    fn test_event_object_portal_session() {
        let json = serde_json::json!({
            "id": "bps_evt_test_404",
            "object": "billing_portal.session",
            "configuration": "bpc_test_conf",
            "created": 1735689600,
            "customer": "cus_portal_cust",
            "url": "https://billing.stripe.com/p/session/bps_evt_test_404"
        });

        let data = EventData { object: json };
        let obj = data
            .deserialize_object()
            .expect("Failed to deserialize billing_portal.session event");

        match obj {
            EventObject::PortalSession(bps) => {
                assert_eq!(bps.id, "bps_evt_test_404");
                assert_eq!(bps.object, "billing_portal.session");
                assert_eq!(bps.customer, "cus_portal_cust");
                assert_eq!(bps.configuration.as_deref(), Some("bpc_test_conf"));
                assert_eq!(
                    bps.url,
                    "https://billing.stripe.com/p/session/bps_evt_test_404"
                );
            }
            other => panic!("Expected PortalSession variant, got {other:?}"),
        }
    }

    #[test]
    fn test_event_object_portal_configuration() {
        let json = serde_json::json!({
            "id": "bpc_evt_test_505",
            "object": "billing_portal.configuration",
            "active": true,
            "created": 1735689600,
            "default_return_url": "https://example.com/return"
        });

        let data = EventData { object: json };
        let obj = data
            .deserialize_object()
            .expect("Failed to deserialize billing_portal.configuration event");

        match obj {
            EventObject::PortalConfiguration(bpc) => {
                assert_eq!(bpc.id, "bpc_evt_test_505");
                assert_eq!(bpc.object, "billing_portal.configuration");
                assert!(bpc.active);
                assert_eq!(
                    bpc.default_return_url.as_deref(),
                    Some("https://example.com/return")
                );
            }
            other => panic!("Expected PortalConfiguration variant, got {other:?}"),
        }
    }

    #[test]
    fn test_event_object_tax_transaction() {
        let json = serde_json::json!({
            "id": "tax_tx_evt_606",
            "object": "tax.transaction",
            "created": 1735689600,
            "currency": "usd",
            "livemode": false,
            "posted_at": 1735689600,
            "reference": "order_evt_606",
            "tax_date": 1735689600,
            "type": "transaction"
        });

        let data = EventData { object: json };
        let obj = data
            .deserialize_object()
            .expect("Failed to deserialize tax.transaction event");

        match obj {
            EventObject::TaxTransaction(tx) => {
                assert_eq!(tx.id, "tax_tx_evt_606");
                assert_eq!(tx.object, "tax.transaction");
                assert_eq!(tx.currency, "usd");
                assert_eq!(tx.reference, "order_evt_606");
                assert!(tx.is_standard_transaction());
            }
            other => panic!("Expected TaxTransaction variant, got {other:?}"),
        }
    }

    #[test]
    fn test_event_object_tax_calculation() {
        let json = serde_json::json!({
            "id": "tax_calc_evt_707",
            "object": "tax.calculation",
            "amount_total": 10500,
            "currency": "usd",
            "customer": null,
            "customer_details": {
                "address": {
                    "country": "US",
                    "postal_code": "94103",
                    "state": "CA"
                },
                "address_source": "shipping"
            },
            "livemode": false,
            "tax_amount_exclusive": 500,
            "tax_amount_inclusive": 0,
            "tax_date": 1735689600
        });

        let data = EventData { object: json };
        let obj = data
            .deserialize_object()
            .expect("Failed to deserialize tax.calculation event");

        match obj {
            EventObject::TaxCalculation(calc) => {
                assert_eq!(calc.id, "tax_calc_evt_707");
                assert_eq!(calc.object, "tax.calculation");
                assert_eq!(calc.amount_total, 10500);
            }
            other => panic!("Expected TaxCalculation variant, got {other:?}"),
        }
    }

    #[test]
    fn test_event_object_verification_report() {
        let json = serde_json::json!({
            "id": "vr_test_808",
            "object": "identity.verification_report",
            "created": 1735689600,
            "livemode": false,
            "type": "document",
            "status": "verified"
        });

        let data = EventData { object: json };
        let obj = data
            .deserialize_object()
            .expect("Failed to deserialize identity.verification_report event");

        match obj {
            EventObject::VerificationReport(report) => {
                assert_eq!(report.id, "vr_test_808");
                assert_eq!(report.object, "identity.verification_report");
                assert_eq!(report.report_type, VerificationReportType::Document);
                assert_eq!(report.status, VerificationReportStatus::Verified);
            }
            other => panic!("Expected VerificationReport variant, got {other:?}"),
        }
    }

    #[test]
    fn test_event_object_treasury_financial_account() {
        let json = serde_json::json!({
            "id": "fa_test_999",
            "object": "treasury.financial_account",
            "active_features": ["deposit_insurance", "financial_addresses.aba"],
            "balance": {
                "cash": {"usd": 5000000},
                "inbound_pending": {"usd": 100000},
                "outbound_pending": {"usd": 50000}
            },
            "country": "US",
            "created": 1728345600,
            "financial_addresses": [],
            "livemode": false,
            "pending_features": [],
            "restricted_features": [],
            "status": "open",
            "supported_currencies": ["usd"]
        });

        let data = EventData { object: json };
        let obj = data
            .deserialize_object()
            .expect("Failed to deserialize treasury.financial_account event");

        match obj {
            EventObject::TreasuryFinancialAccount(fa) => {
                assert_eq!(fa.id, "fa_test_999");
                assert_eq!(fa.object, "treasury.financial_account");
                assert!(fa.is_open());
                assert_eq!(fa.balance.cash_amount("usd"), 5000000);
            }
            other => panic!("Expected TreasuryFinancialAccount variant, got {other:?}"),
        }
    }

    #[test]
    fn test_event_object_terminal_reader_and_token() {
        let reader_json = serde_json::json!({
            "id": "tmr_test_777",
            "object": "terminal.reader",
            "device_type": "bbpos_wisepos_e",
            "livemode": false,
            "serial_number": "WPOS-777",
            "status": "online"
        });

        let data = EventData {
            object: reader_json,
        };
        let obj = data
            .deserialize_object()
            .expect("Failed to deserialize terminal.reader event");

        match obj {
            EventObject::TerminalReader(r) => {
                assert_eq!(r.id(), "tmr_test_777");
                assert!(r.is_online());
                assert!(r.is_emv_certified());
            }
            other => panic!("Expected TerminalReader variant, got {other:?}"),
        }

        let token_json = serde_json::json!({
            "object": "terminal.connection_token",
            "secret": "pst_test_secret_999",
            "location": "loc_test_123"
        });

        let token_data = EventData { object: token_json };
        let token_obj = token_data
            .deserialize_object()
            .expect("Failed to deserialize terminal.connection_token event");

        match token_obj {
            EventObject::TerminalConnectionToken(tok) => {
                assert_eq!(tok.secret(), "pst_test_secret_999");
                assert!(tok.is_bound_to_location("loc_test_123"));
            }
            other => panic!("Expected TerminalConnectionToken variant, got {other:?}"),
        }
    }
}
