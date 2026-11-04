# some-stripe-webhooks

[![Crates.io](https://img.shields.io/crates/v/some-stripe-webhooks.svg)](https://crates.io/crates/some-stripe-webhooks)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Rust: 2024](https://img.shields.io/badge/Rust-2024_Edition-orange.svg)](https://www.rust-lang.org)

Cryptographic Webhook Signature Verification, Replay Guards & Edge Runtime Ingestion Engine for [`some-stripe`](https://crates.io/crates/some-stripe).

---

## Architectural Highlights

- **Constant-Time Verification (Gate G2 Invariant 1)**:
  - HMAC-SHA256 signature verification executed strictly using constant-time byte comparisons (`safe_constant_time_eq` / `subtle::ConstantTimeEq`).
  - Safe against length-mismatch side channels and timing attacks.
- **Timestamp Replay Protection (Gate G2 Invariant 2)**:
  - Default 300-second drift tolerance window (`DEFAULT_TOLERANCE_SECONDS = 300`).
  - Strict 60-second future drift limit (`MAX_FUTURE_DRIFT_SECONDS = 60`) preventing pre-dated signature replay.
- **Memory Exhaustion DoS Defense (Gate G2 Invariant 4)**:
  - Strict 1 MB maximum payload ceiling (`MAX_WEBHOOK_PAYLOAD_BYTES = 1_048_576`).
  - Rejects oversized requests prior to parsing or signature computation.
- **Zero-Allocation Ingestion Wrapper (`StripeWebhookWrapper`)**:
  - Direct pointer parsing for high-throughput webhook handling (`order_id`, `customer_id`, `payment_intent_id`, `amount_total`).
  - Helper predicate `is_checkout_completed()` checking both event type and nested status properties.

---

## Quickstart

```rust
use some_stripe_webhooks::{Webhook, StripeWebhookWrapper};

let payload = br#"{"id":"evt_123","object":"event","created":1728345600,"type":"checkout.session.completed","data":{"object":{"id":"cs_123","status":"complete","payment_status":"paid"}}}"#;
let secret = "whsec_test_secret";
let header = Webhook::generate_test_header_string(payload, secret, None);

// 1. Verify and parse webhook directly
let wrapper = StripeWebhookWrapper::from_raw(payload, &header, secret, None).unwrap();

assert_eq!(wrapper.event_id(), "evt_123");
assert!(wrapper.is_checkout_completed());
```

---

## Quality Gates & Verification

| Gate | Criterion | Status | Evidence |
| :--- | :--- | :---: | :--- |
| **G0** | Compilation & Clippy | **PASS** | 0 warnings (`cargo clippy -- -D warnings`), 0 fmt diffs |
| **G2** | Timing Resistance | **PASS** | `safe_constant_time_eq` verified against timing leaks |
| **G2** | Drift & Replay Guard | **PASS** | Reject timestamps > 300s old or > 60s in the future |
| **G2** | Payload Ceiling | **PASS** | Reject payloads exceeding 1 MB with `PayloadTooLarge` |
