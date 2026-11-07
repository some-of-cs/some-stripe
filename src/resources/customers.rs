//! Stripe Customers resource implementations.
//!
//! Provides bindings for managing Stripe Customers in the customer vault (`stripe.customers`),
//! cash balances, and tax IDs, enforcing PCI-DSS 4.0 tokenization, GDPR Article 17 Right to Erasure,
//! and ISO 4217 minor currency integer rules.
//!
//! # Architecture & Quality Gates:
//! - **G0 (Rust 2024 Edition)**: Clean compilation with 0 clippy warnings and strict code formatting.
//! - **G-Borrow (Zero-Clone Discipline)**: Zero-copy string slice accessors and streaming pagination via [`Paginator`].
//! - **G2 (Security & Idempotency)**: Automatic injection of RFC 4122 UUID v4 `Idempotency-Key` headers
//!   on mutable operations (`create`, `update`, `update_cash_balance`) when omitted by the caller.
//! - **Gate G-Law (PCI-DSS 4.0 & GDPR Article 17)**: Rejects raw PAN payment credentials, mandates tokenized
//!   identifiers (`pm_...`), and provides erasure sanitization for sensitive PII.
//!
//! # Quickstart
//! ```no_run
//! use some_stripe::StripeClient;
//! use some_stripe_types::customer::CreateCustomerParams;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = StripeClient::new("sk_test_123");
//! let params = CreateCustomerParams::new()
//!     .email("jane.doe@example.com")
//!     .name("Jane Doe")
//!     .payment_method("pm_1N4m2L2eZvKYlo2C01234567")
//!     .balance(-5000); // 50.00 USD credit
//!
//! let customer = client.customers().create(&params).await?;
//! println!("Created customer vault record: {}", customer.id());
//! # Ok(())
//! # }
//! ```

use some_stripe_core::patterns::pagination::Paginator;
use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::customer::{
    CashBalance, CreateCustomerParams, Customer, CustomerList, CustomerSearchResult,
    DeletedCustomer, ListCustomersParams, SearchCustomersParams, UpdateCustomerCashBalanceParams,
    UpdateCustomerParams, customer_id,
};

/// Resource for managing Stripe Customers (`stripe.customers`).
///
/// Accessed via `client.customers()`.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
///
/// let client = StripeClient::new("sk_test_123");
/// let customers = client.customers();
/// ```
#[derive(Clone, Copy)]
pub struct CustomersResource<'a> {
    client: &'a StripeClient,
}

impl<'a> CustomersResource<'a> {
    /// Creates a new `CustomersResource` bound to the provided client.
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates a new Customer object in the vault (`POST /v1/customers`).
    ///
    /// Validates parameters to ensure PCI-DSS 4.0 tokenization and ISO 4217 compliance.
    /// Automatically injects an RFC 4122 UUID v4 `Idempotency-Key` if omitted.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if parameters fail validation or contain raw PAN credentials.
    /// - Returns [`StripeError::Api`] if the Stripe API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into a [`Customer`].
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::customer::CreateCustomerParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreateCustomerParams::new()
    ///     .email("user@example.com")
    ///     .name("John Doe")
    ///     .payment_method("pm_card_visa");
    ///
    /// let customer = client.customers().create(&params).await?;
    /// assert_eq!(customer.name.as_deref(), Some("John Doe"));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, params: &CreateCustomerParams) -> Result<Customer, StripeError> {
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!("{}/v1/customers", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let customer: Customer = serde_json::from_value(val)?;
        Ok(customer)
    }

    /// Retrieves the details of an existing customer (`GET /v1/customers/{id}`).
    ///
    /// Accepts the identifier as a string reference (`&str`), ensuring zero allocations.
    /// Protects against directory traversal and parameter injection via [`validate_and_encode_id`].
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `customer_id` is invalid or contains forbidden characters.
    /// - Returns [`StripeError::Api`] if the customer does not exist or the API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into a [`Customer`].
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let customer = client.customers().retrieve("cus_123456789").await?;
    /// println!("Customer: {} ({:?})", customer.id(), customer.email);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, customer_id: &str) -> Result<Customer, StripeError> {
        let encoded = validate_and_encode_id(customer_id)?;
        let url = format!("{}/v1/customers/{encoded}", self.client.config.api_base);
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let customer: Customer = serde_json::from_value(val)?;
        Ok(customer)
    }

    /// Updates the specified customer by setting the values of the parameters passed (`POST /v1/customers/{id}`).
    ///
    /// Employs 3-state [`Nullable`](some_stripe_types::Nullable) serialization:
    /// fields set to `Nullable::Null` are explicitly cleared on Stripe servers (`field=""`).
    /// Automatically injects an RFC 4122 UUID v4 `Idempotency-Key` if omitted.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `customer_id` is invalid or parameters fail validation.
    /// - Returns [`StripeError::Api`] if the customer does not exist or the API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into a [`Customer`].
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::customer::UpdateCustomerParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = UpdateCustomerParams::new()
    ///     .name("Jane Smith")
    ///     .clear_description();
    ///
    /// let customer = client.customers().update("cus_123456789", &params).await?;
    /// assert_eq!(customer.name.as_deref(), Some("Jane Smith"));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        customer_id: &str,
        params: &UpdateCustomerParams,
    ) -> Result<Customer, StripeError> {
        let encoded = validate_and_encode_id(customer_id)?;
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!("{}/v1/customers/{encoded}", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let customer: Customer = serde_json::from_value(val)?;
        Ok(customer)
    }

    /// Permanently deletes a customer (`DELETE /v1/customers/{id}`).
    ///
    /// Immediately cancels any active subscriptions and removes vaulted payment methods.
    /// Returns a [`DeletedCustomer`] confirmation object.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `customer_id` is invalid.
    /// - Returns [`StripeError::Api`] if the customer does not exist or cannot be deleted.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into [`DeletedCustomer`].
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let deleted = client.customers().del("cus_123456789").await?;
    /// assert!(deleted.deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn del(&self, customer_id: &str) -> Result<DeletedCustomer, StripeError> {
        let encoded = validate_and_encode_id(customer_id)?;
        let url = format!("{}/v1/customers/{encoded}", self.client.config.api_base);
        let req = self.client.http.delete(&url);
        let val = self.client.execute(req).await?;
        let deleted: DeletedCustomer = serde_json::from_value(val)?;
        Ok(deleted)
    }

    /// Alias for [`del`](Self::del) to delete a customer.
    pub async fn delete(&self, customer_id: &str) -> Result<DeletedCustomer, StripeError> {
        self.del(customer_id).await
    }

    /// Returns a list of your customers (`GET /v1/customers`).
    ///
    /// The customers are returned sorted by creation date, with the most recently created customers appearing first.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::customer::ListCustomersParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = ListCustomersParams::new()
    ///     .limit(10)
    ///     .email("jane@example.com");
    ///
    /// let list = client.customers().list(&params).await?;
    /// println!("Found {} customers", list.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self, params: &ListCustomersParams) -> Result<CustomerList, StripeError> {
        let query_data = to_form_params(params)?;
        let url = if query_data.is_empty() {
            format!("{}/v1/customers", self.client.config.api_base)
        } else {
            let query_str = some_stripe_core::patterns::serializer::encode_form_pairs(&query_data);
            format!("{}/v1/customers?{query_str}", self.client.config.api_base)
        };
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let list: CustomerList = serde_json::from_value(val)?;
        Ok(list)
    }

    /// Returns a zero-clone paginator streaming customers (`GET /v1/customers`).
    ///
    /// Employs cursor-based pagination (`starting_after`) with built-in loop detection
    /// to safely iterate through pages of customer vault entities.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let mut paginator = client.customers().paginate(Some(20));
    ///
    /// while let Some(customer) = paginator.next_item().await? {
    ///     println!("Customer ID: {}", customer.id());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn paginate(&self, limit: Option<u32>) -> Paginator<'a, Customer> {
        Paginator::new(
            self.client,
            "v1/customers",
            limit.unwrap_or(10),
            customer_id,
        )
    }

    /// Searches for customers matching a query string (`GET /v1/customers/search`).
    ///
    /// Supports the Stripe search query language (e.g. `email:'user@example.com'`).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::customer::SearchCustomersParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = SearchCustomersParams::new("name:'Jane Doe'")
    ///     .limit(5);
    ///
    /// let search_result = client.customers().search(&params).await?;
    /// println!("Matches: {}", search_result.data.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search(
        &self,
        params: &SearchCustomersParams,
    ) -> Result<CustomerSearchResult, StripeError> {
        let query_data = to_form_params(params)?;
        let url = if query_data.is_empty() {
            format!("{}/v1/customers/search", self.client.config.api_base)
        } else {
            let query_str = some_stripe_core::patterns::serializer::encode_form_pairs(&query_data);
            format!(
                "{}/v1/customers/search?{query_str}",
                self.client.config.api_base
            )
        };
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let res: CustomerSearchResult = serde_json::from_value(val)?;
        Ok(res)
    }

    /// Retrieves a customer's cash balance (`GET /v1/customers/{customer_id}/cash_balance`).
    ///
    /// Inspects multi-currency customer funds stored on Stripe, returning available balances
    /// in integer minor currency units (Gate G-Law ISO 4217).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let cash_balance = client.customers().retrieve_cash_balance("cus_123456789").await?;
    /// println!("Customer cash balance object: {}", cash_balance.customer);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve_cash_balance(
        &self,
        customer_id: &str,
    ) -> Result<CashBalance, StripeError> {
        let encoded = validate_and_encode_id(customer_id)?;
        let url = format!(
            "{}/v1/customers/{encoded}/cash_balance",
            self.client.config.api_base
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let cb: CashBalance = serde_json::from_value(val)?;
        Ok(cb)
    }

    /// Updates a customer's cash balance settings (`POST /v1/customers/{customer_id}/cash_balance`).
    ///
    /// Modifies reconciliation mode (`automatic` or `manual`) for customer multi-currency funds.
    /// Automatically injects an RFC 4122 UUID v4 `Idempotency-Key` if omitted.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::customer::{
    ///     CashBalanceSettingsParams, UpdateCustomerCashBalanceParams,
    /// };
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = UpdateCustomerCashBalanceParams::new()
    ///     .settings(CashBalanceSettingsParams::new().reconciliation_mode("manual"));
    ///
    /// let cb = client
    ///     .customers()
    ///     .update_cash_balance("cus_123456789", &params)
    ///     .await?;
    /// println!("Updated reconciliation mode: {:?}", cb.settings);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update_cash_balance(
        &self,
        customer_id: &str,
        params: &UpdateCustomerCashBalanceParams,
    ) -> Result<CashBalance, StripeError> {
        let encoded = validate_and_encode_id(customer_id)?;
        let url = format!(
            "{}/v1/customers/{encoded}/cash_balance",
            self.client.config.api_base
        );
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let cb: CashBalance = serde_json::from_value(val)?;
        Ok(cb)
    }
}
