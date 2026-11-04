//! API Facade & RequestOptions Unit Tests for US-1.1.0 Unit 2.

use some_stripe_core::client::{RequestOptions, StripeClient, StripeConfig, StripeHeaderExt};
use std::time::Duration;

#[test]
fn test_facade_request_options_chaining() {
    let opts = RequestOptions::new()
        .stripe_account("acct_sub_456")
        .idempotency_key("idemp_unique_key_123")
        .stripe_version("2024-06-20")
        .timeout(Duration::from_secs(15));

    assert_eq!(opts.stripe_account.as_deref(), Some("acct_sub_456"));
    assert_eq!(opts.idempotency_key.as_deref(), Some("idemp_unique_key_123"));
    assert_eq!(opts.stripe_version.as_deref(), Some("2024-06-20"));
    assert_eq!(opts.timeout, Some(Duration::from_secs(15)));
}

#[test]
fn test_header_ext_trait() {
    let client = reqwest::Client::new();
    let req = client.get("https://api.stripe.com/v1/charges")
        .stripe_account("acct_test")
        .idempotency_key("idemp_test")
        .stripe_version("2024-06-20");

    let built = req.build().unwrap();
    assert_eq!(built.headers().get("Stripe-Account").unwrap(), "acct_test");
    assert_eq!(built.headers().get("Idempotency-Key").unwrap(), "idemp_test");
    assert_eq!(built.headers().get("Stripe-Version").unwrap(), "2024-06-20");
}
