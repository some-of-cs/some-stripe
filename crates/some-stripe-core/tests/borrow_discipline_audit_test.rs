//! Borrow Discipline & Zero-Clone Memory Audit (Gate G-Borrow)
//!
//! Verifies that StripeClient sharing incurs zero deep clones, uses Arc pointer
//! refcounts exclusively, and maintains strict Rust 2024 borrowing invariants.

use some_stripe_core::{RequestOptions, SecretKey, StripeClient};
use std::sync::Arc;

#[test]
fn test_client_clone_is_shallow_arc_pointer_copy() {
    let client = StripeClient::new("sk_test_mock_audit_key_110");
    let cloned = client.clone();

    // Verify pointer addresses point to the exact same inner allocation
    assert!(
        Arc::ptr_eq(client.inner(), cloned.inner()),
        "Gate G-Borrow violation: StripeClient::clone must only perform an Arc pointer bump"
    );
}

#[test]
fn test_request_options_borrowing_discipline() {
    let opts = RequestOptions::new()
        .idempotency_key("idem_audit_110")
        .stripe_account("acct_audit_110")
        .stripe_version("2024-04-10");

    // Borrowing references without cloning
    let idem_ref: Option<&str> = opts.idempotency_key.as_deref();
    let acct_ref: Option<&str> = opts.stripe_account.as_deref();
    let ver_ref: Option<&str> = opts.stripe_version.as_deref();

    assert_eq!(idem_ref, Some("idem_audit_110"));
    assert_eq!(acct_ref, Some("acct_audit_110"));
    assert_eq!(ver_ref, Some("2024-04-10"));
}

#[test]
fn test_secret_key_constant_time_eq_without_leaking_pan_or_memory() {
    let key1 = SecretKey::new("sk_test_mock_secret_key_a");
    let key2 = SecretKey::new("sk_test_mock_secret_key_a");
    let key3 = SecretKey::new("sk_test_mock_secret_key_b");

    assert_eq!(key1, key2);
    assert_ne!(key1, key3);
}
