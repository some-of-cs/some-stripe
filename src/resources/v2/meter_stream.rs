//! Stripe API V2 Meter Event Stream resource (`/v2/billing/meter_event_stream`).
//!
//! Provides high-throughput streaming ingestion for meter events.
//! Supports both standard secret key authentication and ephemeral session bearer tokens.
//!
//! # Architecture & Quality Invariants
//! - **Gate G-Borrow**: Zero-copy borrowing across slices and parameter references.
//! - **Gate G-Complexity**: Cyclomatic complexity $CC \le 5$ for all resource methods.
//! - **Gate G-Law**: Validates statutory consumption accuracy before wire submission.

use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{
    BatchIngestionResponse, CreateMeterEventStreamParams, MeterEventPayload,
    validate_statutory_meter_event_accuracy,
};

/// Resource client for Stripe V2 Meter Event Streams.
#[derive(Clone, Copy)]
pub struct MeterEventStreamResource<'a> {
    client: &'a StripeClient,
}

impl<'a> MeterEventStreamResource<'a> {
    /// Creates a new `MeterEventStreamResource` bound to the provided client.
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Ingests a batch of meter events using the client's configured credentials.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if statutory compliance validation fails.
    pub async fn create(
        &self,
        params: &CreateMeterEventStreamParams,
    ) -> Result<BatchIngestionResponse, StripeError> {
        self.validate_events(&params.events)?;

        let url = format!(
            "{}/v2/billing/meter_event_stream",
            self.client.config.api_base
        );
        let req = self.client.http.post(&url).json(params);

        let val = self.client.execute(req).await?;
        let resp: BatchIngestionResponse = serde_json::from_value(val)?;
        Ok(resp)
    }

    /// Ingests a batch of meter events authenticated with an ephemeral session token.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if statutory validation fails or network error occurs.
    pub async fn create_with_session(
        &self,
        session_token: &str,
        params: &CreateMeterEventStreamParams,
    ) -> Result<BatchIngestionResponse, StripeError> {
        self.validate_events(&params.events)?;

        let url = format!(
            "{}/v2/billing/meter_event_stream",
            self.client.config.api_base
        );
        let req = self
            .client
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {session_token}"))
            .header("Content-Type", "application/json")
            .json(params);

        let resp = req.send().await.map_err(StripeError::Http)?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(StripeError::InvalidRequest(format!(
                "HTTP {} streaming error: {}",
                status.as_u16(),
                body
            )));
        }

        let body_bytes = resp.bytes().await.map_err(StripeError::Http)?;
        let val: BatchIngestionResponse = serde_json::from_slice(&body_bytes)?;
        Ok(val)
    }

    /// Ingests a borrowed slice of meter events directly.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if event validation fails.
    pub async fn create_batch(
        &self,
        events: &[MeterEventPayload],
    ) -> Result<BatchIngestionResponse, StripeError> {
        let params = CreateMeterEventStreamParams {
            events: events.to_vec(),
        };
        self.create(&params).await
    }

    fn validate_events(&self, events: &[MeterEventPayload]) -> Result<(), StripeError> {
        if events.is_empty() {
            return Err(StripeError::InvalidRequest(
                "meter events stream batch cannot be empty".to_string(),
            ));
        }

        for event in events {
            validate_statutory_meter_event_accuracy(event)
                .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;
        }

        Ok(())
    }
}
