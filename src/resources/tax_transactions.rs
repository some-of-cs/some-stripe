//! Stripe Tax Transactions resource implementation.
//!
//! Provides endpoints to commit tax liabilities to official reporting ledgers
//! from tax calculations, record tax reversals, retrieve transactions,
//! and streamingly paginate transaction line items.

use some_stripe_core::patterns::pagination::Paginator;
use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CreateTaxTransactionFromCalculationParams, CreateTaxTransactionReversalParams,
    ListTaxTransactionLineItemsParams, TaxTransaction, TaxTransactionLineItem,
    tax_transaction_line_item_id,
};

/// Resource for managing Stripe Tax Transactions (`POST /v1/tax/transactions/*`).
///
/// Wraps endpoints for creating transactions from calculations (`POST /v1/tax/transactions/create_from_calculation`),
/// creating transaction reversals (`POST /v1/tax/transactions/create_reversal`),
/// retrieving existing transactions (`GET /v1/tax/transactions/{id}`),
/// and listing line items (`GET /v1/tax/transactions/{id}/line_items`).
#[derive(Clone, Copy)]
pub struct TransactionsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> TransactionsResource<'a> {
    /// Creates a new `TransactionsResource` bound to the provided [`StripeClient`].
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Commits a tax liability from an existing Tax Calculation into an official ledger transaction (`POST /v1/tax/transactions/create_from_calculation`).
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `calculation` or `reference` is empty or whitespace.
    /// - Returns [`StripeError::Api`] if the Stripe API returns an error or calculation is expired.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into a [`TaxTransaction`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::CreateTaxTransactionFromCalculationParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreateTaxTransactionFromCalculationParams::new("taxcalc_123", "order_ref_456");
    /// let tx = client.tax_transactions().create_from_calculation(&params).await?;
    /// println!("Created tax transaction: {}", tx.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_from_calculation(
        &self,
        params: &CreateTaxTransactionFromCalculationParams,
    ) -> Result<TaxTransaction, StripeError> {
        if params.calculation.trim().is_empty() {
            return Err(StripeError::InvalidRequest(
                "Calculation ID must not be empty".into(),
            ));
        }
        if params.reference.trim().is_empty() {
            return Err(StripeError::InvalidRequest(
                "Transaction reference must not be empty".into(),
            ));
        }

        let url = format!(
            "{}/v1/tax/transactions/create_from_calculation",
            self.client.config.api_base
        );
        let form_data = to_form_params(params)?;
        let idempotency_key =
            crate::resources::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let tx: TaxTransaction = serde_json::from_value(val)?;
        Ok(tx)
    }

    /// Creates a full or partial reversal for an existing tax transaction (`POST /v1/tax/transactions/create_reversal`).
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `original_transaction` or `reference` is empty.
    /// - Returns [`StripeError::Api`] if the API returns an error or reversal parameters are invalid.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into a [`TaxTransaction`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::CreateTaxTransactionReversalParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreateTaxTransactionReversalParams::full("tax_tx_123", "refund_ref_456");
    /// let reversal = client.tax_transactions().create_reversal(&params).await?;
    /// println!("Created tax reversal: {}", reversal.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_reversal(
        &self,
        params: &CreateTaxTransactionReversalParams,
    ) -> Result<TaxTransaction, StripeError> {
        if params.original_transaction.trim().is_empty() {
            return Err(StripeError::InvalidRequest(
                "Original transaction ID must not be empty".into(),
            ));
        }
        if params.reference.trim().is_empty() {
            return Err(StripeError::InvalidRequest(
                "Reversal reference must not be empty".into(),
            ));
        }

        let url = format!(
            "{}/v1/tax/transactions/create_reversal",
            self.client.config.api_base
        );
        let form_data = to_form_params(params)?;
        let idempotency_key =
            crate::resources::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let tx: TaxTransaction = serde_json::from_value(val)?;
        Ok(tx)
    }

    /// Retrieves an existing tax transaction (`GET /v1/tax/transactions/{id}`).
    ///
    /// Validates `id` against path-traversal attacks prior to dispatch.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `id` is empty, contains invalid characters, or fails path validation.
    /// - Returns [`StripeError::Api`] if the transaction does not exist or API request fails.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into a [`TaxTransaction`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let tx = client.tax_transactions().retrieve("tax_tx_123").await?;
    /// println!("Retrieved tax transaction: {}", tx.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, id: &str) -> Result<TaxTransaction, StripeError> {
        let encoded_id = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/tax/transactions/{encoded_id}",
            self.client.config.api_base
        );

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let tx: TaxTransaction = serde_json::from_value(val)?;
        Ok(tx)
    }

    /// Returns an asynchronous streaming [`Paginator`] for auto-paginating through line items of a tax transaction (`GET /v1/tax/transactions/{id}/line_items`).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::ListTaxTransactionLineItemsParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = ListTaxTransactionLineItemsParams::new().with_limit(10);
    /// let mut stream = client.tax_transactions().list_line_items("tax_tx_123", &params);
    /// while let Some(item) = stream.next_item().await? {
    ///     println!("Line item: {} - amount: {}", item.id, item.amount);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_line_items(
        &self,
        transaction_id: &str,
        params: &ListTaxTransactionLineItemsParams,
    ) -> Paginator<'a, TaxTransactionLineItem> {
        let (path, initial_err) = match validate_and_encode_id(transaction_id) {
            Ok(encoded) => (format!("v1/tax/transactions/{encoded}/line_items"), None),
            Err(e) => (String::new(), Some(e)),
        };

        let mut paginator = Paginator::new(
            self.client,
            path,
            params.limit.unwrap_or(10),
            tax_transaction_line_item_id,
        );

        if let Some(err) = initial_err {
            paginator = paginator.with_initial_error(err);
        }
        if let Some(ref sa) = params.starting_after {
            paginator = paginator.starting_after(sa);
        }
        if let Some(ref eb) = params.ending_before {
            paginator = paginator.ending_before(eb);
        }

        paginator
    }

    /// Convenience helper returning an auto-paginator for tax transaction line items with default page limit (10).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let mut stream = client.tax_transactions().list_line_items_default("tax_tx_123");
    /// while let Some(item) = stream.next_item().await? {
    ///     println!("Line item: {}", item.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_line_items_default(
        &self,
        transaction_id: &str,
    ) -> Paginator<'a, TaxTransactionLineItem> {
        self.list_line_items(
            transaction_id,
            &ListTaxTransactionLineItemsParams::default(),
        )
    }
}
