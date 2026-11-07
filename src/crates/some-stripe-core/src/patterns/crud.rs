//! Generic CRUD endpoint helpers and traits.

use crate::client::StripeClient;
use crate::error::StripeError;
use crate::patterns::serializer::percent_encode;
use async_trait::async_trait;
use serde::Serialize;

/// Validates that a resource ID is non-empty, safe from path traversal, and returns its percent-encoded representation.
///
/// Stripe resource IDs contain alphanumeric characters and underscores or colons (e.g., `cus_123`, `pi_3Mtw...`).
/// This function verifies that the ID does not contain directory traversal sequences, URL control characters,
/// or ASCII control characters, preventing path traversal and SSRF attacks when constructing REST URLs.
///
/// # Errors
/// Returns [`StripeError::InvalidRequest`] if the ID:
/// - Is empty or contains only whitespace.
/// - Contains ASCII control characters.
/// - Contains directory traversal sequences (`..`).
/// - Contains path separators (`/`, `\`).
/// - Contains URL control characters or query/fragment delimiters (`?`, `#`).
///
/// # Examples
/// ```rust
/// use some_stripe_core::patterns::crud::validate_and_encode_id;
///
/// let safe_id = validate_and_encode_id("cus_12345").unwrap();
/// assert_eq!(safe_id, "cus_12345");
///
/// let encoded = validate_and_encode_id("cus:tenant_1").unwrap();
/// assert_eq!(encoded, "cus%3Atenant_1");
///
/// // Path traversal, control characters, or URL delimiter attempts are strictly rejected:
/// assert!(validate_and_encode_id("../admin").is_err());
/// assert!(validate_and_encode_id("cus_123?action=delete").is_err());
/// assert!(validate_and_encode_id("   ").is_err());
/// assert!(validate_and_encode_id("cus_123\0evil").is_err());
/// ```
pub fn validate_and_encode_id(id: &str) -> Result<String, StripeError> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return Err(StripeError::InvalidRequest(
            "Resource ID cannot be empty or whitespace".into(),
        ));
    }

    if trimmed.chars().any(|c| c.is_ascii_control()) {
        return Err(StripeError::InvalidRequest(
            "ID contains control characters".into(),
        ));
    }

    if trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains('?')
        || trimmed.contains('#')
    {
        return Err(StripeError::InvalidRequest(format!(
            "Invalid resource ID '{id}': path traversal and URL control characters are not permitted"
        )));
    }

    Ok(percent_encode(trimmed))
}

/// Assembles a normalized Stripe REST API endpoint URL.
pub fn resource_url(api_base: &str, resource_path: &str) -> String {
    let base = api_base.trim_end_matches('/');
    let path = resource_path.trim_start_matches('/');
    format!("{base}/{path}")
}

/// Assembles a REST API endpoint URL for an individual resource item by ID with strict validation.
///
/// Ensures the resource ID does not contain path traversal sequences (`..`), slashes, control characters, or URL delimiters.
///
/// # Errors
/// Returns [`StripeError::InvalidRequest`] if `id` fails [`validate_and_encode_id`].
///
/// # Examples
/// ```rust
/// use some_stripe_core::patterns::crud::resource_url_with_id;
///
/// let url = resource_url_with_id("https://api.stripe.com", "/v1/customers", "cus_123").unwrap();
/// assert_eq!(url, "https://api.stripe.com/v1/customers/cus_123");
///
/// // Rejects invalid IDs with path traversal:
/// assert!(resource_url_with_id("https://api.stripe.com", "/v1/customers", "../admin").is_err());
/// ```
pub fn resource_url_with_id(
    api_base: &str,
    resource_path: &str,
    id: &str,
) -> Result<String, StripeError> {
    let base = api_base.trim_end_matches('/');
    let path = resource_path.trim_start_matches('/');
    let safe_id = validate_and_encode_id(id)?;
    Ok(format!("{base}/{path}/{safe_id}"))
}

/// Assembles a REST API endpoint URL for an individual resource item by ID with strict validation.
///
/// Alias / delegator for [`resource_url_with_id`].
///
/// # Errors
/// Returns [`StripeError::InvalidRequest`] if `id` fails [`validate_and_encode_id`].
///
/// # Examples
/// ```rust
/// use some_stripe_core::patterns::crud::try_resource_url_with_id;
///
/// let url = try_resource_url_with_id("https://api.stripe.com", "/v1/customers", "cus_123").unwrap();
/// assert_eq!(url, "https://api.stripe.com/v1/customers/cus_123");
///
/// // Rejects invalid IDs with path traversal:
/// assert!(try_resource_url_with_id("https://api.stripe.com", "/v1/customers", "../admin").is_err());
/// ```
pub fn try_resource_url_with_id(
    api_base: &str,
    resource_path: &str,
    id: &str,
) -> Result<String, StripeError> {
    resource_url_with_id(api_base, resource_path, id)
}

/// Assembles a sub-resource endpoint URL with strict validation.
///
/// Validates the parent resource ID to prevent path traversal or URL control injection attacks.
///
/// # Errors
/// Returns [`StripeError::InvalidRequest`] if `id` fails [`validate_and_encode_id`].
///
/// # Examples
/// ```rust
/// use some_stripe_core::patterns::crud::resource_sub_url;
///
/// let url = resource_sub_url("https://api.stripe.com", "/v1/customers", "cus_123", "subscriptions").unwrap();
/// assert_eq!(url, "https://api.stripe.com/v1/customers/cus_123/subscriptions");
///
/// // Strict validation blocks path traversal tokens in ID:
/// assert!(resource_sub_url("https://api.stripe.com", "/v1/customers", "../admin", "cancel").is_err());
/// ```
pub fn resource_sub_url(
    api_base: &str,
    resource_path: &str,
    id: &str,
    action: &str,
) -> Result<String, StripeError> {
    let base = api_base.trim_end_matches('/');
    let path = resource_path.trim_start_matches('/');
    let safe_id = validate_and_encode_id(id)?;
    let action = action.trim_start_matches('/');
    Ok(format!("{base}/{path}/{safe_id}/{action}"))
}

/// Assembles a sub-resource endpoint URL with strict validation.
///
/// Alias / delegator for [`resource_sub_url`].
///
/// # Errors
/// Returns [`StripeError::InvalidRequest`] if `id` fails [`validate_and_encode_id`].
///
/// # Examples
/// ```rust
/// use some_stripe_core::patterns::crud::try_resource_sub_url;
///
/// let url = try_resource_sub_url("https://api.stripe.com", "/v1/customers", "cus_123", "subscriptions").unwrap();
/// assert_eq!(url, "https://api.stripe.com/v1/customers/cus_123/subscriptions");
///
/// // Strict validation blocks path traversal tokens in ID:
/// assert!(try_resource_sub_url("https://api.stripe.com", "/v1/customers", "../admin", "cancel").is_err());
/// ```
pub fn try_resource_sub_url(
    api_base: &str,
    resource_path: &str,
    id: &str,
    action: &str,
) -> Result<String, StripeError> {
    resource_sub_url(api_base, resource_path, id, action)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_and_encode_id_success() {
        assert_eq!(validate_and_encode_id("cus_12345").unwrap(), "cus_12345");
        assert_eq!(validate_and_encode_id(" cus_12345 ").unwrap(), "cus_12345");
        assert_eq!(validate_and_encode_id("cus:abc").unwrap(), "cus%3Aabc");
    }

    #[test]
    fn test_validate_and_encode_id_rejects_empty_whitespace() {
        assert!(matches!(
            validate_and_encode_id(""),
            Err(StripeError::InvalidRequest(_))
        ));
        assert!(matches!(
            validate_and_encode_id("   "),
            Err(StripeError::InvalidRequest(_))
        ));
    }

    #[test]
    fn test_validate_and_encode_id_rejects_path_traversal() {
        assert!(validate_and_encode_id("../cus_123").is_err());
        assert!(validate_and_encode_id("cus/123").is_err());
        assert!(validate_and_encode_id("cus\\123").is_err());
        assert!(validate_and_encode_id("cus_123?foo=bar").is_err());
        assert!(validate_and_encode_id("cus_123#frag").is_err());
    }

    #[test]
    fn test_validate_and_encode_id_rejects_control_characters() {
        let err = validate_and_encode_id("cus_\0evil").unwrap_err();
        assert!(
            matches!(err, StripeError::InvalidRequest(ref msg) if msg == "ID contains control characters")
        );

        let err2 = validate_and_encode_id("cus_\x07bell").unwrap_err();
        assert!(
            matches!(err2, StripeError::InvalidRequest(ref msg) if msg == "ID contains control characters")
        );

        let err3 = validate_and_encode_id("cus_\x1funit").unwrap_err();
        assert!(
            matches!(err3, StripeError::InvalidRequest(ref msg) if msg == "ID contains control characters")
        );

        let err4 = validate_and_encode_id("cus_\r\ninjection").unwrap_err();
        assert!(
            matches!(err4, StripeError::InvalidRequest(ref msg) if msg == "ID contains control characters")
        );
    }

    #[test]
    fn test_resource_url_with_id() {
        let url =
            resource_url_with_id("https://api.stripe.com", "/v1/customers", "cus_123").unwrap();
        assert_eq!(url, "https://api.stripe.com/v1/customers/cus_123");

        let err = resource_url_with_id("https://api.stripe.com", "/v1/customers", "../admin");
        assert!(err.is_err());

        let err_empty = resource_url_with_id("https://api.stripe.com", "/v1/customers", "   ");
        assert!(err_empty.is_err());

        let err_ctrl =
            resource_url_with_id("https://api.stripe.com", "/v1/customers", "cus_123\0null");
        assert!(err_ctrl.is_err());
    }

    #[test]
    fn test_try_resource_url_with_id() {
        let url =
            try_resource_url_with_id("https://api.stripe.com", "/v1/customers", "cus_123").unwrap();
        assert_eq!(url, "https://api.stripe.com/v1/customers/cus_123");

        let err = try_resource_url_with_id("https://api.stripe.com", "/v1/customers", "../admin");
        assert!(err.is_err());
    }

    #[test]
    fn test_resource_sub_url() {
        let url = resource_sub_url(
            "https://api.stripe.com",
            "/v1/customers",
            "cus_123",
            "subscriptions",
        )
        .unwrap();
        assert_eq!(
            url,
            "https://api.stripe.com/v1/customers/cus_123/subscriptions"
        );

        let err = resource_sub_url(
            "https://api.stripe.com",
            "/v1/customers",
            "../admin",
            "subscriptions",
        );
        assert!(err.is_err());

        let err_empty = resource_sub_url(
            "https://api.stripe.com",
            "/v1/customers",
            "",
            "subscriptions",
        );
        assert!(err_empty.is_err());

        let err_ctrl = resource_sub_url(
            "https://api.stripe.com",
            "/v1/customers",
            "cus_123\x08backspace",
            "subscriptions",
        );
        assert!(err_ctrl.is_err());
    }

    #[test]
    fn test_try_resource_sub_url() {
        let url = try_resource_sub_url(
            "https://api.stripe.com",
            "/v1/customers",
            "cus_123",
            "subscriptions",
        )
        .unwrap();
        assert_eq!(
            url,
            "https://api.stripe.com/v1/customers/cus_123/subscriptions"
        );

        let err = try_resource_sub_url(
            "https://api.stripe.com",
            "/v1/customers",
            "../admin",
            "subscriptions",
        );
        assert!(err.is_err());
    }
}
