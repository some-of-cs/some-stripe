# some-stripe-types

[![Crates.io](https://img.shields.io/crates/v/some-stripe-types.svg)](https://crates.io/crates/some-stripe-types)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Rust: 2024](https://img.shields.io/badge/Rust-2024_Edition-orange.svg)](https://www.rust-lang.org)

Strongly Typed Domain Models, Events & Request DTOs for [`some-stripe`](https://crates.io/crates/some-stripe).

---

## Architectural Highlights

- **Strongly Typed Domain Entities**:
  - Full serde JSON serialization and deserialization for core Stripe resources: `PaymentIntent`, `CheckoutSession`, `Account`, `Event`, and `EventData`.
  - Type-safe identifiers: `AccountId` (`acct_...`), `IdempotencyKey`.
- **Statutory Financial & PCI-DSS 4.0 Compliance (Gate G-Law)**:
  - Strict custom `std::fmt::Debug` implementations masking sensitive fields (e.g. `client_secret` in `PaymentIntent`) with `[REDACTED]`.
  - Zero raw PAN or secret retention in formatted error strings and logs.
  - Minor currency integer amounts (`i64` in cents) preventing IEEE 754 floating-point rounding errors (ISO 4217).
- **Operation-Scoped Request DTOs**:
  - `CreateCheckoutSessionParams`, `CreatePaymentIntentParams`, `CapturePaymentIntentParams`, `CancelPaymentIntentParams`.
  - Clean separation between immutable domain state and updatable parameters.

---

## Quickstart

```rust
use some_stripe_types::{AccountId, IdempotencyKey, PaymentIntent};

let account_id = AccountId::from("acct_1H2X3Y4Z");
assert_eq!(account_id.to_string(), "acct_1H2X3Y4Z");

let idempotency_key = IdempotencyKey::from("idem_tx_checkout_88712");
assert_eq!(idempotency_key.to_string(), "idem_tx_checkout_88712");

let payload = r#"{
    "id": "pi_123456789",
    "object": "payment_intent",
    "amount": 5000,
    "currency": "usd",
    "status": "succeeded"
}"#;

let intent: PaymentIntent = serde_json::from_str(payload).unwrap();
assert_eq!(intent.id, "pi_123456789");
assert_eq!(intent.amount, 5000);
```

---

## Quality Gates & Verification

| Gate | Criterion | Status | Evidence |
| :--- | :--- | :---: | :--- |
| **G0** | Compilation & Clippy | **PASS** | 0 warnings (`cargo clippy -- -D warnings`), 0 fmt diffs |
| **G-Borrow** | Zero-Clone Discipline | **PASS** | Zero clone calls inside hot deserialization pipelines |
| **G-Law** | PCI-DSS Redaction | **PASS** | Sensitive `client_secret` redacted to `[REDACTED]` in `Debug` |
| **G1** | Contract Testing | **PASS** | Comprehensive JSON roundtrip tests in `types_contract_test.rs` |
