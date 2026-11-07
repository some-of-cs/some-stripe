//! Stripe Tax Calculations resource implementation (`stripe.tax.calculations`).
//!
//! Provides endpoints to compute real-time sales tax, VAT, and GST liabilities across 50+ jurisdictions,
//! expanding itemized tax breakdowns, and retrieving paginated line items.
//!
//! # Architecture & Quality Gates:
//! - **Gate G-Borrow (Zero-Clone Discipline)**: Parameter borrowing by reference (`&CreateTaxCalculationParams`),
//!   zero deep `.clone()` calls on entities in request pipelines, and borrowed string slices for accessors.
//! - **Gate G2 (Security & Idempotency)**: Automatic injection of RFC 4122 UUID v4 `Idempotency-Key` headers
//!   on mutable operations (`create`) when omitted by the caller via [`super::resolve_idempotency_key`].
//! - **Security Invariant**: Strict path traversal prevention via [`validate_and_encode_id`].
//! - **Gate G-Law (ISO 4217 & EU VAT Directive 2006/112/EC)**: Integer minor currency units precision and
//!   support for EU reverse charge zero-rate evaluations.
//! - **Gate G-Complexity**: Cyclomatic complexity <= 6 across all resource methods.
//! - **Gate G-Docs**: Runnable doc-tests verified by `cargo test --doc`.

use some_stripe_core::patterns::pagination::Paginator;
use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CreateTaxCalculationParams, ListTaxCalculationLineItemsParams, TaxCalculation,
    TaxCalculationLineItem, TaxCalculationLineItemList, tax_calculation_line_item_id,
};

/// Resource for managing Stripe Tax Calculations (`stripe.tax.calculations`).
///
/// Enables real-time tax calculation for shopping carts, checkout flows, and custom invoicing engines.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
/// use some_stripe::resources::CalculationsResource;
/// use some_stripe_types::{
///     CreateTaxCalculationParams, CustomerTaxAddress, CustomerTaxDetails, TaxLineItemParam,
/// };
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let client = StripeClient::new("sk_test_123");
/// let resource = CalculationsResource::new(&client);
///
/// let params = CreateTaxCalculationParams::new("usd")
///     .customer_details(
///         CustomerTaxDetails::new()
///             .address(CustomerTaxAddress::new("US").state("CA").postal_code("94103"))
///             .address_source("shipping"),
///     )
///     .line_item(TaxLineItemParam::new(10000, "item_1").tax_code("txcd_10000000"));
///
/// let calculation = resource.create(&params).await?;
/// println!("Calculated exclusive tax: {}", calculation.tax_amount_exclusive);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy)]
pub struct CalculationsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> CalculationsResource<'a> {
    /// Creates a new `CalculationsResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates a new Tax Calculation (`POST /v1/tax/calculations`).
    ///
    /// Evaluates applicable tax rates and returns the calculated tax liability and itemized breakdown.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe::resources::CalculationsResource;
    /// use some_stripe_types::{CreateTaxCalculationParams, CustomerTaxAddress, CustomerTaxDetails, TaxLineItemParam};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let resource = CalculationsResource::new(&client);
    /// let params = CreateTaxCalculationParams::new("usd")
    ///     .customer_details(
    ///         CustomerTaxDetails::new()
    ///             .address(CustomerTaxAddress::new("US").state("CA").postal_code("94103"))
    ///             .address_source("shipping")
    ///     )
    ///     .line_item(TaxLineItemParam::new(10000, "item_1"));
    ///
    /// let calculation = resource.create(&params).await?;
    /// assert_eq!(calculation.currency, "usd");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        params: &CreateTaxCalculationParams,
    ) -> Result<TaxCalculation, StripeError> {
        let url = format!("{}/v1/tax/calculations", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let calculation: TaxCalculation = serde_json::from_value(val)?;
        Ok(calculation)
    }

    /// Retrieves an existing tax calculation by ID (`GET /v1/tax/calculations/{id}`).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe::resources::CalculationsResource;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let resource = CalculationsResource::new(&client);
    ///
    /// let calculation = resource.retrieve("taxcalc_12345").await?;
    /// println!("Retrieved tax calculation: {}", calculation.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, calculation_id: &str) -> Result<TaxCalculation, StripeError> {
        let safe_id = validate_and_encode_id(calculation_id)?;
        let url = format!(
            "{}/v1/tax/calculations/{}",
            self.client.config.api_base, safe_id
        );

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let calculation: TaxCalculation = serde_json::from_value(val)?;
        Ok(calculation)
    }

    /// Returns a zero-clone streaming [`Paginator`] for line items of a tax calculation (`GET /v1/tax/calculations/{id}/line_items`).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe::resources::CalculationsResource;
    /// use some_stripe_types::ListTaxCalculationLineItemsParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let resource = CalculationsResource::new(&client);
    /// let params = ListTaxCalculationLineItemsParams::new().limit(25);
    ///
    /// let mut paginator = resource.list_line_items("taxcalc_123", &params);
    /// while let Some(item) = paginator.next_item().await? {
    ///     println!("Line item: {} Amount: {}", item.id, item.amount);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_line_items(
        &self,
        calculation_id: &str,
        params: &ListTaxCalculationLineItemsParams,
    ) -> Paginator<'a, TaxCalculationLineItem> {
        let (path, initial_err) = match validate_and_encode_id(calculation_id) {
            Ok(encoded) => (format!("v1/tax/calculations/{encoded}/line_items"), None),
            Err(e) => (String::new(), Some(e)),
        };

        let mut paginator = Paginator::new(
            self.client,
            path,
            params.limit.unwrap_or(10),
            tax_calculation_line_item_id,
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

    /// Retrieves a single paginated list page of line items (`GET /v1/tax/calculations/{id}/line_items`).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe::resources::CalculationsResource;
    /// use some_stripe_types::ListTaxCalculationLineItemsParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let resource = CalculationsResource::new(&client);
    /// let params = ListTaxCalculationLineItemsParams::new().limit(10);
    ///
    /// let page = resource.list_line_items_page("taxcalc_123", Some(&params)).await?;
    /// for item in page.data {
    ///     println!("Line item: {}", item.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_line_items_page(
        &self,
        calculation_id: &str,
        params: Option<&ListTaxCalculationLineItemsParams>,
    ) -> Result<TaxCalculationLineItemList, StripeError> {
        let safe_id = validate_and_encode_id(calculation_id)?;
        let mut url = format!(
            "{}/v1/tax/calculations/{}/line_items",
            self.client.config.api_base, safe_id
        );

        if let Some(p) = params {
            let mut query_params = Vec::new();
            if let Some(limit) = p.limit {
                query_params.push(format!("limit={limit}"));
            }
            if let Some(ref sa) = p.starting_after {
                let enc = some_stripe_core::patterns::serializer::percent_encode(sa);
                query_params.push(format!("starting_after={enc}"));
            }
            if let Some(ref eb) = p.ending_before {
                let enc = some_stripe_core::patterns::serializer::percent_encode(eb);
                query_params.push(format!("ending_before={enc}"));
            }
            if !query_params.is_empty() {
                url.push('?');
                url.push_str(&query_params.join("&"));
            }
        }

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let list: TaxCalculationLineItemList = serde_json::from_value(val)?;
        Ok(list)
    }
}
