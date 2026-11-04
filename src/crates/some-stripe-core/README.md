# some-stripe-core

[![Crates.io](https://img.shields.io/crates/v/some-stripe-core.svg)](https://crates.io/crates/some-stripe-core)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Rust: 2024](https://img.shields.io/badge/Rust-2024_Edition-orange.svg)](https://www.rust-lang.org)

Microkernel Core Runtime, Dual-Protocol Serialization & Thread-Safe Concurrency Engine for [`some-stripe`](https://crates.io/crates/some-stripe).

---

## Architectural Highlights

- **Thread-Safe Lock-Free Client (`StripeClient`)**:
  - Thin wrapper over `Arc<ClientInner>` providing zero-contention request dispatch across thousands of concurrent Tokio tasks.
  - Rust zero-clone borrowing discipline (Gate G-Borrow): entity clones are strictly forbidden; pointer clones (`Arc::clone`) are used for thread sharing.
- **Constant-Time Secret Protection (`SecretKey`)**:
  - Secure in-memory storage implementing `zeroize::ZeroizeOnDrop`.
  - Constant-time equality (`subtle::ConstantTimeEq`) preventing timing side-channel attacks (Gate G2).
  - Redacted from `Debug` and `Display` output (`[REDACTED]`) per PCI-DSS 4.0 requirements (Gate G-Law).
- **Dual-Protocol Serialization Engine (`patterns/`)**:
  - **3-State `Nullable<T>`**: Distinguishes between `Value(T)`, `Null` (clears remote fields via `field=""` in v1 form-encoded or `"field": null` in v2 JSON), and `Omit` (skipped on wire).
  - **Zero-Indexed Bracket Notation**: Strictly encodes nested object arrays as `items[0][id]=...&items[1][id]=...`, preventing Stripe backend index confusion.
  - **RFC 3986 Percent-Encoding**: Strict character escape set (`%5B`, `%5D`, `%20`) with bounded stack recursion (max depth 16) to prevent denial of service.
- **Zero-Clone Async Pagination Stream Engine (`Paginator<T>`)**:
  - Consumes items via `VecDeque<T>` move semantics without requiring `T: Clone`.
  - Automatically bridges empty intermediate pages (resolves Stripe Issue #2730).
  - Flat memory residency ($\mathcal{O}(\text{limit})$) during high-throughput stream traversal.
- **Cyclic Cursor Sentinel Circuit Breaker (`CursorLoopDetector`)**:
  - Bounded-history ring buffer tracking pagination cursors.
  - Terminate immediately with `StripeError::PaginationLoopDetected` if a cursor token repeats, preventing infinite loops and billing exhaustion.
- **Pluggable HTTP Transport Port (`HttpClient`)**:
  - `#[async_trait] pub trait HttpClient: Send + Sync + Debug`.
  - Production `ReqwestTransport` with rustls TLS, connection pooling, and exponential backoff retry with full jitter for HTTP 429 and 5xx responses.

---

## Quickstart

```rust
use some_stripe_core::{StripeClient, StripeConfig, SecretKey, RequestOptions};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let key = SecretKey::new("sk_test_mock_key_123");
    let config = StripeConfig::new(key)
        .with_timeout(Duration::from_secs(30))
        .with_max_retries(3);

    let client = StripeClient::with_config(config);
    assert_eq!(client.config.api_base, "https://api.stripe.com");

    let opts = RequestOptions::new()
        .stripe_account("acct_connected_123")
        .idempotency_key("idem_tx_abc");
    assert_eq!(opts.stripe_account.as_deref(), Some("acct_connected_123"));
}
```

---

## Dual-Protocol Serialization Example

```rust
use some_stripe_core::patterns::{Nullable, serialize_v1_form, serialize_v2_json};
use serde::Serialize;

#[derive(Serialize)]
struct CustomerUpdateRequest<'a> {
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    email: Nullable<&'a str>,
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    description: Nullable<&'a str>,
}

let update = CustomerUpdateRequest {
    email: Nullable::Null, // Clears the remote field
    description: Nullable::Omit, // Excluded from payload
};

// API v1: Form urlencoded
assert_eq!(serialize_v1_form(&update).unwrap(), "email=");

// API v2: JSON payload
assert_eq!(serialize_v2_json(&update).unwrap(), r#"{"email":null}"#);
```

---

## Quality Gates & Verification

| Gate | Criterion | Status | Evidence |
| :--- | :--- | :---: | :--- |
| **G0** | Compilation & Clippy | **PASS** | 0 warnings (`cargo clippy -- -D warnings`), 0 fmt diffs |
| **G-Borrow** | Borrowing Discipline | **PASS** | `Arc::ptr_eq` pointer copies, zero domain clones |
| **G2** | Cryptographic Security | **PASS** | Constant-time HMAC & secret equality, zeroize on drop |
| **G-Law** | PCI-DSS 4.0 Compliance | **PASS** | `[REDACTED]` debug masking, zero PAN memory persistence |
| **G1** | Concurrency & Wire Tests| **PASS** | 10k Tokio concurrency stress tests & mock HTTP assertions |
