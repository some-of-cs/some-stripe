# `some-stripe` + Axum (`axum 0.8`) Examples Suite (`examples/axum/`)

This directory provides a complete, production-ready **Axum (`axum 0.8`)** integration suite covering **all 8 Stripe architectural modules (API v1 & v2)**, including both a **unified modular server (`examples/axum/axum_stripe_server/`)** and **6 standalone microservice examples (`01_axum_webhooks.rs`..`06_axum_v2_meter_billing.rs`)**.

---

## Architectural Highlights for Axum

1. **Native `$VAR_NAME` Secret Resolution (PR #150)**:
   - Every Axum example and integration test initializes `StripeClient::new("$STRIPE_SECRET_KEY")` and `SecretKey::new("$STRIPE_WEBHOOK_SECRET")` (or `AppState::new("$STRIPE_SECRET_KEY", "$STRIPE_WEBHOOK_SECRET")`).
   - `some-stripe` automatically resolves `$VAR_NAME` references from process environment variables or workspace dotenv files in priority order: `.env.local`, `.env.dev`, `.env.development`, `.env.test`, `.env.prod`, `.env.production`, `.env`, `.env.example`, or custom `.env.*` files.
2. **Lock-Free Zero-Clone Concurrency (`Gate G-Borrow`)**:
   - `AppState` holds `Arc<StripeClient>`, [`SecretKey`] (`ZeroizeOnDrop` + `[REDACTED]` `Debug`), and `Arc<MeterStreamCollector>`.
   - Resource handles derive `Copy` over `&'a StripeClient`, enabling zero-allocation nested namespace calls (`state.stripe.checkout().sessions`, `state.stripe.payment_intents()`, `state.stripe.v2().billing()`).
3. **Constant-Time Webhook Security & V2 Thin Event Hydration (`Gate G2` & Module 2)**:
   - Extracts raw request bytes (`axum::body::Bytes`) before JSON parsing.
   - Detects API V2 thin events via `is_thin_event(&body)`, verifies them with `construct_v2_thin_event`, and hydrates full live resources via `ThinEventHydrator`.
   - Verifies V1 snapshot events in constant time (`subtle::ConstantTimeEq`) with a 300-second replay drift guard (`Some(300)`).
4. **Statutory & Financial Compliance (`Gate G-Law`)**:
   - EU Omnibus Directive 2019/2161 price transparency (`validate_omnibus_price_transparency`) with inline `PaymentLinkPriceDataParams` binding, GDPR Art. 15/17 customer portability & erasure, and BSA/FinCEN individual cardholder KYC fields.

---

## Axum Example Matrix

| Example Source Path | Command | Port | Modules & SDK Services Covered |
| :--- | :--- | :---: | :--- |
| **[`examples/axum/axum_stripe_server/`](axum_stripe_server/main.rs)** | `cargo run --example axum_stripe_server` | `3000` | **All 8 Modules Unified**: Mounts `/webhooks/stripe`, `/payments/*`, `/checkout/*`, `/billing/*`, `/connect/*`, `/specialized/*`, and `/v2/*` |
| **[`examples/axum/01_axum_webhooks.rs`](01_axum_webhooks.rs)** | `cargo run --example 01_axum_webhooks` | `3001` | **Module 2**: Constant-time HMAC-SHA256 verification, V2 Thin Event routing & hydration, `EventObject` union |
| **[`examples/axum/02_axum_checkout_and_payments.rs`](02_axum_checkout_and_payments.rs)** | `cargo run --example 02_axum_checkout_and_payments` | `3002` | **Modules 3 & 4**: Hosted Checkout Sessions, Payment Links (EU Omnibus `price_data` binding), PaymentIntents, Refunds |
| **[`examples/axum/03_axum_subscriptions_and_invoicing.rs`](03_axum_subscriptions_and_invoicing.rs)** | `cargo run --example 03_axum_subscriptions_and_invoicing` | `3003` | **Module 5**: Customer vaulting (GDPR Art. 15/17), 14-day trial Subscriptions, Invoices, Customer Portal |
| **[`examples/axum/04_axum_connect_marketplace.rs`](04_axum_connect_marketplace.rs)** | `cargo run --example 04_axum_connect_marketplace` | `3004` | **Module 6**: Express Connected Accounts, `AccountOnboarding` links, KYC Persons (`Stripe-Account`), Transfers, Payouts |
| **[`examples/axum/05_axum_specialized_products.rs`](05_axum_specialized_products.rs)** | `cargo run --example 05_axum_specialized_products` | `3005` | **Module 7**: Stripe Tax (calculate & commit), Identity KYC sessions, Issuing Cardholders & Cards, Treasury, Terminal |
| **[`examples/axum/06_axum_v2_meter_billing.rs`](06_axum_v2_meter_billing.rs)** | `cargo run --example 06_axum_v2_meter_billing` | `3006` | **Module 8**: V2 Core Events, Webhook Event Destinations, V2 Accounts, Billing Meters, shared `MeterStreamCollector` |

---

## Running the Full Axum Integration Test Suite (`tests/axum_examples_integration/`)

```bash
# Copy .env.example to .env and set your STRIPE_SECRET_KEY (or rely on .env.example fallback)
cp .env.example .env

# Run all Axum integration test scenarios across examples 01..06 and axum_stripe_server
cargo test --test axum_examples_integration_test
```
