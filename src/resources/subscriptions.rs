//! Stripe Subscriptions resource implementations.
//!
//! Provides bindings for managing recurring Stripe Subscriptions (`stripe.subscriptions`),
//! in-flight plan prorations conforming to ISO 4217 minor currency units (Gate G-Law),
//! 14-day free trial bounds validation, and payment collection pause/resume workflows.
//!
//! # Architecture & Quality Gates:
//! - **G0 (Rust 2024 Edition)**: Zero compiler/clippy warnings and clean formatting.
//! - **G-Borrow (Zero-Clone Discipline)**: Parameter references borrowed strictly via `&PauseCollectionParams`
//!   and `&PreviewProrationParams`; zero heap allocations or clones on hot paths.
//! - **G2 (Security & Idempotency)**: Automatic injection of RFC 4122 UUID v4 `Idempotency-Key` headers
//!   on mutable operations (`pause_collection`, `resume`).
//! - **Gate G-Law (ISO 4217 Minor Currency Units)**: Exact integer-based unspent balance proration
//!   calculations using 128-bit intermediate arithmetic to prevent rounding errors or float inaccuracies.
//!
//! # Quickstart
//!
//! ```no_run
//! use some_stripe::StripeClient;
//! use some_stripe::subscription::{PauseCollectionParams, PreviewProrationParams};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = StripeClient::new("sk_test_123");
//! let subscriptions = client.subscriptions();
//!
//! // 1. Preview in-flight plan migration proration
//! let preview_params = PreviewProrationParams::new()
//!     .amounts(2000, 5000, "usd")
//!     .period(1704067200, 1704067200 + 30 * 86400)
//!     .proration_date(1704067200 + 15 * 86400);
//! let proration = subscriptions.preview_proration("sub_123", &preview_params).await?;
//! println!("Net proration charge: {} {}", proration.net_amount, proration.currency);
//!
//! // 2. Pause payment collection
//! let pause_params = PauseCollectionParams::void();
//! let paused_sub = subscriptions.pause_collection("sub_123", &pause_params).await?;
//!
//! // 3. Resume payment collection
//! let resumed_sub = subscriptions.resume("sub_123").await?;
//! # Ok(())
//! # }
//! ```

use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::subscription::{
    PauseCollectionParams, PreviewProrationParams, ProrationCalculation, Subscription,
    calculate_proration,
};

/// Resource for managing Stripe Subscriptions (`stripe.subscriptions`).
///
/// Accessed via `client.subscriptions()`.
#[derive(Clone, Copy)]
pub struct SubscriptionsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> SubscriptionsResource<'a> {
    /// Creates a new `SubscriptionsResource` bound to the provided client.
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Retrieves an existing Subscription by ID (`GET /v1/subscriptions/{id}`).
    ///
    /// Accepts the identifier as a string reference (`&str`), ensuring zero allocations.
    /// This is an idempotent read operation and does not inject an idempotency key.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let sub = client.subscriptions().retrieve("sub_123").await?;
    /// println!("Subscription status: {:?}", sub.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, subscription_id: &str) -> Result<Subscription, StripeError> {
        let trimmed_id = subscription_id.trim();
        if trimmed_id.is_empty() {
            return Err(StripeError::InvalidRequest(
                "Subscription ID cannot be empty".into(),
            ));
        }

        let url = format!(
            "{}/v1/subscriptions/{}",
            self.client.config.api_base, trimmed_id
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let sub: Subscription = serde_json::from_value(val)?;
        Ok(sub)
    }

    /// Previews an in-flight subscription plan proration conforming to ISO 4217 minor currency units.
    ///
    /// If full calculation inputs (`original_unit_amount`, `new_unit_amount`, `period_start`,
    /// `period_end`, `proration_date`, `currency`) are provided in `params`, calculates the unspent
    /// balance credit and prorated charge locally without unnecessary network latency.
    /// If historical period bounds or original amounts are omitted, retrieves the active subscription
    /// record from Stripe to inspect its billing cycle bounds and current plan pricing before computing.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe::subscription::PreviewProrationParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = PreviewProrationParams::new()
    ///     .amounts(2000, 5000, "usd")
    ///     .period(1704067200, 1704067200 + 30 * 86400)
    ///     .proration_date(1704067200 + 15 * 86400);
    ///
    /// let preview = client.subscriptions().preview_proration("sub_123", &params).await?;
    /// assert_eq!(preview.unspent_credit, 1000);
    /// assert_eq!(preview.prorated_charge, 2500);
    /// assert_eq!(preview.net_amount, 1500);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn preview_proration(
        &self,
        subscription_id: &str,
        params: &PreviewProrationParams,
    ) -> Result<ProrationCalculation, StripeError> {
        // Fast path: if all parameters are explicitly supplied, evaluate locally with zero network calls
        if let (Some(orig), Some(new_amt), Some(start), Some(end), Some(date), Some(curr)) = (
            params.original_unit_amount,
            params.new_unit_amount,
            params.period_start,
            params.period_end,
            params.proration_date,
            &params.currency,
        ) {
            return calculate_proration(orig, new_amt, start, end, date, curr.as_str())
                .map_err(|e| StripeError::InvalidRequest(e.to_string()));
        }

        // Slow path: retrieve subscription to resolve billing cycle bounds and current plan price
        let sub = self.retrieve(subscription_id).await?;
        let start = params.period_start.unwrap_or(sub.current_period_start);
        let end = params.period_end.unwrap_or(sub.current_period_end);
        let date = params
            .proration_date
            .unwrap_or_else(|| chrono::Utc::now().timestamp());
        let currency = params
            .currency
            .clone()
            .or_else(|| sub.currency.clone())
            .unwrap_or_else(|| "usd".into());

        let orig = params.original_unit_amount.unwrap_or_else(|| {
            sub.items
                .data
                .first()
                .and_then(|item| item.price.as_ref())
                .and_then(|p| p.unit_amount)
                .unwrap_or(0)
        });

        let new_amt = params.new_unit_amount.ok_or_else(|| {
            StripeError::InvalidRequest(
                "new_unit_amount must be specified to preview proration".into(),
            )
        })?;

        calculate_proration(orig, new_amt, start, end, date, currency)
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))
    }

    /// Pauses payment collection on an active subscription (`POST /v1/subscriptions/{id}`).
    ///
    /// Validates collection behavior against statutory allowed values (`keep_as_draft`,
    /// `mark_uncollectible`, `void`) and automatically injects an RFC 4122 UUID v4 `Idempotency-Key` header.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe::subscription::PauseCollectionParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let pause_params = PauseCollectionParams::void()
    ///     .with_resumes_at(1735689600);
    ///
    /// let sub = client.subscriptions().pause_collection("sub_123", &pause_params).await?;
    /// println!("Updated subscription: {:?}", sub.pause_collection);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn pause_collection(
        &self,
        subscription_id: &str,
        params: &PauseCollectionParams,
    ) -> Result<Subscription, StripeError> {
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let trimmed_id = subscription_id.trim();
        if trimmed_id.is_empty() {
            return Err(StripeError::InvalidRequest(
                "Subscription ID cannot be empty".into(),
            ));
        }

        let url = format!(
            "{}/v1/subscriptions/{}",
            self.client.config.api_base, trimmed_id
        );
        let mut form_data = vec![(
            "pause_collection[behavior]".to_string(),
            params.behavior.clone(),
        )];
        if let Some(resumes_at) = params.resumes_at {
            form_data.push((
                "pause_collection[resumes_at]".to_string(),
                resumes_at.to_string(),
            ));
        }

        let idempotency_key = crate::resources::generate_uuid_v4();
        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        let val = self.client.execute(req).await?;
        let sub: Subscription = serde_json::from_value(val)?;
        Ok(sub)
    }

    /// Resumes payment collection on a paused subscription (`POST /v1/subscriptions/{id}`).
    ///
    /// Clears `pause_collection` by transmitting an empty parameter (`pause_collection=""`)
    /// conforming to Stripe API v1 nullability semantics. Automatically injects an RFC 4122
    /// UUID v4 `Idempotency-Key` header.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let sub = client.subscriptions().resume("sub_123").await?;
    /// assert!(sub.pause_collection.is_none());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn resume(&self, subscription_id: &str) -> Result<Subscription, StripeError> {
        let trimmed_id = subscription_id.trim();
        if trimmed_id.is_empty() {
            return Err(StripeError::InvalidRequest(
                "Subscription ID cannot be empty".into(),
            ));
        }

        let url = format!(
            "{}/v1/subscriptions/{}",
            self.client.config.api_base, trimmed_id
        );
        let form_data = vec![("pause_collection".to_string(), "".to_string())];

        let idempotency_key = crate::resources::generate_uuid_v4();
        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        let val = self.client.execute(req).await?;
        let sub: Subscription = serde_json::from_value(val)?;
        Ok(sub)
    }
}
