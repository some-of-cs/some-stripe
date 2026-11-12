//! Top-Level Integration Test Aggregator for the `some-stripe` Rocket.rs (`rocket 0.5`) Example Suite.
//!
//! Runs all 8 Rocket.rs integration test suites (`checkout_and_payment_links`, `payments_and_settlement`,
//! `customers_and_billing`, `connect_marketplace`, `specialized_products`, `v2_next_gen_services`,
//! `webhooks_and_security`, and `full_sdk_server`).
//!
//! # Running the Integration Test Suite
//! ```bash
//! cargo test --test rocket_examples_integration_test
//! cargo test --test 'rocket_*'
//! ```

#[path = "checkout_and_payment_links/main.rs"]
mod checkout_and_payment_links;

#[path = "payments_and_settlement/main.rs"]
mod payments_and_settlement;

#[path = "customers_and_billing/main.rs"]
mod customers_and_billing;

#[path = "connect_marketplace/main.rs"]
mod connect_marketplace;

#[path = "specialized_products/main.rs"]
mod specialized_products;

#[path = "v2_next_gen_services/main.rs"]
mod v2_next_gen_services;

#[path = "webhooks_and_security/main.rs"]
mod webhooks_and_security;

#[path = "full_sdk_server/main.rs"]
mod full_sdk_server;
