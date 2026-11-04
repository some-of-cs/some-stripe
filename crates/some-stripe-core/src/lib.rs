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
pub use patterns::Nullable;
pub use workflow::{DefaultErrorWorkflow, ErrorAction, ErrorSeverity, ErrorWorkflowHandler};
