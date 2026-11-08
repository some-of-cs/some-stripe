//! Stripe API V2 Billing Meter Event Stream resource.
//!
//! Provides high-throughput event ingestion endpoints (`/v2/billing/meter_event_stream` and `/v2/billing/meter_events`)
//! supporting zero-copy slice ingestion, session bearer token authorization, idempotency keys, and batch limits.
//!
//! # Architecture & Quality Invariants:
//! - **Gate G-Borrow**: Zero-copy borrowing of events slice (`&[MeterEventPayload]`); no unnecessary heap allocations.
//! - **Gate G-Docs**: Runnable doc-tests and comprehensive rustdoc documentation on all public components.
//! - **Gate G-Complexity**: Cyclomatic complexity CC <= 6 on all functions with max nesting depth <= 2.
//! - **Gate G2 & G-Law**: Path safety and tenant-isolation support via connected account scoping (`with_account`).

use some_stripe_core::client::RequestOptions;
use some_stripe_core::patterns::meter_serializer::MeterBatchSerializer;
use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{
    BatchIngestionResponse, CreateMeterEventStreamParams, MeterEventPayload, V2MeterEvent,
};

/// Resource for ingesting high-throughput usage meter event streams (`POST /v2/billing/meter_event_stream` and `/v2/billing/meter_events`).
#[derive(Clone, Copy, Debug)]
pub struct MeterEventStreamResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> MeterEventStreamResource<'a> {
    /// Creates a new [`MeterEventStreamResource`] bound to the given client.
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Scopes this resource to operate on behalf of a connected account via the `Stripe-Account` header.
    #[must_use]
    pub fn with_account(&self, account_id: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account_id),
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Ingests a batch of meter events from borrowed parameters (`POST /v2/billing/meter_event_stream`).
    ///
    /// Uses default client secret authentication.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if `params.events` is empty or exceeds 1,000 events.
    /// Returns [`StripeError::Http`] or [`StripeError::Api`] if the ingestion request fails.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::{CreateMeterEventStreamParams, MeterEventPayload, StripeClient};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreateMeterEventStreamParams::single(
    ///     MeterEventPayload::new("tokens", "cus_123", "500"),
    /// );
    /// let response = client.meter_event_stream().create(&params).await?;
    /// assert!(response.is_success());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        params: &CreateMeterEventStreamParams,
    ) -> Result<BatchIngestionResponse, StripeError> {
        self.create_batch(params.events(), None).await
    }

    /// Ingests a batch of meter events using an ephemeral session bearer token (`POST /v2/billing/meter_event_stream`).
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if `params.events` is empty or exceeds 1,000 events.
    /// Returns [`StripeError::Http`] or [`StripeError::Api`] if the ingestion request fails.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::{CreateMeterEventStreamParams, MeterEventPayload, StripeClient};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreateMeterEventStreamParams::single(
    ///     MeterEventPayload::new("tokens", "cus_123", "500"),
    /// );
    /// let response = client
    ///     .meter_event_stream()
    ///     .create_with_session(&params, "sess_tok_abc")
    ///     .await?;
    /// assert!(response.is_success());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_with_session(
        &self,
        params: &CreateMeterEventStreamParams,
        session_token: &str,
    ) -> Result<BatchIngestionResponse, StripeError> {
        self.create_batch(params.events(), Some(session_token))
            .await
    }

    /// Ingests a slice of meter events directly with zero heap cloning (`POST /v2/billing/meter_event_stream`).
    ///
    /// If `session_token` is provided, authorization uses `Bearer <session_token>`. Otherwise,
    /// falls back to default client authentication.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if `events` is empty or exceeds 1,000 items.
    /// Returns [`StripeError::Http`] or [`StripeError::Api`] upon network or server errors.
    pub async fn create_batch(
        &self,
        events: &[MeterEventPayload],
        session_token: Option<&str>,
    ) -> Result<BatchIngestionResponse, StripeError> {
        self.create_batch_with_options(events, session_token, None)
            .await
    }

    /// Ingests a slice of meter events with optional session token and batch-level idempotency key (`POST /v2/billing/meter_event_stream`).
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if `events` is empty or exceeds 1,000 items.
    /// Returns [`StripeError::Http`] or [`StripeError::Api`] upon network or server errors.
    pub async fn create_batch_with_options(
        &self,
        events: &[MeterEventPayload],
        session_token: Option<&str>,
        idempotency_key: Option<&str>,
    ) -> Result<BatchIngestionResponse, StripeError> {
        let url = format!(
            "{}/v2/billing/meter_event_stream",
            self.client.config.api_base
        );
        self.execute_batch_post(&url, events, session_token, idempotency_key)
            .await
    }

    /// Ingests a slice of meter events to the standard meter events endpoint (`POST /v2/billing/meter_events`).
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if `events` is empty or exceeds 1,000 items.
    /// Returns [`StripeError::Http`] or [`StripeError::Api`] upon network or server errors.
    pub async fn create_meter_events_batch(
        &self,
        events: &[MeterEventPayload],
        session_token: Option<&str>,
        idempotency_key: Option<&str>,
    ) -> Result<BatchIngestionResponse, StripeError> {
        let url = format!("{}/v2/billing/meter_events", self.client.config.api_base);
        self.execute_batch_post(&url, events, session_token, idempotency_key)
            .await
    }

    /// Synchronously creates and validates a single meter event (`POST /v2/billing/meter_events`).
    ///
    /// Returns strongly typed [`V2MeterEvent`] containing confirmation of acceptance.
    /// If `event.identifier` is present, it is automatically forwarded as the `Idempotency-Key` header.
    ///
    /// # Errors
    /// Returns [`StripeError::Http`] or [`StripeError::Api`] if creation or validation fails.
    /// Returns [`StripeError::Json`] if response deserialization fails.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::{MeterEventPayload, StripeClient};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let event = MeterEventPayload::new("api_calls", "cus_123", "1")
    ///     .with_identifier("req_evt_001");
    /// let result = client.meter_event_stream().create_meter_event(&event).await?;
    /// assert_eq!(result.event_name(), "api_calls");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_meter_event(
        &self,
        event: &MeterEventPayload,
    ) -> Result<V2MeterEvent, StripeError> {
        self.create_meter_event_with_options(event, None, None)
            .await
    }

    /// Synchronously creates and validates a single meter event with optional session token and idempotency key (`POST /v2/billing/meter_events`).
    ///
    /// # Errors
    /// Returns [`StripeError::Http`] or [`StripeError::Api`] if creation fails.
    pub async fn create_meter_event_with_options(
        &self,
        event: &MeterEventPayload,
        session_token: Option<&str>,
        idempotency_key: Option<&str>,
    ) -> Result<V2MeterEvent, StripeError> {
        let url = format!("{}/v2/billing/meter_events", self.client.config.api_base);
        let body = MeterBatchSerializer::serialize_single_event_json(event)?;

        let mut req = self
            .client
            .http
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::ACCEPT, "application/json")
            .body(body);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let opts =
            self.build_request_options(session_token, idempotency_key.or(event.identifier()));
        let val = self.client.execute_with_options(req, opts.as_ref()).await?;

        let meter_event: V2MeterEvent = serde_json::from_value(val)?;
        Ok(meter_event)
    }

    /// Internal helper to execute a batch POST request against the specified URL.
    async fn execute_batch_post(
        &self,
        url: &str,
        events: &[MeterEventPayload],
        session_token: Option<&str>,
        idempotency_key: Option<&str>,
    ) -> Result<BatchIngestionResponse, StripeError> {
        MeterBatchSerializer::validate_batch_size(events)?;
        let body = MeterBatchSerializer::serialize_batch_json(events)?;

        let mut req = self
            .client
            .http
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::ACCEPT, "application/json")
            .body(body);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let opts = self.build_request_options(session_token, idempotency_key);
        let val = self.client.execute_with_options(req, opts.as_ref()).await?;

        let response: BatchIngestionResponse = serde_json::from_value(val)?;
        Ok(response)
    }

    /// Builds a [`RequestOptions`] instance with session token and/or idempotency key.
    fn build_request_options(
        &self,
        session_token: Option<&str>,
        idempotency_key: Option<&str>,
    ) -> Option<RequestOptions> {
        if session_token.is_none() && idempotency_key.is_none() {
            return None;
        }

        let mut opts = RequestOptions::new();
        if let Some(tok) = session_token {
            opts = opts.bearer_token(tok);
        }
        if let Some(key) = idempotency_key {
            opts = opts.idempotency_key(key);
        }
        Some(opts)
    }
}
