//! Customizable error workflows, retry policies, and security incident hooks.

use crate::error::StripeWebhookError;
use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

/// Severity classification for Stripe API and webhook errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// Informational or minor transient failure.
    Info,
    /// Recoverable condition, such as network timeout or upstream 429 / 5xx.
    Warning,
    /// Unrecoverable business rejection (e.g. card declined, invalid amount).
    Error,
    /// Security-critical anomaly (e.g. forged signature, tampered payload, replay attempt).
    Critical,
}

/// The decision or resolution returned by an error workflow handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorResolution {
    /// The operation should be retried after the specified delay.
    Retry {
        /// Delay before the next retry attempt.
        delay: Duration,
        /// Next attempt number (1-indexed).
        next_attempt: u32,
    },
    /// The operation should fail immediately without retry.
    Fail {
        /// Reason explaining why the error is non-retryable.
        reason: String,
    },
    /// The operation failed, and an incident alert or escalation was emitted.
    Escalate {
        /// Generated incident reference ID.
        incident_id: String,
        /// Severity of the incident.
        severity: ErrorSeverity,
        /// Diagnostic message.
        message: String,
    },
}

/// Extensible trait for defining customized error workflows.
#[async_trait]
pub trait ErrorWorkflowHandler: Send + Sync + Debug {
    /// Evaluates an HTTP or network error and decides whether to retry or fail.
    async fn on_network_error(
        &self,
        error: &reqwest::Error,
        attempt: u32,
        max_retries: u32,
    ) -> ErrorResolution;

    /// Evaluates a structured Stripe REST API error response.
    async fn on_api_error(
        &self,
        status: u16,
        code: &str,
        message: &str,
        attempt: u32,
        max_retries: u32,
    ) -> ErrorResolution;

    /// Evaluates a webhook verification failure, such as signature mismatch or timestamp drift.
    async fn on_webhook_error(
        &self,
        error: &StripeWebhookError,
        payload_len: usize,
    ) -> ErrorResolution;
}

/// Default built-in error workflow implementation.
///
/// - Retries on network timeouts, connection drops, and HTTP 429/500/502/503/504 with exponential backoff.
/// - Fails immediately on 400/401/403/404 client errors.
/// - Escalates webhook signature tampering and expired timestamps as [`ErrorSeverity::Critical`].
#[derive(Debug, Clone)]
pub struct DefaultErrorWorkflow {
    /// Maximum number of retries for transient errors.
    pub max_retries: u32,
    /// Initial base backoff delay.
    pub initial_delay: Duration,
    /// Multiplier applied to the backoff delay on each subsequent retry.
    pub backoff_multiplier: f64,
}

impl Default for DefaultErrorWorkflow {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(250),
            backoff_multiplier: 2.0,
        }
    }
}

impl DefaultErrorWorkflow {
    /// Creates a new default error workflow with standard retry settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Customizes the maximum retry attempts.
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Customizes the initial retry delay.
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    fn calculate_backoff(&self, attempt: u32) -> Duration {
        let factor = self
            .backoff_multiplier
            .powi(attempt.saturating_sub(1) as i32);
        Duration::from_secs_f64((self.initial_delay.as_secs_f64() * factor).min(30.0))
    }
}

#[async_trait]
impl ErrorWorkflowHandler for DefaultErrorWorkflow {
    async fn on_network_error(
        &self,
        error: &reqwest::Error,
        attempt: u32,
        max_retries: u32,
    ) -> ErrorResolution {
        if attempt < max_retries && (error.is_timeout() || error.is_connect()) {
            ErrorResolution::Retry {
                delay: self.calculate_backoff(attempt),
                next_attempt: attempt + 1,
            }
        } else {
            ErrorResolution::Fail {
                reason: format!("Network request failed after {attempt} attempts: {error}"),
            }
        }
    }

    async fn on_api_error(
        &self,
        status: u16,
        code: &str,
        message: &str,
        attempt: u32,
        max_retries: u32,
    ) -> ErrorResolution {
        // HTTP 429 Too Many Requests or 5xx Server Errors are retryable
        let is_transient = status == 429 || (500..=599).contains(&status);

        if is_transient && attempt < max_retries {
            ErrorResolution::Retry {
                delay: self.calculate_backoff(attempt),
                next_attempt: attempt + 1,
            }
        } else {
            ErrorResolution::Fail {
                reason: format!("Stripe API rejected request (HTTP {status}) [{code}]: {message}"),
            }
        }
    }

    async fn on_webhook_error(
        &self,
        error: &StripeWebhookError,
        payload_len: usize,
    ) -> ErrorResolution {
        match error {
            StripeWebhookError::BadSignature => ErrorResolution::Escalate {
                incident_id: format!("sec_tamper_{}", chrono::Utc::now().timestamp_micros()),
                severity: ErrorSeverity::Critical,
                message: format!(
                    "Cryptographic signature mismatch on payload of {payload_len} bytes"
                ),
            },
            StripeWebhookError::TimestampOutsideTolerance {
                timestamp,
                now,
                tolerance,
            } => ErrorResolution::Escalate {
                incident_id: format!("sec_replay_{}", chrono::Utc::now().timestamp_micros()),
                severity: ErrorSeverity::Critical,
                message: format!(
                    "Webhook timestamp {timestamp} outside acceptable tolerance window of {tolerance}s (now {now})"
                ),
            },
            other => ErrorResolution::Fail {
                reason: format!("Webhook verification failed: {other}"),
            },
        }
    }
}

/// Hook invoked on webhook security anomalies.
pub type StripeSecurityHook = Arc<dyn Fn(&StripeWebhookError, usize) + Send + Sync>;
/// Hook invoked on Stripe API errors.
pub type StripeApiErrorHook = Arc<dyn Fn(u16, &str, &str) + Send + Sync>;

/// A builder for registering custom callbacks and creating customized error workflows.
#[derive(Clone, Default)]
pub struct CustomErrorWorkflowBuilder {
    max_retries: u32,
    initial_delay: Duration,
    on_security_hook: Option<StripeSecurityHook>,
    on_api_error_hook: Option<StripeApiErrorHook>,
}

impl Debug for CustomErrorWorkflowBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomErrorWorkflowBuilder")
            .field("max_retries", &self.max_retries)
            .field("initial_delay", &self.initial_delay)
            .finish()
    }
}

impl CustomErrorWorkflowBuilder {
    /// Creates a new builder for a custom error workflow.
    pub fn new() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(250),
            on_security_hook: None,
            on_api_error_hook: None,
        }
    }

    /// Sets the maximum number of retry attempts.
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Sets the initial backoff delay.
    pub fn initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Registers a custom listener invoked whenever a security violation occurs.
    pub fn on_security_incident<F>(mut self, hook: F) -> Self
    where
        F: Fn(&StripeWebhookError, usize) + Send + Sync + 'static,
    {
        self.on_security_hook = Some(Arc::new(hook));
        self
    }

    /// Registers a custom listener invoked on API errors.
    pub fn on_api_error<F>(mut self, hook: F) -> Self
    where
        F: Fn(u16, &str, &str) + Send + Sync + 'static,
    {
        self.on_api_error_hook = Some(Arc::new(hook));
        self
    }

    /// Builds the custom [`ErrorWorkflowHandler`].
    pub fn build(self) -> Arc<dyn ErrorWorkflowHandler> {
        Arc::new(CustomErrorWorkflow {
            max_retries: self.max_retries,
            initial_delay: self.initial_delay,
            on_security_hook: self.on_security_hook,
            on_api_error_hook: self.on_api_error_hook,
        })
    }
}

#[derive(Clone)]
struct CustomErrorWorkflow {
    max_retries: u32,
    initial_delay: Duration,
    on_security_hook: Option<StripeSecurityHook>,
    on_api_error_hook: Option<StripeApiErrorHook>,
}

impl Debug for CustomErrorWorkflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomErrorWorkflow")
            .field("max_retries", &self.max_retries)
            .field("initial_delay", &self.initial_delay)
            .finish()
    }
}

#[async_trait]
impl ErrorWorkflowHandler for CustomErrorWorkflow {
    async fn on_network_error(
        &self,
        error: &reqwest::Error,
        attempt: u32,
        max_retries: u32,
    ) -> ErrorResolution {
        if attempt < max_retries && (error.is_timeout() || error.is_connect()) {
            ErrorResolution::Retry {
                delay: self.initial_delay * attempt,
                next_attempt: attempt + 1,
            }
        } else {
            ErrorResolution::Fail {
                reason: format!("Custom workflow: Network error after {attempt} attempts: {error}"),
            }
        }
    }

    async fn on_api_error(
        &self,
        status: u16,
        code: &str,
        message: &str,
        attempt: u32,
        max_retries: u32,
    ) -> ErrorResolution {
        if let Some(hook) = &self.on_api_error_hook {
            hook(status, code, message);
        }

        if (status == 429 || status >= 500) && attempt < max_retries {
            ErrorResolution::Retry {
                delay: self.initial_delay * attempt,
                next_attempt: attempt + 1,
            }
        } else {
            ErrorResolution::Fail {
                reason: format!("Custom workflow: API error HTTP {status} [{code}]: {message}"),
            }
        }
    }

    async fn on_webhook_error(
        &self,
        error: &StripeWebhookError,
        payload_len: usize,
    ) -> ErrorResolution {
        if let Some(hook) = &self.on_security_hook {
            hook(error, payload_len);
        }

        match error {
            StripeWebhookError::BadSignature
            | StripeWebhookError::TimestampOutsideTolerance { .. } => ErrorResolution::Escalate {
                incident_id: format!(
                    "custom_sec_incident_{}",
                    chrono::Utc::now().timestamp_micros()
                ),
                severity: ErrorSeverity::Critical,
                message: format!("Custom workflow detected security violation: {error}"),
            },
            _ => ErrorResolution::Fail {
                reason: format!("Custom workflow: webhook error: {error}"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[tokio::test]
    async fn test_default_workflow_retries_transient_500() {
        let workflow = DefaultErrorWorkflow::new();
        let resolution = workflow
            .on_api_error(503, "service_unavailable", "Under heavy load", 1, 3)
            .await;

        assert!(matches!(resolution, ErrorResolution::Retry { .. }));
    }

    #[tokio::test]
    async fn test_default_workflow_fails_on_400() {
        let workflow = DefaultErrorWorkflow::new();
        let resolution = workflow
            .on_api_error(400, "invalid_request", "Missing parameter", 1, 3)
            .await;

        assert!(matches!(resolution, ErrorResolution::Fail { .. }));
    }

    #[tokio::test]
    async fn test_default_workflow_escalates_tampered_webhook() {
        let workflow = DefaultErrorWorkflow::new();
        let resolution = workflow
            .on_webhook_error(&StripeWebhookError::BadSignature, 128)
            .await;

        match resolution {
            ErrorResolution::Escalate {
                severity, message, ..
            } => {
                assert_eq!(severity, ErrorSeverity::Critical);
                assert!(message.contains("signature mismatch"));
            }
            _ => panic!("Expected Escalate resolution"),
        }
    }

    #[tokio::test]
    async fn test_custom_workflow_builder_hooks() {
        let hook_called = Arc::new(AtomicBool::new(false));
        let hook_clone = hook_called.clone();

        let workflow = CustomErrorWorkflowBuilder::new()
            .max_retries(2)
            .on_security_incident(move |_err, _len| {
                hook_clone.store(true, Ordering::SeqCst);
            })
            .build();

        let res = workflow
            .on_webhook_error(&StripeWebhookError::BadSignature, 256)
            .await;

        assert!(hook_called.load(Ordering::SeqCst));
        assert!(matches!(res, ErrorResolution::Escalate { .. }));
    }
}
