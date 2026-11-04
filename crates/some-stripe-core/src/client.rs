//! Stripe API client configuration and execution engine.

use crate::error::StripeError;
use crate::workflow::{DefaultErrorWorkflow, ErrorWorkflowHandler};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// A secret API key securely stored in memory, automatically zeroized on drop,
/// and redacted from all `Debug` and `Display` implementations (PCI-DSS 4.0 / Gate G2).
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretKey {
    inner: String,
}

impl SecretKey {
    /// Creates a new securely held [`SecretKey`].
    pub fn new(key: impl Into<String>) -> Self {
        Self { inner: key.into() }
    }

    /// Exposes the underlying secret string for authenticated HTTP transmissions.
    pub fn expose_secret(&self) -> &str {
        &self.inner
    }
}

impl Deref for SecretKey {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl AsRef<str> for SecretKey {
    fn as_ref(&self) -> &str {
        &self.inner
    }
}

impl<T: Into<String>> From<T> for SecretKey {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl std::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl std::fmt::Display for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

/// Constant-time comparison for secret keys to prevent timing side-channel attacks (Gate G2).
impl PartialEq for SecretKey {
    fn eq(&self, other: &Self) -> bool {
        if self.inner.len() != other.inner.len() {
            return false;
        }
        self.inner.as_bytes().ct_eq(other.inner.as_bytes()).into()
    }
}

impl Eq for SecretKey {}

/// Extension trait for [`reqwest::RequestBuilder`] providing fluent Stripe header injection.
pub trait StripeHeaderExt {
    /// Injects `Stripe-Account` header for Stripe Connect multi-tenancy.
    fn stripe_account(self, account_id: &str) -> Self;
    /// Injects `Idempotency-Key` header to safely retry state-mutating requests.
    fn idempotency_key(self, key: &str) -> Self;
    /// Injects `Stripe-Version` pinned API version header.
    fn stripe_version(self, version: &str) -> Self;
}

impl StripeHeaderExt for reqwest::RequestBuilder {
    fn stripe_account(self, account_id: &str) -> Self {
        self.header("Stripe-Account", account_id)
    }

    fn idempotency_key(self, key: &str) -> Self {
        self.header("Idempotency-Key", key)
    }

    fn stripe_version(self, version: &str) -> Self {
        self.header("Stripe-Version", version)
    }
}

/// Per-request options allowing callers to override multi-tenant headers, idempotency keys,
/// API versions, or network timeouts on a specific call without mutating the base client.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestOptions {
    /// Connected account ID for Stripe Connect multi-tenancy (`Stripe-Account` header).
    pub stripe_account: Option<String>,
    /// Unique idempotency key to prevent duplicate mutations (`Idempotency-Key` header).
    pub idempotency_key: Option<String>,
    /// Pinned Stripe API version override (`Stripe-Version` header).
    pub stripe_version: Option<String>,
    /// Per-request execution timeout override.
    pub timeout: Option<Duration>,
}

impl RequestOptions {
    /// Creates empty, default [`RequestOptions`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches a `Stripe-Account` connected account header for Connect multi-tenancy.
    pub fn stripe_account(mut self, account_id: impl Into<String>) -> Self {
        self.stripe_account = Some(account_id.into());
        self
    }

    /// Attaches an `Idempotency-Key` header to protect mutations from duplicate execution.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Attaches a `Stripe-Version` header override.
    pub fn stripe_version(mut self, version: impl Into<String>) -> Self {
        self.stripe_version = Some(version.into());
        self
    }

    /// Sets a per-request execution timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// Configuration options for initializing a [`StripeClient`].
#[derive(Clone)]
pub struct StripeConfig {
    /// Secret API key for authentication (`sk_live_...` or `sk_test_...`).
    pub secret_key: SecretKey,
    /// Base URL of the Stripe REST API. Defaults to `https://api.stripe.com`.
    pub api_base: String,
    /// Network timeout for HTTP requests. Defaults to 30 seconds.
    pub timeout: Duration,
    /// Optional application info string sent in User-Agent metadata.
    pub app_info: Option<String>,
    /// Maximum retry attempts for retryable errors (429, 5xx). Defaults to 3.
    pub max_retries: u32,
    /// Configurable error workflow and retry strategy.
    pub error_workflow: Arc<dyn ErrorWorkflowHandler>,
}

impl std::fmt::Debug for StripeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StripeConfig")
            .field("secret_key", &"[REDACTED]")
            .field("api_base", &self.api_base)
            .field("timeout", &self.timeout)
            .field("app_info", &self.app_info)
            .field("max_retries", &self.max_retries)
            .field("error_workflow", &self.error_workflow)
            .finish()
    }
}

impl StripeConfig {
    /// Creates a new configuration with default settings and the given secret key.
    pub fn new(secret_key: impl Into<SecretKey>) -> Self {
        Self {
            secret_key: secret_key.into(),
            api_base: "https://api.stripe.com".into(),
            timeout: Duration::from_secs(30),
            app_info: None,
            max_retries: 3,
            error_workflow: Arc::new(DefaultErrorWorkflow::new()),
        }
    }

    /// Overrides the base URL (useful for local wiremock testing or staging proxies).
    pub fn with_api_base(mut self, api_base: impl Into<String>) -> Self {
        self.api_base = api_base.into();
        self
    }

    /// Overrides the network timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the maximum retry attempts for transient errors.
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Sets a custom error workflow and retry handler.
    pub fn with_error_workflow(mut self, handler: Arc<dyn ErrorWorkflowHandler>) -> Self {
        self.error_workflow = handler;
        self
    }
}

/// Thread-safe interior state for [`StripeClient`] shared across clones.
pub struct ClientInner {
    /// Active Stripe configuration.
    pub config: StripeConfig,
    /// Shared HTTP client connection pool.
    pub http: reqwest::Client,
}

impl std::fmt::Debug for ClientInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientInner")
            .field("config", &self.config)
            .finish()
    }
}

/// The main Stripe client used to interact with the Stripe API, modeled after `stripe-node`.
///
/// Backed by an `Arc<ClientInner>` to guarantee lock-free, zero-contention concurrency across
/// thousands of asynchronous tasks while enforcing Rust zero-clone borrowing discipline.
#[derive(Clone, Debug)]
pub struct StripeClient {
    pub(crate) inner: Arc<ClientInner>,
}

impl Deref for StripeClient {
    type Target = ClientInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl StripeClient {
    /// Creates a new `StripeClient` configured with default settings and the provided secret key.
    pub fn new(secret_key: impl Into<SecretKey>) -> Self {
        Self::with_config(StripeConfig::new(secret_key))
    }

    /// Creates a new `StripeClient` with custom [`StripeConfig`].
    pub fn with_config(config: StripeConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_default();

        Self {
            inner: Arc::new(ClientInner { config, http }),
        }
    }

    /// Reference to the underlying atomic inner state.
    pub fn inner(&self) -> &Arc<ClientInner> {
        &self.inner
    }

    /// Access the configured error workflow handler.
    pub fn error_workflow(&self) -> &Arc<dyn ErrorWorkflowHandler> {
        &self.config.error_workflow
    }

    /// Sends requests to the Stripe API without extra options.
    pub async fn execute(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<serde_json::Value, StripeError> {
        self.execute_with_options(request, None).await
    }

    /// Sends requests to the Stripe API with optional [`RequestOptions`].
    pub async fn execute_with_options(
        &self,
        mut request: reqwest::RequestBuilder,
        options: Option<&RequestOptions>,
    ) -> Result<serde_json::Value, StripeError> {
        if let Some(opts) = options {
            if let Some(acct) = &opts.stripe_account {
                request = request.header("Stripe-Account", acct);
            }
            if let Some(idemp) = &opts.idempotency_key {
                request = request.header("Idempotency-Key", idemp);
            }
            if let Some(ver) = &opts.stripe_version {
                request = request.header("Stripe-Version", ver);
            }
            if let Some(to) = opts.timeout {
                request = request.timeout(to);
            }
        }

        let response = request
            .basic_auth(self.config.secret_key.expose_secret(), None::<&str>)
            .send()
            .await?;

        let status = response.status();
        let body_bytes = response.bytes().await?;

        if !status.is_success() {
            let error_json: serde_json::Value =
                serde_json::from_slice(&body_bytes).unwrap_or_default();
            let err_obj = error_json.get("error");

            let status_code = status.as_u16();
            let code = err_obj
                .and_then(|e| e.get("code"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let message = err_obj
                .and_then(|e| e.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("Stripe request rejected")
                .to_string();
            let error_type = err_obj
                .and_then(|e| e.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("api_error")
                .to_string();
            let doc_url = err_obj
                .and_then(|e| e.get("doc_url"))
                .and_then(|v| v.as_str())
                .map(str::to_string);

            // Notify configured error workflow
            let _ = self
                .config
                .error_workflow
                .on_api_error(status_code, &code, &message, 1, self.config.max_retries)
                .await;

            return Err(StripeError::Api {
                status: status_code,
                code,
                message,
                error_type,
                doc_url,
            });
        }

        let value: serde_json::Value = serde_json::from_slice(&body_bytes)?;
        Ok(value)
    }
}

// Compile-time static assertions ensuring StripeClient and its dependencies implement Send + Sync
#[allow(dead_code)]
const _: () = {
    fn assert_send_sync<T: Send + Sync>() {}
    fn verify() {
        assert_send_sync::<StripeClient>();
        assert_send_sync::<ClientInner>();
        assert_send_sync::<SecretKey>();
        assert_send_sync::<StripeConfig>();
        assert_send_sync::<RequestOptions>();
    }
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_key_redaction_and_constant_time_equality() {
        let key1 = SecretKey::new("sk_test_12345");
        let key2 = SecretKey::new("sk_test_12345");
        let key3 = SecretKey::new("sk_test_99999");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
        assert_eq!(format!("{key1:?}"), "[REDACTED]");
        assert_eq!(format!("{key1}"), "[REDACTED]");
    }

    #[test]
    fn test_request_options_builder() {
        let opts = RequestOptions::new()
            .stripe_account("acct_test_123")
            .idempotency_key("idemp_key_abc")
            .stripe_version("2024-06-20")
            .timeout(Duration::from_secs(10));

        assert_eq!(opts.stripe_account.as_deref(), Some("acct_test_123"));
        assert_eq!(opts.idempotency_key.as_deref(), Some("idemp_key_abc"));
        assert_eq!(opts.stripe_version.as_deref(), Some("2024-06-20"));
        assert_eq!(opts.timeout, Some(Duration::from_secs(10)));
    }
}
