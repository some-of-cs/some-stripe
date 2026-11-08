//! Stripe API v2 Billing Meter Event Session resource implementation.
//!
//! Provides endpoints to create, retrieve, and expire short-lived meter event sessions
//! (`POST /v2/billing/meter_event_session`, `GET /v2/billing/meter_event_session/{id}`,
//! and `POST /v2/billing/meter_event_session/{id}/expire`).

use some_stripe_core::patterns::serializer::serialize_v2_json;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::meter_session::{CreateMeterEventSessionParams, MeterEventSession};

/// Resource for managing Stripe Billing Meter Event Sessions (`stripe.v2().billing().meter_event_session()`).
#[derive(Clone, Copy)]
pub struct MeterEventSessionResource<'a> {
    client: &'a StripeClient,
}

impl<'a> MeterEventSessionResource<'a> {
    /// Creates a new `MeterEventSessionResource` bound to the provided [`StripeClient`].
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates a new Meter Event Session (`POST /v2/billing/meter_event_session`).
    ///
    /// Generates a short-lived (15-minute) cryptographic authentication token
    /// for high-throughput meter event ingestion.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::Api`] if the Stripe API rejects the request.
    /// - Returns [`StripeError::Json`] if JSON serialization or deserialization fails.
    /// - Returns [`StripeError::Http`] if network transport fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::meter_session::CreateMeterEventSessionParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreateMeterEventSessionParams::new().with_idempotency_key("session_req_001");
    ///
    /// let session = client.meter_event_sessions().create(&params).await?;
    /// println!("Created session id: {}", session.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        params: &CreateMeterEventSessionParams,
    ) -> Result<MeterEventSession, StripeError> {
        let url = format!(
            "{}/v2/billing/meter_event_session",
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
        let session: MeterEventSession = serde_json::from_value(val)?;
        Ok(session)
    }

    /// Retrieves an existing Meter Event Session (`GET /v2/billing/meter_event_session/{id}`).
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `id` is empty or contains path traversal characters.
    /// - Returns [`StripeError::Api`] if the session ID does not exist.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized.
    pub async fn retrieve(&self, id: &str) -> Result<MeterEventSession, StripeError> {
        let encoded_id = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/billing/meter_event_session/{}",
            self.client.config.api_base, encoded_id
        );

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let session: MeterEventSession = serde_json::from_value(val)?;
        Ok(session)
    }

    /// Expires an active Meter Event Session immediately (`POST /v2/billing/meter_event_session/{id}/expire`).
    ///
    /// Revokes the session's authentication token so that further requests using this token
    /// will fail with authentication errors.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `id` is empty or invalid.
    /// - Returns [`StripeError::Api`] if the session ID is not found.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized.
    pub async fn expire(&self, id: &str) -> Result<MeterEventSession, StripeError> {
        self.expire_with_idempotency_key(id, None).await
    }

    /// Expires an active Meter Event Session with an explicit idempotency key.
    pub async fn expire_with_idempotency_key(
        &self,
        id: &str,
        idempotency_key: Option<&str>,
    ) -> Result<MeterEventSession, StripeError> {
        let encoded_id = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/billing/meter_event_session/{}/expire",
            self.client.config.api_base, encoded_id
        );
        let resolved_idemp = crate::resources::resolve_idempotency_key(idempotency_key);

        let req = self
            .client
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Idempotency-Key", &*resolved_idemp)
            .body("{}");

        let val = self.client.execute(req).await?;
        let session: MeterEventSession = serde_json::from_value(val)?;
        Ok(session)
    }
}
