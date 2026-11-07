//! Stripe Issuing Transactions resource implementations.
//!
//! Provides endpoints to retrieve, update, and list cardholder transactions
//! (`/v1/issuing/transactions`), enforcing strict input sanitization, zero-clone borrowing,
//! and immutable ledger compliance.

use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    IssuingTransaction, IssuingTransactionList, ListIssuingTransactionsParams,
    UpdateIssuingTransactionParams,
};

/// Resource for managing Stripe Issuing Transactions (`stripe.issuing().transactions()` or direct).
///
/// Wraps endpoints for retrieving transactions (`GET /v1/issuing/transactions/{id}`),
/// updating metadata (`POST /v1/issuing/transactions/{id}`), and listing transactions
/// (`GET /v1/issuing/transactions`).
#[derive(Clone, Copy)]
pub struct IssuingTransactionsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> IssuingTransactionsResource<'a> {
    /// Creates a new `IssuingTransactionsResource` bound to the provided [`StripeClient`].
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Sets the `Stripe-Account` header for multi-tenant Connect operations.
    #[must_use]
    pub fn with_account(mut self, account: &'a str) -> Self {
        self.account = Some(account);
        self
    }

    /// Returns a reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Retrieves an existing issuing transaction (`GET /v1/issuing/transactions/{id}`).
    ///
    /// Validates `id` to prevent path traversal and injection attacks before transmitting.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `id` is empty, contains whitespace,
    ///   or includes invalid characters such as `'..'`, `'/'`, `'\\'`, `'?'`, or `'#'`.
    /// - Returns [`StripeError::Api`] if the transaction does not exist or the Stripe API returns an error.
    /// - Returns [`StripeError::Json`] if the response body cannot be deserialized into an [`IssuingTransaction`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let txn = client.issuing_transactions().retrieve("ipi_12345").await?;
    ///
    /// println!("Transaction ID: {}", txn.id);
    /// println!("Amount: {} {}", txn.amount, txn.currency);
    /// println!("Type: {:?}", txn.r#type);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, id: &str) -> Result<IssuingTransaction, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/issuing/transactions/{encoded}",
            self.client.config.api_base
        );
        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }
        let val = self.client.execute(req).await?;
        let txn: IssuingTransaction = serde_json::from_value(val)?;
        Ok(txn)
    }

    /// Updates specified properties of an existing issuing transaction (`POST /v1/issuing/transactions/{id}`).
    ///
    /// Primarily used to attach or update custom key-value `metadata`.
    ///
    /// Automatically injects an RFC 4122 compliant UUID v4 `Idempotency-Key` header
    /// if none is provided in `params`.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `id` fails path traversal checks.
    /// - Returns [`StripeError::Api`] if the API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into an [`IssuingTransaction`].
    pub async fn update(
        &self,
        id: &str,
        params: &UpdateIssuingTransactionParams,
    ) -> Result<IssuingTransaction, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/issuing/transactions/{encoded}",
            self.client.config.api_base
        );
        let form_data = to_form_params(params)?;
        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

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
        let txn: IssuingTransaction = serde_json::from_value(val)?;
        Ok(txn)
    }

    /// Lists issuing transactions matching the specified query criteria (`GET /v1/issuing/transactions`).
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::Api`] if the Stripe API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into an [`IssuingTransactionList`].
    pub async fn list(
        &self,
        params: Option<&ListIssuingTransactionsParams>,
    ) -> Result<IssuingTransactionList, StripeError> {
        let mut url = format!("{}/v1/issuing/transactions", self.client.config.api_base);
        let query_entries = build_transactions_query(params);
        if !query_entries.is_empty() {
            let serialized = query_entries
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&");
            url.push('?');
            url.push_str(&serialized);
        }

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let list: IssuingTransactionList = serde_json::from_value(val)?;
        Ok(list)
    }
}

/// Builds flat key-value pairs for listing issuing transactions with low branching complexity.
fn build_transactions_query(
    params: Option<&ListIssuingTransactionsParams>,
) -> Vec<(&'static str, String)> {
    let mut query = Vec::new();
    let Some(p) = params else {
        return query;
    };
    if let Some(ref card) = p.card {
        query.push(("card", card.clone()));
    }
    if let Some(ref cardholder) = p.cardholder {
        query.push(("cardholder", cardholder.clone()));
    }
    if let Some(r#type) = p.r#type {
        query.push(("type", r#type.to_string()));
    }
    if let Some(created) = p.created {
        query.push(("created", created.to_string()));
    }
    if let Some(limit) = p.limit {
        query.push(("limit", limit.to_string()));
    }
    if let Some(ref after) = p.starting_after {
        query.push(("starting_after", after.clone()));
    }
    if let Some(ref before) = p.ending_before {
        query.push(("ending_before", before.clone()));
    }
    query
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_transactions_query_empty() {
        let q = build_transactions_query(None);
        assert!(q.is_empty());
    }

    #[test]
    fn test_build_transactions_query_populated() {
        use some_stripe_types::IssuingTransactionType;

        let params = ListIssuingTransactionsParams::new()
            .with_card("ic_card_abc")
            .with_cardholder("ich_holder_xyz")
            .with_type(IssuingTransactionType::Capture)
            .with_limit(20)
            .with_starting_after("ipi_prev");

        let q = build_transactions_query(Some(&params));
        assert_eq!(q.len(), 5);
        assert!(q.contains(&("card", "ic_card_abc".to_string())));
        assert!(q.contains(&("cardholder", "ich_holder_xyz".to_string())));
        assert!(q.contains(&("type", "capture".to_string())));
        assert!(q.contains(&("limit", "20".to_string())));
        assert!(q.contains(&("starting_after", "ipi_prev".to_string())));
    }

    #[test]
    fn test_resource_instantiation() {
        let client = StripeClient::new("sk_test_mock");
        let resource = IssuingTransactionsResource::new(&client).with_account("acct_connect_2");
        assert_eq!(resource.account, Some("acct_connect_2"));
    }
}
