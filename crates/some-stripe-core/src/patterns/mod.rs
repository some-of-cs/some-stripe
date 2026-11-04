//! Shared patterns catalog for `some-stripe`.

pub mod builder;
pub mod crud;
pub mod idempotency;
pub mod nullable;
pub mod pagination;
pub mod serializer;
pub mod transport;

pub use builder::StripeRequestBuilder;
pub use crud::{Creatable, Retrievable, resource_sub_url, resource_url, resource_url_with_id};
pub use idempotency::IdempotencyKeyManager;
pub use nullable::Nullable;
pub use pagination::{CursorLoopDetector, ListParams, ListResponse, Paginator};
pub use serializer::{
    percent_encode, serialize_v1_form, serialize_v2_json, to_form_params, to_form_urlencoded,
};
pub use transport::{HttpClient, ReqwestTransport, compute_backoff_delay};
