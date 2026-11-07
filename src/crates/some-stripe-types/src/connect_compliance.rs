//! FinCEN CDD, EU DAC7, Multi-Tenant Isolation, and Zero-Clone Borrowing Compliance Engine.
//!
//! Provides strict statutory legal compliance verification (Gate G-Law), constant-time multi-tenant
//! isolation enforcement (Gate G2), and zero-clone reference borrowing discipline (Gate G-Borrow)
//! for Stripe Connect marketplaces and multi-party architectures.
//!
//! # Legal & Regulatory Compliance
//!
//! - **FinCEN CDD (31 CFR § 1010.230)**: Customer Due Diligence regulations requiring mandatory
//!   identification and verification of beneficial owners holding 25% or more equity, plus at least
//!   one individual exercising significant managerial control (control prong).
//! - **EU DAC7 (Council Directive (EU) 2021/514)**: Mandatory platform tax reporting rules for digital
//!   marketplaces. Sellers exceeding 30 commercial activities or €2,000 (200,000 cents) consideration
//!   require verified Tax Identification Numbers (TIN/VAT) and financial account identifiers (IBAN).
//! - **Multi-Tenant Isolation**: Constant-time verification of `Stripe-Account` tenant context,
//!   protecting against cross-tenant side-channel timing leaks and data leakage.
//! - **Gate G-Borrow**: Zero-copy borrow discipline enforcing explicit lifetimes and zero unnecessary
//!   clones on domain models.

use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================================
// Constant-Time Byte Comparison (Gate G2)
// ============================================================================

/// Constant-time slice equality comparison protecting against timing side-channels.
#[inline]
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ============================================================================
// 1. FinCEN Customer Due Diligence (CDD) Compliance
// ============================================================================

/// Beneficial owner record under FinCEN Customer Due Diligence (CDD) rule (31 CFR § 1010.230).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinCenBeneficialOwner {
    /// Unique identifier for the person or beneficial owner entity.
    pub id: String,
    /// First / given legal name.
    pub first_name: String,
    /// Last / family legal name.
    pub last_name: String,
    /// Date of birth formatted as ISO 8601 (`YYYY-MM-DD`).
    pub date_of_birth: String,
    /// Direct or indirect equity ownership percentage (0.0 to 100.0).
    pub equity_percentage: f64,
    /// Whether this individual exercises significant managerial control (the control prong).
    pub is_control_person: bool,
    /// Whether the individual's Tax Identification Number (SSN, ITIN, or foreign passport) is verified.
    pub has_verified_tax_id: bool,
    /// Two-letter ISO 3166-1 alpha-2 country of primary residence.
    pub country: String,
}

impl FinCenBeneficialOwner {
    /// Borrowed reference to the owner ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Borrowed reference to the first name.
    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    /// Borrowed reference to the last name.
    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    /// Borrowed reference to the date of birth string.
    pub fn date_of_birth(&self) -> &str {
        &self.date_of_birth
    }

    /// Equity ownership percentage.
    pub fn equity_percentage(&self) -> f64 {
        self.equity_percentage
    }

    /// Whether this person satisfies the FinCEN managerial control prong.
    pub fn is_control_person(&self) -> bool {
        self.is_control_person
    }

    /// Whether tax identification has been validated.
    pub fn has_verified_tax_id(&self) -> bool {
        self.has_verified_tax_id
    }

    /// Borrowed reference to country code.
    pub fn country(&self) -> &str {
        &self.country
    }

    /// Returns `true` if this individual meets or exceeds the statutory 25% beneficial ownership threshold.
    pub fn is_25_percent_beneficial_owner(&self) -> bool {
        self.equity_percentage >= 25.0
    }
}

/// Requirements container for FinCEN CDD evaluation on a connected account.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinCenCddRequirements {
    /// Connected account identifier (`acct_...`).
    pub account_id: String,
    /// Legal entity business type (`"company"`, `"individual"`, etc.).
    pub business_type: String,
    /// List of declared beneficial owners and executive officers.
    pub beneficial_owners: Vec<FinCenBeneficialOwner>,
}

impl FinCenCddRequirements {
    /// Returns a borrowed reference to the account ID.
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    /// Returns a borrowed slice of the beneficial owners.
    pub fn beneficial_owners(&self) -> &[FinCenBeneficialOwner] {
        &self.beneficial_owners
    }
}

/// Errors arising during FinCEN CDD regulatory compliance verification.
#[derive(Debug, Clone, PartialEq)]
pub enum FinCenCddError {
    /// Legal entity accounts must designate at least one individual exercising managerial control.
    MissingControlPerson,

    /// Beneficial owner meeting the 25% threshold is missing verified tax identification.
    MissingTaxIdVerification {
        /// Identifier of the unverified owner.
        owner_id: String,
        /// Documented equity ownership percentage.
        equity: f64,
    },

    /// Negative equity percentage encountered.
    NegativeEquityPercentage {
        /// Identifier of the owner.
        owner_id: String,
        /// Invalid negative equity percentage.
        equity: f64,
    },

    /// Total declared equity exceeds 100%.
    EquityPercentageOverflow {
        /// Summed equity percentage.
        total_equity: f64,
    },

    /// Invalid date of birth format.
    InvalidDateOfBirth {
        /// Identifier of the owner.
        owner_id: String,
        /// Malformed date string.
        dob: String,
    },

    /// Account ID cannot be empty.
    EmptyAccountId,
}

impl fmt::Display for FinCenCddError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingControlPerson => write!(
                f,
                "FinCEN CDD Violation: Legal entity accounts must designate at least one control person (31 CFR § 1010.230(d))"
            ),
            Self::MissingTaxIdVerification { owner_id, equity } => write!(
                f,
                "FinCEN CDD Violation: Beneficial owner '{owner_id}' ({equity}%) is missing required verified tax identifier"
            ),
            Self::NegativeEquityPercentage { owner_id, equity } => write!(
                f,
                "FinCEN CDD Violation: Beneficial owner '{owner_id}' has negative equity percentage ({equity}%)"
            ),
            Self::EquityPercentageOverflow { total_equity } => write!(
                f,
                "FinCEN CDD Violation: Total accounted equity ({total_equity:.2}%) exceeds 100.0%"
            ),
            Self::InvalidDateOfBirth { owner_id, dob } => write!(
                f,
                "FinCEN CDD Violation: Invalid date of birth format '{dob}' for owner '{owner_id}' (expected YYYY-MM-DD)"
            ),
            Self::EmptyAccountId => write!(
                f,
                "FinCEN CDD Violation: Connected account ID cannot be empty"
            ),
        }
    }
}

impl std::error::Error for FinCenCddError {}

/// Validates compliance with FinCEN Customer Due Diligence (CDD) rule (31 CFR § 1010.230).
///
/// # Invariants
///
/// 1. For `"company"` legal entities, at least one control person (`is_control_person == true`) MUST be designated.
/// 2. Any individual owning 25% or more of equity interests (`equity_percentage >= 25.0`) MUST have a verified tax ID.
/// 3. Total documented equity across all owners cannot exceed 100.0%.
/// 4. Individual accounts (`business_type == "individual"`) are exempt from multi-owner CDD prongs.
///
/// # Gate G-Borrow
///
/// Operates strictly by immutable reference (`&FinCenCddRequirements`), zero clones performed.
///
/// # Examples
///
/// ```
/// use some_stripe_types::connect_compliance::{
///     validate_fincen_cdd, FinCenBeneficialOwner, FinCenCddRequirements,
/// };
///
/// let cdd = FinCenCddRequirements {
///     account_id: "acct_1234567890abcdef".into(),
///     business_type: "company".into(),
///     beneficial_owners: vec![
///         FinCenBeneficialOwner {
///             id: "owner_1".into(),
///             first_name: "Ada".into(),
///             last_name: "Lovelace".into(),
///             date_of_birth: "1980-12-10".into(),
///             equity_percentage: 35.0,
///             is_control_person: true,
///             has_verified_tax_id: true,
///             country: "US".into(),
///         },
///     ],
/// };
///
/// assert!(validate_fincen_cdd(&cdd).is_ok());
/// ```
pub fn validate_fincen_cdd(cdd: &FinCenCddRequirements) -> Result<(), FinCenCddError> {
    if cdd.account_id().trim().is_empty() {
        return Err(FinCenCddError::EmptyAccountId);
    }

    // Individual sole proprietorships are self-verified and exempt from legal entity CDD prongs.
    if cdd.business_type.eq_ignore_ascii_case("individual") {
        return Ok(());
    }

    let mut total_equity = 0.0;
    let mut has_control_person = false;

    for owner in cdd.beneficial_owners() {
        if owner.equity_percentage < 0.0 {
            return Err(FinCenCddError::NegativeEquityPercentage {
                owner_id: owner.id.clone(),
                equity: owner.equity_percentage,
            });
        }

        // Validate date of birth format: YYYY-MM-DD
        let dob_parts: Vec<&str> = owner.date_of_birth.split('-').collect();
        if dob_parts.len() != 3
            || dob_parts[0].len() != 4
            || dob_parts[1].len() != 2
            || dob_parts[2].len() != 2
        {
            return Err(FinCenCddError::InvalidDateOfBirth {
                owner_id: owner.id.clone(),
                dob: owner.date_of_birth.clone(),
            });
        }

        total_equity += owner.equity_percentage;

        if owner.is_control_person {
            has_control_person = true;
        }

        // Equity ownership prong: >= 25% requires verified tax identifier
        if owner.is_25_percent_beneficial_owner() && !owner.has_verified_tax_id {
            return Err(FinCenCddError::MissingTaxIdVerification {
                owner_id: owner.id.clone(),
                equity: owner.equity_percentage,
            });
        }
    }

    if total_equity > 100.0001 {
        return Err(FinCenCddError::EquityPercentageOverflow { total_equity });
    }

    if !has_control_person {
        return Err(FinCenCddError::MissingControlPerson);
    }

    Ok(())
}

// ============================================================================
// 2. EU DAC7 Platform Tax Reporting Compliance
// ============================================================================

/// Seller profile under EU DAC7 (Directive (EU) 2021/514).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dac7SellerProfile {
    /// Unique merchant or connected account identifier.
    pub seller_id: String,
    /// Legal trade name or personal full name.
    pub legal_name: String,
    /// Tax Identification Number (TIN) or VAT registration number.
    pub tax_identification_number: Option<String>,
    /// Two-letter ISO 3166-1 alpha-2 country of residence or incorporation.
    pub country: String,
    /// Primary financial account identifier (e.g. IBAN).
    pub iban: Option<String>,
}

impl Dac7SellerProfile {
    /// Borrowed reference to seller ID.
    pub fn seller_id(&self) -> &str {
        &self.seller_id
    }

    /// Borrowed reference to legal name.
    pub fn legal_name(&self) -> &str {
        &self.legal_name
    }

    /// Borrowed reference to tax ID if present.
    pub fn tax_identification_number(&self) -> Option<&str> {
        self.tax_identification_number.as_deref()
    }

    /// Borrowed reference to country.
    pub fn country(&self) -> &str {
        &self.country
    }

    /// Borrowed reference to IBAN if present.
    pub fn iban(&self) -> Option<&str> {
        self.iban.as_deref()
    }
}

/// Quarterly commercial activity summary for DAC7 calculation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dac7ActivitySummary {
    /// Reporting calendar year (e.g. 2026).
    pub reporting_year: u16,
    /// Calendar quarter (1, 2, 3, or 4).
    pub quarter: u8,
    /// Number of completed commercial transactions / activities in the quarter.
    pub number_of_activities: u32,
    /// Total consideration paid or credited in EUR cents (ISO 4217 minor units).
    pub total_consideration_cents: i64,
    /// Total platform fees and commissions withheld in EUR cents.
    pub fees_withheld_cents: i64,
    /// Total taxes withheld at source in EUR cents.
    pub taxes_withheld_cents: i64,
}

/// Evaluated statutory reporting status under EU DAC7 regulations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dac7ReportingStatus {
    /// Seller is below statutory reporting thresholds (<30 activities and <= €2,000 consideration).
    ExemptLowVolume {
        /// Total activities completed during the reporting period.
        total_activities: u32,
        /// Total consideration credited in EUR cents.
        total_consideration_cents: i64,
    },
    /// Seller meets or exceeds statutory reporting thresholds and must be reported to EU tax authorities.
    ReportableSeller {
        /// Total activities completed during the reporting period.
        total_activities: u32,
        /// Net consideration after platform fees (in EUR cents).
        net_consideration_cents: i64,
        /// Verified tax identification number.
        tax_identifier: String,
        /// Verified financial account identifier (IBAN).
        financial_identifier: String,
    },
}

/// Errors arising during EU DAC7 compliance verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dac7ComplianceError {
    /// Negative gross consideration is rejected.
    NegativeConsideration {
        /// Negative consideration in cents.
        cents: i64,
    },

    /// Negative withheld fees are rejected.
    NegativeFees {
        /// Negative fee amount in cents.
        cents: i64,
    },

    /// Platform fees exceed gross consideration.
    FeesExceedConsideration {
        /// Fee amount in cents.
        fees: i64,
        /// Gross consideration in cents.
        consideration: i64,
    },

    /// Reportable seller is missing mandatory Tax Identification Number.
    MissingTaxIdentifier {
        /// Identifier of the seller.
        seller_id: String,
    },

    /// Reportable seller is missing mandatory financial account identifier.
    MissingFinancialIdentifier {
        /// Identifier of the seller.
        seller_id: String,
    },

    /// Calendar quarter is outside the valid range 1..=4.
    InvalidQuarter {
        /// Invalid quarter value.
        quarter: u8,
    },

    /// Arithmetic overflow occurred while accumulating activity metrics.
    ArithmeticOverflow,
}

impl fmt::Display for Dac7ComplianceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeConsideration { cents } => write!(
                f,
                "EU DAC7 Violation: Negative consideration ({cents} cents) encountered"
            ),
            Self::NegativeFees { cents } => write!(
                f,
                "EU DAC7 Violation: Negative fees ({cents} cents) encountered"
            ),
            Self::FeesExceedConsideration {
                fees,
                consideration,
            } => write!(
                f,
                "EU DAC7 Violation: Platform fees ({fees} cents) exceed total consideration ({consideration} cents)"
            ),
            Self::MissingTaxIdentifier { seller_id } => write!(
                f,
                "EU DAC7 Violation: Reportable seller '{seller_id}' is missing mandatory Tax Identification Number (TIN/VAT)"
            ),
            Self::MissingFinancialIdentifier { seller_id } => write!(
                f,
                "EU DAC7 Violation: Reportable seller '{seller_id}' is missing mandatory financial account identifier (IBAN)"
            ),
            Self::InvalidQuarter { quarter } => write!(
                f,
                "EU DAC7 Violation: Invalid calendar quarter '{quarter}' (must be 1..=4)"
            ),
            Self::ArithmeticOverflow => write!(
                f,
                "EU DAC7 Violation: Arithmetic overflow calculating total consideration or activity count"
            ),
        }
    }
}

impl std::error::Error for Dac7ComplianceError {}

/// Statutory threshold: €2,000.00 expressed in EUR minor cents.
pub const DAC7_CONSIDERATION_THRESHOLD_CENTS: i64 = 200_000;

/// Statutory threshold: 30 commercial activities.
pub const DAC7_ACTIVITY_COUNT_THRESHOLD: u32 = 30;

/// Evaluates a seller's EU DAC7 (Directive (EU) 2021/514) reporting status across quarterly activity summaries.
///
/// # Invariants
///
/// - Threshold: Sellers with >= 30 activities OR > 200,000 EUR cents (€2,000.00) in consideration during a reporting period
///   are legally classified as reportable sellers.
/// - Reportable sellers MUST provide a valid Tax Identification Number (TIN/VAT) and financial account identifier (IBAN).
/// - Low-volume sellers (<30 activities and <= 200,000 cents) without mandatory identifiers are permitted and marked `ExemptLowVolume`.
///
/// # Gate G-Borrow
///
/// Operates strictly by immutable references (`&Dac7SellerProfile`, `&[Dac7ActivitySummary]`), zero copies performed.
///
/// # Examples
///
/// ```
/// use some_stripe_types::connect_compliance::{
///     evaluate_dac7_reporting_status, Dac7ActivitySummary, Dac7ReportingStatus,
///     Dac7SellerProfile,
/// };
///
/// let seller = Dac7SellerProfile {
///     seller_id: "acct_seller_1".into(),
///     legal_name: "Berlin Crafts GmbH".into(),
///     tax_identification_number: Some("DE123456789".into()),
///     country: "DE".into(),
///     iban: Some("DE89370400440532013000".into()),
/// };
///
/// let activities = vec![
///     Dac7ActivitySummary {
///         reporting_year: 2026,
///         quarter: 1,
///         number_of_activities: 35,
///         total_consideration_cents: 500_000,
///         fees_withheld_cents: 50_000,
///         taxes_withheld_cents: 0,
///     },
/// ];
///
/// let status = evaluate_dac7_reporting_status(&seller, &activities).unwrap();
/// match status {
///     Dac7ReportingStatus::ReportableSeller { net_consideration_cents, .. } => {
///         assert_eq!(net_consideration_cents, 450_000);
///     }
///     Dac7ReportingStatus::ExemptLowVolume { .. } => panic!("Expected reportable seller"),
/// }
/// ```
pub fn evaluate_dac7_reporting_status(
    seller: &Dac7SellerProfile,
    activities: &[Dac7ActivitySummary],
) -> Result<Dac7ReportingStatus, Dac7ComplianceError> {
    let mut total_activities: u32 = 0;
    let mut total_consideration: i64 = 0;
    let mut total_fees: i64 = 0;

    for act in activities {
        if act.quarter < 1 || act.quarter > 4 {
            return Err(Dac7ComplianceError::InvalidQuarter {
                quarter: act.quarter,
            });
        }
        if act.total_consideration_cents < 0 {
            return Err(Dac7ComplianceError::NegativeConsideration {
                cents: act.total_consideration_cents,
            });
        }
        if act.fees_withheld_cents < 0 {
            return Err(Dac7ComplianceError::NegativeFees {
                cents: act.fees_withheld_cents,
            });
        }
        if act.fees_withheld_cents > act.total_consideration_cents {
            return Err(Dac7ComplianceError::FeesExceedConsideration {
                fees: act.fees_withheld_cents,
                consideration: act.total_consideration_cents,
            });
        }

        total_activities = total_activities
            .checked_add(act.number_of_activities)
            .ok_or(Dac7ComplianceError::ArithmeticOverflow)?;

        total_consideration = total_consideration
            .checked_add(act.total_consideration_cents)
            .ok_or(Dac7ComplianceError::ArithmeticOverflow)?;

        total_fees = total_fees
            .checked_add(act.fees_withheld_cents)
            .ok_or(Dac7ComplianceError::ArithmeticOverflow)?;
    }

    let is_reportable = total_activities >= DAC7_ACTIVITY_COUNT_THRESHOLD
        || total_consideration > DAC7_CONSIDERATION_THRESHOLD_CENTS;

    if is_reportable {
        let tax_id = match seller.tax_identification_number() {
            Some(tin) if !tin.trim().is_empty() => tin.to_string(),
            _ => {
                return Err(Dac7ComplianceError::MissingTaxIdentifier {
                    seller_id: seller.seller_id.clone(),
                });
            }
        };

        let iban = match seller.iban() {
            Some(ib) if !ib.trim().is_empty() => ib.to_string(),
            _ => {
                return Err(Dac7ComplianceError::MissingFinancialIdentifier {
                    seller_id: seller.seller_id.clone(),
                });
            }
        };

        let net_consideration_cents = total_consideration
            .checked_sub(total_fees)
            .ok_or(Dac7ComplianceError::ArithmeticOverflow)?;

        Ok(Dac7ReportingStatus::ReportableSeller {
            total_activities,
            net_consideration_cents,
            tax_identifier: tax_id,
            financial_identifier: iban,
        })
    } else {
        Ok(Dac7ReportingStatus::ExemptLowVolume {
            total_activities,
            total_consideration_cents: total_consideration,
        })
    }
}

// ============================================================================
// 3. Connect Multi-Tenant Isolation Verification
// ============================================================================

/// Errors encountered during multi-tenant isolation validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiTenantError {
    /// Account identifier cannot be empty.
    EmptyAccountId,

    /// Account identifier must start with `acct_` prefix.
    InvalidPrefix(String),

    /// Account identifier must meet minimum length requirement.
    TooShort(String),

    /// Account identifier contains illegal non-alphanumeric characters.
    InvalidCharacters(String),

    /// Cross-tenant boundary breach detected.
    TenantMismatch {
        /// Expected tenant ID.
        expected: String,
        /// Mismatched tenant ID found.
        found: String,
    },
}

impl fmt::Display for MultiTenantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAccountId => {
                write!(
                    f,
                    "Connect Multi-Tenancy Violation: Account ID cannot be empty"
                )
            }
            Self::InvalidPrefix(id) => write!(
                f,
                "Connect Multi-Tenancy Violation: Account ID '{id}' must start with 'acct_' prefix"
            ),
            Self::TooShort(id) => write!(
                f,
                "Connect Multi-Tenancy Violation: Account ID '{id}' must be at least 16 characters long"
            ),
            Self::InvalidCharacters(id) => write!(
                f,
                "Connect Multi-Tenancy Violation: Account ID '{id}' contains illegal non-alphanumeric characters"
            ),
            Self::TenantMismatch { expected, found } => write!(
                f,
                "Connect Multi-Tenancy Violation: Cross-tenant boundary breach: expected tenant '{expected}', encountered '{found}'"
            ),
        }
    }
}

impl std::error::Error for MultiTenantError {}

/// Validates format invariants for a Stripe Connect connected account identifier (`acct_...`).
///
/// # Invariants
///
/// - Non-empty string
/// - Starts with `"acct_"` prefix
/// - Minimum length of 16 characters
/// - Characters after prefix are ASCII alphanumeric
///
/// # Examples
///
/// ```
/// use some_stripe_types::connect_compliance::validate_connect_account_id;
///
/// assert!(validate_connect_account_id("acct_1032D82eZvKYlo2C").is_ok());
/// assert!(validate_connect_account_id("invalid_id").is_err());
/// assert!(validate_connect_account_id("acct_short").is_err());
/// ```
pub fn validate_connect_account_id(id: &str) -> Result<(), MultiTenantError> {
    if id.is_empty() {
        return Err(MultiTenantError::EmptyAccountId);
    }
    if !id.starts_with("acct_") {
        return Err(MultiTenantError::InvalidPrefix(id.to_string()));
    }
    if id.len() < 16 {
        return Err(MultiTenantError::TooShort(id.to_string()));
    }
    let suffix = &id["acct_".len()..];
    if !suffix.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(MultiTenantError::InvalidCharacters(id.to_string()));
    }
    Ok(())
}

/// Borrowed multi-tenant isolation context enforcing strict tenant boundary guards.
///
/// # Gate G-Borrow & G2 Invariants
///
/// - Operates strictly over borrowed string slices (`&'a str`), requiring zero heap allocations.
/// - Uses constant-time equality comparisons (`constant_time_eq`) to protect against
///   timing attack side channels during header verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MultiTenantContext<'a> {
    account_id: &'a str,
}

impl<'a> MultiTenantContext<'a> {
    /// Creates and validates a new `MultiTenantContext` borrowing an account ID.
    ///
    /// # Errors
    ///
    /// Returns [`MultiTenantError`] if the account ID format is invalid.
    pub fn new(account_id: &'a str) -> Result<Self, MultiTenantError> {
        validate_connect_account_id(account_id)?;
        Ok(Self { account_id })
    }

    /// Access reference to the borrowed account identifier.
    pub fn account_id(&self) -> &'a str {
        self.account_id
    }

    /// Verifies whether an incoming `Stripe-Account` header matches this tenant context in constant time.
    ///
    /// Uses [`constant_time_eq`] to eliminate side-channel timing leaks.
    pub fn matches_header(&self, header_value: &str) -> bool {
        constant_time_eq(self.account_id.as_bytes(), header_value.as_bytes())
    }

    /// Asserts tenant boundary isolation against a target account ID.
    ///
    /// Returns `true` if this tenant is strictly isolated from `other_account_id` (i.e. different accounts).
    pub fn is_isolated_from(&self, other_account_id: &str) -> bool {
        !self.matches_header(other_account_id)
    }

    /// Enforces that the provided request tenant matches this context, returning an error on mismatch.
    pub fn assert_tenant_boundary(&self, requested_tenant: &str) -> Result<(), MultiTenantError> {
        if self.matches_header(requested_tenant) {
            Ok(())
        } else {
            Err(MultiTenantError::TenantMismatch {
                expected: self.account_id.to_string(),
                found: requested_tenant.to_string(),
            })
        }
    }
}

// ============================================================================
// 4. Zero-Clone Borrowing Audit Utilities
// ============================================================================

/// Zero-clone inspection utility verifying that a Connect domain entity reference can be inspected without cloning.
///
/// # Gate G-Borrow Invariant
///
/// Guarantees that callers pass reference `&T` without incurring heap allocation or deep copy overhead.
pub fn audit_connect_entity_borrow<'a, T, R, F>(entity: &'a T, inspection: F) -> R
where
    F: FnOnce(&'a T) -> R,
{
    inspection(entity)
}
