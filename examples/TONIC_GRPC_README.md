# `some-stripe` — Tonic gRPC Framework Examples Suite

This directory contains a complete, production-grade **Tonic (`tonic 0.12` + `prost 0.13`) gRPC** example suite for the `some-stripe` Rust SDK. It mirrors the 8-module architecture of the Actix-Web suite while demonstrating idiomatic gRPC service design, Protocol Buffers v3 (`proto3`) message contracts, native `"$STRIPE_SECRET_KEY"` environment resolution, zero-clone borrow discipline, and structured `StripeError` $\to$ `tonic::Status` mapping.

---

## 1. Architectural Highlights

1. **Native `$VAR_NAME` Secret Resolution (Zero `.env` Boilerplate)**:
   Every Tonic gRPC server initializes the SDK directly with `"$STRIPE_SECRET_KEY"` and `"$STRIPE_WEBHOOK_SECRET"`:
   ```rust
   let client = StripeClient::new("$STRIPE_SECRET_KEY")?;
   let webhook_secret = SecretKey::new("$STRIPE_WEBHOOK_SECRET")?;
   ```
   The `some-stripe-core` engine automatically checks process environment variables and cascades through `.env.local`, `.env.dev`, `.env.development`, `.env.test`, `.env.prod`, `.env.production`, `.env`, and `.env.example`.

2. **Zero-`protoc` Compilation + Canonical `.proto` Schema**:
   - The full Protocol Buffers v3 contract is documented in [`examples/proto/some_stripe.proto`](./proto/some_stripe.proto) (`package some_stripe.v1;`) for polyglot gRPC clients (Go, TypeScript, Python, Java, `grpcurl`).
   - Inside each Rust example (`examples/tonic_*.rs`), request and response messages use `#[derive(Clone, PartialEq, ::prost::Message)]` alongside `#[tonic::async_trait]` service traits so `cargo check --examples` and `cargo run --example tonic_*` compile out of the box without requiring an external `protoc` compiler binary on the host machine.

3. **Strict Borrow Discipline (`Gate G-Borrow`)**:
   - All SDK parameter payloads (`PaymentIntentCreateParams`, `CheckoutSessionCreateParams`, `CustomerCreateParams`, `SubscriptionCreateParams`, `AccountCreateParams`, etc.) are passed by shared reference (`&params`) to `some-stripe` service methods.
   - `Arc<AppState>` shares the thread-safe `StripeClient` across concurrent Tokio tasks without deep-cloning credentials or HTTP connection pools.

4. **Security Hardening (`Gate G2`)**:
   - Webhook verification in `tonic_core_and_webhooks.rs` and `tonic_full_sdk_server.rs` enforces a **1 MiB payload cap**, constant-time **HMAC-SHA256** comparison (`subtle::ConstantTimeEq`), **300-second replay window** validation, and `ZeroizeOnDrop` secret key storage.

---

## 2. Error Mapping: `StripeError` & `WebhookError` $\to$ `tonic::Status`

Every Tonic gRPC example maps `some_stripe_core::error::StripeError` and `some_stripe_webhooks::WebhookError` deterministically to canonical gRPC status codes (`tonic::Code`):

| SDK Error Variant | gRPC `tonic::Code` | When Triggered |
| :--- | :--- | :--- |
| `StripeError::InvalidHeader(_)` / `InvalidInput(_)` | `Code::InvalidArgument` | Malformed request fields, empty required IDs, or invalid metadata keys |
| `StripeError::InvalidSecretKey(_)` | `Code::Unauthenticated` | Missing or invalid `"$STRIPE_SECRET_KEY"` / `"$STRIPE_WEBHOOK_SECRET"` |
| `StripeError::AuthenticationError(_)` | `Code::Unauthenticated` | HTTP 401 from Stripe API |
| `StripeError::PermissionError(_)` | `Code::PermissionDenied` | HTTP 403 restricted API key or Connect permission error |
| `StripeError::RateLimitError(_)` | `Code::ResourceExhausted` | HTTP 429 rate-limit exceeded |
| `StripeError::IdempotencyError(_)` | `Code::AlreadyExists` | Conflicting idempotency key parameters |
| `StripeError::ApiError { status: 404, .. }` | `Code::NotFound` | Resource ID (`pi_...`, `cus_...`, `sub_...`, `acct_...`) not found |
| `StripeError::NetworkError(_)` / `Timeout` | `Code::Unavailable` | Upstream connectivity or timeout failure |
| `WebhookError::PayloadTooLarge` | `Code::ResourceExhausted` | Webhook raw payload exceeds 1 MiB (`1_048_576` bytes) |
| `WebhookError::InvalidSignature` / `TimestampExpired` | `Code::Unauthenticated` | Constant-time HMAC mismatch or timestamp outside 300s tolerance |

---

## 3. Complete Tonic gRPC Examples Matrix

| Module | Example Binary | Default Port | `.proto` Service Name | Covered `some-stripe` SDK Services & RPCs |
| :--- | :--- | :--- | :--- | :--- |
| **Module 1** | [`tonic_core_and_webhooks.rs`](./tonic_core_and_webhooks.rs) | `50051` | `some_stripe.v1.CoreWebhooksService` | `StripeClient::new("$STRIPE_SECRET_KEY")`, `StripeConfig::builder()`, `SecretKey::new("$STRIPE_WEBHOOK_SECRET")`, `Webhook::construct_event` (`GetHealth`, `InspectConfig`, `VerifyWebhook`) |
| **Module 2** | [`tonic_payments_and_settlement.rs`](./tonic_payments_and_settlement.rs) | `50052` | `some_stripe.v1.PaymentsSettlementService` | `PaymentIntentsService`, `ChargesService`, `RefundsService`, `DisputesService`, `BalanceService`, `BalanceTransactionsService`, `PayoutsService` (`CreatePaymentIntent`, `ConfirmPaymentIntent`, `CapturePaymentIntent`, `RetrieveCharge`, `CreateRefund`, `ListDisputes`, `GetBalanceAndPayout`) |
| **Module 3** | [`tonic_checkout_and_payment_links.rs`](./tonic_checkout_and_payment_links.rs) | `50053` | `some_stripe.v1.CheckoutLinksService` | `CheckoutSessionsService`, `PaymentLinksService`, `PricesService`, `ProductsService`, `CouponsService`, `PromotionCodesService`, `TaxRatesService` (`CreateCatalogBundle`, `CreateCheckoutSession`, `ExpireCheckoutSession`, `CreatePaymentLink`) |
| **Module 4** | [`tonic_customers_and_subscriptions.rs`](./tonic_customers_and_subscriptions.rs) | `50054` | `some_stripe.v1.CustomersSubscriptionsService` | `CustomersService`, `PaymentMethodsService`, `SetupIntentsService`, `SubscriptionsService`, `SubscriptionItemsService`, `SubscriptionSchedulesService`, `UsageRecordsService` (`OnboardCustomer`, `CreateSubscription`, `PauseResumeSubscription`, `ReportMeteredUsage`) |
| **Module 5** | [`tonic_invoicing_and_billing_portal.rs`](./tonic_invoicing_and_billing_portal.rs) | `50055` | `some_stripe.v1.InvoicingPortalService` | `InvoicesService`, `InvoiceItemsService`, `CreditNotesService`, `QuotesService`, `BillingPortalSessionsService`, `BillingPortalConfigurationsService` (`CreateAndFinalizeInvoice`, `IssueCreditNote`, `AcceptQuote`, `CreateBillingPortalSession`) |
| **Module 6** | [`tonic_connect_marketplace.rs`](./tonic_connect_marketplace.rs) | `50056` | `some_stripe.v1.ConnectMarketplaceService` | `AccountsService`, `AccountLinksService`, `TransfersService`, `ApplicationFeesService`, `TopupsService` (`OnboardConnectedAccount`, `CreateDestinationTransfer`, `RefundApplicationFee`, `CreatePlatformTopup`) |
| **Module 7** | [`tonic_specialized_products.rs`](./tonic_specialized_products.rs) | `50057` | `some_stripe.v1.SpecializedProductsService` | `RadarEarlyFraudWarningsService`, `IssuingCardsService`, `TerminalConnectionTokensService`, `IdentityVerificationSessionsService`, `FinancialConnectionsSessionsService`, `TreasuryFinancialAccountsService`, `CryptoOnrampSessionsService`, `ClimateOrdersService`, `TaxCalculationsService`, `ForwardingRequestsService`, `EntitlementsActiveEntitlementsService` |
| **Module 8** | [`tonic_v2_next_gen_services.rs`](./tonic_v2_next_gen_services.rs) | `50058` | `some_stripe.v1.V2NextGenService` | `V2BillingMeterEventsService`, `V2BillingMeterEventSessionService`, `V2BillingMeterEventStreamService`, `V2CoreEventDestinationsService`, `V2CoreEventsService`, `MeterIngestionEngine` (`RecordMeterEvent`, `OpenMeterSession`, `CreateEventDestination`, `EnqueueBufferedMeter`) |
| **Unified Hub** | [`tonic_full_sdk_server.rs`](./tonic_full_sdk_server.rs) | `50050` | `some_stripe.v1.UnifiedStripeHubService` | All 68+ v1 & v2 `some-stripe` SDK services consolidated into a single multi-domain Tonic gRPC hub (`GetHubOverview`, `ExecutePaymentFlow`, `ExecuteSubscriptionFlow`, `ExecuteConnectFlow`, `VerifyWebhookEvent`) |

---

## 4. Quick Start & Running the Tonic gRPC Examples

### Step 1: Configure `.env` (or `.env.local` / `.env.dev`)

Create a `.env` file in the workspace root:

```dotenv
STRIPE_SECRET_KEY=sk_test_51YourStripeSecretKeyHere
STRIPE_WEBHOOK_SECRET=whsec_YourWebhookSigningSecretHere
```

Because `StripeClient::new("$STRIPE_SECRET_KEY")` resolves `$VAR_NAME` tokens automatically, you do **not** need `dotenvy` or `std::env::var` setup code.

### Step 2: Run Any Tonic gRPC Example Server

```bash
# Run Module 1: Core SDK, RequestOptions & Webhooks gRPC Server (Port 50051)
cargo run --example tonic_core_and_webhooks

# Run Module 2: Payments, Captures, Refunds, Disputes & Settlement gRPC Server (Port 50052)
cargo run --example tonic_payments_and_settlement

# Run Module 3: Checkout Sessions, Payment Links & Catalog gRPC Server (Port 50053)
cargo run --example tonic_checkout_and_payment_links

# Run Module 4: Customers, PaymentMethods, SetupIntents & Subscriptions gRPC Server (Port 50054)
cargo run --example tonic_customers_and_subscriptions

# Run Module 5: Invoicing, Credit Notes, Quotes & Billing Portal gRPC Server (Port 50055)
cargo run --example tonic_invoicing_and_billing_portal

# Run Module 6: Stripe Connect Marketplace & Multi-Party Transfers gRPC Server (Port 50056)
cargo run --example tonic_connect_marketplace

# Run Module 7: Specialized Stripe Products (Issuing, Identity, Tax, Treasury, Climate) (Port 50057)
cargo run --example tonic_specialized_products

# Run Module 8: Stripe v2 API & High-Throughput Meter Ingestion Engine (Port 50058)
cargo run --example tonic_v2_next_gen_services

# Run the Unified Multi-Domain Tonic gRPC Hub (Port 50050)
cargo run --example tonic_full_sdk_server
```

---

## 5. Invoking RPCs with `grpcurl`

Using [`examples/proto/some_stripe.proto`](./proto/some_stripe.proto), you can test any running server directly with `grpcurl`:

```bash
# 1. Check health & SDK configuration on Module 1 (Port 50051)
grpcurl -plaintext \
  -import-path examples/proto \
  -proto some_stripe.proto \
  -d '{"caller": "cli-smoke-test"}' \
  127.0.0.1:50051 some_stripe.v1.CoreWebhooksService/GetHealth

# 2. Create a PaymentIntent on Module 2 (Port 50052)
grpcurl -plaintext \
  -import-path examples/proto \
  -proto some_stripe.proto \
  -d '{
    "amount": 4900,
    "currency": "usd",
    "customer_id": "cus_test_123",
    "idempotency_key": "idem_pi_001"
  }' \
  127.0.0.1:50052 some_stripe.v1.PaymentsSettlementService/CreatePaymentIntent

# 3. Verify a Stripe Webhook Signature over gRPC on Module 1 (Port 50051)
grpcurl -plaintext \
  -import-path examples/proto \
  -proto some_stripe.proto \
  -d '{
    "raw_payload": "{\"id\":\"evt_123\",\"object\":\"event\",\"type\":\"payment_intent.succeeded\"}",
    "stripe_signature": "t=1710000000,v1=..."
  }' \
  127.0.0.1:50051 some_stripe.v1.CoreWebhooksService/VerifyWebhook
```

---

## 6. Running the Tonic gRPC Integration Test Suite

A dedicated integration test suite in [`tests/tonic_examples/main.rs`](../tests/tonic_examples/main.rs) tests all 9 Tonic gRPC service handlers end-to-end against both a deterministic local `MockStripeServer` and live `.env` `"$STRIPE_SECRET_KEY"` credentials:

```bash
cargo test --test tonic_examples_integration_test
```
