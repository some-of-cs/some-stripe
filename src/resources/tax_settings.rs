//! Stripe Tax Settings resource implementation.
//!
//! Provides management of account-wide Stripe Tax configurations (`/v1/tax/settings`).
//! Adheres to:
//! - **Gate G-Borrow**: Zero-clone borrowing discipline with reference accessors.
//! - **Gate G-Law**: EU VAT Directive 2006/112/EC and international tax nexus compliance.
//! - **Gate G-Docs**: Runnable doctests and comprehensive rustdoc on public APIs.
//! - **Gate G-Complexity**: Strict cyclomatic complexity <= 6.

use some_stripe_core::patterns::to_form_params;
use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{TaxSettings, UpdateTaxSettingsParams};

/// Resource for managing account-wide Stripe Tax Settings (`/v1/tax/settings`).
///
/// Tax Settings govern head office physical address and fallback tax behavior/codes
/// required before enabling automatic tax calculation.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
/// use some_stripe::resources::TaxSettingsResource;
///
/// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
/// let client = StripeClient::new("sk_test_123");
/// let settings_resource = TaxSettingsResource::new(&client);
/// let settings = settings_resource.retrieve().await?;
/// println!("Tax calculation status: {}", settings.status);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy)]
pub struct TaxSettingsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> TaxSettingsResource<'a> {
    /// Creates a new `TaxSettingsResource` bound to the provided client.
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Borrows the inner `StripeClient`.
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

    /// Retrieves the current tax settings for the account (`GET /v1/tax/settings`).
    ///
    /// # Errors
    /// Returns [`StripeError`] if the network request fails, authentication is invalid,
    /// or deserialization encounters mismatched schema.
    pub async fn retrieve(&self) -> Result<TaxSettings, StripeError> {
        let url = format!("{}/v1/tax/settings", self.client.config.api_base);
        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }
        let val = self.client.execute(req).await?;
        let settings: TaxSettings = serde_json::from_value(val)?;
        Ok(settings)
    }

    /// Updates tax settings parameters such as head office address or default tax code (`POST /v1/tax/settings`).
    ///
    /// # Errors
    /// Returns [`StripeError`] if invalid addresses or tax codes are supplied,
    /// or if the request fails execution.
    pub async fn update(
        &self,
        params: &UpdateTaxSettingsParams,
    ) -> Result<TaxSettings, StripeError> {
        let url = format!("{}/v1/tax/settings", self.client.config.api_base);
        let form_data = to_form_params(params)?;
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
        let settings: TaxSettings = serde_json::from_value(val)?;
        Ok(settings)
    }
}
