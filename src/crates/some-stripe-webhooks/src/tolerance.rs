//! Replay attack protection and configurable timestamp drift tolerance.
//!
//! Provides [`ToleranceConfig`] to validate webhook event timestamps against current system time,
//! enforcing past expiration tolerance and guarding against excessive future clock drift.
//!
//! ## Replay Invariant (Gate G2)
//!
//! An event timestamp `t` is valid if and only if:
//!
//! ```text
//! current_timestamp - past_tolerance_seconds <= t <= current_timestamp + max_future_drift_seconds
//! ```
//!
//! Events outside this range are rejected with [`StripeWebhookError::TimestampOutsideTolerance`].

pub use crate::constants::{DEFAULT_TOLERANCE_SECONDS, MAX_FUTURE_DRIFT_SECONDS};
use some_stripe_core::error::StripeWebhookError;

/// Configuration defining timestamp tolerance bounds for webhook verification.
///
/// Enforces Gate G2 replay invariants: an event timestamp `t` is valid if and only if
/// `current - past_tolerance <= t <= current + max_future_drift`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToleranceConfig {
    /// Maximum age allowed for an event in seconds before it is rejected as expired.
    pub past_tolerance_seconds: u64,
    /// Maximum allowed future drift in seconds to protect against pre-dated events and clock skew.
    pub max_future_drift_seconds: i64,
}

impl Default for ToleranceConfig {
    /// Creates a default tolerance configuration:
    /// - `past_tolerance_seconds`: 300 (5 minutes)
    /// - `max_future_drift_seconds`: 60 (1 minute)
    fn default() -> Self {
        Self {
            past_tolerance_seconds: DEFAULT_TOLERANCE_SECONDS,
            max_future_drift_seconds: MAX_FUTURE_DRIFT_SECONDS,
        }
    }
}

impl ToleranceConfig {
    /// Constructs a `ToleranceConfig` with a custom past tolerance and standard 60s future drift guard.
    #[must_use]
    pub const fn new(past_tolerance_seconds: u64) -> Self {
        Self {
            past_tolerance_seconds,
            max_future_drift_seconds: MAX_FUTURE_DRIFT_SECONDS,
        }
    }

    /// Constructs a `ToleranceConfig` with custom past tolerance and custom future drift bounds.
    #[must_use]
    pub const fn with_bounds(past_tolerance_seconds: u64, max_future_drift_seconds: i64) -> Self {
        Self {
            past_tolerance_seconds,
            max_future_drift_seconds,
        }
    }

    /// Validates whether an `event_timestamp` falls within the permissible time window
    /// relative to `current_timestamp`.
    ///
    /// # Invariants (Gate G2)
    /// Valid if and only if:
    /// `current_timestamp - past_tolerance_seconds <= event_timestamp <= current_timestamp + max_future_drift_seconds`
    ///
    /// # Errors
    /// Returns [`StripeWebhookError::TimestampOutsideTolerance`] if `event_timestamp` is older than
    /// the past tolerance window or newer than the future drift window.
    pub fn check_timestamp(
        &self,
        event_timestamp: i64,
        current_timestamp: i64,
    ) -> Result<(), StripeWebhookError> {
        if self.past_tolerance_seconds > 0 {
            let lower_bound = current_timestamp.saturating_sub(self.past_tolerance_seconds as i64);
            if event_timestamp < lower_bound {
                return Err(StripeWebhookError::TimestampOutsideTolerance {
                    timestamp: event_timestamp,
                    now: current_timestamp,
                    tolerance: self.past_tolerance_seconds,
                });
            }
        }

        let upper_bound = current_timestamp.saturating_add(self.max_future_drift_seconds);
        if event_timestamp > upper_bound {
            return Err(StripeWebhookError::TimestampOutsideTolerance {
                timestamp: event_timestamp,
                now: current_timestamp,
                tolerance: self.past_tolerance_seconds,
            });
        }

        Ok(())
    }
}

impl From<u64> for ToleranceConfig {
    fn from(past_tolerance_seconds: u64) -> Self {
        Self::new(past_tolerance_seconds)
    }
}

impl From<Option<u64>> for ToleranceConfig {
    fn from(opt: Option<u64>) -> Self {
        match opt {
            Some(past) => Self::new(past),
            None => Self::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_timestamp_within_bounds() {
        let config = ToleranceConfig::default();
        let now = 1_700_000_000;

        // Valid timestamp: 45s in the past
        assert!(config.check_timestamp(now - 45, now).is_ok());

        // Valid timestamp: exactly now
        assert!(config.check_timestamp(now, now).is_ok());

        // Valid timestamp: 45s in the future (within 60s future drift)
        assert!(config.check_timestamp(now + 45, now).is_ok());

        // Boundary exact check: exactly at past tolerance limit (now - 300)
        assert!(config.check_timestamp(now - 300, now).is_ok());

        // Boundary exact check: exactly at future drift limit (now + 60)
        assert!(config.check_timestamp(now + 60, now).is_ok());
    }

    #[test]
    fn test_expired_timestamp_rejected() {
        let config = ToleranceConfig::default();
        let now = 1_700_000_000;

        // 301 seconds in past -> rejected
        let result = config.check_timestamp(now - 301, now);
        assert_eq!(
            result,
            Err(StripeWebhookError::TimestampOutsideTolerance {
                timestamp: now - 301,
                now,
                tolerance: 300,
            })
        );
    }

    #[test]
    fn test_distant_future_timestamp_rejected() {
        let config = ToleranceConfig::default();
        let now = 1_700_000_000;

        // 61 seconds in future -> rejected
        let result = config.check_timestamp(now + 61, now);
        assert_eq!(
            result,
            Err(StripeWebhookError::TimestampOutsideTolerance {
                timestamp: now + 61,
                now,
                tolerance: 300,
            })
        );
    }

    #[test]
    fn test_configurable_custom_tolerance() {
        let config = ToleranceConfig::new(600);
        let now = 1_700_000_000;

        // 500s in past is valid with 600s tolerance
        assert!(config.check_timestamp(now - 500, now).is_ok());

        // 601s in past is rejected
        let err = config.check_timestamp(now - 601, now).unwrap_err();
        assert_eq!(
            err,
            StripeWebhookError::TimestampOutsideTolerance {
                timestamp: now - 601,
                now,
                tolerance: 600,
            }
        );
    }

    #[test]
    fn test_custom_bounds() {
        let config = ToleranceConfig::with_bounds(120, 10);
        let now = 1_700_000_000;

        assert!(config.check_timestamp(now - 120, now).is_ok());
        assert!(config.check_timestamp(now + 10, now).is_ok());

        assert!(config.check_timestamp(now - 121, now).is_err());
        assert!(config.check_timestamp(now + 11, now).is_err());
    }
}
