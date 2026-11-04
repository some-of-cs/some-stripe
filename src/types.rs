//! Strongly typed models for the Stripe API, modeled after stripe-node.

use serde::{Deserialize, Serialize};

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

/// Represents a Stripe PaymentIntent object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIntent {
    /// Unique identifier for the payment intent (`pi_...`).
    pub id: String,

    /// Object type, always `payment_intent`.
    #[serde(default = "default_payment_intent_object")]
    pub object: String,

    /// Amount intended to be collected in minor currency units.
    pub amount: i64,

    /// Amount that was captured in minor units.
    pub amount_received: Option<i64>,

    /// Three-letter ISO currency code in lowercase.
    pub currency: String,

    /// Status of this PaymentIntent: `requires_payment_method`, `requires_confirmation`,
    /// `requires_action`, `processing`, `requires_capture`, `canceled`, or `succeeded`.
    pub status: String,

    /// Controls when the funds will be captured from the customer's account: `automatic` or `manual`.
    pub capture_method: Option<String>,

    /// The client secret of this PaymentIntent used for client-side confirmation.
    pub client_secret: Option<String>,

    /// Time at which the object was created (UNIX epoch seconds).
    pub created: Option<i64>,

    /// Reason for cancellation, if canceled.
    pub cancellation_reason: Option<String>,
}

fn default_payment_intent_object() -> String {
    "payment_intent".into()
}

/// Parameters for creating a Stripe PaymentIntent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePaymentIntentParams {
    /// Amount intended to be collected in minor currency units.
    pub amount: i64,

    /// Three-letter ISO currency code.
    pub currency: String,

    /// Capture method: `automatic` (default) or `manual`.
    pub capture_method: Option<String>,

    /// Description of the payment.
    pub description: Option<String>,

    /// Unique idempotency key.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

/// Parameters for capturing a PaymentIntent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapturePaymentIntentParams {
    /// The amount to capture from the authorization. If omitted, captures full amount.
    pub amount_to_capture: Option<i64>,

    /// Unique idempotency key.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

/// Parameters for canceling a PaymentIntent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CancelPaymentIntentParams {
    /// Reason for canceling this PaymentIntent.
    pub cancellation_reason: Option<String>,

    /// Unique idempotency key.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

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

fn default_event_object() -> String {
    "event".into()
}

/// Container for the data object payload of an Event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    /// Raw JSON object containing the payload (e.g. CheckoutSession or PaymentIntent).
    pub object: serde_json::Value,
}
