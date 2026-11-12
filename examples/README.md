# `some-stripe` Framework Examples

All web and RPC framework integrations are organized into dedicated per-framework subdirectories under `examples/`:

| Framework | Directory | Examples & Unified Server | Integration Test Suite |
| :--- | :--- | :--- | :--- |
| **Axum 0.7** | [`examples/axum/`](./axum/README.md) | `01_axum_webhooks.rs` .. `06_axum_v2_meter_billing.rs` + [`axum_stripe_server/`](./axum/axum_stripe_server/main.rs) | [`tests/axum_examples_integration/`](../tests/axum_examples_integration/README.md) |
| **Actix-Web 4** | [`examples/actix_web/`](./actix_web/README.md) | `actix_core_and_webhooks.rs` .. `actix_v2_next_gen_services.rs` + [`actix_full_sdk_server.rs`](./actix_web/actix_full_sdk_server.rs) | [`tests/actix_examples/`](../tests/actix_examples/README.md) |
| **Tonic gRPC 0.12** | [`examples/tonic_grpc/`](./tonic_grpc/README.md) | `tonic_core_and_webhooks.rs` .. `tonic_v2_next_gen_services.rs` + [`tonic_full_sdk_server.rs`](./tonic_grpc/tonic_full_sdk_server.rs) + [`proto/some_stripe.proto`](./tonic_grpc/proto/some_stripe.proto) | [`tests/tonic_examples/`](../tests/tonic_examples/README.md) |
| **Rocket 0.5** | [`examples/rocket_stripe/`](./rocket_stripe/README.md) | Standalone workspace member (`examples/rocket_stripe/src/main.rs`) | `examples/rocket_stripe/tests/` |

---

## Native `$VAR_NAME` Secret Resolution

Every framework example uses `some-stripe`'s native `$VAR_NAME` automatic secret resolution (`StripeClient::new("$STRIPE_SECRET_KEY")`, `StripeConfig::new("$STRIPE_SECRET_KEY")`, `SecretKey::new("$STRIPE_WEBHOOK_SECRET")`), which resolves variables from process environment or `.env` / `.env.local` / `.env.dev` / `.env.example` automatically.

---

## Quick Start

### Axum Examples (`examples/axum/`)
```bash
cargo run --example 01_axum_webhooks
cargo run --example 02_axum_checkout_and_payments
cargo run --example 03_axum_subscriptions_and_invoicing
cargo run --example 04_axum_connect_marketplace
cargo run --example 05_axum_specialized_products
cargo run --example 06_axum_v2_meter_billing
cargo run --example axum_stripe_server
```

### Actix-Web Examples (`examples/actix_web/`)
```bash
cargo run --example actix_core_and_webhooks
cargo run --example actix_payments_and_settlement
cargo run --example actix_customers_and_subscriptions
cargo run --example actix_checkout_and_payment_links
cargo run --example actix_invoicing_and_billing_portal
cargo run --example actix_connect_marketplace
cargo run --example actix_specialized_products
cargo run --example actix_v2_next_gen_services
cargo run --example actix_full_sdk_server
```

### Tonic gRPC Examples (`examples/tonic_grpc/`)
```bash
cargo run --example tonic_core_and_webhooks
cargo run --example tonic_payments_and_settlement
cargo run --example tonic_customers_and_subscriptions
cargo run --example tonic_checkout_and_payment_links
cargo run --example tonic_invoicing_and_billing_portal
cargo run --example tonic_connect_marketplace
cargo run --example tonic_specialized_products
cargo run --example tonic_v2_next_gen_services
cargo run --example tonic_full_sdk_server
```

### Rocket Example (`examples/rocket_stripe/`)
```bash
cargo run --manifest-path examples/rocket_stripe/Cargo.toml
```
