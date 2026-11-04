//! Cryptographic and operational constants for Stripe webhook verification.

/// Default timestamp tolerance window in seconds (5 minutes / 300s), per Stripe specification.
pub const DEFAULT_TOLERANCE_SECONDS: u64 = 300;

/// Maximum future drift tolerance in seconds (Gate G2 replay protection against clock skew).
pub const MAX_FUTURE_DRIFT_SECONDS: i64 = 60;

/// Maximum incoming webhook payload size (1 MB / Gate G2 memory exhaustion guard).
pub const MAX_WEBHOOK_PAYLOAD_BYTES: usize = 1_048_576;

/// Stripe signature header timestamp key.
pub const TIMESTAMP_PREFIX: &str = "t=";

/// Stripe signature header v1 scheme prefix.
pub const V1_SIGNATURE_PREFIX: &str = "v1=";
