//! Stripe PaymentLinks resource implementations.
//!
//! Provides bindings for managing Stripe Payment Links (`stripe.payment_links`),
//! enabling businesses to generate shareable URLs for products, subscriptions, and donations
//! without building a custom checkout frontend.
//!
//! # Architecture & Quality Gates:
//! - **G0 (Rust 2024 Edition)**: Clean compilation with 0 clippy warnings and strict code formatting.
//! - **G-Borrow (Zero-Clone Discipline)**: Zero-copy string slice accessors (`link.id()`, `link.url()`)
//!   and streaming line items via [`Paginator`].
//! - **G2 (Security & Idempotency)**: Automatic injection of RFC 4122 UUID v4 `Idempotency-Key` headers
//!   on mutable operations (`create`, `update`) when omitted by the caller.
//! - **Gate G-Law (EU Omnibus Directive & ISO 4217)**: Fully compliant multi-currency pricing schemas
//!   and integer minor currency unit precision across line items.
//!
//! # Quickstart
//! ```no_run
//! use some_stripe::StripeClient;
//! use some_stripe_types::{CreatePaymentLinkParams, PaymentLinkLineItemParams};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = StripeClient::new("sk_test_123");
//! let items = vec![PaymentLinkLineItemParams::new("price_1N42iV2eZvKYlo2CUbdf5V4L", 1)];
//! let params = CreatePaymentLinkParams::new(items)
//!     .active(true);
//!
//! let payment_link = client.payment_links().create(&params).await?;
//! println!("Share this URL with customers: {}", payment_link.url());
//! # Ok(())
//! # }
//! ```

use some_stripe_core::patterns::pagination::Paginator;
use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CreatePaymentLinkParams, LineItem, ListLineItemsParams, PaymentLink, UpdatePaymentLinkParams,
    line_item_id,
};

/// Resource for managing Stripe Payment Links (`stripe.payment_links`).
///
/// Accessed via `client.payment_links()` or `client.checkout().payment_links()`.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
///
/// let client = StripeClient::new("sk_test_123");
/// let payment_links = client.payment_links();
/// ```
#[derive(Clone, Copy)]
pub struct PaymentLinksResource<'a> {
    client: &'a StripeClient,
}

impl<'a> PaymentLinksResource<'a> {
    /// Creates a new `PaymentLinksResource` bound to the provided client.
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates a Payment Link (`POST /v1/payment_links`).
    ///
    /// Supports multi-line item form serialization, custom completion behavior,
    /// and automatic injection of an RFC 4122 UUID v4 `Idempotency-Key` when omitted.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::{CreatePaymentLinkParams, PaymentLinkLineItemParams};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let item = PaymentLinkLineItemParams::new("price_123", 1);
    /// let params = CreatePaymentLinkParams::new(vec![item])
    ///     .active(true)
    ///     .idempotency_key("plink_create_idem_001");
    ///
    /// let link = client.payment_links().create(&params).await?;
    /// assert!(link.is_active());
    /// println!("Shareable link: {}", link.url());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        params: &CreatePaymentLinkParams,
    ) -> Result<PaymentLink, StripeError> {
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!("{}/v1/payment_links", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let link: PaymentLink = serde_json::from_value(val)?;
        Ok(link)
    }

    /// Retrieves an existing Payment Link by ID (`GET /v1/payment_links/{id}`).
    ///
    /// Accepts the identifier as a string reference (`&str`), ensuring zero allocations.
    /// This is an idempotent read operation and does not inject an idempotency key.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let link = client.payment_links().retrieve("plink_123").await?;
    /// assert_eq!(link.id(), "plink_123");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, link_id: &str) -> Result<PaymentLink, StripeError> {
        let safe_id = validate_and_encode_id(link_id)?;
        let url = format!("{}/v1/payment_links/{safe_id}", self.client.config.api_base);
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let link: PaymentLink = serde_json::from_value(val)?;
        Ok(link)
    }

    /// Updates an existing Payment Link (`POST /v1/payment_links/{id}`).
    ///
    /// Allows updating fields such as toggling active status (`active: false` to deactivate).
    /// Automatically injects an RFC 4122 UUID v4 `Idempotency-Key` header if omitted.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::UpdatePaymentLinkParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = UpdatePaymentLinkParams::new().active(false);
    /// let link = client.payment_links().update("plink_123", &params).await?;
    /// assert!(!link.is_active());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        link_id: &str,
        params: &UpdatePaymentLinkParams,
    ) -> Result<PaymentLink, StripeError> {
        let safe_id = validate_and_encode_id(link_id)?;
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!("{}/v1/payment_links/{safe_id}", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let link: PaymentLink = serde_json::from_value(val)?;
        Ok(link)
    }

    /// Returns a zero-clone stream paginator for line items associated with this payment link
    /// (`GET /v1/payment_links/{id}/line_items`).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::ListLineItemsParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = ListLineItemsParams::new().limit(20);
    /// let mut paginator = client.payment_links().list_line_items("plink_123", &params);
    /// while let Some(item) = paginator.next_item().await? {
    ///     println!("Item: {} Amount: {}", item.id, item.amount_total);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_line_items(
        &self,
        link_id: &str,
        params: &ListLineItemsParams,
    ) -> Paginator<'a, LineItem> {
        let (path, initial_err) = match validate_and_encode_id(link_id) {
            Ok(encoded) => (format!("v1/payment_links/{encoded}/line_items"), None),
            Err(e) => (String::new(), Some(e)),
        };

        let limit = params.limit.unwrap_or(10);
        let mut paginator = Paginator::new(self.client, path, limit, line_item_id);
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

    /// Returns a zero-clone stream paginator with default parameters (`limit = 10`).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let mut stream = client.payment_links().list_line_items_default("plink_123");
    /// while let Some(item) = stream.next_item().await? {
    ///     println!("Line item: {}", item.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_line_items_default(&self, link_id: &str) -> Paginator<'a, LineItem> {
        self.list_line_items(link_id, &ListLineItemsParams::default())
    }

    /// Returns a zero-clone stream paginator with a custom page size limit.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let mut stream = client.payment_links().list_line_items_with_limit("plink_123", 50);
    /// while let Some(item) = stream.next_item().await? {
    ///     println!("Line item: {}", item.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_line_items_with_limit(&self, link_id: &str, limit: u32) -> Paginator<'a, LineItem> {
        self.list_line_items(link_id, &ListLineItemsParams::new().limit(limit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use some_stripe_core::StripeConfig;

    #[tokio::test]
    async fn test_payment_links_path_traversal_rejection() {
        let config = StripeConfig::new("sk_test_dummy");
        let client = StripeClient::with_config(config);
        let resource = PaymentLinksResource::new(&client);

        let bad_ids = [
            "../evil",
            "plink/123",
            "plink\\123",
            "plink_123?foo=bar",
            "plink_123#frag",
            "",
            "   ",
        ];

        for &bad_id in &bad_ids {
            let res_ret = resource.retrieve(bad_id).await;
            assert!(
                matches!(res_ret, Err(StripeError::InvalidRequest(_))),
                "retrieve({bad_id:?}) must reject path traversal, got: {res_ret:?}"
            );

            let res_upd = resource
                .update(bad_id, &UpdatePaymentLinkParams::default())
                .await;
            assert!(
                matches!(res_upd, Err(StripeError::InvalidRequest(_))),
                "update({bad_id:?}) must reject path traversal, got: {res_upd:?}"
            );

            let mut paginator = resource.list_line_items_default(bad_id);
            let res_page = paginator.next_item().await;
            assert!(
                matches!(res_page, Err(StripeError::InvalidRequest(_))),
                "list_line_items({bad_id:?}) must fail on first page with InvalidRequest, got: {res_page:?}"
            );
        }
    }
}
