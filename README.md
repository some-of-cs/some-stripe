# some-stripe

<div align="center">

> **Idiomatic, high-performance Rust SDK and constant-time webhook verification engine for Stripe (API v1 & v2).**  
> Modeled after `stripe-node` ergonomics • Built for the Rust 2024 edition • Zero-clone microkernel & timing-safe cryptography

[![Crates.io](https://img.shields.io/crates/v/some-stripe.svg)](https://crates.io/crates/some-stripe)
[![Version](https://img.shields.io/badge/version-0.8.0-blue.svg)](Cargo.toml)
[![Docs.rs](https://docs.rs/some-stripe/badge.svg)](https://docs.rs/some-stripe)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust: Edition 2024](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](Cargo.toml)
[![CI](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/some-of-cs/some-stripe)
[![Sponsor](https://img.shields.io/badge/Sponsor-%E2%99%A5-ff69b4.svg)](https://github.com/sponsors/some-of-cs)

[Why some-stripe?](#why-some-stripe) •
[Key Features](#key-features) •
[Installation](#installation) •
[Quick Start](#quick-start) •
[Axum & Actix-Web Integration](#axum--actix-web-framework-integration) •
[Webhook & V2 Security](#security--cryptographic-integrity) •
[Architecture](#architecture-overview) •
[Development Status (v0.8.0)](#development-status--module-matrix-v080) •
[Sponsors](#backers--corporate-sponsors)

</div>

---

## Why some-stripe?

Integrating Stripe in modern Rust services often introduces friction:

1. **Compilation Overhead**: Monolithic OpenAPI codegen generates hundreds of thousands of lines of code, bloating compile times and consuming gigabytes of RAM during `cargo build`—even when a microservice only needs to trigger a single checkout session or ingest usage meters.
2. **Developer Ergonomics**: Transitioning between TypeScript/Node services and Rust can be jarring. `some-stripe` adopts the familiar hierarchical namespace ergonomics of `stripe-node` (`stripe.checkout().sessions`, `stripe.payment_intents()`, `stripe.subscriptions()`, `stripe.v2().core()`, `stripe.v2().billing()`).
3. **Serverless & Edge Footprint**: Validating webhook signatures or parsing thin event notifications in AWS Lambda, Cloudflare Workers, or Fastly Compute should not require pulling in hundreds of megabytes of domain types or heavy HTTP client machinery.
4. **Dual-Protocol Serialization & Nullability Precision**: Accurate zero-indexed bracket encoding (`items[0][price]=...`) and 3-state `Nullable<T>` (`Value`, `Null` $\to$ `""`, `Omit`) for Stripe API v1 (`application/x-www-form-urlencoded`), paired with strict JSON serialization and URL-driven pagination for Stripe API v2 (`application/json`).

**`some-stripe` (`v0.8.0`)** solves these challenges with a **4-crate microkernel architecture**: an ultra-lightweight core runtime, zero-copy domain types, an isolated constant-time webhook engine, and comprehensive coverage across **all 8 Stripe product modules (API v1 & v2)**.

---

## Key Features

- **Familiar `stripe-node` Hierarchy (API v1 & v2)**: Natural nested namespaces (`stripe.checkout().sessions`, `stripe.payment_intents()`, `stripe.customers()`, `stripe.subscriptions()`, `stripe.invoices()`, `stripe.accounts()`, `stripe.transfers()`, `stripe.tax()`, `stripe.identity()`, `stripe.issuing()`, `stripe.treasury()`, `stripe.terminal()`, `stripe.v2().core()`, `stripe.v2().billing()`) mounted on a lock-free, `Arc`-backed `StripeClient`.
- **Dual-Protocol Engine (v1 Form & v2 JSON)**:
  - **API v1**: RFC 1738 indexed-bracket form encoding, 3-state `Nullable<T>`, cursor-based `Paginator<T>` with cycle detection, and polymorphic `Expandable<T>` resources.
  - **API v2**: Operation-scoped JSON payloads, SSRF-safe URL-driven `V2Paginator<T>` (`next_page_url` / `previous_page_url`), V2 Core Events & Event Destinations (Amazon EventBridge, Azure Event Grid, Webhooks), `ThinEventHydrator`, and High-Throughput Usage Meter Event Streams (`MeterIngestionEngine`).
- **Cryptographic Webhook & Thin Event Engine**: Constant-time HMAC-SHA256 verification (`safe_constant_time_eq` / `subtle::ConstantTimeEq`), strict 300-second timestamp drift tolerance (`DEFAULT_TOLERANCE_SECONDS = 300`), 1 MB payload ceiling, discriminated union `EventObject` deserialization, and tamper-evident V2 thin event hydration.
- **High-Throughput AI & Usage Meter Billing**: Built-in `MeterIngestionEngine` and `MeterStreamCollector` featuring ephemeral session bearer token caching (`ZeroizeOnDrop`), automatic preemptive token renewal, client-side deduplication, and 1,000-event batch streaming.
- **Zero-Clone Borrowing & Secret Hygiene**: Strict Rust 2024 `&T` borrowing discipline (`Copy` resource handles, `Cow<'_, str>` RFC 4122 UUID v4 idempotency keys) and automatic memory zeroization (`ZeroizeOnDrop`) with `[REDACTED]` `Debug` formatting on all secret keys and ephemeral tokens.
- **Statutory & Financial Compliance**: Built-in guards for PCI-DSS 4.0 (zero raw PAN in memory), PSD2 SCA 3DS2 redirects, GDPR Art. 15/17 erasure & portability, EU Omnibus Directive 2019/2161 price transparency, and FTC Section 5 meter billing accuracy.

---

## Installation

Add `some-stripe` to your `Cargo.toml`:

```toml
[dependencies]
some-stripe = "0.8.0"
tokio = { version = "1", features = ["full"] }
```

> [!NOTE]
> `some-stripe` requires **Rust 1.85+** (Rust 2024 Edition).

---

## Quick Start

### 1. Creating a Hosted Checkout Session (API v1)

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
            unit_amount: 5000, // $50.00 (in minor units)
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

### 2. Capturing a PaymentIntent (API v1)

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

### 3. Constant-Time Webhook Verification & V2 Thin Event Hydration

Verify webhook signatures in constant time and hydrate lightweight API v2 thin event notifications with replay protection:

```rust,no_run
use some_stripe::{StripeClient, StripeWebhookWrapper, V2ThinEvent};

async fn handle_webhook(
    stripe: &StripeClient,
    payload: &[u8],
    sig_header: &str,
    webhook_secret: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Verify HMAC-SHA256 in constant time with a 300s replay drift window
    let verified = StripeWebhookWrapper::from_raw(payload, sig_header, webhook_secret, None)?;
    println!("Verified v1 event: {} [{}]", verified.event_id(), verified.event_type());

    // 2. Or parse & hydrate a lightweight Stripe API v2 Thin Event Notification
    let thin: V2ThinEvent = serde_json::from_slice(payload)?;
    let full_v2_event = stripe.v2().core().hydrator().hydrate(&thin).await?;
    println!("Hydrated v2 event: {} ({})", full_v2_event.id, full_v2_event.event_type);

    Ok(())
}
```

---

### 4. High-Throughput Usage Meter Ingestion (Stripe API v2 Billing)

Stream high-frequency AI token or infrastructure usage events using `stripe.v2().billing()` and `MeterIngestionEngine`:

```rust,no_run
use some_stripe::{
    AggregationFormula, CreateBillingMeterParams, CustomerMappingSettings,
    MeterEventPayload, StripeClient, ValueSettings,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stripe = StripeClient::new("sk_test_YOUR_SECRET_KEY");

    // 1. Provision a V2 Billing Meter
    let meter = stripe
        .v2()
        .billing()
        .meters()
        .create(&CreateBillingMeterParams::new(
            "AI LLM Inference Tokens",
            "llm_tokens",
            AggregationFormula::Sum,
            ValueSettings::new("value"),
            CustomerMappingSettings::by_id("stripe_customer_id"),
        ))
        .await?;
    println!("Created V2 Meter: {} ({:?})", meter.id, meter.status);

    // 2. High-throughput buffered ingestion with session token caching & deduplication
    let engine = stripe.v2().billing().create_ingestion_engine();
    let event = MeterEventPayload::new("llm_tokens", "cus_12345", "1500")
        .with_identifier("evt_unique_idemp_001")
        .with_timestamp(1_700_000_000);

    engine.push_event(event).await?;
    engine.flush().await?;
    Ok(())
}
```

---

### 5. Resilient Retry Policies & Security Escalation

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

`some-stripe` is engineered from the ground up for zero-trust financial environments:

- **Constant-Time Verification (Gate G2)**: All signature comparisons use `safe_constant_time_eq` (`subtle::ConstantTimeEq`) to eliminate timing side-channel vulnerabilities.
- **Replay Protection (`DEFAULT_TOLERANCE_SECONDS = 300`)**: Both v1 webhook wrappers and v2 `ThinEventHydrator` enforce a strict 300-second clock drift window against stale event replays and future-dated forged payloads.
- **Automatic Secret Zeroization (`ZeroizeOnDrop`)**: API secret keys (`SecretKey`), v2 account access tokens (`V2AccountToken`), and ephemeral meter session bearer tokens (`SessionTokenDetails`) are automatically scrubbed from memory on drop and masked as `[REDACTED]` in `Debug` / `Display` formatting.
- **SSRF & Path Traversal Isolation**: All resource IDs are validated via `validate_and_encode_id` to prevent URL path traversal (`..`, `/`, `?`, `#`), and URL-driven v2 pagination (`resolve_v2_url`) strips untrusted external hosts to prevent Server-Side Request Forgery (SSRF).
- **Bounded Payload Allocations**: Incoming webhook requests enforce a strict 1 MB (`MAX_WEBHOOK_PAYLOAD_BYTES`) ceiling before allocation.

---

## Axum & Actix-Web Framework Integration

`some-stripe` is engineered from the ground up for asynchronous Rust web microservices and includes native **`$VAR_NAME` environment and `.env*` secret resolution**:

```rust,no_run
use some_stripe::{SecretKey, StripeClient};

// Automatically resolves $STRIPE_SECRET_KEY and $STRIPE_WEBHOOK_SECRET from process env,
// .env.local, .env.dev, .env.development, .env.test, .env.prod, .env.production, .env, or .env.example!
let stripe = StripeClient::new("$STRIPE_SECRET_KEY");
let webhook_secret = SecretKey::new("$STRIPE_WEBHOOK_SECRET");
```

Complete, production-grade [**Axum (`axum 0.8`) and Actix-Web (`actix-web 4`) Example Suites**](examples/) are included in `examples/`, accompanied by full integration test suites in [`tests/axum_examples_integration/`](tests/axum_examples_integration/) and [`tests/actix_examples/`](tests/actix_examples/):

```bash
# Run the Unified Axum Server (Port 3000) or Standalone Axum Microservices (Ports 3001..3006)
cargo run --example axum_stripe_server
cargo run --example 01_axum_webhooks
cargo run --example 02_axum_checkout_and_payments
cargo run --example 03_axum_subscriptions_and_invoicing
cargo run --example 04_axum_connect_marketplace
cargo run --example 05_axum_specialized_products
cargo run --example 06_axum_v2_meter_billing

# Run the Full Axum Examples Integration Test Suite
cargo test --test axum_examples_integration_test
```

See [**`examples/README.md`**](examples/README.md) and [**`tests/axum_examples_integration/README.md`**](tests/axum_examples_integration/README.md) for full endpoint specifications and test scenarios.

---

## Rocket.rs Web Framework Example

A comprehensive, production-ready web application showcasing how to integrate `some-stripe` into the **[Rocket.rs (v0.5)](https://rocket.rs/)** web framework is provided under [`examples/rocket_stripe`](examples/rocket_stripe/):

- **Native `$VAR_NAME` Secret Resolution**: Uses `StripeClient::new("$STRIPE_SECRET_KEY")` and `SecretKey::new("$STRIPE_WEBHOOK_SECRET")` to automatically resolve secrets from process environment variables or `.env.local`, `.env.dev`, `.env`, and `.env.example`.
- **End-to-End Coverage**: Route modules and dual-mode integration test suites (`examples/rocket_stripe/tests/`) for all 8 Stripe SDK services (Checkout, Payments & Settlements, Customers & Billing, Connect & Marketplaces, Tax, Identity, Issuing, Treasury, Terminal, API V2 Core & High-Throughput Usage Meters).
- **Cryptographic Webhooks**: Constant-time signature verification (`Webhook::construct_event`), replay protection (300s tolerance), 1 MB payload limits, and automatic V2 thin event hydration.
- **Runnable Documentation**: Complete directory of endpoints and verbatim `curl` commands in [`examples/rocket_stripe/README.md`](examples/rocket_stripe/README.md).

Run the example or its integration test suite directly:
```bash
cargo run --bin rocket-stripe-example
cargo test -p rocket-stripe-example
```

---

## Architecture Overview

`some-stripe` is structured as a 4-crate Cargo workspace following a strict microkernel separation of concerns:

```text
some-stripe/
├── Cargo.toml                           # Root facade crate (some-stripe v0.8.0) & workspace manifest
├── src/
│   ├── lib.rs                           # Unified StripeClient, StripeClientExt, and public re-exports
│   ├── crates/
│   │   ├── some-stripe-core/            # Microkernel runtime, transport, retry workflows & serializers
│   │   │   └── src/
│   │   │       ├── client.rs            # Lock-free Arc<ClientInner> HTTP execution engine
│   │   │       ├── error.rs             # StripeError & StripeWebhookError taxonomy
│   │   │       ├── idempotency.rs       # RFC 4122 UUID v4 generator & header injection
│   │   │       ├── workflow.rs          # Jittered exponential backoff & security escalation hooks
│   │   │       └── patterns/            # Shared DRY patterns catalog
│   │   │           ├── serializer.rs    # v1 indexed-bracket form & v2 JSON serializers
│   │   │           ├── nullable.rs      # 3-state Nullable<T> (Value, Null -> "", Omit)
│   │   │           ├── pagination.rs    # v1 cursor Paginator<T> with CursorLoopDetector
│   │   │           ├── v2_pagination.rs # v2 URL-driven V2Paginator<T> with SSRF & loop guards
│   │   │           └── meter_serializer.rs # High-throughput JSON/NDJSON meter batch serializer
│   │   ├── some-stripe-types/           # Zero-copy domain models, DTOs & EventObject union
│   │   └── some-stripe-webhooks/        # Constant-time HMAC-SHA256 & v2 thin event probe engine
│   └── resources/                       # Idiomatic resource namespaces (Modules 1..8)
│       ├── checkout.rs                  # Module 4: Hosted Checkout Sessions & Payment Links
│       ├── payment_intents.rs           # Module 3: PaymentIntents, Refunds, Disputes, Balance
│       ├── customers.rs / subscriptions.rs / invoices.rs / credit_notes.rs / quotes.rs / billing_portal.rs # Module 5
│       ├── accounts.rs / persons.rs / transfers.rs / application_fees.rs / payouts.rs # Module 6: Connect
│       ├── tax.rs / identity.rs / issuing*.rs / treasury*.rs / terminal.rs # Module 7: Specialized Products
│       ├── v2_core.rs / v2_events.rs / v2_destinations.rs / v2_accounts.rs / v2_hydration.rs # Module 8 (US-8.1.0)
│       └── v2/                          # Module 8 (US-8.2.0): V2 Billing & Meter Ingestion Engine
│           ├── billing.rs               # stripe.v2().billing() facade
│           ├── billing_meters.rs        # V2 Billing Meters CRUD (/v2/billing/meters)
│           ├── meter_session.rs         # Ephemeral Meter Event Sessions (/v2/billing/meter_event_session)
│           ├── meter_stream.rs          # Batch Meter Event Stream (/v2/billing/meter_event_stream)
│           ├── meter_adjustments.rs     # Meter Event Adjustments (/v2/billing/meter_event_adjustments)
│           └── ingestion_engine.rs      # MeterIngestionEngine & MeterStreamCollector
├── tests/                               # End-to-end wire mock & omnibus integration test suites
└── docs/                                # Architectural research, user stories & live Kanban boards
```

---

## Development Status & Module Matrix (`v0.8.0`)

All **8 Core Architectural Modules** (16 User Stories, 80 Micro-Units) are **100% implemented, verified, and merged into `main`**:

| Module | Domain & Capabilities | Client Namespace & Accessors | Quality Gates | Status (`v0.8.0`) |
| :--- | :--- | :--- | :---: | :---: |
| **Module 1: Microkernel Core Runtime** | Lock-free `StripeClient`, dual-protocol v1/v2 serializers, 3-state `Nullable<T>`, `Expandable<T>`, `Paginator<T>`, RFC 4122 UUID v4 idempotency, jittered backoff | `StripeClient::new()`, `StripeConfig`, `Nullable`, `Expandable`, `Paginator` | `G0`–`G4`, `G-Borrow`, `G-Complexity`, `G-Docs` | ✅ **Available** |
| **Module 2: Cryptographic Webhooks** | Constant-time HMAC-SHA256 (`subtle::ConstantTimeEq`), 300s replay drift guard, 1 MB payload cap, `EventObject` union, v2 thin event probe | `stripe.webhooks()`, `StripeWebhookWrapper::from_raw()`, `is_thin_event()` | `G0`–`G4`, `G2`, `G-Borrow`, `G-Docs` | ✅ **Available** |
| **Module 3: Payments & Settlement** | PaymentIntents lifecycle, 3DS2 SCA redirect handling, Refunds, Disputes, Balance & BalanceTransactions ledger | `stripe.payment_intents()`, `stripe.refunds()`, `stripe.disputes()`, `stripe.balance()`, `stripe.balance_transactions()` | `G0`–`G4`, `G-Law`, `G-Borrow`, `G-Docs` | ✅ **Available** |
| **Module 4: Hosted Checkout & Links** | Hosted Checkout Sessions (`create`, `retrieve`, `expire`, `list_line_items`), Payment Links, EU Omnibus Directive 2019/2161 price transparency | `stripe.checkout().sessions`, `stripe.payment_links()`, `validate_omnibus_price_transparency()` | `G0`–`G4`, `G-Law`, `G-Borrow`, `G-Docs` | ✅ **Available** |
| **Module 5: Customers & Billing** | Customers (GDPR Art. 15/17 erasure/export), Subscriptions, SubscriptionSchedules, Invoices, CreditNotes, Quotes, BillingPortal | `stripe.customers()`, `stripe.subscriptions()`, `stripe.subscription_schedules()`, `stripe.invoices()`, `stripe.credit_notes()`, `stripe.quotes()`, `stripe.billing_portal()` | `G0`–`G4`, `G-Law`, `G-Borrow`, `G-Docs` | ✅ **Available** |
| **Module 6: Connect & Marketplaces** | Connected Accounts, KYC Persons, AccountLinks, AccountSessions, Transfers, ApplicationFees, Payouts, `Stripe-Account` multi-tenancy | `stripe.accounts()`, `stripe.persons()`, `stripe.account_links()`, `stripe.account_sessions()`, `stripe.transfers()`, `stripe.application_fees()`, `stripe.payouts()` | `G0`–`G4`, `G-Law`, `G-Borrow`, `G-Docs` | ✅ **Available** |
| **Module 7: Specialized Product Namespaces** | Stripe Tax (Calculations, Transactions, Registrations, Settings), Identity (`VerificationSession` PII redaction), Issuing, Treasury, Terminal | `stripe.tax()`, `stripe.identity()`, `stripe.issuing()`, `stripe.treasury()`, `stripe.terminal()` | `G0`–`G4`, `G2`, `G-Law`, `G-Borrow`, `G-Docs` | ✅ **Available** |
| **Module 8: API V2 Next-Gen Services** | V2 Core Events, Event Destinations (EventBridge, Azure Event Grid, Webhooks), V2 Accounts & Tokens, URL-driven `V2Paginator<T>`, `ThinEventHydrator`, V2 Billing Meters, Meter Sessions/Streams/Adjustments, `MeterIngestionEngine` | `stripe.v2().core()`, `stripe.v2().billing()`, `stripe.v2().hydrator()`, `stripe.v2().billing().create_ingestion_engine()` | `G0`–`G4`, `G2`, `G-Law`, `G-Borrow`, `G-Complexity`, `G-Docs` | ✅ **Available** |

### Quality Gate Compliance Summary (`v0.8.0`)

- **Gate G0 (Compilation & Formatting)**: `0` warnings under `cargo clippy --workspace --all-targets -- -D warnings` and clean `cargo fmt --all -- --check` on Rust 2024 edition.
- **Gate G-Borrow (Zero-Clone Discipline)**: Zero deep `.clone()` calls on domain entities; all resource namespaces derive `Clone, Copy` over `&'a StripeClient`.
- **Gate G2 (Cryptographic & Security Invariants)**: Constant-time signature verification, 300s timestamp drift replay protection, SSRF URL origin enforcement, path traversal rejection, and `ZeroizeOnDrop` + `[REDACTED]` secret redaction.
- **Gate G-Law (Statutory Financial Compliance)**: PCI-DSS 4.0 zero raw PAN, PSD2 SCA 3DS2, GDPR Art. 15/17, ISO 4217 minor currency units, EU Omnibus Directive 2019/2161, and FTC Section 5 meter accuracy.
- **Gate G-Complexity**: Cyclomatic complexity $CC \le 6$ across coordinator functions and endpoints (hard cap $CC \le 10$, `0` functions $> 15$).
- **Gate G1, G4 & G-Docs**: 100% passing unit tests, wire contract mock suites, and runnable rustdoc doctests (`cargo test --workspace && cargo test --doc`).

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

1. Ensure all workspace unit, wire integration, and doc-tests pass:
   ```bash
   cargo test --workspace
   cargo test --doc
   ```
2. Ensure code compiles without warnings on the Rust 2024 edition:
   ```bash
   cargo clippy --workspace --all-targets -- -D warnings
   ```
3. Format all code according to standard `rustfmt`:
   ```bash
   cargo fmt --all -- --check
   ```

---

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE)).
