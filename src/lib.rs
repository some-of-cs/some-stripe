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
//!     currency: "usd".into(),
//!     unit_amount: 5000, // $50.00
//!     product_name: "Wallet Deposit".into(),
//!     quantity: 1,
//!     expires_at: None,
//!     idempotency_key: Some("dep_sess_123".into()),
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

pub mod client;
pub mod error;
pub mod resources;
pub mod types;
pub mod webhook;
pub mod workflow;

pub use client::{StripeClient, StripeConfig};
pub use error::{StripeError, StripeWebhookError};
pub use resources::{
    AccountsResource, CheckoutResource, PaymentIntentsResource, SessionsResource, WebhooksResource,
};
pub use types::*;
pub use webhook::{
    safe_constant_time_eq, verify_stripe_signature, StripeWebhookWrapper, Webhook,
    DEFAULT_TOLERANCE_SECONDS,
};
pub use workflow::{
    CustomErrorWorkflowBuilder, DefaultErrorWorkflow, ErrorResolution, ErrorSeverity,
    ErrorWorkflowHandler, StripeApiErrorHook, StripeSecurityHook,
};

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
                currency: "usd".into(),
                unit_amount: 50000,
                product_name: "Wallet deposit".into(),
                quantity: 1,
                expires_at: Some(1800),
                idempotency_key: Some("dep_fixture".into()),
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

        let canceled = client.payment_intents().cancel("pi_456", None).await.unwrap();
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
