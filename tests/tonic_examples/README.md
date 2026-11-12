# Tonic gRPC Integration Test Folders (`tests/tonic_examples/`)

Each Tonic gRPC example in [`examples/`](../../examples/) has a dedicated integration test folder under `tests/tonic_examples/` that resolves `"$STRIPE_SECRET_KEY"` and `"$STRIPE_WEBHOOK_SECRET"` directly from your root `.env` file via `SecretKey::new`.

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
  - `DotEnvConfig::load()` in [`tests/tonic_examples/common/mod.rs`](common/mod.rs) resolves `"$STRIPE_SECRET_KEY"` and `"$STRIPE_WEBHOOK_SECRET"` from `.env`, injects them into `StripeClient` and each Tonic gRPC service implementation (`*GrpcServer`), and verifies every Tonic `Request<T>` / `Response<T>`, `Authorization: Bearer <key>` wire header, JSON/Protobuf payload, and statutory/cryptographic invariant against a local async wire server (`MockStripeServer`).
- **Live Stripe API Mode (`STRIPE_RUN_LIVE_TESTS=1`)**:
  - When `STRIPE_RUN_LIVE_TESTS=1` and a valid `sk_test_...` key is present in `.env`, the live `.env` gRPC balance probe scenarios also execute directly against `https://api.stripe.com`.

## 2. Per-Example Integration Test Folder Matrix

| Integration Test Folder | `Cargo.toml` Test Target | Example Covered | Scenarios Verified |
| :--- | :--- | :--- | :--- |
| [`tests/tonic_examples/core_and_webhooks/`](core_and_webhooks/main.rs) | `tonic_core_and_webhooks_test` | `tonic_core_and_webhooks` | Native `"$STRIPE_SECRET_KEY"` & `"$STRIPE_WEBHOOK_SECRET"` loading, `get_health` `[REDACTED]` secret check, `preview_dual_protocol` 3-state `Nullable<T>` (`Value`, `Null`, `Omit`) v1 form & v2 JSON encoding, valid v1 HMAC-SHA256 `verify_v1_webhook` (metadata & field), tampered signature (`Code::Unauthenticated`), missing header (`Code::Unauthenticated`), stale timestamp replay guard (`Code::Unauthenticated`), >1 MiB payload ceiling guard, `verify_v2_thin_webhook` |
| [`tests/tonic_examples/payments_and_settlement/`](payments_and_settlement/main.rs) | `tonic_payments_and_settlement_test` | `tonic_payments_and_settlement` | `PaymentIntent` create/retrieve/confirm (PSD2 3DS2 `requires_3ds2_redirect` detection)/capture/cancel, `.env` Bearer header wire check, `Refund` create/retrieve/update/cancel, `Dispute` retrieve/update/close, `Balance` & paginated `BalanceTransactions`, invalid JSON & upstream error status mapping (`Code::InvalidArgument`, `Code::Unavailable`), optional live `.env` balance probe |
| [`tests/tonic_examples/checkout_and_payment_links/`](checkout_and_payment_links/main.rs) | `tonic_checkout_and_payment_links_test` | `tonic_checkout_and_payment_links` | `CheckoutSession` create/retrieve (`is_fulfillment_ready`)/expire/line-items, open-redirect domain allowlist rejection (`Code::InvalidArgument`), `PaymentLink` create/retrieve/update/line-items, EU Omnibus Directive 2019/2161 valid & invalid promotion checks (`validate_omnibus_promotion`) |
| [`tests/tonic_examples/customers_and_subscriptions/`](customers_and_subscriptions/main.rs) | `tonic_customers_and_subscriptions_test` | `tonic_customers_and_subscriptions` | `Customer` CRUD, GDPR Art. 17 erasure (`delete_customer`), list/search, cash balance, `Subscription` lifecycle (pause/resume collection, proration preview, search, list), `SubscriptionItem` CRUD & metered `create_usage_record`, `SubscriptionSchedule` lifecycle, validation error mapping (`Code::InvalidArgument`) |
| [`tests/tonic_examples/invoicing_and_billing_portal/`](invoicing_and_billing_portal/main.rs) | `tonic_invoicing_and_billing_portal_test` | `tonic_invoicing_and_billing_portal` | `Invoice` lifecycle (create, retrieve, update, delete, finalize, pay, void, mark_uncollectible, send, list, upcoming `id: None`, line item mutations, binary PDF download `BinaryPdfResponse`), `CreditNote` CRUD & `validate_credit_deduction` invariant checks, `Quote` lifecycle & binary PDF download, `BillingPortal` sessions & configurations |
| [`tests/tonic_examples/connect_marketplace/`](connect_marketplace/main.rs) | `tonic_connect_marketplace_test` | `tonic_connect_marketplace` | Connected `Account` CRUD & fraud rejection (`reject_account`), `AccountLink` & `AccountSession` creation, KYC `Person` CRUD, destination `Transfer` & reversals, `ApplicationFee` & fee refunds, platform `Payout` CRUD/cancel/reverse |
| [`tests/tonic_examples/specialized_products/`](specialized_products/main.rs) | `tonic_specialized_products_test` | `tonic_specialized_products` | `Tax` calculations/transactions/reversals/registrations/settings, `Identity` verification sessions (including GDPR Art. 17 `redact_verification_session`) & reports, `Issuing` cardholders/cards/authorizations (`approve`/`decline`)/transactions, `Treasury` financial accounts/outbound transfers/outbound payments/received credits, `Terminal` locations/readers/connection tokens |
| [`tests/tonic_examples/v2_next_gen_services/`](v2_next_gen_services/main.rs) | `tonic_v2_next_gen_services_test` | `tonic_v2_next_gen_services` | V2 Core `events`, `hydrate_v2_thin_event` freshness guard, `event_destinations` (CRUD, enable, disable, ping), V2 `accounts` & `[REDACTED]` `create_v2_account_token`, V2 Billing `meters`, `[REDACTED]` `create_meter_event_session`, synchronous `create_synchronous_meter_event`, `create_meter_event_stream_batch`, `MeterIngestionEngine` `enqueue_meter_event` & `flush_ingestion_engine`, `meter_event_adjustments` |
| [`tests/tonic_examples/full_sdk_server/`](full_sdk_server/main.rs) | `tonic_full_sdk_server_test` | `tonic_full_sdk_server` | `UnifiedStripeGrpcHub` `get_service_catalog` discovery and end-to-end gRPC dispatch across all 8 mounted service servers (`core_webhooks`, `payments`, `checkout`, `customers`, `invoicing`, `connect`, `specialized`, and `v2`) |

## 3. Running the Integration Tests

```bash
# Run all Tonic gRPC integration test targets
cargo test --test 'tonic_*'

# Or run an individual example's integration test target:
cargo test --test tonic_core_and_webhooks_test
cargo test --test tonic_payments_and_settlement_test
cargo test --test tonic_checkout_and_payment_links_test
cargo test --test tonic_customers_and_subscriptions_test
cargo test --test tonic_invoicing_and_billing_portal_test
cargo test --test tonic_connect_marketplace_test
cargo test --test tonic_specialized_products_test
cargo test --test tonic_v2_next_gen_services_test
cargo test --test tonic_full_sdk_server_test
cargo test --test tonic_examples_integration_test
```
