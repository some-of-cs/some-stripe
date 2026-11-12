# Rocket.rs Integration Test Folders (`tests/rocket_examples/`)

Each Rocket.rs (`rocket 0.5`) service module in [`examples/rocket_stripe/`](../../examples/rocket_stripe/) has a dedicated integration test folder under `tests/rocket_examples/` (as well as crate-local tests in `examples/rocket_stripe/tests/`) that resolves `"$STRIPE_SECRET_KEY"` and `"$STRIPE_WEBHOOK_SECRET"` directly from your root `.env` file via `SecretKey::new`.

---

## 1. Configuring `.env`

Create a `.env` file in the repository root (already protected by `.gitignore` so secrets are never committed):

```bash
cp .env.example .env
```

Edit `.env` and add your Stripe test key:

```dotenv
STRIPE_SECRET_KEY=sk_test_your_stripe_secret_key
STRIPE_WEBHOOK_SECRET=whsec_test_your_webhook_secret
STRIPE_RUN_LIVE_TESTS=0
```

- **Default Deterministic Wire + `.env` Mode (`STRIPE_RUN_LIVE_TESTS=0`)**:
  - `setup_mock_rocket_client` in [`tests/rocket_examples/common/mod.rs`](common/mod.rs) resolves `"$STRIPE_SECRET_KEY"` and `"$STRIPE_WEBHOOK_SECRET"` via `SecretKey::new`, mounts all 37 Rocket routes via `rocket::local::asynchronous::Client::untracked`, and verifies every HTTP method, path, `Idempotency-Key` header, and RFC 1738 / JSON request body against an ephemeral local async TCP server.
- **Live Stripe API Mode (when `STRIPE_SECRET_KEY` is configured in `.env`)**:
  - `setup_live_env_rocket_client` detects when a real `sk_test_...` key is configured in `.env` and executes live end-to-end sandbox probes against `https://api.stripe.com`.

---

## 2. Per-Module Integration Test Folder Matrix

| Integration Test Folder | `Cargo.toml` Test Target | Route Module Covered | Scenarios Verified |
| :--- | :--- | :--- | :--- |
| [`tests/rocket_examples/checkout_and_payment_links/`](checkout_and_payment_links/main.rs) | `rocket_checkout_and_payment_links_test` | `examples/rocket_stripe/src/routes/checkout.rs` | Service index discovery (`GET /`), `404`/`400`/`422` JSON catchers, `CheckoutSession` create/retrieve/expire, EU Omnibus Directive 2019/2161 `PaymentLink` valid creation & violation rejection (`400 Bad Request`), optional live `.env` Checkout Session probe |
| [`tests/rocket_examples/payments_and_settlement/`](payments_and_settlement/main.rs) | `rocket_payments_and_settlement_test` | `examples/rocket_stripe/src/routes/payments.rs` | `PaymentIntent` full lifecycle (`create`, `retrieve`, `confirm`, `capture`, `cancel`), `Refund` create & retrieve, `Dispute` retrieve & close, `Balance` & `BalanceTransaction` ledger, optional live `.env` Balance & PaymentIntent probe |
| [`tests/rocket_examples/customers_and_billing/`](customers_and_billing/main.rs) | `rocket_customers_and_billing_test` | `examples/rocket_stripe/src/routes/billing.rs` | `Customer` vault (`create`, `retrieve`, GDPR Art. 17 `delete`), `Subscription` lifecycle (`create` with indexed-bracket items, `retrieve`, `cancel`), `Invoice` lifecycle (`create`, `retrieve`, `finalize`, `pay`), `BillingPortal` session creation, optional live `.env` Customer probe |
| [`tests/rocket_examples/connect_marketplace/`](connect_marketplace/main.rs) | `rocket_connect_marketplace_test` | `examples/rocket_stripe/src/routes/connect.rs` | Connected `Account` creation (`express`/`standard`/`custom`) & retrieval, `AccountLink` onboarding URL generation, ISO 4217 zero-amount `Transfer` rejection guard (`400 Bad Request`), valid `Transfer` creation & `TransferReversal`, `ApplicationFee` retrieval |
| [`tests/rocket_examples/specialized_products/`](specialized_products/main.rs) | `rocket_specialized_products_test` | `examples/rocket_stripe/src/routes/specialized.rs` | Stripe `Tax` settings retrieval, `Identity` verification session creation & retrieval, `Issuing` virtual card issuance, `Treasury` commercial financial account opening, `Terminal` POS connection token generation, optional live `.env` Terminal token probe |
| [`tests/rocket_examples/v2_next_gen_services/`](v2_next_gen_services/main.rs) | `rocket_v2_next_gen_services_test` | `examples/rocket_stripe/src/routes/v2_services.rs` | Stripe API v2 Core `events`, `event_destinations`, and `accounts` retrieval, `ThinEventHydrator` (`POST /api/v2/events/hydrate`), and V2 high-throughput usage `meter_events` ingestion (`POST /api/v2/billing/meter-events`) |
| [`tests/rocket_examples/webhooks_and_security/`](webhooks_and_security/main.rs) | `rocket_webhooks_and_security_test` | `examples/rocket_stripe/src/routes/webhooks.rs` | Missing/blank/forged `Stripe-Signature` rejection (`400 Bad Request`), replay protection (`> 300s` old) and future drift (`> 60s` ahead) rejection, V1 event routing (`checkout.session.completed` paid vs unpaid, `payment_intent.succeeded`, `invoice.paid`, `customer.subscription.deleted`), V2 thin event verification + hydration, and native `"$STRIPE_WEBHOOK_SECRET"` resolution |
| [`tests/rocket_examples/full_sdk_server/`](full_sdk_server/main.rs) | `rocket_full_sdk_server_test` | `examples/rocket_stripe/src/lib.rs` | Unified service catalog (`GET /`), `[REDACTED]` `SecretKey` debug formatting check, and cross-namespace end-to-end dispatch across Billing, Payments, V2 Meters, and Webhooks |

---

## 3. Running the Integration Tests

```bash
# Run the unified Rocket.rs integration test target from the root workspace:
cargo test --test rocket_examples_integration_test

# Run all 9 root Rocket.rs integration test targets:
cargo test --test 'rocket_*'

# Or run an individual Rocket.rs service integration test target:
cargo test --test rocket_checkout_and_payment_links_test
cargo test --test rocket_payments_and_settlement_test
cargo test --test rocket_customers_and_billing_test
cargo test --test rocket_connect_marketplace_test
cargo test --test rocket_specialized_products_test
cargo test --test rocket_v2_next_gen_services_test
cargo test --test rocket_webhooks_and_security_test
cargo test --test rocket_full_sdk_server_test

# Or run the crate-local integration test suite directly:
cargo test -p rocket-stripe-example
```
