//! Stripe Checkout resource implementations.

use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{CheckoutSession, CreateCheckoutSessionParams};

/// Root resource for Stripe Checkout operations (`stripe.checkout`).
#[derive(Clone, Copy)]
pub struct CheckoutResource<'a> {
    /// Checkout Sessions resource (`stripe.checkout.sessions`).
    pub sessions: SessionsResource<'a>,
}

impl<'a> CheckoutResource<'a> {
    pub(crate) fn new(client: &'a StripeClient) -> Self {
        Self {
            sessions: SessionsResource::new(client),
        }
    }

    /// Access the Sessions resource.
    pub fn sessions(&self) -> &SessionsResource<'a> {
        &self.sessions
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

        let generated_key;
        let idempotency_key = match params.idempotency_key.as_deref() {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = crate::resources::generate_uuid_v4();
                &generated_key
            }
        };

        let form_pairs;
        let mut legacy_form_data;
        let form_data: &[(String, String)] = if params.line_items.is_some() {
            form_pairs = some_stripe_core::patterns::serializer::to_form_params(params)?;
            &form_pairs
        } else {
            legacy_form_data = vec![
                ("success_url".to_string(), params.success_url.clone()),
                ("mode".to_string(), params.mode.clone()),
            ];
            if let Some(cancel_url) = &params.cancel_url {
                legacy_form_data.push(("cancel_url".to_string(), cancel_url.clone()));
            }
            if let Some(client_ref) = &params.client_reference_id {
                legacy_form_data.push(("client_reference_id".to_string(), client_ref.clone()));
            }
            if let Some(curr) = &params.currency {
                legacy_form_data.push((
                    "line_items[0][price_data][currency]".to_string(),
                    curr.clone(),
                ));
            }
            if let Some(amt) = params.unit_amount {
                legacy_form_data.push((
                    "line_items[0][price_data][unit_amount]".to_string(),
                    amt.to_string(),
                ));
            }
            if let Some(name) = &params.product_name {
                legacy_form_data.push((
                    "line_items[0][price_data][product_data][name]".to_string(),
                    name.clone(),
                ));
            }
            if let Some(qty) = params.quantity {
                legacy_form_data.push(("line_items[0][quantity]".to_string(), qty.to_string()));
            }
            if let Some(exp) = params.expires_at {
                legacy_form_data.push(("expires_at".to_string(), exp.to_string()));
            }
            &legacy_form_data
        };

        let req = self
            .client
            .http
            .post(&url)
            .form(form_data)
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
