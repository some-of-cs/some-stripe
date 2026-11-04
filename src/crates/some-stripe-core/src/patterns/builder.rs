//! Reusable fluent request builder pattern for Stripe REST API endpoints.

use crate::client::StripeClient;
use crate::error::StripeError;
use crate::patterns::crud::resource_url;
use crate::patterns::idempotency::IdempotencyKeyManager;
use crate::patterns::serializer::{to_form_params, to_form_urlencoded};
use reqwest::Method;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::time::Duration;

/// Fluent request builder standardizing outbound HTTP request preparation.
pub struct StripeRequestBuilder<'a> {
    client: &'a StripeClient,
    method: Method,
    path: String,
    idempotency_key: Option<String>,
    stripe_account: Option<String>,
    timeout: Option<Duration>,
    form_params: Option<Vec<(String, String)>>,
    form_body: Option<String>,
    deferred_error: Option<StripeError>,
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
            form_body: None,
            deferred_error: None,
        }
    }

    /// Attaches an `Idempotency-Key` header to the request after validating its format.
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

    /// Attaches form-urlencoded key-value tuples directly.
    pub fn form(mut self, params: Vec<(String, String)>) -> Self {
        self.form_params = Some(params);
        self
    }

    /// Attaches a JSON payload encoded into form-urlencoded format with fluent chaining.
    pub fn form_json(mut self, value: &serde_json::Value) -> Self {
        match to_form_params(value) {
            Ok(pairs) => match to_form_urlencoded(value) {
                Ok(encoded) => {
                    self.form_params = Some(pairs);
                    self.form_body = Some(encoded);
                }
                Err(e) => {
                    self.deferred_error = Some(e);
                }
            },
            Err(e) => {
                self.deferred_error = Some(e);
            }
        }
        self
    }

    /// Fallible variant of [`form_json`](Self::form_json) returning immediate error on invalid inputs.
    pub fn try_form_json(mut self, value: &serde_json::Value) -> Result<Self, StripeError> {
        let pairs = to_form_params(value)?;
        let encoded = to_form_urlencoded(value)?;
        self.form_params = Some(pairs);
        self.form_body = Some(encoded);
        Ok(self)
    }

    /// Serializes any parameter struct into form-urlencoded parameters and attaches them.
    pub fn form_serialize<S: Serialize>(self, params: &S) -> Result<Self, StripeError> {
        let val = serde_json::to_value(params)?;
        self.try_form_json(&val)
    }

    /// Access the prepared form-urlencoded key-value tuples, if set.
    pub fn form_params(&self) -> Option<&[(String, String)]> {
        self.form_params.as_deref()
    }

    /// Access the raw percent-encoded form body string, if set.
    pub fn form_body(&self) -> Option<&str> {
        self.form_body.as_deref()
    }

    /// Executes the prepared request, parses the returned JSON, and deserializes into `T`.
    pub async fn execute<T: DeserializeOwned>(self) -> Result<T, StripeError> {
        if let Some(err) = self.deferred_error {
            return Err(err);
        }

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

        if let Some(body) = self.form_body {
            req = req
                .header(
                    reqwest::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(body);
        } else if let Some(form) = self.form_params {
            req = req.form(&form);
        }

        let val = self.client.execute(req).await?;
        let res: T = serde_json::from_value(val)?;
        Ok(res)
    }
}
