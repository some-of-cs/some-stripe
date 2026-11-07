//! Stripe Subscription Schedules resource implementations.
//!
//! Provides bindings for managing Stripe Subscription Schedules (`stripe.subscription_schedules`),
//! enabling multi-phase subscription lifecycles, scheduled tier migrations, and controlled releases.
//!
//! # Architecture & Quality Gates:
//! - **G0 (Rust 2024 Edition)**: Clean compilation with 0 clippy warnings and strict code formatting.
//! - **G-Borrow (Zero-Clone Discipline)**: Zero-copy string slice accessors (`schedule.id()`, `schedule.customer()`)
//!   and streaming schedule iteration via [`Paginator`].
//! - **G2 (Security & Idempotency)**: Automatic injection of RFC 4122 UUID v4 `Idempotency-Key` headers
//!   on mutable operations (`create`, `update`, `release`, `cancel`) when omitted by the caller.
//! - **G2 (CRLF & Path Traversal Prevention)**: All IDs are validated and percent-encoded via [`validate_and_encode_id`].
//! - **Gate G-Law**: Multi-phase schedule transition validation and exact minor currency integer rules.
//!
//! # Quickstart
//! ```no_run
//! use some_stripe::StripeClient;
//! use some_stripe_types::{
//!     CreateSubscriptionScheduleParams, CreateSubscriptionSchedulePhaseParams,
//!     SubscriptionScheduleEndBehavior, SubscriptionSchedulePhaseItemParams,
//! };
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = StripeClient::new("sk_test_123");
//! let item = SubscriptionSchedulePhaseItemParams::new("price_pro_trial").quantity(1);
//! let phase = CreateSubscriptionSchedulePhaseParams::new(vec![item]).iterations(1);
//! let params = CreateSubscriptionScheduleParams::new()
//!     .customer("cus_123")
//!     .start_date_now()
//!     .add_phase(phase)
//!     .end_behavior(SubscriptionScheduleEndBehavior::Release);
//!
//! let schedule = client.subscription_schedules().create(&params).await?;
//! println!("Created schedule: {}", schedule.id());
//! # Ok(())
//! # }
//! ```

use some_stripe_core::patterns::pagination::Paginator;
use some_stripe_core::patterns::serializer::{encode_form_pairs, to_form_params};
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::subscription::{
    CancelSubscriptionScheduleParams, CreateSubscriptionScheduleParams,
    ListSubscriptionSchedulesParams, ReleaseSubscriptionScheduleParams, SubscriptionSchedule,
    UpdateSubscriptionScheduleParams, subscription_schedule_id,
};

/// Resource for managing Stripe Subscription Schedules (`stripe.subscription_schedules`).
///
/// Accessed via `client.subscription_schedules()`.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
///
/// let client = StripeClient::new("sk_test_123");
/// let schedules = client.subscription_schedules();
/// ```
#[derive(Clone, Copy)]
pub struct SubscriptionSchedulesResource<'a> {
    client: &'a StripeClient,
}

impl<'a> SubscriptionSchedulesResource<'a> {
    /// Creates a new `SubscriptionSchedulesResource` bound to the provided client.
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates a Subscription Schedule (`POST /v1/subscription_schedules`).
    ///
    /// Supports multi-phase configurations, trial specifications, and automatic
    /// injection of an RFC 4122 UUID v4 `Idempotency-Key` header when omitted.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::{
    ///     CreateSubscriptionScheduleParams, CreateSubscriptionSchedulePhaseParams,
    ///     SubscriptionSchedulePhaseItemParams,
    /// };
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let item = SubscriptionSchedulePhaseItemParams::new("price_monthly_plan").quantity(1);
    /// let phase = CreateSubscriptionSchedulePhaseParams::new(vec![item]).iterations(3);
    /// let params = CreateSubscriptionScheduleParams::new()
    ///     .customer("cus_123")
    ///     .start_date_now()
    ///     .add_phase(phase);
    ///
    /// let schedule = client.subscription_schedules().create(&params).await?;
    /// assert_eq!(schedule.customer(), "cus_123");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        params: &CreateSubscriptionScheduleParams,
    ) -> Result<SubscriptionSchedule, StripeError> {
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!("{}/v1/subscription_schedules", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let schedule: SubscriptionSchedule = serde_json::from_value(val)?;
        Ok(schedule)
    }

    /// Retrieves an existing Subscription Schedule by ID (`GET /v1/subscription_schedules/{id}`).
    ///
    /// Validates and percent-encodes the schedule identifier to prevent path traversal
    /// and CRLF injection vulnerabilities (Gate G2). Does not inject an idempotency key.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let schedule = client.subscription_schedules().retrieve("sub_sched_123").await?;
    /// println!("Schedule customer: {}", schedule.customer());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, schedule_id: &str) -> Result<SubscriptionSchedule, StripeError> {
        let encoded = validate_and_encode_id(schedule_id)?;
        let url = format!(
            "{}/v1/subscription_schedules/{}",
            self.client.config.api_base, encoded
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let schedule: SubscriptionSchedule = serde_json::from_value(val)?;
        Ok(schedule)
    }

    /// Updates an existing Subscription Schedule (`POST /v1/subscription_schedules/{id}`).
    ///
    /// Allows mutating phases, end behavior, and proration policies.
    /// Automatically injects an RFC 4122 UUID v4 `Idempotency-Key` header if omitted.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::{
    ///     CreateSubscriptionSchedulePhaseParams, ProrationBehavior,
    ///     SubscriptionScheduleEndBehavior, SubscriptionSchedulePhaseItemParams,
    ///     UpdateSubscriptionScheduleParams,
    /// };
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let item = SubscriptionSchedulePhaseItemParams::new("price_upgraded").quantity(2);
    /// let phase = CreateSubscriptionSchedulePhaseParams::new(vec![item]).iterations(6);
    /// let params = UpdateSubscriptionScheduleParams::new()
    ///     .phases(vec![phase])
    ///     .proration_behavior(ProrationBehavior::CreateProrations)
    ///     .end_behavior(SubscriptionScheduleEndBehavior::Release);
    ///
    /// let schedule = client.subscription_schedules().update("sub_sched_123", &params).await?;
    /// assert_eq!(schedule.id(), "sub_sched_123");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        schedule_id: &str,
        params: &UpdateSubscriptionScheduleParams,
    ) -> Result<SubscriptionSchedule, StripeError> {
        let encoded = validate_and_encode_id(schedule_id)?;
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!(
            "{}/v1/subscription_schedules/{}",
            self.client.config.api_base, encoded
        );
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let schedule: SubscriptionSchedule = serde_json::from_value(val)?;
        Ok(schedule)
    }

    /// Releases an existing Subscription Schedule (`POST /v1/subscription_schedules/{id}/release`).
    ///
    /// Once released, the underlying subscription continues running outside the schedule's management.
    /// Automatically injects an RFC 4122 UUID v4 `Idempotency-Key` header if omitted.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::ReleaseSubscriptionScheduleParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = ReleaseSubscriptionScheduleParams::new().preserve_cancel_date(true);
    /// let schedule = client.subscription_schedules().release("sub_sched_123", &params).await?;
    /// assert_eq!(schedule.status().as_str(), "released");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn release(
        &self,
        schedule_id: &str,
        params: &ReleaseSubscriptionScheduleParams,
    ) -> Result<SubscriptionSchedule, StripeError> {
        let encoded = validate_and_encode_id(schedule_id)?;
        let url = format!(
            "{}/v1/subscription_schedules/{}/release",
            self.client.config.api_base, encoded
        );
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let schedule: SubscriptionSchedule = serde_json::from_value(val)?;
        Ok(schedule)
    }

    /// Cancels an existing Subscription Schedule (`POST /v1/subscription_schedules/{id}/cancel`).
    ///
    /// Terminates the schedule and optionally invoices or prorates the underlying subscription.
    /// Automatically injects an RFC 4122 UUID v4 `Idempotency-Key` header if omitted.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::CancelSubscriptionScheduleParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CancelSubscriptionScheduleParams::new()
    ///     .invoice_now(true)
    ///     .prorate(true);
    /// let schedule = client.subscription_schedules().cancel("sub_sched_123", &params).await?;
    /// assert_eq!(schedule.status().as_str(), "canceled");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel(
        &self,
        schedule_id: &str,
        params: &CancelSubscriptionScheduleParams,
    ) -> Result<SubscriptionSchedule, StripeError> {
        let encoded = validate_and_encode_id(schedule_id)?;
        let url = format!(
            "{}/v1/subscription_schedules/{}/cancel",
            self.client.config.api_base, encoded
        );
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let schedule: SubscriptionSchedule = serde_json::from_value(val)?;
        Ok(schedule)
    }

    /// Returns a zero-clone [`Paginator`] stream for iterating over Subscription Schedules (`GET /v1/subscription_schedules`).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::ListSubscriptionSchedulesParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = ListSubscriptionSchedulesParams::new().limit(25);
    /// let mut stream = client.subscription_schedules().list(&params);
    /// while let Some(sched) = stream.next_item().await? {
    ///     println!("Schedule: {}", sched.id());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list(
        &self,
        params: &ListSubscriptionSchedulesParams,
    ) -> Paginator<'a, SubscriptionSchedule> {
        let (base_path, initial_err) = match to_form_params(params) {
            Ok(mut pairs) => {
                pairs
                    .retain(|(k, _)| k != "limit" && k != "starting_after" && k != "ending_before");
                let path = if pairs.is_empty() {
                    "v1/subscription_schedules".to_string()
                } else {
                    format!("v1/subscription_schedules?{}", encode_form_pairs(&pairs))
                };
                (path, None)
            }
            Err(e) => ("v1/subscription_schedules".to_string(), Some(e)),
        };

        let mut paginator = Paginator::new(
            self.client,
            base_path,
            params.limit.unwrap_or(10),
            subscription_schedule_id,
        );

        if let Some(err) = initial_err {
            paginator = paginator.with_initial_error(err);
        }
        if let Some(ref sa) = params.starting_after {
            paginator = paginator.starting_after(sa);
        }
        if let Some(ref eb) = params.ending_before {
            paginator = paginator.ending_before(eb);
        }

        paginator
    }

    /// Convenience helper returning an auto-paginator for schedules with default page limit (10).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let mut stream = client.subscription_schedules().list_default();
    /// while let Some(sched) = stream.next_item().await? {
    ///     println!("Schedule: {}", sched.id());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_default(&self) -> Paginator<'a, SubscriptionSchedule> {
        self.list(&ListSubscriptionSchedulesParams::default())
    }

    /// Convenience helper returning an auto-paginator for schedules with an explicit page limit.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let mut stream = client.subscription_schedules().list_with_limit(20);
    /// while let Some(sched) = stream.next_item().await? {
    ///     println!("Schedule: {}", sched.id());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_with_limit(&self, limit: u32) -> Paginator<'a, SubscriptionSchedule> {
        self.list(&ListSubscriptionSchedulesParams::new().limit(limit))
    }
}
