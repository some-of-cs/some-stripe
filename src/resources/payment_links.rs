//! Stripe PaymentLinks resource implementations.

use some_stripe_core::patterns::pagination::Paginator;
use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{
    CreatePaymentLinkParams, LineItem, ListLineItemsParams, PaymentLink, UpdatePaymentLinkParams,
    line_item_id,
};

/// Resource for managing Stripe Payment Links (`stripe.payment_links`).
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
    pub async fn create(
        &self,
        params: &CreatePaymentLinkParams,
    ) -> Result<PaymentLink, StripeError> {
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!("{}/v1/payment_links", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let generated_key;
        let idempotency_key = match params.idempotency_key.as_deref() {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = crate::resources::generate_uuid_v4();
                &generated_key
            }
        };

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        let val = self.client.execute(req).await?;
        let link: PaymentLink = serde_json::from_value(val)?;
        Ok(link)
    }

    /// Retrieves an existing Payment Link by ID (`GET /v1/payment_links/{id}`).
    ///
    /// Accepts the identifier as a string reference (`&str`), ensuring zero allocations.
    /// This is an idempotent read operation and does not inject an idempotency key.
    pub async fn retrieve(&self, link_id: &str) -> Result<PaymentLink, StripeError> {
        let url = format!(
            "{}/v1/payment_links/{}",
            self.client.config.api_base, link_id
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let link: PaymentLink = serde_json::from_value(val)?;
        Ok(link)
    }

    /// Updates an existing Payment Link (`POST /v1/payment_links/{id}`).
    ///
    /// Allows updating fields such as toggling active status (`active: false` to deactivate).
    /// Automatically injects an RFC 4122 UUID v4 `Idempotency-Key` header if omitted.
    pub async fn update(
        &self,
        link_id: &str,
        params: &UpdatePaymentLinkParams,
    ) -> Result<PaymentLink, StripeError> {
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!(
            "{}/v1/payment_links/{}",
            self.client.config.api_base, link_id
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

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        let val = self.client.execute(req).await?;
        let link: PaymentLink = serde_json::from_value(val)?;
        Ok(link)
    }

    /// Returns a zero-clone stream paginator for line items associated with this payment link
    /// (`GET /v1/payment_links/{id}/line_items`).
    pub fn list_line_items(
        &self,
        link_id: &str,
        params: &ListLineItemsParams,
    ) -> Paginator<'a, LineItem> {
        let limit = params.limit.unwrap_or(10);
        Paginator::new(
            self.client,
            format!("v1/payment_links/{link_id}/line_items"),
            limit,
            line_item_id,
        )
    }

    /// Returns a zero-clone stream paginator with default parameters (`limit = 10`).
    pub fn list_line_items_default(&self, link_id: &str) -> Paginator<'a, LineItem> {
        self.list_line_items(link_id, &ListLineItemsParams::default())
    }

    /// Returns a zero-clone stream paginator with a custom page size limit.
    pub fn list_line_items_with_limit(&self, link_id: &str, limit: u32) -> Paginator<'a, LineItem> {
        self.list_line_items(link_id, &ListLineItemsParams::new().limit(limit))
    }
}
