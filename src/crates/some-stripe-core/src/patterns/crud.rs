//! Generic CRUD endpoint helpers and traits.

use crate::client::StripeClient;
use crate::error::StripeError;
use async_trait::async_trait;
use serde::Serialize;

/// Validates and percent-encodes a resource ID or pagination cursor to prevent path traversal
/// and query parameter injection attacks.
///
/// Ensures the identifier is non-empty, contains no path traversal sequences (`..`, `/`, `\`),
/// and contains no query or URI fragment delimiters (`?`, `#`). Safe characters are percent-encoded
/// according to RFC 3986.
///
/// # Errors
///
/// - Returns [`StripeError::InvalidRequest`] with message `"Resource ID cannot be empty or whitespace"`
///   if `id` is empty or consists entirely of whitespace.
/// - Returns [`StripeError::InvalidRequest`] if `id` contains `'..'`, `'/'`, `'\\'`, `'?'`, or `'#'`.
///
/// # Examples
///
/// ```
/// use some_stripe_core::patterns::crud::validate_and_encode_id;
///
/// let id = validate_and_encode_id("dp_12345").unwrap();
/// assert_eq!(id, "dp_12345");
///
/// assert!(validate_and_encode_id("").is_err());
/// assert!(validate_and_encode_id("   ").is_err());
/// assert!(validate_and_encode_id("../../account").is_err());
/// assert!(validate_and_encode_id("dp_123?inject=1").is_err());
/// assert!(validate_and_encode_id("dp_123#frag").is_err());
/// ```
pub fn validate_and_encode_id(id: &str) -> Result<String, StripeError> {
    if id.trim().is_empty() {
        return Err(StripeError::InvalidRequest(
            "Resource ID cannot be empty or whitespace".into(),
        ));
    }
    if id.contains("..")
        || id.contains('/')
        || id.contains('\\')
        || id.contains('?')
        || id.contains('#')
    {
        return Err(StripeError::InvalidRequest(format!(
            "Resource ID contains invalid characters (path traversal or query delimiter): '{id}'"
        )));
    }
    Ok(crate::patterns::serializer::percent_encode(id))
}

/// Assembles a normalized Stripe REST API endpoint URL.
///
/// # Examples
///
/// ```
/// use some_stripe_core::patterns::crud::resource_url;
///
/// let url = resource_url("https://api.stripe.com/", "/v1/charges");
/// assert_eq!(url, "https://api.stripe.com/v1/charges");
/// ```
pub fn resource_url(api_base: &str, resource_path: &str) -> String {
    let base = api_base.trim_end_matches('/');
    let path = resource_path.trim_start_matches('/').trim_end_matches('/');
    format!("{base}/{path}")
}

/// Assembles a REST API endpoint URL for an individual resource item by ID.
///
/// Validates and percent-encodes `id` to prevent path traversal and injection attacks.
///
/// # Errors
///
/// Returns [`StripeError::InvalidRequest`] if `id` is empty, whitespace-only, or contains forbidden characters.
///
/// # Examples
///
/// ```
/// use some_stripe_core::patterns::crud::resource_url_with_id;
///
/// let url = resource_url_with_id("https://api.stripe.com", "v1/disputes", "dp_123").unwrap();
/// assert_eq!(url, "https://api.stripe.com/v1/disputes/dp_123");
///
/// assert!(resource_url_with_id("https://api.stripe.com", "v1/disputes", "../../account").is_err());
/// ```
pub fn resource_url_with_id(
    api_base: &str,
    resource_path: &str,
    id: &str,
) -> Result<String, StripeError> {
    let encoded = validate_and_encode_id(id)?;
    let base = api_base.trim_end_matches('/');
    let path = resource_path.trim_start_matches('/').trim_end_matches('/');
    Ok(format!("{base}/{path}/{encoded}"))
}

/// Assembles a sub-resource endpoint URL.
///
/// Validates and percent-encodes `id` to prevent path traversal and injection attacks.
///
/// # Errors
///
/// Returns [`StripeError::InvalidRequest`] if `id` is empty, whitespace-only, or contains forbidden characters.
///
/// # Examples
///
/// ```
/// use some_stripe_core::patterns::crud::resource_sub_url;
///
/// let url = resource_sub_url("https://api.stripe.com", "v1/disputes", "dp_123", "close").unwrap();
/// assert_eq!(url, "https://api.stripe.com/v1/disputes/dp_123/close");
///
/// assert!(resource_sub_url("https://api.stripe.com", "v1/disputes", "dp_123?inject=1", "close").is_err());
/// ```
pub fn resource_sub_url(
    api_base: &str,
    resource_path: &str,
    id: &str,
    action: &str,
) -> Result<String, StripeError> {
    let encoded = validate_and_encode_id(id)?;
    let base = api_base.trim_end_matches('/');
    let path = resource_path.trim_start_matches('/');
    let act = action.trim_matches('/');
    Ok(format!("{base}/{path}/{encoded}/{act}"))
}

/// Trait implemented by endpoints that support retrieving resources by ID.
///
/// Types implementing `Retrievable` provide an asynchronous lookup by their unique Stripe
/// resource identifier (such as `dp_...`, `re_...`, or `acct_...`).
///
/// # Example
///
/// ```
/// use async_trait::async_trait;
/// use some_stripe_core::patterns::crud::Retrievable;
/// use some_stripe_core::{StripeClient, StripeError};
///
/// struct MyResource {
///     pub id: String,
/// }
///
/// #[async_trait]
/// impl Retrievable for MyResource {
///     async fn retrieve(_client: &StripeClient, id: &str) -> Result<Self, StripeError> {
///         Ok(MyResource { id: id.to_string() })
///     }
/// }
/// ```
#[async_trait]
pub trait Retrievable: Sized {
    /// Retrieve a resource by its unique identifier.
    async fn retrieve(client: &StripeClient, id: &str) -> Result<Self, StripeError>;
}

/// Trait implemented by endpoints that support creating resources from parameters.
///
/// Generic over the parameter DTO `Params`, which must implement [`Serialize`] and be safe
/// to transmit across threads (`Send + Sync`).
///
/// # Example
///
/// ```
/// use async_trait::async_trait;
/// use serde::Serialize;
/// use some_stripe_core::patterns::crud::Creatable;
/// use some_stripe_core::{StripeClient, StripeError};
///
/// #[derive(Serialize)]
/// struct CreateWidgetParams {
///     name: String,
/// }
///
/// struct Widget {
///     id: String,
///     name: String,
/// }
///
/// #[async_trait]
/// impl Creatable<CreateWidgetParams> for Widget {
///     async fn create(_client: &StripeClient, params: &CreateWidgetParams) -> Result<Self, StripeError> {
///         Ok(Widget {
///             id: "wid_123".to_string(),
///             name: params.name.clone(),
///         })
///     }
/// }
/// ```
#[async_trait]
pub trait Creatable<Params: Serialize + Send + Sync>: Sized {
    /// Create a new resource with the specified parameters.
    async fn create(client: &StripeClient, params: &Params) -> Result<Self, StripeError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_and_encode_id_valid() {
        assert_eq!(validate_and_encode_id("dp_12345").unwrap(), "dp_12345");
        assert_eq!(
            validate_and_encode_id("txn_abc-def.xyz").unwrap(),
            "txn_abc-def.xyz"
        );
    }

    #[test]
    fn test_validate_and_encode_id_empty_and_whitespace() {
        let err1 = validate_and_encode_id("").unwrap_err();
        match err1 {
            StripeError::InvalidRequest(msg) => {
                assert_eq!(msg, "Resource ID cannot be empty or whitespace");
            }
            _ => panic!("Expected InvalidRequest, got {:?}", err1),
        }

        let err2 = validate_and_encode_id("   ").unwrap_err();
        match err2 {
            StripeError::InvalidRequest(msg) => {
                assert_eq!(msg, "Resource ID cannot be empty or whitespace");
            }
            _ => panic!("Expected InvalidRequest, got {:?}", err2),
        }
    }

    #[test]
    fn test_validate_and_encode_id_path_traversal() {
        assert!(matches!(
            validate_and_encode_id("../../account"),
            Err(StripeError::InvalidRequest(_))
        ));
        assert!(matches!(
            validate_and_encode_id("/etc/passwd"),
            Err(StripeError::InvalidRequest(_))
        ));
        assert!(matches!(
            validate_and_encode_id("..\\windows\\system32"),
            Err(StripeError::InvalidRequest(_))
        ));
    }

    #[test]
    fn test_validate_and_encode_id_query_and_fragment_injection() {
        assert!(matches!(
            validate_and_encode_id("dp_123?inject=1"),
            Err(StripeError::InvalidRequest(_))
        ));
        assert!(matches!(
            validate_and_encode_id("dp_123#frag"),
            Err(StripeError::InvalidRequest(_))
        ));
    }

    #[test]
    fn test_validate_and_encode_id_percent_encodes_special_chars() {
        let encoded = validate_and_encode_id("dp:123@456").unwrap();
        assert_eq!(encoded, "dp%3A123%40456");
    }

    #[test]
    fn test_resource_url_constructors() {
        let url = resource_url("https://api.stripe.com/", "/v1/charges/");
        assert_eq!(url, "https://api.stripe.com/v1/charges");

        let item_url =
            resource_url_with_id("https://api.stripe.com/", "/v1/charges", "ch_123").unwrap();
        assert_eq!(item_url, "https://api.stripe.com/v1/charges/ch_123");

        let sub_url = resource_sub_url(
            "https://api.stripe.com/",
            "/v1/charges",
            "ch_123",
            "/refund",
        )
        .unwrap();
        assert_eq!(sub_url, "https://api.stripe.com/v1/charges/ch_123/refund");
    }
}
