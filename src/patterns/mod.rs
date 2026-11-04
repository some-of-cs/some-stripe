//! Reusable architectural patterns, generic endpoint builders, and shared abstractions for `some-stripe`.
//!
//! # Architecture & Purpose
//!
//! To eliminate code duplication (DRY) and prevent boilerplate across the 38+ Stripe v1 and v2 services,
//! this module centralizes all shared behavioral patterns:
//!
//! - **Request Execution**: [`StripeRequestBuilder`](builder::StripeRequestBuilder) handles URL resolution,
//!   idempotency keys, multi-tenant headers, and execution.
//! - **CRUD Abstractions**: Standard endpoint traits ([`Creatable`](crud::Creatable), [`Retrievable`](crud::Retrievable),
//!   [`Listable`](crud::Listable)) and path formatting helpers.
//! - **Idempotency**: [`IdempotencyKeyManager`](idempotency::IdempotencyKeyManager) ensures key safety,
//!   UUID generation, and retry key retention.
//! - **Pagination Adapters**: Reusable stream engines for cursor-based and URL-driven list endpoints.
//!
//! # Examples
//!
//! ```
//! use some_stripe::patterns::crud::resource_url_with_id;
//!
//! let url = resource_url_with_id("https://api.stripe.com", "v1/customers", "cus_123");
//! assert_eq!(url, "https://api.stripe.com/v1/customers/cus_123");
//! ```

pub mod builder;
pub mod crud;
pub mod idempotency;
pub mod pagination;

pub use builder::StripeRequestBuilder;
pub use crud::{Creatable, Retrievable, resource_sub_url, resource_url, resource_url_with_id};
pub use idempotency::IdempotencyKeyManager;
pub use pagination::ListResponse;
