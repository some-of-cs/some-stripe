//! Stripe API V2 Billing Meters resource implementation (`stripe.v2.billing.meters`).
//!
//! Provides endpoints to create, retrieve, update, deactivate, reactivate, and list
//! V2 billing meters. Billing meters ingest and aggregate usage events into billable metric totals.
//!
//! # Architecture & Quality Gates:
//! - **Gate G-Borrow (Zero-Clone Discipline)**: Parameter borrowing by reference (`&CreateBillingMeterParams`),
//!   zero deep `.clone()` calls on entities in request pipelines, and borrowed string slices for accessors.
//! - **Gate G2 (Security & Idempotency)**: Automatic injection of RFC 4122 UUID v4 `Idempotency-Key` headers
//!   on mutable operations (`create`, `update`, `deactivate`, `reactivate`) when omitted by the caller
//!   via [`crate::resources::resolve_idempotency_key`].
//! - **Security Invariant**: Strict path traversal prevention via [`validate_and_encode_id`].
//! - **Gate G-Complexity**: Cyclomatic complexity <= 6 across all resource methods.
//! - **Gate G-Docs**: Runnable doc-tests verified by `cargo test --doc`.

use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CreateBillingMeterParams, ListBillingMetersParams, UpdateBillingMeterParams, V2BillingMeter,
    V2BillingMeterList,
};

/// Resource for managing Stripe API V2 Billing Meters (`stripe.v2.billing.meters`).
///
/// Enables configuring event aggregation rules, customer ID bindings, and meter lifecycle state.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
/// use some_stripe::resources::BillingMetersResource;
/// use some_stripe_types::{
///     AggregationFormula, CreateBillingMeterParams, CustomerMappingSettings, ValueSettings,
/// };
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let client = StripeClient::new("sk_test_123");
/// let resource = BillingMetersResource::new(&client);
///
/// let params = CreateBillingMeterParams::new(
///     "AI Token Ingestion",
///     "llm_tokens",
///     AggregationFormula::Sum,
///     ValueSettings::new("tokens"),
///     CustomerMappingSettings::by_id("stripe_customer_id"),
/// );
///
/// let meter = resource.create(&params).await?;
/// println!("Created meter: {}", meter.id);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct BillingMetersResource<'a> {
    client: &'a StripeClient,
}

impl<'a> BillingMetersResource<'a> {
    /// Creates a new `BillingMetersResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates a new V2 Billing Meter (`POST /v2/billing/meters`).
    ///
    /// Configures event aggregation, value payload extraction, and customer mapping.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe::resources::BillingMetersResource;
    /// use some_stripe_types::{
    ///     AggregationFormula, CreateBillingMeterParams, CustomerMappingSettings, ValueSettings,
    /// };
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let resource = BillingMetersResource::new(&client);
    /// let params = CreateBillingMeterParams::new(
    ///     "Compute Seconds",
    ///     "compute_sec",
    ///     AggregationFormula::Sum,
    ///     ValueSettings::new("seconds"),
    ///     CustomerMappingSettings::by_id("customer_id"),
    /// );
    ///
    /// let meter = resource.create(&params).await?;
    /// assert_eq!(meter.event_name, "compute_sec");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        params: &CreateBillingMeterParams,
    ) -> Result<V2BillingMeter, StripeError> {
        let url = format!("{}/v2/billing/meters", self.client.config.api_base);
        let idempotency_key =
            crate::resources::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .header("Idempotency-Key", &*idempotency_key)
            .json(params);

        let val = self.client.execute(req).await?;
        let meter: V2BillingMeter = serde_json::from_value(val)?;
        Ok(meter)
    }

    /// Retrieves an existing V2 Billing Meter (`GET /v2/billing/meters/{id}`).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe::resources::BillingMetersResource;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let resource = BillingMetersResource::new(&client);
    ///
    /// let meter = resource.retrieve("mtr_12345").await?;
    /// println!("Retrieved meter: {}", meter.display_name);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, id: &str) -> Result<V2BillingMeter, StripeError> {
        let safe_id = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/billing/meters/{}",
            self.client.config.api_base, safe_id
        );

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let meter: V2BillingMeter = serde_json::from_value(val)?;
        Ok(meter)
    }

    /// Updates an existing V2 Billing Meter (`POST /v2/billing/meters/{id}`).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe::resources::BillingMetersResource;
    /// use some_stripe_types::UpdateBillingMeterParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let resource = BillingMetersResource::new(&client);
    /// let params = UpdateBillingMeterParams::new().display_name("Updated Compute Meter");
    ///
    /// let meter = resource.update("mtr_12345", &params).await?;
    /// assert_eq!(meter.display_name, "Updated Compute Meter");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        id: &str,
        params: &UpdateBillingMeterParams,
    ) -> Result<V2BillingMeter, StripeError> {
        let safe_id = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/billing/meters/{}",
            self.client.config.api_base, safe_id
        );
        let idempotency_key =
            crate::resources::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .header("Idempotency-Key", &*idempotency_key)
            .json(params);

        let val = self.client.execute(req).await?;
        let meter: V2BillingMeter = serde_json::from_value(val)?;
        Ok(meter)
    }

    /// Deactivates an active V2 Billing Meter (`POST /v2/billing/meters/{id}/deactivate`).
    ///
    /// When deactivated, the meter ceases accepting new usage events.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe::resources::BillingMetersResource;
    /// use some_stripe_types::MeterStatus;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let resource = BillingMetersResource::new(&client);
    ///
    /// let meter = resource.deactivate("mtr_12345").await?;
    /// assert_eq!(meter.status, MeterStatus::Inactive);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn deactivate(&self, id: &str) -> Result<V2BillingMeter, StripeError> {
        let safe_id = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/billing/meters/{}/deactivate",
            self.client.config.api_base, safe_id
        );
        let idempotency_key = crate::resources::resolve_idempotency_key(None);

        let req = self
            .client
            .http
            .post(&url)
            .header("Idempotency-Key", &*idempotency_key)
            .header("Content-Length", "0");

        let val = self.client.execute(req).await?;
        let meter: V2BillingMeter = serde_json::from_value(val)?;
        Ok(meter)
    }

    /// Reactivates an inactive V2 Billing Meter (`POST /v2/billing/meters/{id}/reactivate`).
    ///
    /// When reactivated, the meter resumes accepting and aggregating usage events.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe::resources::BillingMetersResource;
    /// use some_stripe_types::MeterStatus;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let resource = BillingMetersResource::new(&client);
    ///
    /// let meter = resource.reactivate("mtr_12345").await?;
    /// assert_eq!(meter.status, MeterStatus::Active);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn reactivate(&self, id: &str) -> Result<V2BillingMeter, StripeError> {
        let safe_id = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/billing/meters/{}/reactivate",
            self.client.config.api_base, safe_id
        );
        let idempotency_key = crate::resources::resolve_idempotency_key(None);

        let req = self
            .client
            .http
            .post(&url)
            .header("Idempotency-Key", &*idempotency_key)
            .header("Content-Length", "0");

        let val = self.client.execute(req).await?;
        let meter: V2BillingMeter = serde_json::from_value(val)?;
        Ok(meter)
    }

    /// Lists V2 Billing Meters (`GET /v2/billing/meters`).
    ///
    /// Supports cursor-based pagination and filtering by meter status.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe::resources::BillingMetersResource;
    /// use some_stripe_types::{ListBillingMetersParams, MeterStatus};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let resource = BillingMetersResource::new(&client);
    /// let params = ListBillingMetersParams::new().limit(10).status(MeterStatus::Active);
    ///
    /// let meters = resource.list(&params).await?;
    /// for meter in meters {
    ///     println!("Meter: {} ({})", meter.display_name, meter.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        &self,
        params: &ListBillingMetersParams,
    ) -> Result<Vec<V2BillingMeter>, StripeError> {
        let query_str = build_list_query_string(params);
        let url = format!(
            "{}/v2/billing/meters{query_str}",
            self.client.config.api_base
        );

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;

        parse_meter_list(val)
    }
}

/// Helper function to construct query string for meter listing.
fn build_list_query_string(params: &ListBillingMetersParams) -> String {
    let mut parts = Vec::new();
    if let Some(limit) = params.limit {
        parts.push(format!("limit={limit}"));
    }
    if let Some(status) = params.status {
        parts.push(format!("status={}", status.as_str()));
    }
    if let Some(ref sa) = params.starting_after {
        let enc = some_stripe_core::patterns::serializer::percent_encode(sa);
        parts.push(format!("starting_after={enc}"));
    }
    if let Some(ref eb) = params.ending_before {
        let enc = some_stripe_core::patterns::serializer::percent_encode(eb);
        parts.push(format!("ending_before={enc}"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("?{}", parts.join("&"))
    }
}

/// Helper function to parse either a raw JSON array or a [`V2BillingMeterList`] object.
fn parse_meter_list(val: serde_json::Value) -> Result<Vec<V2BillingMeter>, StripeError> {
    if val.is_array() {
        serde_json::from_value(val).map_err(StripeError::from)
    } else {
        let list: V2BillingMeterList = serde_json::from_value(val)?;
        Ok(list.data)
    }
}
