//! # Stripe Customer Billing Portal Types & Self-Serve Sessions
//!
//! Strongly typed domain models, session parameters, portal configurations, and security guards
//! for Stripe Customer Billing Portal, enforcing open redirect prevention and short-lived session expiry.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validates customer billing portal return URLs against open redirect vulnerabilities (Gate G2).
///
/// Disallows non-HTTPS protocols, javascript URIs, and malformed URLs containing CRLF characters.
///
/// # Errors
/// Returns an error message if the URL does not start with `https://` or contains malicious characters.
///
/// # Example
/// ```
/// use some_stripe_types::validate_portal_url;
///
/// assert!(validate_portal_url("https://example.com/account").is_ok());
/// assert!(validate_portal_url("http://insecure.com").is_err());
/// assert!(validate_portal_url("javascript:alert(1)").is_err());
/// assert!(validate_portal_url("https://example.com/foo\r\nbar").is_err());
/// ```
pub fn validate_portal_url(url: &str) -> Result<(), &'static str> {
    if !url.starts_with("https://") {
        return Err("Portal return URL must strictly use HTTPS protocol");
    }
    if url.contains("javascript:") || url.contains('\r') || url.contains('\n') {
        return Err("Portal return URL contains invalid or malicious characters");
    }
    Ok(())
}

/// Checks if a billing portal session has expired based on standard Stripe TTL (default 300 seconds).
///
/// # Example
/// ```
/// use some_stripe_types::check_portal_session_expiry;
///
/// let created = 1_000_000;
/// assert!(!check_portal_session_expiry(created, 1_000_100)); // 100s later: not expired
/// assert!(!check_portal_session_expiry(created, 1_000_299)); // 299s later: not expired
/// assert!(check_portal_session_expiry(created, 1_000_300));  // 300s later: expired
/// assert!(check_portal_session_expiry(created, 1_000_500));  // 500s later: expired
/// ```
#[must_use]
pub fn check_portal_session_expiry(created: i64, current_timestamp: i64) -> bool {
    // Portal sessions typically expire 5 minutes (300 seconds) after creation if unused
    current_timestamp >= created + 300
}

/// Represents a Customer Billing Portal Session (`billing_portal.session`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortalSession {
    /// Unique identifier for the session (`bps_...`).
    pub id: String,

    /// Object type, always `billing_portal.session`.
    #[serde(default = "default_portal_session_object")]
    pub object: String,

    /// The configuration used by this session (`bpc_...`), if customized.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration: Option<String>,

    /// Time at which the object was created (UNIX epoch seconds).
    pub created: i64,

    /// The ID of the customer for this session (`cus_...`).
    pub customer: String,

    /// Has the value `true` if live mode or `false` if test mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub livemode: Option<bool>,

    /// The account on behalf of which this session was created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_behalf_of: Option<String>,

    /// The URL to which the customer is redirected after managing billing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_url: Option<String>,

    /// The short-lived URL to which the customer is redirected to enter the portal.
    pub url: String,
}

impl PortalSession {
    /// Checks whether this session has expired at the given timestamp.
    #[must_use]
    pub fn is_expired(&self, current_timestamp: i64) -> bool {
        check_portal_session_expiry(self.created, current_timestamp)
    }
}

fn default_portal_session_object() -> String {
    "billing_portal.session".into()
}

/// Parameters for creating a new [`PortalSession`] (`POST /v1/billing_portal/sessions`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePortalSessionParams {
    /// The ID of the customer (`cus_...`).
    pub customer: String,

    /// The ID of an existing portal configuration (`bpc_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration: Option<String>,

    /// The account on behalf of which to create the session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_behalf_of: Option<String>,

    /// The default URL to return the customer to after exiting the portal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_url: Option<String>,

    /// Unique idempotency key for safe execution (Gate G2).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreatePortalSessionParams {
    /// Constructs parameters for a specified customer ID.
    #[must_use]
    pub fn for_customer(customer: impl Into<String>) -> Self {
        Self {
            customer: customer.into(),
            ..Default::default()
        }
    }

    /// Sets the portal configuration ID (`bpc_...`).
    #[must_use]
    pub fn with_configuration(mut self, config: impl Into<String>) -> Self {
        self.configuration = Some(config.into());
        self
    }

    /// Sets the return URL after the customer exits the portal.
    #[must_use]
    pub fn with_return_url(mut self, url: impl Into<String>) -> Self {
        self.return_url = Some(url.into());
        self
    }

    /// Sets the connected account on behalf of which to create the session.
    #[must_use]
    pub fn with_on_behalf_of(mut self, account: impl Into<String>) -> Self {
        self.on_behalf_of = Some(account.into());
        self
    }

    /// Sets an explicit idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Business profile settings within a [`PortalConfiguration`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortalBusinessProfile {
    /// Display headline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headline: Option<String>,

    /// Public URL of the privacy policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_policy_url: Option<String>,

    /// Public URL of the terms of service.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terms_of_service_url: Option<String>,
}

impl PortalBusinessProfile {
    /// Creates a new empty `PortalBusinessProfile`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the business profile headline.
    #[must_use]
    pub fn with_headline(mut self, headline: impl Into<String>) -> Self {
        self.headline = Some(headline.into());
        self
    }

    /// Sets the privacy policy URL.
    #[must_use]
    pub fn with_privacy_policy_url(mut self, url: impl Into<String>) -> Self {
        self.privacy_policy_url = Some(url.into());
        self
    }

    /// Sets the terms of service URL.
    #[must_use]
    pub fn with_terms_of_service_url(mut self, url: impl Into<String>) -> Self {
        self.terms_of_service_url = Some(url.into());
        self
    }
}

/// Feature flags enabled for self-service within a [`PortalConfiguration`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortalFeatures {
    /// Customer details update permissions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer_update: Option<PortalFeatureEnabled>,

    /// Invoice history viewing permissions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice_history: Option<PortalFeatureEnabled>,

    /// Payment method update permissions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_method_update: Option<PortalFeatureEnabled>,

    /// Subscription cancellation permissions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription_cancel: Option<PortalFeatureEnabled>,
}

impl PortalFeatures {
    /// Creates a new empty `PortalFeatures`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables or disables customer update feature.
    #[must_use]
    pub fn with_customer_update(mut self, enabled: bool) -> Self {
        self.customer_update = Some(PortalFeatureEnabled::new(enabled));
        self
    }

    /// Enables or disables invoice history feature.
    #[must_use]
    pub fn with_invoice_history(mut self, enabled: bool) -> Self {
        self.invoice_history = Some(PortalFeatureEnabled::new(enabled));
        self
    }

    /// Enables or disables payment method update feature.
    #[must_use]
    pub fn with_payment_method_update(mut self, enabled: bool) -> Self {
        self.payment_method_update = Some(PortalFeatureEnabled::new(enabled));
        self
    }

    /// Enables or disables subscription cancellation feature.
    #[must_use]
    pub fn with_subscription_cancel(mut self, enabled: bool) -> Self {
        self.subscription_cancel = Some(PortalFeatureEnabled::new(enabled));
        self
    }
}

/// Generic feature flag structure.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortalFeatureEnabled {
    /// Whether the feature is enabled.
    pub enabled: bool,
}

impl PortalFeatureEnabled {
    /// Creates a new `PortalFeatureEnabled` with the given state.
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

/// Represents a Stripe Customer Billing Portal Configuration (`billing_portal.configuration`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortalConfiguration {
    /// Unique identifier for the configuration (`bpc_...`).
    pub id: String,

    /// Object type, always `billing_portal.configuration`.
    #[serde(default = "default_portal_config_object")]
    pub object: String,

    /// Whether this configuration is currently active.
    pub active: bool,

    /// Business profile information displayed to customer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_profile: Option<PortalBusinessProfile>,

    /// Features enabled in this configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<PortalFeatures>,

    /// The default URL to return the customer to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_return_url: Option<String>,

    /// Whether this is the default configuration for the account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,

    /// Time at which the object was created (UNIX epoch seconds).
    pub created: i64,

    /// Key-value metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl PortalConfiguration {
    /// Returns whether this configuration is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }
}

fn default_portal_config_object() -> String {
    "billing_portal.configuration".into()
}

/// Parameters for creating a [`PortalConfiguration`] (`POST /v1/billing_portal/configurations`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePortalConfigurationParams {
    /// Business profile information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_profile: Option<PortalBusinessProfile>,

    /// The default URL to return customer to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_return_url: Option<String>,

    /// Information about feature flags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<PortalFeatures>,

    /// Set of key-value pairs for metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Unique idempotency key.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreatePortalConfigurationParams {
    /// Creates a new empty `CreatePortalConfigurationParams`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the default return URL.
    #[must_use]
    pub fn with_default_return_url(mut self, url: impl Into<String>) -> Self {
        self.default_return_url = Some(url.into());
        self
    }

    /// Sets the business profile.
    #[must_use]
    pub fn with_business_profile(mut self, profile: PortalBusinessProfile) -> Self {
        self.business_profile = Some(profile);
        self
    }

    /// Sets the features configuration.
    #[must_use]
    pub fn with_features(mut self, features: PortalFeatures) -> Self {
        self.features = Some(features);
        self
    }

    /// Sets metadata key-value pairs.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets an explicit idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for updating a [`PortalConfiguration`] (`POST /v1/billing_portal/configurations/{id}`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatePortalConfigurationParams {
    /// Whether the configuration is active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,

    /// Business profile information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_profile: Option<PortalBusinessProfile>,

    /// The default URL to return customer to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_return_url: Option<String>,

    /// Information about feature flags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<PortalFeatures>,

    /// Set of key-value pairs for metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Unique idempotency key.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl UpdatePortalConfigurationParams {
    /// Creates a new empty `UpdatePortalConfigurationParams`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether the configuration is active.
    #[must_use]
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }

    /// Sets the business profile.
    #[must_use]
    pub fn with_business_profile(mut self, profile: PortalBusinessProfile) -> Self {
        self.business_profile = Some(profile);
        self
    }

    /// Sets the default return URL.
    #[must_use]
    pub fn with_default_return_url(mut self, url: impl Into<String>) -> Self {
        self.default_return_url = Some(url.into());
        self
    }

    /// Sets the features configuration.
    #[must_use]
    pub fn with_features(mut self, features: PortalFeatures) -> Self {
        self.features = Some(features);
        self
    }

    /// Sets metadata key-value pairs.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets an explicit idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Paginated list of [`PortalConfiguration`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortalConfigurationList {
    /// String representing the object's type, always `list`.
    #[serde(default = "default_list_object")]
    pub object: String,

    /// List of portal configurations.
    #[serde(default)]
    pub data: Vec<PortalConfiguration>,

    /// True if this list has another page of items after this one that can be fetched.
    #[serde(default)]
    pub has_more: bool,

    /// The URL where this list can be accessed.
    #[serde(default)]
    pub url: String,
}

fn default_list_object() -> String {
    "list".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_portal_url_security_rules() {
        // Valid HTTPS urls
        assert!(validate_portal_url("https://example.com").is_ok());
        assert!(validate_portal_url("https://dashboard.stripe.com/billing").is_ok());
        assert!(validate_portal_url("https://app.example.com:8443/settings?tab=billing").is_ok());

        // Insecure HTTP rejected
        assert_eq!(
            validate_portal_url("http://insecure.example.com").unwrap_err(),
            "Portal return URL must strictly use HTTPS protocol"
        );

        // Javascript URI rejected
        assert_eq!(
            validate_portal_url("javascript:alert('xss')").unwrap_err(),
            "Portal return URL must strictly use HTTPS protocol"
        );
        assert_eq!(
            validate_portal_url("https://example.com?cb=javascript:evil()").unwrap_err(),
            "Portal return URL contains invalid or malicious characters"
        );

        // CRLF injection rejected
        assert_eq!(
            validate_portal_url("https://example.com/return\r\nSet-Cookie:bad").unwrap_err(),
            "Portal return URL contains invalid or malicious characters"
        );
        assert_eq!(
            validate_portal_url("https://example.com/return\nLocation:evil").unwrap_err(),
            "Portal return URL contains invalid or malicious characters"
        );
    }

    #[test]
    fn test_check_portal_session_expiry_300s_window() {
        let created = 1_700_000_000;

        // Fresh session (0 to 299 seconds)
        assert!(!check_portal_session_expiry(created, created));
        assert!(!check_portal_session_expiry(created, created + 150));
        assert!(!check_portal_session_expiry(created, created + 299));

        // Expired session (300 seconds and beyond)
        assert!(check_portal_session_expiry(created, created + 300));
        assert!(check_portal_session_expiry(created, created + 301));
        assert!(check_portal_session_expiry(created, created + 3600));
    }

    #[test]
    fn test_portal_session_serde() {
        let json = serde_json::json!({
            "id": "bps_12345",
            "object": "billing_portal.session",
            "configuration": "bpc_conf_999",
            "created": 1700000000,
            "customer": "cus_test_cust",
            "livemode": false,
            "return_url": "https://example.com/account",
            "url": "https://billing.stripe.com/session/bps_12345"
        });

        let session: PortalSession = serde_json::from_value(json).expect("Deserialization failed");
        assert_eq!(session.id, "bps_12345");
        assert_eq!(session.object, "billing_portal.session");
        assert_eq!(session.customer, "cus_test_cust");
        assert_eq!(session.configuration.as_deref(), Some("bpc_conf_999"));
        assert!(!session.is_expired(1700000100));
        assert!(session.is_expired(1700000400));
    }

    #[test]
    fn test_portal_configuration_serde() {
        let json = serde_json::json!({
            "id": "bpc_test_config",
            "object": "billing_portal.configuration",
            "active": true,
            "business_profile": {
                "headline": "Acme Inc Billing",
                "privacy_policy_url": "https://example.com/privacy",
                "terms_of_service_url": "https://example.com/terms"
            },
            "features": {
                "customer_update": { "enabled": true },
                "invoice_history": { "enabled": true },
                "payment_method_update": { "enabled": true },
                "subscription_cancel": { "enabled": false }
            },
            "default_return_url": "https://example.com/dashboard",
            "is_default": true,
            "created": 1700000000,
            "metadata": {
                "env": "production"
            }
        });

        let config: PortalConfiguration =
            serde_json::from_value(json).expect("Deserialization failed");
        assert_eq!(config.id, "bpc_test_config");
        assert!(config.is_active());
        assert_eq!(
            config.default_return_url.as_deref(),
            Some("https://example.com/dashboard")
        );
        let bp = config.business_profile.unwrap();
        assert_eq!(bp.headline.as_deref(), Some("Acme Inc Billing"));
        let feats = config.features.unwrap();
        assert!(feats.customer_update.unwrap().enabled);
        assert!(!feats.subscription_cancel.unwrap().enabled);
    }
}
