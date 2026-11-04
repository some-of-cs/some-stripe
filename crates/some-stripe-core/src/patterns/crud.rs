//! Generic CRUD endpoint helpers and traits.

use crate::client::StripeClient;
use crate::error::StripeError;
use async_trait::async_trait;
use serde::Serialize;

/// Assembles a normalized Stripe REST API endpoint URL.
pub fn resource_url(api_base: &str, resource_path: &str) -> String {
    let base = api_base.trim_end_matches('/');
    let path = resource_path.trim_start_matches('/');
    format!("{base}/{path}")
}

/// Assembles a REST API endpoint URL for an individual resource item by ID.
pub fn resource_url_with_id(api_base: &str, resource_path: &str, id: &str) -> String {
    let base = api_base.trim_end_matches('/');
    let path = resource_path.trim_start_matches('/');
    format!("{base}/{path}/{id}")
}

/// Assembles a sub-resource endpoint URL.
pub fn resource_sub_url(api_base: &str, resource_path: &str, id: &str, action: &str) -> String {
    let base = api_base.trim_end_matches('/');
    let path = resource_path.trim_start_matches('/');
    format!("{base}/{path}/{id}/{action}")
}

/// Trait implemented by endpoints that support retrieving resources by ID.
#[async_trait]
pub trait Retrievable: Sized {
    /// Retrieve a resource by its unique identifier.
    async fn retrieve(client: &StripeClient, id: &str) -> Result<Self, StripeError>;
}

/// Trait implemented by endpoints that support creating resources.
#[async_trait]
pub trait Creatable<Params: Serialize + Send + Sync>: Sized {
    /// Create a new resource with the specified parameters.
    async fn create(client: &StripeClient, params: &Params) -> Result<Self, StripeError>;
}
