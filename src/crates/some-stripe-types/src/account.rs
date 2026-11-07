//! Stripe Connect Account domain entities, capabilities, requirements, and parameter builders.
//!
//! Models merchant and connected accounts (`acct_...`), verification requirements,
//! capabilities, and parameter builders for account provisioning and modification.
//!
//! # Architecture & Quality Gates
//!
//! - **Gate G-Borrow**: Zero-clone borrowing discipline on domain models and accessors.
//! - **Gate G-Law**: EU AMLD5, DAC7, and FinCEN Customer Due Diligence (CDD) KYC compliance.
//! - **Dual-Protocol**: API v1 form-encoded bracket notation and API v2 JSON serde support with [`Nullable`].

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::nullable::Nullable;

/// Legal business entity type of a Stripe Connect account.
///
/// Complies with FinCEN CDD and EU DAC7 legal entity classification standards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BusinessType {
    /// Individual or sole proprietorship.
    Individual,
    /// Incorporated company, corporation, or partnership.
    Company,
    /// Non-profit or tax-exempt organization.
    #[serde(rename = "non_profit")]
    NonProfit,
    /// Government entity or municipality.
    #[serde(rename = "government_entity")]
    GovernmentEntity,
}

impl BusinessType {
    /// Returns the static string representation of this business type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Individual => "individual",
            Self::Company => "company",
            Self::NonProfit => "non_profit",
            Self::GovernmentEntity => "government_entity",
        }
    }
}

impl std::fmt::Display for BusinessType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for BusinessType {
    fn from(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "company" => Self::Company,
            "non_profit" | "nonprofit" => Self::NonProfit,
            "government_entity" => Self::GovernmentEntity,
            _ => Self::Individual,
        }
    }
}

impl From<String> for BusinessType {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

/// Connected Account type specifying Stripe onboarding and liability model.
///
/// Stripe Connect accounts are partitioned into three primary architectures:
/// - [`AccountType::Standard`]: Full Stripe Dashboard access; merchant is liable for chargebacks.
/// - [`AccountType::Express`]: Co-branded Dashboard; platform controls payouts and fee structures.
/// - [`AccountType::Custom`]: Completely white-labeled; platform manages onboarding and is liable for chargebacks.
/// - [`AccountType::None`]: Standalone / unmanaged account profile.
///
/// # Examples
///
/// ```
/// use some_stripe_types::AccountType;
///
/// let standard = AccountType::Standard;
/// assert_eq!(standard.as_str(), "standard");
/// assert!(standard.is_standard());
///
/// let json = serde_json::to_string(&AccountType::Custom).unwrap();
/// assert_eq!(json, r#""custom""#);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    /// Standard connected account with full direct Stripe Dashboard access.
    Standard,
    /// Express connected account with platform-managed onboarding and lightweight Dashboard.
    Express,
    /// Custom connected account with 100% white-labeled experience and platform liability.
    Custom,
    /// Standalone or unclassified account type.
    None,
}

impl AccountType {
    /// Returns the static string slice of this account type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Express => "express",
            Self::Custom => "custom",
            Self::None => "none",
        }
    }

    /// Returns `true` if this is a standard connected account.
    pub fn is_standard(&self) -> bool {
        matches!(self, Self::Standard)
    }

    /// Returns `true` if this is an express connected account.
    pub fn is_express(&self) -> bool {
        matches!(self, Self::Express)
    }

    /// Returns `true` if this is a custom connected account.
    pub fn is_custom(&self) -> bool {
        matches!(self, Self::Custom)
    }
}

impl std::fmt::Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for AccountType {
    fn from(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "standard" => Self::Standard,
            "express" => Self::Express,
            "custom" => Self::Custom,
            _ => Self::None,
        }
    }
}

impl From<String> for AccountType {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl std::str::FromStr for AccountType {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}

/// Status of an account capability (e.g. `card_payments`, `transfers`).
///
/// # Examples
///
/// ```
/// use some_stripe_types::CapabilityStatus;
///
/// let active = CapabilityStatus::Active;
/// assert_eq!(active.as_str(), "active");
/// assert!(active.is_active());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    /// Capability is active and fully operational.
    Active,
    /// Capability is inactive or disabled.
    Inactive,
    /// Capability is pending verification and requirements fulfillment.
    Pending,
    /// Capability was not requested.
    Unrequested,
    /// Capability has been requested during account creation or update.
    Requested,
}

impl CapabilityStatus {
    /// Returns the static string representation of this capability status.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Pending => "pending",
            Self::Unrequested => "unrequested",
            Self::Requested => "requested",
        }
    }

    /// Returns `true` if this capability is active.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns `true` if this capability is pending verification.
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Returns `true` if this capability was requested.
    pub fn is_requested(&self) -> bool {
        matches!(self, Self::Requested)
    }
}

impl std::fmt::Display for CapabilityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for CapabilityStatus {
    fn from(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "active" => Self::Active,
            "inactive" => Self::Inactive,
            "pending" => Self::Pending,
            "requested" => Self::Requested,
            _ => Self::Unrequested,
        }
    }
}

impl From<String> for CapabilityStatus {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

/// Represents the status of capabilities enabled or requested on a Stripe Account.
///
/// # Examples
///
/// ```
/// use some_stripe_types::{AccountCapabilities, CapabilityStatus};
///
/// let caps = AccountCapabilities {
///     card_payments: Some(CapabilityStatus::Active),
///     transfers: Some(CapabilityStatus::Active),
///     ..Default::default()
/// };
/// assert!(caps.has_active_card_payments());
/// assert!(caps.has_active_transfers());
/// assert!(caps.is_fully_active());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountCapabilities {
    /// Card payments processing capability (`active`, `inactive`, `pending`, `unrequested`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_payments: Option<CapabilityStatus>,

    /// Account transfers and payout distribution capability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfers: Option<CapabilityStatus>,

    /// Tax reporting form 1099-K capability (US).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_reporting_us_1099_k: Option<CapabilityStatus>,

    /// Tax reporting form 1099-MISC capability (US).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_reporting_us_1099_misc: Option<CapabilityStatus>,

    /// ACSS direct debit capability (Canada).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acss_debit_payments: Option<CapabilityStatus>,

    /// Bacs direct debit capability (United Kingdom).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bacs_debit_payments: Option<CapabilityStatus>,

    /// Bancontact debit payments capability (Europe).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bancontact_payments: Option<CapabilityStatus>,

    /// SEPA direct debit capability (Single Euro Payments Area).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sepa_debit_payments: Option<CapabilityStatus>,

    /// US ACH direct debit payments capability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub us_bank_account_ach_payments: Option<CapabilityStatus>,

    /// Embedded Treasury financial accounts capability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub treasury: Option<CapabilityStatus>,

    /// Commercial Card Issuing capability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_issuing: Option<CapabilityStatus>,
}

impl AccountCapabilities {
    /// Returns `true` if the card payments capability is active.
    pub fn has_active_card_payments(&self) -> bool {
        self.card_payments == Some(CapabilityStatus::Active)
    }

    /// Returns `true` if the transfers capability is active.
    pub fn has_active_transfers(&self) -> bool {
        self.transfers == Some(CapabilityStatus::Active)
    }

    /// Returns `true` if both card payments and transfers are active.
    pub fn is_fully_active(&self) -> bool {
        self.has_active_card_payments() && self.has_active_transfers()
    }
}

/// Request parameter payload for requesting capabilities when creating or updating an account.
///
/// # Examples
///
/// ```
/// use some_stripe_types::{AccountCapabilitiesParams, CapabilityStatus};
///
/// let params = AccountCapabilitiesParams::new()
///     .card_payments(CapabilityStatus::Requested)
///     .transfers(CapabilityStatus::Requested);
/// assert_eq!(params.card_payments, Some(CapabilityStatus::Requested));
/// assert_eq!(params.transfers, Some(CapabilityStatus::Requested));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountCapabilitiesParams {
    /// Request card payments processing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_payments: Option<CapabilityStatus>,

    /// Request transfers capability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfers: Option<CapabilityStatus>,

    /// Request tax reporting form 1099-K.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_reporting_us_1099_k: Option<CapabilityStatus>,

    /// Request tax reporting form 1099-MISC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_reporting_us_1099_misc: Option<CapabilityStatus>,

    /// Request ACSS direct debit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acss_debit_payments: Option<CapabilityStatus>,

    /// Request Bacs direct debit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bacs_debit_payments: Option<CapabilityStatus>,

    /// Request Bancontact debit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bancontact_payments: Option<CapabilityStatus>,

    /// Request SEPA direct debit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sepa_debit_payments: Option<CapabilityStatus>,

    /// Request US ACH debit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub us_bank_account_ach_payments: Option<CapabilityStatus>,

    /// Request Treasury capability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub treasury: Option<CapabilityStatus>,

    /// Request Card Issuing capability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_issuing: Option<CapabilityStatus>,
}

impl AccountCapabilitiesParams {
    /// Creates a new empty capability parameters builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the requested card payments capability status.
    pub fn card_payments(mut self, status: CapabilityStatus) -> Self {
        self.card_payments = Some(status);
        self
    }

    /// Sets the requested transfers capability status.
    pub fn transfers(mut self, status: CapabilityStatus) -> Self {
        self.transfers = Some(status);
        self
    }
}

/// An error detailing why an account requirement verification failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRequirementError {
    /// Machine-readable error code (e.g. `invalid_address_city`, `verification_document_failed_copy`).
    pub code: String,

    /// Human-readable explanation of why the verification failed.
    pub reason: String,

    /// The specific requirement field that triggered this error (e.g. `individual.verification.document`).
    pub requirement: String,
}

/// An alternative set of requirements that may satisfy a verification check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRequirementAlternative {
    /// Alternative fields that can be submitted.
    #[serde(default)]
    pub alternative_fields_due: Vec<String>,

    /// Original requirement fields that these alternatives satisfy.
    #[serde(default)]
    pub original_fields_due: Vec<String>,
}

/// Identity and regulatory verification requirements for an account.
///
/// Encapsulates requirements mandated by FinCEN Customer Due Diligence (CDD),
/// EU DAC7, AMLD5, and local KYC regimes.
///
/// # Examples
///
/// ```
/// use some_stripe_types::AccountRequirements;
///
/// let reqs = AccountRequirements {
///     currently_due: vec!["external_account".into()],
///     eventually_due: vec!["tos_acceptance.date".into()],
///     ..Default::default()
/// };
/// assert!(!reqs.is_complete());
/// assert!(!reqs.is_disabled());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRequirements {
    /// Fields that are currently required and must be collected immediately to keep the account active.
    #[serde(default)]
    pub currently_due: Vec<String>,

    /// Fields that will eventually need to be collected, but are not immediately blocking payouts.
    #[serde(default)]
    pub eventually_due: Vec<String>,

    /// Fields that have passed their collection deadline. Payouts or charges may be disabled.
    #[serde(default)]
    pub past_due: Vec<String>,

    /// Fields that have been submitted and are pending review by Stripe.
    #[serde(default)]
    pub pending_verification: Vec<String>,

    /// Detailed verification errors explaining why submitted requirements were rejected.
    #[serde(default)]
    pub errors: Vec<AccountRequirementError>,

    /// Reason why the account was disabled (e.g. `requirements.past_due`, `rejected.fraud`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,

    /// UNIX timestamp (seconds) by which `currently_due` requirements must be satisfied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_deadline: Option<i64>,

    /// Alternative sets of verification requirements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alternatives: Option<Vec<AccountRequirementAlternative>>,
}

impl AccountRequirements {
    /// Returns `true` if all requirements are fulfilled (no items in `currently_due` or `past_due`).
    pub fn is_complete(&self) -> bool {
        self.currently_due.is_empty() && self.past_due.is_empty()
    }

    /// Returns `true` if any verification errors are present.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Returns `true` if the account is currently disabled.
    pub fn is_disabled(&self) -> bool {
        self.disabled_reason.is_some()
    }

    /// Returns `true` if any submitted requirements are pending verification.
    pub fn has_pending_verification(&self) -> bool {
        !self.pending_verification.is_empty()
    }
}

/// Business profile information for a Stripe Account.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountBusinessProfile {
    /// Merchant category code (MCC).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcc: Option<String>,

    /// Customer-facing business name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Description of products or services sold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_description: Option<String>,

    /// Customer support email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_email: Option<String>,

    /// Customer support phone number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_phone: Option<String>,

    /// Customer support URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_url: Option<String>,

    /// Business website URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Request parameters for configuring an account's business profile.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountBusinessProfileParams {
    /// Merchant category code (MCC).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcc: Option<String>,

    /// Customer-facing business name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Description of products or services sold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_description: Option<String>,

    /// Customer support email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_email: Option<String>,

    /// Customer support phone number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_phone: Option<String>,

    /// Customer support URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_url: Option<String>,

    /// Business website URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Company information for an incorporated entity Stripe Account.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountCompany {
    /// Legal company name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Company business phone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,

    /// Whether the company's tax ID has been provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_id_provided: Option<bool>,

    /// Whether the company's VAT ID has been provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vat_id_provided: Option<bool>,

    /// Corporate structure (e.g. `private_corporation`, `llc`, `sole_proprietorship`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structure: Option<String>,
}

/// Request parameters for company details on an incorporated account.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountCompanyParams {
    /// Legal company name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Company business phone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,

    /// Tax identification number (EIN, VAT, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<String>,

    /// VAT identification number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vat_id: Option<String>,

    /// Corporate structure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structure: Option<String>,
}

/// Individual personal details for a sole proprietorship Stripe Account.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountIndividual {
    /// First name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,

    /// Last name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,

    /// Email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Phone number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,

    /// Whether government ID number was provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_number_provided: Option<bool>,

    /// Whether the last 4 digits of SSN were provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssn_last_4_provided: Option<bool>,
}

/// Request parameters for personal details of an individual account holder.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountIndividualParams {
    /// First name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,

    /// Last name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,

    /// Email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Phone number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,

    /// Full government identification number (SSN, National ID).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_number: Option<String>,

    /// Last 4 digits of SSN.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssn_last_4: Option<String>,
}

/// Branding and visual customization settings for an account.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountBrandingSettings {
    /// File ID of the brand icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// File ID of the brand logo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,

    /// Primary brand HEX color code (e.g. `#00d924`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_color: Option<String>,

    /// Secondary brand HEX color code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_color: Option<String>,
}

/// Request parameters for branding settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountBrandingSettingsParams {
    /// File ID of the brand icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// File ID of the brand logo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,

    /// Primary brand HEX color code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_color: Option<String>,

    /// Secondary brand HEX color code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_color: Option<String>,
}

/// Card payment processing configuration settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountCardPaymentsSettings {
    /// Prefix appended to the statement descriptor for card charges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_descriptor_prefix: Option<String>,
}

/// Request parameters for card payment settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountCardPaymentsSettingsParams {
    /// Prefix appended to the statement descriptor for card charges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_descriptor_prefix: Option<String>,
}

/// Stripe Dashboard configuration for an account.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountDashboardSettings {
    /// Display name shown in the Stripe Dashboard.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Timezone for the account (e.g. `America/Los_Angeles`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

/// Request parameters for Dashboard settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountDashboardSettingsParams {
    /// Display name shown in the Stripe Dashboard.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Timezone for the account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

/// Payment settings including default statement descriptors.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountPaymentsSettings {
    /// Default statement descriptor appearing on customer bank statements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_descriptor: Option<String>,

    /// Japanese Kana statement descriptor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_descriptor_kana: Option<String>,

    /// Japanese Kanji statement descriptor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_descriptor_kanji: Option<String>,
}

/// Request parameters for payment settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountPaymentsSettingsParams {
    /// Default statement descriptor appearing on customer bank statements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_descriptor: Option<String>,

    /// Japanese Kana statement descriptor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_descriptor_kana: Option<String>,

    /// Japanese Kanji statement descriptor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_descriptor_kanji: Option<String>,
}

/// Payout schedule details for automatic disbursements.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountPayoutSchedule {
    /// Number of days funds are held before payout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay_days: Option<u32>,

    /// Disbursement cadence (`daily`, `weekly`, `monthly`, `manual`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,

    /// Day of the month for monthly payouts (1-31).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monthly_anchor: Option<u32>,

    /// Day of the week for weekly payouts (`monday`, `friday`, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weekly_anchor: Option<String>,
}

/// Request parameters for payout schedule configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountPayoutScheduleParams {
    /// Number of days funds are held before payout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay_days: Option<u32>,

    /// Disbursement cadence (`daily`, `weekly`, `monthly`, `manual`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,

    /// Day of the month for monthly payouts (1-31).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monthly_anchor: Option<u32>,

    /// Day of the week for weekly payouts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weekly_anchor: Option<String>,
}

/// Payout disbursement and debit settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountPayoutsSettings {
    /// Whether Stripe may debit negative account balances from the linked bank account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debit_negative_balances: Option<bool>,

    /// Automatic payout schedule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<AccountPayoutSchedule>,

    /// Statement descriptor appearing on bank deposits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_descriptor: Option<String>,
}

/// Request parameters for payout settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountPayoutsSettingsParams {
    /// Whether Stripe may debit negative account balances.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debit_negative_balances: Option<bool>,

    /// Automatic payout schedule configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<AccountPayoutScheduleParams>,

    /// Statement descriptor appearing on bank deposits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_descriptor: Option<String>,
}

/// Complete configuration settings for an account.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountSettings {
    /// Branding customizations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branding: Option<AccountBrandingSettings>,

    /// Card payments settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_payments: Option<AccountCardPaymentsSettings>,

    /// Dashboard settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dashboard: Option<AccountDashboardSettings>,

    /// Payments settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payments: Option<AccountPaymentsSettings>,

    /// Payout disbursement settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payouts: Option<AccountPayoutsSettings>,
}

/// Request parameters for configuring account settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountSettingsParams {
    /// Branding customizations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branding: Option<AccountBrandingSettingsParams>,

    /// Card payments settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_payments: Option<AccountCardPaymentsSettingsParams>,

    /// Dashboard settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dashboard: Option<AccountDashboardSettingsParams>,

    /// Payments settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payments: Option<AccountPaymentsSettingsParams>,

    /// Payout disbursement settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payouts: Option<AccountPayoutsSettingsParams>,
}

/// Terms of Service acceptance details.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountTosAcceptance {
    /// UNIX timestamp (seconds) when the merchant accepted the Stripe Services Agreement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<i64>,

    /// IP address from which the Terms of Service were accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,

    /// User agent string of the browser that accepted the terms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,

    /// Service agreement type (`full`, `recipient`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_agreement: Option<String>,
}

/// Request parameters for attesting Terms of Service acceptance.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountTosAcceptanceParams {
    /// UNIX timestamp (seconds) when the agreement was accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<i64>,

    /// IP address from which the agreement was accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,

    /// User agent of the browser used to accept the agreement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,

    /// Service agreement type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_agreement: Option<String>,
}

/// Accounts v2 controller properties governing fee, loss, and requirement liability.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountController {
    /// Whether this account is controlled by the platform.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_controller: Option<bool>,

    /// Controller type (`application`, `account`).
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub controller_type: Option<String>,

    /// Fee liability ownership.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fees: Option<AccountControllerFees>,

    /// Loss liability ownership for chargebacks and refunds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub losses: Option<AccountControllerLosses>,

    /// Responsible party for requirement collection (`stripe`, `application`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirement_collection: Option<String>,

    /// Stripe Dashboard configuration for the account controller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stripe_dashboard: Option<AccountControllerDashboard>,
}

/// Controller fee liability settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountControllerFees {
    /// Entity paying Stripe processing fees (`account`, `application`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payer: Option<String>,
}

/// Controller negative balance and chargeback loss liability settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountControllerLosses {
    /// Entity liable for payment losses (`stripe`, `application`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payments: Option<String>,
}

/// Controller Dashboard access type settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountControllerDashboard {
    /// Dashboard type (`full`, `express`, `none`).
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub dashboard_type: Option<String>,
}

/// Represents a Stripe Account object (merchant or connected account profile).
///
/// Encapsulates all identity, KYC requirements, capabilities, and payment settings
/// for platform merchants under Stripe Connect.
///
/// # Invariants
///
/// - **Gate G-Borrow**: Zero-clone borrowing discipline on field accessors.
/// - **Gate G-Law**: FinCEN CDD and EU DAC7 compliance verification fields.
///
/// # Examples
///
/// ```
/// use some_stripe_types::{Account, AccountType};
///
/// let json = r#"{
///     "id": "acct_1H2X3Y4Z",
///     "object": "account",
///     "type": "custom",
///     "country": "US",
///     "charges_enabled": true,
///     "payouts_enabled": true
/// }"#;
/// let account: Account = serde_json::from_str(json).unwrap();
/// assert_eq!(account.id(), "acct_1H2X3Y4Z");
/// assert_eq!(account.type_(), Some(AccountType::Custom));
/// assert!(account.is_charges_enabled());
/// assert!(account.is_payouts_enabled());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Account {
    /// Unique identifier for the account (`acct_...`).
    pub id: String,

    /// Object type, always `account`.
    #[serde(default = "default_account_object")]
    pub object: String,

    /// Connect account type (`standard`, `express`, `custom`, `none`).
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub account_type: Option<AccountType>,

    /// Capabilities enabled or requested on this account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<AccountCapabilities>,

    /// Information needed to satisfy KYC and identity verification requirements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirements: Option<AccountRequirements>,

    /// Whether the account can create live charges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charges_enabled: Option<bool>,

    /// Whether Stripe can send payouts to this account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payouts_enabled: Option<bool>,

    /// Country of the account (two-letter ISO 3166-1 alpha-2 code).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Default currency code for the account (three-letter ISO 4217 code).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_currency: Option<String>,

    /// Business entity type (`individual`, `company`, `non_profit`, `government_entity`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_type: Option<BusinessType>,

    /// Primary email address of the account holder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Account configuration settings (branding, payouts, card payments).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<AccountSettings>,

    /// Key-value metadata attached to this account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Whether all required onboarding details have been submitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details_submitted: Option<bool>,

    /// Time at which the account was created (UNIX epoch seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,

    /// Customer-facing business profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_profile: Option<AccountBusinessProfile>,

    /// Details on the incorporated entity (for `company` business type).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company: Option<AccountCompany>,

    /// Details on the individual account holder (for `individual` business type).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub individual: Option<AccountIndividual>,

    /// Accounts v2 controller configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller: Option<AccountController>,

    /// Details on Terms of Service acceptance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tos_acceptance: Option<AccountTosAcceptance>,
}

fn default_account_object() -> String {
    "account".into()
}

impl Account {
    /// Zero-clone borrowed slice accessor for the account ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Accessor for the account type.
    pub fn type_(&self) -> Option<AccountType> {
        self.account_type
    }

    /// Zero-clone borrowed slice accessor for country code.
    pub fn country(&self) -> Option<&str> {
        self.country.as_deref()
    }

    /// Zero-clone borrowed slice accessor for default currency.
    pub fn default_currency(&self) -> Option<&str> {
        self.default_currency.as_deref()
    }

    /// Zero-clone borrowed slice accessor for contact email.
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    /// Returns `true` if charges are enabled.
    pub fn is_charges_enabled(&self) -> bool {
        self.charges_enabled.unwrap_or(false)
    }

    /// Returns `true` if payouts are enabled.
    pub fn is_payouts_enabled(&self) -> bool {
        self.payouts_enabled.unwrap_or(false)
    }

    /// Returns `true` if onboarding is complete and no requirements are currently blocking.
    pub fn is_onboarding_complete(&self) -> bool {
        self.details_submitted.unwrap_or(false)
            && self.requirements.as_ref().map_or(true, |r| r.is_complete())
    }

    /// Borrowed reference to account capabilities.
    pub fn capabilities(&self) -> Option<&AccountCapabilities> {
        self.capabilities.as_ref()
    }

    /// Borrowed reference to verification requirements.
    pub fn requirements(&self) -> Option<&AccountRequirements> {
        self.requirements.as_ref()
    }

    /// Borrowed reference to account settings.
    pub fn settings(&self) -> Option<&AccountSettings> {
        self.settings.as_ref()
    }

    /// Borrowed reference to business profile.
    pub fn business_profile(&self) -> Option<&AccountBusinessProfile> {
        self.business_profile.as_ref()
    }

    /// Borrowed reference to company details.
    pub fn company(&self) -> Option<&AccountCompany> {
        self.company.as_ref()
    }

    /// Borrowed reference to individual details.
    pub fn individual(&self) -> Option<&AccountIndividual> {
        self.individual.as_ref()
    }

    /// Borrowed reference to controller properties.
    pub fn controller(&self) -> Option<&AccountController> {
        self.controller.as_ref()
    }

    /// Borrowed reference to Terms of Service acceptance.
    pub fn tos_acceptance(&self) -> Option<&AccountTosAcceptance> {
        self.tos_acceptance.as_ref()
    }
}

/// Request parameters for creating a new Connected Account (`POST /v1/accounts`).
///
/// Supports dual-protocol serialization: form-encoded bracket notation for API v1
/// and JSON payloads for API v2.
///
/// # Examples
///
/// ```
/// use some_stripe_types::{
///     AccountCapabilitiesParams, AccountType, CapabilityStatus, CreateAccountParams,
/// };
///
/// let params = CreateAccountParams::new()
///     .account_type(AccountType::Custom)
///     .country("US")
///     .email("merchant@example.com")
///     .capabilities(
///         AccountCapabilitiesParams::new()
///             .card_payments(CapabilityStatus::Requested)
///             .transfers(CapabilityStatus::Requested),
///     );
///
/// assert_eq!(params.account_type, Some(AccountType::Custom));
/// assert_eq!(params.country.as_deref(), Some("US"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CreateAccountParams {
    /// Connected account type (`standard`, `express`, `custom`).
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub account_type: Option<AccountType>,

    /// Two-letter country code (ISO 3166-1 alpha-2) where the account resides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Primary email address for the account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Legal entity business type (`individual`, `company`, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_type: Option<BusinessType>,

    /// Default three-letter ISO 4217 currency code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_currency: Option<String>,

    /// Capabilities to request for this account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<AccountCapabilitiesParams>,

    /// Business profile parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_profile: Option<AccountBusinessProfileParams>,

    /// Incorporated company details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company: Option<AccountCompanyParams>,

    /// Individual personal details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub individual: Option<AccountIndividualParams>,

    /// Account settings parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<AccountSettingsParams>,

    /// Terms of service acceptance attestation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tos_acceptance: Option<AccountTosAcceptanceParams>,

    /// Custom key-value metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Optional idempotency key for safe request retries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

impl CreateAccountParams {
    /// Creates a new default parameter builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the account type.
    pub fn account_type(mut self, t: impl Into<AccountType>) -> Self {
        self.account_type = Some(t.into());
        self
    }

    /// Alias for [`account_type`](Self::account_type).
    pub fn type_(mut self, t: impl Into<AccountType>) -> Self {
        self.account_type = Some(t.into());
        self
    }

    /// Sets the country code.
    pub fn country(mut self, c: impl Into<String>) -> Self {
        self.country = Some(c.into());
        self
    }

    /// Sets the contact email.
    pub fn email(mut self, e: impl Into<String>) -> Self {
        self.email = Some(e.into());
        self
    }

    /// Sets the business entity type.
    pub fn business_type(mut self, b: impl Into<BusinessType>) -> Self {
        self.business_type = Some(b.into());
        self
    }

    /// Sets the default currency code.
    pub fn default_currency(mut self, c: impl Into<String>) -> Self {
        self.default_currency = Some(c.into());
        self
    }

    /// Sets the requested capabilities.
    pub fn capabilities(mut self, c: AccountCapabilitiesParams) -> Self {
        self.capabilities = Some(c);
        self
    }

    /// Sets business profile details.
    pub fn business_profile(mut self, p: AccountBusinessProfileParams) -> Self {
        self.business_profile = Some(p);
        self
    }

    /// Sets incorporated company parameters.
    pub fn company(mut self, c: AccountCompanyParams) -> Self {
        self.company = Some(c);
        self
    }

    /// Sets individual personal parameters.
    pub fn individual(mut self, i: AccountIndividualParams) -> Self {
        self.individual = Some(i);
        self
    }

    /// Sets account settings.
    pub fn settings(mut self, s: AccountSettingsParams) -> Self {
        self.settings = Some(s);
        self
    }

    /// Sets Terms of Service acceptance attestation.
    pub fn tos_acceptance(mut self, t: AccountTosAcceptanceParams) -> Self {
        self.tos_acceptance = Some(t);
        self
    }

    /// Sets custom metadata key-value pairs.
    pub fn metadata(mut self, m: HashMap<String, String>) -> Self {
        self.metadata = Some(m);
        self
    }

    /// Sets the idempotency key for safe request retries.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Request parameters for updating an existing Connected Account (`POST /v1/accounts/{id}`).
///
/// Features three-state [`Nullable<T>`] fields to distinguish between omitting a parameter,
/// clearing it (`Nullable::Null` -> `""` / `null`), or setting a new value (`Nullable::Value(T)`).
///
/// # Examples
///
/// ```
/// use some_stripe_types::{Nullable, UpdateAccountParams};
///
/// let params = UpdateAccountParams::new()
///     .email(Nullable::Value("new_email@example.com".into()));
/// assert!(params.email.is_value());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateAccountParams {
    /// Business entity type.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub business_type: Nullable<BusinessType>,

    /// Primary email address.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub email: Nullable<String>,

    /// Default currency code.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub default_currency: Nullable<String>,

    /// Requested capabilities updates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<AccountCapabilitiesParams>,

    /// Business profile updates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_profile: Option<AccountBusinessProfileParams>,

    /// Company updates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company: Option<AccountCompanyParams>,

    /// Individual updates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub individual: Option<AccountIndividualParams>,

    /// Settings updates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<AccountSettingsParams>,

    /// Terms of service acceptance updates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tos_acceptance: Option<AccountTosAcceptanceParams>,

    /// Key-value metadata updates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Optional idempotency key for safe request retries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

impl UpdateAccountParams {
    /// Creates a new default update parameter builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets or clears the business type.
    pub fn business_type(mut self, b: Nullable<BusinessType>) -> Self {
        self.business_type = b;
        self
    }

    /// Sets or clears the contact email.
    pub fn email(mut self, e: Nullable<String>) -> Self {
        self.email = e;
        self
    }

    /// Sets or clears the default currency.
    pub fn default_currency(mut self, c: Nullable<String>) -> Self {
        self.default_currency = c;
        self
    }

    /// Sets capability updates.
    pub fn capabilities(mut self, c: AccountCapabilitiesParams) -> Self {
        self.capabilities = Some(c);
        self
    }

    /// Sets business profile updates.
    pub fn business_profile(mut self, p: AccountBusinessProfileParams) -> Self {
        self.business_profile = Some(p);
        self
    }

    /// Sets company updates.
    pub fn company(mut self, c: AccountCompanyParams) -> Self {
        self.company = Some(c);
        self
    }

    /// Sets individual updates.
    pub fn individual(mut self, i: AccountIndividualParams) -> Self {
        self.individual = Some(i);
        self
    }

    /// Sets settings updates.
    pub fn settings(mut self, s: AccountSettingsParams) -> Self {
        self.settings = Some(s);
        self
    }

    /// Sets Terms of Service acceptance attestation.
    pub fn tos_acceptance(mut self, t: AccountTosAcceptanceParams) -> Self {
        self.tos_acceptance = Some(t);
        self
    }

    /// Sets custom metadata updates.
    pub fn metadata(mut self, m: HashMap<String, String>) -> Self {
        self.metadata = Some(m);
        self
    }

    /// Sets the idempotency key for safe request retries.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for rejecting an account (`POST /v1/accounts/{id}/reject`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectAccountParams {
    /// The reason for rejecting the account. Can be `fraud`, `terms_of_service`, or `other`.
    pub reason: String,

    /// Optional idempotency key for safe request retries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

impl RejectAccountParams {
    /// Creates a new reject parameters block with the specified reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            idempotency_key: None,
        }
    }

    /// Sets the idempotency key for safe request retries.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for listing connected accounts (`GET /v1/accounts`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListAccountsParams {
    /// A limit on the number of objects to be returned, between 1 and 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// A cursor for use in pagination. An object ID defining your place in the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// A cursor for use in pagination. An object ID defining your place in the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,

    /// Filter accounts created at this UNIX timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
}

impl ListAccountsParams {
    /// Creates a new default parameter block for listing accounts.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum number of accounts to return.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets starting_after cursor.
    pub fn starting_after(mut self, cursor: impl Into<String>) -> Self {
        self.starting_after = Some(cursor.into());
        self
    }

    /// Sets ending_before cursor.
    pub fn ending_before(mut self, cursor: impl Into<String>) -> Self {
        self.ending_before = Some(cursor.into());
        self
    }

    /// Sets created timestamp filter.
    pub fn created(mut self, created: i64) -> Self {
        self.created = Some(created);
        self
    }
}

/// Paginated list of Connect Account objects (`object = "list"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountList {
    /// Object type, always `list`.
    #[serde(default = "default_list_object")]
    pub object: String,

    /// List of accounts for this page.
    pub data: Vec<Account>,

    /// True if more items exist.
    pub has_more: bool,

    /// Request URL.
    pub url: String,
}

fn default_list_object() -> String {
    "list".into()
}

/// Confirmation object returned upon account deletion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletedAccount {
    /// Unique identifier of deleted account.
    pub id: String,

    /// Object type, always `account`.
    #[serde(default = "default_account_object")]
    pub object: String,

    /// Always `true` if deleted.
    pub deleted: bool,
}

/// Function for paginator ID extraction.
pub fn account_id(account: &Account) -> &str {
    account.id()
}
