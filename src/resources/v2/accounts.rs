//! Stripe API v2 Core Accounts resource implementation (`/v2/core/accounts`).
//!
//! Provides next-generation merchant account management and short-lived account tokens.
//!
//! # Architecture & Quality Invariants
//! - **Gate G-Borrow**: Parameters borrowed strictly by reference; zero deep clones.
//! - **Gate G-Complexity**: Cyclomatic complexity $CC \le 6$ for each method.
//! - **Gate G-Docs**: Comprehensive rustdoc documentation and runnable doctests.

use some_stripe_core::patterns::crud::{resource_sub_url, try_resource_url_with_id};
use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{
    CreateV2AccountParams, CreateV2AccountTokenParams, ListV2AccountsParams, UpdateV2AccountParams,
    V2Account, V2AccountToken, V2List,
};

/// Resource for managing Stripe API v2 Core Accounts (`stripe.v2().core().accounts()`).
#[derive(Clone, Copy)]
pub struct V2AccountsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> V2AccountsResource<'a> {
    /// Creates a new `V2AccountsResource` bound to the provided [`StripeClient`].
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Scopes this resource to operate on behalf of a specific connected account.
    #[must_use]
    pub fn with_account(&self, account: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account),
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates a new v2 Core Account (`POST /v2/core/accounts`).
    pub async fn create(&self, params: &CreateV2AccountParams) -> Result<V2Account, StripeError> {
        let url = format!("{}/v2/core/accounts", self.client.config.api_base);

        let mut req = self
            .client
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .json(params);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let account: V2Account = serde_json::from_value(val)?;
        Ok(account)
    }

    /// Retrieves a v2 Core Account by ID (`GET /v2/core/accounts/{id}`).
    pub async fn retrieve(&self, id: &str) -> Result<V2Account, StripeError> {
        let url = try_resource_url_with_id(&self.client.config.api_base, "/v2/core/accounts", id)?;

        let mut req = self
            .client
            .http
            .get(&url)
            .header("Content-Type", "application/json");

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let account: V2Account = serde_json::from_value(val)?;
        Ok(account)
    }

    /// Updates a v2 Core Account (`POST /v2/core/accounts/{id}`).
    pub async fn update(
        &self,
        id: &str,
        params: &UpdateV2AccountParams,
    ) -> Result<V2Account, StripeError> {
        let url = try_resource_url_with_id(&self.client.config.api_base, "/v2/core/accounts", id)?;

        let mut req = self
            .client
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .json(params);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let account: V2Account = serde_json::from_value(val)?;
        Ok(account)
    }

    /// Deletes a v2 Core Account (`DELETE /v2/core/accounts/{id}`).
    pub async fn del(&self, id: &str) -> Result<V2Account, StripeError> {
        let url = try_resource_url_with_id(&self.client.config.api_base, "/v2/core/accounts", id)?;

        let mut req = self
            .client
            .http
            .delete(&url)
            .header("Content-Type", "application/json");

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let account: V2Account = serde_json::from_value(val)?;
        Ok(account)
    }

    /// Lists v2 Core Accounts (`GET /v2/core/accounts`).
    pub async fn list(&self, params: &ListV2AccountsParams) -> Result<V2List<V2Account>, StripeError> {
        let mut url = format!("{}/v2/core/accounts", self.client.config.api_base);

        let mut query_params: Vec<(&str, std::borrow::Cow<'_, str>)> = Vec::new();
        if let Some(limit) = params.limit {
            query_params.push(("limit", std::borrow::Cow::Owned(limit.to_string())));
        }
        if let Some(ref page) = params.page {
            query_params.push(("page", std::borrow::Cow::Borrowed(page.as_str())));
        }

        if !query_params.is_empty() {
            let qs = query_params
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&");
            url = format!("{url}?{qs}");
        }

        let mut req = self
            .client
            .http
            .get(&url)
            .header("Content-Type", "application/json");

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let list: V2List<V2Account> = serde_json::from_value(val)?;
        Ok(list)
    }

    /// Creates a short-lived access token for an account (`POST /v2/core/accounts/{id}/tokens`).
    pub async fn create_token(
        &self,
        id: &str,
        params: &CreateV2AccountTokenParams,
    ) -> Result<V2AccountToken, StripeError> {
        let url = resource_sub_url(
            &self.client.config.api_base,
            "/v2/core/accounts",
            id,
            "tokens",
        )?;

        let mut req = self
            .client
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .json(params);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let token: V2AccountToken = serde_json::from_value(val)?;
        Ok(token)
    }
}
