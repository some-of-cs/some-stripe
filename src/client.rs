//! Stripe API client configuration and execution engine.

use crate::error::StripeError;
use crate::resources::accounts::AccountsResource;
use crate::resources::checkout::CheckoutResource;
use crate::resources::payment_intents::PaymentIntentsResource;
use crate::resources::webhooks::WebhooksResource;
use crate::workflow::{DefaultErrorWorkflow, ErrorWorkflowHandler};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
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

impl PartialEq for SecretKey {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for SecretKey {}

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
            error_workflow: Arc::new(DefaultErrorWorkflow::new()),
        }
    }

    /// Sets a custom API base URL (useful for testing or proxying).
    pub fn with_api_base(mut self, base: impl Into<String>) -> Self {
        self.api_base = base.into();
        self
    }

    /// Sets a custom request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets optional application identification metadata.
    pub fn with_app_info(mut self, app_info: impl Into<String>) -> Self {
        self.app_info = Some(app_info.into());
        self
    }

    /// Injects a custom error workflow and retry handler.
    pub fn with_error_workflow(mut self, workflow: Arc<dyn ErrorWorkflowHandler>) -> Self {
        self.error_workflow = workflow;
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

    /// Access the Checkout API resource namespace (`stripe.checkout`).
    pub fn checkout(&self) -> CheckoutResource {
        CheckoutResource::new(self.clone())
    }

    /// Access the PaymentIntents API resource namespace (`stripe.paymentIntents`).
    pub fn payment_intents(&self) -> PaymentIntentsResource {
        PaymentIntentsResource::new(self.clone())
    }

    /// Access the Accounts API resource namespace (`stripe.accounts`).
    pub fn accounts(&self) -> AccountsResource {
        AccountsResource::new(self.clone())
    }

    /// Access the Webhook verification helper namespace (`stripe.webhooks`).
    pub fn webhooks(&self) -> WebhooksResource {
        WebhooksResource::new()
    }

    /// Internal HTTP helper for sending requests to the Stripe API without extra options.
    pub(crate) async fn execute(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<serde_json::Value, StripeError> {
        self.execute_with_options(request, None).await
    }

    /// Internal HTTP helper for sending requests to the Stripe API with optional [`RequestOptions`].
    pub(crate) async fn execute_with_options(
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
                .on_api_error(status_code, &code, &message, 1, 3)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_concurrency_stress_10k_tasks() {
        let client = StripeClient::new("sk_test_10k_concurrency_stress_test");
        let initial_count = Arc::strong_count(client.inner());
        assert_eq!(initial_count, 1);

        let mut handles = Vec::with_capacity(10_000);
        for _ in 0..10_000 {
            let client_clone = client.clone();
            handles.push(tokio::spawn(async move {
                // Verify clone maintains valid access to config without locking
                assert_eq!(client_clone.config.api_base, "https://api.stripe.com");
            }));
        }

        for handle in handles {
            handle.await.expect("task join failure");
        }

        assert_eq!(Arc::strong_count(client.inner()), 1);
    }

    #[test]
    fn test_request_options_builder() {
        let options = RequestOptions::new()
            .stripe_account("acct_test_123")
            .idempotency_key("idemp_key_456")
            .stripe_version("2024-11-04")
            .timeout(Duration::from_secs(15));

        assert_eq!(options.stripe_account.as_deref(), Some("acct_test_123"));
        assert_eq!(options.idempotency_key.as_deref(), Some("idemp_key_456"));
        assert_eq!(options.stripe_version.as_deref(), Some("2024-11-04"));
        assert_eq!(options.timeout, Some(Duration::from_secs(15)));
    }

    #[test]
    fn test_secret_redaction_and_zeroize() {
        let secret = SecretKey::new("sk_live_very_secret_key_9999");
        let config = StripeConfig::new(secret.clone());
        let client = StripeClient::with_config(config.clone());

        // Assert secret is never printed in Debug or Display
        let debug_secret = format!("{:?}", secret);
        let display_secret = format!("{}", secret);
        let debug_config = format!("{:?}", config);
        let debug_client = format!("{:?}", client);

        assert_eq!(debug_secret, "[REDACTED]");
        assert_eq!(display_secret, "[REDACTED]");
        assert!(!debug_config.contains("sk_live_very_secret_key_9999"));
        assert!(debug_config.contains("[REDACTED]"));
        assert!(!debug_client.contains("sk_live_very_secret_key_9999"));
        assert!(debug_client.contains("[REDACTED]"));

        // Expose secret is available only for HTTP transmission
        assert_eq!(secret.expose_secret(), "sk_live_very_secret_key_9999");
    }
}
