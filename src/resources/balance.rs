//! Stripe Balance and BalanceTransactions resource implementations.
//!
//! Provides access to merchant funds and immutable balance transaction ledgers,
//! enforcing ISO 4217 minor currency integer safety and double-entry accounting invariants.

use some_stripe_core::patterns::pagination::Paginator;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{Balance, BalanceTransaction, balance_transaction_id};

/// Resource for inspecting the merchant's account balance (`stripe.balance`).
///
/// Wraps the `GET /v1/balance` endpoint, exposing real-time available, pending,
/// and instant payout balances across all operating currencies.
#[derive(Clone, Copy)]
pub struct BalanceResource<'a> {
    client: &'a StripeClient,
}

impl<'a> BalanceResource<'a> {
    /// Creates a new `BalanceResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Retrieves current account balance funds (`GET /v1/balance`).
    ///
    /// Returns breakdown of funds that are available to be paid out, funds that are pending,
    /// and funds eligible for instant payout. All amounts are strictly expressed in
    /// ISO 4217 minor currency integer units ([`i64`]).
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::AuthenticationError`] if the configured secret key is unauthorized.
    /// - Returns [`StripeError::Api`] if the Stripe API returns an HTTP error status code.
    /// - Returns [`StripeError::Json`] if the response body cannot be deserialized into a [`Balance`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let balance = client.balance().retrieve().await?;
    ///
    /// for fund in &balance.available {
    ///     println!("Available: {} {}", fund.amount, fund.currency);
    /// }
    /// for fund in &balance.pending {
    ///     println!("Pending: {} {}", fund.amount, fund.currency);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self) -> Result<Balance, StripeError> {
        let url = format!("{}/v1/balance", self.client.config.api_base);
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let balance: Balance = serde_json::from_value(val)?;
        Ok(balance)
    }
}

/// Resource for inspecting and listing balance transactions (`stripe.balance_transactions`).
///
/// Wraps endpoints for retrieving individual balance transaction records (`GET /v1/balance_transactions/{id}`)
/// or streaming transactions via cursor-based pagination (`GET /v1/balance_transactions`).
///
/// # Accounting Invariant
///
/// Every balance transaction adheres to the fundamental double-entry ledger invariant:
/// `net == amount - fee`. You can assert this invariant programmatically using
/// [`BalanceTransaction::verify_accounting_invariant`].
#[derive(Clone, Copy)]
pub struct BalanceTransactionsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> BalanceTransactionsResource<'a> {
    /// Creates a new `BalanceTransactionsResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Retrieves an existing balance transaction by ID (`GET /v1/balance_transactions/{id}`).
    ///
    /// Validates `id` to prevent path traversal and injection attacks before transmitting.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `id` is empty, contains whitespace, or contains
    ///   forbidden characters (`..`, `/`, `\`, `?`, `#`).
    /// - Returns [`StripeError::Api`] if the balance transaction ID does not exist or the API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into a [`BalanceTransaction`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let txn = client.balance_transactions().retrieve("txn_1032Hu2eZvKYlo2CEPtcnUvl").await?;
    ///
    /// println!("Transaction ID: {}", txn.id);
    /// println!("Net amount: {} {}", txn.net, txn.currency);
    /// assert!(txn.verify_accounting_invariant());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, id: &str) -> Result<BalanceTransaction, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/balance_transactions/{encoded}",
            self.client.config.api_base
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let txn: BalanceTransaction = serde_json::from_value(val)?;
        Ok(txn)
    }

    /// Returns a zero-clone paginator streaming balance transactions (`GET /v1/balance_transactions`).
    ///
    /// Employs cursor-based pagination (`starting_after`) with built-in loop detection
    /// to safely iterate through pages of balance ledger entries.
    ///
    /// # Parameters
    ///
    /// - `limit`: Optional maximum number of entries to return per page (defaults to 10 if `None`).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let mut paginator = client.balance_transactions().list(Some(25));
    ///
    /// while let Some(txn) = paginator.next_item().await? {
    ///     println!("Transaction: {} (net: {} {})", txn.id, txn.net, txn.currency);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list(&self, limit: Option<u32>) -> Paginator<'a, BalanceTransaction> {
        Paginator::new(
            self.client,
            "v1/balance_transactions",
            limit.unwrap_or(10),
            balance_transaction_id,
        )
    }
}
