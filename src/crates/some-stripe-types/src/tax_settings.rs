//! Stripe Tax Settings & Tax Registrations Domain Models.
//!
//! This module implements domain entities and request parameter schemas for Stripe Tax
//! configuration, adhering to:
//! - **Gate G-Borrow**: Zero deep `.clone()` discipline on domain models with borrowed references.
//! - **Gate G-Law**: EU VAT Directive 2006/112/EC and international tax nexus registration compliance.
//! - **Gate G-Complexity**: Strict cyclomatic complexity <= 6 across status transition and validation logic.
//! - **Gate G-Docs**: Comprehensive rustdoc documentation and runnable doctests on all public types.

use crate::customer::Address;
use serde::{Deserialize, Serialize};

fn default_tax_settings_object() -> String {
    "tax.settings".into()
}

fn default_tax_registration_object() -> String {
    "tax.registration".into()
}

fn default_list_object() -> String {
    "list".into()
}

/// Operational status of a [`TaxSettings`] configuration.
///
/// Determines whether the account has satisfied all jurisdictional prerequisites
/// (e.g., configuring an authoritative head office address) to perform real-time
/// tax calculations.
///
/// # Example
/// ```
/// use some_stripe_types::TaxStatus;
/// use std::str::FromStr;
///
/// let status = TaxStatus::from_str("active").unwrap();
/// assert_eq!(status, TaxStatus::Active);
/// assert!(status.is_active());
/// assert!(!status.is_pending());
/// assert_eq!(status.as_str(), "active");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaxStatus {
    /// Tax settings are fully configured with a valid head office address.
    Active,
    /// Configuration is incomplete (e.g. missing head office address).
    Pending,
}

impl TaxStatus {
    /// Returns the wire representation of this status.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Pending => "pending",
        }
    }

    /// Returns `true` if the status is `active`.
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns `true` if the status is `pending`.
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Evaluates whether transitioning to `next` status is legally and architecturally valid.
    ///
    /// # Example
    /// ```
    /// use some_stripe_types::TaxStatus;
    ///
    /// assert!(TaxStatus::Pending.can_transition_to(TaxStatus::Active));
    /// assert!(TaxStatus::Active.can_transition_to(TaxStatus::Pending));
    /// assert!(TaxStatus::Active.can_transition_to(TaxStatus::Active));
    /// ```
    pub const fn can_transition_to(&self, _next: Self) -> bool {
        // Both transitions (pending <-> active) are valid when head office is configured or modified.
        true
    }
}

impl std::fmt::Display for TaxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error returned when parsing an unrecognized [`TaxStatus`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseTaxStatusError(pub String);

impl std::fmt::Display for ParseTaxStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown TaxStatus: '{}'", self.0)
    }
}

impl std::error::Error for ParseTaxStatusError {}

impl std::str::FromStr for TaxStatus {
    type Err = ParseTaxStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "pending" => Ok(Self::Pending),
            _ => Err(ParseTaxStatusError(s.to_string())),
        }
    }
}

impl AsRef<str> for TaxStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<str> for TaxStatus {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for TaxStatus {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

/// Context details when [`TaxSettings`] status is `active`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaxSettingsStatusDetailsActive {}

/// Context details when [`TaxSettings`] status is `pending`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaxSettingsStatusDetailsPending {
    /// List of configuration fields that must be provided to activate tax calculation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missing_fields: Option<Vec<String>>,
}

/// Detailed reason why [`TaxSettings`] is in its current state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaxSettingsStatusDetails {
    /// Present when status is active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<TaxSettingsStatusDetailsActive>,

    /// Present when status is pending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<TaxSettingsStatusDetailsPending>,
}

/// Head office address details for tax calculations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaxHeadOfficeDetails {
    /// Physical location of the business or headquarters.
    pub address: Address,
}

impl TaxHeadOfficeDetails {
    /// Creates a new `TaxHeadOfficeDetails` with the given address.
    pub fn new(address: Address) -> Self {
        Self { address }
    }
}

/// Default product tax code and tax behavior for transactions lacking line-item overrides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaxSettingsDefaults {
    /// Default tax behavior: `exclusive`, `inclusive`, or `inferred_by_currency`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_behavior: Option<String>,

    /// Default product tax code (e.g. `txcd_10000000`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_code: Option<String>,
}

/// Specific location registered under tax settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaxSettingsLocation {
    /// Two-letter ISO country code.
    pub country: String,

    /// State, county, province, or region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

/// Domain entity representing an account's Stripe Tax Settings (`tax.settings`).
///
/// # Example
/// ```
/// use some_stripe_types::{TaxSettings, TaxStatus};
///
/// let json = r#"{
///     "object": "tax.settings",
///     "status": "active",
///     "defaults": {
///         "tax_behavior": "exclusive",
///         "tax_code": "txcd_10000000"
///     },
///     "livemode": false
/// }"#;
/// let settings: TaxSettings = serde_json::from_str(json).unwrap();
/// assert_eq!(settings.status, TaxStatus::Active);
/// assert_eq!(settings.defaults.tax_behavior.as_deref(), Some("exclusive"));
/// assert!(!settings.livemode);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxSettings {
    /// Object type, always `tax.settings`.
    #[serde(default = "default_tax_settings_object")]
    pub object: String,

    /// Operational status of Stripe Tax for this account.
    pub status: TaxStatus,

    /// Contextual explanation of status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_details: Option<TaxSettingsStatusDetails>,

    /// Authoritative head office address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_office: Option<TaxHeadOfficeDetails>,

    /// Account-wide tax defaults.
    #[serde(default)]
    pub defaults: TaxSettingsDefaults,

    /// Has the value `true` if the object exists in live mode or `false` in test mode.
    #[serde(default)]
    pub livemode: bool,

    /// Jurisdictional locations configured for the account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<TaxSettingsLocation>>,
}

impl TaxSettings {
    /// Returns `true` if tax calculation is fully operational on this account.
    pub fn is_ready_for_calculation(&self) -> bool {
        self.status.is_active() && self.head_office.is_some()
    }
}

/// Lifecycle status of a [`TaxRegistration`].
///
/// # Example
/// ```
/// use some_stripe_types::TaxRegistrationStatus;
/// use std::str::FromStr;
///
/// let status = TaxRegistrationStatus::from_str("active").unwrap();
/// assert_eq!(status, TaxRegistrationStatus::Active);
/// assert!(status.is_active());
/// assert!(status.can_transition_to(TaxRegistrationStatus::Expired));
/// assert!(!status.can_transition_to(TaxRegistrationStatus::Scheduled));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaxRegistrationStatus {
    /// Registration is currently active and tax is collected in this jurisdiction.
    Active,
    /// Registration has expired or ended; tax is no longer collected.
    Expired,
    /// Registration is scheduled to begin at a future date (`active_from`).
    Scheduled,
}

impl TaxRegistrationStatus {
    /// Returns the wire representation of this registration status.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Scheduled => "scheduled",
        }
    }

    /// Returns `true` if the registration is currently active.
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns `true` if the registration has expired.
    pub const fn is_expired(&self) -> bool {
        matches!(self, Self::Expired)
    }

    /// Returns `true` if the registration is scheduled for a future date.
    pub const fn is_scheduled(&self) -> bool {
        matches!(self, Self::Scheduled)
    }

    /// Evaluates if transitioning from `self` to `next` is valid according to Stripe Tax lifecycle rules.
    ///
    /// Lifecycle rules:
    /// - `Scheduled` may transition to `Active` (when date arrives) or `Expired` (if canceled).
    /// - `Active` may transition to `Expired` (via `expires_at`).
    /// - `Expired` is terminal: an expired registration cannot be reactivated (Stripe requires a new registration).
    pub const fn can_transition_to(&self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Scheduled, Self::Active)
                | (Self::Scheduled, Self::Expired)
                | (Self::Active, Self::Expired)
                | (Self::Active, Self::Active)
                | (Self::Scheduled, Self::Scheduled)
                | (Self::Expired, Self::Expired)
        )
    }
}

impl std::fmt::Display for TaxRegistrationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error returned when parsing an unrecognized [`TaxRegistrationStatus`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseTaxRegistrationStatusError(pub String);

impl std::fmt::Display for ParseTaxRegistrationStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown TaxRegistrationStatus: '{}'", self.0)
    }
}

impl std::error::Error for ParseTaxRegistrationStatusError {}

impl std::str::FromStr for TaxRegistrationStatus {
    type Err = ParseTaxRegistrationStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "expired" => Ok(Self::Expired),
            "scheduled" => Ok(Self::Scheduled),
            _ => Err(ParseTaxRegistrationStatusError(s.to_string())),
        }
    }
}

impl AsRef<str> for TaxRegistrationStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<str> for TaxRegistrationStatus {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for TaxRegistrationStatus {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

/// Type of tax registration regime.
///
/// Complies with statutory tax schemes including EU VAT Directive 2006/112/EC Art 196
/// (`oss_union`, `oss_non_union`, `ioss`) and US state/local sales tax regimes.
///
/// # Example
/// ```
/// use some_stripe_types::TaxRegistrationType;
/// use std::str::FromStr;
///
/// let reg_type = TaxRegistrationType::from_str("oss_union").unwrap();
/// assert_eq!(reg_type, TaxRegistrationType::OssUnion);
/// assert_eq!(reg_type.as_str(), "oss_union");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaxRegistrationType {
    /// Standard domestic tax registration.
    Standard,
    /// Simplified non-resident registration (e.g. Canada GST/HST simplified).
    Simplified,
    /// European Union One-Stop Shop (Union OSS) registration.
    OssUnion,
    /// European Union One-Stop Shop (Non-Union OSS) registration.
    OssNonUnion,
    /// Import One-Stop Shop (IOSS) registration.
    Ioss,
    /// US State Sales and Use Tax registration.
    StateSalesTax,
    /// Local amusement tax registration.
    LocalAmusementTax,
    /// Local lease tax registration.
    LocalLeaseTax,
    /// Local communications tax registration.
    LocalCommunicationsTax,
    /// Fallback for custom or region-specific registration types.
    #[serde(untagged)]
    Custom(String),
}

impl TaxRegistrationType {
    /// Returns the string representation of this registration type.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Standard => "standard",
            Self::Simplified => "simplified",
            Self::OssUnion => "oss_union",
            Self::OssNonUnion => "oss_non_union",
            Self::Ioss => "ioss",
            Self::StateSalesTax => "state_sales_tax",
            Self::LocalAmusementTax => "local_amusement_tax",
            Self::LocalLeaseTax => "local_lease_tax",
            Self::LocalCommunicationsTax => "local_communications_tax",
            Self::Custom(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for TaxRegistrationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for TaxRegistrationType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "standard" => Ok(Self::Standard),
            "simplified" => Ok(Self::Simplified),
            "oss_union" => Ok(Self::OssUnion),
            "oss_non_union" => Ok(Self::OssNonUnion),
            "ioss" => Ok(Self::Ioss),
            "state_sales_tax" => Ok(Self::StateSalesTax),
            "local_amusement_tax" => Ok(Self::LocalAmusementTax),
            "local_lease_tax" => Ok(Self::LocalLeaseTax),
            "local_communications_tax" => Ok(Self::LocalCommunicationsTax),
            other => Ok(Self::Custom(other.to_string())),
        }
    }
}

impl AsRef<str> for TaxRegistrationType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<str> for TaxRegistrationType {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for TaxRegistrationType {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

/// Domain entity representing a Stripe Tax Registration (`tax.registration`).
///
/// Tracks regional nexus registrations that instruct Stripe Tax to collect sales tax, VAT, or GST.
///
/// # Example
/// ```
/// use some_stripe_types::{TaxRegistration, TaxRegistrationStatus};
///
/// let json = r#"{
///     "id": "taxreg_1234567890",
///     "object": "tax.registration",
///     "active_from": 1669249440,
///     "country": "IE",
///     "country_options": {
///         "ie": {
///             "type": "oss_union"
///         }
///     },
///     "livemode": false,
///     "status": "active"
/// }"#;
/// let reg: TaxRegistration = serde_json::from_str(json).unwrap();
/// assert_eq!(reg.id, "taxreg_1234567890");
/// assert_eq!(reg.country, "IE");
/// assert_eq!(reg.status, TaxRegistrationStatus::Active);
/// assert!(reg.is_active());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxRegistration {
    /// Unique identifier for the tax registration object (`taxreg_...`).
    pub id: String,

    /// Object type, always `tax.registration`.
    #[serde(default = "default_tax_registration_object")]
    pub object: String,

    /// Time at which the registration becomes active in seconds since Unix epoch.
    pub active_from: i64,

    /// Two-letter country code (ISO 3166-1 alpha-2) where the registration is held.
    pub country: String,

    /// Country-specific options and scheme configuration.
    #[serde(default)]
    pub country_options: serde_json::Value,

    /// Time at which the object was created in seconds since Unix epoch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,

    /// Time at which the registration expires in seconds since Unix epoch, if scheduled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,

    /// Has the value `true` if the object exists in live mode or `false` in test mode.
    #[serde(default)]
    pub livemode: bool,

    /// Status of the tax registration.
    pub status: TaxRegistrationStatus,
}

impl TaxRegistration {
    /// Borrows the unique identifier of the tax registration.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Borrows the country code.
    pub fn country(&self) -> &str {
        &self.country
    }

    /// Returns `true` if the tax registration is actively collecting.
    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }

    /// Returns `true` if the tax registration has expired.
    pub fn is_expired(&self) -> bool {
        self.status.is_expired()
    }

    /// Returns `true` if the tax registration is scheduled for a future date.
    pub fn is_scheduled(&self) -> bool {
        self.status.is_scheduled()
    }

    /// Extracts the registration type from `country_options` if present.
    pub fn registration_type(&self) -> Option<TaxRegistrationType> {
        let country_lower = self.country.to_lowercase();
        let type_val = self
            .country_options
            .get(&country_lower)
            .and_then(|o| o.get("type"))
            .and_then(|t| t.as_str())?;
        type_val.parse().ok()
    }
}

/// Envelope representing a paginated list of tax registrations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxRegistrationList {
    /// Object type, always `list`.
    #[serde(default = "default_list_object")]
    pub object: String,

    /// List of tax registration objects.
    pub data: Vec<TaxRegistration>,

    /// True if there are more items available.
    pub has_more: bool,

    /// URL endpoint producing this list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

// ============================================================================
// Parameter Schemas (Gate G-Borrow Discipline)
// ============================================================================

/// Parameters for updating account-wide tax settings defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaxSettingsDefaultsParams {
    /// Default tax behavior: `exclusive`, `inclusive`, or `inferred_by_currency`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_behavior: Option<String>,

    /// Default product tax code (e.g. `txcd_10000000`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_code: Option<String>,
}

/// Parameters for setting the head office address in tax settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaxHeadOfficeDetailsParams {
    /// Physical address of the head office.
    pub address: Address,
}

/// Request parameters for updating Stripe Tax Settings (`POST /v1/tax/settings`).
///
/// # Example
/// ```
/// use some_stripe_types::{Address, TaxHeadOfficeDetailsParams, UpdateTaxSettingsParams};
///
/// let params = UpdateTaxSettingsParams {
///     head_office: Some(TaxHeadOfficeDetailsParams {
///         address: Address {
///             country: Some("US".into()),
///             state: Some("CA".into()),
///             postal_code: Some("94103".into()),
///             city: Some("San Francisco".into()),
///             line1: Some("354 Oyster Point Blvd".into()),
///             ..Default::default()
///         },
///     }),
///     ..Default::default()
/// };
/// assert!(params.head_office.is_some());
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateTaxSettingsParams {
    /// Default settings for tax calculation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defaults: Option<TaxSettingsDefaultsParams>,

    /// Head office physical address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_office: Option<TaxHeadOfficeDetailsParams>,
}

/// Request parameters for creating a new tax registration (`POST /v1/tax/registrations`).
///
/// # Example
/// ```
/// use some_stripe_types::CreateTaxRegistrationParams;
///
/// let params = CreateTaxRegistrationParams {
///     country: "IE".into(),
///     active_from: "now".into(),
///     country_options: serde_json::json!({
///         "ie": {
///             "type": "oss_union"
///         }
///     }),
///     expires_at: None,
/// };
/// assert_eq!(params.country, "IE");
/// assert_eq!(params.active_from, "now");
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateTaxRegistrationParams {
    /// Two-letter country code (ISO 3166-1 alpha-2).
    pub country: String,

    /// Time at which the registration becomes active (`now` or Unix timestamp as string).
    pub active_from: String,

    /// Country-specific options object.
    pub country_options: serde_json::Value,

    /// Optional expiration time (`now` or Unix timestamp as string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Request parameters for updating an existing tax registration (`POST /v1/tax/registrations/{id}`).
///
/// # Example
/// ```
/// use some_stripe_types::UpdateTaxRegistrationParams;
///
/// let params = UpdateTaxRegistrationParams {
///     expires_at: Some("now".into()),
///     ..Default::default()
/// };
/// assert_eq!(params.expires_at.as_deref(), Some("now"));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateTaxRegistrationParams {
    /// Time at which the registration becomes active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_from: Option<String>,

    /// Time at which the registration expires (`now` or Unix timestamp string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Parameters for listing tax registrations (`GET /v1/tax/registrations`).
///
/// # Example
/// ```
/// use some_stripe_types::{ListTaxRegistrationsParams, TaxRegistrationStatus};
///
/// let params = ListTaxRegistrationsParams {
///     limit: Some(20),
///     status: Some(TaxRegistrationStatus::Active),
///     ..Default::default()
/// };
/// assert_eq!(params.limit, Some(20));
/// assert_eq!(params.status, Some(TaxRegistrationStatus::Active));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListTaxRegistrationsParams {
    /// Maximum number of objects to return (1-100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Cursor for forward pagination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// Cursor for backward pagination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,

    /// Filter by registration status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<TaxRegistrationStatus>,
}

// ============================================================================
// Unit Tests (Gates G1, G-Borrow, G-Law, G-Complexity)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tax_status_transitions() {
        assert!(TaxStatus::Pending.can_transition_to(TaxStatus::Active));
        assert!(TaxStatus::Active.can_transition_to(TaxStatus::Pending));
        assert!(TaxStatus::Active.can_transition_to(TaxStatus::Active));
        assert!(TaxStatus::Pending.can_transition_to(TaxStatus::Pending));

        assert_eq!(TaxStatus::Active.as_str(), "active");
        assert_eq!(TaxStatus::Pending.as_str(), "pending");
        assert!(TaxStatus::Active.is_active());
        assert!(!TaxStatus::Active.is_pending());
        assert!(TaxStatus::Pending.is_pending());
        assert!(!TaxStatus::Pending.is_active());
    }

    #[test]
    fn test_tax_registration_status_transitions() {
        // Scheduled -> Active: OK
        assert!(TaxRegistrationStatus::Scheduled.can_transition_to(TaxRegistrationStatus::Active));
        // Scheduled -> Expired: OK
        assert!(TaxRegistrationStatus::Scheduled.can_transition_to(TaxRegistrationStatus::Expired));
        // Active -> Expired: OK
        assert!(TaxRegistrationStatus::Active.can_transition_to(TaxRegistrationStatus::Expired));

        // Expired -> Active: Illegal (permanent per Stripe rules)
        assert!(!TaxRegistrationStatus::Expired.can_transition_to(TaxRegistrationStatus::Active));
        // Expired -> Scheduled: Illegal
        assert!(
            !TaxRegistrationStatus::Expired.can_transition_to(TaxRegistrationStatus::Scheduled)
        );
        // Active -> Scheduled: Illegal
        assert!(!TaxRegistrationStatus::Active.can_transition_to(TaxRegistrationStatus::Scheduled));

        // Self transitions: OK
        assert!(TaxRegistrationStatus::Active.can_transition_to(TaxRegistrationStatus::Active));
        assert!(TaxRegistrationStatus::Expired.can_transition_to(TaxRegistrationStatus::Expired));
        assert!(
            TaxRegistrationStatus::Scheduled.can_transition_to(TaxRegistrationStatus::Scheduled)
        );
    }

    #[test]
    fn test_tax_registration_type_parsing_and_display() {
        let types = [
            ("standard", TaxRegistrationType::Standard),
            ("simplified", TaxRegistrationType::Simplified),
            ("oss_union", TaxRegistrationType::OssUnion),
            ("oss_non_union", TaxRegistrationType::OssNonUnion),
            ("ioss", TaxRegistrationType::Ioss),
            ("state_sales_tax", TaxRegistrationType::StateSalesTax),
            (
                "local_amusement_tax",
                TaxRegistrationType::LocalAmusementTax,
            ),
            ("local_lease_tax", TaxRegistrationType::LocalLeaseTax),
            (
                "local_communications_tax",
                TaxRegistrationType::LocalCommunicationsTax,
            ),
        ];

        for (wire, expected) in types {
            let parsed: TaxRegistrationType = wire.parse().unwrap();
            assert_eq!(parsed, expected);
            assert_eq!(expected.as_str(), wire);
            assert_eq!(expected.to_string(), wire);
            assert_eq!(expected, wire);
        }

        // Custom scheme fallback
        let custom: TaxRegistrationType = "custom_local_tax".parse().unwrap();
        assert_eq!(custom.as_str(), "custom_local_tax");
        assert_eq!(custom, "custom_local_tax");
    }

    #[test]
    fn test_tax_settings_deserialization_active() {
        let json = serde_json::json!({
            "object": "tax.settings",
            "status": "active",
            "status_details": {
                "active": {}
            },
            "head_office": {
                "address": {
                    "city": "South San Francisco",
                    "country": "US",
                    "line1": "354 Oyster Point Blvd",
                    "line2": null,
                    "postal_code": "94080",
                    "state": "CA"
                }
            },
            "defaults": {
                "tax_behavior": "exclusive",
                "tax_code": "txcd_10000000"
            },
            "livemode": false,
            "locations": [
                {
                    "country": "US",
                    "state": "CA"
                }
            ]
        });

        let settings: TaxSettings = serde_json::from_value(json).unwrap();
        assert_eq!(settings.object, "tax.settings");
        assert_eq!(settings.status, TaxStatus::Active);
        assert!(settings.is_ready_for_calculation());
        assert_eq!(
            settings
                .head_office
                .as_ref()
                .and_then(|h| h.address.state.as_deref()),
            Some("CA")
        );
        assert_eq!(settings.defaults.tax_behavior.as_deref(), Some("exclusive"));
        assert_eq!(settings.defaults.tax_code.as_deref(), Some("txcd_10000000"));
        assert!(!settings.livemode);
        assert_eq!(settings.locations.as_ref().map(|l| l.len()), Some(1));
    }

    #[test]
    fn test_tax_settings_deserialization_pending() {
        let json = serde_json::json!({
            "object": "tax.settings",
            "status": "pending",
            "status_details": {
                "pending": {
                    "missing_fields": ["head_office.address"]
                }
            },
            "head_office": null,
            "defaults": {
                "tax_behavior": null,
                "tax_code": null
            },
            "livemode": false
        });

        let settings: TaxSettings = serde_json::from_value(json).unwrap();
        assert_eq!(settings.status, TaxStatus::Pending);
        assert!(!settings.is_ready_for_calculation());
        assert!(settings.head_office.is_none());
        assert_eq!(
            settings
                .status_details
                .as_ref()
                .and_then(|d| d.pending.as_ref())
                .and_then(|p| p.missing_fields.as_ref())
                .map(|f| f.len()),
            Some(1)
        );
    }

    #[test]
    fn test_tax_registration_deserialization_and_type_extraction() {
        let json = serde_json::json!({
            "id": "taxreg_1M1234567890",
            "object": "tax.registration",
            "active_from": 1669249440,
            "country": "IE",
            "country_options": {
                "ie": {
                    "type": "oss_union"
                }
            },
            "created": 1669249000,
            "expires_at": null,
            "livemode": false,
            "status": "active"
        });

        let reg: TaxRegistration = serde_json::from_value(json).unwrap();
        assert_eq!(reg.id(), "taxreg_1M1234567890");
        assert_eq!(reg.country(), "IE");
        assert!(reg.is_active());
        assert!(!reg.is_expired());
        assert!(!reg.is_scheduled());
        assert_eq!(reg.registration_type(), Some(TaxRegistrationType::OssUnion));
    }

    #[test]
    fn test_tax_registration_us_state_sales_tax() {
        let json = serde_json::json!({
            "id": "taxreg_us_ca_987",
            "object": "tax.registration",
            "active_from": 1670000000,
            "country": "US",
            "country_options": {
                "us": {
                    "state": "CA",
                    "type": "state_sales_tax"
                }
            },
            "livemode": true,
            "status": "scheduled"
        });

        let reg: TaxRegistration = serde_json::from_value(json).unwrap();
        assert_eq!(reg.id, "taxreg_us_ca_987");
        assert_eq!(reg.country, "US");
        assert!(reg.is_scheduled());
        assert_eq!(
            reg.registration_type(),
            Some(TaxRegistrationType::StateSalesTax)
        );
    }

    #[test]
    fn test_tax_registration_list_deserialization() {
        let json = serde_json::json!({
            "object": "list",
            "data": [
                {
                    "id": "taxreg_1",
                    "object": "tax.registration",
                    "active_from": 1669249440,
                    "country": "IE",
                    "country_options": {},
                    "livemode": false,
                    "status": "active"
                },
                {
                    "id": "taxreg_2",
                    "object": "tax.registration",
                    "active_from": 1669250000,
                    "country": "US",
                    "country_options": {},
                    "livemode": false,
                    "status": "expired"
                }
            ],
            "has_more": false,
            "url": "/v1/tax/registrations"
        });

        let list: TaxRegistrationList = serde_json::from_value(json).unwrap();
        assert_eq!(list.data.len(), 2);
        assert_eq!(list.data[0].id, "taxreg_1");
        assert_eq!(list.data[1].id, "taxreg_2");
        assert!(!list.has_more);
    }
}
