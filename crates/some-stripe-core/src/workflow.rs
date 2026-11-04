//! Pluggable error handling, circuit breakers, and custom retry workflows.

use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::atomic::{AtomicU32, Ordering};

/// Severity levels for categorizing errors and security alerts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// Informational warning (e.g. transient 500 error that can be retried).
    Warning,
    /// Standard error condition requiring developer visibility.
    Error,
    /// Critical security alert (e.g. forged webhook signature, replay drift violation).
    Critical,
}

/// Actions returned by an error workflow handler instructing the execution pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorAction {
    /// Retry the failed HTTP operation with exponential backoff.
    Retry,
    /// Fail immediately and return the error to the caller without further retries.
    Fail,
}

/// Trait defining hook callbacks for API errors, webhook security violations, and retries.
#[async_trait]
pub trait ErrorWorkflowHandler: Send + Sync + Debug {
    /// Invoked whenever the Stripe REST API returns an HTTP 4xx or 5xx status code.
    async fn on_api_error(
        &self,
        status: u16,
        code: &str,
        message: &str,
        attempt: u32,
        max_retries: u32,
    ) -> ErrorAction;

    /// Invoked whenever a webhook verification failure occurs (signature mismatch, replay drift).
    async fn on_webhook_error(&self, severity: ErrorSeverity, reason: &str);
}

/// Default built-in error workflow implementation with automatic exponential backoff.
#[derive(Debug, Default)]
pub struct DefaultErrorWorkflow {
    retry_count: AtomicU32,
}

impl DefaultErrorWorkflow {
    /// Creates a new [`DefaultErrorWorkflow`].
    pub fn new() -> Self {
        Self {
            retry_count: AtomicU32::new(0),
        }
    }
}

#[async_trait]
impl ErrorWorkflowHandler for DefaultErrorWorkflow {
    async fn on_api_error(
        &self,
        status: u16,
        _code: &str,
        _message: &str,
        attempt: u32,
        max_retries: u32,
    ) -> ErrorAction {
        if attempt >= max_retries {
            return ErrorAction::Fail;
        }

        match status {
            // Rate limiting: retry with backoff
            429 => {
                self.retry_count.fetch_add(1, Ordering::SeqCst);
                ErrorAction::Retry
            }
            // Ephemeral server errors: retry
            500 | 502 | 503 | 504 => {
                self.retry_count.fetch_add(1, Ordering::SeqCst);
                ErrorAction::Retry
            }
            // Client errors: fail fast
            _ => ErrorAction::Fail,
        }
    }

    async fn on_webhook_error(&self, _severity: ErrorSeverity, _reason: &str) {
        // Default workflow ignores or logs; custom workflows can alert APM/PagerDuty
    }
}
