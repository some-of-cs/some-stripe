//! Stripe Account Sessions resource implementation (`POST /v1/account_sessions`).
//!
//! Creates ephemeral client secrets for embedding Stripe Connect components into web applications.

use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::account_session::{AccountSession, CreateAccountSessionParams};

/// Resource for managing Stripe Account Sessions (`stripe.account_sessions`).
///
/// Wraps the `POST /v1/account_sessions` endpoint for initializing embedded Connect components.
#[derive(Clone, Copy)]
pub struct AccountSessionsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> AccountSessionsResource<'a> {
    /// Creates a new `AccountSessionsResource` bound to the provided [`StripeClient`].
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

    /// Scopes this resource to operate on behalf of a specific connected account.
    pub fn with_account(&self, account_id: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account_id),
        }
    }

    /// Creates an embedded Account Session (`POST /v1/account_sessions`).
    ///
    /// Validates `params.account` to protect against path traversal and header injection.
    /// Automatically injects an RFC 4122 UUID v4 `Idempotency-Key` header if omitted.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `params.account` contains invalid characters or validation fails.
    /// - Returns [`StripeError::Api`] if the Stripe API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into an [`AccountSession`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::account_session::{AccountSessionComponents, CreateAccountSessionParams};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreateAccountSessionParams::for_account("acct_1032D82eZvKYlo2C")
    ///     .with_payments(true);
    ///
    /// let session = client.account_sessions().create(&params).await?;
    /// println!("Client secret: {}", session.client_secret);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        params: &CreateAccountSessionParams,
    ) -> Result<AccountSession, StripeError> {
        validate_and_encode_id(&params.account)?;

        let url = format!("{}/v1/account_sessions", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let generated_key;
        let idempotency_key = match params.idempotency_key.as_deref() {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = crate::resources::generate_uuid_v4();
                &generated_key
            }
        };

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let session: AccountSession = serde_json::from_value(val)?;
        Ok(session)
    }
}
