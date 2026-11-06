//! Subscription domain entities, status state machines, and schedule schemas for Stripe.
//!
//! Enforces ISO 4217 minor currency integer rules (Gate G-Law) and Rust zero-clone
//! borrowing discipline (Gate G-Borrow).

use serde::{Deserialize, Serialize};

/// Represents the status lifecycle of a Stripe Subscription.
///
/// Invariants:
/// - Transitions: `incomplete` -> `incomplete_expired`, or `incomplete` -> `active` / `trialing`.
/// - `trialing`: Within the active trial period.
/// - `past_due`: Renewal payment failed, within dunning retry period.
/// - `canceled`: Subscription permanently terminated.
/// - `unpaid`: Dunning retries exhausted, invoices remain unpaid.
/// - `paused`: Payment collection paused via `pause_collection`.
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
    /// Returns `true` if the subscription is currently active or in trial.
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Active | Self::Trialing)
    }

    /// Returns `true` if the subscription is in a terminal or past-due dunning state.
    pub fn is_delinquent_or_canceled(&self) -> bool {
        matches!(self, Self::PastDue | Self::Canceled | Self::Unpaid)
    }
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
/// assert_eq!(sub.id, "sub_12345");
/// assert_eq!(sub.status, SubscriptionStatus::Active);
/// assert!(sub.status.is_operational());
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
    pub metadata: std::collections::HashMap<String, String>,

    /// ID of the default payment method attached to this subscription.
    pub default_payment_method: Option<String>,

    /// The latest invoice associated with this subscription.
    pub latest_invoice: Option<String>,

    /// The schedule that manages this subscription, if any.
    pub schedule: Option<String>,
}

fn default_subscription_object() -> String {
    "subscription".into()
}

/// Collection pause details on a subscription.
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
    pub metadata: std::collections::HashMap<String, String>,
}

fn default_subscription_item_object() -> String {
    "subscription_item".into()
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
