//! Composable `some-payment` capability adapter layer for `some-stripe`.
//!
//! Provides [`StripeGatewayAdapter`], a thin, thread-safe wrapper around [`StripeClient`]
//! implementing the orthogonal capability traits defined in [`capabilities`]:
//! - [`PaymentCapability`]
//! - [`KycCapability`]
//! - [`PayoutCapability`]
//! - [`WebhookCapability`]
//! - [`StripeExt`] (direct escape hatch to the underlying [`StripeClient`])

pub mod capabilities;

pub use capabilities::{
    KycCapability, PaymentCapability, PayoutCapability, StripeExt, UnifiedGatewayError,
    WebhookCapability,
};

use async_trait::async_trait;
use some_stripe_core::error::StripeWebhookError;
use some_stripe_types::identity_session::{CreateVerificationSessionParams, VerificationSession};
use some_stripe_types::{
    CancelPaymentIntentParams, CapturePaymentIntentParams, CreatePaymentIntentParams,
    CreatePayoutParams, CreateRefundParams, Event, PaymentIntent, Payout, Refund,
};
use some_stripe_webhooks::Webhook;

use crate::StripeClient;

/// Thread-safe `some-payment` gateway adapter wrapping [`StripeClient`].
///
/// Implements the orthogonal capability traits ([`PaymentCapability`], [`KycCapability`],
/// [`PayoutCapability`], [`WebhookCapability`]) while preserving direct access to the full
/// Stripe SDK via [`StripeGatewayAdapter::client`] and [`StripeExt::stripe_client`].
#[derive(Clone, Debug)]
pub struct StripeGatewayAdapter {
    client: StripeClient,
}

impl StripeGatewayAdapter {
    /// Creates a new [`StripeGatewayAdapter`] wrapping the provided [`StripeClient`].
    #[must_use]
    pub fn new(client: StripeClient) -> Self {
        Self { client }
    }

    /// Returns a shared reference to the underlying [`StripeClient`] escape hatch.
    #[must_use]
    pub fn client(&self) -> &StripeClient {
        &self.client
    }
}

impl From<StripeClient> for StripeGatewayAdapter {
    fn from(client: StripeClient) -> Self {
        Self::new(client)
    }
}

impl StripeExt for StripeGatewayAdapter {
    fn stripe_client(&self) -> &StripeClient {
        self.client()
    }
}

#[async_trait]
impl PaymentCapability for StripeGatewayAdapter {
    async fn create_payment_intent(
        &self,
        params: &CreatePaymentIntentParams,
    ) -> Result<PaymentIntent, UnifiedGatewayError> {
        self.client
            .payment_intents()
            .create(params)
            .await
            .map_err(UnifiedGatewayError::from)
    }

    async fn capture_payment_intent(
        &self,
        intent_id: &str,
        params: Option<&CapturePaymentIntentParams>,
    ) -> Result<PaymentIntent, UnifiedGatewayError> {
        self.client
            .payment_intents()
            .capture(intent_id, params)
            .await
            .map_err(UnifiedGatewayError::from)
    }

    async fn cancel_payment_intent(
        &self,
        intent_id: &str,
        params: Option<&CancelPaymentIntentParams>,
    ) -> Result<PaymentIntent, UnifiedGatewayError> {
        self.client
            .payment_intents()
            .cancel(intent_id, params)
            .await
            .map_err(UnifiedGatewayError::from)
    }

    async fn refund_payment(
        &self,
        params: &CreateRefundParams,
    ) -> Result<Refund, UnifiedGatewayError> {
        self.client
            .refunds()
            .create(params)
            .await
            .map_err(UnifiedGatewayError::from)
    }
}

#[async_trait]
impl KycCapability for StripeGatewayAdapter {
    async fn create_verification_session(
        &self,
        params: &CreateVerificationSessionParams,
    ) -> Result<VerificationSession, UnifiedGatewayError> {
        self.client
            .identity_sessions()
            .create(params)
            .await
            .map_err(UnifiedGatewayError::from)
    }

    async fn get_verification_session(
        &self,
        session_id: &str,
    ) -> Result<VerificationSession, UnifiedGatewayError> {
        self.client
            .identity_sessions()
            .retrieve(session_id)
            .await
            .map_err(UnifiedGatewayError::from)
    }

    async fn is_verified(&self, session_id: &str) -> Result<bool, UnifiedGatewayError> {
        let session = self.get_verification_session(session_id).await?;
        Ok(session.is_verified())
    }
}

#[async_trait]
impl PayoutCapability for StripeGatewayAdapter {
    async fn create_payout(
        &self,
        params: &CreatePayoutParams,
    ) -> Result<Payout, UnifiedGatewayError> {
        self.client
            .payouts()
            .create(params)
            .await
            .map_err(UnifiedGatewayError::from)
    }

    async fn get_payout(&self, payout_id: &str) -> Result<Payout, UnifiedGatewayError> {
        self.client
            .payouts()
            .retrieve(payout_id)
            .await
            .map_err(UnifiedGatewayError::from)
    }
}

impl WebhookCapability for StripeGatewayAdapter {
    fn verify_and_parse_event(
        &self,
        payload: &[u8],
        sig_header: &str,
        secret: &str,
        tolerance: Option<u64>,
    ) -> Result<Event, UnifiedGatewayError> {
        if secret.trim().is_empty() {
            return Err(UnifiedGatewayError::Webhook(
                StripeWebhookError::MissingSecret,
            ));
        }

        Webhook::construct_event(payload, sig_header, secret, tolerance)
            .map_err(UnifiedGatewayError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use some_stripe_core::error::StripeError;

    #[test]
    fn test_adapter_new_and_stripe_ext_escape_hatch() {
        let client = StripeClient::new("sk_test_adapter_unit_123");
        let adapter = StripeGatewayAdapter::new(client);
        let cloned = adapter.clone();

        assert_eq!(adapter.client().config.api_base, "https://api.stripe.com");
        assert_eq!(
            cloned.stripe_client().config.api_base,
            "https://api.stripe.com"
        );

        let debug_output = format!("{adapter:?}");
        assert!(!debug_output.contains("sk_test_adapter_unit_123"));
    }

    #[test]
    fn test_unified_gateway_error_mapping() {
        let rate_limit = UnifiedGatewayError::from(StripeError::Api {
            status: 429,
            code: "rate_limit".into(),
            message: "Too many requests".into(),
            error_type: "api_error".into(),
            doc_url: None,
        });
        assert!(rate_limit.is_retryable());
        assert_eq!(rate_limit.status_code(), Some(429));
        assert_eq!(rate_limit.provider_code(), Some("rate_limit"));

        let declined = UnifiedGatewayError::from(StripeError::Api {
            status: 400,
            code: "card_declined".into(),
            message: "Card declined".into(),
            error_type: "card_error".into(),
            doc_url: None,
        });
        assert!(!declined.is_retryable());
        assert_eq!(declined.status_code(), Some(400));
        assert_eq!(declined.provider_code(), Some("card_declined"));

        let conn_err = UnifiedGatewayError::from(StripeError::ConnectionError("reset".into()));
        assert!(conn_err.is_retryable());
        assert_eq!(conn_err.status_code(), None);
        assert_eq!(conn_err.provider_code(), None);

        let webhook_err = UnifiedGatewayError::from(StripeWebhookError::BadSignature);
        assert!(!webhook_err.is_retryable());
        assert_eq!(webhook_err.status_code(), None);
        assert_eq!(
            webhook_err.provider_code(),
            Some("webhook_verification_failed")
        );
    }

    #[test]
    fn test_webhook_capability_signed_payload() {
        let adapter = StripeGatewayAdapter::new(StripeClient::new("sk_test_webhook_ok"));
        let payload = br#"{"id":"evt_adapter_123","object":"event","created":1728345600,"type":"payment_intent.succeeded","data":{"object":{"id":"pi_123","status":"succeeded"}}}"#;
        let secret = "whsec_adapter_test_secret";
        let now = chrono::Utc::now().timestamp();
        let header = Webhook::generate_test_header_string(payload, secret, Some(now));

        let result = adapter.verify_and_parse_event(payload, &header, secret, None);
        let Ok(event) = result else {
            panic!("expected signed webhook payload to verify cleanly");
        };
        assert_eq!(event.id, "evt_adapter_123");
    }

    #[test]
    fn test_webhook_capability_forged_and_blank_secret_rejected() {
        let adapter = StripeGatewayAdapter::new(StripeClient::new("sk_test_webhook_forged"));
        let payload = br#"{"id":"evt_forged_123","object":"event","created":1728345600,"type":"payment_intent.succeeded","data":{"object":{"id":"pi_123","status":"succeeded"}}}"#;
        let secret = "whsec_adapter_real_secret";
        let now = chrono::Utc::now().timestamp();
        let forged_header =
            Webhook::generate_test_header_string(payload, "whsec_attacker_wrong_secret", Some(now));

        let forged_result = adapter.verify_and_parse_event(payload, &forged_header, secret, None);
        assert!(matches!(
            forged_result,
            Err(UnifiedGatewayError::Webhook(
                StripeWebhookError::BadSignature
            ))
        ));

        let blank_secret_result =
            adapter.verify_and_parse_event(payload, &forged_header, "   ", None);
        assert!(matches!(
            blank_secret_result,
            Err(UnifiedGatewayError::Webhook(
                StripeWebhookError::MissingSecret
            ))
        ));
    }
}
