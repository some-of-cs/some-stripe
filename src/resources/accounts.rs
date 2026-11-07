//! Stripe Accounts resource implementations.
//!
//! Provides access to merchant and connected account details via the Stripe REST API,
//! adhering to strict borrowing discipline, tenant isolation, and input validation invariants.

use some_stripe_core::patterns::pagination::Paginator;
use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::account::{
    Account, AccountList, CreateAccountParams, DeletedAccount, ListAccountsParams,
    RejectAccountParams, UpdateAccountParams, account_id,
};

/// Resource for managing Stripe Accounts (`stripe.accounts`).
///
/// Wraps endpoints for retrieving the authenticated merchant's profile (`GET /v1/account`)
/// or inspecting, creating, updating, rejecting, and deleting connected accounts by ID.
///
/// # Security & Input Invariants
///
/// Account identifiers passed to resource methods are strictly validated
/// and percent-encoded via [`validate_and_encode_id`], protecting against directory traversal
/// (`..`, `/`, `\`) and parameter injection attacks (`?`, `#`).
#[derive(Clone, Copy)]
pub struct AccountsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> AccountsResource<'a> {
    /// Creates a new `AccountsResource` bound to the provided [`StripeClient`].
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
    pub fn with_account(&self, account_id: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account_id),
        }
    }

    /// Retrieves the details of the merchant's account associated with the secret key (`GET /v1/account`).
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::AuthenticationError`] if the configured API secret key is invalid or unauthorized.
    /// - Returns [`StripeError::Api`] if the Stripe API returns a non-2xx status code.
    /// - Returns [`StripeError::Json`] if the response body cannot be deserialized into an [`Account`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let account = client.accounts().retrieve().await?;
    ///
    /// println!("Authenticated account ID: {}", account.id);
    /// println!("Charges enabled: {:?}", account.charges_enabled);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self) -> Result<Account, StripeError> {
        let url = format!("{}/v1/account", self.client.config.api_base);
        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }
        let val = self.client.execute(req).await?;
        let account: Account = serde_json::from_value(val)?;
        Ok(account)
    }

    /// Alias for [`retrieve`](Self::retrieve) to access the authenticated merchant's account.
    ///
    /// # Errors
    ///
    /// Returns [`StripeError`] if the API request fails or the response payload cannot be parsed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let account = client.accounts().retrieve_current().await?;
    ///
    /// assert!(!account.id.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve_current(&self) -> Result<Account, StripeError> {
        self.retrieve().await
    }

    /// Retrieves an account by its unique identifier (`GET /v1/accounts/{id}`).
    ///
    /// Validates `account_id` to prevent directory traversal and parameter injection
    /// attacks before issuing the HTTP request.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `account_id` is empty, contains whitespace,
    ///   or includes invalid characters such as `'..'`, `'/'`, `'\\'`, `'?'`, or `'#'`.
    /// - Returns [`StripeError::Api`] if the account ID does not exist or the API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into an [`Account`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let account = client.accounts().retrieve_by_id("acct_1032D82eZvKYlo2C").await?;
    ///
    /// println!("Retrieved connected account: {}", account.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve_by_id(&self, account_id: &str) -> Result<Account, StripeError> {
        let encoded = validate_and_encode_id(account_id)?;
        let url = format!("{}/v1/accounts/{encoded}", self.client.config.api_base);
        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }
        let val = self.client.execute(req).await?;
        let account: Account = serde_json::from_value(val)?;
        Ok(account)
    }

    /// Creates a new connected account (`POST /v1/accounts`).
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if parameters fail validation.
    /// - Returns [`StripeError::Api`] if Stripe returns an API error.
    /// - Returns [`StripeError::Json`] if deserialization fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::account::{AccountType, CreateAccountParams};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreateAccountParams::new()
    ///     .account_type(AccountType::Standard)
    ///     .country("US")
    ///     .email("merchant@example.com");
    ///
    /// let account = client.accounts().create(&params).await?;
    /// println!("Created account: {}", account.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, params: &CreateAccountParams) -> Result<Account, StripeError> {
        let url = format!("{}/v1/accounts", self.client.config.api_base);
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
        let account: Account = serde_json::from_value(val)?;
        Ok(account)
    }

    /// Updates the specified account by setting the values of the parameters passed (`POST /v1/accounts/{id}`).
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `account_id` is invalid.
    /// - Returns [`StripeError::Api`] if the account does not exist or update fails.
    /// - Returns [`StripeError::Json`] if deserialization fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::account::UpdateAccountParams;
    /// use some_stripe_types::Nullable;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = UpdateAccountParams::new()
    ///     .email(Nullable::Value("updated@example.com".into()));
    ///
    /// let account = client.accounts().update("acct_1032D82eZvKYlo2C", &params).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        account_id: &str,
        params: &UpdateAccountParams,
    ) -> Result<Account, StripeError> {
        let encoded = validate_and_encode_id(account_id)?;
        let url = format!("{}/v1/accounts/{encoded}", self.client.config.api_base);
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
        let account: Account = serde_json::from_value(val)?;
        Ok(account)
    }

    /// Permanently deletes an account (`DELETE /v1/accounts/{id}`).
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `account_id` is invalid.
    /// - Returns [`StripeError::Api`] if the account does not exist or deletion fails.
    /// - Returns [`StripeError::Json`] if deserialization fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let deleted = client.accounts().del("acct_1032D82eZvKYlo2C").await?;
    /// assert!(deleted.deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn del(&self, account_id: &str) -> Result<DeletedAccount, StripeError> {
        let encoded = validate_and_encode_id(account_id)?;
        let url = format!("{}/v1/accounts/{encoded}", self.client.config.api_base);
        let mut req = self.client.http.delete(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }
        let val = self.client.execute(req).await?;
        let deleted: DeletedAccount = serde_json::from_value(val)?;
        Ok(deleted)
    }

    /// Alias for [`del`](Self::del) to delete an account.
    pub async fn delete(&self, account_id: &str) -> Result<DeletedAccount, StripeError> {
        self.del(account_id).await
    }

    /// Rejects an account with a given reason (`POST /v1/accounts/{id}/reject`).
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `account_id` is invalid.
    /// - Returns [`StripeError::Api`] if reject fails.
    /// - Returns [`StripeError::Json`] if deserialization fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::account::RejectAccountParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = RejectAccountParams::new("fraud");
    /// let account = client.accounts().reject("acct_1032D82eZvKYlo2C", &params).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn reject(
        &self,
        account_id: &str,
        params: &RejectAccountParams,
    ) -> Result<Account, StripeError> {
        let encoded = validate_and_encode_id(account_id)?;
        let url = format!(
            "{}/v1/accounts/{encoded}/reject",
            self.client.config.api_base
        );
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
        let account: Account = serde_json::from_value(val)?;
        Ok(account)
    }

    /// Returns a list of connected accounts (`GET /v1/accounts`).
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::Api`] if list retrieval fails.
    /// - Returns [`StripeError::Json`] if deserialization fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::account::ListAccountsParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = ListAccountsParams::new().limit(10);
    /// let list = client.accounts().list(&params).await?;
    /// println!("Total accounts: {}", list.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self, params: &ListAccountsParams) -> Result<AccountList, StripeError> {
        let query_data = to_form_params(params)?;
        let url = if query_data.is_empty() {
            format!("{}/v1/accounts", self.client.config.api_base)
        } else {
            let query_str = some_stripe_core::patterns::serializer::encode_form_pairs(&query_data);
            format!("{}/v1/accounts?{query_str}", self.client.config.api_base)
        };
        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }
        let val = self.client.execute(req).await?;
        let list: AccountList = serde_json::from_value(val)?;
        Ok(list)
    }

    /// Returns a zero-clone paginator streaming accounts (`GET /v1/accounts`).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let mut paginator = client.accounts().paginate(Some(10));
    /// while let Some(account) = paginator.next_item().await? {
    ///     println!("Account: {}", account.id());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn paginate(&self, limit: Option<u32>) -> Paginator<'a, Account> {
        Paginator::new(self.client, "v1/accounts", limit.unwrap_or(10), account_id)
    }
}
