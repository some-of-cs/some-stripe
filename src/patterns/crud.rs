//! Reusable CRUD traits, endpoint URL formatters, and path construction patterns.
//!
//! # Purpose
//! Eliminates repetitive URL formatting and boilerplate method signatures across resource namespaces.

use crate::error::StripeError;
use async_trait::async_trait;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Formats a base Stripe endpoint URL from an API base and resource path segment.
///
/// # Examples
///
/// ```
/// use some_stripe::patterns::crud::resource_url;
///
/// let endpoint = resource_url("https://api.stripe.com", "v1/payment_intents");
/// assert_eq!(endpoint, "https://api.stripe.com/v1/payment_intents");
/// ```
#[inline]
pub fn resource_url(base: &str, path: &str) -> String {
    let clean_base = base.trim_end_matches('/');
    let clean_path = path.trim_start_matches('/');
    format!("{clean_base}/{clean_path}")
}

/// Formats a resource instance URL with an identifier.
///
/// # Examples
///
/// ```
/// use some_stripe::patterns::crud::resource_url_with_id;
///
/// let url = resource_url_with_id("https://api.stripe.com", "v1/checkout/sessions", "cs_123");
/// assert_eq!(url, "https://api.stripe.com/v1/checkout/sessions/cs_123");
/// ```
#[inline]
pub fn resource_url_with_id(base: &str, path: &str, id: &str) -> String {
    let parent = resource_url(base, path);
    format!("{parent}/{id}")
}

/// Formats a sub-resource URL attached to a parent resource identifier.
///
/// # Examples
///
/// ```
/// use some_stripe::patterns::crud::resource_sub_url;
///
/// let url = resource_sub_url("https://api.stripe.com", "v1/customers", "cus_123", "balance_transactions");
/// assert_eq!(url, "https://api.stripe.com/v1/customers/cus_123/balance_transactions");
/// ```
#[inline]
pub fn resource_sub_url(base: &str, path: &str, id: &str, subpath: &str) -> String {
    let clean_sub = subpath.trim_start_matches('/');
    format!("{}/{}/{}", resource_url(base, path), id, clean_sub)
}

/// A standard contract for Stripe resources that support creation (`POST /v1/{resource}`).
#[async_trait]
pub trait Creatable<Params, Entity>: Send + Sync
where
    Params: Serialize + Sync,
    Entity: DeserializeOwned + Send,
{
    /// Creates a new entity using the provided parameter reference.
    async fn create(&self, params: &Params) -> Result<Entity, StripeError>;
}

/// A standard contract for Stripe resources that can be retrieved by unique identifier (`GET /v1/{resource}/{id}`).
#[async_trait]
pub trait Retrievable<Entity>: Send + Sync
where
    Entity: DeserializeOwned + Send,
{
    /// Retrieves the entity with the specified identifier.
    async fn retrieve(&self, id: &str) -> Result<Entity, StripeError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_url_formatting() {
        assert_eq!(
            resource_url("https://api.stripe.com/", "/v1/charges"),
            "https://api.stripe.com/v1/charges"
        );
        assert_eq!(
            resource_url("https://api.stripe.com", "v1/refunds"),
            "https://api.stripe.com/v1/refunds"
        );
    }

    #[test]
    fn test_resource_url_with_id_formatting() {
        assert_eq!(
            resource_url_with_id("https://api.stripe.com", "v1/customers", "cus_999"),
            "https://api.stripe.com/v1/customers/cus_999"
        );
    }

    #[test]
    fn test_resource_sub_url_formatting() {
        assert_eq!(
            resource_sub_url(
                "https://api.stripe.com",
                "v1/accounts",
                "acct_123",
                "/capabilities"
            ),
            "https://api.stripe.com/v1/accounts/acct_123/capabilities"
        );
    }
}
