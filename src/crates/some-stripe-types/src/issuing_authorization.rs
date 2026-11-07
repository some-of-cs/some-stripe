//! Stripe Issuing Authorizations domain models, parameters, and BSA/AML compliance verifications.
//!
//! Provides domain types for real-time card transaction approvals, declines, updates,
//! and listings under the Stripe Issuing API (`/v1/issuing/authorizations`).
//!
//! # Bank Secrecy Act (BSA) & Anti-Money Laundering (AML) Compliance
//!
//! In accordance with 31 CFR § 1010.311 (Currency Transaction Reports) and 31 CFR § 1020.320
//! (Suspicious Activity Reports), card authorization volumes exceeding statutory thresholds
//! are flagged for regulatory compliance reporting.
//!
//! # Zero PAN Leakage Invariant (PCI-DSS 4.0)
//!
//! In accordance with PCI-DSS 4.0 Requirement 3, Primary Account Numbers (PAN) and card verification
//! values (CVC/CVV) MUST NEVER be present in plain authorization metadata or merchant strings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Bank Secrecy Act Currency Transaction Report (CTR) filing threshold in cents ($10,000.00 USD).
pub const BSA_CTR_THRESHOLD_CENTS: i64 = 1_000_000;

/// Suspicious Activity Report (SAR) regulatory review threshold in cents ($5,000.00 USD).
pub const BSA_SAR_THRESHOLD_CENTS: i64 = 500_000;

fn default_authorization_object() -> String {
    "issuing.authorization".to_string()
}

fn default_list_object() -> String {
    "list".to_string()
}

/// Status of an [`IssuingAuthorization`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssuingAuthorizationStatus {
    /// The authorization was approved and has since closed.
    Closed,
    /// The authorization is pending evaluation or capture.
    Pending,
    /// The authorization was reversed or expired.
    Reversed,
}

impl std::fmt::Display for IssuingAuthorizationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => write!(f, "closed"),
            Self::Pending => write!(f, "pending"),
            Self::Reversed => write!(f, "reversed"),
        }
    }
}

/// Method by which the card was used in an authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssuingAuthorizationMethod {
    /// EMV contact chip presentment.
    Chip,
    /// Contactless NFC card or device presentment.
    Contactless,
    /// Manually keyed-in card number.
    KeyedIn,
    /// Online e-commerce or card-not-present transaction.
    Online,
    /// Magnetic stripe swipe.
    Swipe,
}

/// Merchant data associated with an [`IssuingAuthorization`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationMerchantData {
    /// Generalized category of the merchant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    /// Merchant Category Code (MCC).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_code: Option<String>,

    /// City where the merchant is located.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,

    /// Country where the merchant is located.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Name of the merchant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Unique identifier assigned to the merchant by the card network.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_id: Option<String>,

    /// Postal code of the merchant location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,

    /// State or province of the merchant location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,

    /// Terminal ID used at the merchant location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_id: Option<String>,
}

/// Verification check results performed during an authorization attempt.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationVerificationData {
    /// Address line 1 check result (`match`, `mismatch`, `not_provided`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_line1_check: Option<String>,

    /// Address postal code check result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_postal_code_check: Option<String>,

    /// Card CVC check result (`match`, `mismatch`, `not_provided`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cvc_check: Option<String>,

    /// Card expiration date check result (`match`, `mismatch`, `not_provided`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiry_check: Option<String>,

    /// Postal code check result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postal_code_check: Option<String>,

    /// PIN check result (`match`, `mismatch`, `not_provided`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin_check: Option<String>,
}

/// Breakdown of specific amount components in an authorization.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuingAuthorizationAmountDetails {
    /// ATM withdrawal fee included in the amount, in minor currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub atm_fee: Option<i64>,

    /// Cashback amount requested by cardholder, in minor currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cashback_amount: Option<i64>,
}

/// Entry in the authorization request history audit log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationRequestHistory {
    /// Amount authorized or requested in this iteration (in cents / minor currency units).
    pub amount: i64,

    /// Detailed breakdown of amount components.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_details: Option<IssuingAuthorizationAmountDetails>,

    /// Whether this request was approved.
    pub approved: bool,

    /// Time at which this request was recorded (seconds since Unix epoch).
    pub created: i64,

    /// Three-letter ISO currency code of the cardholder.
    pub currency: String,

    /// Amount requested by merchant in merchant currency.
    pub merchant_amount: i64,

    /// Currency presented by merchant.
    pub merchant_currency: String,

    /// Reason explaining why the request was approved or declined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Spending control or velocity violations that caused a decline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub violations: Option<Vec<String>>,
}

/// Pending authorization request details pending real-time webhook decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuingAuthorizationPendingRequest {
    /// Additional amount Stripe will hold if approved (in minor currency units).
    pub amount: i64,

    /// Granular breakdown of amount components.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_details: Option<IssuingAuthorizationAmountDetails>,

    /// Three-letter ISO currency code.
    pub currency: String,

    /// Whether the user can dynamically override/control the approved amount.
    pub is_amount_controllable: bool,

    /// Amount requested in local merchant currency.
    pub merchant_amount: i64,

    /// Local merchant currency code.
    pub merchant_currency: String,
}

/// Card summary associated with an [`IssuingAuthorization`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuingAuthorizationCard {
    /// Unique card identifier (`ic_...`).
    pub id: String,

    /// Cardholder identifier (`ich_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cardholder: Option<String>,

    /// Last 4 digits of the card number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last4: Option<String>,

    /// Card brand (e.g. `Visa`, `Mastercard`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,

    /// Currency of the card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    /// Two-digit expiration month.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exp_month: Option<i64>,

    /// Four-digit expiration year.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exp_year: Option<i64>,

    /// Card status (e.g. `active`, `inactive`, `canceled`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Card type (`virtual` or `physical`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

/// Reference to an issuing card, either an ID string or an expanded card object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IssuingCardReference {
    /// Unexpanded card ID (`ic_...`).
    Id(String),
    /// Expanded card details.
    Expanded(IssuingAuthorizationCard),
}

impl IssuingCardReference {
    /// Returns the card ID regardless of whether the card is expanded or unexpanded.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Id(id) => id,
            Self::Expanded(card) => &card.id,
        }
    }
}

/// A Stripe Issuing Authorization object (`issuing.authorization`).
///
/// When an issued card is used to make a purchase, an `IssuingAuthorization` is generated.
/// It contains full audit trails, merchant details, and verification checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuingAuthorization {
    /// Unique identifier for the authorization (`iauth_...`).
    pub id: String,

    /// String representing the object's type, always `issuing.authorization`.
    #[serde(default = "default_authorization_object")]
    pub object: String,

    /// Total amount authorized or rejected in minor currency units (e.g. cents).
    pub amount: i64,

    /// Detailed breakdown of amount components.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_details: Option<IssuingAuthorizationAmountDetails>,

    /// Whether the authorization has been approved.
    pub approved: bool,

    /// How the card was presented.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization_method: Option<IssuingAuthorizationMethod>,

    /// The card used for this authorization.
    pub card: IssuingCardReference,

    /// Cardholder identifier (`ich_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cardholder: Option<String>,

    /// Time at which the authorization was created (seconds since Unix epoch).
    pub created: i64,

    /// Three-letter ISO currency code (lowercase).
    pub currency: String,

    /// True if the authorization exists in live mode, false in test mode.
    pub livemode: bool,

    /// Total amount in merchant currency.
    pub merchant_amount: i64,

    /// Currency presented by the merchant.
    pub merchant_currency: String,

    /// Details about the seller/merchant.
    pub merchant_data: AuthorizationMerchantData,

    /// Set of key-value pairs for custom metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Pending authorization request details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_request: Option<IssuingAuthorizationPendingRequest>,

    /// Chronological log of request attempts and modifications.
    #[serde(default)]
    pub request_history: Vec<AuthorizationRequestHistory>,

    /// Current status of the authorization (`closed`, `pending`, `reversed`).
    pub status: IssuingAuthorizationStatus,

    /// List of transaction IDs associated with this authorization.
    #[serde(default)]
    pub transactions: Vec<String>,

    /// Verification check results (CVC, postal code, expiry, PIN).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_data: Option<AuthorizationVerificationData>,

    /// Digital wallet used (e.g. `apple_pay`, `google_pay`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wallet: Option<String>,
}

/// Parameters for approving an [`IssuingAuthorization`] (`POST /v1/issuing/authorizations/{id}/approve`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApproveAuthorizationParams {
    /// If `pending_request.is_amount_controllable` is true, a custom positive amount to hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i64>,

    /// Key-value metadata to attach to the authorization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Unique idempotency key for safe execution.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl ApproveAuthorizationParams {
    /// Creates a new empty `ApproveAuthorizationParams`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the approved amount in minor currency units.
    #[must_use]
    pub fn with_amount(mut self, amount: i64) -> Self {
        self.amount = Some(amount);
        self
    }

    /// Sets the metadata key-value map.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Adds a single metadata entry.
    #[must_use]
    pub fn with_metadata_entry(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let map = self.metadata.get_or_insert_with(HashMap::new);
        map.insert(key.into(), value.into());
        self
    }

    /// Sets the idempotency key for this request.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for declining an [`IssuingAuthorization`] (`POST /v1/issuing/authorizations/{id}/decline`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclineAuthorizationParams {
    /// Key-value metadata to attach to the authorization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Unique idempotency key for safe execution.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl DeclineAuthorizationParams {
    /// Creates a new empty `DeclineAuthorizationParams`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the metadata key-value map.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Adds a single metadata entry.
    #[must_use]
    pub fn with_metadata_entry(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let map = self.metadata.get_or_insert_with(HashMap::new);
        map.insert(key.into(), value.into());
        self
    }

    /// Sets the idempotency key for this request.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for updating an [`IssuingAuthorization`] (`POST /v1/issuing/authorizations/{id}`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateIssuingAuthorizationParams {
    /// Key-value metadata to attach to the authorization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Unique idempotency key for safe execution.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl UpdateIssuingAuthorizationParams {
    /// Creates a new empty `UpdateIssuingAuthorizationParams`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the metadata key-value map.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Adds a single metadata entry.
    #[must_use]
    pub fn with_metadata_entry(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let map = self.metadata.get_or_insert_with(HashMap::new);
        map.insert(key.into(), value.into());
        self
    }

    /// Sets the idempotency key for this request.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for listing [`IssuingAuthorization`] objects (`GET /v1/issuing/authorizations`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListIssuingAuthorizationsParams {
    /// Only return authorizations associated with this card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<String>,

    /// Only return authorizations associated with this cardholder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cardholder: Option<String>,

    /// Exact creation timestamp filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,

    /// Only return authorizations with the specified status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<IssuingAuthorizationStatus>,

    /// Limit on the number of objects to return (between 1 and 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,

    /// Cursor for forward pagination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// Cursor for backward pagination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListIssuingAuthorizationsParams {
    /// Creates a new empty `ListIssuingAuthorizationsParams`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Filters by card ID.
    #[must_use]
    pub fn with_card(mut self, card: impl Into<String>) -> Self {
        self.card = Some(card.into());
        self
    }

    /// Filters by cardholder ID.
    #[must_use]
    pub fn with_cardholder(mut self, cardholder: impl Into<String>) -> Self {
        self.cardholder = Some(cardholder.into());
        self
    }

    /// Filters by status.
    #[must_use]
    pub fn with_status(mut self, status: IssuingAuthorizationStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Sets pagination limit.
    #[must_use]
    pub fn with_limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets cursor for forward pagination.
    #[must_use]
    pub fn with_starting_after(mut self, id: impl Into<String>) -> Self {
        self.starting_after = Some(id.into());
        self
    }

    /// Sets cursor for backward pagination.
    #[must_use]
    pub fn with_ending_before(mut self, id: impl Into<String>) -> Self {
        self.ending_before = Some(id.into());
        self
    }
}

/// Paginated list of [`IssuingAuthorization`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuingAuthorizationList {
    /// String representing the object's type, always `list`.
    #[serde(default = "default_list_object")]
    pub object: String,

    /// Details about each object in the list.
    pub data: Vec<IssuingAuthorization>,

    /// True if this list has another page of items after this one.
    pub has_more: bool,

    /// The URL where this list can be accessed.
    pub url: String,
}

// ============================================================================
// Compliance & Statutory Verification Engines (Gate G-Law & Gate G2)
// ============================================================================

/// Error encountered during zero-PAN leakage security checks (PCI-DSS 4.0).
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PanLeakageError {
    /// A raw primary account number matching the Luhn checksum was detected.
    #[error("Detected unmasked 13-19 digit card PAN sequence violating PCI-DSS 4.0")]
    RawPanDetected,
}

/// Determines whether an authorization amount meets or exceeds the Bank Secrecy Act (BSA)
/// Currency Transaction Report (CTR) threshold of $10,000.00 USD (31 CFR § 1010.311).
///
/// # Example
///
/// ```
/// use some_stripe_types::is_bsa_ctr_reportable;
///
/// assert!(is_bsa_ctr_reportable(1_000_000)); // $10,000.00
/// assert!(!is_bsa_ctr_reportable(999_999));  // $9,999.99
/// ```
#[must_use]
pub fn is_bsa_ctr_reportable(amount: i64) -> bool {
    amount >= BSA_CTR_THRESHOLD_CENTS
}

/// Determines whether an authorization amount triggers Suspicious Activity Report (SAR)
/// regulatory review under 31 CFR § 1020.320 threshold of $5,000.00 USD.
///
/// # Example
///
/// ```
/// use some_stripe_types::is_bsa_sar_threshold_exceeded;
///
/// assert!(is_bsa_sar_threshold_exceeded(500_000)); // $5,000.00
/// assert!(!is_bsa_sar_threshold_exceeded(499_999)); // $4,999.99
/// ```
#[must_use]
pub fn is_bsa_sar_threshold_exceeded(amount: i64) -> bool {
    amount >= BSA_SAR_THRESHOLD_CENTS
}

/// Computes the standard Luhn checksum algorithm over a digit slice.
fn is_valid_luhn(digits: &[u8]) -> bool {
    if digits.len() < 13 || digits.len() > 19 {
        return false;
    }
    let mut sum = 0u32;
    let mut double = false;
    for &byte in digits.iter().rev() {
        let mut d = u32::from(byte - b'0');
        if double {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
        double = !double;
    }
    sum.is_multiple_of(10)
}

/// Scans text for potential unmasked 13–19 digit sequences matching the Luhn algorithm (PCI-DSS 4.0).
///
/// Ignores non-digit separators like hyphens or spaces when analyzing contiguous card number patterns.
#[must_use]
pub fn contains_raw_pan(input: &str) -> bool {
    let clean: Vec<u8> = input
        .bytes()
        .filter(|&b| b.is_ascii_digit() || b == b'-' || b == b' ')
        .collect();

    let digits: Vec<u8> = clean
        .iter()
        .copied()
        .filter(|b| b.is_ascii_digit())
        .collect();

    if digits.len() < 13 {
        return false;
    }

    // Check sliding windows of lengths 13 through 19
    for len in 13..=19.min(digits.len()) {
        for window in digits.windows(len) {
            if is_valid_luhn(window) {
                return true;
            }
        }
    }
    false
}

/// Enforces the zero PAN leakage invariant on arbitrary text fields (PCI-DSS 4.0 SAQ A).
///
/// # Errors
///
/// Returns [`PanLeakageError::RawPanDetected`] if an unmasked card PAN is detected.
pub fn validate_zero_pan_leakage(input: &str) -> Result<(), PanLeakageError> {
    if contains_raw_pan(input) {
        Err(PanLeakageError::RawPanDetected)
    } else {
        Ok(())
    }
}

/// Tracks card spending velocity and validates spend against configured velocity limits.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IssuingVelocityTracker {
    /// Accumulated spend in minor currency units for the active interval.
    pub accumulated_amount: i64,
    /// Maximum permitted spend in minor currency units.
    pub max_permitted_limit: i64,
}

impl IssuingVelocityTracker {
    /// Creates a new velocity tracker with a configured limit.
    #[must_use]
    pub fn new(max_permitted_limit: i64) -> Self {
        Self {
            accumulated_amount: 0,
            max_permitted_limit,
        }
    }

    /// Validates whether a prospective authorization amount would exceed the velocity ceiling.
    ///
    /// # Errors
    ///
    /// Returns an error message if the limit would be exceeded.
    pub fn evaluate_authorization(&self, requested_amount: i64) -> Result<i64, &'static str> {
        if requested_amount <= 0 {
            return Err("Requested authorization amount must be positive");
        }
        let prospective = match self.accumulated_amount.checked_add(requested_amount) {
            Some(sum) => sum,
            None => return Err("Integer overflow calculating spending velocity"),
        };
        if prospective > self.max_permitted_limit {
            return Err("Spending velocity cap exceeded for the active interval");
        }
        Ok(prospective)
    }

    /// Records an approved authorization amount into the tracker.
    pub fn record_approved(&mut self, amount: i64) -> Result<(), &'static str> {
        let new_total = self.evaluate_authorization(amount)?;
        self.accumulated_amount = new_total;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_deserialization_with_unexpanded_card() {
        let json = r#"{
            "id": "iauth_12345",
            "object": "issuing.authorization",
            "amount": 2500,
            "approved": true,
            "authorization_method": "chip",
            "card": "ic_card_999",
            "cardholder": "ich_holder_111",
            "created": 1700000000,
            "currency": "usd",
            "livemode": false,
            "merchant_amount": 2500,
            "merchant_currency": "usd",
            "merchant_data": {
                "name": "Acme Store",
                "category": "grocery_stores",
                "category_code": "5411",
                "city": "San Francisco",
                "country": "US",
                "postal_code": "94103"
            },
            "status": "closed",
            "transactions": ["ipi_txn_555"]
        }"#;

        let auth: IssuingAuthorization = serde_json::from_str(json).unwrap();
        assert_eq!(auth.id, "iauth_12345");
        assert_eq!(auth.amount, 2500);
        assert!(auth.approved);
        assert_eq!(auth.card.id(), "ic_card_999");
        assert_eq!(auth.merchant_data.name.as_deref(), Some("Acme Store"));
        assert_eq!(auth.status, IssuingAuthorizationStatus::Closed);
        assert_eq!(auth.transactions, vec!["ipi_txn_555"]);
    }

    #[test]
    fn test_authorization_deserialization_with_expanded_card() {
        let json = r#"{
            "id": "iauth_67890",
            "object": "issuing.authorization",
            "amount": 5000,
            "approved": false,
            "authorization_method": "contactless",
            "card": {
                "id": "ic_expanded_123",
                "last4": "4242",
                "brand": "Visa",
                "exp_month": 12,
                "exp_year": 2028,
                "status": "active",
                "type": "virtual"
            },
            "created": 1700000500,
            "currency": "usd",
            "livemode": false,
            "merchant_amount": 5000,
            "merchant_currency": "usd",
            "merchant_data": {
                "name": "Coffee Shop"
            },
            "status": "pending",
            "verification_data": {
                "cvc_check": "match",
                "postal_code_check": "match"
            }
        }"#;

        let auth: IssuingAuthorization = serde_json::from_str(json).unwrap();
        assert_eq!(auth.card.id(), "ic_expanded_123");
        match &auth.card {
            IssuingCardReference::Expanded(card) => {
                assert_eq!(card.last4.as_deref(), Some("4242"));
                assert_eq!(card.r#type.as_deref(), Some("virtual"));
            }
            _ => panic!("Expected expanded card"),
        }
        assert_eq!(
            auth.verification_data
                .as_ref()
                .unwrap()
                .cvc_check
                .as_deref(),
            Some("match")
        );
    }

    #[test]
    fn test_approve_authorization_params_builder() {
        let params = ApproveAuthorizationParams::new()
            .with_amount(4500)
            .with_metadata_entry("order_id", "ord_789")
            .with_idempotency_key("idem_app_1");

        assert_eq!(params.amount, Some(4500));
        assert_eq!(
            params.metadata.as_ref().unwrap().get("order_id").unwrap(),
            "ord_789"
        );
        assert_eq!(params.idempotency_key.as_deref(), Some("idem_app_1"));
    }

    #[test]
    fn test_decline_authorization_params_builder() {
        let params = DeclineAuthorizationParams::new()
            .with_metadata_entry("reason", "fraud_suspected")
            .with_idempotency_key("idem_dec_1");

        assert_eq!(
            params.metadata.as_ref().unwrap().get("reason").unwrap(),
            "fraud_suspected"
        );
        assert_eq!(params.idempotency_key.as_deref(), Some("idem_dec_1"));
    }

    #[test]
    fn test_list_issuing_authorizations_params() {
        let params = ListIssuingAuthorizationsParams::new()
            .with_card("ic_123")
            .with_cardholder("ich_456")
            .with_status(IssuingAuthorizationStatus::Pending)
            .with_limit(25)
            .with_starting_after("iauth_000");

        assert_eq!(params.card.as_deref(), Some("ic_123"));
        assert_eq!(params.cardholder.as_deref(), Some("ich_456"));
        assert_eq!(params.status, Some(IssuingAuthorizationStatus::Pending));
        assert_eq!(params.limit, Some(25));
        assert_eq!(params.starting_after.as_deref(), Some("iauth_000"));
    }

    #[test]
    fn test_bsa_aml_thresholds() {
        assert!(is_bsa_ctr_reportable(1_000_000));
        assert!(is_bsa_ctr_reportable(1_500_000));
        assert!(!is_bsa_ctr_reportable(999_999));

        assert!(is_bsa_sar_threshold_exceeded(500_000));
        assert!(is_bsa_sar_threshold_exceeded(750_000));
        assert!(!is_bsa_sar_threshold_exceeded(499_999));
    }

    #[test]
    fn test_zero_pan_leakage_detection() {
        // Safe strings (no card numbers)
        assert!(!contains_raw_pan("Purchase at Acme Corp #12345"));
        assert!(!contains_raw_pan("Card ending in 4242"));
        assert!(validate_zero_pan_leakage("Safe description").is_ok());

        // Valid test Visa card number: 4111 1111 1111 1111 (Luhn valid)
        let visa_pan = "4111111111111111";
        assert!(contains_raw_pan(visa_pan));
        assert!(validate_zero_pan_leakage(visa_pan).is_err());

        // Hyphenated PAN
        let hyphenated = "4111-1111-1111-1111";
        assert!(contains_raw_pan(hyphenated));
        assert!(validate_zero_pan_leakage(hyphenated).is_err());
    }

    #[test]
    fn test_velocity_tracker() {
        let mut tracker = IssuingVelocityTracker::new(100_000); // $1,000.00 limit

        assert!(tracker.evaluate_authorization(40_000).is_ok());
        tracker.record_approved(40_000).unwrap();
        assert_eq!(tracker.accumulated_amount, 40_000);

        assert!(tracker.evaluate_authorization(50_000).is_ok());
        tracker.record_approved(50_000).unwrap();
        assert_eq!(tracker.accumulated_amount, 90_000);

        // Exceed limit
        assert!(tracker.evaluate_authorization(15_000).is_err());
        assert!(tracker.record_approved(15_000).is_err());
        assert_eq!(tracker.accumulated_amount, 90_000);

        // Negative amount rejected
        assert!(tracker.evaluate_authorization(-500).is_err());
    }
}
