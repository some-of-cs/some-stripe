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
///
/// Indicates whether payment collection is paused, the collection behavior while paused,
/// and the optional timestamp at which collection will resume.
///
/// # Examples
///
/// ```
/// use some_stripe_types::subscription::PauseCollection;
///
/// let pause = PauseCollection::void();
/// assert!(pause.is_void());
/// assert!(!pause.is_keep_as_draft());
/// assert_eq!(pause.behavior, "void");
/// assert_eq!(pause.resumes_at, None);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PauseCollection {
    /// The payment collection behavior while paused.
    /// Values: `keep_as_draft`, `mark_uncollectible`, `void`.
    pub behavior: String,

    /// The time at which payment collection will resume.
    pub resumes_at: Option<i64>,
}

impl PauseCollection {
    /// Creates a new `PauseCollection` instance.
    pub fn new(behavior: impl Into<String>, resumes_at: Option<i64>) -> Self {
        Self {
            behavior: behavior.into(),
            resumes_at,
        }
    }

    /// Creates a `PauseCollection` with behavior set to `void`.
    pub fn void() -> Self {
        Self::new("void", None)
    }

    /// Creates a `PauseCollection` with behavior set to `mark_uncollectible`.
    pub fn mark_uncollectible() -> Self {
        Self::new("mark_uncollectible", None)
    }

    /// Creates a `PauseCollection` with behavior set to `keep_as_draft`.
    pub fn keep_as_draft() -> Self {
        Self::new("keep_as_draft", None)
    }

    /// Returns `true` if invoices are voided while paused.
    pub fn is_void(&self) -> bool {
        self.behavior == "void"
    }

    /// Returns `true` if invoices are marked uncollectible while paused.
    pub fn is_mark_uncollectible(&self) -> bool {
        self.behavior == "mark_uncollectible"
    }

    /// Returns `true` if invoices are kept as draft while paused.
    pub fn is_keep_as_draft(&self) -> bool {
        self.behavior == "keep_as_draft"
    }

    /// Returns `true` if a configured `resumes_at` has already passed relative to `current_time`.
    pub fn is_resumed(&self, current_time: i64) -> bool {
        match self.resumes_at {
            Some(resumes) => current_time >= resumes,
            None => false,
        }
    }
}

/// Validation errors for collection pause operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PauseCollectionError {
    /// The provided pause behavior string is unrecognized.
    InvalidBehavior(String),
    /// The configured `resumes_at` timestamp is not strictly in the future.
    ResumesAtMustBeFuture {
        /// The invalid resumes timestamp.
        resumes_at: i64,
        /// The current reference time.
        current_time: i64,
    },
}

impl std::fmt::Display for PauseCollectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBehavior(b) => write!(
                f,
                "Invalid pause collection behavior '{b}': must be 'keep_as_draft', 'mark_uncollectible', or 'void'"
            ),
            Self::ResumesAtMustBeFuture {
                resumes_at,
                current_time,
            } => write!(
                f,
                "resumes_at ({resumes_at}) must be strictly in the future relative to current time ({current_time})"
            ),
        }
    }
}

impl std::error::Error for PauseCollectionError {}

/// Parameters for pausing payment collection on a subscription.
///
/// Invariants:
/// - `behavior` must be one of: `"keep_as_draft"`, `"mark_uncollectible"`, `"void"`.
/// - `resumes_at`, if specified, must be a Unix timestamp in the future.
///
/// # Examples
///
/// ```
/// use some_stripe_types::subscription::PauseCollectionParams;
///
/// let params = PauseCollectionParams::void()
///     .with_resumes_at(1735689600);
/// assert!(params.validate().is_ok());
/// assert_eq!(params.behavior, "void");
/// assert_eq!(params.resumes_at, Some(1735689600));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PauseCollectionParams {
    /// The payment collection behavior while paused.
    /// Values: `keep_as_draft`, `mark_uncollectible`, `void`.
    pub behavior: String,

    /// The time at which payment collection will resume (Unix timestamp).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resumes_at: Option<i64>,
}

impl PauseCollectionParams {
    /// Creates a new `PauseCollectionParams` with the specified behavior.
    pub fn new(behavior: impl Into<String>) -> Self {
        Self {
            behavior: behavior.into(),
            resumes_at: None,
        }
    }

    /// Creates parameters to void invoices while paused.
    pub fn void() -> Self {
        Self::new("void")
    }

    /// Creates parameters to mark invoices uncollectible while paused.
    pub fn mark_uncollectible() -> Self {
        Self::new("mark_uncollectible")
    }

    /// Creates parameters to keep invoices as draft while paused.
    pub fn keep_as_draft() -> Self {
        Self::new("keep_as_draft")
    }

    /// Sets the optional `resumes_at` timestamp.
    pub fn with_resumes_at(mut self, resumes_at: i64) -> Self {
        self.resumes_at = Some(resumes_at);
        self
    }

    /// Validates the behavior invariant against supported Stripe behaviors.
    pub fn validate(&self) -> Result<(), PauseCollectionError> {
        match self.behavior.as_str() {
            "keep_as_draft" | "mark_uncollectible" | "void" => Ok(()),
            other => Err(PauseCollectionError::InvalidBehavior(other.to_string())),
        }
    }

    /// Validates both the behavior invariant and that `resumes_at` is strictly in the future.
    pub fn validate_with_time(&self, current_time: i64) -> Result<(), PauseCollectionError> {
        self.validate()?;
        if let Some(resumes) = self.resumes_at
            && resumes <= current_time
        {
            return Err(PauseCollectionError::ResumesAtMustBeFuture {
                resumes_at: resumes,
                current_time,
            });
        }
        Ok(())
    }
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

impl ProrationBehavior {
    /// Returns the static string representation of the proration behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// use some_stripe_types::subscription::ProrationBehavior;
    ///
    /// assert_eq!(ProrationBehavior::CreateProrations.as_str(), "create_prorations");
    /// assert_eq!(ProrationBehavior::None.as_str(), "none");
    /// assert_eq!(ProrationBehavior::AlwaysInvoice.as_str(), "always_invoice");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CreateProrations => "create_prorations",
            Self::None => "none",
            Self::AlwaysInvoice => "always_invoice",
        }
    }

    /// Returns `true` if proration calculations or invoices are generated.
    ///
    /// # Examples
    ///
    /// ```
    /// use some_stripe_types::subscription::ProrationBehavior;
    ///
    /// assert!(ProrationBehavior::CreateProrations.is_proration_enabled());
    /// assert!(ProrationBehavior::AlwaysInvoice.is_proration_enabled());
    /// assert!(!ProrationBehavior::None.is_proration_enabled());
    /// ```
    pub fn is_proration_enabled(&self) -> bool {
        matches!(self, Self::CreateProrations | Self::AlwaysInvoice)
    }
}

impl std::fmt::Display for ProrationBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ---------------------------------------------------------------------------
// ISO 4217 Minor Currency Unit In-Flight Proration Math (Gate G-Law & G-Borrow)
// ---------------------------------------------------------------------------

/// Errors encountered during in-flight subscription plan proration calculation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProrationError {
    /// The billing cycle end timestamp is not strictly greater than start timestamp.
    InvalidBillingPeriod {
        /// Cycle start timestamp.
        period_start: i64,
        /// Cycle end timestamp.
        period_end: i64,
    },
    /// The proration event timestamp is outside the active billing period.
    InvalidProrationDate {
        /// Evaluated modification timestamp.
        proration_date: i64,
        /// Cycle start timestamp.
        period_start: i64,
        /// Cycle end timestamp.
        period_end: i64,
    },
    /// One or more unit amounts are negative.
    NegativeUnitAmount {
        /// Original plan price in minor units.
        original: i64,
        /// New plan price in minor units.
        new: i64,
    },
    /// The provided currency ISO string is empty.
    EmptyCurrency,
    /// Arithmetic overflow occurred during 128-bit integer computation.
    ArithmeticOverflow,
}

impl std::fmt::Display for ProrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBillingPeriod {
                period_start,
                period_end,
            } => write!(
                f,
                "Invalid billing period: period_end ({period_end}) must be strictly greater than period_start ({period_start})"
            ),
            Self::InvalidProrationDate {
                proration_date,
                period_start,
                period_end,
            } => write!(
                f,
                "Proration date ({proration_date}) is outside billing period [{period_start}, {period_end}]"
            ),
            Self::NegativeUnitAmount { original, new } => write!(
                f,
                "Unit amounts must be non-negative: original={original}, new={new}"
            ),
            Self::EmptyCurrency => write!(f, "Currency code cannot be empty"),
            Self::ArithmeticOverflow => {
                write!(
                    f,
                    "Arithmetic overflow occurred during proration computation"
                )
            }
        }
    }
}

impl std::error::Error for ProrationError {}

/// Detailed result of an in-flight subscription plan proration calculation conforming to ISO 4217.
///
/// Invariants:
/// - All currency values are exact minor currency units (cents, etc.). Zero floating point arithmetic is used.
/// - `unspent_credit` represents the unused value of the old plan for the remaining duration of the cycle.
/// - `prorated_charge` represents the cost of the new plan for the remaining duration of the cycle.
/// - `net_amount = prorated_charge - unspent_credit`:
///   - Positive (> 0): Net additional charge due immediately (plan upgrade).
///   - Negative (< 0): Net unspent credit added to customer balance (plan downgrade).
///   - Zero (= 0): No net balance change.
///
/// # Examples
///
/// ```
/// use some_stripe_types::subscription::calculate_proration;
///
/// // Upgrade from $20/month (2000 cents) to $50/month (5000 cents) at midpoint of a 30-day billing period
/// let start = 1704067200; // Day 0
/// let end = start + 30 * 86400; // Day 30
/// let proration_date = start + 15 * 86400; // Day 15
///
/// let result = calculate_proration(2000, 5000, start, end, proration_date, "usd").unwrap();
/// assert_eq!(result.unspent_credit, 1000); // $10.00 credit for unused old plan
/// assert_eq!(result.prorated_charge, 2500); // $25.00 charge for new plan
/// assert_eq!(result.net_amount, 1500); // $15.00 net charge due
/// assert!(result.is_upgrade());
/// assert!(result.is_net_charge());
/// assert_eq!(result.currency, "usd");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProrationCalculation {
    /// Original plan unit amount in minor currency units (e.g., cents).
    pub original_unit_amount: i64,
    /// New plan unit amount in minor currency units.
    pub new_unit_amount: i64,
    /// Billing cycle start timestamp (Unix timestamp).
    pub period_start: i64,
    /// Billing cycle end timestamp (Unix timestamp).
    pub period_end: i64,
    /// Plan modification / proration timestamp (Unix timestamp).
    pub proration_date: i64,
    /// Elapsed seconds within the current billing period.
    pub elapsed_seconds: i64,
    /// Remaining seconds within the current billing period.
    pub remaining_seconds: i64,
    /// Total duration of the current billing period in seconds.
    pub total_seconds: i64,
    /// Credit due to the customer for the unused portion of the original plan.
    pub unspent_credit: i64,
    /// Prorated charge for the remaining portion of the new plan.
    pub prorated_charge: i64,
    /// Net amount due (`prorated_charge - unspent_credit`). Positive indicates charge; negative indicates credit.
    pub net_amount: i64,
    /// Three-letter ISO currency code (lowercase).
    pub currency: String,
}

impl ProrationCalculation {
    /// Returns `true` if the new plan unit amount is greater than the original plan unit amount.
    pub fn is_upgrade(&self) -> bool {
        self.new_unit_amount > self.original_unit_amount
    }

    /// Returns `true` if the new plan unit amount is less than the original plan unit amount.
    pub fn is_downgrade(&self) -> bool {
        self.new_unit_amount < self.original_unit_amount
    }

    /// Returns `true` if the net balance change requires an immediate charge (`net_amount > 0`).
    pub fn is_net_charge(&self) -> bool {
        self.net_amount > 0
    }

    /// Returns `true` if the net balance change results in a customer balance credit (`net_amount < 0`).
    pub fn is_net_credit(&self) -> bool {
        self.net_amount < 0
    }

    /// Returns the absolute value of the credit when `net_amount < 0`, or 0 otherwise.
    pub fn net_credit_amount(&self) -> i64 {
        if self.net_amount < 0 {
            -self.net_amount
        } else {
            0
        }
    }
}

/// Computes in-flight plan migration proration math conforming strictly to ISO 4217 minor currency units.
///
/// Invariants:
/// - Guarantees zero floating point representation and zero fractional rounding leakage by
///   utilizing 128-bit integer intermediate multiplication.
/// - Validates bounds: `period_end > period_start`, `period_start <= proration_date <= period_end`,
///   and non-negative unit amounts.
///
/// # Examples
///
/// ```
/// use some_stripe_types::subscription::calculate_proration;
///
/// let start = 1704067200;
/// let end = start + 30 * 86400;
/// let proration_date = start + 10 * 86400; // 20 days remaining out of 30
///
/// // Downgrade from $90/mo to $30/mo
/// let proration = calculate_proration(9000, 3000, start, end, proration_date, "usd").unwrap();
/// assert_eq!(proration.unspent_credit, 6000); // 2/3 of $90 = $60
/// assert_eq!(proration.prorated_charge, 2000); // 2/3 of $30 = $20
/// assert_eq!(proration.net_amount, -4000); // $40 credit due to customer
/// assert!(proration.is_downgrade());
/// assert!(proration.is_net_credit());
/// assert_eq!(proration.net_credit_amount(), 4000);
/// ```
pub fn calculate_proration(
    original_unit_amount: i64,
    new_unit_amount: i64,
    period_start: i64,
    period_end: i64,
    proration_date: i64,
    currency: impl Into<String>,
) -> Result<ProrationCalculation, ProrationError> {
    let curr = currency.into().trim().to_lowercase();
    if curr.is_empty() {
        return Err(ProrationError::EmptyCurrency);
    }

    if period_end <= period_start {
        return Err(ProrationError::InvalidBillingPeriod {
            period_start,
            period_end,
        });
    }

    if proration_date < period_start || proration_date > period_end {
        return Err(ProrationError::InvalidProrationDate {
            proration_date,
            period_start,
            period_end,
        });
    }

    if original_unit_amount < 0 || new_unit_amount < 0 {
        return Err(ProrationError::NegativeUnitAmount {
            original: original_unit_amount,
            new: new_unit_amount,
        });
    }

    let total_seconds = period_end - period_start;
    let remaining_seconds = period_end - proration_date;
    let elapsed_seconds = proration_date - period_start;

    // Use i128 for intermediate arithmetic to eliminate any overflow possibility
    let unspent_credit_128 =
        ((original_unit_amount as i128) * (remaining_seconds as i128)) / (total_seconds as i128);
    let prorated_charge_128 =
        ((new_unit_amount as i128) * (remaining_seconds as i128)) / (total_seconds as i128);

    let unspent_credit =
        i64::try_from(unspent_credit_128).map_err(|_| ProrationError::ArithmeticOverflow)?;
    let prorated_charge =
        i64::try_from(prorated_charge_128).map_err(|_| ProrationError::ArithmeticOverflow)?;
    let net_amount = prorated_charge - unspent_credit;

    Ok(ProrationCalculation {
        original_unit_amount,
        new_unit_amount,
        period_start,
        period_end,
        proration_date,
        elapsed_seconds,
        remaining_seconds,
        total_seconds,
        unspent_credit,
        prorated_charge,
        net_amount,
        currency: curr,
    })
}

// ---------------------------------------------------------------------------
// 14-Day Free Trials and Trial Bounds Calculators
// ---------------------------------------------------------------------------

/// Number of seconds in a standard 24-hour solar day (86,400 seconds).
pub const SECONDS_PER_DAY: i64 = 86_400;

/// Default free trial duration: 14 days in seconds (1,209,600 seconds).
pub const FOURTEEN_DAYS_SECONDS: i64 = 14 * SECONDS_PER_DAY;

/// Minimum allowable trial duration in seconds under Stripe live mode rules (48 hours = 172,800 seconds).
pub const MIN_TRIAL_DURATION_SECONDS: i64 = 48 * 3600;

/// Errors encountered during trial period calculation or validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrialBoundsError {
    /// `trial_period_days` must be at least 1 day.
    InvalidTrialPeriodDays(u32),
    /// `trial_end` must be strictly in the future relative to the start / reference time.
    TrialEndMustBeFuture {
        /// Reference start or current timestamp.
        start_time: i64,
        /// Provided trial end timestamp.
        trial_end: i64,
    },
    /// The trial duration is shorter than the minimum statutory or API limit.
    TrialDurationTooShort {
        /// Actual duration in seconds.
        duration_seconds: i64,
        /// Minimum required duration in seconds.
        min_seconds: i64,
    },
}

impl std::fmt::Display for TrialBoundsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTrialPeriodDays(days) => {
                write!(f, "trial_period_days must be at least 1 day, got {days}")
            }
            Self::TrialEndMustBeFuture {
                start_time,
                trial_end,
            } => write!(
                f,
                "trial_end ({trial_end}) must be strictly in the future relative to start time ({start_time})"
            ),
            Self::TrialDurationTooShort {
                duration_seconds,
                min_seconds,
            } => write!(
                f,
                "trial duration ({duration_seconds}s) is shorter than minimum required ({min_seconds}s)"
            ),
        }
    }
}

impl std::error::Error for TrialBoundsError {}

/// Calculates the Unix timestamp for the end of a free trial given a start timestamp and trial duration in days.
///
/// # Examples
///
/// ```
/// use some_stripe_types::subscription::calculate_trial_end_from_days;
///
/// let start = 1704067200;
/// let trial_end = calculate_trial_end_from_days(start, 14).unwrap();
/// assert_eq!(trial_end, start + 14 * 86400);
/// assert!(calculate_trial_end_from_days(start, 0).is_err());
/// ```
pub fn calculate_trial_end_from_days(
    start_time: i64,
    trial_period_days: u32,
) -> Result<i64, TrialBoundsError> {
    if trial_period_days == 0 {
        return Err(TrialBoundsError::InvalidTrialPeriodDays(0));
    }
    let added_seconds = (trial_period_days as i64) * SECONDS_PER_DAY;
    Ok(start_time + added_seconds)
}

/// Convenience calculator for a standard 14-day free trial.
///
/// # Examples
///
/// ```
/// use some_stripe_types::subscription::{calculate_14_day_trial_end, FOURTEEN_DAYS_SECONDS};
///
/// let start = 1704067200;
/// assert_eq!(calculate_14_day_trial_end(start), start + FOURTEEN_DAYS_SECONDS);
/// ```
pub fn calculate_14_day_trial_end(start_time: i64) -> i64 {
    start_time + FOURTEEN_DAYS_SECONDS
}

/// Validates that a proposed `trial_end` timestamp satisfies Stripe future constraints and optional minimum duration.
///
/// # Examples
///
/// ```
/// use some_stripe_types::subscription::validate_trial_bounds;
///
/// let now = 1704067200;
/// let valid_end = now + 14 * 86400;
/// assert!(validate_trial_bounds(now, valid_end, Some(48 * 3600)).is_ok());
///
/// let past_end = now - 100;
/// assert!(validate_trial_bounds(now, past_end, None).is_err());
/// ```
pub fn validate_trial_bounds(
    current_time: i64,
    trial_end: i64,
    min_duration_seconds: Option<i64>,
) -> Result<(), TrialBoundsError> {
    if trial_end <= current_time {
        return Err(TrialBoundsError::TrialEndMustBeFuture {
            start_time: current_time,
            trial_end,
        });
    }

    if let Some(min_sec) = min_duration_seconds {
        let duration = trial_end - current_time;
        if duration < min_sec {
            return Err(TrialBoundsError::TrialDurationTooShort {
                duration_seconds: duration,
                min_seconds: min_sec,
            });
        }
    }

    Ok(())
}

/// Calculates the number of full days represented by a trial period.
///
/// # Examples
///
/// ```
/// use some_stripe_types::subscription::calculate_trial_period_days;
///
/// let start = 1704067200;
/// let end = start + 14 * 86400;
/// assert_eq!(calculate_trial_period_days(start, end).unwrap(), 14);
/// ```
pub fn calculate_trial_period_days(
    start_time: i64,
    trial_end: i64,
) -> Result<u32, TrialBoundsError> {
    if trial_end <= start_time {
        return Err(TrialBoundsError::TrialEndMustBeFuture {
            start_time,
            trial_end,
        });
    }
    let duration = trial_end - start_time;
    Ok((duration / SECONDS_PER_DAY) as u32)
}

/// Determines whether a subscription is currently within its trial period.
///
/// # Examples
///
/// ```
/// use some_stripe_types::subscription::is_subscription_in_trial;
///
/// let start = 1704067200;
/// let end = start + 14 * 86400;
///
/// assert!(is_subscription_in_trial(start + 100, Some(start), Some(end)));
/// assert!(!is_subscription_in_trial(end + 10, Some(start), Some(end)));
/// assert!(!is_subscription_in_trial(start, None, None));
/// ```
pub fn is_subscription_in_trial(
    current_time: i64,
    trial_start: Option<i64>,
    trial_end: Option<i64>,
) -> bool {
    match (trial_start, trial_end) {
        (Some(start), Some(end)) => current_time >= start && current_time < end,
        (None, Some(end)) => current_time < end,
        _ => false,
    }
}

/// Parameters for previewing an in-flight subscription plan proration.
///
/// Supports either local exact ISO 4217 minor currency calculation or wire previewing via Stripe.
///
/// # Examples
///
/// ```
/// use some_stripe_types::subscription::{PreviewProrationParams, ProrationBehavior};
///
/// let params = PreviewProrationParams::new()
///     .subscription("sub_12345")
///     .proration_behavior(ProrationBehavior::CreateProrations)
///     .amounts(2000, 5000, "usd")
///     .period(1704067200, 1704067200 + 30 * 86400)
///     .proration_date(1704067200 + 15 * 86400);
///
/// assert_eq!(params.original_unit_amount, Some(2000));
/// assert_eq!(params.new_unit_amount, Some(5000));
/// assert_eq!(params.currency.as_deref(), Some("usd"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PreviewProrationParams {
    /// The target subscription ID to preview plan changes on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription: Option<String>,

    /// Customer ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,

    /// Specific timestamp at which to evaluate proration. If omitted, current time is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proration_date: Option<i64>,

    /// Proration behavior (`create_prorations`, `none`, `always_invoice`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proration_behavior: Option<ProrationBehavior>,

    /// Original plan unit amount in minor currency units (for local calculation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_unit_amount: Option<i64>,

    /// New plan unit amount in minor currency units.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_unit_amount: Option<i64>,

    /// Billing period start timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_start: Option<i64>,

    /// Billing period end timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_end: Option<i64>,

    /// Three-letter ISO currency code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
}

impl PreviewProrationParams {
    /// Creates an empty `PreviewProrationParams` builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the subscription ID.
    pub fn subscription(mut self, id: impl Into<String>) -> Self {
        self.subscription = Some(id.into());
        self
    }

    /// Sets the customer ID.
    pub fn customer(mut self, id: impl Into<String>) -> Self {
        self.customer = Some(id.into());
        self
    }

    /// Sets the proration calculation date timestamp.
    pub fn proration_date(mut self, ts: i64) -> Self {
        self.proration_date = Some(ts);
        self
    }

    /// Sets the proration behavior.
    pub fn proration_behavior(mut self, pb: ProrationBehavior) -> Self {
        self.proration_behavior = Some(pb);
        self
    }

    /// Sets the original and new unit amounts in minor currency units and the ISO currency code.
    pub fn amounts(mut self, original: i64, new_amount: i64, currency: impl Into<String>) -> Self {
        self.original_unit_amount = Some(original);
        self.new_unit_amount = Some(new_amount);
        self.currency = Some(currency.into());
        self
    }

    /// Sets the new unit amount in minor currency units.
    pub fn new_unit_amount(mut self, amt: i64) -> Self {
        self.new_unit_amount = Some(amt);
        self
    }

    /// Sets the original unit amount in minor currency units.
    pub fn original_unit_amount(mut self, amt: i64) -> Self {
        self.original_unit_amount = Some(amt);
        self
    }

    /// Sets the ISO currency code.
    pub fn currency(mut self, curr: impl Into<String>) -> Self {
        self.currency = Some(curr.into());
        self
    }

    /// Sets the billing period start and end timestamps.
    pub fn period(mut self, start: i64, end: i64) -> Self {
        self.period_start = Some(start);
        self.period_end = Some(end);
        self
    }
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
