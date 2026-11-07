//! # Stripe Issuing Cardholder Domain Models & Parameters
//!
//! Strongly typed domain models, parameters, and KYC/AML compliance records for
//! Stripe Issuing Cardholders (`stripe.issuing.cardholders`), adhering to the
//! Bank Secrecy Act (BSA) and FinCEN customer due diligence rules.

use crate::issuing_card::{
    IssuingCardAddress, ParseIssuingError, SpendingControls, SpendingControlsParams,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// Entity type of the cardholder (`individual` or `company`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IssuingCardholderType {
    /// An individual person.
    #[default]
    Individual,
    /// A corporate legal entity or company.
    Company,
}

impl IssuingCardholderType {
    /// Returns the static string representation of this cardholder type.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Individual => "individual",
            Self::Company => "company",
        }
    }
}

impl fmt::Display for IssuingCardholderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for IssuingCardholderType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for IssuingCardholderType {
    type Err = ParseIssuingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "individual" => Ok(Self::Individual),
            "company" => Ok(Self::Company),
            _ => Err(ParseIssuingError(format!("Unknown cardholder type: {s}"))),
        }
    }
}

impl From<&str> for IssuingCardholderType {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or(Self::Individual)
    }
}

impl From<String> for IssuingCardholderType {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl PartialEq<str> for IssuingCardholderType {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for IssuingCardholderType {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<IssuingCardholderType> for str {
    fn eq(&self, other: &IssuingCardholderType) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<IssuingCardholderType> for &str {
    fn eq(&self, other: &IssuingCardholderType) -> bool {
        *self == other.as_str()
    }
}

/// Authorization status of the cardholder (`active`, `inactive`, `blocked`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IssuingCardholderStatus {
    /// Cardholder is active and authorized to receive and use cards.
    #[default]
    Active,
    /// Cardholder is inactive; transactions on all associated cards are declined.
    Inactive,
    /// Cardholder is blocked due to compliance or fraud flags.
    Blocked,
}

impl IssuingCardholderStatus {
    /// Returns the static string representation of this cardholder status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Blocked => "blocked",
        }
    }
}

impl fmt::Display for IssuingCardholderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for IssuingCardholderStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for IssuingCardholderStatus {
    type Err = ParseIssuingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "blocked" => Ok(Self::Blocked),
            _ => Err(ParseIssuingError(format!("Unknown cardholder status: {s}"))),
        }
    }
}

impl From<&str> for IssuingCardholderStatus {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or(Self::Active)
    }
}

impl From<String> for IssuingCardholderStatus {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl PartialEq<str> for IssuingCardholderStatus {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for IssuingCardholderStatus {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<IssuingCardholderStatus> for str {
    fn eq(&self, other: &IssuingCardholderStatus) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<IssuingCardholderStatus> for &str {
    fn eq(&self, other: &IssuingCardholderStatus) -> bool {
        *self == other.as_str()
    }
}

/// Date of birth for KYC and customer verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct IssuingDateOfBirth {
    /// Day of the month (1-31).
    pub day: u8,
    /// Month (1-12).
    pub month: u8,
    /// Four-digit year of birth.
    pub year: i32,
}

/// Billing address details for a Cardholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingCardholderBilling {
    /// Physical address for card billing.
    pub address: IssuingCardAddress,
}

/// Billing address parameters for creating or updating a Cardholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingCardholderBillingParams {
    /// Billing address.
    pub address: IssuingCardAddress,
}

/// Verification documentation for an individual cardholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingCardholderVerification {
    /// Verification document details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<IssuingCardholderVerificationDocument>,
}

/// Document details for verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingCardholderVerificationDocument {
    /// Front of identity document file ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub front: Option<String>,
    /// Back of identity document file ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back: Option<String>,
}

/// Verification parameters for individual cardholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingCardholderVerificationParams {
    /// Document file IDs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<IssuingCardholderVerificationDocument>,
}

/// Card issuing user acceptance and terms for an individual cardholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingCardholderIndividualCardIssuing {
    /// User terms acceptance details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_terms_acceptance: Option<IssuingUserTermsAcceptance>,
}

/// Acceptance timestamp and IP for card issuing terms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingUserTermsAcceptance {
    /// Date when the terms were accepted (Unix epoch seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<i64>,
    /// Client IP address that accepted the terms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    /// User agent string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
}

/// KYC details for an individual cardholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingCardholderIndividual {
    /// Card issuing terms acceptance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_issuing: Option<IssuingCardholderIndividualCardIssuing>,
    /// Date of birth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dob: Option<IssuingDateOfBirth>,
    /// Legal first name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    /// Legal last name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    /// Identity verification documents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<IssuingCardholderVerification>,
}

/// Individual KYC parameters for creating or updating a cardholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingCardholderIndividualParams {
    /// First name.
    pub first_name: String,
    /// Last name.
    pub last_name: String,
    /// Date of birth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dob: Option<IssuingDateOfBirth>,
    /// Card issuing user terms acceptance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_issuing: Option<IssuingCardholderIndividualCardIssuing>,
    /// Verification documents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<IssuingCardholderVerificationParams>,
}

/// Corporate details for a company cardholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingCardholderCompany {
    /// Whether a tax ID was provided for the company.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_id_provided: Option<bool>,
}

/// Corporate parameters for creating or updating a company cardholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingCardholderCompanyParams {
    /// Corporate taxpayer identification number (e.g. US EIN).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<String>,
}

/// KYC/AML requirements and verification status for the cardholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssuingCardholderRequirements {
    /// Reason why the cardholder is disabled, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,
    /// Fields that are past due for verification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub past_due: Option<Vec<String>>,
}

/// Represents a Stripe Issuing Cardholder (`issuing.cardholder`).
///
/// Holds identity and authorization data for individuals or corporate entities
/// permitted to hold virtual and physical commercial spend cards.
///
/// # Examples
/// ```rust
/// use some_stripe_types::{IssuingCardholder, IssuingCardholderType, IssuingCardholderStatus};
///
/// let json = r#"{
///   "id": "ich_123",
///   "object": "issuing.cardholder",
///   "billing": {
///     "address": {
///       "city": "San Francisco",
///       "country": "US",
///       "line1": "123 Market St",
///       "postal_code": "94105",
///       "state": "CA"
///     }
///   },
///   "created": 1700000000,
///   "livemode": false,
///   "name": "Jane Doe",
///   "status": "active",
///   "type": "individual"
/// }"#;
///
/// let ch: IssuingCardholder = serde_json::from_str(json).unwrap();
/// assert_eq!(ch.id, "ich_123");
/// assert_eq!(ch.name, "Jane Doe");
/// assert_eq!(ch.r#type, IssuingCardholderType::Individual);
/// assert_eq!(ch.status, IssuingCardholderStatus::Active);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuingCardholder {
    /// Unique identifier for the Cardholder (e.g., `ich_123`).
    pub id: String,
    /// String representing the object's type. Always `issuing.cardholder`.
    #[serde(default = "default_issuing_cardholder_object")]
    pub object: String,
    /// Cardholder billing address used for AVS (Address Verification Service).
    pub billing: IssuingCardholderBilling,
    /// Additional company details if `type = "company"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company: Option<IssuingCardholderCompany>,
    /// Time at which the cardholder was created (Unix epoch seconds).
    pub created: i64,
    /// Cardholder's email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Additional individual details if `type = "individual"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub individual: Option<IssuingCardholderIndividual>,
    /// Whether the cardholder exists in livemode.
    pub livemode: bool,
    /// Key-value metadata attached to this cardholder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Full legal name of the cardholder.
    pub name: String,
    /// Cardholder's phone number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<String>,
    /// Compliance requirements information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirements: Option<IssuingCardholderRequirements>,
    /// Spending controls applied across all cards owned by this cardholder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spending_controls: Option<SpendingControls>,
    /// Current cardholder status (`active`, `inactive`, `blocked`).
    pub status: IssuingCardholderStatus,
    /// Cardholder entity type (`individual` or `company`).
    #[serde(rename = "type")]
    pub r#type: IssuingCardholderType,
}

fn default_issuing_cardholder_object() -> String {
    "issuing.cardholder".to_string()
}

/// Parameters for creating a new Issuing Cardholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CreateCardholderParams {
    /// Full legal name of the cardholder.
    pub name: String,
    /// Billing address information.
    pub billing: IssuingCardholderBillingParams,
    /// Type of cardholder entity (`individual` or `company`).
    #[serde(rename = "type")]
    pub r#type: IssuingCardholderType,
    /// Cardholder's primary email.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Cardholder's telephone number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<String>,
    /// Desired status (defaults to `active`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<IssuingCardholderStatus>,
    /// Spending controls to enforce.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spending_controls: Option<SpendingControlsParams>,
    /// Individual KYC details (mandatory if `type = "individual"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub individual: Option<IssuingCardholderIndividualParams>,
    /// Company AML details (mandatory if `type = "company"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company: Option<IssuingCardholderCompanyParams>,
    /// Key-value metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Optional idempotency key for safe retries.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
    /// Specifies which fields in the response should be expanded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expand: Option<Vec<String>>,
}

/// Parameters for updating an existing Issuing Cardholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UpdateCardholderParams {
    /// Updated legal name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Updated billing address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing: Option<IssuingCardholderBillingParams>,
    /// Updated email.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Updated phone number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<String>,
    /// Updated status (`active`, `inactive`, `blocked`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<IssuingCardholderStatus>,
    /// Updated spending controls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spending_controls: Option<SpendingControlsParams>,
    /// Updated individual KYC details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub individual: Option<IssuingCardholderIndividualParams>,
    /// Updated company AML details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company: Option<IssuingCardholderCompanyParams>,
    /// Updated metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Optional idempotency key for safe retries.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
    /// Specifies which fields in the response should be expanded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expand: Option<Vec<String>>,
}

/// Parameters for listing Issuing Cardholders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ListIssuingCardholdersParams {
    /// Filter cardholders created on or after this timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
    /// Filter by cardholder email.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Filter by phone number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<String>,
    /// Filter by cardholder status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<IssuingCardholderStatus>,
    /// Filter by cardholder type (`individual` or `company`).
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<IssuingCardholderType>,
    /// Limit on number of results (1 to 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Pagination cursor: return results after this ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,
    /// Pagination cursor: return results before this ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

/// Paginated list of Issuing Cardholders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuingCardholderList {
    /// String representing the object's type. Always `list`.
    pub object: String,
    /// List of Issuing cardholder objects.
    pub data: Vec<IssuingCardholder>,
    /// True if there are more items available.
    pub has_more: bool,
    /// The URL endpoint for this list query.
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cardholder_type_conversions() {
        assert_eq!(IssuingCardholderType::Individual.as_str(), "individual");
        assert_eq!(IssuingCardholderType::Company.as_str(), "company");
        assert_eq!(
            IssuingCardholderType::from("individual"),
            IssuingCardholderType::Individual
        );
        assert_eq!(
            IssuingCardholderType::from("company"),
            IssuingCardholderType::Company
        );
        assert_eq!(IssuingCardholderType::Individual, "individual");
        assert_eq!("company", IssuingCardholderType::Company);
    }

    #[test]
    fn test_cardholder_status_conversions() {
        assert_eq!(IssuingCardholderStatus::Active.as_str(), "active");
        assert_eq!(IssuingCardholderStatus::Inactive.as_str(), "inactive");
        assert_eq!(IssuingCardholderStatus::Blocked.as_str(), "blocked");
        assert_eq!(
            IssuingCardholderStatus::from("active"),
            IssuingCardholderStatus::Active
        );
    }

    #[test]
    fn test_cardholder_serialization_roundtrip() {
        let ch = IssuingCardholder {
            id: "ich_123".into(),
            object: "issuing.cardholder".into(),
            billing: IssuingCardholderBilling {
                address: IssuingCardAddress {
                    city: "San Francisco".into(),
                    country: "US".into(),
                    line1: "123 Market St".into(),
                    line2: None,
                    postal_code: "94105".into(),
                    state: Some("CA".into()),
                },
            },
            company: None,
            created: 1700000000,
            email: Some("jenny@example.com".into()),
            individual: Some(IssuingCardholderIndividual {
                first_name: Some("Jenny".into()),
                last_name: Some("Rosen".into()),
                dob: Some(IssuingDateOfBirth {
                    day: 1,
                    month: 1,
                    year: 1980,
                }),
                card_issuing: None,
                verification: None,
            }),
            livemode: false,
            metadata: None,
            name: "Jenny Rosen".into(),
            phone_number: Some("+15555555555".into()),
            requirements: None,
            spending_controls: None,
            status: IssuingCardholderStatus::Active,
            r#type: IssuingCardholderType::Individual,
        };

        let json = serde_json::to_string(&ch).unwrap();
        let deserialized: IssuingCardholder = serde_json::from_str(&json).unwrap();
        assert_eq!(ch, deserialized);
    }
}
