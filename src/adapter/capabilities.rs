//! Orthogonal `some-payment` capability trait contracts and unified error types.
//!
//! Defines zero-dependency, composable gateway capability traits that allow
//! downstream routers (such as `some-payment`) to depend only on the specific
//! Stripe capabilities they require (`PaymentCapability`, `KycCapability`,
//! `PayoutCapability`, `WebhookCapability`, and the [`StripeExt`] escape hatch).

use async_trait::async_trait;
use some_stripe_core::error::{StripeError, StripeWebhookError};
use some_stripe_types::identity_session::{CreateVerificationSessionParams, VerificationSession};
use some_stripe_types::{
    CancelPaymentIntentParams, CapturePaymentIntentParams, CreatePaymentIntentParams,
    CreatePayoutParams, CreateRefundParams, Event, PaymentIntent, Payout, Refund,
};
use thiserror::Error;

use crate::StripeClient;

/// Unified gateway error returned by all `some-payment` capability adapter operations.
///
/// Wraps underlying [`StripeError`] and [`StripeWebhookError`] variants while exposing
/// provider-agnostic retryability, HTTP status, and error code inspection helpers.
#[derive(Debug, Error)]
pub enum UnifiedGatewayError {
    /// An error returned by the Stripe REST API or HTTP transport layer.
    #[error("stripe gateway error: {0}")]
    Stripe(#[from] StripeError),

    /// An error occurring during fail-closed webhook signature verification or parsing.
    #[error("stripe webhook verification error: {0}")]
    Webhook(#[from] StripeWebhookError),
}

impl UnifiedGatewayError {
    /// Returns `true` if this error is transient and safe to retry.
    ///
    /// Delegates to [`StripeError::is_retryable`] for API/transport errors and
    /// always returns `false` for cryptographic webhook verification errors.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Stripe(err) => err.is_retryable(),
            Self::Webhook(_) => false,
        }
    }

    /// Returns the HTTP status code associated with this error, if available.
    #[must_use]
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::Stripe(err) => err.status_code(),
            Self::Webhook(_) => None,
        }
    }

    /// Returns the provider-specific error code (such as `rate_limit` or `card_declined`), if available.
    #[must_use]
    pub fn provider_code(&self) -> Option<&str> {
        match self {
            Self::Stripe(err) => err.stripe_code(),
            Self::Webhook(_) => Some("webhook_verification_failed"),
        }
    }
}

/// Core payment lifecycle capability for creating, capturing, canceling, and refunding payments.
#[async_trait]
pub trait PaymentCapability: Send + Sync {
    /// Creates a new Stripe [`PaymentIntent`].
    ///
    /// # Errors
    ///
    /// Returns [`UnifiedGatewayError`] if parameter validation, HTTP transport,
    /// or the Stripe API rejects the request.
    async fn create_payment_intent(
        &self,
        params: &CreatePaymentIntentParams,
    ) -> Result<PaymentIntent, UnifiedGatewayError>;

    /// Captures funds for an authorized [`PaymentIntent`].
    ///
    /// # Errors
    ///
    /// Returns [`UnifiedGatewayError`] if the intent cannot be captured or the
    /// Stripe API returns an error.
    async fn capture_payment_intent(
        &self,
        intent_id: &str,
        params: Option<&CapturePaymentIntentParams>,
    ) -> Result<PaymentIntent, UnifiedGatewayError>;

    /// Cancels an active [`PaymentIntent`].
    ///
    /// # Errors
    ///
    /// Returns [`UnifiedGatewayError`] if the intent cannot be canceled or the
    /// Stripe API returns an error.
    async fn cancel_payment_intent(
        &self,
        intent_id: &str,
        params: Option<&CancelPaymentIntentParams>,
    ) -> Result<PaymentIntent, UnifiedGatewayError>;

    /// Issues a full or partial [`Refund`] against a charge or payment intent.
    ///
    /// # Errors
    ///
    /// Returns [`UnifiedGatewayError`] if the refund parameters are invalid or
    /// the Stripe API rejects the request.
    async fn refund_payment(
        &self,
        params: &CreateRefundParams,
    ) -> Result<Refund, UnifiedGatewayError>;
}

/// Identity verification (KYC) capability backed by Stripe Identity verification sessions.
#[async_trait]
pub trait KycCapability: Send + Sync {
    /// Creates a new Stripe Identity [`VerificationSession`].
    ///
    /// # Errors
    ///
    /// Returns [`UnifiedGatewayError`] if session creation fails or returns an API error.
    async fn create_verification_session(
        &self,
        params: &CreateVerificationSessionParams,
    ) -> Result<VerificationSession, UnifiedGatewayError>;

    /// Retrieves an existing Stripe Identity [`VerificationSession`] by ID.
    ///
    /// # Errors
    ///
    /// Returns [`UnifiedGatewayError`] if the session ID is invalid or retrieval fails.
    async fn get_verification_session(
        &self,
        session_id: &str,
    ) -> Result<VerificationSession, UnifiedGatewayError>;

    /// Checks whether the specified verification session has reached the verified state.
    ///
    /// # Errors
    ///
    /// Returns [`UnifiedGatewayError`] if retrieving the underlying session fails.
    async fn is_verified(&self, session_id: &str) -> Result<bool, UnifiedGatewayError>;
}

/// Merchant and connected-account payout capability backed by Stripe Payouts.
#[async_trait]
pub trait PayoutCapability: Send + Sync {
    /// Creates a new Stripe [`Payout`].
    ///
    /// # Errors
    ///
    /// Returns [`UnifiedGatewayError`] if payout creation fails or returns an API error.
    async fn create_payout(
        &self,
        params: &CreatePayoutParams,
    ) -> Result<Payout, UnifiedGatewayError>;

    /// Retrieves an existing Stripe [`Payout`] by ID.
    ///
    /// # Errors
    ///
    /// Returns [`UnifiedGatewayError`] if the payout ID is invalid or retrieval fails.
    async fn get_payout(&self, payout_id: &str) -> Result<Payout, UnifiedGatewayError>;
}

/// Fail-closed webhook signature verification and event parsing capability.
pub trait WebhookCapability: Send + Sync {
    /// Verifies the `Stripe-Signature` header using constant-time `HMAC-SHA256`
    /// comparison and deserializes the verified [`Event`] payload.
    ///
    /// # Errors
    ///
    /// Returns [`UnifiedGatewayError::Webhook`] if the webhook secret is blank,
    /// the signature header is malformed or forged, the timestamp is outside the
    /// tolerance window, or the JSON payload cannot be parsed.
    fn verify_and_parse_event(
        &self,
        payload: &[u8],
        sig_header: &str,
        secret: &str,
        tolerance: Option<u64>,
    ) -> Result<Event, UnifiedGatewayError>;
}

/// Escape-hatch extension trait exposing direct access to the underlying [`StripeClient`].
pub trait StripeExt {
    /// Returns a shared reference to the underlying [`StripeClient`].
    fn stripe_client(&self) -> &StripeClient;
}
