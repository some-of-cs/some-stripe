//! Stripe Accounts resource implementations.

use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::Account;

/// Resource for managing Stripe Accounts (`stripe.accounts`).
#[derive(Clone)]
pub struct AccountsResource {
    client: StripeClient,
}

impl AccountsResource {
    pub(crate) fn new(client: StripeClient) -> Self {
        Self { client }
    }

    /// Retrieves the details of the merchant's account associated with the secret key (`GET /v1/account`).
    pub async fn retrieve(&self) -> Result<Account, StripeError> {
        let url = format!("{}/v1/account", self.client.config.api_base);
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let account: Account = serde_json::from_value(val)?;
        Ok(account)
    }

    /// Alias for [`retrieve`] to access the authenticated merchant's account.
    pub async fn retrieve_current(&self) -> Result<Account, StripeError> {
        self.retrieve().await
    }

    /// Retrieves an account by its unique identifier (`GET /v1/accounts/{id}`).
    pub async fn retrieve_by_id(&self, account_id: &str) -> Result<Account, StripeError> {
        let url = format!("{}/v1/accounts/{}", self.client.config.api_base, account_id);
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let account: Account = serde_json::from_value(val)?;
        Ok(account)
    }
}
