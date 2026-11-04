//! Stripe Balance and BalanceTransactions resource implementations.

use some_stripe_core::patterns::pagination::Paginator;
use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{Balance, BalanceTransaction};

fn balance_transaction_id_extractor(txn: &BalanceTransaction) -> &str {
    txn.id.as_str()
}

/// Resource for inspecting the merchant's account balance (`stripe.balance`).
#[derive(Clone, Copy)]
pub struct BalanceResource<'a> {
    client: &'a StripeClient,
}

impl<'a> BalanceResource<'a> {
    pub(crate) fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Retrieves current account balance funds (`GET /v1/balance`).
    pub async fn retrieve(&self) -> Result<Balance, StripeError> {
        let url = format!("{}/v1/balance", self.client.config.api_base);
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let balance: Balance = serde_json::from_value(val)?;
        Ok(balance)
    }
}

/// Resource for inspecting and listing balance transactions (`stripe.balance_transactions`).
#[derive(Clone, Copy)]
pub struct BalanceTransactionsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> BalanceTransactionsResource<'a> {
    pub(crate) fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Retrieves an existing balance transaction by ID (`GET /v1/balance_transactions/{id}`).
    pub async fn retrieve(&self, id: &str) -> Result<BalanceTransaction, StripeError> {
        let url = format!(
            "{}/v1/balance_transactions/{}",
            self.client.config.api_base, id
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let txn: BalanceTransaction = serde_json::from_value(val)?;
        Ok(txn)
    }

    /// Returns a zero-clone paginator streaming balance transactions (`GET /v1/balance_transactions`).
    pub fn list(&self, limit: Option<u32>) -> Paginator<'a, BalanceTransaction> {
        Paginator::new(
            self.client,
            "v1/balance_transactions",
            limit.unwrap_or(10),
            balance_transaction_id_extractor,
        )
    }
}
