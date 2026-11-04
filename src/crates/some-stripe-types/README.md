# some-stripe-types

[![Crates.io](https://img.shields.io/crates/v/some-stripe-types.svg)](https://crates.io/crates/some-stripe-types)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Rust: 2024](https://img.shields.io/badge/Rust-2024_Edition-orange.svg)](https://www.rust-lang.org)

Strongly Typed Domain Models, Events & Request DTOs for [`some-stripe`](https://crates.io/crates/some-stripe).

---

## Architectural Highlights

- **Strongly Typed Domain Entities**:
  - Full serde JSON serialization and deserialization for core Stripe resources: `PaymentIntent`, `CheckoutSession`, `Account`, `Invoice`, `Dispute`, `Event`, and `EventData`.
  - Type-safe identifiers: `AccountId` (`acct_...`), `IdempotencyKey`, `DisputeId` (`dp_...`).
- **Disputes Management & Evidence Submission Locking**:
  - Complete 8-status dispute lifecycle state machine (`DisputeStatus`).
  - Exhaustive 20-field evidence container (`DisputeEvidence`) for fulfillment proofs, customer communications, duplicate charge explanations, and policy disclosures.
  - Evidence submission tracking (`DisputeEvidenceDetails`) with due dates and past-due detection.
  - Submission locking invariants: `Dispute::is_evidence_locked` and `UpdateDisputeParams::is_submission_final` prevent modifying finalized or closed dispute evidence.
- **Discriminated Event Union & Upcoming Invoice Guard**:
  - `EventObject` tagged union (`payment_intent`, `checkout.session`, `invoice`, `dispute`, with `#[serde(other)] Unknown` fallback).
  - `Invoice.id` is typed as `Option<String>` to safely accommodate `invoice.upcoming` preview events where `id` is null on Stripe payloads.
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
use some_stripe_types::{AccountId, IdempotencyKey, PaymentIntent, EventData, EventObject};

let account_id = AccountId::from("acct_1H2X3Y4Z");
assert_eq!(account_id.to_string(), "acct_1H2X3Y4Z");

let idempotency_key = IdempotencyKey::from("idem_tx_checkout_88712");
assert_eq!(idempotency_key.to_string(), "idem_tx_checkout_88712");

// EventObject Deserialization with invoice.upcoming null id support
let data = EventData {
    object: serde_json::json!({
        "id": null,
        "object": "invoice",
        "customer": "cus_preview_123",
        "amount_due": 3500
    }),
};

match data.deserialize_object().unwrap() {
    EventObject::Invoice(inv) => {
        assert!(inv.id.is_none());
        assert_eq!(inv.amount_due, Some(3500));
    }
    _ => panic!("Expected Invoice"),
}