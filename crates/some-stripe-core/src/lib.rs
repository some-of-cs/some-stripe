//! # some-stripe-core
//!
//! Microkernel core runtime, dual-protocol serialization, and thread-safe client execution
//! engine for `some-stripe`.

pub mod client;
pub mod error;
pub mod patterns;
pub mod workflow;

pub use client::{
    ClientInner, RequestOptions, SecretKey, StripeClient, StripeConfig, StripeHeaderExt,
};
pub use error::{StripeError, StripeWebhookError};
pub use patterns::{
    Nullable, percent_encode, serialize_v1_form, serialize_v2_json, to_form_params,
    to_form_urlencoded,
};
pub use workflow::{DefaultErrorWorkflow, ErrorAction, ErrorSeverity, ErrorWorkflowHandler};
