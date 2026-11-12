# `some-stripe` + Actix-Web Examples Suite

This directory provides a complete, production-ready suite of **Actix-Web (`actix-web 4`)** examples covering **all 8 architectural modules and 34+ SDK resource namespaces (API v1 & v2)** in `some-stripe`.

---

## Architectural Highlights for Actix-Web

1. **Zero-Lock Shared Client State (`web::Data<StripeClient>`)**:
   - `StripeClient` wraps `Arc<ClientInner>` internally. Registering `web::Data<StripeClient>` in `HttpServer::new` shares a single HTTP connection pool across all Actix worker threads with zero lock contention.
2. **Zero-Clone Borrowing Discipline (Gate G-Borrow)**:
   - Every `some-stripe` resource handle (`stripe.payment_intents()`, `stripe.customers()`, `stripe.v2().core()`, etc.) is a zero-cost `Copy` view over `&'a StripeClient`.
   - Handlers pass borrowed references (`&params`, `id.as_str()`, `&payload`) directly to SDK methods without heap-cloning domain structs.
3. **Constant-Time Webhook Verification & 1 MB Payload Guard (Gate G2)**:
   - Webhook handlers extract raw `web::Bytes` and the `Stripe-Signature` header to verify HMAC-SHA256 signatures in constant time (`subtle::ConstantTimeEq`) with a 300-second replay window (`DEFAULT_TOLERANCE_SECONDS = 300`) and a strict 1 MB payload cap (`MAX_WEBHOOK_PAYLOAD_BYTES`).
4. **Centralized `actix_web::ResponseError` Integration**:
   - Each example maps `StripeError`, `StripeWebhookError`, and `HydrationError` cleanly into structured JSON HTTP responses while keeping handler cyclomatic complexity low ($CC \le 4$).

---

## Example Matrix & Associated Service Pull Requests

| Example Target | Port | PR | Modules & SDK Services Covered |
| :--- | :---: | :---: | :--- |
| **`actix_core_and_webhooks`** | `8081` | #141 | **Modules 1 & 2**: `StripeClient`, `StripeConfig`, `CustomErrorWorkflowBuilder`, `RequestOptions`, 3-state `Nullable<T>` (`serialize_v1_form`, `serialize_v2_json`), `stripe.webhooks()`, `StripeWebhookWrapper`, `EventObject` union, `is_thin_event` |
| **`actix_payments_and_settlement`** | `8082` | #147 | **Module 3**: `stripe.payment_intents()` (`create`, `retrieve`, `confirm`, `capture`, `cancel`), `stripe.refunds()`, `stripe.disputes()`, `stripe.balance()`, `stripe.balance_transactions()` (`Paginator<BalanceTransaction>`) |
| **`actix_checkout_and_payment_links`** | `8083` | #140 | **Module 4**: `stripe.checkout().sessions()` (`create`, `retrieve`, `expire`, `list_line_items_default`), `stripe.payment_links()`, `is_fulfillment_ready`, `validate_redirect_url`, `validate_omnibus_price_transparency`, `validate_tax_behavior_transparency` |
| **`actix_customers_and_subscriptions`** | `8084` | #142 | **Module 5 (Part A)**: `stripe.customers()` (CRUD, `search`, `cash_balance`, GDPR Art. 17 `delete`), `stripe.subscriptions()` (CRUD, `pause_collection`, `resume_collection`, `preview_proration`, `search`), `stripe.subscriptions().items()` (`create_usage_record`), `stripe.subscription_schedules()` |
| **`actix_invoicing_and_billing_portal`** | `8085` | #144 | **Module 5 (Part B)**: `stripe.invoices()` (CRUD, `finalize_invoice`, `pay`, `void_invoice`, `mark_uncollectible`, `send_invoice`, `upcoming`, `add_lines`, `update_lines`, `remove_lines`, zero-copy `pdf`), `stripe.credit_notes()`, `stripe.quotes()`, `stripe.billing_portal()` |
| **`actix_connect_marketplace`** | `8086` | #146 | **Module 6**: `stripe.accounts()` (CRUD, `reject`), `stripe.account_links()`, `stripe.account_sessions()`, `stripe.persons()` (KYC/CDD), `stripe.transfers()` & reversals, `stripe.application_fees()` & refunds, `stripe.payouts()` |
| **`actix_specialized_products`** | `8087` | #143 | **Module 7**: `stripe.tax()` (`calculations`, `transactions`, `registrations`, `settings`), `stripe.identity()` (`verification_sessions` with `redact`, `verification_reports`), `stripe.issuing()` (`cardholders`, `cards`, `authorizations`, `transactions`), `stripe.treasury()` (`financial_accounts`, `outbound_transfers`, `outbound_payments`, `received_credits`), `stripe.terminal()` (`locations`, `readers`, `connection_tokens`) |
| **`actix_v2_next_gen_services`** | `8088` | #145 | **Module 8**: `stripe.v2().core()` (`events`, `hydrator`, `event_destinations`, `accounts` & tokens), `stripe.v2().billing()` (`meters`, `meter_event_session`, `meter_event_stream`, `meter_event_adjustments`), `stripe.meter_events()`, `MeterIngestionEngine` |
| **`actix_full_sdk_server`** | `8080` | Final PR | **Unified Reference Server**: Mounts all 8 suites under `/api/v1/*` and `/api/v2-suite/*` plus `/api/catalog` discovery |

---

## Quick Start & Running the Examples

### 1. Run an Individual Service Example
```bash
export STRIPE_SECRET_KEY="sk_test_your_key"
export STRIPE_WEBHOOK_SECRET="whsec_your_secret"

# Example 1: Core & Webhooks (http://127.0.0.1:8081)
cargo run --example actix_core_and_webhooks

# Example 2: Payments, Refunds, Disputes & Balance (http://127.0.0.1:8082)
cargo run --example actix_payments_and_settlement

# Example 3: Hosted Checkout & Payment Links (http://127.0.0.1:8083)
cargo run --example actix_checkout_and_payment_links

# Example 4: Customers, Subscriptions, Items & Schedules (http://127.0.0.1:8084)
cargo run --example actix_customers_and_subscriptions

# Example 5: Invoices, Credit Notes, Quotes & Billing Portal (http://127.0.0.1:8085)
cargo run --example actix_invoicing_and_billing_portal

# Example 6: Connect & Multi-Party Marketplaces (http://127.0.0.1:8086)
cargo run --example actix_connect_marketplace

# Example 7: Tax, Identity, Issuing, Treasury & Terminal (http://127.0.0.1:8087)
cargo run --example actix_specialized_products

# Example 8: Stripe API v2 Core, Hydration & Billing Meters (http://127.0.0.1:8088)
cargo run --example actix_v2_next_gen_services
```

### 2. Run the Unified All-Services Server
```bash
STRIPE_SECRET_KEY="sk_test_your_key" STRIPE_WEBHOOK_SECRET="whsec_your_secret" \
  cargo run --example actix_full_sdk_server
```

Then inspect the mounted service catalog:
```bash
curl -s http://127.0.0.1:8080/api/catalog | jq .
```


---

# `some-stripe` + Axum (`axum 0.8`) Examples Suite

In addition to the Actix-Web suite above, this directory provides a complete, production-ready **Axum (`axum 0.8`)** integration suite covering **all 8 Stripe architectural modules (API v1 & v2)**, including both a **unified modular server (`examples/axum_stripe_server/`)** and **6 standalone microservice examples (`01_axum_webhooks`..`06_axum_v2_meter_billing`)**.

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

## Axum Example Matrix

| Example Target | Command | Port | Modules & SDK Services Covered |
| :--- | :--- | :---: | :--- |
| **`axum_stripe_server`** | `cargo run --example axum_stripe_server` | `3000` | **All 8 Modules Unified**: Mounts `/webhooks/stripe`, `/payments/*`, `/checkout/*`, `/billing/*`, `/connect/*`, `/specialized/*`, and `/v2/*` |
| **`01_axum_webhooks`** | `cargo run --example 01_axum_webhooks` | `3001` | **Module 2**: Constant-time HMAC-SHA256 verification, V2 Thin Event routing & hydration, `EventObject` union |
| **`02_axum_checkout_and_payments`** | `cargo run --example 02_axum_checkout_and_payments` | `3002` | **Modules 3 & 4**: Hosted Checkout Sessions, Payment Links (EU Omnibus `price_data` binding), PaymentIntents, Refunds |
| **`03_axum_subscriptions_and_invoicing`** | `cargo run --example 03_axum_subscriptions_and_invoicing` | `3003` | **Module 5**: Customer vaulting (GDPR Art. 15/17), 14-day trial Subscriptions, Invoices, Customer Portal |
| **`04_axum_connect_marketplace`** | `cargo run --example 04_axum_connect_marketplace` | `3004` | **Module 6**: Express Connected Accounts, `AccountOnboarding` links, KYC Persons (`Stripe-Account`), Transfers, Payouts |
| **`05_axum_specialized_products`** | `cargo run --example 05_axum_specialized_products` | `3005` | **Module 7**: Stripe Tax (calculate & commit), Identity KYC sessions, Issuing Cardholders & Cards, Treasury, Terminal |
| **`06_axum_v2_meter_billing`** | `cargo run --example 06_axum_v2_meter_billing` | `3006` | **Module 8**: V2 Core Events, Webhook Event Destinations, V2 Accounts, Billing Meters, shared `MeterStreamCollector` |

## Running the Full Axum Integration Test Suite (`tests/axum_examples_integration/`)

```bash
# Copy .env.example to .env and set your STRIPE_SECRET_KEY (or rely on .env.example fallback)
cp .env.example .env

# Run all 23 Axum integration test scenarios across examples 01..06 and axum_stripe_server
cargo test --test axum_examples_integration_test
```

---

## Tonic gRPC Framework Examples Suite (`examples/tonic_*.rs`)

In addition to the Actix-Web REST suite above, `some-stripe` provides a complete **Tonic gRPC (`tonic` + `prost`)** example suite covering all 8 SDK modules plus a unified multi-domain gRPC hub and a canonical Protocol Buffers v3 schema ([`examples/proto/some_stripe.proto`](./proto/some_stripe.proto)).

- **Full Tonic gRPC Documentation**: See [`examples/TONIC_GRPC_README.md`](./TONIC_GRPC_README.md)
- **Protocol Buffers v3 Schema**: [`examples/proto/some_stripe.proto`](./proto/some_stripe.proto)
- **Tonic gRPC Example Targets**:
  - `cargo run --example tonic_core_and_webhooks`
  - `cargo run --example tonic_payments_and_settlement`
  - `cargo run --example tonic_checkout_and_payment_links`
  - `cargo run --example tonic_customers_and_subscriptions`
  - `cargo run --example tonic_invoicing_and_billing_portal`
  - `cargo run --example tonic_connect_marketplace`
  - `cargo run --example tonic_specialized_products`
  - `cargo run --example tonic_v2_next_gen_services`
  - `cargo run --example tonic_full_sdk_server`
- **Tonic gRPC Integration Tests**:
  - `cargo test --test tonic_examples_integration_test`
