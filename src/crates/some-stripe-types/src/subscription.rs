//! Subscription domain entities, status state machines, and parameter schemas for Stripe.
//!
//! Enforces ISO 4217 minor currency integer rules (Gate G-Law) and Rust zero-clone
//! borrowing discipline (Gate G-Borrow).
//!
//! # Architecture & Quality Gates:
//! - **G0 (Rust 2024 Edition)**: Clean compilation with 0 clippy warnings and strict code formatting.
//! - **G-Borrow (Zero-Clone Discipline)**: Zero-copy accessors (`sub.id()`, `sub.customer()`),
//!   borrowed request parameters (`&CreateSubscriptionParams`), and cursor-based pagination.
//! - **G2 (Security & Idempotency)**: Redaction of sensitive fields and automatic idempotency key handling.
//! - **Gate G-Law (ISO 4217 & Minor Currency Units)**: Minor integer amounts for all unit prices.
//!
//! # Examples
//! ```
//! use some_stripe_types::subscription::{
//!     CreateSubscriptionParams, Subscription, SubscriptionItemParams, SubscriptionStatus,
//! };
//!
//! let params = CreateSubscriptionParams::new(
//!     "cus_12345",
//!     vec![SubscriptionItemParams::new("price_pro_tier", 1)],
//! )
//! .trial_period_days(14);
//!
//! assert!(params.validate().is_ok());
//! assert_eq!(params.customer, "cus_12345");
//! assert_eq!(params.trial_period_days, Some(14));
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the status lifecycle of a Stripe Subscription.
///
/// Invariants:
/// - Transitions: `incomplete` -> `incomplete_expired`, or `incomplete` -> `active` / `trialing`.
/// - `trialing`: Within the active trial period.
/// - `past_due`: Renewal payment failed, within dunning retry period.
/// - `canceled`: Subscription permanently terminated (terminal state).
/// - `unpaid`: Dunning retries exhausted, invoices remain unpaid.
/// - `paused`: Payment collection paused via `pause_collection`.
///
/// # Examples
/// ```
/// use some_stripe_types::subscription::SubscriptionStatus;
///
/// let status = SubscriptionStatus::Trialing;
/// assert!(status.is_operational());
/// assert!(status.can_transition_to(SubscriptionStatus::Active));
/// assert!(status.can_transition_to(SubscriptionStatus::Canceled));
/// assert_eq!(status.as_str(), "trialing");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    /// Incomplete payment setup; initial payment required.
    Incomplete,
    /// Incomplete payment setup expired before confirmation.
    IncompleteExpired,
    /// Subscription is currently in a free trial.
    Trialing,
    /// Subscription is active with valid payment method.
    #[default]
    Active,
    /// Most recent invoice payment failed; awaiting dunning retries.
    PastDue,
    /// Subscription has been permanently terminated.
    Canceled,
    /// Dunning retries exhausted without payment.
    Unpaid,
    /// Payment collection is currently paused.
    Paused,
}

impl SubscriptionStatus {
    /// Returns the string slice representation of the status.
    ///
    /// # Examples
    /// ```
    /// use some_stripe_types::subscription::SubscriptionStatus;
    ///
    /// assert_eq!(SubscriptionStatus::Active.as_str(), "active");
    /// assert_eq!(SubscriptionStatus::Paused.as_str(), "paused");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Incomplete => "incomplete",
            Self::IncompleteExpired => "incomplete_expired",
            Self::Trialing => "trialing",
            Self::Active => "active",
            Self::PastDue => "past_due",
            Self::Canceled => "canceled",
            Self::Unpaid => "unpaid",
            Self::Paused => "paused",
        }
    }

    /// Returns `true` if the subscription is currently active or in trial.
    ///
    /// # Examples
    /// ```
    /// use some_stripe_types::subscription::SubscriptionStatus;
    ///
    /// assert!(SubscriptionStatus::Active.is_operational());
    /// assert!(SubscriptionStatus::Trialing.is_operational());
    /// assert!(!SubscriptionStatus::Canceled.is_operational());
    /// ```
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Active | Self::Trialing)
    }

    /// Returns `true` if the subscription is in a delinquent, unpaid, or canceled state.
    ///
    /// # Examples
    /// ```
    /// use some_stripe_types::subscription::SubscriptionStatus;
    ///
    /// assert!(SubscriptionStatus::PastDue.is_delinquent_or_canceled());
    /// assert!(SubscriptionStatus::Canceled.is_delinquent_or_canceled());
    /// assert!(!SubscriptionStatus::Active.is_delinquent_or_canceled());
    /// ```
    pub fn is_delinquent_or_canceled(&self) -> bool {
        matches!(self, Self::PastDue | Self::Canceled | Self::Unpaid)
    }

    /// Returns `true` if this status is terminal (cannot transition to any other status).
    ///
    /// # Examples
    /// ```
    /// use some_stripe_types::subscription::SubscriptionStatus;
    ///
    /// assert!(SubscriptionStatus::Canceled.is_terminal());
    /// assert!(SubscriptionStatus::IncompleteExpired.is_terminal());
    /// assert!(!SubscriptionStatus::Active.is_terminal());
    /// ```
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Canceled | Self::IncompleteExpired)
    }

    /// Validates whether a transition from `self` to `next` is allowed in the Stripe state machine.
    ///
    /// # Examples
    /// ```
    /// use some_stripe_types::subscription::SubscriptionStatus;
    ///
    /// assert!(SubscriptionStatus::Active.can_transition_to(SubscriptionStatus::PastDue));
    /// assert!(SubscriptionStatus::Active.can_transition_to(SubscriptionStatus::Paused));
    /// assert!(SubscriptionStatus::Active.can_transition_to(SubscriptionStatus::Canceled));
    /// assert!(!SubscriptionStatus::Canceled.can_transition_to(SubscriptionStatus::Active));
    /// ```
    pub fn can_transition_to(&self, next: SubscriptionStatus) -> bool {
        if *self == next {
            return true;
        }
        if self.is_terminal() {
            return false;
        }
        match self {
            Self::Incomplete => matches!(
                next,
                Self::Active | Self::Trialing | Self::IncompleteExpired | Self::Canceled
            ),
            Self::Trialing => matches!(
                next,
                Self::Active | Self::PastDue | Self::Paused | Self::Canceled
            ),
            Self::Active => matches!(
                next,
                Self::PastDue | Self::Paused | Self::Canceled | Self::Unpaid
            ),
            Self::PastDue => matches!(
                next,
                Self::Active | Self::Unpaid | Self::Paused | Self::Canceled
            ),
            Self::Paused => matches!(next, Self::Active | Self::PastDue | Self::Canceled),
            Self::Unpaid => matches!(next, Self::Active | Self::Canceled),
            Self::Canceled | Self::IncompleteExpired => false,
        }
    }

    /// Validates whether a transition to `next` is permitted, returning an error if illegal.
    ///
    /// # Errors
    /// Returns [`SubscriptionStateError`] if the transition is prohibited by Stripe lifecycle invariants.
    ///
    /// # Examples
    /// ```
    /// use some_stripe_types::subscription::{SubscriptionStatus, SubscriptionStateError};
    ///
    /// assert!(SubscriptionStatus::Active.validate_transition(SubscriptionStatus::Paused).is_ok());
    /// assert!(matches!(
    ///     SubscriptionStatus::Canceled.validate_transition(SubscriptionStatus::Active),
    ///     Err(SubscriptionStateError::TerminalState(_))
    /// ));
    /// ```
    pub fn validate_transition(
        &self,
        next: SubscriptionStatus,
    ) -> Result<(), SubscriptionStateError> {
        if *self == next {
            return Ok(());
        }
        if self.is_terminal() {
            return Err(SubscriptionStateError::TerminalState(*self));
        }
        if self.can_transition_to(next) {
            Ok(())
        } else {
            Err(SubscriptionStateError::InvalidTransition {
                from: *self,
                to: next,
                reason: "Illegal state machine transition according to Stripe subscription lifecycle rules",
            })
        }
    }
}

impl std::fmt::Display for SubscriptionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error returned when an illegal subscription state transition is attempted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionStateError {
    /// Attempted an invalid transition between two statuses.
    InvalidTransition {
        /// The starting status.
        from: SubscriptionStatus,
        /// The target status attempted.
        to: SubscriptionStatus,
        /// Reason explaining the failure.
        reason: &'static str,
    },
    /// The current status is terminal and cannot transition to any new status.
    TerminalState(SubscriptionStatus),
}

impl std::fmt::Display for SubscriptionStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to, reason } => {
                write!(
                    f,
                    "Invalid subscription state transition from '{from}' to '{to}': {reason}"
                )
            }
            Self::TerminalState(status) => {
                write!(
                    f,
                    "Cannot transition from terminal subscription state '{status}'"
                )
            }
        }
    }
}

impl std::error::Error for SubscriptionStateError {}

/// Configuration settings for payment collection on a subscription.
///
/// # Examples
/// ```
/// use some_stripe_types::subscription::PaymentSettings;
///
/// let settings = PaymentSettings::new()
///     .payment_method_types(vec!["card".into(), "us_bank_account".into()])
///     .save_default_payment_method("on_subscription");
///
/// assert_eq!(settings.payment_method_types.as_ref().unwrap().len(), 2);
/// assert_eq!(settings.save_default_payment_method.as_deref(), Some("on_subscription"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PaymentSettings {
    /// Payment-method-specific configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_method_options: Option<SubscriptionPaymentMethodOptions>,

    /// The list of payment method types to provide to every invoice created by the subscription.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_method_types: Option<Vec<String>>,

    /// Configure whether Stripe updates `subscription.default_payment_method` when a new
    /// payment method is used. Values: `off`, `on_subscription`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save_default_payment_method: Option<String>,
}

impl PaymentSettings {
    /// Creates a new default [`PaymentSettings`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the list of permitted payment method types (e.g. `card`, `us_bank_account`).
    pub fn payment_method_types(mut self, types: Vec<String>) -> Self {
        self.payment_method_types = Some(types);
        self
    }

    /// Configures default payment method saving behavior.
    pub fn save_default_payment_method(mut self, behavior: impl Into<String>) -> Self {
        self.save_default_payment_method = Some(behavior.into());
        self
    }

    /// Sets payment method options.
    pub fn payment_method_options(mut self, options: SubscriptionPaymentMethodOptions) -> Self {
        self.payment_method_options = Some(options);
        self
    }
}

/// Payment-method-specific options for a subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SubscriptionPaymentMethodOptions {
    /// Specific settings for cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<SubscriptionCardOptions>,
}

/// Card options on a subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SubscriptionCardOptions {
    /// We strongly recommend that you rely on our SCA Engine to automatically prompt your
    /// customers for 3DS. Values: `any`, `automatic`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_three_d_secure: Option<String>,
}

/// Represents a Stripe Subscription object.
///
/// # Examples
///
/// ```
/// use some_stripe_types::subscription::{Subscription, SubscriptionStatus};
///
/// let json = r#"{
///     "id": "sub_12345",
///     "object": "subscription",
///     "customer": "cus_12345",
///     "status": "active",
///     "current_period_start": 1728345600,
///     "current_period_end": 1730937600,
///     "cancel_at_period_end": false,
///     "created": 1728345600,
///     "items": {
///         "object": "list",
///         "data": [],
///         "has_more": false,
///         "total_count": 0,
///         "url": "/v1/subscription_items"
///     }
/// }"#;
/// let sub: Subscription = serde_json::from_str(json).unwrap();
/// assert_eq!(sub.id(), "sub_12345");
/// assert_eq!(sub.status(), SubscriptionStatus::Active);
/// assert!(sub.is_operational());
/// assert!(sub.is_active());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    /// Unique identifier for the subscription (`sub_...`).
    pub id: String,

    /// Object type, always `subscription`.
    #[serde(default = "default_subscription_object")]
    pub object: String,

    /// ID of the customer who owns the subscription.
    pub customer: String,

    /// Current status of the subscription.
    pub status: SubscriptionStatus,

    /// Start of the current billing period (Unix timestamp).
    pub current_period_start: i64,

    /// End of the current billing period (Unix timestamp).
    pub current_period_end: i64,

    /// If the subscription has been canceled, the date of cancellation (Unix timestamp).
    pub canceled_at: Option<i64>,

    /// If `true`, the subscription will be canceled at the end of the current period.
    #[serde(default)]
    pub cancel_at_period_end: bool,

    /// Time at which the subscription was created (Unix timestamp).
    pub created: i64,

    /// Three-letter ISO currency code, in lowercase.
    pub currency: Option<String>,

    /// If specified, details of the trial period.
    pub trial_start: Option<i64>,

    /// End of the trial period (Unix timestamp).
    pub trial_end: Option<i64>,

    /// Collection pause details if the subscription is paused.
    pub pause_collection: Option<PauseCollection>,

    /// List of subscription items on this subscription.
    pub items: SubscriptionItemList,

    /// Key-value metadata attached to the subscription.
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    /// ID of the default payment method attached to this subscription.
    pub default_payment_method: Option<String>,

    /// The latest invoice associated with this subscription.
    pub latest_invoice: Option<String>,

    /// The schedule that manages this subscription, if any.
    pub schedule: Option<String>,

    /// Payment settings applied to recurring invoices on this subscription.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_settings: Option<PaymentSettings>,

    /// Description of the subscription.
    pub description: Option<String>,
}

impl Subscription {
    /// Returns a zero-copy string slice to the subscription ID (`sub_...`).
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns a zero-copy string slice to the customer ID (`cus_...`).
    pub fn customer(&self) -> &str {
        &self.customer
    }

    /// Returns the current lifecycle status of the subscription.
    pub fn status(&self) -> SubscriptionStatus {
        self.status
    }

    /// Returns `true` if the subscription is operational (`active` or `trialing`).
    pub fn is_operational(&self) -> bool {
        self.status.is_operational()
    }

    /// Returns `true` if the subscription is active.
    pub fn is_active(&self) -> bool {
        self.status == SubscriptionStatus::Active
    }

    /// Returns `true` if the subscription is in free trial.
    pub fn is_trialing(&self) -> bool {
        self.status == SubscriptionStatus::Trialing
    }

    /// Returns `true` if the subscription is permanently canceled.
    pub fn is_canceled(&self) -> bool {
        self.status == SubscriptionStatus::Canceled
    }

    /// Returns `true` if the subscription payment collection is paused.
    pub fn is_paused(&self) -> bool {
        self.status == SubscriptionStatus::Paused
    }

    /// Returns a slice of the subscription items attached to this subscription.
    pub fn items(&self) -> &[SubscriptionItem] {
        &self.items.data
    }

    /// Transition this subscription to a new status, checking state machine validity.
    ///
    /// # Errors
    /// Returns [`SubscriptionStateError`] if the transition is illegal or the current status is terminal.
    ///
    /// # Examples
    /// ```
    /// use some_stripe_types::subscription::{Subscription, SubscriptionStatus};
    ///
    /// let json = r#"{
    ///     "id": "sub_active_123",
    ///     "object": "subscription",
    ///     "customer": "cus_123",
    ///     "status": "active",
    ///     "current_period_start": 1728345600,
    ///     "current_period_end": 1730937600,
    ///     "cancel_at_period_end": false,
    ///     "created": 1728345600,
    ///     "items": { "object": "list", "data": [], "has_more": false, "url": "/v1/subscription_items" }
    /// }"#;
    /// let mut sub: Subscription = serde_json::from_str(json).unwrap();
    /// assert!(sub.transition_to(SubscriptionStatus::Paused).is_ok());
    /// assert_eq!(sub.status(), SubscriptionStatus::Paused);
    /// ```
    pub fn transition_to(
        &mut self,
        next: SubscriptionStatus,
    ) -> Result<(), SubscriptionStateError> {
        self.status.validate_transition(next)?;
        self.status = next;
        Ok(())
    }
}

/// Extracts the ID string slice from a [`Subscription`] for zero-clone paginators.
///
/// # Examples
/// ```
/// use some_stripe_types::subscription::{subscription_id, Subscription};
///
/// let json = r#"{
///     "id": "sub_cursor_001",
///     "object": "subscription",
///     "customer": "cus_123",
///     "status": "active",
///     "current_period_start": 1728345600,
///     "current_period_end": 1730937600,
///     "cancel_at_period_end": false,
///     "created": 1728345600,
///     "items": { "object": "list", "data": [], "has_more": false, "url": "/v1/subscription_items" }
/// }"#;
/// let sub: Subscription = serde_json::from_str(json).unwrap();
/// assert_eq!(subscription_id(&sub), "sub_cursor_001");
/// ```
pub fn subscription_id(sub: &Subscription) -> &str {
    &sub.id
}

fn default_subscription_object() -> String {
    "subscription".into()
}

/// Collection pause details on a subscription.
///
/// # Examples
/// ```
/// use some_stripe_types::subscription::PauseCollection;
///
/// let pause = PauseCollection {
///     behavior: "void".into(),
///     resumes_at: Some(1735689600),
/// };
/// assert_eq!(pause.behavior, "void");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PauseCollection {
    /// The payment collection behavior while paused.
    /// Values: `keep_as_draft`, `mark_uncollectible`, `void`.
    pub behavior: String,

    /// The time at which payment collection will resume.
    pub resumes_at: Option<i64>,
}

/// Paginated list of Subscription items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionItemList {
    /// Object type, always `list`.
    #[serde(default = "default_list_object")]
    pub object: String,

    /// The subscription items.
    pub data: Vec<SubscriptionItem>,

    /// Whether there are more items available.
    #[serde(default)]
    pub has_more: bool,

    /// Total count of items, if returned by API.
    pub total_count: Option<u64>,

    /// URL for fetching the next page.
    pub url: String,
}

fn default_list_object() -> String {
    "list".into()
}

/// Represents an individual item within a Subscription.
///
/// # Examples
/// ```
/// use some_stripe_types::subscription::SubscriptionItem;
///
/// let json = r#"{
///     "id": "si_12345",
///     "object": "subscription_item",
///     "subscription": "sub_12345",
///     "quantity": 3,
///     "created": 1728345600
/// }"#;
/// let item: SubscriptionItem = serde_json::from_str(json).unwrap();
/// assert_eq!(item.id, "si_12345");
/// assert_eq!(item.quantity, Some(3));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionItem {
    /// Unique identifier for the subscription item (`si_...`).
    pub id: String,

    /// Object type, always `subscription_item`.
    #[serde(default = "default_subscription_item_object")]
    pub object: String,

    /// The ID of the subscription this item belongs to.
    pub subscription: String,

    /// The price ID associated with this item.
    pub price: Option<SubscriptionPrice>,

    /// Quantity of the plan to which the customer is subscribed.
    pub quantity: Option<u64>,

    /// Time at which the item was created (Unix timestamp).
    pub created: i64,

    /// Key-value metadata attached to the item.
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    /// Define thresholds at which an invoice will be sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing_thresholds: Option<SubscriptionItemBillingThresholds>,

    /// Proration behavior applied to this item change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proration_behavior: Option<ProrationBehavior>,

    /// A list of tax rates applied to this item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_rates: Option<Vec<String>>,
}

impl SubscriptionItem {
    /// Zero-copy access to the subscription item ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Zero-copy access to the parent subscription ID.
    pub fn subscription(&self) -> &str {
        &self.subscription
    }

    /// Quantity of units subscribed.
    pub fn quantity(&self) -> Option<u64> {
        self.quantity
    }
}

/// Extracts the ID from a [`SubscriptionItem`] for pagination cursors.
pub fn subscription_item_id(item: &SubscriptionItem) -> &str {
    &item.id
}

fn default_subscription_item_object() -> String {
    "subscription_item".into()
}

/// Define thresholds at which an invoice will be sent, and the subscription in-flight usage reset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SubscriptionItemBillingThresholds {
    /// Number of units that meets the threshold to send an invoice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_gte: Option<i64>,
}

impl SubscriptionItemBillingThresholds {
    /// Creates a new `SubscriptionItemBillingThresholds` with `usage_gte`.
    pub fn new(usage_gte: i64) -> Self {
        Self {
            usage_gte: Some(usage_gte),
        }
    }
}

/// Represents a deleted subscription item response from the Stripe API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletedSubscriptionItem {
    /// Unique identifier of the deleted subscription item.
    pub id: String,

    /// Object type, always `subscription_item`.
    #[serde(default = "default_subscription_item_object")]
    pub object: String,

    /// Whether the subscription item was deleted.
    pub deleted: bool,
}

impl DeletedSubscriptionItem {
    /// Returns `true` if the item was successfully deleted.
    pub fn is_deleted(&self) -> bool {
        self.deleted
    }

    /// Zero-copy ID accessor.
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Represents a Stripe Usage Record for metered subscription items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageRecord {
    /// Unique identifier for the usage record (`mbur_...` or `ur_...`).
    pub id: String,

    /// Object type, always `usage_record`.
    #[serde(default = "default_usage_record_object")]
    pub object: String,

    /// Has the value `true` if the object exists in live mode or `false` in test mode.
    #[serde(default)]
    pub livemode: bool,

    /// The quantity of units used.
    pub quantity: i64,

    /// The ID of the subscription item this usage record belongs to.
    pub subscription_item: String,

    /// The timestamp when this usage occurred (Unix timestamp).
    pub timestamp: i64,

    /// The action for this usage record (`increment` or `set`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

fn default_usage_record_object() -> String {
    "usage_record".into()
}

impl UsageRecord {
    /// Zero-copy access to the usage record ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Zero-copy access to the subscription item ID.
    pub fn subscription_item(&self) -> &str {
        &self.subscription_item
    }

    /// The quantity of units recorded.
    pub fn quantity(&self) -> i64 {
        self.quantity
    }
}

/// Action to take when creating a usage record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UsageRecordAction {
    /// Increments the existing usage by the specified quantity.
    #[default]
    Increment,
    /// Overwrites the usage with the specified quantity.
    Set,
}

/// Parameters for creating a usage record on a metered subscription item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CreateUsageRecordParams {
    /// The quantity of usage to record.
    pub quantity: i64,

    /// The timestamp for the usage record. Can be a Unix timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,

    /// How to handle the usage record: `increment` (default) or `set`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<UsageRecordAction>,

    /// Optional idempotency key for this request.
    #[serde(skip_serializing)]
    pub idempotency_key: Option<String>,
}

impl CreateUsageRecordParams {
    /// Creates a new `CreateUsageRecordParams` with the specified quantity.
    pub fn new(quantity: i64) -> Self {
        Self {
            quantity,
            timestamp: None,
            action: None,
            idempotency_key: None,
        }
    }

    /// Sets the timestamp.
    pub fn timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets the action (`increment` or `set`).
    pub fn action(mut self, action: UsageRecordAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Sets the idempotency key.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for creating a subscription item directly via the API (`POST /v1/subscription_items`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CreateSubscriptionItemParams {
    /// The ID of the subscription to add the item to.
    pub subscription: String,

    /// The ID of the price to subscribe to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,

    /// The quantity of the subscription item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<u64>,

    /// Key-value metadata attached to the item.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,

    /// Proration behavior to apply.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proration_behavior: Option<ProrationBehavior>,

    /// Define thresholds at which an invoice will be sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_thresholds: Option<SubscriptionItemBillingThresholds>,

    /// A list of tax rates applied to this item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_rates: Option<Vec<String>>,

    /// Optional idempotency key for this request.
    #[serde(skip_serializing)]
    pub idempotency_key: Option<String>,
}

impl CreateSubscriptionItemParams {
    /// Creates a new `CreateSubscriptionItemParams` for the given subscription and price.
    pub fn new(subscription: impl Into<String>, price: impl Into<String>) -> Self {
        Self {
            subscription: subscription.into(),
            price: Some(price.into()),
            ..Default::default()
        }
    }

    /// Sets the quantity.
    pub fn quantity(mut self, quantity: u64) -> Self {
        self.quantity = Some(quantity);
        self
    }

    /// Sets the proration behavior.
    pub fn proration_behavior(mut self, behavior: ProrationBehavior) -> Self {
        self.proration_behavior = Some(behavior);
        self
    }

    /// Sets billing thresholds.
    pub fn billing_thresholds(mut self, thresholds: SubscriptionItemBillingThresholds) -> Self {
        self.billing_thresholds = Some(thresholds);
        self
    }

    /// Sets metadata.
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Sets an idempotency key.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for updating an existing subscription item (`POST /v1/subscription_items/{id}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UpdateSubscriptionItemParams {
    /// The ID of the new price to subscribe to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,

    /// The new quantity of the subscription item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<u64>,

    /// Key-value metadata attached to the item.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,

    /// Proration behavior to apply.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proration_behavior: Option<ProrationBehavior>,

    /// Define thresholds at which an invoice will be sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_thresholds: Option<SubscriptionItemBillingThresholds>,

    /// A list of tax rates applied to this item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_rates: Option<Vec<String>>,

    /// Indicates whether the update is off-session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub off_session: Option<bool>,

    /// Optional idempotency key for this request.
    #[serde(skip_serializing)]
    pub idempotency_key: Option<String>,
}

impl UpdateSubscriptionItemParams {
    /// Creates a new empty `UpdateSubscriptionItemParams`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the updated price ID.
    pub fn price(mut self, price: impl Into<String>) -> Self {
        self.price = Some(price.into());
        self
    }

    /// Sets the updated quantity.
    pub fn quantity(mut self, quantity: u64) -> Self {
        self.quantity = Some(quantity);
        self
    }

    /// Sets the proration behavior.
    pub fn proration_behavior(mut self, behavior: ProrationBehavior) -> Self {
        self.proration_behavior = Some(behavior);
        self
    }

    /// Sets the billing thresholds.
    pub fn billing_thresholds(mut self, thresholds: SubscriptionItemBillingThresholds) -> Self {
        self.billing_thresholds = Some(thresholds);
        self
    }

    /// Sets metadata.
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Sets an idempotency key.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for deleting a subscription item (`DELETE /v1/subscription_items/{id}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DeleteSubscriptionItemParams {
    /// Delete all usage for the given subscription item. Allowed only when the current plan's usage type is metered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clear_usage: Option<bool>,

    /// Determines how to handle prorations when the subscription item is deleted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proration_behavior: Option<ProrationBehavior>,

    /// Optional idempotency key for this request.
    #[serde(skip_serializing)]
    pub idempotency_key: Option<String>,
}

impl DeleteSubscriptionItemParams {
    /// Creates a new empty `DeleteSubscriptionItemParams`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `clear_usage`.
    pub fn clear_usage(mut self, clear: bool) -> Self {
        self.clear_usage = Some(clear);
        self
    }

    /// Sets `proration_behavior`.
    pub fn proration_behavior(mut self, behavior: ProrationBehavior) -> Self {
        self.proration_behavior = Some(behavior);
        self
    }
}

/// Parameters for listing subscription items (`GET /v1/subscription_items`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListSubscriptionItemsParams {
    /// The ID of the subscription whose items are being retrieved.
    pub subscription: String,

    /// A cursor for pagination. An object ID defining your place in the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// A cursor for pagination. An object ID defining your place in the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,

    /// A limit on the number of objects to be returned (between 1 and 100).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl ListSubscriptionItemsParams {
    /// Creates a new `ListSubscriptionItemsParams` for a given subscription.
    pub fn new(subscription: impl Into<String>) -> Self {
        Self {
            subscription: subscription.into(),
            starting_after: None,
            ending_before: None,
            limit: None,
        }
    }

    /// Sets the page limit.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the starting_after cursor.
    pub fn starting_after(mut self, cursor: impl Into<String>) -> Self {
        self.starting_after = Some(cursor.into());
        self
    }

    /// Sets the ending_before cursor.
    pub fn ending_before(mut self, cursor: impl Into<String>) -> Self {
        self.ending_before = Some(cursor.into());
        self
    }
}

/// Simplified price information attached to a subscription item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionPrice {
    /// Unique identifier for the price (`price_...`).
    pub id: String,

    /// Three-letter ISO currency code, in lowercase.
    pub currency: String,

    /// The unit amount in minor currency integer units (e.g., cents).
    pub unit_amount: Option<i64>,

    /// The ID of the product this price is associated with.
    pub product: String,

    /// Whether the price is active.
    #[serde(default)]
    pub active: bool,
}

/// Proration behavior options for subscription modifications.
///
/// # Examples
/// ```
/// use some_stripe_types::subscription::ProrationBehavior;
///
/// assert_eq!(ProrationBehavior::default(), ProrationBehavior::CreateProrations);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProrationBehavior {
    /// Automatically calculates and creates proration invoice items.
    #[default]
    CreateProrations,
    /// Disables prorations; no prorations are calculated.
    None,
    /// Immediately creates and attempts to pay a proration invoice.
    AlwaysInvoice,
}

/// Represents a Stripe Subscription Schedule object for multi-phase lifecycles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionSchedule {
    /// Unique identifier for the schedule (`sub_sched_...`).
    pub id: String,

    /// Object type, always `subscription_schedule`.
    #[serde(default = "default_subscription_schedule_object")]
    pub object: String,

    /// The customer who owns this schedule.
    pub customer: String,

    /// Current status of the schedule (`not_started`, `active`, `completed`, `released`, `canceled`).
    pub status: String,

    /// Phases configured for this schedule.
    #[serde(default)]
    pub phases: Vec<SubscriptionSchedulePhase>,

    /// The subscription currently managed by this schedule, if any.
    pub subscription: Option<String>,

    /// Time at which the schedule was created (Unix timestamp).
    pub created: i64,
}

fn default_subscription_schedule_object() -> String {
    "subscription_schedule".into()
}

/// Individual phase in a multi-phase Subscription Schedule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionSchedulePhase {
    /// Start of the phase (Unix timestamp).
    pub start_date: i64,

    /// End of the phase (Unix timestamp).
    pub end_date: i64,

    /// Number of iterations this phase will run for.
    pub iterations: Option<u64>,

    /// Proration behavior to apply when transitioning to or from this phase.
    pub proration_behavior: Option<ProrationBehavior>,
}

// ============================================================================
// Parameter DTOs & Validation
// ============================================================================

/// Parameters for creating a Stripe Subscription (`POST /v1/subscriptions`).
///
/// Supports multi-item subscriptions, trial periods, payment settings, custom metadata,
/// and automatic idempotency key handling.
///
/// # Examples
/// ```
/// use some_stripe_types::subscription::{
///     CreateSubscriptionParams, SubscriptionItemParams, ProrationBehavior,
/// };
///
/// let params = CreateSubscriptionParams::new(
///     "cus_12345",
///     vec![
///         SubscriptionItemParams::new("price_seat_monthly", 5),
///         SubscriptionItemParams::new("price_addon_security", 1),
///     ],
/// )
/// .trial_period_days(14)
/// .proration_behavior(ProrationBehavior::CreateProrations)
/// .idempotency_key("sub_idem_create_001");
///
/// assert!(params.validate().is_ok());
/// assert_eq!(params.items.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateSubscriptionParams {
    /// The identifier of the customer to subscribe.
    pub customer: String,

    /// A list of up to 20 subscription items, each with an attached price.
    pub items: Vec<SubscriptionItemParams>,

    /// Boolean indicating whether this subscription should cancel at the end of the current period.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancel_at_period_end: Option<bool>,

    /// Three-letter ISO currency code, in lowercase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    /// ID of the default payment method for this subscription.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_payment_method: Option<String>,

    /// An arbitrary string attached to the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Use `allow_incomplete` to transition the subscription to `incomplete` if the first invoice cannot be paid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_behavior: Option<String>,

    /// Payment settings passed to invoices created by the subscription.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_settings: Option<PaymentSettings>,

    /// Determines how to handle prorations resulting from the create operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proration_behavior: Option<ProrationBehavior>,

    /// Unix timestamp representing the end of the trial period.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trial_end: Option<i64>,

    /// Indicates if a plan's `trial_period_days` should be applied to the subscription.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trial_from_plan: Option<bool>,

    /// Integer representing the number of trial period days before the customer is charged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trial_period_days: Option<u32>,

    /// Either `charge_automatically`, or `send_invoice`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_method: Option<String>,

    /// Number of days a customer has to pay invoices generated by this subscription.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub days_until_due: Option<u32>,

    /// Specifies which fields in the response should be expanded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expand: Option<Vec<String>>,

    /// Optional idempotency key for safe retries (omitted from serialized form body).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateSubscriptionParams {
    /// Creates a new [`CreateSubscriptionParams`] with the required customer and line items.
    pub fn new(customer: impl Into<String>, items: Vec<SubscriptionItemParams>) -> Self {
        Self {
            customer: customer.into(),
            items,
            cancel_at_period_end: None,
            currency: None,
            default_payment_method: None,
            description: None,
            metadata: None,
            payment_behavior: None,
            payment_settings: None,
            proration_behavior: None,
            trial_end: None,
            trial_from_plan: None,
            trial_period_days: None,
            collection_method: None,
            days_until_due: None,
            expand: None,
            idempotency_key: None,
        }
    }

    /// Appends an item to the subscription.
    pub fn item(mut self, item: SubscriptionItemParams) -> Self {
        self.items.push(item);
        self
    }

    /// Appends an item to the subscription (alias for [`item`]).
    pub fn add_item(mut self, item: SubscriptionItemParams) -> Self {
        self.items.push(item);
        self
    }

    /// Sets whether the subscription will cancel at the period end.
    pub fn cancel_at_period_end(mut self, cancel: bool) -> Self {
        self.cancel_at_period_end = Some(cancel);
        self
    }

    /// Sets the currency for the subscription.
    pub fn currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = Some(currency.into().to_lowercase());
        self
    }

    /// Sets the default payment method ID (`pm_...`).
    pub fn default_payment_method(mut self, pm: impl Into<String>) -> Self {
        self.default_payment_method = Some(pm.into());
        self
    }

    /// Sets an optional description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Replaces the metadata map.
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Inserts a key-value pair into the metadata map.
    pub fn add_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let map = self.metadata.get_or_insert_with(HashMap::new);
        map.insert(key.into(), value.into());
        self
    }

    /// Sets the payment behavior (e.g. `allow_incomplete`, `default_incomplete`).
    pub fn payment_behavior(mut self, behavior: impl Into<String>) -> Self {
        self.payment_behavior = Some(behavior.into());
        self
    }

    /// Sets custom payment settings.
    pub fn payment_settings(mut self, settings: PaymentSettings) -> Self {
        self.payment_settings = Some(settings);
        self
    }

    /// Sets the proration behavior.
    pub fn proration_behavior(mut self, pb: ProrationBehavior) -> Self {
        self.proration_behavior = Some(pb);
        self
    }

    /// Sets the trial end timestamp (seconds since Unix epoch).
    pub fn trial_end(mut self, timestamp: i64) -> Self {
        self.trial_end = Some(timestamp);
        self
    }

    /// Sets whether trial should be inherited from plan.
    pub fn trial_from_plan(mut self, from_plan: bool) -> Self {
        self.trial_from_plan = Some(from_plan);
        self
    }

    /// Sets trial duration in days.
    pub fn trial_period_days(mut self, days: u32) -> Self {
        self.trial_period_days = Some(days);
        self
    }

    /// Sets the collection method (`charge_automatically` or `send_invoice`).
    pub fn collection_method(mut self, method: impl Into<String>) -> Self {
        self.collection_method = Some(method.into());
        self
    }

    /// Sets days until due for manual invoices.
    pub fn days_until_due(mut self, days: u32) -> Self {
        self.days_until_due = Some(days);
        self
    }

    /// Sets fields to expand in the response.
    pub fn expand(mut self, expand: Vec<String>) -> Self {
        self.expand = Some(expand);
        self
    }

    /// Sets the idempotency key for safe retries.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates parameters before dispatching network requests.
    ///
    /// # Errors
    /// Returns [`SubscriptionValidationError`] if required fields are missing or invalid.
    pub fn validate(&self) -> Result<(), SubscriptionValidationError> {
        if self.customer.trim().is_empty() {
            return Err(SubscriptionValidationError::InvalidCustomerId(
                self.customer.clone(),
            ));
        }
        if self.items.is_empty() {
            return Err(SubscriptionValidationError::EmptyItems);
        }
        for (i, item) in self.items.iter().enumerate() {
            if item.price.is_none() && item.id.is_none() {
                return Err(SubscriptionValidationError::InvalidParameter {
                    field: "items",
                    reason: format!("item at index {i} must specify either price or id"),
                });
            }
        }
        if let Some(curr) = &self.currency
            && curr.len() != 3
        {
            return Err(SubscriptionValidationError::InvalidCurrency(curr.clone()));
        }
        Ok(())
    }
}

/// Item parameter schema for creating or updating items on a Subscription.
///
/// # Examples
/// ```
/// use some_stripe_types::subscription::SubscriptionItemParams;
///
/// let item = SubscriptionItemParams::new("price_basic_10", 2);
/// assert_eq!(item.price.as_deref(), Some("price_basic_10"));
/// assert_eq!(item.quantity, Some(2));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SubscriptionItemParams {
    /// The ID of the price object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,

    /// The quantity you'd like to apply to the subscription item you're creating.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<u64>,

    /// Subscription item to update (when updating an existing subscription item).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Delete all usage for a given subscription item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Determines how to handle prorations resulting from this item change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proration_behavior: Option<ProrationBehavior>,

    /// Define thresholds at which an invoice will be sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing_thresholds: Option<SubscriptionItemBillingThresholds>,

    /// A list of tax rates applied to this item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_rates: Option<Vec<String>>,
}

/// Type alias for [`SubscriptionItemParams`] providing backward and singular compatibility.
pub type SubscriptionItemParam = SubscriptionItemParams;

impl SubscriptionItemParams {
    /// Creates a new subscription item parameter with the given price and quantity.
    pub fn new(price: impl Into<String>, quantity: u64) -> Self {
        Self {
            price: Some(price.into()),
            quantity: Some(quantity),
            ..Default::default()
        }
    }

    /// Creates a subscription item parameter with price and default quantity 1.
    pub fn price(price: impl Into<String>) -> Self {
        Self {
            price: Some(price.into()),
            ..Default::default()
        }
    }

    /// Creates a subscription item parameter with price.
    pub fn with_price(price: impl Into<String>) -> Self {
        Self::price(price)
    }

    /// Creates a subscription item parameter with price and quantity.
    pub fn with_price_and_quantity(price: impl Into<String>, quantity: u64) -> Self {
        Self::new(price, quantity)
    }

    /// Creates a subscription item parameter to modify an existing item by ID.
    pub fn for_update(id: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            ..Default::default()
        }
    }

    /// Creates a subscription item parameter to delete an existing item by ID.
    pub fn for_deletion(id: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            deleted: Some(true),
            ..Default::default()
        }
    }

    /// Sets the quantity.
    pub fn with_quantity(mut self, quantity: u64) -> Self {
        self.quantity = Some(quantity);
        self
    }

    /// Sets the quantity.
    pub fn quantity(mut self, quantity: u64) -> Self {
        self.quantity = Some(quantity);
        self
    }

    /// Sets the existing subscription item ID (`si_...`).
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Marks the item for deletion during an update.
    pub fn deleted(mut self, deleted: bool) -> Self {
        self.deleted = Some(deleted);
        self
    }

    /// Sets metadata.
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets billing thresholds.
    pub fn billing_thresholds(mut self, thresholds: SubscriptionItemBillingThresholds) -> Self {
        self.billing_thresholds = Some(thresholds);
        self
    }

    /// Sets tax rates.
    pub fn tax_rates(mut self, tax_rates: Vec<String>) -> Self {
        self.tax_rates = Some(tax_rates);
        self
    }

    /// Sets the proration behavior for this item.
    pub fn proration_behavior(mut self, pb: ProrationBehavior) -> Self {
        self.proration_behavior = Some(pb);
        self
    }
}

/// Parameters for updating an existing Stripe Subscription (`POST /v1/subscriptions/:id`).
///
/// # Examples
/// ```
/// use some_stripe_types::subscription::{
///     UpdateSubscriptionParams, SubscriptionItemParams, PauseCollectionParams,
/// };
///
/// let params = UpdateSubscriptionParams::new()
///     .cancel_at_period_end(true)
///     .pause_collection(PauseCollectionParams::void());
///
/// assert!(params.validate().is_ok());
/// assert_eq!(params.cancel_at_period_end, Some(true));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UpdateSubscriptionParams {
    /// Boolean indicating whether this subscription should cancel at the end of the current period.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancel_at_period_end: Option<bool>,

    /// ID of the default payment method for this subscription.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_payment_method: Option<String>,

    /// An arbitrary string attached to the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// A list of up to 20 subscription items, each with an attached price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<SubscriptionItemParams>>,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// If specified, the payment collection behavior for this subscription is paused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pause_collection: Option<PauseCollectionParams>,

    /// Use `allow_incomplete` to transition the subscription to `incomplete` if the first invoice cannot be paid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_behavior: Option<String>,

    /// Payment settings passed to invoices created by the subscription.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_settings: Option<PaymentSettings>,

    /// Determines how to handle prorations resulting from the update operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proration_behavior: Option<ProrationBehavior>,

    /// Unix timestamp representing the end of the trial period.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trial_end: Option<i64>,

    /// Specifies which fields in the response should be expanded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expand: Option<Vec<String>>,

    /// Optional idempotency key for safe retries (omitted from serialized form body).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl UpdateSubscriptionParams {
    /// Creates a new default [`UpdateSubscriptionParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether the subscription will cancel at period end.
    pub fn cancel_at_period_end(mut self, cancel: bool) -> Self {
        self.cancel_at_period_end = Some(cancel);
        self
    }

    /// Sets default payment method ID (`pm_...`).
    pub fn default_payment_method(mut self, pm: impl Into<String>) -> Self {
        self.default_payment_method = Some(pm.into());
        self
    }

    /// Sets description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Sets subscription items.
    pub fn items(mut self, items: Vec<SubscriptionItemParams>) -> Self {
        self.items = Some(items);
        self
    }

    /// Appends a subscription item change.
    pub fn add_item(mut self, item: SubscriptionItemParams) -> Self {
        self.items.get_or_insert_with(Vec::new).push(item);
        self
    }

    /// Sets collection pause options.
    pub fn pause_collection(mut self, pause: PauseCollectionParams) -> Self {
        self.pause_collection = Some(pause);
        self
    }

    /// Sets payment behavior.
    pub fn payment_behavior(mut self, pb: impl Into<String>) -> Self {
        self.payment_behavior = Some(pb.into());
        self
    }

    /// Sets payment settings.
    pub fn payment_settings(mut self, ps: PaymentSettings) -> Self {
        self.payment_settings = Some(ps);
        self
    }

    /// Sets proration behavior.
    pub fn proration_behavior(mut self, pb: ProrationBehavior) -> Self {
        self.proration_behavior = Some(pb);
        self
    }

    /// Sets trial end.
    pub fn trial_end(mut self, end: i64) -> Self {
        self.trial_end = Some(end);
        self
    }

    /// Sets metadata.
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Appends a key-value pair to metadata.
    pub fn add_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let map = self.metadata.get_or_insert_with(HashMap::new);
        map.insert(key.into(), value.into());
        self
    }

    /// Sets fields to expand.
    pub fn expand(mut self, expand: Vec<String>) -> Self {
        self.expand = Some(expand);
        self
    }

    /// Sets idempotency key.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates parameters.
    pub fn validate(&self) -> Result<(), SubscriptionValidationError> {
        if let Some(pause) = &self.pause_collection {
            let b = pause.behavior.as_str();
            if b != "keep_as_draft" && b != "mark_uncollectible" && b != "void" {
                return Err(SubscriptionValidationError::InvalidPauseBehavior(
                    pause.behavior.clone(),
                ));
            }
        }
        Ok(())
    }
}

/// Parameters for pausing payment collection on a subscription.
///
/// # Examples
/// ```
/// use some_stripe_types::subscription::PauseCollectionParams;
///
/// let pause = PauseCollectionParams::void().with_resumes_at(1735689600);
/// assert_eq!(pause.behavior, "void");
/// assert_eq!(pause.resumes_at, Some(1735689600));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PauseCollectionParams {
    /// The payment collection behavior while paused.
    /// Values: `keep_as_draft`, `mark_uncollectible`, `void`.
    pub behavior: String,

    /// The time at which payment collection will resume.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resumes_at: Option<i64>,
}

impl PauseCollectionParams {
    /// Sets collection pause behavior to void invoices.
    pub fn void() -> Self {
        Self {
            behavior: "void".into(),
            resumes_at: None,
        }
    }

    /// Sets collection pause behavior to keep invoices as draft.
    pub fn keep_as_draft() -> Self {
        Self {
            behavior: "keep_as_draft".into(),
            resumes_at: None,
        }
    }

    /// Sets collection pause behavior to mark invoices uncollectible.
    pub fn mark_uncollectible() -> Self {
        Self {
            behavior: "mark_uncollectible".into(),
            resumes_at: None,
        }
    }

    /// Specifies when payment collection should resume.
    pub fn with_resumes_at(mut self, timestamp: i64) -> Self {
        self.resumes_at = Some(timestamp);
        self
    }
}

/// Parameters for canceling a Stripe Subscription (`DELETE /v1/subscriptions/:id`).
///
/// # Examples
/// ```
/// use some_stripe_types::subscription::CancelSubscriptionParams;
///
/// let params = CancelSubscriptionParams::new()
///     .prorate(true)
///     .invoice_now(true);
/// assert_eq!(params.prorate, Some(true));
/// assert_eq!(params.invoice_now, Some(true));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CancelSubscriptionParams {
    /// Will generate a final invoice that invoices for any un-invoiced metered usage and
    /// prorations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice_now: Option<bool>,

    /// Will generate a proration invoice item that credits remaining unused time until the
    /// subscription period end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prorate: Option<bool>,

    /// Details about why this subscription was canceled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_details: Option<CancellationDetailsParams>,

    /// Optional idempotency key for safe retries (omitted from serialized form body).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CancelSubscriptionParams {
    /// Creates a new default [`CancelSubscriptionParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to invoice immediately upon cancellation.
    pub fn invoice_now(mut self, invoice: bool) -> Self {
        self.invoice_now = Some(invoice);
        self
    }

    /// Sets whether to prorate upon cancellation.
    pub fn prorate(mut self, prorate: bool) -> Self {
        self.prorate = Some(prorate);
        self
    }

    /// Sets cancellation details.
    pub fn cancellation_details(mut self, details: CancellationDetailsParams) -> Self {
        self.cancellation_details = Some(details);
        self
    }

    /// Sets the idempotency key for cancellation.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Details about why a subscription was canceled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CancellationDetailsParams {
    /// Additional comments about why the user canceled the subscription.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    /// The customer's feedback on why they canceled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback: Option<String>,
}

/// Parameters for resuming a paused Stripe Subscription (`POST /v1/subscriptions/:id/resume`).
///
/// # Examples
/// ```
/// use some_stripe_types::subscription::{ResumeSubscriptionParams, ProrationBehavior};
///
/// let params = ResumeSubscriptionParams::new()
///     .billing_cycle_anchor("now")
///     .proration_behavior(ProrationBehavior::CreateProrations);
/// assert_eq!(params.billing_cycle_anchor.as_deref(), Some("now"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ResumeSubscriptionParams {
    /// Determines how to handle billing cycle anchor after resume (`now` or `unchanged`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing_cycle_anchor: Option<String>,

    /// Determines how to handle prorations resulting from the resume operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proration_behavior: Option<ProrationBehavior>,

    /// Optional idempotency key for safe retries (omitted from serialized form body).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl ResumeSubscriptionParams {
    /// Creates a new default [`ResumeSubscriptionParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets billing cycle anchor behavior (`now` or `unchanged`).
    pub fn billing_cycle_anchor(mut self, anchor: impl Into<String>) -> Self {
        self.billing_cycle_anchor = Some(anchor.into());
        self
    }

    /// Sets proration behavior upon resume.
    pub fn proration_behavior(mut self, pb: ProrationBehavior) -> Self {
        self.proration_behavior = Some(pb);
        self
    }

    /// Sets the idempotency key.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for listing Subscriptions (`GET /v1/subscriptions`).
///
/// # Examples
/// ```
/// use some_stripe_types::subscription::ListSubscriptionsParams;
///
/// let params = ListSubscriptionsParams::new()
///     .customer("cus_12345")
///     .status("active")
///     .limit(50);
///
/// assert_eq!(params.customer.as_deref(), Some("cus_12345"));
/// assert_eq!(params.status.as_deref(), Some("active"));
/// assert_eq!(params.limit, Some(50));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ListSubscriptionsParams {
    /// Filter by customer identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,

    /// Filter by price identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,

    /// Filter by status (`active`, `canceled`, `trialing`, `past_due`, `all`, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Cursor for forward pagination: object ID defining your place in the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// Cursor for backward pagination: object ID defining your place in the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,

    /// A limit on the number of objects to be returned (1 to 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Filter by start of current period.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_period_start: Option<i64>,

    /// Filter by end of current period.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_period_end: Option<i64>,

    /// Filter by creation time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
}

impl ListSubscriptionsParams {
    /// Creates a new default [`ListSubscriptionsParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the customer filter.
    pub fn customer(mut self, customer: impl Into<String>) -> Self {
        self.customer = Some(customer.into());
        self
    }

    /// Sets the price filter.
    pub fn price(mut self, price: impl Into<String>) -> Self {
        self.price = Some(price.into());
        self
    }

    /// Sets the status filter.
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Sets the maximum items to return (clamped between 1 and 100).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit.clamp(1, 100));
        self
    }

    /// Sets starting after cursor.
    pub fn starting_after(mut self, cursor: impl Into<String>) -> Self {
        self.starting_after = Some(cursor.into());
        self
    }

    /// Sets ending before cursor.
    pub fn ending_before(mut self, cursor: impl Into<String>) -> Self {
        self.ending_before = Some(cursor.into());
        self
    }
}

/// Parameters for searching subscriptions (`GET /v1/subscriptions/search`).
///
/// # Examples
/// ```
/// use some_stripe_types::subscription::SearchSubscriptionsParams;
///
/// let params = SearchSubscriptionsParams::new("status:'active' AND metadata['tier']:'gold'");
/// assert_eq!(params.query, "status:'active' AND metadata['tier']:'gold'");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchSubscriptionsParams {
    /// The search query string following Stripe search syntax.
    pub query: String,

    /// A limit on the number of objects to be returned (1 to 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// A cursor for pagination across search results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
}

impl SearchSubscriptionsParams {
    /// Creates a new search parameter struct with the query.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            limit: None,
            page: None,
        }
    }

    /// Sets search results limit.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit.clamp(1, 100));
        self
    }

    /// Sets search pagination cursor page.
    pub fn page(mut self, page: impl Into<String>) -> Self {
        self.page = Some(page.into());
        self
    }
}

/// Validation errors for subscription parameter schemas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionValidationError {
    /// A subscription must have at least one line item.
    EmptyItems,
    /// Invalid customer ID supplied.
    InvalidCustomerId(String),
    /// Invalid 3-letter currency code supplied.
    InvalidCurrency(String),
    /// The trial end timestamp is in the past.
    InvalidTrialEnd {
        /// Supplied trial end timestamp.
        trial_end: i64,
        /// Reference current timestamp.
        current_time: i64,
    },
    /// Invalid pause behavior.
    InvalidPauseBehavior(String),
    /// A general parameter validation failure.
    InvalidParameter {
        /// Field name.
        field: &'static str,
        /// Description of the error.
        reason: String,
    },
}

impl std::fmt::Display for SubscriptionValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyItems => write!(f, "Subscription must have at least one line item"),
            Self::InvalidCustomerId(id) => write!(f, "Invalid customer ID: '{id}'"),
            Self::InvalidCurrency(c) => write!(f, "Currency must be a 3-letter ISO code: '{c}'"),
            Self::InvalidTrialEnd {
                trial_end,
                current_time,
            } => write!(
                f,
                "trial_end ({trial_end}) must be strictly in the future relative to current timestamp ({current_time})"
            ),
            Self::InvalidPauseBehavior(b) => write!(f, "Invalid pause behavior: '{b}'"),
            Self::InvalidParameter { field, reason } => {
                write!(f, "Invalid subscription parameter '{field}': {reason}")
            }
        }
    }
}

impl std::error::Error for SubscriptionValidationError {}
