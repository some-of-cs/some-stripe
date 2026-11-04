//! Reusable fluent request builder pattern for Stripe REST API endpoints.
//!
//! # Purpose
//! Eliminates repetitive HTTP header injection, URL assembly, form encoding, and response
//! deserialization across all resource services.

use crate::client::StripeClient;
use crate::error::StripeError;
use crate::patterns::crud::resource_url;
use crate::patterns::idempotency::IdempotencyKeyManager;
use reqwest::Method;
use serde::de::DeserializeOwned;
use std::time::Duration;

/// Fluent request builder that standardizes outbound request execution.
///
/// # Examples
///
/// ```no_run
/// use some_stripe::{StripeClient, patterns::builder::StripeRequestBuilder};
/// use reqwest::Method;
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let client = StripeClient::new("sk_test_123");
/// let builder = StripeRequestBuilder::new(&client, Method::GET, "v1/account")
///     .stripe_account("acct_sub_123");
/// # Ok(())
/// # }
/// ```
pub struct StripeRequestBuilder<'a> {
    client: &'a StripeClient,
    method: Method,
    path: String,
    idempotency_key: Option<String>,
    stripe_account: Option<String>,
    timeout: Option<Duration>,
    form_params: Option<Vec<(String, String)>>,
}

impl<'a> StripeRequestBuilder<'a> {
    /// Creates a new builder for a specified HTTP method and resource path.
    pub fn new(client: &'a StripeClient, method: Method, path: impl Into<String>) -> Self {
        Self {
            client,
            method,
            path: path.into(),
            idempotency_key: None,
            stripe_account: None,
            timeout: None,
            form_params: None,
        }
    }

    /// Attaches an `Idempotency-Key` header to the request after validating its format.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if the idempotency key fails validation.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Result<Self, StripeError> {
        let key_str = key.into();
        IdempotencyKeyManager::validate(&key_str)?;
        self.idempotency_key = Some(key_str);
        Ok(self)
    }

    /// Attaches an optional `Idempotency-Key` if present.
    pub fn maybe_idempotency_key(
        self,
        key: Option<impl Into<String>>,
    ) -> Result<Self, StripeError> {
        if let Some(k) = key {
            self.idempotency_key(k)
        } else {
            Ok(self)
        }
    }

    /// Attaches a `Stripe-Account` header for Connect multi-tenancy.
    pub fn stripe_account(mut self, account_id: impl Into<String>) -> Self {
        self.stripe_account = Some(account_id.into());
        self
    }

    /// Sets a per-request execution timeout.
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Attaches form-urlencoded key-value tuples.
    pub fn form(mut self, params: Vec<(String, String)>) -> Self {
        self.form_params = Some(params);
        self
    }

    /// Executes the prepared request, parses the returned JSON, and deserializes into `T`.
    pub async fn execute<T: DeserializeOwned>(self) -> Result<T, StripeError> {
        let url = resource_url(&self.client.config.api_base, &self.path);
        let mut req = self.client.http.request(self.method, &url);

        if let Some(key) = &self.idempotency_key {
            req = req.header("Idempotency-Key", key);
        }

        if let Some(acct) = &self.stripe_account {
            req = req.header("Stripe-Account", acct);
        }

        if let Some(t) = self.timeout {
            req = req.timeout(t);
        }

        if let Some(form) = self.form_params {
            req = req.form(&form);
        }

        let val = self.client.execute(req).await?;
        let res: T = serde_json::from_value(val)?;
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_initialization() {
        let client = StripeClient::new("sk_test_key");
        let builder = StripeRequestBuilder::new(&client, Method::POST, "v1/charges")
            .stripe_account("acct_123")
            .timeout(Duration::from_secs(5));

        assert_eq!(builder.path, "v1/charges");
        assert_eq!(builder.stripe_account, Some("acct_123".into()));
        assert_eq!(builder.timeout, Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_builder_idempotency_validation() {
        let client = StripeClient::new("sk_test_key");
        let builder = StripeRequestBuilder::new(&client, Method::POST, "v1/refunds");

        assert!(builder.idempotency_key("valid_key").is_ok());

        let builder2 = StripeRequestBuilder::new(&client, Method::POST, "v1/refunds");
        assert!(builder2.idempotency_key("").is_err());
    }
}
