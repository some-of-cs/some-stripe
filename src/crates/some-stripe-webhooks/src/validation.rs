//! Webhook secret pre-validation and sanitization guards.

/// Evaluates whether a webhook signing secret is blank, empty, or consists solely of whitespace.
///
/// Under Gate G2 security rules, empty or whitespace-only keys must be rejected immediately
/// before performing any expensive cryptographic operations or allocations.
///
/// # Examples
///
/// ```
/// use some_stripe_webhooks::validation::is_blank_webhook_secret;
///
/// assert!(is_blank_webhook_secret(""));
/// assert!(is_blank_webhook_secret("   \t\n "));
/// assert!(!is_blank_webhook_secret("whsec_live_test_secret"));
/// ```
#[inline]
pub fn is_blank_webhook_secret(secret: &str) -> bool {
    secret.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blank_secrets_detected() {
        assert!(is_blank_webhook_secret(""));
        assert!(is_blank_webhook_secret(" "));
        assert!(is_blank_webhook_secret("   \t\r\n  "));
        assert!(!is_blank_webhook_secret("whsec_abc123"));
    }
}
