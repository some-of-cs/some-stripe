//! Stripe API V2 Billing Meter Events resource.
//!
//! Provides synchronous meter event ingestion endpoints (`POST /v2/billing/meter_events`)
//! returning strongly typed [`V2MeterEvent`] responses with validation and idempotency.
//!
//! # Architecture & Quality Invariants:
//! - **Gate G-Borrow**: Zero-copy borrowing of parameters by reference.
//! - **Gate G-Docs**: Runnable doc-tests and complete rustdoc documentation.
//! - **Gate G-Complexity**: Cyclomatic complexity CC <= 6 on all functions. Max nesting depth <= 2.
//! - **Gate G2 & G-Law**: Multi-tenant isolation support via connected account scoping (`with_account`).

use super::meter_stream::MeterEventStreamResource;
use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{
    BatchIngestionResponse, CreateMeterEventParams, CreateMeterEventStreamParams,
    MeterEventPayload, V2MeterEvent,
};

/// Resource for synchronous meter event ingestion (`POST /v2/billing/meter_events`).
#[derive(Clone, Copy, Debug)]
pub struct MeterEventsResource<'a> {
    stream_resource: MeterEventStreamResource<'a>,
}

impl<'a> MeterEventsResource<'a> {
    /// Creates a new [`MeterEventsResource`] bound to the given client.
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            stream_resource: MeterEventStreamResource::new(client),
        }
    }

    /// Scopes this resource to operate on behalf of a connected account.
    #[must_use]
    pub fn with_account(&self, account_id: &'a str) -> Self {
        Self {
            stream_resource: self.stream_resource.with_account(account_id),
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.stream_resource.client()
    }

    /// Synchronously creates and validates a single meter event (`POST /v2/billing/meter_events`).
    ///
    /// # Errors
    /// Returns [`StripeError::Http`] or [`StripeError::Api`] if the Stripe API rejects the event.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::{CreateMeterEventParams, StripeClient};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreateMeterEventParams::new("api_requests", "cus_abc", "1")
    ///     .with_identifier("evt_idemp_101");
    /// let event = client.meter_events().create(&params).await?;
    /// assert_eq!(event.event_name(), "api_requests");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        params: &CreateMeterEventParams,
    ) -> Result<V2MeterEvent, StripeError> {
        let payload = params.to_payload();
        self.create_payload(&payload).await
    }

    /// Synchronously creates a meter event from an existing [`MeterEventPayload`] (`POST /v2/billing/meter_events`).
    ///
    /// # Errors
    /// Returns [`StripeError`] if creation or deserialization fails.
    pub async fn create_payload(
        &self,
        event: &MeterEventPayload,
    ) -> Result<V2MeterEvent, StripeError> {
        self.stream_resource.create_meter_event(event).await
    }

    /// Ingests a batch of meter events into the billing meter events endpoint (`POST /v2/billing/meter_events`).
    ///
    /// # Errors
    /// Returns [`StripeError`] if batch validation or network request fails.
    pub async fn create_batch(
        &self,
        events: &[MeterEventPayload],
    ) -> Result<BatchIngestionResponse, StripeError> {
        self.stream_resource
            .create_meter_events_batch(events, None, None)
            .await
    }

    /// Ingests a stream of meter events using high-throughput async processing (`POST /v2/billing/meter_event_stream`).
    ///
    /// # Errors
    /// Returns [`StripeError`] if ingestion fails.
    pub async fn create_stream(
        &self,
        params: &CreateMeterEventStreamParams,
    ) -> Result<BatchIngestionResponse, StripeError> {
        self.stream_resource.create(params).await
    }
}
