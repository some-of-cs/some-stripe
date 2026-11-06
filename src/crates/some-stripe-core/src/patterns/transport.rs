//! Pluggable HTTP transport port and connection pooling abstractions.

use crate::error::StripeError;
use async_trait::async_trait;
use std::fmt::Debug;
use std::time::Duration;

/// Pluggable HTTP client transport trait allowing custom network backends.
#[async_trait]
pub trait HttpClient: Send + Sync + Debug {
    /// Dispatches an asynchronous HTTP request over the network.
    async fn execute_raw(
        &self,
        method: reqwest::Method,
        url: &str,
        headers: reqwest::header::HeaderMap,
        body: Option<Vec<u8>>,
        timeout: Option<Duration>,
    ) -> Result<reqwest::Response, StripeError>;
}

/// Production HTTP transport implementation backed by `reqwest::Client`.
///
/// Features:
/// - Connection pooling with 90-second idle timeout and 32 max idle connections per host.
/// - 60-second TCP keepalive to maintain live HTTP/1.1 and HTTP/2 connections.
/// - Strict redirect policy of `none` (prevents credential leakage on redirects per Gate G2 / RFC-9110).
///
/// # Examples
/// ```rust
/// use some_stripe_core::patterns::transport::ReqwestTransport;
/// use std::time::Duration;
///
/// let transport = ReqwestTransport::new(Duration::from_secs(30));
/// assert!(transport.inner().get("https://api.stripe.com/healthcheck").build().is_ok());
/// ```
#[derive(Debug, Clone)]
pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    /// Creates a new `ReqwestTransport` with default connection pooling and timeouts.
    pub fn new(timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(32)
            .tcp_keepalive(Duration::from_secs(60))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_default();

        Self { client }
    }

    /// Access the underlying `reqwest::Client`.
    pub fn inner(&self) -> &reqwest::Client {
        &self.client
    }
}

#[async_trait]
impl HttpClient for ReqwestTransport {
    async fn execute_raw(
        &self,
        method: reqwest::Method,
        url: &str,
        headers: reqwest::header::HeaderMap,
        body: Option<Vec<u8>>,
        timeout: Option<Duration>,
    ) -> Result<reqwest::Response, StripeError> {
        let mut req = self.client.request(method, url).headers(headers);
        if let Some(t) = timeout {
            req = req.timeout(t);
        }
        if let Some(b) = body {
            req = req.body(b);
        }
        let res = req.send().await?;
        Ok(res)
    }
}

/// Computes exponential backoff delay with deterministic pseudorandom jitter.
///
/// The formula calculates:
/// `delay = min(base_ms * 2^min(attempt, 6), max_ms) - jitter`
/// where jitter avoids thundering herd spikes when multiple requests fail simultaneously.
///
/// # Examples
/// ```rust
/// use some_stripe_core::patterns::transport::compute_backoff_delay;
/// use std::time::Duration;
///
/// let delay0 = compute_backoff_delay(0, 100, 5000);
/// assert!(delay0 >= Duration::from_millis(50));
/// assert!(delay0 <= Duration::from_millis(100));
///
/// let delay_max = compute_backoff_delay(10, 100, 5000);
/// assert!(delay_max <= Duration::from_millis(5000));
/// ```
pub fn compute_backoff_delay(attempt: u32, base_ms: u64, max_ms: u64) -> Duration {
    let exponential = base_ms.saturating_mul(1u64 << attempt.min(6));
    let capped = exponential.min(max_ms);
    // Add simple deterministic pseudorandom jitter
    let jitter = (capped / 4).saturating_mul((attempt as u64 % 3) + 1) / 3;
    Duration::from_millis(capped.saturating_sub(jitter))
}
