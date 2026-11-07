//! # some-stripe-core
//!
//! Microkernel core runtime, dual-protocol serialization, and thread-safe client execution
//! engine for `some-stripe`.
//!
//! ## Architectural Pillars
//!
//! 1. **Thread-Safe Client Concurrency**:
//!    - Zero-lock `StripeClient` sharing state via `Arc<ClientInner>`.
//!    - Lock-free, zero-contention request dispatch across thousands of concurrent Tokio tasks.
//!    - Secret key zeroization on drop and `Debug` redaction (`[REDACTED]`).
//!
//! 2. **Dual-Protocol Serialization & 3-State Nullability**:
//!    - Canonical 3-state `Nullable<T>` (`Value`, `Null`, `Omit`).
//!    - Encodes `Nullable::Null` as empty string (`field=`) in Stripe API v1 form payloads to clear fields,
//!      and as `null` (`"field": null`) in Stripe API v2 JSON payloads.
//!    - Zero-indexed bracket notation for nested arrays of objects (`items[0][price]=...`).
//!
//! 3. **Auto-Pagination Stream Engine & HTTP Transport**:
//!    - Zero-clone lazy async pagination stream (`VecDeque<T>` move semantics).
//!    - Bounded-history circular cursor sentinel circuit breaker preventing infinite pagination loops.
//!    - Pluggable `HttpClient` transport port with exponential backoff and jitter.
//!
//! 4. **Idempotency-Aware Retries & Resilient Transport**:
//!    - Production `ReqwestTransport` with 90-second idle connection pooling, 60-second TCP keepalive, and redirect policy `none`.
//!    - Automatic exponential backoff with deterministic jitter (`compute_backoff_delay`).
//!    - Strict retry safety: state-mutating POST requests are only retried across network drops when guarded by an `Idempotency-Key`.
//!
//! 5. **Anti-Traversal Resource ID Security**:
//!    - Strict path traversal prevention via `validate_and_encode_id`, `try_resource_url_with_id`, and `try_resource_sub_url`.
//!    - Blocks path separators (`/`, `\`), directory traversal (`..`), and URI delimiter injection (`?`, `#`).
//!
//! ## Client Initialization Example
//!
//! ```
//! use some_stripe_core::{StripeClient, StripeConfig, SecretKey, RequestOptions};
//! use std::time::Duration;
//!
//! let key = SecretKey::new("sk_test_mock_123");
//! let config = StripeConfig::new(key)
//!     .with_timeout(Duration::from_secs(15))
//!     .with_max_retries(2);
//!
//! let client = StripeClient::with_config(config);
//! assert_eq!(client.config.api_base, "https://api.stripe.com");
//!
//! let opts = RequestOptions::new()
//!     .stripe_account("acct_123")
//!     .idempotency_key("idem_abc");
//! assert_eq!(opts.stripe_account.as_deref(), Some("acct_123"));
//! ```
//!
//! ## Dual-Protocol Serialization Example
//!
//! ```
//! use some_stripe_core::patterns::{Nullable, serialize_v1_form, serialize_v2_json};
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct CustomerUpdate<'a> {
//!     #[serde(default, skip_serializing_if = "Nullable::is_omit")]
//!     email: Nullable<&'a str>,
//!     #[serde(default, skip_serializing_if = "Nullable::is_omit")]
//!     name: Nullable<&'a str>,
//! }
//!
//! // Nullable::Null serializes as email= in Stripe v1 form urlencoded to clear the field:
//! let update = CustomerUpdate {
//!     email: Nullable::Null,
//!     name: Nullable::Omit,
//! };
//! assert_eq!(serialize_v1_form(&update).unwrap(), "email=");
//!
//! // In Stripe v2 JSON, Nullable::Null serializes as {"email":null}:
//! assert_eq!(serialize_v2_json(&update).unwrap(), r#"{"email":null}"#);
//! ```

pub mod client;
pub mod error;
pub mod patterns;
pub mod workflow;

pub use bytes::Bytes;
pub use client::{
    ClientInner, RequestOptions, SecretKey, StripeClient, StripeConfig, StripeHeaderExt,
};
pub use error::{RedirectError, StripeError, StripeWebhookError};
pub use patterns::{
    Nullable, percent_encode, serialize_v1_form, serialize_v2_json, to_form_params,
    to_form_urlencoded, validate_and_encode_id,
};
pub use workflow::{DefaultErrorWorkflow, ErrorAction, ErrorSeverity, ErrorWorkflowHandler};
