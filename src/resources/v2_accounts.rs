//! Stripe API v2 Core Accounts and Access Tokens resource implementation.
//!
//! Provides CRUD operations for V2 Core Accounts (`POST /v2/core/accounts`,
//! `GET /v2/core/accounts/{id}`, `POST /v2/core/accounts/{id}`, `DELETE /v2/core/accounts/{id}`,
//! `GET /v2/core/accounts`) and short-lived access token generation (`POST /v2/core/accounts/{id}/tokens`).
//!
//! # Protocol Compliance
//! All endpoints operate strictly under Stripe API v2 JSON protocol:
//! - Request bodies are encoded as `application/json` payloads.
//! - Sensitive session tokens returned by `create_token` implement [`ZeroizeOnDrop`](zeroize::ZeroizeOnDrop)
//!   and redact secret strings in [`std::fmt::Debug`] (Gate G2).
//! - Paths are protected against directory traversal and invalid ID characters via [`validate_and_encode_id`].

use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::v2_account::{
    CreateV2AccountParams, CreateV2AccountTokenParams, ListV2AccountsParams, UpdateV2AccountParams,
    V2Account, V2AccountList, V2AccountToken,
};

/// Resource handle for Stripe API v2 Core Accounts operations (`stripe.v2_accounts()`
/// or `stripe.v2().core().accounts()`).
#[derive(Clone, Copy)]
pub struct V2AccountsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> V2AccountsResource<'a> {
    /// Creates a new `V2AccountsResource` wrapping the provided [`StripeClient`].
    #[must_use]
    pub const fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Returns the underlying [`StripeClient`] reference.
    #[must_use]
    pub const fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this resource to operate on behalf of a specific connected account
    /// by attaching the `Stripe-Account` tenant header to all outbound requests.
    #[must_use]
    pub const fn with_account(&self, account: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account),
        }
    }

    /// Creates a new Stripe API v2 Core Account (`POST /v2/core/accounts`).
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if parameters fail validation,
    /// [`StripeError::Http`] on network failure, or [`StripeError::Api`] if Stripe returns an error.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::v2_account::CreateV2AccountParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreateV2AccountParams::new()
    ///     .with_display_name("Acme Services")
    ///     .with_contact_email("billing@acme.example.com");
    ///
    /// let account = client.v2_accounts().create(&params).await?;
    /// println!("Created v2 account: {}", account.id());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, params: &CreateV2AccountParams) -> Result<V2Account, StripeError> {
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!("{}/v2/core/accounts", self.client.config.api_base);
        let body_str = serde_json::to_string(params)?;
        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let mut req = self
            .client
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Idempotency-Key", &*idempotency_key)
            .body(body_str);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let account: V2Account = serde_json::from_value(val)?;
        Ok(account)
    }

    /// Retrieves an existing Stripe API v2 Core Account by ID (`GET /v2/core/accounts/{id}`).
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if `id` is invalid,
    /// [`StripeError::Http`] on network failure, or [`StripeError::Api`] if not found.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let account = client.v2_accounts().retrieve("acct_v2_123").await?;
    /// println!("Account status: {:?}", account.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, id: &str) -> Result<V2Account, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!("{}/v2/core/accounts/{encoded}", self.client.config.api_base);

        let mut req = self
            .client
            .http
            .get(&url)
            .header("Accept", "application/json");

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let account: V2Account = serde_json::from_value(val)?;
        Ok(account)
    }

    /// Updates an existing Stripe API v2 Core Account (`POST /v2/core/accounts/{id}`).
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if `id` or `params` fail validation,
    /// [`StripeError::Http`] on network transport failure, or [`StripeError::Api`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::v2_account::{UpdateV2AccountParams, V2AccountStatus};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = UpdateV2AccountParams::new()
    ///     .with_display_name("Acme Global Services")
    ///     .with_status(V2AccountStatus::Active);
    ///
    /// let updated = client.v2_accounts().update("acct_v2_123", &params).await?;
    /// println!("Updated account: {}", updated.id());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        id: &str,
        params: &UpdateV2AccountParams,
    ) -> Result<V2Account, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!("{}/v2/core/accounts/{encoded}", self.client.config.api_base);
        let body_str = serde_json::to_string(params)?;
        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let mut req = self
            .client
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Idempotency-Key", &*idempotency_key)
            .body(body_str);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let account: V2Account = serde_json::from_value(val)?;
        Ok(account)
    }

    /// Closes and deletes a Stripe API v2 Core Account (`DELETE /v2/core/accounts/{id}`).
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if `id` is invalid,
    /// [`StripeError::Http`] on network failure, or [`StripeError::Api`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let closed = client.v2_accounts().del("acct_v2_123").await?;
    /// println!("Closed account: {}", closed.id());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn del(&self, id: &str) -> Result<V2Account, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!("{}/v2/core/accounts/{encoded}", self.client.config.api_base);

        let mut req = self
            .client
            .http
            .delete(&url)
            .header("Accept", "application/json");

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let account: V2Account = serde_json::from_value(val)?;
        Ok(account)
    }

    /// Fetches a paginated envelope of Stripe API v2 Core Accounts (`GET /v2/core/accounts`).
    ///
    /// # Errors
    /// Returns [`StripeError::Http`] on network failure or [`StripeError::Api`].
    pub async fn list_page(
        &self,
        params: Option<&ListV2AccountsParams>,
    ) -> Result<V2AccountList, StripeError> {
        let mut url = format!("{}/v2/core/accounts", self.client.config.api_base);
        append_list_query(&mut url, params);

        let mut req = self
            .client
            .http
            .get(&url)
            .header("Accept", "application/json");

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let page: V2AccountList = if val.is_array() {
            let data: Vec<V2Account> = serde_json::from_value(val)?;
            V2AccountList {
                object: "list".into(),
                data,
                next_page_url: None,
                previous_page_url: None,
                total_count: None,
            }
        } else {
            serde_json::from_value(val)?
        };

        Ok(page)
    }

    /// Lists Stripe API v2 Core Accounts matching the optional filter criteria (`GET /v2/core/accounts`).
    ///
    /// # Errors
    /// Returns [`StripeError::Http`] on network failure or [`StripeError::Api`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::v2_account::{ListV2AccountsParams, V2AccountStatus};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = ListV2AccountsParams::new()
    ///     .with_limit(10)
    ///     .with_status(V2AccountStatus::Active);
    ///
    /// let accounts = client.v2_accounts().list(Some(&params)).await?;
    /// for acct in accounts {
    ///     println!("Account ID: {}", acct.id());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        &self,
        params: Option<&ListV2AccountsParams>,
    ) -> Result<Vec<V2Account>, StripeError> {
        self.list_page(params).await.map(|page| page.data)
    }

    /// Generates a short-lived access token for a Stripe API v2 Core Account
    /// (`POST /v2/core/accounts/{id}/tokens`).
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if `id` or `params` fail validation,
    /// [`StripeError::Http`] on network failure, or [`StripeError::Api`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::v2_account::CreateV2AccountTokenParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreateV2AccountTokenParams::new()
    ///     .with_ttl_seconds(3600);
    ///
    /// let token = client.v2_accounts().create_token("acct_v2_123", &params).await?;
    /// println!("Generated token for account: {}", token.account_id());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_token(
        &self,
        id: &str,
        params: &CreateV2AccountTokenParams,
    ) -> Result<V2AccountToken, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!(
            "{}/v2/core/accounts/{encoded}/tokens",
            self.client.config.api_base
        );
        let body_str = serde_json::to_string(params)?;
        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let mut req = self
            .client
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Idempotency-Key", &*idempotency_key)
            .body(body_str);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let token: V2AccountToken = serde_json::from_value(val)?;
        Ok(token)
    }
}

fn append_list_query(url: &mut String, params: Option<&ListV2AccountsParams>) {
    let Some(p) = params else {
        return;
    };
    let pairs = p.to_query_pairs();
    if pairs.is_empty() {
        return;
    }
    let query_str = pairs
        .into_iter()
        .map(|(k, v)| format!("{k}={}", some_stripe_core::patterns::percent_encode(&v)))
        .collect::<Vec<_>>()
        .join("&");
    url.push('?');
    url.push_str(&query_str);
}
