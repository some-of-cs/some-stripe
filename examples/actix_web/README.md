# `some-stripe` + Actix-Web (`actix-web 4`) Examples Suite (`examples/actix_web/`)

This directory provides a complete, production-ready suite of **Actix-Web (`actix-web 4`)** examples covering **all 8 architectural modules and 34+ SDK resource namespaces (API v1 & v2)** in `some-stripe`.

---

## Architectural Highlights for Actix-Web

1. **Zero-Lock Shared Client State (`web::Data<StripeClient>`)**:
   - `StripeClient` wraps `Arc<ClientInner>` internally. Registering `web::Data<StripeClient>` in `HttpServer::new` shares a single HTTP connection pool across all Actix worker threads with zero lock contention.
2. **Native `$VAR_NAME` Secret Resolution (PR #150)**:
   - Every example initializes `StripeClient::new("$STRIPE_SECRET_KEY")` and `SecretKey::new("$STRIPE_WEBHOOK_SECRET")`, resolving secrets automatically from process environment variables or `.env.local`, `.env.dev`, `.env`, `.env.example`, etc.
3. **Zero-Clone Borrowing Discipline (`Gate G-Borrow`)**:
   - Every `some-stripe` resource handle (`stripe.payment_intents()`, `stripe.customers()`, `stripe.v2().core()`, etc.) is a zero-cost `Copy` view over `&'a StripeClient`.
   - Handlers pass borrowed references (`&params`, `id.as_str()`, `&payload`) directly to SDK methods without heap-cloning domain structs.
4. **Constant-Time Webhook Verification & 1 MB Payload Guard (`Gate G2`)**:
   - Webhook handlers extract raw `web::Bytes` and the `Stripe-Signature` header to verify HMAC-SHA256 signatures in constant time (`subtle::ConstantTimeEq`) with a 300-second replay window (`DEFAULT_TOLERANCE_SECONDS = 300`) and a strict 1 MB payload cap (`MAX_WEBHOOK_PAYLOAD_BYTES`).
5. **Centralized `actix_web::ResponseError` Integration**:
   - Each example maps `StripeError`, `StripeWebhookError`, and `HydrationError` cleanly into structured JSON HTTP responses while keeping handler cyclomatic complexity low ($CC \le 4$).

---

## Actix-Web Example Matrix

| Example Source Path | Command | Port | PR | Modules & SDK Services Covered |
| :--- | :--- | :---: | :---: | :--- |
| **[`examples/actix_web/actix_core_and_webhooks.rs`](actix_core_and_webhooks.rs)** | `cargo run --example actix_core_and_webhooks` | `8081` | #141 | **Modules 1 & 2**: `StripeClient`, `StripeConfig`, `CustomErrorWorkflowBuilder`, `RequestOptions`, 3-state `Nullable<T>`, `stripe.webhooks()`, `StripeWebhookWrapper`, `EventObject` union, `is_thin_event` |
| **[`examples/actix_web/actix_payments_and_settlement.rs`](actix_payments_and_settlement.rs)** | `cargo run --example actix_payments_and_settlement` | `8082` | #147 | **Module 3**: `stripe.payment_intents()` (`create`, `retrieve`, `confirm`, `capture`, `cancel`), `stripe.refunds()`, `stripe.disputes()`, `stripe.balance()`, `stripe.balance_transactions()` |
| **[`examples/actix_web/actix_checkout_and_payment_links.rs`](actix_checkout_and_payment_links.rs)** | `cargo run --example actix_checkout_and_payment_links` | `8083` | #140 | **Module 4**: `stripe.checkout().sessions()`, `stripe.payment_links()`, `is_fulfillment_ready`, `validate_redirect_url`, `validate_omnibus_price_transparency`, `validate_tax_behavior_transparency` |
| **[`examples/actix_web/actix_customers_and_subscriptions.rs`](actix_customers_and_subscriptions.rs)** | `cargo run --example actix_customers_and_subscriptions` | `8084` | #142 | **Module 5 (Part A)**: `stripe.customers()` (CRUD, `search`, `cash_balance`, GDPR Art. 17 `delete`), `stripe.subscriptions()`, `stripe.subscriptions().items()`, `stripe.subscription_schedules()` |
| **[`examples/actix_web/actix_invoicing_and_billing_portal.rs`](actix_invoicing_and_billing_portal.rs)** | `cargo run --example actix_invoicing_and_billing_portal` | `8085` | #144 | **Module 5 (Part B)**: `stripe.invoices()` (CRUD, `finalize_invoice`, `pay`, `void_invoice`, `mark_uncollectible`, `send_invoice`, `upcoming`, zero-copy `pdf`), `stripe.credit_notes()`, `stripe.quotes()`, `stripe.billing_portal()` |
| **[`examples/actix_web/actix_connect_marketplace.rs`](actix_connect_marketplace.rs)** | `cargo run --example actix_connect_marketplace` | `8086` | #146 | **Module 6**: `stripe.accounts()`, `stripe.account_links()`, `stripe.account_sessions()`, `stripe.persons()` (KYC/CDD), `stripe.transfers()`, `stripe.application_fees()`, `stripe.payouts()` |
| **[`examples/actix_web/actix_specialized_products.rs`](actix_specialized_products.rs)** | `cargo run --example actix_specialized_products` | `8087` | #143 | **Module 7**: `stripe.tax()`, `stripe.identity()`, `stripe.issuing()`, `stripe.treasury()`, `stripe.terminal()` |
| **[`examples/actix_web/actix_v2_next_gen_services.rs`](actix_v2_next_gen_services.rs)** | `cargo run --example actix_v2_next_gen_services` | `8088` | #145 | **Module 8**: `stripe.v2().core()` (`events`, `hydrator`, `event_destinations`, `accounts`), `stripe.v2().billing()` (`meters`, `meter_event_session`, `meter_event_stream`, `meter_event_adjustments`), `MeterIngestionEngine` |
| **[`examples/actix_web/actix_full_sdk_server.rs`](actix_full_sdk_server.rs)** | `cargo run --example actix_full_sdk_server` | `8080` | #148 | **Unified Reference Server**: Mounts all 8 suites under `/api/v1/*` and `/api/v2-suite/*` plus `/api/catalog` discovery |

---

## Running the Actix-Web Integration Test Suites (`tests/actix_examples/`)

```bash
# Run all 9 Actix-Web integration test targets
cargo test --test 'actix_*_test'
```
