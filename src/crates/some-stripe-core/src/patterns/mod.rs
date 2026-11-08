//! Shared patterns catalog for `some-stripe`.

pub mod builder;
pub mod crud;
pub mod idempotency;
pub mod meter_serializer;
pub mod nullable;
pub mod pagination;
pub mod serializer;
pub mod transport;

pub use builder::StripeRequestBuilder;
pub use crud::{
    Creatable, Retrievable, resource_sub_url, resource_url, resource_url_with_id,
    try_resource_sub_url, try_resource_url_with_id, validate_and_encode_id,
};
pub use idempotency::IdempotencyKeyManager;
pub use meter_serializer::{ESTIMATED_EVENT_BYTES, MeterBatchSerializer};
pub use nullable::Nullable;
pub use pagination::{CursorLoopDetector, ListParams, ListResponse, Paginator};
pub use serializer::{
    encode_form_pairs, percent_encode, serialize_v1_form, serialize_v2_json, to_form_params,
    to_form_urlencoded,
};
pub use transport::{HttpClient, ReqwestTransport, compute_backoff_delay};
