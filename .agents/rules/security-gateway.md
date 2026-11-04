# Rule: Security Gateway & Cryptographic Invariants (Gate G2)

## Core Objective
Ensure that `some-stripe` is resilient against timing side-channels, replay attacks, memory exhaustion, and credential leakage in zero-trust edge and serverless environments.

## Mandatory Invariants
1. **Constant-Time Cryptographic Comparison**:
   - All signature byte comparisons (HMAC digests in `Stripe-Signature`) MUST use constant-time operations (`subtle::ConstantTimeEq` or `openssl::memcmp::eq`).
   - Standard short-circuiting comparisons (`==`, `eq`, `std::cmp::PartialEq`) on secret or hash slices are strictly prohibited.
2. **Replay Protection Gate**:
   - Webhook timestamps `t` MUST be validated against the configured tolerance window (`DEFAULT_TOLERANCE_SECONDS = 300`).
   - Events older than 300 seconds or drifting more than 60 seconds into the future MUST be rejected with `StripeWebhookError::TimestampOutsideTolerance`.
3. **Secret Redaction & Memory Zeroization**:
   - Client secrets (`sk_live_...`, `whsec_...`) MUST implement `zeroize::Zeroize` or volatile memory overwrite upon `Drop`.
   - The `std::fmt::Debug` implementation for `StripeConfig` and client structs MUST redact all secrets to `"[REDACTED]"`.
4. **Bounded Memory Allocation**:
   - Maximum webhook payload size is capped at 1 MB (`MAX_PAYLOAD_BYTES = 1_048_576`). Payloads exceeding this limit must be rejected prior to HMAC allocation.
5. **Security Escalation Telemetry**:
   - Forged signatures and replay attacks MUST trigger the registered `on_security_incident` callback with `ErrorSeverity::Critical`.
