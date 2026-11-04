//! Idempotency key manager ensuring robust API retry safety.

use crate::error::StripeError;

/// Idempotency key validator per the Stripe API specification.
pub struct IdempotencyKeyManager;

impl IdempotencyKeyManager {
    /// Validates an idempotency key string.
    pub fn validate(key: &str) -> Result<(), StripeError> {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return Err(StripeError::InvalidRequest(
                "Idempotency key must not be empty or whitespace".into(),
            ));
        }

        if trimmed.len() > 255 {
            return Err(StripeError::InvalidRequest(format!(
                "Idempotency key length {} exceeds Stripe limit of 255 characters",
                trimmed.len()
            )));
        }

        if !trimmed.is_ascii() {
            return Err(StripeError::InvalidRequest(
                "Idempotency key must contain only valid ASCII characters".into(),
            ));
        }

        Ok(())
    }
}
