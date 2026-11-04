//! Security Incident Escalation Hook Bridge for Webhook Failures.
//!
//! Routes cryptographic signature mismatches, timestamp tolerance drift violations,
//! and payload exhaustion attacks to the [`ErrorWorkflowHandler`] with [`ErrorSeverity::Critical`].

use some_stripe_core::error::StripeWebhookError;
use some_stripe_core::workflow::{ErrorSeverity, ErrorWorkflowHandler};
use some_stripe_types::Event;

use crate::Webhook;
use crate::v2_events::V2ThinEvent;

/// Returns `true` if the specified [`StripeWebhookError`] represents a security incident
/// requiring immediate developer or security operations escalation.
///
/// Security incidents include:
/// - [`StripeWebhookError::BadSignature`]: Potential request forgery, secret compromise, or payload tampering.
/// - [`StripeWebhookError::TimestampOutsideTolerance`]: Potential replay attack or severe clock skew.
/// - [`StripeWebhookError::PayloadTooLarge`]: Potential denial-of-service or memory exhaustion vector.
///
/// # Example
///
/// ```
/// use some_stripe_webhooks::escalation::is_security_incident;
/// use some_stripe_core::error::StripeWebhookError;
///
/// assert!(is_security_incident(&StripeWebhookError::BadSignature));
/// ```
pub fn is_security_incident(err: &StripeWebhookError) -> bool {
    matches!(
        err,
        StripeWebhookError::BadSignature
            | StripeWebhookError::TimestampOutsideTolerance { .. }
            | StripeWebhookError::PayloadTooLarge { .. }
    )
}

/// Categorizes a [`StripeWebhookError`] into an appropriate [`ErrorSeverity`].
///
/// - [`StripeWebhookError::BadSignature`], [`StripeWebhookError::TimestampOutsideTolerance`],
///   and [`StripeWebhookError::PayloadTooLarge`] map to [`ErrorSeverity::Critical`].
/// - All other parsing or structural errors map to [`ErrorSeverity::Error`].
///
/// # Example
///
/// ```
/// use some_stripe_webhooks::escalation::determine_severity;
/// use some_stripe_core::error::StripeWebhookError;
/// use some_stripe_core::workflow::ErrorSeverity;
///
/// assert_eq!(
///     determine_severity(&StripeWebhookError::BadSignature),
///     ErrorSeverity::Critical
/// );
/// ```
pub fn determine_severity(err: &StripeWebhookError) -> ErrorSeverity {
    if is_security_incident(err) {
        ErrorSeverity::Critical
    } else {
        ErrorSeverity::Error
    }
}

/// Escalates a webhook verification error to an [`ErrorWorkflowHandler`].
///
/// Critical security incidents (forgery, replay attacks, payload overflow) are dispatched with
/// [`ErrorSeverity::Critical`] and a detailed diagnostic explanation.
pub async fn escalate_webhook_error<H: ErrorWorkflowHandler + ?Sized>(
    handler: &H,
    error: &StripeWebhookError,
) {
    let severity = determine_severity(error);
    let reason = match error {
        StripeWebhookError::BadSignature => {
            "CRITICAL SECURITY ALERT: Webhook cryptographic signature mismatch (HMAC-SHA256 verification failed; possible payload tampering or forged webhook)".to_string()
        }
        StripeWebhookError::TimestampOutsideTolerance {
            timestamp,
            now,
            tolerance,
        } => {
            format!(
                "CRITICAL SECURITY ALERT: Webhook timestamp drift outside tolerance window (event timestamp {timestamp} vs current time {now}, tolerance {tolerance}s; potential replay attack)"
            )
        }
        StripeWebhookError::PayloadTooLarge { size, max_allowed } => {
            format!(
                "CRITICAL SECURITY ALERT: Webhook payload size {size} bytes exceeds maximum limit {max_allowed} bytes (potential memory exhaustion attack)"
            )
        }
        StripeWebhookError::MissingSignatureHeader => {
            "Webhook verification failed: missing Stripe-Signature header".to_string()
        }
        StripeWebhookError::MissingSecret => {
            "Webhook verification failed: missing webhook signing secret".to_string()
        }
        StripeWebhookError::EmptyPayload => {
            "Webhook verification failed: empty payload received".to_string()
        }
        StripeWebhookError::InvalidTimestamp => {
            "Webhook verification failed: invalid timestamp in Stripe-Signature header".to_string()
        }
        StripeWebhookError::MissingSignature => {
            "Webhook verification failed: no v1 signature scheme found in header".to_string()
        }
        StripeWebhookError::MalformedPayload(msg) => {
            format!("Webhook verification failed: malformed payload JSON: {msg}")
        }
        StripeWebhookError::BadKey => {
            "Webhook verification failed: invalid or blank webhook signing secret".to_string()
        }
    };

    handler.on_webhook_error(severity, &reason).await;
}

/// A security-aware webhook verification bridge that dispatches security violations
/// and errors directly to an [`ErrorWorkflowHandler`].
#[derive(Debug, Clone)]
pub struct WebhookEscalationBridge<H: ErrorWorkflowHandler> {
    handler: H,
}

impl<H: ErrorWorkflowHandler> WebhookEscalationBridge<H> {
    /// Creates a new [`WebhookEscalationBridge`] wrapping the provided [`ErrorWorkflowHandler`].
    pub fn new(handler: H) -> Self {
        Self { handler }
    }

    /// Returns a reference to the underlying [`ErrorWorkflowHandler`].
    pub fn handler(&self) -> &H {
        &self.handler
    }

    /// Verifies the webhook signature header, escalating any verification failure
    /// to the inner error workflow handler.
    pub async fn verify_header(
        &self,
        payload: &[u8],
        header: &str,
        secret: &str,
        tolerance: Option<u64>,
    ) -> Result<bool, StripeWebhookError> {
        match Webhook::verify_header(payload, header, secret, tolerance) {
            Ok(v) => Ok(v),
            Err(err) => {
                escalate_webhook_error(&self.handler, &err).await;
                Err(err)
            }
        }
    }

    /// Verifies the webhook and constructs an API v1 [`Event`], escalating any failure.
    pub async fn construct_event(
        &self,
        payload: &[u8],
        sig_header: &str,
        secret: &str,
        tolerance: Option<u64>,
    ) -> Result<Event, StripeWebhookError> {
        match Webhook::construct_event(payload, sig_header, secret, tolerance) {
            Ok(event) => Ok(event),
            Err(err) => {
                escalate_webhook_error(&self.handler, &err).await;
                Err(err)
            }
        }
    }

    /// Verifies the webhook and constructs an API v2 [`V2ThinEvent`], escalating any failure.
    pub async fn construct_v2_thin_event(
        &self,
        payload: &[u8],
        sig_header: &str,
        secret: &str,
        tolerance: Option<u64>,
    ) -> Result<V2ThinEvent, StripeWebhookError> {
        match Webhook::construct_v2_thin_event(payload, sig_header, secret, tolerance) {
            Ok(v2_event) => Ok(v2_event),
            Err(err) => {
                escalate_webhook_error(&self.handler, &err).await;
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use some_stripe_core::workflow::ErrorAction;
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct MockEscalationHandler {
        escalations: Mutex<Vec<(ErrorSeverity, String)>>,
    }

    #[async_trait]
    impl ErrorWorkflowHandler for MockEscalationHandler {
        async fn on_api_error(
            &self,
            _status: u16,
            _code: &str,
            _message: &str,
            _attempt: u32,
            _max_retries: u32,
        ) -> ErrorAction {
            ErrorAction::Fail
        }

        async fn on_webhook_error(&self, severity: ErrorSeverity, reason: &str) {
            self.escalations
                .lock()
                .unwrap()
                .push((severity, reason.to_string()));
        }
    }

    #[tokio::test]
    async fn test_escalate_bad_signature_critical() {
        let handler = MockEscalationHandler::default();
        let error = StripeWebhookError::BadSignature;

        assert!(is_security_incident(&error));
        assert_eq!(determine_severity(&error), ErrorSeverity::Critical);

        escalate_webhook_error(&handler, &error).await;

        let records = handler.escalations.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].0, ErrorSeverity::Critical);
        assert!(records[0].1.contains("CRITICAL SECURITY ALERT"));
        assert!(records[0].1.contains("signature mismatch"));
    }

    #[tokio::test]
    async fn test_escalate_timestamp_tolerance_drift_critical() {
        let handler = MockEscalationHandler::default();
        let error = StripeWebhookError::TimestampOutsideTolerance {
            timestamp: 1000,
            now: 2000,
            tolerance: 300,
        };

        assert!(is_security_incident(&error));
        assert_eq!(determine_severity(&error), ErrorSeverity::Critical);

        escalate_webhook_error(&handler, &error).await;

        let records = handler.escalations.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].0, ErrorSeverity::Critical);
        assert!(records[0].1.contains("CRITICAL SECURITY ALERT"));
        assert!(records[0].1.contains("replay attack"));
    }

    #[tokio::test]
    async fn test_escalate_payload_too_large_critical() {
        let handler = MockEscalationHandler::default();
        let error = StripeWebhookError::PayloadTooLarge {
            size: 2_000_000,
            max_allowed: 1_048_576,
        };

        assert!(is_security_incident(&error));
        assert_eq!(determine_severity(&error), ErrorSeverity::Critical);

        escalate_webhook_error(&handler, &error).await;

        let records = handler.escalations.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].0, ErrorSeverity::Critical);
        assert!(records[0].1.contains("memory exhaustion"));
    }

    #[tokio::test]
    async fn test_escalate_malformed_payload_error() {
        let handler = MockEscalationHandler::default();
        let error = StripeWebhookError::MalformedPayload("syntax error".to_string());

        assert!(!is_security_incident(&error));
        assert_eq!(determine_severity(&error), ErrorSeverity::Error);

        escalate_webhook_error(&handler, &error).await;

        let records = handler.escalations.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].0, ErrorSeverity::Error);
    }

    #[tokio::test]
    async fn test_webhook_escalation_bridge_fires_on_forged_signature() {
        let handler = MockEscalationHandler::default();
        let bridge = WebhookEscalationBridge::new(handler);

        let payload = b"{\"id\":\"v2_evt_test\"}";
        let now = chrono::Utc::now().timestamp();
        let forged_header =
            format!("t={now},v1=0000000000000000000000000000000000000000000000000000000000000000");
        let secret = "whsec_test";

        let result = bridge
            .verify_header(payload, &forged_header, secret, None)
            .await;
        assert_eq!(result.unwrap_err(), StripeWebhookError::BadSignature);

        let escalations = bridge.handler().escalations.lock().unwrap();
        assert_eq!(escalations.len(), 1);
        assert_eq!(escalations[0].0, ErrorSeverity::Critical);
    }

    #[tokio::test]
    async fn test_webhook_escalation_bridge_v2_thin_event_success() {
        let handler = MockEscalationHandler::default();
        let bridge = WebhookEscalationBridge::new(handler);

        let payload = br#"{"id":"v2_evt_ok","object":"v2.core.event","type":"test.event","created":1728345600,"livemode":false}"#;
        let secret = "whsec_ok";
        let now = chrono::Utc::now().timestamp();
        let header = Webhook::generate_test_header_string(payload, secret, Some(now));

        let v2_event = bridge
            .construct_v2_thin_event(payload, &header, secret, Some(300))
            .await
            .expect("should construct v2 event");

        assert_eq!(v2_event.id, "v2_evt_ok");
        assert_eq!(v2_event.event_type, "test.event");

        let escalations = bridge.handler().escalations.lock().unwrap();
        assert!(
            escalations.is_empty(),
            "No escalation should occur on success"
        );
    }
}
