//! Stripe API client configuration and execution engine.

use crate::error::StripeError;
use crate::resources::accounts::AccountsResource;
use crate::resources::checkout::CheckoutResource;
use crate::resources::payment_intents::PaymentIntentsResource;
use crate::resources::webhooks::WebhooksResource;
use crate::workflow::{DefaultErrorWorkflow, ErrorWorkflowHandler};
use std::sync::Arc;
use std::time::Duration;

/// Configuration options for initializing a [`StripeClient`].
#[derive(Clone)]
pub struct StripeConfig {
    /// Secret API key for authentication (`sk_live_...` or `sk_test_...`).
    pub secret_key: String,

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
    pub fn new(secret_key: impl Into<String>) -> Self {
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

/// The main Stripe client used to interact with the Stripe API, modeled after `stripe-node`.
#[derive(Clone)]
pub struct StripeClient {
    pub(crate) config: Arc<StripeConfig>,
    pub(crate) http: reqwest::Client,
}

impl StripeClient {
    /// Creates a new `StripeClient` configured with default settings and the provided secret key.
    pub fn new(secret_key: impl Into<String>) -> Self {
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
            config: Arc::new(config),
            http,
        }
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

    /// Internal HTTP helper for sending requests to the Stripe API.
    pub(crate) async fn execute(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<serde_json::Value, StripeError> {
        let response = request
            .basic_auth(&self.config.secret_key, None::<&str>)
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
