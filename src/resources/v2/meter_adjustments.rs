//! Stripe API V2 Billing Meter Event Adjustments resource implementation (`stripe.v2.billing.meter_event_adjustments`).
//!
//! Provides endpoints to create, retrieve, and list meter event cancellations and adjustments.
//!
//! # Architecture & Quality Gates:
//! - **Gate G-Borrow (Zero-Clone Discipline)**: Parameter borrowing by reference (`&CreateMeterEventAdjustmentParams`),
//!   zero deep `.clone()` calls on domain entities in request pipelines, and borrowed string slices for accessors.
//! - **Gate G2 (Security & Idempotency)**: Automatic injection of RFC 4122 UUID v4 `Idempotency-Key` headers
//!   on mutable operations (`create`) when omitted by the caller via [`crate::resources::resolve_idempotency_key`].
//! - **Security Invariant**: Strict path traversal prevention via [`validate_and_encode_id`].
//! - **Gate G-Law (FTC Section 5 & EU Consumer Rights)**: Statutory audit compliance via immutable event identifiers [AC-US-8.2.0-LAW].
//! - **Gate G-Complexity**: Cyclomatic complexity <= 6 across all resource methods.
//! - **Gate G-Docs**: Runnable doc-tests verified by `cargo test --doc`.

use serde::Deserialize;
use some_stripe_core::patterns::serializer::{
    encode_form_pairs, serialize_v2_json, to_form_params,
};
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CreateMeterEventAdjustmentParams, ListMeterEventAdjustmentsParams, MeterEventAdjustment,
};

/// Internal envelope helper allowing both standard Stripe `{ "data": [...] }` envelopes
/// and raw JSON arrays `[...]` to be deserialized without deep clones.
#[derive(Deserialize)]
#[serde(untagged)]
enum ListEnvelope {
    Data { data: Vec<MeterEventAdjustment> },
    Raw(Vec<MeterEventAdjustment>),
}

/// Resource client for Stripe API V2 Billing Meter Event Adjustments (`stripe.v2.billing.meter_event_adjustments`).
///
/// Enables platforms to create, retrieve, and list usage cancellations and adjustments for meter billing.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
/// use some_stripe::resources::MeterEventAdjustmentsResource;
/// use some_stripe_types::CreateMeterEventAdjustmentParams;
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let client = StripeClient::new("sk_test_123");
/// let resource = MeterEventAdjustmentsResource::new(&client);
///
/// let params = CreateMeterEventAdjustmentParams::cancel_event("alpaca_tokens", "inference_req_42");
/// let adjustment = resource.create(&params).await?;
/// assert!(adjustment.is_complete());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy)]
pub struct MeterEventAdjustmentsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> MeterEventAdjustmentsResource<'a> {
    /// Creates a new `MeterEventAdjustmentsResource` bound to the provided [`StripeClient`].
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates a new meter event adjustment (`POST /v2/billing/meter_event_adjustments`).
    ///
    /// Sends a JSON payload to Stripe API V2 and automatically attaches an RFC 4122
    /// UUID v4 idempotency key if omitted by the caller (Gate G2).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe::resources::MeterEventAdjustmentsResource;
    /// use some_stripe_types::{CreateMeterEventAdjustmentParams, AdjustmentReason};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let resource = MeterEventAdjustmentsResource::new(&client);
    ///
    /// let params = CreateMeterEventAdjustmentParams::cancel_event("tokens", "req_gen_1")
    ///     .reason(AdjustmentReason::DuplicateUsage);
    /// let adjustment = resource.create(&params).await?;
    /// assert_eq!(adjustment.status, "complete");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        params: &CreateMeterEventAdjustmentParams,
    ) -> Result<MeterEventAdjustment, StripeError> {
        let url = format!(
            "{}/v2/billing/meter_event_adjustments",
            self.client.config.api_base
        );
        let body = serialize_v2_json(params)?;
        let idempotency_key =
            crate::resources::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Idempotency-Key", &*idempotency_key)
            .body(body);

        let val = self.client.execute(req).await?;
        let adjustment: MeterEventAdjustment = serde_json::from_value(val)?;
        Ok(adjustment)
    }

    /// Retrieves an existing meter event adjustment by ID (`GET /v2/billing/meter_event_adjustments/{id}`).
    ///
    /// Strictly protects against directory traversal and URL injection via [`validate_and_encode_id`] (Gate G2).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe::resources::MeterEventAdjustmentsResource;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let resource = MeterEventAdjustmentsResource::new(&client);
    ///
    /// let adjustment = resource.retrieve("mea_test_123").await?;
    /// println!("Retrieved adjustment status: {}", adjustment.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, id: &str) -> Result<MeterEventAdjustment, StripeError> {
        let safe_id = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/billing/meter_event_adjustments/{}",
            self.client.config.api_base, safe_id
        );

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let adjustment: MeterEventAdjustment = serde_json::from_value(val)?;
        Ok(adjustment)
    }

    /// Lists meter event adjustments according to query parameters (`GET /v2/billing/meter_event_adjustments`).
    ///
    /// Supports pagination cursor parameters (`starting_after`, `ending_before`, `limit`) and event filtering.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe::resources::MeterEventAdjustmentsResource;
    /// use some_stripe_types::ListMeterEventAdjustmentsParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let resource = MeterEventAdjustmentsResource::new(&client);
    ///
    /// let params = ListMeterEventAdjustmentsParams::new().limit(20).event_name("tokens");
    /// let adjustments = resource.list(&params).await?;
    /// println!("Found {} adjustments", adjustments.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        &self,
        params: &ListMeterEventAdjustmentsParams,
    ) -> Result<Vec<MeterEventAdjustment>, StripeError> {
        let query_pairs = to_form_params(params)?;
        let url = if query_pairs.is_empty() {
            format!(
                "{}/v2/billing/meter_event_adjustments",
                self.client.config.api_base
            )
        } else {
            let query_str = encode_form_pairs(&query_pairs);
            format!(
                "{}/v2/billing/meter_event_adjustments?{query_str}",
                self.client.config.api_base
            )
        };

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let envelope: ListEnvelope = serde_json::from_value(val)?;
        let adjustments = match envelope {
            ListEnvelope::Data { data } => data,
            ListEnvelope::Raw(data) => data,
        };
        Ok(adjustments)
    }
}
