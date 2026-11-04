//! Shared patterns catalog for `some-stripe`.

pub mod nullable;
pub mod serializer;

pub use nullable::Nullable;
pub use serializer::{
    percent_encode, serialize_v1_form, serialize_v2_json, to_form_params, to_form_urlencoded,
};
