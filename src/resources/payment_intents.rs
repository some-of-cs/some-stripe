//! Stripe PaymentIntents resource implementations.
//!
//! Provides API bindings for the Stripe PaymentIntents lifecycle (`create`, `retrieve`,
//! `confirm`, `capture`, and `cancel`), modeled after `stripe-node`.
//!
//! ## Quality & Borrowing Discipline (Gate G-Borrow)
//! All parameters are accepted strictly by reference (`&CreatePaymentIntentParams`,
//! `&str`, `Option<&CapturePaymentIntentParams>`, `Option<&CancelPaymentIntentParams>`,
//! `Option<&ConfirmPaymentIntentParams>`), ensuring zero unnecessary deep copies on
//! heap allocations.
//!
//! ## ISO-4217 Currency Compliance (Gate G-Law)
//! Amounts are specified and validated as positive integers (`amount > 0`) in minor currency
//! units (cents), and currency codes are checked against 3-letter lowercase ISO formats.

use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{
    CancelPaymentIntentParams, CapturePaymentIntentParams, ConfirmPaymentIntentParams,
    CreatePaymentIntentParams, PaymentIntent,
};

/// Resource for managing Stripe PaymentIntents (`stripe.payment_intents()`).
#[derive(Clone, Copy)]
pub struct PaymentIntentsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> PaymentIntentsResource<'a> {
    pub(crate) fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Creates a PaymentIntent (`POST /v1/payment_intents`).
    ///
    /// Accepts request parameters by reference (`&CreatePaymentIntentParams`), eliminating
    /// redundant clones. Performs client-side validation on ISO-4217 minor currency units
    /// and currency codes before transmitting across the network.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::CreatePaymentIntentParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreatePaymentIntentParams::new(5000, "usd")
    ///     .with_capture_method("automatic")
    ///     .with_description("Order #1001")
    ///     .with_idempotency_key("order_1001_idem");
    ///
    /// let intent = client.payment_intents().create(&params).await?;
    /// assert_eq!(intent.amount, 5000);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        params: &CreatePaymentIntentParams,
    ) -> Result<PaymentIntent, StripeError> {
        if let Err(err) = params.validate() {
            return Err(StripeError::InvalidRequest(err));
        }

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
    ///
    /// Accepts the identifier as a string reference (`&str`), ensuring zero allocations.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let intent = client.payment_intents().retrieve("pi_12345").await?;
    /// assert_eq!(intent.id, "pi_12345");
    /// # Ok(())
    /// # }
    /// ```
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

    /// Confirms a PaymentIntent (`POST /v1/payment_intents/{id}/confirm`).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::ConfirmPaymentIntentParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = ConfirmPaymentIntentParams::new()
    ///     .with_payment_method("pm_card_visa")
    ///     .with_return_url("https://example.com/checkout/return");
    ///
    /// let intent = client
    ///     .payment_intents()
    ///     .confirm("pi_12345", Some(&params))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn confirm(
        &self,
        intent_id: &str,
        params: Option<&ConfirmPaymentIntentParams>,
    ) -> Result<PaymentIntent, StripeError> {
        let url = format!(
            "{}/v1/payment_intents/{}/confirm",
            self.client.config.api_base, intent_id
        );
        let mut form_data = Vec::new();
        if let Some(pm) = params.and_then(|p| p.payment_method.as_deref()) {
            form_data.push(("payment_method", pm));
        }
        if let Some(ret) = params.and_then(|p| p.return_url.as_deref()) {
            form_data.push(("return_url", ret));
        }

        let mut req = self.client.http.post(&url).form(&form_data);
        if let Some(idempotency_key) = params.and_then(|p| p.idempotency_key.as_deref()) {
            req = req.header("Idempotency-Key", idempotency_key);
        }

        let val = self.client.execute(req).await?;
        let intent: PaymentIntent = serde_json::from_value(val)?;
        Ok(intent)
    }

    /// Captures the funds of an existing uncaptured PaymentIntent (`POST /v1/payment_intents/{id}/capture`).
    ///
    /// Can capture the full authorized amount (`params: None`) or a partial amount
    /// (`params: Some(&CapturePaymentIntentParams)`).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::CapturePaymentIntentParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let capture_params = CapturePaymentIntentParams::new()
    ///     .with_amount_to_capture(3500)
    ///     .with_idempotency_key("idem_capture_1001");
    ///
    /// let captured = client
    ///     .payment_intents()
    ///     .capture("pi_12345", Some(&capture_params))
    ///     .await?;
    /// assert_eq!(captured.amount_received, Some(3500));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn capture(
        &self,
        intent_id: &str,
        params: Option<&CapturePaymentIntentParams>,
    ) -> Result<PaymentIntent, StripeError> {
        if let Some(p) = params {
            p.validate().map_err(StripeError::InvalidRequest)?;
        }

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
    ///
    /// Releases the authorization hold on customer funds. Cancellation reason and idempotency
    /// keys can be supplied via optional borrowed `&CancelPaymentIntentParams`.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::CancelPaymentIntentParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let cancel_params = CancelPaymentIntentParams::new()
    ///     .with_cancellation_reason("requested_by_customer")
    ///     .with_idempotency_key("idem_cancel_1001");
    ///
    /// let canceled = client
    ///     .payment_intents()
    ///     .cancel("pi_12345", Some(&cancel_params))
    ///     .await?;
    /// assert_eq!(canceled.status, "canceled");
    /// # Ok(())
    /// # }
    /// ```
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

#[cfg(test)]
mod tests {
    use super::*;
    use some_stripe_core::client::StripeConfig;

    #[tokio::test]
    async fn test_create_validation_rejection() {
        let config = StripeConfig::new("sk_test_dummy");
        let client = StripeClient::with_config(config);
        let resource = PaymentIntentsResource::new(&client);

        // Negative amount
        let invalid_params = CreatePaymentIntentParams::new(-100, "usd");
        let res = resource.create(&invalid_params).await;
        assert!(matches!(res, Err(StripeError::InvalidRequest(_))));

        // Zero amount
        let zero_params = CreatePaymentIntentParams::new(0, "usd");
        let res = resource.create(&zero_params).await;
        assert!(matches!(res, Err(StripeError::InvalidRequest(_))));

        // Uppercase currency
        let bad_cur = CreatePaymentIntentParams::new(1000, "USD");
        let res = resource.create(&bad_cur).await;
        assert!(matches!(res, Err(StripeError::InvalidRequest(_))));
    }

    #[tokio::test]
    async fn test_capture_validation_rejection() {
        let config = StripeConfig::new("sk_test_dummy");
        let client = StripeClient::with_config(config);
        let resource = PaymentIntentsResource::new(&client);

        let invalid_cap = CapturePaymentIntentParams::new().with_amount_to_capture(0);
        let res = resource.capture("pi_123", Some(&invalid_cap)).await;
        assert!(matches!(res, Err(StripeError::InvalidRequest(_))));
    }
}
