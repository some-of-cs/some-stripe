//! Stripe Identity Verification Sessions resource implementation.
//!
//! Exposes operations for managing Identity Verification Sessions (`/v1/identity/verification_sessions`),
//! including session creation, retrieval, updates, cancellation, and GDPR Article 17 redaction.
//!
//! ## Biometric Privacy & GDPR Article 17 Invariant (Gate G2 & Gate G-Law)
//! Under GDPR Article 17 (Right to Erasure) and EU biometric privacy regulations:
//! - Raw biometric images and facial geometry vectors are never retained in application memory.
//! - The [`VerificationSessionsResource::redact`] endpoint allows callers to expunge
//!   personal identity records and biometric selfie verifications permanently on Stripe servers.

use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::identity_session::{
    CreateVerificationSessionParams, UpdateVerificationSessionParams, VerificationSession,
    validate_identity_url,
};

/// Resource for managing Stripe Identity Verification Sessions (`stripe.identity().verification_sessions()`).
///
/// Wraps the `/v1/identity/verification_sessions` endpoints for initiating and managing
/// biometric and document verification workflows.
#[derive(Clone, Copy)]
pub struct VerificationSessionsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> VerificationSessionsResource<'a> {
    /// Creates a new `VerificationSessionsResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this resource to operate on behalf of a specific connected account
    /// by attaching the `Stripe-Account` tenant header to all outbound requests.
    pub fn with_account(&self, account: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account),
        }
    }

    /// Creates a new Identity Verification Session (`POST /v1/identity/verification_sessions`).
    ///
    /// Initiates a customer verification flow. Returns a [`VerificationSession`] containing
    /// the secure redirect URL (`session.url`) and ephemeral client secret.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `params.return_url` contains invalid schemes or CRLF characters.
    /// - Returns [`StripeError::Api`] if the Stripe API rejects the request.
    /// - Returns [`StripeError::Json`] if response deserialization fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::identity_session::{
    ///     CreateVerificationSessionParams, DocumentOptions, VerificationOptions,
    /// };
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreateVerificationSessionParams::document()
    ///     .with_options(
    ///         VerificationOptions::new().with_document(
    ///             DocumentOptions::new().with_matching_selfie(true),
    ///         ),
    ///     )
    ///     .with_return_url("https://platform.com/kyc/complete");
    ///
    /// let session = client.identity_sessions().create(&params).await?;
    /// println!("Verification URL: {:?}", session.url());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        params: &CreateVerificationSessionParams,
    ) -> Result<VerificationSession, StripeError> {
        if let Some(ref url) = params.return_url {
            validate_identity_url(url).map_err(|e| StripeError::InvalidRequest(e.to_string()))?;
        }

        let url = format!(
            "{}/v1/identity/verification_sessions",
            self.client.config.api_base
        );
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let session: VerificationSession = serde_json::from_value(val)?;
        Ok(session)
    }

    /// Retrieves an existing Verification Session (`GET /v1/identity/verification_sessions/{id}`).
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `id` is invalid or contains forbidden characters.
    /// - Returns [`StripeError::Api`] if the session is not found or the request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let session = client.identity_sessions().retrieve("vs_123456").await?;
    /// println!("Session status: {}", session.status());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, id: &str) -> Result<VerificationSession, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/identity/verification_sessions/{encoded}",
            self.client.config.api_base
        );

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let session: VerificationSession = serde_json::from_value(val)?;
        Ok(session)
    }

    /// Updates an existing Verification Session (`POST /v1/identity/verification_sessions/{id}`).
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `id` is invalid.
    /// - Returns [`StripeError::Api`] if the update cannot be applied.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::identity_session::{
    ///     DocumentOptions, UpdateVerificationSessionParams, VerificationOptions,
    /// };
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = UpdateVerificationSessionParams::new()
    ///     .with_options(
    ///         VerificationOptions::new().with_document(
    ///             DocumentOptions::new().with_matching_selfie(false),
    ///         ),
    ///     );
    ///
    /// let session = client.identity_sessions().update("vs_123456", &params).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        id: &str,
        params: &UpdateVerificationSessionParams,
    ) -> Result<VerificationSession, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/identity/verification_sessions/{encoded}",
            self.client.config.api_base
        );
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let session: VerificationSession = serde_json::from_value(val)?;
        Ok(session)
    }

    /// Cancels a Verification Session (`POST /v1/identity/verification_sessions/{id}/cancel`).
    ///
    /// A canceled session cannot be reopened. The customer will no longer be able to submit
    /// verification documents through the session URL.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `id` is invalid.
    /// - Returns [`StripeError::Api`] if cancellation fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let session = client.identity_sessions().cancel("vs_123456").await?;
    /// assert!(session.is_canceled());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel(&self, id: &str) -> Result<VerificationSession, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/identity/verification_sessions/{encoded}/cancel",
            self.client.config.api_base
        );
        let form_data: Vec<(&str, &str)> = Vec::new();
        let idempotency_key = super::resolve_idempotency_key(None);

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let session: VerificationSession = serde_json::from_value(val)?;
        Ok(session)
    }

    /// Redacts a Verification Session (`POST /v1/identity/verification_sessions/{id}/redact`).
    ///
    /// Permanent erasure of all personal data, document photos, and biometric records
    /// under GDPR Article 17 (Right to Erasure). Redaction is irreversible.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `id` is invalid.
    /// - Returns [`StripeError::Api`] if redaction fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let session = client.identity_sessions().redact("vs_123456").await?;
    /// assert!(session.is_redacted());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn redact(&self, id: &str) -> Result<VerificationSession, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/identity/verification_sessions/{encoded}/redact",
            self.client.config.api_base
        );
        let form_data: Vec<(&str, &str)> = Vec::new();
        let idempotency_key = super::resolve_idempotency_key(None);

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let session: VerificationSession = serde_json::from_value(val)?;
        Ok(session)
    }
}
