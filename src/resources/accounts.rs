//! Stripe Accounts resource implementations.
//!
//! Provides access to merchant and connected account details via the Stripe REST API,
//! adhering to strict borrowing discipline and input validation invariants.

use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::Account;

/// Resource for managing Stripe Accounts (`stripe.accounts`).
///
/// Wraps endpoints for retrieving the authenticated merchant's profile (`GET /v1/account`)
/// or inspecting connected accounts by ID (`GET /v1/accounts/{id}`).
///
/// # Security & Input Invariants
///
/// Account identifiers passed to [`retrieve_by_id`](Self::retrieve_by_id) are strictly validated
/// and percent-encoded via [`validate_and_encode_id`], protecting against directory traversal
/// (`..`, `/`, `\`) and parameter injection attacks (`?`, `#`).
#[derive(Clone, Copy)]
pub struct AccountsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> AccountsResource<'a> {
    /// Creates a new `AccountsResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
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
        let req = self.client.http.get(&url);
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
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let account: Account = serde_json::from_value(val)?;
        Ok(account)
    }
}
