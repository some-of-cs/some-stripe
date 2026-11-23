# `some-stripe` — AI Agent Usage Guide

Async Rust SDK & constant-time webhook engine for Stripe API v1 & v2 (`stripe-node`-style ergonomics).

## 1. Install & Features

```toml
[dependencies]
some-stripe = "0.9.0" # Standalone default (reqwest + rustls)
# Optional `some-payment-core::PaymentProvider` adapter:
# some-stripe = { version = "0.9.0", features = ["some-payment-adapter"] }
```

## 2. Client Init & Options

```rust
use std::time::Duration;
use some_stripe::{RequestOptions, StripeClient, StripeConfig};

// Direct key or "$ENV_VAR" (auto-resolves from env / .env)
let stripe = StripeClient::new("$STRIPE_SECRET_KEY");

let stripe = StripeClient::with_config(
    StripeConfig::new("sk_test_...")
        .with_api_base("https://api.stripe.com")
        .with_timeout(Duration::from_secs(30))
        .with_max_retries(3),
);

let opts = RequestOptions::new()
    .stripe_account("acct_123")
    .idempotency_key("order_1001_v1")
    .stripe_version("2026-11-18.endive")
    .timeout(Duration::from_secs(15));

// Connect-scoped resource handle (attaches `Stripe-Account`, including Paginator)
let acct_transfers = stripe.transfers().with_account("acct_123");
```

## 3. Resource Namespace Cheat-Sheet

| Domain | Accessors on `StripeClient` |
| --- | --- |
| Payments & Checkout | `payment_intents()`, `checkout().sessions()`, `payment_links()` |
| Customers & Billing | `customers()`, `subscriptions()`, `subscription_schedules()` |
| Invoicing & Quotes | `invoices()`, `credit_notes()`, `quotes()`, `billing_portal()` |
| Refunds, Disputes & Balance | `refunds()`, `disputes()`, `balance()`, `balance_transactions()` |
| Connect & Marketplace | `accounts()`, `account_links()`, `account_sessions()`, `persons()`, `transfers()`, `application_fees()`, `payouts()` |
| Tax & Identity | `tax()` (`calculations`, `transactions`, `settings`, `registrations`), `identity()` (`verification_sessions`, `verification_reports`) |
| Issuing, Treasury & Terminal | `issuing()` (`cards`, `cardholders`, `authorizations`, `transactions`), `treasury()` (`financial_accounts`, `transfers`), `terminal()` (`readers`, `locations`, `connection_tokens`) |
| Webhooks (v1) | `webhooks()` (`Webhook::construct_event`, `verify_stripe_raw_webhook`) |
| API v2 & Thin Events | `v2().core()` (`accounts`, `events`, `event_destinations`), `v2().billing()` (`meters`, `meter_events`, `meter_event_sessions`, `meter_event_stream`, `meter_event_adjustments`), `ThinEventHydrator::new(&stripe).hydrate_thin_event(&thin)` |

## 4. Minimal Copy-Paste Recipes

### PaymentIntent & CheckoutSession
```rust
use some_stripe::{CreateCheckoutSessionParams, CreatePaymentIntentParams, StripeClient};

let stripe = StripeClient::new("$STRIPE_SECRET_KEY");
let pi = stripe.payment_intents().create(
    &CreatePaymentIntentParams::new(5000, "usd")?
        .capture_method("automatic")
        .idempotency_key("order_1001_pi"),
).await?;

let session = stripe.checkout().sessions().create(&CreateCheckoutSessionParams {
    success_url: "https://example.com/success".into(),
    cancel_url: Some("https://example.com/cancel".into()),
    mode: "payment".into(),
    currency: Some("usd".into()),
    unit_amount: Some(5000),
    product_name: Some("Pro Plan".into()),
    quantity: Some(1),
    ..Default::default()
}).await?;
```

### Verify Webhook (Single & Multi-Secret)
```rust
use some_stripe::Webhook;
use some_stripe::webhooks::multi_v1_verifier::{
    WebhookVerificationOptions, verify_stripe_raw_webhook,
};

let event = Webhook::construct_event(raw_body, sig_header, "whsec_...")?;
let verified = verify_stripe_raw_webhook(
    raw_body,
    sig_header,
    &["whsec_new", "whsec_old"],
    WebhookVerificationOptions::new(chrono::Utc::now().timestamp()),
)?;
```

### 3-State `Nullable<T>` & Currency Guardrails
```rust
use some_stripe::Nullable;
use some_stripe::types::currency_guardrails::validate_stripe_money;

let (_omit, _clear, _set) = (Nullable::<String>::Omit, Nullable::<String>::Null, Nullable::Value("v".into()));
let money = validate_stripe_money(5000, "USD")?; // normalized "usd", minor-unit validated
```

### `some-payment-adapter` (`PaymentProvider`)
```rust
#[cfg(feature = "some-payment-adapter")]
use some_stripe::adapter::provider::{StripeProvider, StripeProviderConfig};

let cfg = StripeProviderConfig::new("$STRIPE_SECRET_KEY")
    .with_webhook_secrets(vec!["whsec_current".into(), "whsec_prev".into()])
    .with_stripe_version("2026-11-18.endive");
let provider = StripeProvider::new(cfg)?;
```

## 5. Critical Invariants & Error Handling

- **`StripeError`**: `Api`, `InvalidRequest`, `Http`, `ConnectionError`, `Json`, `Config`, `PaginationLoopDetected`, `Webhook(StripeWebhookError)`, `Redirect(RedirectError)`.
- **`StripeWebhookError`**: `MissingSignatureHeader`, `MissingSecret`, `BadKey`, `EmptyPayload`, `PayloadTooLarge`, `InvalidHeader`, `InvalidTimestamp`, `MissingSignature`, `BadSignature`, `TimestampOutsideTolerance`, `MalformedPayload`.
- **Amounts**: Positive integer minor units (`i64`, never floats). Zero-decimal (`jpy`, `krw`, ...) take whole units; 3-decimal (`bhd`, `kwd`, `omr`, `jod`, `tnd`) require `% 10 == 0`; `UGX`/`HUF`/`TWD` require `% 100 == 0`.
- **Webhooks**: Raw bytes only, 1 MiB max (`MAX_WEBHOOK_PAYLOAD_BYTES`), 300s past (`DEFAULT_TOLERANCE_SECONDS`) / 60s future (`MAX_FUTURE_DRIFT_SECONDS`), constant-time HMAC-SHA256.
- **Idempotency**: Mutating calls pass caller `Idempotency-Key` verbatim or auto-inject an RFC 4122 UUID v4.

For internal repository development & multi-agent PR governance, see `.agents/rules/`.
