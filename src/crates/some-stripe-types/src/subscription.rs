//! Subscription domain entities, status state machines, and schedule schemas for Stripe.
//!
//! Enforces ISO 4217 minor currency integer rules (Gate G-Law), PCI-DSS 4.0 tokenization
//! (Gate G2/G-Law), and Rust 2024 zero-clone borrowing discipline (Gate G-Borrow).
//!
//! # Payment Method Tokenization (PCI-DSS 4.0 Gate G-Law)
//! All payment credentials on subscription entities are referenced strictly through tokenized
//! identifiers (`pm_...`), guaranteeing that zero raw Primary Account Numbers (PAN)
//! or sensitive authentication data (SAD) enter application memory.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Error returned when parsing an unrecognized [`SubscriptionStatus`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseSubscriptionStatusError(pub String);

impl std::fmt::Display for ParseSubscriptionStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown SubscriptionStatus: '{}'", self.0)
    }
}

impl std::error::Error for ParseSubscriptionStatusError {}

/// Represents the status lifecycle of a Stripe Subscription.
///
/// Invariants:
/// - Initial creation: `incomplete` or `trialing` or `active`.
/// - Dunning: Transitions to `past_due` on invoice failure, then `unpaid` when retries expire.
/// - Termination: Transitions to `canceled`.
/// - Paused: Payment collection halted via `pause_collection`.
///
/// # Examples
///
/// ```
/// use some_stripe_types::subscription::SubscriptionStatus;
/// use std::str::FromStr;
///
/// let status = SubscriptionStatus::from_str("trialing").unwrap();
/// assert_eq!(status, SubscriptionStatus::Trialing);
/// assert!(status.is_operational());
/// assert!(!status.is_delinquent_or_canceled());
/// assert_eq!(status.as_str(), "trialing");
/// assert_eq!(status, "trialing");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    /// Initial payment setup required before the subscription can become active.
    Incomplete,
    /// Initial payment setup expired before successful confirmation.
    IncompleteExpired,
    /// Subscription is currently within an active free trial period.
    Trialing,
    /// Subscription is active and in good standing.
    #[default]
    Active,
    /// The most recent payment attempt failed; currently undergoing dunning retries.
    PastDue,
    /// Subscription has been permanently terminated and canceled.
    Canceled,
    /// Dunning retries have been exhausted without payment; invoices remain unpaid.
    Unpaid,
    /// Payment collection is currently paused.
    Paused,
}

impl SubscriptionStatus {
    /// Returns the Stripe API string representation of the status.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
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
    #[inline]
    pub const fn is_operational(&self) -> bool {
        matches!(self, Self::Active | Self::Trialing)
    }

    /// Returns `true` if the subscription is active.
    #[inline]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns `true` if the subscription is in free trial.
    #[inline]
    pub const fn is_trialing(&self) -> bool {
        matches!(self, Self::Trialing)
    }

    /// Returns `true` if the subscription is paused.
    #[inline]
    pub const fn is_paused(&self) -> bool {
        matches!(self, Self::Paused)
    }

    /// Returns `true` if the subscription is canceled.
    #[inline]
    pub const fn is_canceled(&self) -> bool {
        matches!(self, Self::Canceled)
    }

    /// Returns `true` if the subscription is in a terminal or past-due dunning state.
    #[inline]
    pub const fn is_delinquent_or_canceled(&self) -> bool {
        matches!(self, Self::PastDue | Self::Canceled | Self::Unpaid)
    }
}

impl std::fmt::Display for SubscriptionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for SubscriptionStatus {
    type Err = ParseSubscriptionStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "incomplete" => Ok(Self::Incomplete),
            "incomplete_expired" => Ok(Self::IncompleteExpired),
            "trialing" => Ok(Self::Trialing),
            "active" => Ok(Self::Active),
            "past_due" => Ok(Self::PastDue),
            "canceled" => Ok(Self::Canceled),
            "unpaid" => Ok(Self::Unpaid),
            "paused" => Ok(Self::Paused),
            _ => Err(ParseSubscriptionStatusError(s.to_string())),
        }
    }
}

impl AsRef<str> for SubscriptionStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<str> for SubscriptionStatus {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for SubscriptionStatus {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<SubscriptionStatus> for str {
    fn eq(&self, other: &SubscriptionStatus) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<SubscriptionStatus> for &str {
    fn eq(&self, other: &SubscriptionStatus) -> bool {
        *self == other.as_str()
    }
}

/// Represents a Stripe Subscription object.
///
/// Subscriptions allow you to charge a customer on a recurring basis. A subscription
/// ties a vaulted customer to one or more prices.
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
///     "default_payment_method": "pm_card_visa",
///     "items": {
///         "object": "list",
///         "data": [],
///         "has_more": false,
///         "url": "/v1/subscription_items"
///     }
/// }"#;
/// let sub: Subscription = serde_json::from_str(json).unwrap();
/// assert_eq!(sub.id, "sub_12345");
/// assert_eq!(sub.status, SubscriptionStatus::Active);
/// assert!(sub.status.is_operational());
/// assert_eq!(sub.default_payment_method.as_deref(), Some("pm_card_visa"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    /// Unique identifier for the subscription (`sub_...`).
    pub id: String,

    /// Object type, always `subscription`.
    #[serde(default = "default_subscription_object")]
    pub object: String,

    /// ID of the customer who owns the subscription (`cus_...`).
    pub customer: String,

    /// Current status of the subscription.
    pub status: SubscriptionStatus,

    /// Start of the current billing period (Unix epoch seconds).
    pub current_period_start: i64,

    /// End of the current billing period (Unix epoch seconds).
    pub current_period_end: i64,

    /// A date in the future at which the subscription will cancel (Unix epoch seconds).
    pub cancel_at: Option<i64>,

    /// If `true`, the subscription will be canceled at the end of the current period.
    #[serde(default)]
    pub cancel_at_period_end: bool,

    /// If the subscription has been canceled, the date of cancellation (Unix epoch seconds).
    pub canceled_at: Option<i64>,

    /// Time at which the subscription was created (Unix epoch seconds).
    pub created: i64,

    /// Three-letter ISO currency code, in lowercase (e.g. `usd`, `eur`).
    pub currency: Option<String>,

    /// If the subscription has ended, the date the subscription ended (Unix epoch seconds).
    pub ended_at: Option<i64>,

    /// Date when the subscription was started (Unix epoch seconds).
    pub start_date: Option<i64>,

    /// If specified, start of the trial period (Unix epoch seconds).
    pub trial_start: Option<i64>,

    /// If specified, end of the trial period (Unix epoch seconds).
    pub trial_end: Option<i64>,

    /// How payment is collected for this subscription (`charge_automatically` or `send_invoice`).
    pub collection_method: Option<String>,

    /// Number of days until invoices are due, if `collection_method` is `send_invoice`.
    pub days_until_due: Option<u32>,

    /// A date in the past used as an anchor point for calculating recurring billing cycle dates.
    pub billing_cycle_anchor: Option<i64>,

    /// Description of the subscription.
    pub description: Option<String>,

    /// Collection pause details if the subscription is paused.
    pub pause_collection: Option<PauseCollection>,

    /// List of subscription items on this subscription.
    pub items: SubscriptionItemList,

    /// Key-value metadata attached to the subscription.
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    /// ID of the default payment method (`pm_...`) attached to this subscription.
    ///
    /// PCI-DSS 4.0: Only tokenized references are held. Zero raw PAN.
    pub default_payment_method: Option<String>,

    /// The latest invoice associated with this subscription (`in_...`).
    pub latest_invoice: Option<String>,

    /// The schedule that manages this subscription (`sub_sched_...`), if any.
    pub schedule: Option<String>,

    /// Has the value `true` if the object exists in live mode or `false` for test mode.
    #[serde(default)]
    pub livemode: bool,
}

impl Subscription {
    /// Returns the subscription's unique ID (`sub_...`).
    #[inline]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the customer ID (`cus_...`) associated with this subscription.
    #[inline]
    pub fn customer(&self) -> &str {
        &self.customer
    }

    /// Returns the tokenized default payment method ID (`pm_...`) if attached.
    #[inline]
    pub fn default_payment_method(&self) -> Option<&str> {
        self.default_payment_method.as_deref()
    }

    /// Returns `true` if the subscription is currently in a free trial.
    #[inline]
    pub fn is_trialing(&self) -> bool {
        self.status.is_trialing()
    }

    /// Returns `true` if the subscription is active.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }

    /// Returns `true` if the subscription is paused.
    #[inline]
    pub fn is_paused(&self) -> bool {
        self.status.is_paused()
    }

    /// Returns `true` if the subscription is canceled.
    #[inline]
    pub fn is_canceled(&self) -> bool {
        self.status.is_canceled()
    }

    /// Returns a slice of the subscription items attached to this subscription (Gate G-Borrow).
    #[inline]
    pub fn items(&self) -> &[SubscriptionItem] {
        &self.items.data
    }

    /// Returns the number of subscription items attached to this subscription.
    #[inline]
    pub fn item_count(&self) -> usize {
        self.items.data.len()
    }

    /// Returns the price ID of the primary (first) subscription item, if present.
    #[inline]
    pub fn first_price_id(&self) -> Option<&str> {
        self.items.data.first().and_then(|item| item.price_id())
    }
}

fn default_subscription_object() -> String {
    "subscription".into()
}

/// Collection pause details on a subscription.
///
/// # Example
///
/// ```
/// use some_stripe_types::subscription::PauseCollection;
///
/// let pause = PauseCollection {
///     behavior: "void".into(),
///     resumes_at: Some(1735689600),
/// };
/// assert_eq!(pause.behavior, "void");
/// assert_eq!(pause.resumes_at, Some(1735689600));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PauseCollection {
    /// The payment collection behavior while paused.
    /// Values: `keep_as_draft`, `mark_uncollectible`, `void`.
    pub behavior: String,

    /// The time at which payment collection will automatically resume (Unix epoch seconds).
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
    #[serde(default = "default_subscription_items_url")]
    pub url: String,
}

fn default_list_object() -> String {
    "list".into()
}

fn default_subscription_items_url() -> String {
    "/v1/subscription_items".into()
}

/// Represents an individual item within a Subscription.
///
/// Subscription items allow you to bill for multiple products and tiered pricing
/// on a single subscription.
///
/// # Examples
///
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

    /// The ID of the subscription this item belongs to (`sub_...`).
    pub subscription: String,

    /// The price associated with this item.
    pub price: Option<SubscriptionPrice>,

    /// Quantity of the plan to which the customer is subscribed.
    pub quantity: Option<u64>,

    /// Time at which the item was created (Unix epoch seconds).
    pub created: i64,

    /// Key-value metadata attached to the item.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl SubscriptionItem {
    /// Returns the price ID associated with this item if present.
    #[inline]
    pub fn price_id(&self) -> Option<&str> {
        self.price.as_ref().map(|p| p.id.as_str())
    }

    /// Returns the unit amount in minor currency integer units (e.g. cents) if available.
    #[inline]
    pub fn unit_amount(&self) -> Option<i64> {
        self.price.as_ref().and_then(|p| p.unit_amount)
    }

    /// Returns the ISO 4217 lowercase currency code if available.
    #[inline]
    pub fn currency(&self) -> Option<&str> {
        self.price.as_ref().map(|p| p.currency.as_str())
    }
}

fn default_subscription_item_object() -> String {
    "subscription_item".into()
}

/// Price information attached to a subscription item.
///
/// # Example
///
/// ```
/// use some_stripe_types::subscription::SubscriptionPrice;
///
/// let json = r#"{
///     "id": "price_pro_monthly",
///     "object": "price",
///     "active": true,
///     "currency": "usd",
///     "product": "prod_saas",
///     "unit_amount": 2900
/// }"#;
/// let price: SubscriptionPrice = serde_json::from_str(json).unwrap();
/// assert_eq!(price.id, "price_pro_monthly");
/// assert_eq!(price.unit_amount, Some(2900));
/// assert!(price.active);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionPrice {
    /// Unique identifier for the price (`price_...`).
    pub id: String,

    /// Object type, always `price`.
    #[serde(default = "default_price_object")]
    pub object: String,

    /// Whether the price can be used for new purchases.
    #[serde(default = "default_true")]
    pub active: bool,

    /// Three-letter ISO currency code, in lowercase.
    pub currency: String,

    /// The ID of the product this price is associated with (`prod_...`).
    pub product: String,

    /// The unit amount in minor currency integer units (e.g., cents).
    pub unit_amount: Option<i64>,

    /// The recurring components of a price such as `interval` and `usage_type`.
    pub recurring: Option<SubscriptionPriceRecurring>,
}

fn default_price_object() -> String {
    "price".into()
}

fn default_true() -> bool {
    true
}

/// Recurring billing details for a subscription price.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionPriceRecurring {
    /// The frequency at which a subscription is billed (`day`, `week`, `month`, `year`).
    pub interval: String,

    /// The number of intervals between subscription billings.
    pub interval_count: Option<u64>,

    /// Default number of trial days when subscribing to this price.
    pub trial_period_days: Option<u64>,

    /// Configures how the quantity per period should be determined (`licensed`, `metered`).
    pub usage_type: Option<String>,
}

/// Error returned when parsing an unrecognized [`ProrationBehavior`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseProrationBehaviorError(pub String);

impl std::fmt::Display for ParseProrationBehaviorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown ProrationBehavior: '{}'", self.0)
    }
}

impl std::error::Error for ParseProrationBehaviorError {}

/// Proration behavior options for subscription modifications.
///
/// Determines how Stripe handles prorations when updating or canceling subscriptions.
///
/// # Examples
///
/// ```
/// use some_stripe_types::subscription::ProrationBehavior;
/// use std::str::FromStr;
///
/// let pb = ProrationBehavior::from_str("create_prorations").unwrap();
/// assert_eq!(pb, ProrationBehavior::CreateProrations);
/// assert_eq!(pb.as_str(), "create_prorations");
/// assert_eq!(pb, "create_prorations");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProrationBehavior {
    /// Automatically calculates and creates proration invoice items for unused time or upgrades.
    #[default]
    CreateProrations,
    /// Disables prorations; no credit or charge is generated.
    None,
    /// Immediately creates and attempts to pay a proration invoice.
    AlwaysInvoice,
}

impl ProrationBehavior {
    /// Returns the Stripe API string representation of the proration behavior.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::CreateProrations => "create_prorations",
            Self::None => "none",
            Self::AlwaysInvoice => "always_invoice",
        }
    }
}

impl std::fmt::Display for ProrationBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ProrationBehavior {
    type Err = ParseProrationBehaviorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "create_prorations" => Ok(Self::CreateProrations),
            "none" => Ok(Self::None),
            "always_invoice" => Ok(Self::AlwaysInvoice),
            _ => Err(ParseProrationBehaviorError(s.to_string())),
        }
    }
}

impl AsRef<str> for ProrationBehavior {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<str> for ProrationBehavior {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for ProrationBehavior {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

/// Error returned when parsing an unrecognized [`SubscriptionScheduleStatus`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseSubscriptionScheduleStatusError(pub String);

impl std::fmt::Display for ParseSubscriptionScheduleStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown SubscriptionScheduleStatus: '{}'", self.0)
    }
}

impl std::error::Error for ParseSubscriptionScheduleStatusError {}

/// Lifecycle status of a Subscription Schedule.
///
/// # Examples
///
/// ```
/// use some_stripe_types::subscription::SubscriptionScheduleStatus;
/// use std::str::FromStr;
///
/// let s = SubscriptionScheduleStatus::from_str("active").unwrap();
/// assert_eq!(s, SubscriptionScheduleStatus::Active);
/// assert_eq!(s.as_str(), "active");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionScheduleStatus {
    /// The schedule has not yet started.
    #[default]
    NotStarted,
    /// The schedule is currently active.
    Active,
    /// All phases of the schedule have completed.
    Completed,
    /// The schedule was released and is no longer managing the subscription.
    Released,
    /// The schedule was canceled.
    Canceled,
}

impl SubscriptionScheduleStatus {
    /// Returns the Stripe API string representation of the schedule status.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Released => "released",
            Self::Canceled => "canceled",
        }
    }
}

impl std::fmt::Display for SubscriptionScheduleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for SubscriptionScheduleStatus {
    type Err = ParseSubscriptionScheduleStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "not_started" => Ok(Self::NotStarted),
            "active" => Ok(Self::Active),
            "completed" => Ok(Self::Completed),
            "released" => Ok(Self::Released),
            "canceled" => Ok(Self::Canceled),
            _ => Err(ParseSubscriptionScheduleStatusError(s.to_string())),
        }
    }
}

impl AsRef<str> for SubscriptionScheduleStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<str> for SubscriptionScheduleStatus {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for SubscriptionScheduleStatus {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

/// Represents a Stripe Subscription Schedule object for multi-phase lifecycles.
///
/// Subscription schedules allow you to orchestrate changes over time, including trial periods,
/// discount phases, and plan upgrades.
///
/// # Examples
///
/// ```
/// use some_stripe_types::subscription::{SubscriptionSchedule, SubscriptionScheduleStatus};
///
/// let json = r#"{
///     "id": "sub_sched_12345",
///     "object": "subscription_schedule",
///     "customer": "cus_12345",
///     "status": "active",
///     "created": 1728345600,
///     "phases": []
/// }"#;
/// let sched: SubscriptionSchedule = serde_json::from_str(json).unwrap();
/// assert_eq!(sched.id, "sub_sched_12345");
/// assert_eq!(sched.status, SubscriptionScheduleStatus::Active);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionSchedule {
    /// Unique identifier for the schedule (`sub_sched_...`).
    pub id: String,

    /// Object type, always `subscription_schedule`.
    #[serde(default = "default_subscription_schedule_object")]
    pub object: String,

    /// The customer who owns this schedule (`cus_...`).
    pub customer: String,

    /// Current status of the schedule (`not_started`, `active`, `completed`, `released`, `canceled`).
    pub status: SubscriptionScheduleStatus,

    /// Phases configured for this schedule.
    #[serde(default)]
    pub phases: Vec<SubscriptionSchedulePhase>,

    /// The subscription currently managed by this schedule (`sub_...`), if any.
    pub subscription: Option<String>,

    /// Time at which the schedule was created (Unix epoch seconds).
    pub created: i64,

    /// Object representing the current phase of the schedule.
    pub current_phase: Option<SubscriptionScheduleCurrentPhase>,

    /// Behavior at the end of the schedule (`release` or `cancel`).
    pub end_behavior: Option<String>,

    /// Time at which the schedule was released (Unix epoch seconds).
    pub released_at: Option<i64>,

    /// ID of the subscription once released from the schedule.
    pub released_subscription: Option<String>,

    /// Time at which the schedule was canceled (Unix epoch seconds).
    pub canceled_at: Option<i64>,

    /// Time at which the schedule was completed (Unix epoch seconds).
    pub completed_at: Option<i64>,

    /// Has the value `true` if the object exists in live mode or `false` for test mode.
    #[serde(default)]
    pub livemode: bool,

    /// Key-value metadata attached to the schedule.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_subscription_schedule_object() -> String {
    "subscription_schedule".into()
}

/// Represents the current active phase boundary within a Subscription Schedule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionScheduleCurrentPhase {
    /// Start of the current phase (Unix epoch seconds).
    pub start_date: i64,

    /// End of the current phase (Unix epoch seconds).
    pub end_date: i64,
}

/// Individual phase in a multi-phase Subscription Schedule.
///
/// # Example
///
/// ```
/// use some_stripe_types::subscription::{ProrationBehavior, SubscriptionSchedulePhase};
///
/// let phase = SubscriptionSchedulePhase {
///     start_date: 1728345600,
///     end_date: 1730937600,
///     iterations: Some(1),
///     proration_behavior: Some(ProrationBehavior::CreateProrations),
///     items: None,
///     currency: Some("usd".into()),
///     trial_end: None,
///     collection_method: None,
///     description: Some("Introductory promotional phase".into()),
///     metadata: std::collections::HashMap::new(),
/// };
/// assert_eq!(phase.iterations, Some(1));
/// assert_eq!(phase.proration_behavior, Some(ProrationBehavior::CreateProrations));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionSchedulePhase {
    /// Start of the phase (Unix epoch seconds).
    pub start_date: i64,

    /// End of the phase (Unix epoch seconds).
    pub end_date: i64,

    /// Number of iterations this phase will run for.
    pub iterations: Option<u64>,

    /// Proration behavior to apply when transitioning to or from this phase.
    pub proration_behavior: Option<ProrationBehavior>,

    /// List of subscription items in this phase.
    pub items: Option<Vec<SubscriptionSchedulePhaseItem>>,

    /// Three-letter ISO currency code, in lowercase.
    pub currency: Option<String>,

    /// If set, the trial end date for this phase (Unix epoch seconds).
    pub trial_end: Option<i64>,

    /// How payment is collected for this phase (`charge_automatically` or `send_invoice`).
    pub collection_method: Option<String>,

    /// Description of the phase.
    pub description: Option<String>,

    /// Key-value metadata attached to this phase.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Subscription item specification within a Subscription Schedule Phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionSchedulePhaseItem {
    /// The ID of the price for this item (`price_...`).
    pub price: String,

    /// Quantity of the item.
    pub quantity: Option<u64>,
}
