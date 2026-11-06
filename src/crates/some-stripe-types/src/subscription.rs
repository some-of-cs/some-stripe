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

    /// The price associated with this item.
    pub price: Option<SubscriptionPrice>,

    /// Quantity of the plan to which the customer is subscribed.
    pub quantity: Option<u64>,

    /// Time at which the item was created (Unix timestamp).
    pub created: i64,

    /// Key-value metadata attached to the item.
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,

    /// Define thresholds at which an invoice will be sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_thresholds: Option<SubscriptionItemBillingThresholds>,

    /// Proration behavior applied to this item change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proration_behavior: Option<ProrationBehavior>,

    /// A list of tax rates applied to this item.
    #[serde(skip_serializing_if = "Option::is_none")]
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

/// Parameters for creating a subscription item directly via the API.
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
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub metadata: std::collections::HashMap<String, String>,

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
    pub fn metadata(mut self, metadata: std::collections::HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Sets an idempotency key.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for updating an existing subscription item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UpdateSubscriptionItemParams {
    /// The ID of the new price to subscribe to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,

    /// The new quantity of the subscription item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<u64>,

    /// Key-value metadata attached to the item.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub metadata: std::collections::HashMap<String, String>,

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
    pub fn metadata(mut self, metadata: std::collections::HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Sets an idempotency key.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for deleting a subscription item.
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

/// Parameters for listing subscription items.
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

/// An item entry within a multi-item subscription create or update request.
///
/// Encodes to RFC 1738 zero-indexed bracket notation such as `items[0][price]=...&items[0][quantity]=...`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SubscriptionItemParam {
    /// Subscription item ID (when updating or deleting an existing item on a subscription).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// The ID of the price for this item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,

    /// The quantity of the item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<u64>,

    /// Key-value metadata attached to this item.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub metadata: std::collections::HashMap<String, String>,

    /// Define thresholds at which an invoice will be sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_thresholds: Option<SubscriptionItemBillingThresholds>,

    /// A list of tax rates applied to this item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_rates: Option<Vec<String>>,

    /// Set to `true` to remove this item from the subscription (used in subscription update).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,

    /// Proration behavior for this specific item change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proration_behavior: Option<ProrationBehavior>,
}

impl SubscriptionItemParam {
    /// Creates a new subscription item parameter with the specified price.
    pub fn new(price: impl Into<String>) -> Self {
        Self {
            price: Some(price.into()),
            ..Default::default()
        }
    }

    /// Creates a new subscription item parameter with price and quantity.
    pub fn with_price_and_quantity(price: impl Into<String>, quantity: u64) -> Self {
        Self {
            price: Some(price.into()),
            quantity: Some(quantity),
            ..Default::default()
        }
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
    pub fn quantity(mut self, quantity: u64) -> Self {
        self.quantity = Some(quantity);
        self
    }

    /// Sets the metadata.
    pub fn metadata(mut self, metadata: std::collections::HashMap<String, String>) -> Self {
        self.metadata = metadata;
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

    /// Sets proration behavior.
    pub fn proration_behavior(mut self, behavior: ProrationBehavior) -> Self {
        self.proration_behavior = Some(behavior);
        self
    }
}

/// Parameters for creating a multi-item subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CreateSubscriptionParams {
    /// The ID of the customer who will be billed.
    pub customer: String,

    /// List of items on this subscription.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<SubscriptionItemParam>,

    /// Three-letter ISO currency code, in lowercase.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    /// Description of the subscription.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Key-value metadata attached to the subscription.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub metadata: std::collections::HashMap<String, String>,

    /// Proration behavior to apply.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proration_behavior: Option<ProrationBehavior>,

    /// Number of days in the trial period.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trial_period_days: Option<u32>,

    /// Unix timestamp representing the end of the trial period.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trial_end: Option<i64>,

    /// Optional default payment method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_payment_method: Option<String>,

    /// Optional idempotency key for this request.
    #[serde(skip_serializing)]
    pub idempotency_key: Option<String>,
}

impl CreateSubscriptionParams {
    /// Creates a new `CreateSubscriptionParams` for the given customer.
    pub fn new(customer: impl Into<String>) -> Self {
        Self {
            customer: customer.into(),
            ..Default::default()
        }
    }

    /// Adds an item to the subscription creation request.
    pub fn add_item(mut self, item: SubscriptionItemParam) -> Self {
        self.items.push(item);
        self
    }

    /// Sets the trial period in days.
    pub fn trial_period_days(mut self, days: u32) -> Self {
        self.trial_period_days = Some(days);
        self
    }

    /// Sets the trial end timestamp.
    pub fn trial_end(mut self, timestamp: i64) -> Self {
        self.trial_end = Some(timestamp);
        self
    }

    /// Sets the proration behavior.
    pub fn proration_behavior(mut self, behavior: ProrationBehavior) -> Self {
        self.proration_behavior = Some(behavior);
        self
    }

    /// Sets the currency.
    pub fn currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = Some(currency.into());
        self
    }

    /// Sets the default payment method.
    pub fn default_payment_method(mut self, pm: impl Into<String>) -> Self {
        self.default_payment_method = Some(pm.into());
        self
    }

    /// Sets the metadata.
    pub fn metadata(mut self, metadata: std::collections::HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Sets the idempotency key.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for updating an existing subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UpdateSubscriptionParams {
    /// List of items to add, modify, or remove on the subscription.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<SubscriptionItemParam>,

    /// Key-value metadata attached to the subscription.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub metadata: std::collections::HashMap<String, String>,

    /// Proration behavior to apply.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proration_behavior: Option<ProrationBehavior>,

    /// Optional default payment method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_payment_method: Option<String>,

    /// If `true`, cancel the subscription at the end of the current billing period.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_at_period_end: Option<bool>,

    /// Optional idempotency key for this request.
    #[serde(skip_serializing)]
    pub idempotency_key: Option<String>,
}

impl UpdateSubscriptionParams {
    /// Creates a new empty `UpdateSubscriptionParams`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an item modification or addition.
    pub fn add_item(mut self, item: SubscriptionItemParam) -> Self {
        self.items.push(item);
        self
    }

    /// Sets the proration behavior.
    pub fn proration_behavior(mut self, behavior: ProrationBehavior) -> Self {
        self.proration_behavior = Some(behavior);
        self
    }

    /// Sets `cancel_at_period_end`.
    pub fn cancel_at_period_end(mut self, cancel: bool) -> Self {
        self.cancel_at_period_end = Some(cancel);
        self
    }

    /// Sets the default payment method.
    pub fn default_payment_method(mut self, pm: impl Into<String>) -> Self {
        self.default_payment_method = Some(pm.into());
        self
    }

    /// Sets metadata.
    pub fn metadata(mut self, metadata: std::collections::HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Sets the idempotency key.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Paginated list of Subscription objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionList {
    /// Object type, always `list`.
    #[serde(default = "default_list_object")]
    pub object: String,

    /// The list of subscriptions.
    pub data: Vec<Subscription>,

    /// Whether there are more items available.
    #[serde(default)]
    pub has_more: bool,

    /// Total count of items, if returned by API.
    pub total_count: Option<u64>,

    /// URL for fetching the next page.
    pub url: String,
}

impl SubscriptionList {
    /// Returns a slice of the subscriptions.
    pub fn data(&self) -> &[Subscription] {
        &self.data
    }

    /// Returns `true` if there are no subscriptions.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the number of subscriptions.
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl SubscriptionItemList {
    /// Returns a slice of the subscription items.
    pub fn data(&self) -> &[SubscriptionItem] {
        &self.data
    }

    /// Returns `true` if there are no subscription items.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the number of subscription items.
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl Subscription {
    /// Zero-copy access to the subscription ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Zero-copy access to the customer ID.
    pub fn customer(&self) -> &str {
        &self.customer
    }

    /// Current subscription status.
    pub fn status(&self) -> SubscriptionStatus {
        self.status
    }
}

/// Extracts the ID from a [`SubscriptionItem`] for pagination cursors.
pub fn subscription_item_id(item: &SubscriptionItem) -> &str {
    &item.id
}
