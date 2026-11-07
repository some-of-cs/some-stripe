//! Stripe Account Links resource implementation (`POST /v1/account_links`).
//!
//! Creates temporary redirect URLs to Stripe-hosted Connect onboarding and update flows.

use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::account_link::{AccountLink, CreateAccountLinkParams};

/// Resource for managing Stripe Account Links (`stripe.account_links`).
///
/// Wraps the `POST /v1/account_links` endpoint for generating hosted onboarding links.
#[derive(Clone, Copy)]
pub struct AccountLinksResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> AccountLinksResource<'a> {
    /// Creates a new `AccountLinksResource` bound to the provided [`StripeClient`].
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

    /// Creates an account link URL (`POST /v1/account_links`).
    ///
    /// Validates `params.account` to protect against path traversal and header injection.
    /// Automatically injects an RFC 4122 UUID v4 `Idempotency-Key` header if omitted.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `params.account` contains invalid characters or validation fails.
    /// - Returns [`StripeError::Api`] if the Stripe API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into an [`AccountLink`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::account_link::{AccountLinkType, CreateAccountLinkParams};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreateAccountLinkParams::new(
    ///     "acct_1032D82eZvKYlo2C",
    ///     "https://example.com/reauth",
    ///     "https://example.com/return",
    ///     AccountLinkType::AccountOnboarding,
    /// );
    ///
    /// let link = client.account_links().create(&params).await?;
    /// println!("Hosted onboarding URL: {}", link.url);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        params: &CreateAccountLinkParams,
    ) -> Result<AccountLink, StripeError> {
        // Validate account ID to prevent injection attacks
        validate_and_encode_id(&params.account)?;

        let url = format!("{}/v1/account_links", self.client.config.api_base);
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
        let link: AccountLink = serde_json::from_value(val)?;
        Ok(link)
    }
}
