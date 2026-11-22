//! Explicit `Stripe-Version` configuration and injectable HTTP transport (US-9.1.0 Unit 1).
//!
//! This module provides strictly instance-scoped transport configuration without global
//! singletons:
//!
//! - [`StripeVersion`] validates and pins the `Stripe-Version` header value, defaulting to
//!   [`DEFAULT_STRIPE_VERSION`] (`"2026-11-18.endive"`).
//! - [`TransportConfig`] holds per-instance credentials, API base URL, timeout, pinned
//!   API version, and an optional caller-supplied [`reqwest::Client`].
//! - Custom [`fmt::Debug`] formatting always redacts the secret API key as `"[REDACTED]"`.
//!
//! # Examples
//!
//! ```
//! use some_stripe_core::SecretKey;
//! use some_stripe_core::version_transport::{
//!     DEFAULT_STRIPE_VERSION, STRIPE_VERSION_HEADER, StripeVersion, TransportConfig,
//! };
//! use std::time::Duration;
//!
//! let config = TransportConfig::new(SecretKey::new("sk_test_example_123"))
//!     .with_stripe_version(StripeVersion::pinned_default())
//!     .with_timeout(Duration::from_secs(15))
//!     .expect("positive timeout is valid");
//!
//! let headers = config
//!     .build_headers(Some("idem_order_42"))
//!     .expect("headers should build");
//!
//! assert_eq!(
//!     headers
//!         .get(STRIPE_VERSION_HEADER)
//!         .and_then(|v| v.to_str().ok()),
//!     Some(DEFAULT_STRIPE_VERSION),
//! );
//! assert!(!format!("{config:?}").contains("sk_test_"));
//! ```

use crate::client::SecretKey;
use crate::error::StripeError;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};
use std::fmt;
use std::time::Duration;
use zeroize::Zeroize;

/// Pinned default Stripe API version sent on every request (`Stripe-Version` header).
pub const DEFAULT_STRIPE_VERSION: &str = "2026-11-18.endive";

/// Canonical HTTP header name for pinning the Stripe API version.
pub const STRIPE_VERSION_HEADER: &str = "Stripe-Version";

/// Default per-request HTTP timeout in seconds.
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Default Stripe API base URL.
pub const DEFAULT_API_BASE: &str = "https://api.stripe.com";

/// Canonical HTTP header name for request idempotency keys.
pub const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";

/// Maximum permitted byte length for a [`StripeVersion`] value.
pub const MAX_STRIPE_VERSION_LEN: usize = 64;

/// Validated Stripe API version string sent via the `Stripe-Version` HTTP header.
///
/// A valid `StripeVersion` is non-empty, at most [`MAX_STRIPE_VERSION_LEN`] (64) bytes,
/// and consists solely of printable ASCII characters without whitespace or control
/// characters (`0x21..=0x7E`).
///
/// # Examples
///
/// ```
/// use some_stripe_core::version_transport::{DEFAULT_STRIPE_VERSION, StripeVersion};
///
/// let default_ver = StripeVersion::pinned_default();
/// assert_eq!(default_ver.as_str(), DEFAULT_STRIPE_VERSION);
///
/// let custom = StripeVersion::try_new("2025-02-24.acacia").expect("valid version");
/// assert_eq!(custom.as_str(), "2025-02-24.acacia");
/// assert!(StripeVersion::try_new("2026-11-18\r\nX-Inject: 1").is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StripeVersion(String);

impl StripeVersion {
    /// Returns the pinned default Stripe API version (`"2026-11-18.endive"`).
    #[must_use]
    pub fn pinned_default() -> Self {
        Self(DEFAULT_STRIPE_VERSION.to_owned())
    }

    /// Validates and constructs a new [`StripeVersion`].
    ///
    /// # Errors
    ///
    /// Returns [`StripeError::Config`] if `version` is empty, exceeds
    /// [`MAX_STRIPE_VERSION_LEN`] bytes, or contains whitespace, control characters,
    /// or non-ASCII bytes.
    pub fn try_new(version: impl Into<String>) -> Result<Self, StripeError> {
        let raw = version.into();
        validate_stripe_version(&raw)?;
        Ok(Self(raw))
    }

    /// Returns the validated version string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for StripeVersion {
    fn default() -> Self {
        Self::pinned_default()
    }
}

impl fmt::Display for StripeVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for StripeVersion {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for StripeVersion {
    type Error = StripeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl TryFrom<String> for StripeVersion {
    type Error = StripeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

/// Strictly instance-scoped HTTP transport configuration for Stripe API calls.
///
/// Every [`TransportConfig`] owns its own credentials, base URL, pinned
/// [`StripeVersion`], timeout, and optional injected [`reqwest::Client`] with zero
/// global singletons or process-wide shared state.
///
/// # Security
///
/// The custom [`fmt::Debug`] implementation always renders `api_key` as `"[REDACTED]"`
/// to prevent accidental credential exposure in logs or diagnostics.
///
/// # Examples
///
/// ```
/// use some_stripe_core::SecretKey;
/// use some_stripe_core::version_transport::TransportConfig;
///
/// let custom_client = reqwest::Client::builder()
///     .build()
///     .expect("reqwest client builds");
///
/// let config = TransportConfig::new(SecretKey::new("sk_test_isolated"))
///     .with_api_base("https://api.stripe.com")
///     .expect("valid https base")
///     .with_http_client(custom_client);
///
/// let _resolved = config
///     .resolve_http_client()
///     .expect("injected client resolves");
/// ```
#[derive(Clone)]
pub struct TransportConfig {
    api_key: SecretKey,
    api_base: String,
    stripe_version: StripeVersion,
    timeout: Duration,
    http_client: Option<reqwest::Client>,
}

impl TransportConfig {
    /// Creates a new instance-scoped [`TransportConfig`] with pinned defaults.
    ///
    /// Defaults:
    /// - `api_base`: `"https://api.stripe.com"`
    /// - `stripe_version`: [`StripeVersion::pinned_default()`] (`"2026-11-18.endive"`)
    /// - `timeout`: [`DEFAULT_TIMEOUT_SECS`] seconds (30s)
    /// - `http_client`: `None` (resolved on demand via [`Self::resolve_http_client`])
    #[must_use]
    pub fn new(api_key: SecretKey) -> Self {
        Self {
            api_key,
            api_base: DEFAULT_API_BASE.to_owned(),
            stripe_version: StripeVersion::pinned_default(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            http_client: None,
        }
    }

    /// Overrides the pinned [`StripeVersion`] sent in the `Stripe-Version` header.
    #[must_use]
    pub fn with_stripe_version(mut self, version: StripeVersion) -> Self {
        self.stripe_version = version;
        self
    }

    /// Validates and overrides the Stripe API base URL.
    ///
    /// Only `http://` and `https://` URLs with a non-empty host and no whitespace or
    /// control characters are accepted. Trailing slashes are normalized away.
    ///
    /// # Errors
    ///
    /// Returns [`StripeError::Config`] if `api_base` does not start with `http://` or
    /// `https://`, lacks a host, or contains whitespace/control/non-ASCII characters.
    pub fn with_api_base(mut self, api_base: impl Into<String>) -> Result<Self, StripeError> {
        let raw = api_base.into();
        self.api_base = validate_api_base(&raw)?;
        Ok(self)
    }

    /// Validates and overrides the per-request timeout duration.
    ///
    /// # Errors
    ///
    /// Returns [`StripeError::Config`] if `timeout` is zero.
    pub fn with_timeout(mut self, timeout: Duration) -> Result<Self, StripeError> {
        if timeout.is_zero() {
            return Err(StripeError::Config(
                "Transport timeout must be greater than zero".to_owned(),
            ));
        }
        self.timeout = timeout;
        Ok(self)
    }

    /// Injects a caller-managed [`reqwest::Client`] into this transport instance.
    ///
    /// Use this to supply custom TLS roots, proxies, connection pool settings, or
    /// isolated test transports without touching global state.
    #[must_use]
    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Returns a reference to the configured [`SecretKey`].
    #[must_use]
    pub fn api_key(&self) -> &SecretKey {
        &self.api_key
    }

    /// Returns the validated Stripe API base URL.
    #[must_use]
    pub fn api_base(&self) -> &str {
        &self.api_base
    }

    /// Returns the pinned [`StripeVersion`].
    #[must_use]
    pub fn stripe_version(&self) -> &StripeVersion {
        &self.stripe_version
    }

    /// Returns the configured request timeout.
    #[must_use]
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Returns a reference to the injected [`reqwest::Client`], if one was provided.
    #[must_use]
    pub fn http_client(&self) -> Option<&reqwest::Client> {
        self.http_client.as_ref()
    }

    /// Resolves the [`reqwest::Client`] for this instance.
    ///
    /// Returns a cheap clone of the injected [`reqwest::Client`] when configured via
    /// [`Self::with_http_client`]; otherwise constructs a fresh instance-scoped client
    /// configured with [`Self::timeout`] and [`reqwest::redirect::Policy::none`].
    ///
    /// # Errors
    ///
    /// Returns [`StripeError::Http`] if building the default [`reqwest::Client`] fails.
    pub fn resolve_http_client(&self) -> Result<reqwest::Client, StripeError> {
        if let Some(client) = &self.http_client {
            return Ok(client.clone());
        }
        reqwest::Client::builder()
            .timeout(self.timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(StripeError::Http)
    }

    /// Builds the HTTP headers for a Stripe API request.
    ///
    /// Always sets:
    /// - `Authorization: Bearer <api_key>` (marked sensitive)
    /// - `Stripe-Version: <stripe_version>`
    ///
    /// Optionally sets `Idempotency-Key` when `idempotency_key` is `Some(...)`.
    ///
    /// # Errors
    ///
    /// Returns [`StripeError::Config`] if the API key is empty or contains invalid HTTP
    /// header characters, or [`StripeError::InvalidRequest`] if `idempotency_key` is
    /// empty/whitespace-only or contains invalid header characters.
    pub fn build_headers(&self, idempotency_key: Option<&str>) -> Result<HeaderMap, StripeError> {
        let mut headers = HeaderMap::with_capacity(3);
        let auth_value = build_auth_header(&self.api_key)?;
        let version_value = HeaderValue::from_str(self.stripe_version.as_str()).map_err(|_| {
            StripeError::Config("Stripe-Version contains invalid HTTP header bytes".to_owned())
        })?;
        headers.insert(AUTHORIZATION, auth_value);
        headers.insert(HeaderName::from_static("stripe-version"), version_value);
        insert_idempotency_header(&mut headers, idempotency_key)?;
        Ok(headers)
    }
}

impl fmt::Debug for TransportConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransportConfig")
            .field("api_key", &"[REDACTED]")
            .field("api_base", &self.api_base)
            .field("stripe_version", &self.stripe_version)
            .field("timeout", &self.timeout)
            .field("has_custom_http_client", &self.http_client.is_some())
            .finish()
    }
}

fn validate_stripe_version(raw: &str) -> Result<(), StripeError> {
    if raw.is_empty() {
        return Err(StripeError::Config(
            "Stripe-Version must not be empty".to_owned(),
        ));
    }
    if raw.len() > MAX_STRIPE_VERSION_LEN {
        return Err(StripeError::Config(format!(
            "Stripe-Version exceeds maximum length of {MAX_STRIPE_VERSION_LEN} bytes"
        )));
    }
    if !raw.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(StripeError::Config(
            "Stripe-Version must contain only printable ASCII without whitespace or control chars"
                .to_owned(),
        ));
    }
    Ok(())
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1")
}

fn validate_url_components(url: &reqwest::Url) -> Result<(), StripeError> {
    if !url.username().is_empty() || url.password().is_some() {
        return Err(StripeError::Config(
            "API base URL must not contain userinfo credentials".to_owned(),
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(StripeError::Config(
            "API base URL must not contain query parameters or fragments".to_owned(),
        ));
    }
    Ok(())
}

fn validate_url_scheme_and_host(url: &reqwest::Url) -> Result<(), StripeError> {
    let host = url.host_str().ok_or_else(|| {
        StripeError::Config("API base URL must include a non-empty host".to_owned())
    })?;
    let scheme = url.scheme();
    if scheme == "https" || (scheme == "http" && is_loopback_host(host)) {
        return Ok(());
    }
    Err(StripeError::Config(
        "API base URL must use 'https://' (plain 'http://' is permitted only for loopback hosts)"
            .to_owned(),
    ))
}

/// Validates and normalizes a Stripe API base URL.
///
/// Requires printable graphic ASCII characters without backslashes, valid URL
/// syntax with a host, no userinfo/query/fragment components, and `https://`
/// (plain `http://` is permitted only for loopback hosts: `localhost`,
/// `127.0.0.1`, `[::1]`, or `::1`).
///
/// # Errors
///
/// Returns [`StripeError::Config`] if `raw` fails any URL or transport security
/// check.
pub fn validate_api_base(raw: &str) -> Result<String, StripeError> {
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_graphic()) || raw.contains('\\') {
        return Err(StripeError::Config(
            "API base URL must contain only printable ASCII without whitespace, control chars, or backslashes"
                .to_owned(),
        ));
    }
    let url = reqwest::Url::parse(raw)
        .map_err(|err| StripeError::Config(format!("API base URL is not a valid URL: {err}")))?;
    validate_url_components(&url)?;
    validate_url_scheme_and_host(&url)?;
    Ok(raw.trim_end_matches('/').to_owned())
}

fn build_auth_header(api_key: &SecretKey) -> Result<HeaderValue, StripeError> {
    let raw = api_key.expose_secret();
    if raw.is_empty() {
        return Err(StripeError::Config(
            "Stripe API key must not be empty".to_owned(),
        ));
    }
    let mut bearer = format!("Bearer {raw}");
    let parsed = HeaderValue::from_str(&bearer).map_err(|_| {
        StripeError::Config("Stripe API key contains invalid HTTP header characters".to_owned())
    });
    bearer.zeroize();
    let mut value = parsed?;
    value.set_sensitive(true);
    Ok(value)
}

fn insert_idempotency_header(
    headers: &mut HeaderMap,
    idempotency_key: Option<&str>,
) -> Result<(), StripeError> {
    let Some(raw_key) = idempotency_key else {
        return Ok(());
    };
    let validated = crate::idempotency_passthrough::IdempotencyKey::try_new(raw_key)?;
    let value = HeaderValue::from_str(validated.as_str()).map_err(|_| {
        StripeError::InvalidRequest(
            "Idempotency-Key contains invalid HTTP header characters".to_owned(),
        )
    })?;
    headers.insert(HeaderName::from_static("idempotency-key"), value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_header_eq(headers: &HeaderMap, name: &str, expected: &str) {
        assert_eq!(
            headers.get(name).and_then(|v| v.to_str().ok()),
            Some(expected),
        );
    }

    fn assert_debug_redacted(rendered: &str, secret: &str) {
        assert!(rendered.contains("[REDACTED]"));
        for forbidden in [secret, "sk_test_", "sk_live_"] {
            assert!(!rendered.contains(forbidden));
        }
    }

    fn build_isolated_configs() -> (TransportConfig, TransportConfig) {
        let client_a = reqwest::Client::builder().build().expect("client a");
        let client_b = reqwest::Client::builder().build().expect("client b");

        let cfg_a = TransportConfig::new(SecretKey::new("sk_test_tenant_alpha"))
            .with_api_base("https://api.stripe.com/")
            .expect("valid https base")
            .with_stripe_version(StripeVersion::try_new("2026-11-18.endive").expect("ver a"))
            .with_timeout(Duration::from_secs(10))
            .expect("valid timeout a")
            .with_http_client(client_a);

        let live_beta = format!("{}{}", "sk_live", "_tenant_beta");
        let cfg_b = TransportConfig::new(SecretKey::new(live_beta))
            .with_api_base("http://127.0.0.1:12111")
            .expect("valid http local mock base")
            .with_stripe_version(StripeVersion::try_new("2025-02-24.acacia").expect("ver b"))
            .with_timeout(Duration::from_secs(45))
            .expect("valid timeout b")
            .with_http_client(client_b);

        (cfg_a, cfg_b)
    }

    #[test]
    fn tc_9_1_1_default_constants() {
        assert_eq!(DEFAULT_STRIPE_VERSION, "2026-11-18.endive");
        assert_eq!(STRIPE_VERSION_HEADER, "Stripe-Version");
        assert_eq!(DEFAULT_TIMEOUT_SECS, 30);
    }

    #[test]
    fn tc_9_1_1_pinned_default_version_traits() {
        let version = StripeVersion::pinned_default();
        assert_eq!(version.as_str(), "2026-11-18.endive");
        assert_eq!(version.to_string(), "2026-11-18.endive");
        assert_eq!(version.as_ref(), "2026-11-18.endive");
        assert_eq!(StripeVersion::default(), version);
    }

    #[test]
    fn tc_9_1_1_stripe_version_validation_accepts_valid_and_boundary() {
        let pinned = StripeVersion::try_new("2026-11-18.endive").expect("valid pinned version");
        assert_eq!(pinned.as_str(), "2026-11-18.endive");

        let exact_max = "a".repeat(MAX_STRIPE_VERSION_LEN);
        let bounded = StripeVersion::try_new(exact_max.clone()).expect("64-byte boundary valid");
        assert_eq!(bounded.as_str(), exact_max);

        let via_try_from = StripeVersion::try_from("2025-02-24.acacia").expect("TryFrom &str");
        assert_eq!(via_try_from.as_str(), "2025-02-24.acacia");
    }

    #[test]
    fn tc_9_1_1_stripe_version_rejects_invalid_values() {
        let oversized = "v".repeat(MAX_STRIPE_VERSION_LEN + 1);
        let invalid_cases = [
            "",
            " ",
            "2026-11-18 endive",
            " 2026-11-18.endive",
            "2026-11-18.endive\t",
            "2026-11-18\r\nX-Injected: true",
            "2026-11-18\0endive",
            "2026-11-18.é",
            oversized.as_str(),
        ];

        for raw in invalid_cases {
            let err = StripeVersion::try_new(raw).expect_err("invalid version must fail");
            assert!(
                matches!(err, StripeError::Config(_)),
                "expected StripeError::Config for {raw:?}, got {err:?}",
            );
        }
    }

    #[test]
    fn tc_9_1_1_transport_config_defaults() {
        let config = TransportConfig::new(SecretKey::new("sk_test_default_headers"));
        assert_eq!(config.api_base(), DEFAULT_API_BASE);
        assert_eq!(config.stripe_version().as_str(), DEFAULT_STRIPE_VERSION);
        assert_eq!(config.timeout(), Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        assert!(config.http_client().is_none());
    }

    #[test]
    fn tc_9_1_1_build_headers_default_version_and_auth() {
        let config = TransportConfig::new(SecretKey::new("sk_test_default_headers"));
        let headers = config.build_headers(None).expect("headers should build");
        assert_header_eq(&headers, STRIPE_VERSION_HEADER, "2026-11-18.endive");

        let auth = headers
            .get(AUTHORIZATION)
            .expect("Authorization header must be set");
        assert_eq!(auth.to_str().ok(), Some("Bearer sk_test_default_headers"));
        assert!(auth.is_sensitive());
        assert!(headers.get(IDEMPOTENCY_KEY_HEADER).is_none());
    }

    #[test]
    fn tc_9_1_1_build_headers_with_idempotency_key_and_custom_version() {
        let custom_version = StripeVersion::try_new("2026-01-01.custom").expect("valid version");
        let config = TransportConfig::new(SecretKey::new("sk_test_idem_001"))
            .with_stripe_version(custom_version);

        let headers = config
            .build_headers(Some("idem_abc_123"))
            .expect("headers with idempotency key should build");

        assert_header_eq(&headers, STRIPE_VERSION_HEADER, "2026-01-01.custom");
        assert_header_eq(&headers, IDEMPOTENCY_KEY_HEADER, "idem_abc_123");
    }

    #[test]
    fn tc_9_1_1_build_headers_rejects_invalid_api_key() {
        let empty_key_cfg = TransportConfig::new(SecretKey::new(""));
        assert!(matches!(
            empty_key_cfg.build_headers(None),
            Err(StripeError::Config(_))
        ));

        let crlf_secret = "sk_test_secret\r\nEvil: 1";
        let crlf_key_cfg = TransportConfig::new(SecretKey::new(crlf_secret));
        let err = crlf_key_cfg
            .build_headers(None)
            .expect_err("CRLF in API key must fail");
        assert!(!err.to_string().contains("sk_test_secret"));
    }

    #[test]
    fn tc_9_1_1_build_headers_rejects_invalid_idempotency_key() {
        let valid_cfg = TransportConfig::new(SecretKey::new("sk_test_valid_key"));
        for bad_key in ["   ", "idem\r\nInject: 1"] {
            assert!(matches!(
                valid_cfg.build_headers(Some(bad_key)),
                Err(StripeError::InvalidRequest(_))
            ));
        }
    }

    #[test]
    fn tc_9_1_1_injectable_reqwest_client_and_default_resolution() {
        let default_cfg = TransportConfig::new(SecretKey::new("sk_test_default_client"));
        assert!(default_cfg.http_client().is_none());
        let _built = default_cfg
            .resolve_http_client()
            .expect("default client should build");

        let injected = reqwest::Client::builder()
            .timeout(Duration::from_secs(7))
            .build()
            .expect("custom reqwest client builds");
        let custom_cfg = TransportConfig::new(SecretKey::new("sk_test_custom_client"))
            .with_http_client(injected);
        assert!(custom_cfg.http_client().is_some());
        let _resolved = custom_cfg
            .resolve_http_client()
            .expect("injected client should resolve");
    }

    #[test]
    fn tc_9_1_1_instance_isolation_config_fields() {
        let (cfg_a, cfg_b) = build_isolated_configs();
        assert_eq!(cfg_a.api_base(), "https://api.stripe.com");
        assert_eq!(cfg_b.api_base(), "http://127.0.0.1:12111");
        assert_eq!(cfg_a.timeout(), Duration::from_secs(10));
        assert_eq!(cfg_b.timeout(), Duration::from_secs(45));
    }

    #[test]
    fn tc_9_1_1_instance_isolation_headers() {
        let (cfg_a, cfg_b) = build_isolated_configs();
        let headers_a = cfg_a.build_headers(None).expect("headers a");
        let headers_b = cfg_b.build_headers(None).expect("headers b");
        assert_header_eq(&headers_a, STRIPE_VERSION_HEADER, "2026-11-18.endive");
        assert_header_eq(&headers_b, STRIPE_VERSION_HEADER, "2025-02-24.acacia");
        assert_ne!(headers_a.get(AUTHORIZATION), headers_b.get(AUTHORIZATION));
    }

    #[test]
    fn tc_9_1_1_debug_always_redacts_test_and_live_secret_keys() {
        let test_secret = format!("{}{}", "sk_test", "_synthetic_fixture_value");
        let live_secret = format!("{}{}", "sk_live", "_synthetic_fixture_value");

        for secret in [&test_secret, &live_secret] {
            let cfg = TransportConfig::new(SecretKey::new(secret.as_str()));
            assert_debug_redacted(&format!("{cfg:?}"), secret);
            assert_debug_redacted(&format!("{cfg:#?}"), secret);
        }
    }

    #[test]
    fn tc_9_1_1_api_base_and_timeout_validation() {
        let invalid_bases = [
            "",
            "api.stripe.com",
            "ftp://api.stripe.com",
            "javascript:alert(1)",
            "http://api.stripe.com",
            "https://api.stripe.com@evil.example",
            "https://user:pass@api.stripe.com",
            "https://api.stripe.com?foo=bar",
            "https://api.stripe.com#frag",
            "https://api.stripe.com\\evil",
            "https://",
            "http://",
            "https://api.stripe.com/ bad",
            "https://api.stripe.com/\r\nX-Inject: 1",
        ];

        for base in invalid_bases {
            let err = TransportConfig::new(SecretKey::new("sk_test_base_check"))
                .with_api_base(base)
                .expect_err("invalid api_base must fail");
            assert!(
                matches!(err, StripeError::Config(_)),
                "expected StripeError::Config for {base:?}, got {err:?}",
            );
        }

        for valid in [
            "https://api.stripe.com",
            "http://127.0.0.1:8080",
            "http://localhost:8080",
            "http://[::1]:8080",
        ] {
            let cfg = TransportConfig::new(SecretKey::new("sk_test_base_ok"))
                .with_api_base(valid)
                .expect("valid api_base must succeed");
            assert_eq!(cfg.api_base(), valid);
        }

        let timeout_err = TransportConfig::new(SecretKey::new("sk_test_timeout_check"))
            .with_timeout(Duration::ZERO)
            .expect_err("zero timeout must fail");
        assert!(matches!(timeout_err, StripeError::Config(_)));
    }

    #[test]
    fn build_headers_enforces_idempotency_key_length_and_ascii() {
        let cfg = TransportConfig::new(SecretKey::new("sk_test_idem_check"));
        let oversized = "k".repeat(256);
        assert!(matches!(
            cfg.build_headers(Some(&oversized)),
            Err(StripeError::InvalidRequest(_))
        ));
        assert!(matches!(
            cfg.build_headers(Some("idem-non-ascii-é")),
            Err(StripeError::InvalidRequest(_))
        ));
    }
}
