# Axum Examples Full Integration Test Suite

This directory (`tests/axum_examples_integration/`) contains the **full integration test suite** covering all 7 Axum example applications (`01_axum_webhooks` through `06_axum_v2_meter_billing` and the unified `axum_stripe_server`).

---

## Dual-Mode Execution (`.env` Live Stripe API + Deterministic Wire-Mock)

1. **Deterministic Wire-Mock Mode (Always Runs)**:
   - Every test spawns an isolated local TCP HTTP server, mounts the exact Axum `Router` from the corresponding example, and exercises all routes via `tower::ServiceExt::oneshot`.
   - Validates request serialization, HTTP headers (`Stripe-Signature`, `Stripe-Account`, `Idempotency-Key`), statutory compliance rules (EU Omnibus Directive 2019/2161, GDPR Art. 15/17), V2 thin event hydration, and RFC 7807 error status mapping.

2. **Live `.env` Stripe API Mode (Automatic when `STRIPE_SECRET_KEY` is configured)**:
   - Create a `.env` file in the repository root (or copy `.env.example`):
     ```bash
     cp .env.example .env
     ```
   - Set your Stripe test secret key in `.env`:
     ```env
     STRIPE_SECRET_KEY=sk_test_51...
     STRIPE_WEBHOOK_SECRET=whsec_...
     ```
   - `AxumDotEnvConfig::load()` uses `SecretKey::new("$STRIPE_SECRET_KEY")` and `SecretKey::new("$STRIPE_WEBHOOK_SECRET")` (PR #150) to automatically resolve secrets from process env, `.env.local`, `.env.dev`, `.env`, or `.env.example` and executes live E2E assertions against `https://api.stripe.com` when a real `sk_test_...` key is detected.

---

## Test Suite Layout

| File | Target Example | Scenarios Verified |
| :--- | :--- | :--- |
| `common.rs` | Shared Harness | Native `$VAR_NAME` `.env*` resolver (`AxumDotEnvConfig::load`), live/mock `StripeClient` builders, raw HTTP wire parser, Axum `oneshot` helpers |
| `ex01_webhooks_test.rs` | `examples/01_axum_webhooks.rs` | V1 HMAC-SHA256 verification, V2 Thin Event routing (`construct_v2_thin_event`) & hydration, missing header, forged signature, stale timestamp (>300s) replay rejection |
| `ex02_checkout_and_payments_test.rs` | `examples/02_axum_checkout_and_payments.rs` | Hosted Checkout Sessions, Payment Links with EU Omnibus `price_data` binding & deceptive discount rejection (`400`), PaymentIntents lifecycle, Refunds, and live `.env` PaymentIntent test |
| `ex03_subscriptions_and_invoicing_test.rs` | `examples/03_axum_subscriptions_and_invoicing.rs` | Customer vaulting, GDPR Art. 15 portability & Art. 17 erasure, 14-day trial Subscriptions, Invoices, Customer Portal sessions, and live `.env` Customer roundtrip |
| `ex04_connect_marketplace_test.rs` | `examples/04_axum_connect_marketplace.rs` | Express Connected Accounts, `AccountOnboarding` links, KYC Persons with `Stripe-Account` multi-tenancy header, split Transfers, and Payouts |
| `ex05_specialized_products_test.rs` | `examples/05_axum_specialized_products.rs` | Stripe Tax calculate/commit, Identity KYC sessions, Issuing Cardholders (with `individual` first/last name) & active virtual Cards, Treasury FinancialAccounts, and Terminal ConnectionTokens |
| `ex06_v2_meter_billing_test.rs` | `examples/06_axum_v2_meter_billing.rs` | V2 Core Events, Webhook Event Destinations, V2 Accounts, V2 Billing Meters (`event_payload_key: "value"`), event streaming, and shared `MeterStreamCollector` telemetry |
| `ex07_unified_server_test.rs` | `examples/axum_stripe_server/` | Unified server V1/V2 webhooks, `subscription` mode Checkout validation, Credit Note `CreditNoteReason` validation, Balance/BalanceTransactions pagination, Treasury transfers, V2 meter adjustments, and RFC 7807 `401`/`404` status mapping |

---

## Running the Tests

```bash
# Run the full Axum examples integration test suite
cargo test --test axum_examples_integration_test

# Run with live output logging
cargo test --test axum_examples_integration_test -- --nocapture
```
