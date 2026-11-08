//! Stripe Billing Meter Event Session domain models and authentication tokens.
//!
//! Provides short-lived cryptographic authentication tokens for high-throughput
//! usage meter event stream reporting (`POST /v2/billing/meter_event_stream`).
//! Tokens have a 15-minute lifespan and enforce memory sanitization on drop
//! via [`zeroize::ZeroizeOnDrop`] (Gate G2 / PCI-DSS compliance).

use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Current lifecycle status of a Stripe Billing Meter Event Session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MeterEventSessionStatus {
    /// The session token is currently valid and active for streaming meter events.
    #[default]
    Active,
    /// The session token has exceeded its 15-minute validity window.
    Expired,
    /// The session token was explicitly invalidated before expiration.
    Revoked,
}

impl MeterEventSessionStatus {
    /// Returns the static string slice representation of the status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Revoked => "revoked",
        }
    }

    /// Returns `true` if the session is currently active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns `true` if the session is expired.
    #[must_use]
    pub const fn is_expired(&self) -> bool {
        matches!(self, Self::Expired)
    }

    /// Returns `true` if the session is revoked.
    #[must_use]
    pub const fn is_revoked(&self) -> bool {
        matches!(self, Self::Revoked)
    }
}

impl fmt::Display for MeterEventSessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Sensitive authentication token details for high-throughput meter event streaming.
///
/// Wraps the short-lived bearer token returned by `POST /v2/billing/meter_event_session`.
/// Implements [`Zeroize`] and [`ZeroizeOnDrop`] to ensure memory is securely zeroed upon
/// disposal, and redacts formatting in [`fmt::Debug`] and [`fmt::Display`] to prevent
/// token exposure in application logs.
///
/// # Examples
///
/// ```
/// use some_stripe_types::meter_session::SessionTokenDetails;
///
/// let token = SessionTokenDetails::new("token_test_abc123");
/// assert_eq!(token.expose_token(), "token_test_abc123");
/// assert_eq!(format!("{token:?}"), "[REDACTED]");
/// assert_eq!(format!("{token}"), "[REDACTED]");
/// ```
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionTokenDetails {
    token: String,
}

impl SessionTokenDetails {
    /// Creates a new `SessionTokenDetails` wrapping the provided sensitive token.
    #[must_use]
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }

    /// Returns a reference to the underlying bearer token string.
    ///
    /// # Security Invariant
    /// The caller is responsible for ensuring this reference does not leak
    /// into unredacted debug formatting or persistent storage.
    #[must_use]
    pub fn expose_token(&self) -> &str {
        &self.token
    }

    /// Returns the length of the token string.
    #[must_use]
    pub fn len(&self) -> usize {
        self.token.len()
    }

    /// Returns `true` if the token string is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.token.is_empty()
    }
}

impl fmt::Debug for SessionTokenDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl fmt::Display for SessionTokenDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl From<String> for SessionTokenDetails {
    fn from(token: String) -> Self {
        Self::new(token)
    }
}

impl From<&str> for SessionTokenDetails {
    fn from(token: &str) -> Self {
        Self::new(token)
    }
}

/// Stripe Billing Meter Event Session representation (`billing.meter_event_session`).
///
/// Created via `POST /v2/billing/meter_event_session` to supply short-lived credentials
/// for batch meter event ingestion streams.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeterEventSession {
    /// Unique identifier for the session (e.g. `bms_...` or `ses_...`).
    pub id: String,

    /// Object type descriptor, typically `"billing.meter_event_session"` or `"v2.billing.meter_event_session"`.
    pub object: String,

    /// Time at which the object was created (Unix timestamp in seconds).
    pub created: i64,

    /// Time at which the session authentication token expires (Unix timestamp in seconds).
    /// Typically 15 minutes (900 seconds) after `created`.
    pub expires_at: i64,

    /// Short-lived cryptographic bearer token used to authenticate event streams.
    pub authentication_token: SessionTokenDetails,

    /// Current operational status of this meter event session.
    #[serde(default)]
    pub status: MeterEventSessionStatus,

    /// Has the value `true` if the object exists in live mode or `false` for test mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub livemode: Option<bool>,
}

impl MeterEventSession {
    /// Returns `true` if the session is currently active based on status and timestamp.
    #[must_use]
    pub fn is_valid_at(&self, current_timestamp: i64) -> bool {
        self.status.is_active() && current_timestamp < self.expires_at
    }

    /// Returns the remaining lifespan of the session token in seconds.
    #[must_use]
    pub fn remaining_seconds(&self, current_timestamp: i64) -> i64 {
        (self.expires_at - current_timestamp).max(0)
    }
}

/// Request parameters for creating a new Meter Event Session (`POST /v2/billing/meter_event_session`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateMeterEventSessionParams {
    /// Optional idempotency key for network deduplication. Excluded from JSON payload.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateMeterEventSessionParams {
    /// Creates a new default set of parameters.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches an explicit idempotency key to the creation request.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Returns the attached idempotency key, if set.
    #[must_use]
    pub fn idempotency_key(&self) -> Option<&str> {
        self.idempotency_key.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_details_redaction() {
        let token = SessionTokenDetails::new("sensitive_token_value_987");
        assert_eq!(token.expose_token(), "sensitive_token_value_987");
        assert_eq!(format!("{token:?}"), "[REDACTED]");
        assert_eq!(format!("{token}"), "[REDACTED]");
        assert_eq!(token.len(), 25);
        assert!(!token.is_empty());
    }

    #[test]
    fn test_token_details_zeroize() {
        let mut token = SessionTokenDetails::new("secret_payload");
        token.zeroize();
        assert_eq!(token.expose_token(), "");
    }

    #[test]
    fn test_meter_event_session_serde() {
        let json = r#"{
            "id": "bms_123456789",
            "object": "billing.meter_event_session",
            "created": 1728345600,
            "expires_at": 1728346500,
            "authentication_token": "tok_xyz_secret_val",
            "status": "active",
            "livemode": false
        }"#;

        let session: MeterEventSession = serde_json::from_str(json).expect("valid session json");
        assert_eq!(session.id, "bms_123456789");
        assert_eq!(session.object, "billing.meter_event_session");
        assert_eq!(session.created, 1728345600);
        assert_eq!(session.expires_at, 1728346500);
        assert_eq!(
            session.authentication_token.expose_token(),
            "tok_xyz_secret_val"
        );
        assert_eq!(session.status, MeterEventSessionStatus::Active);
        assert_eq!(session.livemode, Some(false));

        // Test serialize roundtrip
        let serialized = serde_json::to_string(&session).expect("serialization succeeds");
        assert!(serialized.contains(r#""id":"bms_123456789""#));
        assert!(serialized.contains(r#""status":"active""#));
        assert!(serialized.contains(r#""authentication_token":"tok_xyz_secret_val""#));
    }

    #[test]
    fn test_meter_event_session_status_helpers() {
        let active = MeterEventSessionStatus::Active;
        assert!(active.is_active());
        assert!(!active.is_expired());
        assert!(!active.is_revoked());
        assert_eq!(active.to_string(), "active");

        let expired = MeterEventSessionStatus::Expired;
        assert!(!expired.is_active());
        assert!(expired.is_expired());
        assert_eq!(expired.to_string(), "expired");

        let revoked = MeterEventSessionStatus::Revoked;
        assert!(!revoked.is_active());
        assert!(revoked.is_revoked());
        assert_eq!(revoked.to_string(), "revoked");
    }

    #[test]
    fn test_meter_event_session_lifetime_checks() {
        let session = MeterEventSession {
            id: "bms_test".into(),
            object: "billing.meter_event_session".into(),
            created: 1000,
            expires_at: 1900,
            authentication_token: SessionTokenDetails::new("tok"),
            status: MeterEventSessionStatus::Active,
            livemode: None,
        };

        assert!(session.is_valid_at(1500));
        assert!(!session.is_valid_at(1900));
        assert!(!session.is_valid_at(2000));
        assert_eq!(session.remaining_seconds(1500), 400);
        assert_eq!(session.remaining_seconds(2000), 0);
    }

    #[test]
    fn test_create_params_builder() {
        let params = CreateMeterEventSessionParams::new().with_idempotency_key("idem_123");
        assert_eq!(params.idempotency_key(), Some("idem_123"));

        // Idempotency key should be skipped during JSON serialization
        let json = serde_json::to_string(&params).expect("serialize params");
        assert_eq!(json, "{}");
    }
}
