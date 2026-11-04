# Rule: Dual-Protocol Serialization & Nullability Mechanics

## Core Objective
Guarantee wire fidelity with Stripe backend parsers across both legacy API v1 endpoints and next-generation API v2 endpoints.

## Mandatory Invariants
1. **API v1 Form-Urlencoded Protocol**:
   - Outgoing v1 requests MUST send `Content-Type: application/x-www-form-urlencoded`. Sending `application/json` to v1 POST endpoints is strictly forbidden (triggers HTTP 400).
   - Array of objects MUST be serialized using explicit zero-based indices (e.g. `items[0][id]=si_123&items[1][id]=si_456`). Unindexed empty brackets (`items[][id]=...`) cause backend misattribution (stripe-node #561) and are prohibited.
2. **3-State Nullable Semantics**:
   - Parameters MUST use `Nullable<T>` with distinct variants:
     - `Nullable::Value(T)`: Serializes the encoded value.
     - `Nullable::Null`: Serializes to `key=""` in v1 form or `key: null` in v2 JSON to explicitly clear remote fields.
     - `Nullable::Omit`: Skips the field entirely from serialization.
3. **API v2 Native JSON Protocol**:
   - Outgoing v2 requests (`/v2/*`) MUST send `Content-Type: application/json` with operation-scoped request DTOs.
   - V2 list endpoints must follow URL-driven pagination (`next_page_url` and `previous_page_url`).
4. **Encoding Determinism**:
   - RFC 3986 percent-encoding must be applied to all key names, bracket tokens, and values.
