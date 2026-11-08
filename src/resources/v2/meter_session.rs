//! Stripe API V2 Meter Event Session resource (`/v2/billing/meter_event_session`).
//!
//! Provides ephemeral session token provisioning for high-throughput meter streaming.
//! Ephemeral tokens authenticate high-frequency event batches without exposing secret keys.
//!
//! # Architecture & Quality Invariants
//! - **Gate G2**: Session tokens are masked as `[REDACTED]` in logs and zeroized on drop.
//! - **Gate G-Borrow**: Zero-copy borrowing across string slices and parameter models.
//! - **Gate G-Complexity**: Cyclomatic complexity $CC \le 5$ for all resource methods.
//! - **Gate G-Docs**: Comprehensive rustdoc documentation and examples.

use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{CreateMeterEventSessionParams, MeterEventSession};

use crate::resources::checkout::validate_and_encode_id;

/// Resource client for Stripe V2 Meter Event Sessions.
#[derive(Clone, Copy)]
pub struct MeterEventSessionResource<'a> {
    client: &'a StripeClient,
}

impl<'a> MeterEventSessionResource<'a> {
    /// Creates a new `MeterEventSessionResource` bound to the provided client.
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates an ephemeral meter event session with default parameters.
    ///
    /// # Errors
    /// Returns [`StripeError`] if network transport or API authentication fails.
    pub async fn create(&self) -> Result<MeterEventSession, StripeError> {
        let params = CreateMeterEventSessionParams::default();
        self.create_with_params(&params).await
    }

    /// Creates an ephemeral meter event session with explicit parameters.
    ///
    /// # Errors
    /// Returns [`StripeError`] if network transport or API authentication fails.
    pub async fn create_with_params(
        &self,
        params: &CreateMeterEventSessionParams,
    ) -> Result<MeterEventSession, StripeError> {
        let url = format!(
            "{}/v2/billing/meter_event_session",
            self.client.config.api_base
        );
        let mut req = self.client.http.post(&url).json(params);

        if let Some(idemp) = &params.idempotency_key {
            req = req.header("Idempotency-Key", idemp);
        }

        let val = self.client.execute(req).await?;
        let session: MeterEventSession = serde_json::from_value(val)?;
        Ok(session)
    }

    /// Retrieves an existing meter event session by its identifier.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if the ID is invalid or whitespace.
    pub async fn retrieve(&self, id: &str) -> Result<MeterEventSession, StripeError> {
        let encoded_id = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/billing/meter_event_sessions/{encoded_id}",
            self.client.config.api_base
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let session: MeterEventSession = serde_json::from_value(val)?;
        Ok(session)
    }

    /// Explicitly expires an active meter event session.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if the ID is invalid.
    pub async fn expire(&self, id: &str) -> Result<MeterEventSession, StripeError> {
        let encoded_id = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/billing/meter_event_sessions/{encoded_id}/expire",
            self.client.config.api_base
        );
        let req = self.client.http.post(&url).json(&serde_json::json!({}));
        let val = self.client.execute(req).await?;
        let session: MeterEventSession = serde_json::from_value(val)?;
        Ok(session)
    }
}
