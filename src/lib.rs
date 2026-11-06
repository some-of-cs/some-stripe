//! # some-stripe
//!
//! An idiomatic, production-ready Rust client and webhook validation library for the Stripe API,
//! designed to closely reflect the ergonomics of the official `stripe-node` SDK while enforcing
//! strict memory safety, zero data races, and constant-time cryptographic verification.
//!
//! ## Overview
//!
//! - **Client Architecture**: Clean namespace separation (`stripe.checkout().sessions()`, `stripe.payment_intents()`, `stripe.accounts()`, `stripe.webhooks()`).
//! - **Webhook Engine**: Constant-time HMAC-SHA256 verification, replay protection against timestamp drift, and strongly typed event parsing.
//! - **Robust Error Handling**: Structured API error payloads with documentation URLs, HTTP status codes, and webhook verification errors.
//! - **Dispute & Evidence Management**: Strongly typed [`Dispute`], [`DisputeEvidence`], [`DisputeEvidenceDetails`], [`DisputeStatus`], and [`UpdateDisputeParams`] with submission locking invariants.
//! - **Customizable Error Workflows**: Configurable retry policies and security escalation hooks.
//!
//! ## Quickstart
//!
//! ```no_run
//! use some_stripe::{StripeClient, CreateCheckoutSessionParams};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let stripe = StripeClient::new("sk_test_...");
//!
//! // 1. Create a Checkout Session
//! let session = stripe.checkout().sessions.create(&CreateCheckoutSessionParams {
//!     success_url: "https://example.com/success".into(),
//!     cancel_url: Some("https://example.com/cancel".into()),
//!     mode: "payment".into(),
//!     client_reference_id: Some("dep_sess_123".into()),
//!     currency: Some("usd".into()),
//!     unit_amount: Some(5000), // $50.00
//!     product_name: Some("Wallet Deposit".into()),
//!     quantity: Some(1),
//!     expires_at: None,
//!     idempotency_key: Some("dep_sess_123".into()),
//!     ..Default::default()
//! }).await?;
//!
//! println!("Hosted checkout URL: {:?}", session.url);
//! # Ok(())
//! # }
//! ```
//!
//! ## Webhook Verification
//!
//! ```
//! use some_stripe::{Webhook, StripeWebhookWrapper};
//!
//! let payload = br#"{"id":"evt_123","object":"event","created":1728345600,"type":"checkout.session.completed","data":{"object":{"id":"cs_123","status":"complete","payment_status":"paid"}}}"#;
//! let secret = "whsec_test_secret";
//! let now = chrono::Utc::now().timestamp();
//! let header = Webhook::generate_test_header_string(payload, secret, Some(now));
//!
//! let wrapper = StripeWebhookWrapper::from_raw(payload, &header, secret, None).unwrap();
//! assert_eq!(wrapper.event_id(), "evt_123");
//! assert!(wrapper.is_checkout_completed());
//! ```

pub use some_stripe_core as core;
pub use some_stripe_types as types;
pub use some_stripe_webhooks as webhooks;

pub use some_stripe_core::client::{
    ClientInner, RequestOptions, SecretKey, StripeConfig, StripeHeaderExt,
};
pub use some_stripe_core::error::{StripeError, StripeWebhookError};
pub use some_stripe_core::patterns;
pub use some_stripe_core::validate_and_encode_id;
pub use some_stripe_core::workflow;
pub use some_stripe_core::workflow::{
    DefaultErrorWorkflow, ErrorAction, ErrorSeverity, ErrorWorkflowHandler,
};

pub use some_stripe_types::*;
pub use some_stripe_webhooks::{
    DEFAULT_TOLERANCE_SECONDS, MAX_FUTURE_DRIFT_SECONDS, MAX_WEBHOOK_PAYLOAD_BYTES,
    StripeSignatureHeader, StripeWebhookWrapper, ToleranceConfig, V2RelatedObject, V2ThinEvent,
    Webhook, WebhookAuditRecord, WebhookEscalationBridge, determine_severity,
    escalate_webhook_error, is_blank_webhook_secret, is_security_incident, is_thin_event,
    parse_signature_header, safe_constant_time_eq, verify_stripe_signature,
};

pub mod resources;
pub use resources::{
    AccountsResource, BalanceResource, BalanceTransactionsResource, CheckoutResource,
    CheckoutSessionExt, DisputesResource, PaymentIntentsResource, PaymentLinksResource,
    RedirectError, RefundsResource, SessionsResource, SubscriptionItemsResource,
    SubscriptionsResource, WebhooksResource, generate_uuid_v4, is_fulfillment_ready,
    is_valid_uuid_v4, validate_redirect_url,
};

/// The main Stripe client used to interact with the Stripe API, modeled after `stripe-node`.
///
/// Wraps the thread-safe `some_stripe_core::StripeClient` microkernel and provides top-level
/// resource accessors (`client.checkout()`, `client.payment_intents()`, `client.accounts()`, `client.webhooks()`).
#[derive(Clone, Debug)]
pub struct StripeClient(pub some_stripe_core::StripeClient);

impl std::ops::Deref for StripeClient {
    type Target = some_stripe_core::StripeClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl StripeClient {
    /// Creates a new `StripeClient` configured with default settings and the provided secret key.
    pub fn new(secret_key: impl Into<SecretKey>) -> Self {
        Self(some_stripe_core::StripeClient::new(secret_key))
    }

    /// Creates a new `StripeClient` with custom [`StripeConfig`].
    pub fn with_config(config: StripeConfig) -> Self {
        Self(some_stripe_core::StripeClient::with_config(config))
    }

    /// Access the Checkout resource namespace (`stripe.checkout`).
    pub fn checkout(&self) -> CheckoutResource<'_> {
        CheckoutResource::new(&self.0)
    }

    /// Access the PaymentIntents resource namespace (`stripe.payment_intents`).
    pub fn payment_intents(&self) -> PaymentIntentsResource<'_> {
        PaymentIntentsResource::new(&self.0)
    }

    /// Access the Accounts resource namespace (`stripe.accounts`).
    pub fn accounts(&self) -> AccountsResource<'_> {
        AccountsResource::new(&self.0)
    }

    /// Access the Webhooks helper resource (`stripe.webhooks`).
    pub fn webhooks(&self) -> WebhooksResource {
        WebhooksResource::new()
    }

    /// Access the Refunds resource namespace (`stripe.refunds`).
    pub fn refunds(&self) -> RefundsResource<'_> {
        RefundsResource::new(&self.0)
    }

    /// Access the Disputes resource namespace (`stripe.disputes`).
    pub fn disputes(&self) -> DisputesResource<'_> {
        DisputesResource::new(&self.0)
    }

    /// Access the Balance resource namespace (`stripe.balance`).
    pub fn balance(&self) -> BalanceResource<'_> {
        BalanceResource::new(&self.0)
    }

    /// Access the BalanceTransactions resource namespace (`stripe.balance_transactions`).
    pub fn balance_transactions(&self) -> BalanceTransactionsResource<'_> {
        BalanceTransactionsResource::new(&self.0)
    }

    /// Access the PaymentLinks resource namespace (`stripe.payment_links`).
    pub fn payment_links(&self) -> PaymentLinksResource<'_> {
        PaymentLinksResource::new(&self.0)
    }

    /// Access the Subscriptions resource namespace (`stripe.subscriptions`).
    pub fn subscriptions(&self) -> SubscriptionsResource<'_> {
        SubscriptionsResource::new(&self.0)
    }
}

/// Validates compliance with EU Omnibus Directive 2019/2161 Article 6a regarding price reductions.
///
/// Any announcement of a price reduction must indicate the prior price applied for at least 30 days.
///
/// # Errors
/// Returns [`StripeError::InvalidRequest`] if any of the statutory conditions are violated.
pub fn validate_omnibus_price_transparency(
    regular_price: i64,
    announced_discounted_price: i64,
    prior_30d_lowest_price: Option<i64>,
) -> Result<(), StripeError> {
    some_stripe_types::validate_omnibus_price_transparency(
        regular_price,
        announced_discounted_price,
        prior_30d_lowest_price,
    )
    .map_err(|e| StripeError::InvalidRequest(e.to_string()))
}

/// Validates transparent tax behavior disclosure under EU consumer protection regulations.
///
/// In B2C / EU Omnibus context, tax behavior cannot be `unspecified`.
///
/// # Errors
/// Returns [`StripeError::InvalidRequest`] if `tax_behavior` is `TaxBehavior::Unspecified`.
pub fn validate_tax_behavior_transparency(tax_behavior: TaxBehavior) -> Result<(), StripeError> {
    some_stripe_types::validate_tax_behavior_transparency(tax_behavior)
        .map_err(|e| StripeError::InvalidRequest(e.to_string()))
}

impl From<some_stripe_core::StripeClient> for StripeClient {
    fn from(core: some_stripe_core::StripeClient) -> Self {
        Self(core)
    }
}

/// Extension trait mounting resource accessors directly on [`some_stripe_core::StripeClient`].
pub trait StripeClientExt {
    /// Access the Checkout resource namespace (`stripe.checkout`).
    fn checkout(&self) -> CheckoutResource<'_>;

    /// Access the PaymentIntents resource namespace (`stripe.payment_intents`).
    fn payment_intents(&self) -> PaymentIntentsResource<'_>;

    /// Access the Accounts resource namespace (`stripe.accounts`).
    fn accounts(&self) -> AccountsResource<'_>;

    /// Access the Webhooks helper resource (`stripe.webhooks`).
    fn webhooks(&self) -> WebhooksResource;

    /// Access the Refunds resource namespace (`stripe.refunds`).
    fn refunds(&self) -> RefundsResource<'_>;

    /// Access the Disputes resource namespace (`stripe.disputes`).
    fn disputes(&self) -> DisputesResource<'_>;

    /// Access the Balance resource namespace (`stripe.balance`).
    fn balance(&self) -> BalanceResource<'_>;

    /// Access the BalanceTransactions resource namespace (`stripe.balance_transactions`).
    fn balance_transactions(&self) -> BalanceTransactionsResource<'_>;

    /// Access the PaymentLinks resource namespace (`stripe.payment_links`).
    fn payment_links(&self) -> PaymentLinksResource<'_>;

    /// Access the Subscriptions resource namespace (`stripe.subscriptions`).
    fn subscriptions(&self) -> SubscriptionsResource<'_>;
}

impl StripeClientExt for some_stripe_core::StripeClient {
    fn checkout(&self) -> CheckoutResource<'_> {
        CheckoutResource::new(self)
    }

    fn payment_intents(&self) -> PaymentIntentsResource<'_> {
        PaymentIntentsResource::new(self)
    }

    fn accounts(&self) -> AccountsResource<'_> {
        AccountsResource::new(self)
    }

    fn webhooks(&self) -> WebhooksResource {
        WebhooksResource::new()
    }

    fn refunds(&self) -> RefundsResource<'_> {
        RefundsResource::new(self)
    }

    fn disputes(&self) -> DisputesResource<'_> {
        DisputesResource::new(self)
    }

    fn balance(&self) -> BalanceResource<'_> {
        BalanceResource::new(self)
    }

    fn balance_transactions(&self) -> BalanceTransactionsResource<'_> {
        BalanceTransactionsResource::new(self)
    }

    fn payment_links(&self) -> PaymentLinksResource<'_> {
        PaymentLinksResource::new(self)
    }

    fn subscriptions(&self) -> SubscriptionsResource<'_> {
        SubscriptionsResource::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_checkout_session_creation_e2e() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 16384];
            let size = socket.read(&mut request).await.unwrap();
            let wire = String::from_utf8(request[..size].to_vec()).unwrap();
            assert!(wire.starts_with("POST /v1/checkout/sessions HTTP/1.1"));
            assert!(wire.contains("idempotency-key: dep_fixture"));
            assert!(wire.contains("expires_at=1800"));
            assert!(wire.contains("unit_amount%5D=50000"));

            let body = r#"{"id":"cs_test_session_id","object":"checkout.session","url":"https://checkout.stripe.com/c/pay/cs_test_session_id"}"#;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });

        let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
        let client = StripeClient::with_config(config);

        let session = client
            .checkout()
            .sessions()
            .create(&CreateCheckoutSessionParams {
                success_url: "https://example.com/return".into(),
                cancel_url: Some("https://example.com/return".into()),
                mode: "payment".into(),
                client_reference_id: Some("dep_fixture".into()),
                currency: Some("usd".into()),
                unit_amount: Some(50000),
                product_name: Some("Wallet deposit".into()),
                quantity: Some(1),
                expires_at: Some(1800),
                idempotency_key: Some("dep_fixture".into()),
                ..Default::default()
            })
            .await
            .unwrap();

        server.await.unwrap();
        assert_eq!(session.id, "cs_test_session_id");
        assert_eq!(
            session.url.as_deref(),
            Some("https://checkout.stripe.com/c/pay/cs_test_session_id")
        );
    }

    #[tokio::test]
    async fn test_payment_intent_capture_and_cancel() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            // First call: Capture
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut req_buf = vec![0; 8192];
            let n = socket.read(&mut req_buf).await.unwrap();
            let req_str = String::from_utf8(req_buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("POST /v1/payment_intents/pi_123/capture HTTP/1.1"));
            assert!(req_str.contains("idempotency-key: idem_cap"));
            let cap_body = r#"{"id":"pi_123","object":"payment_intent","amount":5000,"amount_received":5000,"currency":"usd","status":"succeeded"}"#;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{cap_body}",
                        cap_body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();

            // Second call: Cancel
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut req_buf = vec![0; 8192];
            let n = socket.read(&mut req_buf).await.unwrap();
            let req_str = String::from_utf8(req_buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("POST /v1/payment_intents/pi_456/cancel HTTP/1.1"));
            let cancel_body = r#"{"id":"pi_456","object":"payment_intent","amount":5000,"currency":"usd","status":"canceled"}"#;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{cancel_body}",
                        cancel_body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });

        let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
        let client = StripeClient::with_config(config);

        let captured = client
            .payment_intents()
            .capture(
                "pi_123",
                Some(&CapturePaymentIntentParams {
                    amount_to_capture: Some(5000),
                    idempotency_key: Some("idem_cap".into()),
                }),
            )
            .await
            .unwrap();

        assert_eq!(captured.status, "succeeded");
        assert_eq!(captured.amount_received, Some(5000));

        let canceled = client
            .payment_intents()
            .cancel("pi_456", None)
            .await
            .unwrap();
        assert_eq!(canceled.status, "canceled");

        server.await.unwrap();
    }

    #[tokio::test]
    async fn test_account_retrieve() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut req_buf = vec![0; 4096];
            let n = socket.read(&mut req_buf).await.unwrap();
            let req_str = String::from_utf8(req_buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("GET /v1/account HTTP/1.1"));
            let body = r#"{"id":"acct_fixture123","charges_enabled":true,"payouts_enabled":true}"#;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });

        let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
        let client = StripeClient::with_config(config);

        let account = client.accounts().retrieve().await.unwrap();
        assert_eq!(account.id, "acct_fixture123");
        assert_eq!(account.charges_enabled, Some(true));

        server.await.unwrap();
    }
}
