//! Stripe Webhooks resource and event constructor, modeled after `stripe.webhooks`.

use some_stripe_types::Event;
use some_stripe_webhooks::{StripeWebhookError, Webhook};

/// Webhook verification resource (`stripe.webhooks`).
#[derive(Clone, Default)]
pub struct WebhooksResource;

impl WebhooksResource {
    /// Creates a new Webhooks resource helper.
    pub fn new() -> Self {
        Self
    }

    /// Constructs an [`Event`] from an incoming webhook payload after verifying its cryptographic signature and timestamp.
    ///
    /// # Arguments
    /// - `payload`: The raw body bytes received from the webhook request.
    /// - `header`: The value of the `Stripe-Signature` HTTP header.
    /// - `secret`: The webhook endpoint secret (`whsec_...`).
    /// - `tolerance`: Optional maximum acceptable age of the webhook in seconds. Default is 300 seconds (5 minutes).
    pub fn construct_event(
        &self,
        payload: &[u8],
        header: &str,
        secret: &str,
        tolerance: Option<u64>,
    ) -> Result<Event, StripeWebhookError> {
        Webhook::construct_event(payload, header, secret, tolerance)
    }

    /// Verifies the authenticity and recency of a webhook without parsing the full event.
    pub fn verify_header(
        &self,
        payload: &[u8],
        header: &str,
        secret: &str,
        tolerance: Option<u64>,
    ) -> Result<bool, StripeWebhookError> {
        Webhook::verify_header(payload, header, secret, tolerance)
    }

    /// Generates a valid `Stripe-Signature` header string for unit and integration testing.
    pub fn generate_test_header_string(
        &self,
        payload: &[u8],
        secret: &str,
        timestamp: Option<i64>,
    ) -> String {
        Webhook::generate_test_header_string(payload, secret, timestamp)
    }
}
