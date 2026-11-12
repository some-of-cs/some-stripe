# Actix-Web Integration Test Folders (`tests/actix_examples/`)

Each Actix-Web example in [`examples/actix_web/`](../../examples/actix_web/) has a dedicated integration test folder under `tests/actix_examples/` that loads `STRIPE_SECRET_KEY` and `STRIPE_WEBHOOK_SECRET` directly from your root `.env` file.

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
  - `DotEnvConfig::load()` in [`tests/actix_examples/common/mod.rs`](common/mod.rs) loads your `STRIPE_SECRET_KEY` from `.env`, injects it into `StripeClient`, boots the real Actix-Web routes via `actix_web::test::init_service`, and verifies every HTTP request, `Authorization: Bearer <key>` header, payload, and statutory/cryptographic invariant against a local async wire server.
- **Live Stripe API Mode (`STRIPE_RUN_LIVE_TESTS=1`)**:
  - When `STRIPE_RUN_LIVE_TESTS=1` and a valid `sk_test_...` key is present in `.env`, the live `.env` probe scenarios also execute directly against `https://api.stripe.com`.

---

## 2. Per-Example Integration Test Folder Matrix

| Integration Test Folder | `Cargo.toml` Test Target | Example Covered | Scenarios Verified |
| :--- | :--- | :--- | :--- |
| [`tests/actix_examples/core_and_webhooks/`](core_and_webhooks/main.rs) | `actix_core_and_webhooks_test` | `examples/actix_web/actix_core_and_webhooks.rs` | `.env` key loading, `/health` `[REDACTED]` secret check, `Nullable<T>` (`Value`, `Null`, `Omit`) v1 form & v2 JSON encoding, valid v1 HMAC-SHA256 webhook, tampered signature rejection, missing header rejection, stale timestamp replay rejection, v2 thin event webhook |
| [`tests/actix_examples/payments_and_settlement/`](payments_and_settlement/main.rs) | `actix_payments_and_settlement_test` | `examples/actix_web/actix_payments_and_settlement.rs` | `PaymentIntent` create/retrieve/confirm (PSD2 3DS2 redirect detection)/capture/cancel, `.env` Bearer header wire check, `Refund` CRUD, `Dispute` evidence & close, `Balance` & paginated `BalanceTransactions`, optional live `.env` balance probe |
| [`tests/actix_examples/checkout_and_payment_links/`](checkout_and_payment_links/main.rs) | `actix_checkout_and_payment_links_test` | `examples/actix_web/actix_checkout_and_payment_links.rs` | `CheckoutSession` create/retrieve (`is_fulfillment_ready`)/expire/line-items, open-redirect domain allowlist rejection (`400 Bad Request`), `PaymentLink` CRUD & line-items, EU Omnibus Directive 2019/2161 valid & invalid promotion checks |
| [`tests/actix_examples/customers_and_subscriptions/`](customers_and_subscriptions/main.rs) | `actix_customers_and_subscriptions_test` | `examples/actix_web/actix_customers_and_subscriptions.rs` | `Customer` CRUD, GDPR Art. 17 erasure, search, cash balance, `Subscription` lifecycle (pause/resume collection, proration preview, search), `SubscriptionItem` CRUD & usage records, `SubscriptionSchedule` lifecycle |
| [`tests/actix_examples/invoicing_and_billing_portal/`](invoicing_and_billing_portal/main.rs) | `actix_invoicing_and_billing_portal_test` | `examples/actix_web/actix_invoicing_and_billing_portal.rs` | `Invoice` lifecycle (finalize, pay, void, mark_uncollectible, send, upcoming `id: None`, line item mutations, zero-copy PDF download), `CreditNote` CRUD & balance deduction invariant validation, `Quote` lifecycle & PDF, `BillingPortal` sessions & configurations |
| [`tests/actix_examples/connect_marketplace/`](connect_marketplace/main.rs) | `actix_connect_marketplace_test` | `examples/actix_web/actix_connect_marketplace.rs` | Connected `Account` CRUD & fraud rejection, `AccountLink` & `AccountSession` creation, KYC `Person` CRUD, `Transfer` & reversals, `ApplicationFee` & fee refunds, `Payout` CRUD/cancel/reverse |
| [`tests/actix_examples/specialized_products/`](specialized_products/main.rs) | `actix_specialized_products_test` | `examples/actix_web/actix_specialized_products.rs` | `Tax` calculations/transactions/registrations/settings, `Identity` verification sessions (including GDPR Art. 17 `redact`) & reports, `Issuing` cardholders/cards/authorizations (`approve`/`decline`)/transactions, `Treasury` financial accounts/transfers/payments/credits, `Terminal` locations/readers/connection tokens |
| [`tests/actix_examples/v2_next_gen_services/`](v2_next_gen_services/main.rs) | `actix_v2_next_gen_services_test` | `examples/actix_web/actix_v2_next_gen_services.rs` | V2 Core `events`, `ThinEventHydrator`, `event_destinations` (CRUD, enable, disable, ping), V2 `accounts` & `[REDACTED]` access tokens, V2 Billing `meters`, `[REDACTED]` `meter_event_session`, synchronous `meter_events`, `meter_event_stream`, `MeterIngestionEngine` enqueue/flush, `meter_event_adjustments` |
| [`tests/actix_examples/full_sdk_server/`](full_sdk_server/main.rs) | `actix_full_sdk_server_test` | `examples/actix_web/actix_full_sdk_server.rs` | `/api/catalog` discovery and end-to-end routing across all 8 mounted `/api/v1/*` and `/api/v2-suite/*` service scopes |

---

## 3. Running the Integration Tests

```bash
# Run all 9 Actix-Web integration test suites
cargo test --test 'actix_*_test'

# Or run an individual example's integration test folder:
cargo test --test actix_core_and_webhooks_test
cargo test --test actix_payments_and_settlement_test
cargo test --test actix_checkout_and_payment_links_test
cargo test --test actix_customers_and_subscriptions_test
cargo test --test actix_invoicing_and_billing_portal_test
cargo test --test actix_connect_marketplace_test
cargo test --test actix_specialized_products_test
cargo test --test actix_v2_next_gen_services_test
cargo test --test actix_full_sdk_server_test
```
