# some-stripe

<div align="center">

> **Idiomatic, high-performance Rust SDK and constant-time webhook verification engine for Stripe.**  
> Modeled after `stripe-node` ergonomics • Built for the Rust 2024 edition • Resilient & timing-safe

[![Crates.io](https://img.shields.io/crates/v/some-stripe.svg)](https://crates.io/crates/some-stripe)
[![Docs.rs](https://docs.rs/some-stripe/badge.svg)](https://docs.rs/some-stripe)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust: Edition 2024](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](Cargo.toml)
[![CI](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/some-of-cs/some-stripe)
[![Sponsor](https://img.shields.io/badge/Sponsor-%E2%99%A5-ff69b4.svg)](https://github.com/sponsors/some-of-cs)

[Why some-stripe?](#why-some-stripe) •
[Key Features](#key-features) •
[Installation](#installation) •
[Quick Start](#quick-start) •
[Webhook Security](#security--cryptographic-integrity) •
[Error Workflows](#4-resilient-retry-policies--security-escalation) •
[Architecture](#architecture-overview) •
[Sponsors](#backers--corporate-sponsors)

</div>

---

## Why some-stripe?

Integrating Stripe in modern Rust services often introduces friction:

1. **Compilation Overhead**: Monolithic OpenAPI codegen generates hundreds of thousands of lines of code, bloating compile times and consuming gigabytes of RAM during `cargo build`—even when a microservice only needs to trigger a single checkout session.
2. **Developer Ergonomics**: Transitioning between TypeScript/Node services and Rust can be jarring. `some-stripe` adopts the familiar hierarchical namespace ergonomics of `stripe-node` (`stripe.checkout().sessions()`, `stripe.payment_intents()`, `stripe.accounts()`, `stripe.webhooks()`).
3. **Serverless & Edge Footprint**: Validating webhook signatures in AWS Lambda, Cloudflare Workers, or Fastly Compute should not require pulling in hundreds of megabytes of domain types or heavy HTTP client machinery.
4. **Serialization & Protocol Precision**: Accurate zero-indexed bracket encoding, 3-state nullability handling, and idempotency guarantees prevent runtime HTTP 400 errors.

**`some-stripe`** solves these challenges with a lean, modular architecture: an ultra-lightweight client foundation coupled with an isolated, constant-time cryptographic verification engine.

---

## Key Features

- **Familiar `stripe-node` Hierarchy**: Natural nested namespaces (`stripe.checkout().sessions()`, `stripe.payment_intents()`, `stripe.accounts()`, `stripe.webhooks()`) mounted on a thread-safe, cheaply cloneable `StripeClient` (`Arc<StripeConfig>`).
- **Cryptographic Webhook Engine**: Zero-timing-attack HMAC-SHA256 verification (`safe_constant_time_eq`), automatic 300-second replay attack mitigation, and strongly typed event parsing with zero-boilerplate accessors.
- **Hosted Checkout & PaymentIntents**: First-class support for creating Checkout Sessions, capturing/canceling PaymentIntents, and managing merchant account profiles.
- **Resilient Error Workflows**: Fine-grained exponential backoff with jitter for transient errors (HTTP 429, 5xx, network timeouts), plus dedicated security escalation callbacks for tampered signatures.
- **Zero-Leak Secret Redaction**: Sensitive API keys (`sk_live_...`) are automatically redacted in `Debug` logs to protect production telemetry.
- **Rust 2024 Edition**: Clean async/await, modern error handling (`thiserror`), and zero-unsafe application code.

---

## Installation

Add `some-stripe` to your `Cargo.toml`:

```toml
[dependencies]
some-stripe = "0.1.0"
tokio = { version = "1", features = ["full"] }
```

> [!NOTE]
> `some-stripe` requires **Rust 1.85+** (Rust 2024 Edition).

---

## Quick Start

### 1. Creating a Hosted Checkout Session

```rust,no_run
use some_stripe::{CreateCheckoutSessionParams, StripeClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the thread-safe client (Arc-backed configuration)
    let stripe = StripeClient::new("sk_test_YOUR_SECRET_KEY");

    // Create a Checkout Session
    let session = stripe
        .checkout()
        .sessions
        .create(&CreateCheckoutSessionParams {
            success_url: "https://example.com/payment/success".into(),
            cancel_url: Some("https://example.com/payment/cancel".into()),
            mode: "payment".into(),
            client_reference_id: Some("order_ref_123".into()),
            currency: "usd".into(),
            unit_amount: 5000, // $50.00 (in cents)
            product_name: "Pro Subscription Deposit".into(),
            quantity: 1,
            expires_at: None,
            idempotency_key: Some("idemp_order_123".into()),
        })
        .await?;

    println!("Hosted Checkout URL: {:?}", session.url);
    Ok(())
}
```

---

### 2. Capturing a PaymentIntent

```rust,no_run
use some_stripe::{CapturePaymentIntentParams, StripeClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stripe = StripeClient::new("sk_test_YOUR_SECRET_KEY");

    let intent = stripe
        .payment_intents()
        .capture(
            "pi_12345",
            Some(&CapturePaymentIntentParams {
                amount_to_capture: Some(5000),
                idempotency_key: Some("idem_cap_123".into()),
            }),
        )
        .await?;

    println!(
        "Captured PaymentIntent: id={}, status={}, amount_received={:?}",
        intent.id, intent.status, intent.amount_received
    );
    Ok(())
}
```

---

### 3. Constant-Time Webhook Verification

Verify webhook signatures in constant time to eliminate timing side-channel attacks:

```rust
use some_stripe::StripeWebhookWrapper;

fn handle_webhook(payload: &[u8], sig_header: &str, webhook_secret: &str) {
    // Verifies HMAC-SHA256 in constant time with a 300s replay drift window
    match StripeWebhookWrapper::from_raw(payload, sig_header, webhook_secret, None) {
        Ok(event) => {
            println!("Verified event: {} [{}]", event.event_id(), event.event_type());

            if event.is_checkout_completed() {
                println!(
                    "Payment successful: order={:?}, total={:?} {}",
                    event.order_id(),
                    event.amount_total(),
                    event.currency().unwrap_or_default()
                );
            }
        }
        Err(err) => {
            eprintln!("Rejected invalid webhook signature or expired timestamp: {err}");
        }
    }
}
```

---

### 4. Resilient Retry Policies & Security Escalation

Configure automated retries for transient failures and attach real-time security alerts for tampered signatures:

```rust
use some_stripe::{CustomErrorWorkflowBuilder, StripeClient, StripeConfig};
use std::time::Duration;

let workflow = CustomErrorWorkflowBuilder::new()
    .max_retries(3)
    .initial_delay(Duration::from_millis(500))
    // Trigger security telemetry on invalid signatures or replay drift
    .on_security_incident(|err, payload_size| {
        eprintln!("SECURITY ALERT: Tampered webhook ({payload_size} bytes): {err:?}");
    })
    // Observability hook for API rate limits and errors
    .on_api_error(|status, code, message| {
        eprintln!("Stripe API Error (HTTP {status}) [{code}]: {message}");
    })
    .build();

let config = StripeConfig::new("sk_live_YOUR_SECRET_KEY").with_error_workflow(workflow);
let stripe = StripeClient::with_config(config);
```

---

## Security & Cryptographic Integrity

`some-stripe` is engineered from the ground up for zero-trust environments:

- **Constant-Time Verification**: All signature comparisons use `safe_constant_time_eq` (`openssl::memcmp::eq`) to eliminate timing side-channel vulnerabilities.
- **Replay Protection**: Incoming webhook timestamps (`t`) are validated against a 300-second tolerance window (`DEFAULT_TOLERANCE_SECONDS`) to reject replay attacks.
- **Bounded Allocations**: Webhook processing enforces strict size validation to protect your services from memory exhaustion attacks.
- **Redacted Secret Logging**: API keys and secrets are explicitly redacted from `std::fmt::Debug` implementations to prevent accidental leakage in telemetry or crash reports.

---

## Architecture Overview

```text
some-stripe/
├── src/
│   ├── lib.rs                 # Public prelude, re-exports, and doctests
│   ├── client.rs              # Thread-safe StripeClient (Arc<StripeConfig>)
│   ├── error.rs               # Unified StripeError & StripeWebhookError taxonomy
│   ├── types.rs               # Strongly typed Stripe models and request parameters
│   ├── webhook.rs             # Constant-time HMAC-SHA256 signature engine & wrapper
│   ├── workflow.rs            # Configurable retry policies & security escalation hooks
│   └── resources/             # Resource namespaces (mirroring stripe-node)
│       ├── checkout.rs        # stripe.checkout().sessions
│       ├── payment_intents.rs # stripe.payment_intents()
│       ├── accounts.rs        # stripe.accounts()
│       └── webhooks.rs        # stripe.webhooks() helper namespace
└── docs/                      # Architectural specifications & domain research
```

### Module Matrix & Roadmap

| Resource / Namespace | Included Services & Accessors | Status |
| :--- | :--- | :---: |
| **Checkout** | `stripe.checkout().sessions.create()`, `.retrieve()`, `.expire()` | **Available** |
| **PaymentIntents** | `stripe.payment_intents().create()`, `.retrieve()`, `.capture()`, `.cancel()` | **Available** |
| **Accounts** | `stripe.accounts().retrieve()`, `.retrieve_by_id()` | **Available** |
| **Webhooks** | `stripe.webhooks().construct_event()`, `StripeWebhookWrapper` | **Available** |
| **Error Workflows** | Exponential backoff, jitter retries, security incident callbacks | **Available** |
| **Billing & Subscriptions** | Customers, subscriptions, invoices, prices, products | *Roadmap* |
| **Stripe Connect** | Multi-party accounts, transfers, platform payouts, account sessions | *Roadmap* |
| **Stripe Tax** | Automated sales tax, VAT, GST calculation and registrations | *Roadmap* |
| **Stripe API v2** | High-frequency meter events, thin event hydration | *Roadmap* |

---

## Backers & Corporate Sponsors

`some-stripe` is an independent, community-driven open-source project. If your business relies on this crate to process transactions or verify webhooks, corporate sponsorship directly finances continuous maintenance, zero-day security updates, and upcoming module expansion.

<div align="center">

[![Become a Sponsor](https://img.shields.io/badge/Become%20a%20Sponsor-%E2%99%A5-ea4aaa?style=for-the-badge)](https://github.com/sponsors/some-of-cs)

</div>

### Corporate Sponsorship Tiers & Benefits

| Tier | Monthly | Placement & Benefits |
| :--- | :--- | :--- |
| **🏆 Visionary** | $1,000 / mo | Large logo on `README.md`, website, and Docs.rs header • Prioritized GitHub issues • Dedicated quarterly roadmap review |
| **🥇 Gold Partner** | $500 / mo | Prominent logo on `README.md` • Prioritized PR reviews • Direct issue escalation |
| **🥈 Silver Backer** | $150 / mo | Company logo, link, and mention in `README.md` and release changelogs |
| **☕ Individual** | $10 – $50 / mo | Listed as a contributor and backer on GitHub |

> [!TIP]
> **Need an Invoice or Procurement Documentation?**  
> We can issue formal VAT/sales-tax compliant invoices via Stripe Invoicing for corporate sponsors. Please contact [sponsors@someofcs.tech](mailto:sponsors@someofcs.tech).

---

## Contributing

We welcome community contributions! To ensure code quality and consistency:

1. Ensure all tests and doc-tests pass:
   ```bash
   cargo test
   ```
2. Ensure code compiles without warnings on the Rust 2024 edition:
   ```bash
   cargo clippy --all-targets -- -D warnings
   ```
3. Format all code according to standard `rustfmt`:
   ```bash
   cargo fmt --check
   ```

---

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE)).
