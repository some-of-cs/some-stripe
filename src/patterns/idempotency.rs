//! Reusable idempotency key validation, generation, and lifecycle management patterns.
//!
//! # Purpose
//! Ensures that mutations sent to Stripe adhere to idempotency key specifications (1–255 ASCII characters)
//! and prevents duplicate operations across network retry cycles.

use crate::error::StripeError;

/// Manages idempotency key validation and formatting according to Stripe specifications.
///
/// # Invariants
/// - Keys must be between 1 and 255 printable ASCII characters.
/// - Control characters and whitespace-only strings are rejected.
///
/// # Examples
///
/// ```
/// use some_stripe::patterns::idempotency::IdempotencyKeyManager;
///
/// assert!(IdempotencyKeyManager::validate("order_12345").is_ok());
/// assert!(IdempotencyKeyManager::validate("").is_err());
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct IdempotencyKeyManager;

impl IdempotencyKeyManager {
    /// Validates an idempotency key according to Stripe constraints.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if the key is empty, exceeds 255 characters,
    /// or contains non-ASCII or control characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use some_stripe::patterns::idempotency::IdempotencyKeyManager;
    ///
    /// assert!(IdempotencyKeyManager::validate("valid-uuid-v4-key").is_ok());
    /// assert!(IdempotencyKeyManager::validate("   ").is_err());
    /// ```
    pub fn validate(key: &str) -> Result<(), StripeError> {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return Err(StripeError::InvalidRequest(
                "Idempotency key must not be empty or whitespace only".into(),
            ));
        }

        if key.len() > 255 {
            return Err(StripeError::InvalidRequest(format!(
                "Idempotency key length exceeds maximum of 255 characters (length: {})",
                key.len()
            )));
        }

        if !key.is_ascii() || key.chars().any(|c| c.is_ascii_control()) {
            return Err(StripeError::InvalidRequest(
                "Idempotency key must contain only printable ASCII characters".into(),
            ));
        }

        Ok(())
    }

    /// Derives an idempotency key from a transaction namespace and reference ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use some_stripe::patterns::idempotency::IdempotencyKeyManager;
    ///
    /// let key = IdempotencyKeyManager::scoped_key("checkout", "cart_9876");
    /// assert_eq!(key, "checkout:cart_9876");
    /// ```
    #[inline]
    pub fn scoped_key(namespace: &str, reference_id: &str) -> String {
        format!("{namespace}:{reference_id}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_idempotency_keys() {
        assert!(IdempotencyKeyManager::validate("order_123").is_ok());
        assert!(IdempotencyKeyManager::validate("idem-uuid-v4-abc-123").is_ok());
        assert!(IdempotencyKeyManager::validate("a").is_ok());
    }

    #[test]
    fn test_invalid_idempotency_keys() {
        assert!(IdempotencyKeyManager::validate("").is_err());
        assert!(IdempotencyKeyManager::validate("   \t\n ").is_err());

        let long_key = "a".repeat(256);
        assert!(IdempotencyKeyManager::validate(&long_key).is_err());

        assert!(IdempotencyKeyManager::validate("key_with_\u{1F600}").is_err());
        assert!(IdempotencyKeyManager::validate("key_with_\x00_null").is_err());
    }

    #[test]
    fn test_scoped_key_generation() {
        assert_eq!(
            IdempotencyKeyManager::scoped_key("payment", "tx_555"),
            "payment:tx_555"
        );
    }
}
