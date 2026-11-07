//! Stripe API v2 Core Accounts and Access Tokens Domain Models.
//!
//! Models V2 Accounts, lifecycle statuses, capability configurations,
//! short-lived access tokens, and request parameter DTOs.
//!
//! # Gate G2 & PCI-DSS Compliance
//! [`V2AccountToken`] stores sensitive API session credentials. It implements
//! [`Zeroize`](zeroize::Zeroize) and [`ZeroizeOnDrop`](zeroize::ZeroizeOnDrop)
//! to securely clear token memory on drop, and redacts raw secrets in its
//! [`std::fmt::Debug`] implementation as `[REDACTED]`.
//!
//! # Examples
//!
//! ```
//! use some_stripe_types::v2_account::{
//!     CreateV2AccountParams, V2Account, V2AccountStatus, V2AccountToken,
//! };
//!
//! let params = CreateV2AccountParams::new()
//!     .with_display_name("Merchant North")
//!     .with_contact_email("billing@north.example.com");
//!
//! assert_eq!(params.display_name.as_deref(), Some("Merchant North"));
//! assert_eq!(params.contact_email.as_deref(), Some("billing@north.example.com"));
//!
//! let token = V2AccountToken {
//!     token: "v2_tok_secret_xyz123".into(),
//!     expires_at: 1735689600,
//!     account_id: "acct_v2_123".into(),
//! };
//!
//! // Token debug output is redacted for security (Gate G2)
//! assert_eq!(format!("{token:?}"), "V2AccountToken { token: \"[REDACTED]\", expires_at: 1735689600, account_id: \"acct_v2_123\" }");
//! assert_eq!(token.token(), "v2_tok_secret_xyz123");
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeroize::{Zeroize, ZeroizeOnDrop};

fn default_account_object() -> String {
    "v2.core.account".into()
}

fn default_list_object() -> String {
    "list".into()
}

/// Lifecycle status of a Stripe API v2 Core Account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum V2AccountStatus {
    /// Account is fully operational and capable of active workflows.
    Active,
    /// Account has operational or compliance restrictions pending review.
    Restricted,
    /// Account onboarding or verification is currently in progress.
    Pending,
    /// Account has been administratively suspended.
    Suspended,
    /// Account has been closed and cannot process transactions.
    Closed,
}

impl V2AccountStatus {
    /// Returns the static string representation of this status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Restricted => "restricted",
            Self::Pending => "pending",
            Self::Suspended => "suspended",
            Self::Closed => "closed",
        }
    }
}

impl std::fmt::Display for V2AccountStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Optional feature toggles for a Stripe API v2 Core Account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct V2AccountFeatures {
    /// Whether payment collection is active for this account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payments_enabled: Option<bool>,
    /// Whether outbound payouts are enabled for this account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payouts_enabled: Option<bool>,
    /// Whether V2 event destination forwarding is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_destinations_enabled: Option<bool>,
}

impl V2AccountFeatures {
    /// Creates a new empty [`V2AccountFeatures`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            payments_enabled: None,
            payouts_enabled: None,
            event_destinations_enabled: None,
        }
    }

    /// Sets whether payments are enabled.
    #[must_use]
    pub const fn with_payments_enabled(mut self, enabled: bool) -> Self {
        self.payments_enabled = Some(enabled);
        self
    }

    /// Sets whether payouts are enabled.
    #[must_use]
    pub const fn with_payouts_enabled(mut self, enabled: bool) -> Self {
        self.payouts_enabled = Some(enabled);
        self
    }

    /// Sets whether event destinations are enabled.
    #[must_use]
    pub const fn with_event_destinations_enabled(mut self, enabled: bool) -> Self {
        self.event_destinations_enabled = Some(enabled);
        self
    }
}

/// Configuration settings and feature capabilities for a Stripe API v2 Core Account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct V2AccountConfiguration {
    /// Default three-letter ISO 4217 currency code (e.g. `"usd"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_currency: Option<String>,
    /// Feature capabilities configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<V2AccountFeatures>,
    /// Key-value metadata attached to this configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl V2AccountConfiguration {
    /// Creates a new default [`V2AccountConfiguration`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the default currency for the account.
    #[must_use]
    pub fn with_default_currency(mut self, currency: impl Into<String>) -> Self {
        self.default_currency = Some(currency.into());
        self
    }

    /// Sets feature capabilities for the account.
    #[must_use]
    pub fn with_features(mut self, features: V2AccountFeatures) -> Self {
        self.features = Some(features);
        self
    }

    /// Sets metadata key-value pairs.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Returns `true` if payments are explicitly enabled.
    #[must_use]
    pub fn is_payments_enabled(&self) -> bool {
        self.features
            .as_ref()
            .and_then(|f| f.payments_enabled)
            .unwrap_or(false)
    }

    /// Returns `true` if payouts are explicitly enabled.
    #[must_use]
    pub fn is_payouts_enabled(&self) -> bool {
        self.features
            .as_ref()
            .and_then(|f| f.payouts_enabled)
            .unwrap_or(false)
    }
}

/// A Stripe API v2 Core Account domain entity (`"v2.core.account"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2Account {
    /// Unique identifier for the v2 account (`acct_v2_...` or `acct_...`).
    pub id: String,
    /// Object descriptor, always `"v2.core.account"`.
    #[serde(default = "default_account_object")]
    pub object: String,
    /// Current operational lifecycle status.
    pub status: V2AccountStatus,
    /// Features and capabilities configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration: Option<V2AccountConfiguration>,
    /// Public business display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Primary contact or support email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_email: Option<String>,
    /// Unix timestamp (seconds) when the account was created.
    pub created: i64,
    /// Unix timestamp (seconds) when the account was last updated.
    pub updated: i64,
}

impl V2Account {
    /// Returns the unique account identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns `true` if the account status is [`V2AccountStatus::Active`].
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.status == V2AccountStatus::Active
    }

    /// Returns `true` if the account status is [`V2AccountStatus::Restricted`].
    #[must_use]
    pub fn is_restricted(&self) -> bool {
        self.status == V2AccountStatus::Restricted
    }

    /// Returns `true` if the account status is [`V2AccountStatus::Pending`].
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.status == V2AccountStatus::Pending
    }

    /// Returns `true` if the account status is [`V2AccountStatus::Suspended`].
    #[must_use]
    pub fn is_suspended(&self) -> bool {
        self.status == V2AccountStatus::Suspended
    }

    /// Returns `true` if the account status is [`V2AccountStatus::Closed`].
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.status == V2AccountStatus::Closed
    }

    /// Returns the public display name if set.
    #[must_use]
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    /// Returns the primary contact email if set.
    #[must_use]
    pub fn contact_email(&self) -> Option<&str> {
        self.contact_email.as_deref()
    }
}

/// A short-lived access token for a Stripe API v2 Core Account.
///
/// Implements [`Zeroize`] and [`ZeroizeOnDrop`] for secure memory cleanup (Gate G2),
/// and redacts the raw secret token string in [`std::fmt::Debug`].
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2AccountToken {
    /// The sensitive access token string.
    pub token: String,
    /// Unix timestamp (seconds) when this token expires.
    pub expires_at: i64,
    /// Account identifier to which this access token is bound.
    pub account_id: String,
}

impl V2AccountToken {
    /// Returns the sensitive token value for authenticated API transmissions.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Returns the associated account identifier.
    #[must_use]
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    /// Returns the Unix expiration timestamp in seconds.
    #[must_use]
    pub const fn expires_at(&self) -> i64 {
        self.expires_at
    }

    /// Checks if the token has expired against the given Unix timestamp in seconds.
    #[must_use]
    pub const fn is_expired(&self, current_time: i64) -> bool {
        current_time >= self.expires_at
    }
}

impl Drop for V2AccountToken {
    fn drop(&mut self) {
        self.token.zeroize();
    }
}

impl ZeroizeOnDrop for V2AccountToken {}

impl std::fmt::Debug for V2AccountToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("V2AccountToken")
            .field("token", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .field("account_id", &self.account_id)
            .finish()
    }
}

/// Parameters for creating a new Stripe API v2 Core Account (`POST /v2/core/accounts`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CreateV2AccountParams {
    /// Public business display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Primary contact or support email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_email: Option<String>,
    /// Optional configuration settings and feature toggles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration: Option<V2AccountConfiguration>,
    /// Optional idempotency key for safe retries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

impl CreateV2AccountParams {
    /// Creates a new default [`CreateV2AccountParams`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the business display name.
    #[must_use]
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    /// Sets the primary contact email address.
    #[must_use]
    pub fn with_contact_email(mut self, email: impl Into<String>) -> Self {
        self.contact_email = Some(email.into());
        self
    }

    /// Sets configuration settings.
    #[must_use]
    pub fn with_configuration(mut self, config: V2AccountConfiguration) -> Self {
        self.configuration = Some(config);
        self
    }

    /// Sets an explicit idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates parameter invariants prior to network dispatch.
    ///
    /// # Errors
    /// Returns an error message if `contact_email` is non-empty but does not contain an `'@'`.
    pub fn validate(&self) -> Result<(), &'static str> {
        if let Some(email) = &self.contact_email {
            let trimmed = email.trim();
            if trimmed.is_empty() || !trimmed.contains('@') {
                return Err("contact_email must be a valid non-empty email address");
            }
        }
        if let Some(name) = &self.display_name {
            if name.trim().is_empty() {
                return Err("display_name cannot be empty or whitespace");
            }
        }
        Ok(())
    }
}

/// Parameters for updating an existing Stripe API v2 Core Account (`POST /v2/core/accounts/{id}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UpdateV2AccountParams {
    /// Updated business display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Updated contact or support email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_email: Option<String>,
    /// Updated operational status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<V2AccountStatus>,
    /// Updated configuration settings and feature toggles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration: Option<V2AccountConfiguration>,
    /// Optional idempotency key for safe retries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

impl UpdateV2AccountParams {
    /// Creates a new default [`UpdateV2AccountParams`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the updated business display name.
    #[must_use]
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    /// Sets the updated contact email.
    #[must_use]
    pub fn with_contact_email(mut self, email: impl Into<String>) -> Self {
        self.contact_email = Some(email.into());
        self
    }

    /// Sets the operational lifecycle status.
    #[must_use]
    pub const fn with_status(mut self, status: V2AccountStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Sets updated configuration settings.
    #[must_use]
    pub fn with_configuration(mut self, config: V2AccountConfiguration) -> Self {
        self.configuration = Some(config);
        self
    }

    /// Sets an explicit idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates update parameter invariants prior to network dispatch.
    ///
    /// # Errors
    /// Returns an error message if `contact_email` is non-empty but does not contain an `'@'`.
    pub fn validate(&self) -> Result<(), &'static str> {
        if let Some(email) = &self.contact_email {
            let trimmed = email.trim();
            if trimmed.is_empty() || !trimmed.contains('@') {
                return Err("contact_email must be a valid non-empty email address");
            }
        }
        if let Some(name) = &self.display_name {
            if name.trim().is_empty() {
                return Err("display_name cannot be empty or whitespace");
            }
        }
        Ok(())
    }
}

/// Query parameters for listing Stripe API v2 Core Accounts (`GET /v2/core/accounts`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ListV2AccountsParams {
    /// Maximum number of records to return (1..100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Cursor for forward pagination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,
    /// Cursor for backward pagination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
    /// Filter accounts by operational lifecycle status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<V2AccountStatus>,
}

impl ListV2AccountsParams {
    /// Creates a new default [`ListV2AccountsParams`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the page size limit.
    #[must_use]
    pub const fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets cursor for forward pagination.
    #[must_use]
    pub fn with_starting_after(mut self, cursor: impl Into<String>) -> Self {
        self.starting_after = Some(cursor.into());
        self
    }

    /// Sets cursor for backward pagination.
    #[must_use]
    pub fn with_ending_before(mut self, cursor: impl Into<String>) -> Self {
        self.ending_before = Some(cursor.into());
        self
    }

    /// Filters accounts by status.
    #[must_use]
    pub const fn with_status(mut self, status: V2AccountStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Converts non-empty query parameters into key-value query pairs.
    #[must_use]
    pub fn to_query_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = Vec::with_capacity(4);
        if let Some(limit) = self.limit {
            pairs.push(("limit", limit.to_string()));
        }
        if let Some(sa) = &self.starting_after {
            pairs.push(("starting_after", sa.clone()));
        }
        if let Some(eb) = &self.ending_before {
            pairs.push(("ending_before", eb.clone()));
        }
        if let Some(st) = self.status {
            pairs.push(("status", st.as_str().to_string()));
        }
        pairs
    }
}

/// Request parameters for generating a short-lived access token for a V2 Account
/// (`POST /v2/core/accounts/{id}/tokens`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CreateV2AccountTokenParams {
    /// Desired token time-to-live in seconds (e.g. 3600 for 1 hour).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
    /// Optional fine-grained permission scopes granted to this token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,
    /// Optional idempotency key for safe retries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

impl CreateV2AccountTokenParams {
    /// Creates a new default [`CreateV2AccountTokenParams`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the requested TTL in seconds.
    #[must_use]
    pub const fn with_ttl_seconds(mut self, ttl: u64) -> Self {
        self.ttl_seconds = Some(ttl);
        self
    }

    /// Sets fine-grained permission scopes.
    #[must_use]
    pub fn with_permissions(mut self, perms: Vec<String>) -> Self {
        self.permissions = Some(perms);
        self
    }

    /// Sets an explicit idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates token request invariants prior to network dispatch.
    ///
    /// # Errors
    /// Returns an error message if `ttl_seconds` is `0` or exceeds 86,400 seconds (24 hours).
    pub const fn validate(&self) -> Result<(), &'static str> {
        if let Some(ttl) = self.ttl_seconds {
            if ttl == 0 {
                return Err("ttl_seconds must be greater than zero");
            }
            if ttl > 86_400 {
                return Err("ttl_seconds cannot exceed 86400 (24 hours)");
            }
        }
        Ok(())
    }
}

/// A paginated envelope containing a list of [`V2Account`] records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2AccountList {
    /// Object descriptor, typically `"list"`.
    #[serde(default = "default_list_object")]
    pub object: String,
    /// List of account records returned on the current page.
    pub data: Vec<V2Account>,
    /// Absolute or relative URL to the next page of records, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page_url: Option<String>,
    /// Absolute or relative URL to the previous page of records, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_page_url: Option<String>,
    /// Total count of matching accounts, if supplied by the API.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_count: Option<u64>,
}

impl V2AccountList {
    /// Returns `true` if more records are available via forward pagination.
    #[must_use]
    pub const fn has_more(&self) -> bool {
        self.next_page_url.is_some()
    }

    /// Returns the number of accounts on the current page.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if the current page contains no accounts.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v2_account_status_serialization_and_display() {
        let statuses = [
            (V2AccountStatus::Active, "\"active\"", "active"),
            (V2AccountStatus::Restricted, "\"restricted\"", "restricted"),
            (V2AccountStatus::Pending, "\"pending\"", "pending"),
            (V2AccountStatus::Suspended, "\"suspended\"", "suspended"),
            (V2AccountStatus::Closed, "\"closed\"", "closed"),
        ];

        for (status, expected_json, expected_str) in statuses {
            assert_eq!(status.as_str(), expected_str);
            assert_eq!(status.to_string(), expected_str);
            let serialized = serde_json::to_string(&status).unwrap();
            assert_eq!(serialized, expected_json);
            let deserialized: V2AccountStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    #[test]
    fn test_v2_account_deserialization_and_helpers() {
        let json = serde_json::json!({
            "id": "acct_v2_test_123",
            "object": "v2.core.account",
            "status": "active",
            "display_name": "Test Merchant",
            "contact_email": "ops@merchant.test",
            "created": 1700000000,
            "updated": 1700001000,
            "configuration": {
                "default_currency": "usd",
                "features": {
                    "payments_enabled": true,
                    "payouts_enabled": false,
                    "event_destinations_enabled": true
                }
            }
        });

        let account: V2Account = serde_json::from_value(json).unwrap();
        assert_eq!(account.id(), "acct_v2_test_123");
        assert_eq!(account.object, "v2.core.account");
        assert!(account.is_active());
        assert!(!account.is_restricted());
        assert!(!account.is_pending());
        assert!(!account.is_suspended());
        assert!(!account.is_closed());
        assert_eq!(account.display_name(), Some("Test Merchant"));
        assert_eq!(account.contact_email(), Some("ops@merchant.test"));

        let config = account.configuration.as_ref().unwrap();
        assert_eq!(config.default_currency.as_deref(), Some("usd"));
        assert!(config.is_payments_enabled());
        assert!(!config.is_payouts_enabled());
    }

    #[test]
    fn test_v2_account_token_debug_redaction_and_zeroize() {
        let mut token = V2AccountToken {
            token: "v2_tok_secret_super_confidential".into(),
            expires_at: 1735689600,
            account_id: "acct_v2_test_123".into(),
        };

        // Assert Debug redaction (Gate G2)
        let debug_str = format!("{token:?}");
        assert!(!debug_str.contains("v2_tok_secret_super_confidential"));
        assert!(debug_str.contains("[REDACTED]"));
        assert!(debug_str.contains("acct_v2_test_123"));

        // Assert getters
        assert_eq!(token.token(), "v2_tok_secret_super_confidential");
        assert_eq!(token.account_id(), "acct_v2_test_123");
        assert_eq!(token.expires_at(), 1735689600);
        assert!(!token.is_expired(1700000000));
        assert!(token.is_expired(1735689601));

        // Test manual zeroize
        token.token.zeroize();
        assert!(token.token().is_empty());
    }

    #[test]
    fn test_create_and_update_params_validation() {
        let valid_create = CreateV2AccountParams::new()
            .with_display_name("Valid Name")
            .with_contact_email("valid@email.com");
        assert!(valid_create.validate().is_ok());

        let invalid_email = CreateV2AccountParams::new().with_contact_email("not-an-email");
        assert!(invalid_email.validate().is_err());

        let empty_name = CreateV2AccountParams::new().with_display_name("   ");
        assert!(empty_name.validate().is_err());

        let valid_update = UpdateV2AccountParams::new().with_status(V2AccountStatus::Restricted);
        assert!(valid_update.validate().is_ok());
    }

    #[test]
    fn test_create_token_params_validation() {
        let valid = CreateV2AccountTokenParams::new().with_ttl_seconds(3600);
        assert!(valid.validate().is_ok());

        let zero_ttl = CreateV2AccountTokenParams::new().with_ttl_seconds(0);
        assert!(zero_ttl.validate().is_err());

        let too_large = CreateV2AccountTokenParams::new().with_ttl_seconds(100_000);
        assert!(too_large.validate().is_err());
    }

    #[test]
    fn test_list_params_query_pairs() {
        let params = ListV2AccountsParams::new()
            .with_limit(25)
            .with_status(V2AccountStatus::Active)
            .with_starting_after("acct_v2_cursor");

        let pairs = params.to_query_pairs();
        assert_eq!(pairs.len(), 3);
        assert!(pairs.contains(&("limit", "25".into())));
        assert!(pairs.contains(&("status", "active".into())));
        assert!(pairs.contains(&("starting_after", "acct_v2_cursor".into())));
    }
}
