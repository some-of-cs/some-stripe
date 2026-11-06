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
- **Pure-Rust Zero-Clone Streaming HMAC (Gate G-Borrow)**:
  - Streaming HMAC computation via [`streaming_hmac_sha256_hex`] powered by pure-Rust `hmac` and `sha2` crates (zero C/OpenSSL dependencies, WASM & Edge runtime ready).
  - Eliminates intermediate heap-allocated `${timestamp}.${payload}` vectors.
  - Zero copies of the raw webhook payload during verification.
- **Timestamp Replay Protection (Gate G2 Invariant 2)**:
  - Default 300-second drift tolerance window (`DEFAULT_TOLERANCE_SECONDS = 300`).
  - Strict 60-second future drift limit (`MAX_FUTURE_DRIFT_SECONDS = 60`) preventing pre-dated signature replay.
- **Memory Exhaustion DoS Defense (Gate G2 Invariant 4)**:
  - Strict 1 MB maximum payload ceiling (`MAX_WEBHOOK_PAYLOAD_BYTES = 1_048_576`).
  - Rejects oversized requests prior to parsing or signature computation.
- **Zero-Allocation Ingestion Wrapper (`StripeWebhookWrapper`)**:
  - Direct pointer parsing for high-throughput webhook handling (`order_id`, `customer_id`, `payment_intent_id`, `amount_total`).
  - Helper predicates `is_checkout_completed()` (supporting `paid`, `unpaid`, and `no_payment_required`) and `is_checkout_fulfillment_ready()` (enforcing payment guarantee prior to order fulfillment).
- **Statutory Financial Compliance (Gate G-Law)**:
  - Zero raw PAN handling; error and wrapper types redact secrets in `Debug` / `Display`.
  - PSD2 SCA 3DS 2.0 challenge structures preserved without modification.

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
assert!(wrapper.is_checkout_fulfillment_ready());
```