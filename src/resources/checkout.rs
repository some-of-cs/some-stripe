//! Stripe Checkout resource implementations.

use crate::resources::payment_links::PaymentLinksResource;
use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{CheckoutSession, CreateCheckoutSessionParams};

/// Root resource for Stripe Checkout operations (`stripe.checkout`).
#[derive(Clone, Copy)]
pub struct CheckoutResource<'a> {
    /// Checkout Sessions resource (`stripe.checkout.sessions`).
    pub sessions: SessionsResource<'a>,
    /// Payment Links resource (`stripe.checkout.payment_links`).
    pub payment_links: PaymentLinksResource<'a>,
}

impl<'a> CheckoutResource<'a> {
    pub(crate) fn new(client: &'a StripeClient) -> Self {
        Self {
            sessions: SessionsResource::new(client),
            payment_links: PaymentLinksResource::new(client),
        }
    }

    /// Access the Sessions resource.
    pub fn sessions(&self) -> &SessionsResource<'a> {
        &self.sessions
    }

    /// Access the PaymentLinks resource.
    pub fn payment_links(&self) -> &PaymentLinksResource<'a> {
        &self.payment_links
    }
}

/// Resource for managing Stripe Checkout Sessions (`stripe.checkout.sessions`).
#[derive(Clone, Copy)]
pub struct SessionsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> SessionsResource<'a> {
    pub(crate) fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Creates a Checkout Session (`POST /v1/checkout/sessions`).
    pub async fn create(
        &self,
        params: &CreateCheckoutSessionParams,
    ) -> Result<CheckoutSession, StripeError> {
        let url = format!("{}/v1/checkout/sessions", self.client.config.api_base);
        let amount_str = params.unit_amount.to_string();
        let qty_str = params.quantity.to_string();

        let mut form_data = vec![
            ("success_url", params.success_url.as_str()),
            ("mode", params.mode.as_str()),
            (
                "line_items[0][price_data][currency]",
                params.currency.as_str(),
            ),
            (
                "line_items[0][price_data][unit_amount]",
                amount_str.as_str(),
            ),
            (
                "line_items[0][price_data][product_data][name]",
                params.product_name.as_str(),
            ),
            ("line_items[0][quantity]", qty_str.as_str()),
        ];

        if let Some(cancel_url) = &params.cancel_url {
            form_data.push(("cancel_url", cancel_url.as_str()));
        }
        if let Some(client_ref) = &params.client_reference_id {
            form_data.push(("client_reference_id", client_ref.as_str()));
        }

        let expires_str = params.expires_at.map(|e| e.to_string());
        if let Some(ref exp) = expires_str {
            form_data.push(("expires_at", exp.as_str()));
        }

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
        let session: CheckoutSession = serde_json::from_value(val)?;
        Ok(session)
    }

    /// Retrieves an existing Checkout Session by ID (`GET /v1/checkout/sessions/{id}`).
    pub async fn retrieve(&self, session_id: &str) -> Result<CheckoutSession, StripeError> {
        let url = format!(
            "{}/v1/checkout/sessions/{}",
            self.client.config.api_base, session_id
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let session: CheckoutSession = serde_json::from_value(val)?;
        Ok(session)
    }

    /// Expires an active Checkout Session (`POST /v1/checkout/sessions/{id}/expire`).
    pub async fn expire(&self, session_id: &str) -> Result<CheckoutSession, StripeError> {
        let url = format!(
            "{}/v1/checkout/sessions/{}/expire",
            self.client.config.api_base, session_id
        );
        let req = self.client.http.post(&url);
        let val = self.client.execute(req).await?;
        let session: CheckoutSession = serde_json::from_value(val)?;
        Ok(session)
    }
}
