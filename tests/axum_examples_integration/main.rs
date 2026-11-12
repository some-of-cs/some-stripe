//! Full Integration Test Suite for the `some-stripe` Axum Web Framework Examples (`01`..`06` & Unified Server).
//!
//! # Running the Integration Test Suite
//! 1. Copy `.env.example` to `.env` and add your `STRIPE_SECRET_KEY`:
//!    ```bash
//!    cp .env.example .env
//!    ```
//! 2. Run the integration test target:
//!    ```bash
//!    cargo test --test axum_examples_integration_test
//!    ```

pub mod common;
mod ex01_webhooks_test;
mod ex02_checkout_and_payments_test;
mod ex03_subscriptions_and_invoicing_test;
mod ex04_connect_marketplace_test;
mod ex05_specialized_products_test;
mod ex06_v2_meter_billing_test;
mod ex07_unified_server_test;
