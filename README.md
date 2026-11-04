# some-stripe

[![Crates.io](https://img.shields.io/crates/v/some-stripe.svg)](https://crates.io/crates/some-stripe)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

An idiomatic, production-grade Rust SDK and webhook verification engine for the **Stripe REST API**, modeled directly after `stripe-node`.

## Features

- **Idiomatic Resource Namespaces**:
  - `client.checkout().sessions()`: Create and manage hosted Checkout sessions.
  - `client.payment_intents()`: Create, retrieve, confirm, capture, and cancel payment intents.
  - `client.accounts()`: Retrieve connected and primary merchant account profiles.
  - `client.webhooks()`: Construct and verify incoming webhook events.
- **Robust Webhook Engine**:
  - Constant-time cryptographic HMAC-SHA256 comparison resistant to timing side-channels.
  - Configurable tolerance window (300s default) against clock drift and replay attacks.
  - Strongly typed [`StripeWebhookWrapper`] with zero-boilerplate accessors (`order_id()`, `payment_intent_id()`, `amount_total()`, `currency()`).
- **Customizable Error Workflows**:
  - Fine-grained retry strategies with exponential backoff and jitter for transient errors (HTTP 429, 5xx, network timeouts).
  - Security incident escalation callbacks (`on_security_incident`) for tampered signatures and replay attacks.
  - API error hooks (`on_api_error`) for real-time observability.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
some-stripe = "0.1.0"
```

Or reference directly from GitHub:

```toml
[dependencies]
some-stripe = { git = "https://github.com/some-of-cs/some-stripe.git" }
```

## Quick Start

### 1. Creating a Checkout Session

```rust,no_run
use some_stripe::{StripeClient, CreateCheckoutSessionParams};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = StripeClient::new("sk_test_YOUR_SECRET_KEY");

    let session = client
        .checkout()
        .sessions()
        .create(&CreateCheckoutSessionParams {
            success_url: "https://example.com/payment/success".into(),
            cancel_url: Some("https://example.com/payment/cancel".into()),
            mode: "payment".into(),
            client_reference_id: Some("order_12345".into()),
            currency: "usd".into(),
            unit_amount: 5000, // $50.00 in cents
            product_name: "Property Deposit".into(),
            quantity: 1,
            expires_at: None,
            idempotency_key: Some("order_12345".into()),
        })
        .await?;

    println!("Hosted Checkout URL: {:?}", session.url);
    Ok(())
}
```

### 2. Capturing a PaymentIntent

```rust,no_run
use some_stripe::{StripeClient, CapturePaymentIntentParams};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = StripeClient::new("sk_test_YOUR_SECRET_KEY");

    let intent = client
        .payment_intents()
        .capture(
            "pi_12345",
            Some(&CapturePaymentIntentParams {
                amount_to_capture: Some(5000),
            }),
        )
        .await?;

    println!("Captured: status={}, amount={:?}", intent.status, intent.amount_received);
    Ok(())
}
```

### 3. Webhook Signature Verification & Extraction

```rust
use some_stripe::StripeWebhookWrapper;

fn handle_webhook(raw_body: &[u8], sig_header: &str, webhook_secret: &str) {
    match StripeWebhookWrapper::from_raw(raw_body, sig_header, webhook_secret, None) {
        Ok(event) => {
            println!("Verified event {} ({})", event.event_id(), event.event_type());
            if event.is_checkout_completed() {
                println!(
                    "Checkout session completed: order={:?}, intent={:?}, total={:?}",
                    event.order_id(),
                    event.payment_intent_id(),
                    event.amount_total()
                );
            }
        }
        Err(err) => {
            eprintln!("Invalid webhook signature or expired timestamp: {err}");
        }
    }
}
```

### 4. Custom Error Workflow

```rust
use some_stripe::{CustomErrorWorkflowBuilder, StripeClient, StripeConfig};
use std::time::Duration;

let workflow = CustomErrorWorkflowBuilder::new()
    .max_retries(4)
    .initial_delay(Duration::from_millis(500))
    .on_security_incident(|err, payload_size| {
        eprintln!("ALERT: Signature violation detected on {payload_size} bytes: {err:?}");
    })
    .on_api_error(|status, code, message| {
        eprintln!("Stripe API returned HTTP {status} [{code}]: {message}");
    })
    .build();

let config = StripeConfig::new("sk_live_...").with_error_workflow(workflow);
let client = StripeClient::with_config(config);
```

## Security & Compliance

- Webhook signature headers (`t={timestamp},v1={sig}`) are evaluated using constant-time cryptographic comparisons (`openssl::memcmp::eq`) to prevent timing side channels.
- Timestamps outside the tolerance window (default 300 seconds) are rejected to eliminate replay vulnerabilities.
- Maximum payload sizes are capped at 1 MB to prevent unbounded memory allocation.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE)).
