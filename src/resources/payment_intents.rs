//! Stripe PaymentIntents resource implementations.

use crate::client::StripeClient;
use crate::error::StripeError;
use crate::types::{
    CancelPaymentIntentParams, CapturePaymentIntentParams, CreatePaymentIntentParams, PaymentIntent,
};

/// Resource for managing Stripe PaymentIntents (`stripe.paymentIntents`).
#[derive(Clone)]
pub struct PaymentIntentsResource {
    client: StripeClient,
}

impl PaymentIntentsResource {
    pub(crate) fn new(client: StripeClient) -> Self {
        Self { client }
    }

    /// Creates a PaymentIntent (`POST /v1/payment_intents`).
    pub async fn create(
        &self,
        params: &CreatePaymentIntentParams,
    ) -> Result<PaymentIntent, StripeError> {
        let url = format!("{}/v1/payment_intents", self.client.config.api_base);
        let amount_str = params.amount.to_string();

        let mut form_data = vec![
            ("amount", amount_str.as_str()),
            ("currency", params.currency.as_str()),
        ];

        if let Some(capture_method) = &params.capture_method {
            form_data.push(("capture_method", capture_method.as_str()));
        }
        if let Some(desc) = &params.description {
            form_data.push(("description", desc.as_str()));
        }

        let mut req = self.client.http.post(&url).form(&form_data);
        if let Some(idempotency_key) = &params.idempotency_key {
            req = req.header("Idempotency-Key", idempotency_key);
        }

        let val = self.client.execute(req).await?;
        let intent: PaymentIntent = serde_json::from_value(val)?;
        Ok(intent)
    }

    /// Retrieves a PaymentIntent by ID (`GET /v1/payment_intents/{id}`).
    pub async fn retrieve(&self, intent_id: &str) -> Result<PaymentIntent, StripeError> {
        let url = format!(
            "{}/v1/payment_intents/{}",
            self.client.config.api_base, intent_id
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let intent: PaymentIntent = serde_json::from_value(val)?;
        Ok(intent)
    }

    /// Captures the funds of an existing uncaptured PaymentIntent (`POST /v1/payment_intents/{id}/capture`).
    pub async fn capture(
        &self,
        intent_id: &str,
        params: Option<&CapturePaymentIntentParams>,
    ) -> Result<PaymentIntent, StripeError> {
        let url = format!(
            "{}/v1/payment_intents/{}/capture",
            self.client.config.api_base, intent_id
        );
        let mut form_data = Vec::new();
        let amount_str = params
            .and_then(|p| p.amount_to_capture)
            .map(|a| a.to_string());
        if let Some(ref a) = amount_str {
            form_data.push(("amount_to_capture", a.as_str()));
        }

        let mut req = self.client.http.post(&url).form(&form_data);
        if let Some(idempotency_key) = params.and_then(|p| p.idempotency_key.as_deref()) {
            req = req.header("Idempotency-Key", idempotency_key);
        }

        let val = self.client.execute(req).await?;
        let intent: PaymentIntent = serde_json::from_value(val)?;
        Ok(intent)
    }

    /// Cancels a PaymentIntent (`POST /v1/payment_intents/{id}/cancel`).
    pub async fn cancel(
        &self,
        intent_id: &str,
        params: Option<&CancelPaymentIntentParams>,
    ) -> Result<PaymentIntent, StripeError> {
        let url = format!(
            "{}/v1/payment_intents/{}/cancel",
            self.client.config.api_base, intent_id
        );
        let mut form_data = Vec::new();
        if let Some(reason) = params.and_then(|p| p.cancellation_reason.as_deref()) {
            form_data.push(("cancellation_reason", reason));
        }

        let mut req = self.client.http.post(&url).form(&form_data);
        if let Some(idempotency_key) = params.and_then(|p| p.idempotency_key.as_deref()) {
            req = req.header("Idempotency-Key", idempotency_key);
        }

        let val = self.client.execute(req).await?;
        let intent: PaymentIntent = serde_json::from_value(val)?;
        Ok(intent)
    }
}
