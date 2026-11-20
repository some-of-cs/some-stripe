//! Stripe API client configuration and execution engine.

use crate::error::StripeError;
use crate::patterns::transport::{ReqwestTransport, compute_backoff_delay};
use crate::workflow::{DefaultErrorWorkflow, ErrorAction, ErrorWorkflowHandler};
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
    ///
    /// If `key` begins with `$` (such as `"$STRIPE_SECRET_KEY"` or `"${STRIPE_SECRET_KEY}"`),
    /// it automatically resolves the secret value from the process environment or workspace
    /// dotenv files (`.env.local`, `.env.dev`, `.env.development`, `.env.test`, `.env.prod`,
    /// `.env.production`, `.env`, `.env.example`, or custom `.env.*` files).
    pub fn new(key: impl Into<String>) -> Self {
        let mut raw = key.into();
        let inner = crate::env_secret::resolve_env_secret(&raw);
        raw.zeroize();
        Self { inner }
    }

    /// Creates a [`SecretKey`] resolving any `$VAR_NAME` reference relative to `base_dir`.
    pub fn from_dir(key: impl Into<String>, base_dir: &std::path::Path) -> Self {
        let mut raw = key.into();
        let inner = crate::env_secret::resolve_env_secret_from_dir(&raw, base_dir);
        raw.zeroize();
        Self { inner }
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
#[derive(Clone, Default, PartialEq, Eq)]
pub struct RequestOptions {
    /// Connected account ID for Stripe Connect multi-tenancy (`Stripe-Account` header).
    pub stripe_account: Option<String>,
    /// Unique idempotency key to prevent duplicate mutations (`Idempotency-Key` header).
    pub idempotency_key: Option<String>,
    /// Pinned Stripe API version override (`Stripe-Version` header).
    pub stripe_version: Option<String>,
    /// Per-request execution timeout override.
    pub timeout: Option<Duration>,
    /// Ephemeral bearer token override (e.g. for Meter Event Sessions: `Authorization: Bearer <token>`).
    pub bearer_token: Option<SecretKey>,
}

impl std::fmt::Debug for RequestOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let redacted_token = self.bearer_token.as_ref().map(|_| "[REDACTED]");
        f.debug_struct("RequestOptions")
            .field("stripe_account", &self.stripe_account)
            .field("idempotency_key", &self.idempotency_key)
            .field("stripe_version", &self.stripe_version)
            .field("timeout", &self.timeout)
            .field("bearer_token", &redacted_token)
            .finish()
    }
}

impl RequestOptions {
    /// Creates empty, default [`RequestOptions`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches an ephemeral bearer token override for authentication.
    pub fn bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(SecretKey::new(token));
        self
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

    /// Sets application info metadata sent in the HTTP `User-Agent` header.
    pub fn with_app_info(mut self, app_info: impl Into<String>) -> Self {
        self.app_info = Some(app_info.into());
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

/// Maximum ceiling in seconds applied to the `Retry-After` response header to prevent unbounded sleeping.
pub const MAX_RETRY_AFTER_SECS: u64 = 60;

/// Parses and clamps the `Retry-After` header value, bounded by [`MAX_RETRY_AFTER_SECS`].
///
/// # Examples
/// ```rust
/// use some_stripe_core::client::{retry_after_delay, MAX_RETRY_AFTER_SECS};
/// use std::time::Duration;
///
/// let mut headers = reqwest::header::HeaderMap::new();
/// headers.insert(reqwest::header::RETRY_AFTER, "999999".parse().unwrap());
/// assert_eq!(retry_after_delay(&headers), Some(Duration::from_secs(MAX_RETRY_AFTER_SECS)));
///
/// headers.insert(reqwest::header::RETRY_AFTER, "30".parse().unwrap());
/// assert_eq!(retry_after_delay(&headers), Some(Duration::from_secs(30)));
/// ```
pub fn retry_after_delay(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .map(|secs| Duration::from_secs(secs.min(MAX_RETRY_AFTER_SECS)))
}

/// The main Stripe client used to interact with the Stripe API, modeled after `stripe-node`.
///
/// Backed by an `Arc<ClientInner>` to guarantee lock-free, zero-contention concurrency across
/// thousands of asynchronous tasks while enforcing Rust zero-clone borrowing discipline.
///
/// # Retry and Idempotency Invariants
/// The client executes requests with intelligent, automated retry handling:
/// - **Safe Idempotent Methods**: HTTP `GET`, `HEAD`, `DELETE`, `OPTIONS`, and `PUT` are safely retried
///   on network failures or transient 503/500/429 responses up to `config.max_retries`.
/// - **Idempotency-Key Protected Mutations**: HTTP `POST` requests are **only** retried across network drops
///   if an `Idempotency-Key` header is present (either in request headers or via [`RequestOptions::idempotency_key`]).
///   Without an idempotency key, mutating POST requests fail immediately on network disconnection to prevent
///   duplicate financial transactions on Stripe.
/// - **Exponential Backoff with Jitter**: Retries use [`compute_backoff_delay`] to prevent API stampedes.
/// - **Custom Workflow Delegation**: Retry decisions delegate to [`ErrorWorkflowHandler`]
///   ([`DefaultErrorWorkflow`]), enabling customizable retry policies and rate-limit backoff.
///
/// # Examples
/// ```rust
/// use some_stripe_core::{StripeClient, StripeConfig, SecretKey, RequestOptions};
/// use std::time::Duration;
///
/// let config = StripeConfig::new(SecretKey::new("sk_test_mock_123"))
///     .with_timeout(Duration::from_secs(15))
///     .with_max_retries(3);
///
/// let client = StripeClient::with_config(config);
/// assert_eq!(client.config.max_retries, 3);
///
/// // Per-request idempotency key ensures safe retries on network drops:
/// let opts = RequestOptions::new().idempotency_key("order_create_tx_9876");
/// assert_eq!(opts.idempotency_key.as_deref(), Some("order_create_tx_9876"));
/// ```
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
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_core::StripeClient;
    ///
    /// let client = StripeClient::new("sk_test_123");
    /// assert_eq!(client.config.api_base, "https://api.stripe.com");
    /// ```
    pub fn new(secret_key: impl Into<SecretKey>) -> Self {
        Self::with_config(StripeConfig::new(secret_key))
    }

    /// Creates a new `StripeClient` with custom [`StripeConfig`].
    ///
    /// The client initializes a production [`ReqwestTransport`] connection pool configured with
    /// the specified network timeout, 90-second idle connection pooling, and redirect policy `none`.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_core::{StripeClient, StripeConfig, SecretKey};
    /// use std::time::Duration;
    ///
    /// let config = StripeConfig::new(SecretKey::new("sk_test_123"))
    ///     .with_timeout(Duration::from_secs(10))
    ///     .with_max_retries(2);
    /// let client = StripeClient::with_config(config);
    /// assert_eq!(client.config.max_retries, 2);
    /// ```
    pub fn with_config(config: StripeConfig) -> Self {
        let http = ReqwestTransport::new(config.timeout).inner().clone();

        Self {
            inner: Arc::new(ClientInner { config, http }),
        }
    }

    /// Creates a new `StripeClient` with custom [`StripeConfig`], returning [`StripeError::Config`]
    /// if the underlying HTTP transport fails to initialize.
    ///
    /// # Examples
    /// ```rust
    /// use some_stripe_core::{StripeClient, StripeConfig, SecretKey};
    /// use std::time::Duration;
    ///
    /// let config = StripeConfig::new(SecretKey::new("sk_test_123"))
    ///     .with_timeout(Duration::from_secs(10))
    ///     .with_app_info("some-payment/0.1.0");
    /// let client = StripeClient::try_with_config(config).expect("client init");
    /// assert_eq!(client.config.app_info.as_deref(), Some("some-payment/0.1.0"));
    /// ```
    pub fn try_with_config(config: StripeConfig) -> Result<Self, StripeError> {
        let http = ReqwestTransport::try_new(config.timeout)?.inner().clone();

        Ok(Self {
            inner: Arc::new(ClientInner { config, http }),
        })
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
    ///
    /// # Retry and Idempotency Invariants
    /// Network-level connection drops and timeouts are retried up to `config.max_retries` times
    /// **only if** the request is idempotent (GET, HEAD, DELETE, OPTIONS, PUT) or carries an
    /// `Idempotency-Key` header. State-mutating POST requests without an idempotency key are
    /// never retried automatically across network drops to prevent duplicate actions.
    pub async fn execute_with_options(
        &self,
        request: reqwest::RequestBuilder,
        options: Option<&RequestOptions>,
    ) -> Result<serde_json::Value, StripeError> {
        let body_bytes = self
            .execute_raw_bytes_with_options(request, options)
            .await?;
        let value: serde_json::Value = serde_json::from_slice(&body_bytes)?;
        Ok(value)
    }

    fn prepare_request(
        &self,
        mut request: reqwest::RequestBuilder,
        options: Option<&RequestOptions>,
    ) -> Result<reqwest::Request, StripeError> {
        if let Some(app_info) = &self.config.app_info {
            request = request.header(reqwest::header::USER_AGENT, app_info);
        }
        if let Some(opts) = options {
            request = apply_request_options(request, opts);
        }
        let bearer = options.and_then(|o| o.bearer_token.as_ref());
        request = match bearer {
            Some(tok) => request.bearer_auth(tok.expose_secret()),
            None => request.basic_auth(self.config.secret_key.expose_secret(), None::<&str>),
        };
        request.build().map_err(StripeError::Http)
    }

    async fn should_retry_network_error(
        &self,
        err: &reqwest::Error,
        can_retry_network: bool,
        has_next_req: bool,
        retries_taken: u32,
    ) -> bool {
        if can_retry_network && has_next_req {
            let action = self
                .config
                .error_workflow
                .on_api_error(
                    503,
                    "network_error",
                    &err.to_string(),
                    retries_taken,
                    self.config.max_retries,
                )
                .await;
            if action == ErrorAction::Retry {
                let delay = compute_backoff_delay(retries_taken, 50, 5000);
                tokio::time::sleep(delay).await;
                return true;
            }
        }
        false
    }

    async fn should_retry_api_error(
        &self,
        api_error: &StripeError,
        retry_after_delay: Option<Duration>,
        can_retry_network: bool,
        has_next_req: bool,
        retries_taken: u32,
    ) -> bool {
        let StripeError::Api {
            status: status_code,
            code,
            message,
            ..
        } = api_error
        else {
            return false;
        };

        let action = self
            .config
            .error_workflow
            .on_api_error(
                *status_code,
                code,
                message,
                retries_taken,
                self.config.max_retries,
            )
            .await;

        let can_retry_status = *status_code == 429 || can_retry_network;
        if action == ErrorAction::Retry && can_retry_status && has_next_req {
            let delay =
                retry_after_delay.unwrap_or_else(|| compute_backoff_delay(retries_taken, 50, 5000));
            tokio::time::sleep(delay).await;
            return true;
        }
        false
    }

    /// Executes requests and returns the raw response body as zero-copy `bytes::Bytes`.
    ///
    /// Ideal for binary downloads such as invoice and quote PDFs.
    pub async fn execute_raw_bytes(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<bytes::Bytes, StripeError> {
        self.execute_raw_bytes_with_options(request, None).await
    }

    async fn send_and_read_response(
        &self,
        req: reqwest::Request,
    ) -> Result<(reqwest::StatusCode, Option<Duration>, bytes::Bytes), reqwest::Error> {
        let response = self.http.execute(req).await?;
        let retry_after = parse_retry_after_delay(&response);
        let status = response.status();
        let body_bytes = response.bytes().await?;
        Ok((status, retry_after, body_bytes))
    }

    async fn handle_transport_error(
        &self,
        err: reqwest::Error,
        can_retry_network: bool,
        has_next_req: bool,
        retries_taken: u32,
    ) -> AttemptOutcome {
        if self
            .should_retry_network_error(&err, can_retry_network, has_next_req, retries_taken)
            .await
        {
            return AttemptOutcome::Retry;
        }
        AttemptOutcome::Done(Err(StripeError::Http(err)))
    }

    async fn execute_single_attempt(
        &self,
        req: reqwest::Request,
        can_retry_network: bool,
        has_next_req: bool,
        retries_taken: u32,
    ) -> AttemptOutcome {
        let (status, retry_after, body_bytes) = match self.send_and_read_response(req).await {
            Ok(parts) => parts,
            Err(err) => {
                return self
                    .handle_transport_error(err, can_retry_network, has_next_req, retries_taken)
                    .await;
            }
        };

        if status.is_success() {
            return AttemptOutcome::Done(Ok(body_bytes));
        }

        let api_error = parse_api_error_response(status, &body_bytes);
        if self
            .should_retry_api_error(
                &api_error,
                retry_after,
                can_retry_network,
                has_next_req,
                retries_taken,
            )
            .await
        {
            return AttemptOutcome::Retry;
        }
        AttemptOutcome::Done(Err(api_error))
    }

    /// Executes requests with options and returns the raw response body as zero-copy `bytes::Bytes`.
    pub async fn execute_raw_bytes_with_options(
        &self,
        request: reqwest::RequestBuilder,
        options: Option<&RequestOptions>,
    ) -> Result<bytes::Bytes, StripeError> {
        let mut req = self.prepare_request(request, options)?;
        let can_retry_network = is_request_network_retryable(&req);
        let max_retries = self.config.max_retries;
        let mut retries_taken = 0u32;

        loop {
            let next_req = clone_for_retry(&req, retries_taken, max_retries);
            match self
                .execute_single_attempt(req, can_retry_network, next_req.is_some(), retries_taken)
                .await
            {
                AttemptOutcome::Done(result) => return result,
                AttemptOutcome::Retry => {
                    retries_taken += 1;
                    let Some(cloned) = next_req else {
                        return Err(StripeError::ConnectionError(
                            "retry request could not be cloned".into(),
                        ));
                    };
                    req = cloned;
                }
            }
        }
    }
}

enum AttemptOutcome {
    Done(Result<bytes::Bytes, StripeError>),
    Retry,
}

fn apply_request_options(
    mut request: reqwest::RequestBuilder,
    opts: &RequestOptions,
) -> reqwest::RequestBuilder {
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
    request
}

fn clone_for_retry(
    req: &reqwest::Request,
    retries_taken: u32,
    max_retries: u32,
) -> Option<reqwest::Request> {
    if retries_taken < max_retries {
        req.try_clone()
    } else {
        None
    }
}

fn is_request_network_retryable(req: &reqwest::Request) -> bool {
    let is_idempotent = matches!(
        *req.method(),
        reqwest::Method::GET
            | reqwest::Method::HEAD
            | reqwest::Method::DELETE
            | reqwest::Method::OPTIONS
            | reqwest::Method::PUT
    );
    let has_idempotency_key = req.headers().contains_key("idempotency-key");
    is_idempotent || has_idempotency_key
}

fn parse_retry_after_delay(response: &reqwest::Response) -> Option<Duration> {
    if response.status().as_u16() == 429 {
        retry_after_delay(response.headers())
    } else {
        None
    }
}

fn parse_api_error_response(status: reqwest::StatusCode, body_bytes: &[u8]) -> StripeError {
    let error_json: serde_json::Value = serde_json::from_slice(body_bytes).unwrap_or_default();
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

    StripeError::Api {
        status: status_code,
        code,
        message,
        error_type,
        doc_url,
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

    #[tokio::test]
    async fn test_error_workflow_retry_dispatch_exhaustion() {
        let config = StripeConfig::new("sk_test_123")
            .with_api_base("http://127.0.0.1:9") // invalid port to trigger immediate network failure
            .with_max_retries(2);
        let client = StripeClient::with_config(config);

        let req = client.http.get("http://127.0.0.1:9/v1/test");
        let result = client.execute(req).await;
        assert!(result.is_err(), "must fail after retrying");
    }

    #[derive(Debug, Default)]
    struct CallRecordingWorkflow {
        calls: std::sync::atomic::AtomicU32,
    }

    #[async_trait::async_trait]
    impl ErrorWorkflowHandler for CallRecordingWorkflow {
        async fn on_api_error(
            &self,
            _status: u16,
            _code: &str,
            _message: &str,
            _retries_taken: u32,
            _max_retries: u32,
        ) -> ErrorAction {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            ErrorAction::Retry
        }

        async fn on_webhook_error(&self, _severity: crate::workflow::ErrorSeverity, _reason: &str) {
        }
    }

    #[tokio::test]
    async fn test_non_idempotent_post_without_idempotency_key_does_not_retry_network_error() {
        let workflow = Arc::new(CallRecordingWorkflow::default());
        let config = StripeConfig::new("sk_test_123")
            .with_api_base("http://127.0.0.1:9")
            .with_max_retries(3)
            .with_error_workflow(workflow.clone());
        let client = StripeClient::with_config(config);

        // POST without Idempotency-Key
        let req = client.http.post("http://127.0.0.1:9/v1/charges");
        let result = client.execute(req).await;
        assert!(result.is_err());
        assert_eq!(
            workflow.calls.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "POST without Idempotency-Key must not retry network errors"
        );
    }

    #[tokio::test]
    async fn test_non_idempotent_post_with_idempotency_key_retries_network_error() {
        let workflow = Arc::new(CallRecordingWorkflow::default());
        let config = StripeConfig::new("sk_test_123")
            .with_api_base("http://127.0.0.1:9")
            .with_max_retries(1)
            .with_error_workflow(workflow.clone());
        let client = StripeClient::with_config(config);

        // POST with Idempotency-Key
        let req = client
            .http
            .post("http://127.0.0.1:9/v1/charges")
            .header("Idempotency-Key", "test_idemp_key_123");
        let result = client.execute(req).await;
        assert!(result.is_err());
        assert_eq!(
            workflow.calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "POST with Idempotency-Key should retry up to max_retries"
        );
    }

    #[tokio::test]
    async fn test_max_retries_one_gives_up_to_one_retry() {
        let workflow = Arc::new(CallRecordingWorkflow::default());
        let config = StripeConfig::new("sk_test_123")
            .with_api_base("http://127.0.0.1:9")
            .with_max_retries(1)
            .with_error_workflow(workflow.clone());
        let client = StripeClient::with_config(config);

        // GET request is idempotent
        let req = client.http.get("http://127.0.0.1:9/v1/customers");
        let result = client.execute(req).await;
        assert!(result.is_err());
        assert_eq!(
            workflow.calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "max_retries(1) must yield exactly 1 retry after initial attempt"
        );
    }

    #[tokio::test]
    async fn test_http_429_retry_after_header_adherence() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            // First attempt: 429 Too Many Requests with Retry-After: 0
            let (mut socket1, _) = listener.accept().await.unwrap();
            let mut buf1 = vec![0; 1024];
            let _ = socket1.read(&mut buf1).await.unwrap();
            let resp1 = "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 0\r\nContent-Type: application/json\r\nContent-Length: 46\r\n\r\n{\"error\":{\"code\":\"rate_limit\",\"type\":\"api_error\"}}";
            socket1.write_all(resp1.as_bytes()).await.unwrap();
            drop(socket1);

            // Second attempt: 200 OK
            let (mut socket2, _) = listener.accept().await.unwrap();
            let mut buf2 = vec![0; 1024];
            let _ = socket2.read(&mut buf2).await.unwrap();
            let body = "{\"id\":\"test_ok\"}";
            let resp2 = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket2.write_all(resp2.as_bytes()).await.unwrap();
            drop(socket2);
        });

        let config = StripeConfig::new("sk_test_123")
            .with_api_base(format!("http://{addr}"))
            .with_max_retries(1);
        let client = StripeClient::with_config(config);

        let req = client.http.get(format!("http://{addr}/v1/test_rate_limit"));
        let val = client
            .execute(req)
            .await
            .expect("Request should succeed after 429 retry");
        assert_eq!(val.get("id").and_then(|v| v.as_str()), Some("test_ok"));

        server.await.unwrap();
    }

    #[test]
    fn test_retry_after_clamped_to_max() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::RETRY_AFTER, "999999".parse().unwrap());
        assert_eq!(
            retry_after_delay(&headers),
            Some(Duration::from_secs(MAX_RETRY_AFTER_SECS)),
            "Retry-After: 999999 must be clamped to MAX_RETRY_AFTER_SECS (60s)"
        );

        headers.insert(reqwest::header::RETRY_AFTER, "30".parse().unwrap());
        assert_eq!(
            retry_after_delay(&headers),
            Some(Duration::from_secs(30)),
            "Retry-After: 30 must not be clamped"
        );

        headers.insert(
            reqwest::header::RETRY_AFTER,
            "invalid_number".parse().unwrap(),
        );
        assert_eq!(retry_after_delay(&headers), None);

        let empty_headers = reqwest::header::HeaderMap::new();
        assert_eq!(retry_after_delay(&empty_headers), None);
    }

    #[tokio::test]
    async fn test_non_idempotent_post_without_idempotency_key_does_not_retry_http_500() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 1024];
            let _ = socket.read(&mut buf).await.unwrap();
            let resp = "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nContent-Length: 64\r\nConnection: close\r\n\r\n{\"error\":{\"code\":\"api_error\",\"message\":\"Server error\",\"type\":\"api_error\"}}";
            socket.write_all(resp.as_bytes()).await.unwrap();
            drop(socket);
        });

        let workflow = Arc::new(CallRecordingWorkflow::default());
        let config = StripeConfig::new("sk_test_123")
            .with_api_base(format!("http://{addr}"))
            .with_max_retries(3)
            .with_error_workflow(workflow.clone());
        let client = StripeClient::with_config(config);

        let req = client.http.post(format!("http://{addr}/v1/charges"));
        let result = client.execute(req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            StripeError::Api { status, .. } => assert_eq!(status, 500),
            other => panic!("expected API 500 error, got {other:?}"),
        }

        assert_eq!(
            workflow.calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "Workflow called once for status 500 notification"
        );

        server.await.unwrap();
    }

    #[tokio::test]
    async fn test_post_with_idempotency_key_retries_http_500() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut socket1, _) = listener.accept().await.unwrap();
            let mut buf1 = vec![0; 1024];
            let _ = socket1.read(&mut buf1).await.unwrap();
            let resp1 = "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nContent-Length: 64\r\n\r\n{\"error\":{\"code\":\"api_error\",\"message\":\"Server error\",\"type\":\"api_error\"}}";
            socket1.write_all(resp1.as_bytes()).await.unwrap();
            drop(socket1);

            let (mut socket2, _) = listener.accept().await.unwrap();
            let mut buf2 = vec![0; 1024];
            let _ = socket2.read(&mut buf2).await.unwrap();
            let body = "{\"id\":\"ch_retried_ok\"}";
            let resp2 = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket2.write_all(resp2.as_bytes()).await.unwrap();
            drop(socket2);
        });

        let workflow = Arc::new(CallRecordingWorkflow::default());
        let config = StripeConfig::new("sk_test_123")
            .with_api_base(format!("http://{addr}"))
            .with_max_retries(2)
            .with_error_workflow(workflow.clone());
        let client = StripeClient::with_config(config);

        let req = client
            .http
            .post(format!("http://{addr}/v1/charges"))
            .header("Idempotency-Key", "idem_safe_retry_key");
        let val = client.execute(req).await.expect("should succeed on retry");
        assert_eq!(
            val.get("id").and_then(|v| v.as_str()),
            Some("ch_retried_ok")
        );

        server.await.unwrap();
    }

    #[test]
    fn test_request_options_bearer_token_redacted_and_secret() {
        let opts = RequestOptions::new().bearer_token("tok_ephemeral_secret_98765");
        assert_eq!(
            opts.bearer_token.as_ref().map(|t| t.expose_secret()),
            Some("tok_ephemeral_secret_98765")
        );
        let debug_str = format!("{opts:?}");
        assert!(debug_str.contains("bearer_token: Some("[REDACTED]")"));
        assert!(!debug_str.contains("tok_ephemeral_secret_98765"));

        let empty_debug = format!("{:?}", RequestOptions::new());
        assert!(empty_debug.contains("bearer_token: None"));
    }

    #[test]
    fn test_try_with_config_and_app_info_user_agent() {
        let config = StripeConfig::new("sk_test_123").with_app_info("some-payment/0.1.0");
        let client = StripeClient::try_with_config(config).expect("client init");
        let opts = RequestOptions::new().bearer_token("tok_meter_abc");
        let req = client
            .prepare_request(
                client.http.get("https://api.stripe.com/v1/customers"),
                Some(&opts),
            )
            .expect("prepare_request");

        assert_eq!(
            req.headers()
                .get(reqwest::header::USER_AGENT)
                .and_then(|v| v.to_str().ok()),
            Some("some-payment/0.1.0")
        );
        assert_eq!(
            req.headers()
                .get(reqwest::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok()),
            Some("Bearer tok_meter_abc")
        );
    }

    #[tokio::test]
    async fn test_mid_stream_body_read_error_retries_for_idempotent_get() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            // Attempt 1: send headers with Content-Length: 100 but drop connection mid-body
            let (mut socket1, _) = listener.accept().await.unwrap();
            let mut buf1 = vec![0; 1024];
            let _ = socket1.read(&mut buf1).await.unwrap();
            let resp1 = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{\"partial\":";
            socket1.write_all(resp1.as_bytes()).await.unwrap();
            drop(socket1);

            // Attempt 2: return complete JSON body
            let (mut socket2, _) = listener.accept().await.unwrap();
            let mut buf2 = vec![0; 1024];
            let _ = socket2.read(&mut buf2).await.unwrap();
            let body = "{\"id\":\"body_retry_ok\"}";
            let resp2 = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket2.write_all(resp2.as_bytes()).await.unwrap();
            drop(socket2);
        });

        let config = StripeConfig::new("sk_test_123")
            .with_api_base(format!("http://{addr}"))
            .with_max_retries(1);
        let client = StripeClient::try_with_config(config).expect("client init");

        let req = client.http.get(format!("http://{addr}/v1/customers"));
        let val = client
            .execute(req)
            .await
            .expect("should retry and succeed after mid-stream body EOF");
        assert_eq!(
            val.get("id").and_then(|v| v.as_str()),
            Some("body_retry_ok")
        );

        server.await.unwrap();
    }
}
