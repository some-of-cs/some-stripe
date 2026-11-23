//! Idempotency key manager ensuring robust API retry safety.

use crate::error::StripeError;
use crate::idempotency_passthrough::IdempotencyKey;

/// Idempotency key validator per the Stripe API specification.
pub struct IdempotencyKeyManager;

impl IdempotencyKeyManager {
    /// Validates an idempotency key string per the Stripe API specification.
    ///
    /// Rejects empty or whitespace-only keys, leading or trailing whitespace,
    /// keys longer than 255 bytes, and any non-printable ASCII characters
    /// (including CRLF, NUL, and control characters).
    pub fn validate(key: &str) -> Result<(), StripeError> {
        if key != key.trim() {
            return Err(StripeError::InvalidRequest(
                "Idempotency key must not contain leading or trailing whitespace".into(),
            ));
        }

        IdempotencyKey::try_new(key)
            .map(|_| ())
            .map_err(|err| match err {
                StripeError::InvalidRequest(msg) => StripeError::InvalidRequest(msg),
                other => StripeError::InvalidRequest(other.to_string()),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_accepts_valid_idempotency_keys() {
        assert!(IdempotencyKeyManager::validate("order_12345").is_ok());
        assert!(IdempotencyKeyManager::validate(&"a".repeat(255)).is_ok());
    }

    #[test]
    fn test_validate_rejects_empty_whitespace_and_oversized_keys() {
        let oversized = "a".repeat(256);
        for invalid in [
            "",
            "   ",
            " key_with_leading",
            "key_with_trailing ",
            oversized.as_str(),
        ] {
            let res = IdempotencyKeyManager::validate(invalid);
            assert!(matches!(res, Err(StripeError::InvalidRequest(_))));
        }
    }

    #[test]
    fn test_validate_rejects_control_chars_crlf_nul_and_non_ascii() {
        for invalid in [
            "key\r\nX-Inject: 1",
            "key\nline2",
            "key\0nul",
            "key\twith_tab",
            "key_é_unicode",
        ] {
            assert!(
                matches!(
                    IdempotencyKeyManager::validate(invalid),
                    Err(StripeError::InvalidRequest(_))
                ),
                "expected InvalidRequest for {invalid:?}"
            );
        }
    }
}
