//! Stripe KYC Person Domain Entities, Beneficial Ownership & Customer Due Diligence Schemas.
//!
//! Enforces:
//! - **Gate G-Law & FinCEN CDD Rule (31 CFR § 1010.230)**:
//!   Mandatory identification and verification of beneficial owners (individuals owning $\ge 25\%$ equity)
//!   and designated control persons (representatives, executives, directors).
//! - **Gate G-Law & EU DAC7 / AMLD5 (Directive 2021/514 & Directive 2018/843)**:
//!   Collection, verification, and tax identification reporting for natural person sellers in the EU.
//! - **Gate G2 & PCI-DSS / PII Security**:
//!   Strict redaction of sensitive identification numbers (`id_number`, `id_number_secondary`, `ssn_last_4`)
//!   in all [`std::fmt::Debug`] implementations to prevent leakage into application logs.
//! - **Gate G-Borrow**:
//!   Zero deep clone borrowing discipline and ergonomic builders.
//! - **Dual-Protocol 3-State Nullability**:
//!   [`UpdatePersonParams`] uses [`Nullable<T>`] for explicit field mutation semantics (`Value`, `Null`, `Omit`).
//!
//! # Examples
//!
//! ```
//! use some_stripe_types::person::{Person, PersonRelationship, PersonDob};
//!
//! let json = r#"{
//!     "id": "person_12345",
//!     "object": "person",
//!     "account": "acct_12345",
//!     "first_name": "Jane",
//!     "last_name": "Doe",
//!     "dob": { "day": 15, "month": 6, "year": 1985 },
//!     "relationship": {
//!         "representative": true,
//!         "owner": true,
//!         "percent_ownership": 50.0,
//!         "title": "CEO"
//!     },
//!     "created": 1728345600
//! }"#;
//!
//! let person: Person = serde_json::from_str(json).unwrap();
//! assert_eq!(person.id, "person_12345");
//! assert_eq!(person.first_name.as_deref(), Some("Jane"));
//! assert_eq!(person.last_name.as_deref(), Some("Doe"));
//!
//! let rel = person.relationship.as_ref().unwrap();
//! assert!(rel.is_representative());
//! assert!(rel.is_owner());
//! assert!(rel.is_beneficial_owner());
//! assert!(rel.meets_fincen_cdd_ownership_threshold());
//! ```

pub use crate::customer::CustomerAddress;
pub use crate::nullable::Nullable;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a Stripe KYC Person object associated with a connected account.
///
/// In Stripe Connect, Person objects represent individuals associated with an account's legal
/// entity (e.g., representatives, beneficial owners, directors, executives).
///
/// Sensitive PII fields such as `id_number`, `id_number_secondary`, and `ssn_last_4` are masked
/// as `[REDACTED]` in [`std::fmt::Debug`] outputs to uphold Gate G-Law and Gate G2 invariants.
///
/// # Examples
///
/// ```
/// use some_stripe_types::person::{Person, PersonRelationship};
///
/// let person = Person {
///     id: "person_test_123".into(),
///     first_name: Some("John".into()),
///     last_name: Some("Smith".into()),
///     id_number: Some("123-45-6789".into()),
///     relationship: Some(PersonRelationship {
///         owner: Some(true),
///         percent_ownership: Some(30.0),
///         ..Default::default()
///     }),
///     ..Default::default()
/// };
///
/// // Verify PII redaction in debug output
/// let debug_str = format!("{person:?}");
/// assert!(debug_str.contains("[REDACTED]"));
/// assert!(!debug_str.contains("123-45-6789"));
/// ```
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Person {
    /// Unique identifier for the person (`person_...`).
    pub id: String,

    /// Object type, always `person`.
    #[serde(default = "default_person_object")]
    pub object: String,

    /// The connected account the person is associated with (`acct_...`).
    pub account: Option<String>,

    /// The person's legal first name.
    pub first_name: Option<String>,

    /// The person's legal last name.
    pub last_name: Option<String>,

    /// The person's maiden name.
    pub maiden_name: Option<String>,

    /// The person's primary email address.
    pub email: Option<String>,

    /// The person's primary phone number.
    pub phone: Option<String>,

    /// The person's date of birth.
    pub dob: Option<PersonDob>,

    /// The person's residential address.
    pub address: Option<CustomerAddress>,

    /// The person's registered address (if distinct from residential address).
    pub registered_address: Option<CustomerAddress>,

    /// The person's relationship to the legal entity/account.
    pub relationship: Option<PersonRelationship>,

    /// Verification status of the person.
    pub verification: Option<PersonVerification>,

    /// Identity documents uploaded for this person.
    pub documents: Option<PersonDocuments>,

    /// Whether the person's national ID number was provided.
    pub id_number_provided: Option<bool>,

    /// Whether the person's secondary ID number was provided.
    pub id_number_secondary_provided: Option<bool>,

    /// Whether the last 4 digits of the person's SSN was provided.
    pub ssn_last_4_provided: Option<bool>,

    /// Sensitive identification number (masked in Debug).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_number: Option<String>,

    /// Sensitive secondary identification number (masked in Debug).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_number_secondary: Option<String>,

    /// Sensitive SSN last 4 digits (masked in Debug).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssn_last_4: Option<String>,

    /// Two-letter ISO country code representing the person's nationality.
    pub nationality: Option<String>,

    /// Whether the person is a politically exposed person (`existing`, `none`).
    pub political_exposure: Option<String>,

    /// Outstanding requirements for this person to maintain account active status.
    pub requirements: Option<PersonRequirements>,

    /// Future requirements that will need to be collected for this person.
    pub future_requirements: Option<PersonRequirements>,

    /// Key-value metadata attached to this person.
    pub metadata: Option<HashMap<String, String>>,

    /// Time at which the person was created (Unix epoch seconds).
    #[serde(default)]
    pub created: i64,
}

impl Default for Person {
    fn default() -> Self {
        Self {
            id: String::new(),
            object: default_person_object(),
            account: None,
            first_name: None,
            last_name: None,
            maiden_name: None,
            email: None,
            phone: None,
            dob: None,
            address: None,
            registered_address: None,
            relationship: None,
            verification: None,
            documents: None,
            id_number_provided: None,
            id_number_secondary_provided: None,
            ssn_last_4_provided: None,
            id_number: None,
            id_number_secondary: None,
            ssn_last_4: None,
            nationality: None,
            political_exposure: None,
            requirements: None,
            future_requirements: None,
            metadata: None,
            created: 0,
        }
    }
}

/// Custom [`std::fmt::Debug`] implementation for [`Person`] that masks sensitive ID fields (Gate G-Law & G2).
impl std::fmt::Debug for Person {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Person")
            .field("id", &self.id)
            .field("object", &self.object)
            .field("account", &self.account)
            .field("first_name", &self.first_name)
            .field("last_name", &self.last_name)
            .field("maiden_name", &self.maiden_name)
            .field("email", &self.email)
            .field("phone", &self.phone)
            .field("dob", &self.dob)
            .field("address", &self.address)
            .field("registered_address", &self.registered_address)
            .field("relationship", &self.relationship)
            .field("verification", &self.verification)
            .field("documents", &self.documents)
            .field("id_number_provided", &self.id_number_provided)
            .field(
                "id_number_secondary_provided",
                &self.id_number_secondary_provided,
            )
            .field("ssn_last_4_provided", &self.ssn_last_4_provided)
            .field("id_number", &self.id_number.as_ref().map(|_| "[REDACTED]"))
            .field(
                "id_number_secondary",
                &self.id_number_secondary.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "ssn_last_4",
                &self.ssn_last_4.as_ref().map(|_| "[REDACTED]"),
            )
            .field("nationality", &self.nationality)
            .field("political_exposure", &self.political_exposure)
            .field("requirements", &self.requirements)
            .field("future_requirements", &self.future_requirements)
            .field("metadata", &self.metadata)
            .field("created", &self.created)
            .finish()
    }
}

fn default_person_object() -> String {
    "person".into()
}

/// Date of birth details for a Person.
///
/// # Examples
///
/// ```
/// use some_stripe_types::person::PersonDob;
///
/// let dob = PersonDob::new(1990, 8, 24);
/// assert!(dob.is_valid());
/// assert_eq!(dob.year, Some(1990));
/// assert_eq!(dob.month, Some(8));
/// assert_eq!(dob.day, Some(24));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PersonDob {
    /// The day of birth (1-31).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day: Option<u8>,

    /// The month of birth (1-12).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month: Option<u8>,

    /// The four-digit year of birth (e.g., 1985).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
}

impl PersonDob {
    /// Constructs a new [`PersonDob`] with the given year, month, and day.
    pub fn new(year: i32, month: u8, day: u8) -> Self {
        Self {
            year: Some(year),
            month: Some(month),
            day: Some(day),
        }
    }

    /// Returns `true` if all components are present and within plausible human lifespans.
    pub fn is_valid(&self) -> bool {
        match (self.year, self.month, self.day) {
            (Some(y), Some(m), Some(d)) => {
                (1900..=2100).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d)
            }
            _ => false,
        }
    }
}

/// Describes the relationship between a person and the connected account.
///
/// Enforces FinCEN Customer Due Diligence (31 CFR § 1010.230) and EU AMLD5/DAC7
/// beneficial ownership identification criteria.
///
/// # FinCEN CDD Rule Invariant (Gate G-Law)
/// Under 31 CFR § 1010.230(d):
/// - **Beneficial Ownership Prong**: Any individual with $\ge 25\%$ equity stake.
/// - **Control Prong**: An individual with significant responsibility to control,
///   manage, or direct a legal entity (e.g. representative, executive, or director).
///
/// # Examples
///
/// ```
/// use some_stripe_types::person::PersonRelationship;
///
/// let rel = PersonRelationship::new()
///     .representative(true)
///     .owner(true)
///     .percent_ownership(25.5)
///     .title("Chief Executive Officer");
///
/// assert!(rel.is_representative());
/// assert!(rel.is_owner());
/// assert!(rel.is_beneficial_owner());
/// assert!(rel.meets_fincen_cdd_ownership_threshold());
/// assert!(rel.is_control_person());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PersonRelationship {
    /// Whether the person is the primary representative of the account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub representative: Option<bool>,

    /// Whether the person is an owner of the company.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<bool>,

    /// Whether the person is an executive of the company.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executive: Option<bool>,

    /// Whether the person is a director of the company.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub director: Option<bool>,

    /// The percentage of ownership the person has in the company ($0.0 - 100.0$).
    /// FinCEN CDD Rule triggers mandatory beneficial owner identification at $\ge 25.0\%$.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percent_ownership: Option<f64>,

    /// The person's title (e.g., "CEO", "Managing Member").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl PersonRelationship {
    /// Creates a new, empty [`PersonRelationship`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether the person is the primary account representative.
    pub fn representative(mut self, rep: bool) -> Self {
        self.representative = Some(rep);
        self
    }

    /// Sets whether the person is an owner of the company.
    pub fn owner(mut self, owner: bool) -> Self {
        self.owner = Some(owner);
        self
    }

    /// Sets whether the person is an executive of the company.
    pub fn executive(mut self, exec: bool) -> Self {
        self.executive = Some(exec);
        self
    }

    /// Sets whether the person is a director of the company.
    pub fn director(mut self, dir: bool) -> Self {
        self.director = Some(dir);
        self
    }

    /// Sets the percentage of equity ownership.
    pub fn percent_ownership(mut self, pct: f64) -> Self {
        self.percent_ownership = Some(pct);
        self
    }

    /// Sets the person's job title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Returns `true` if the person is designated as the primary representative.
    pub fn is_representative(&self) -> bool {
        self.representative.unwrap_or(false)
    }

    /// Returns `true` if the person is designated as an owner.
    pub fn is_owner(&self) -> bool {
        self.owner.unwrap_or(false)
    }

    /// Returns `true` if the person is designated as an executive.
    pub fn is_executive(&self) -> bool {
        self.executive.unwrap_or(false)
    }

    /// Returns `true` if the person is designated as a director.
    pub fn is_director(&self) -> bool {
        self.director.unwrap_or(false)
    }

    /// Returns `true` if the person qualifies as a beneficial owner under FinCEN CDD
    /// or EU AMLD5 directives (explicit owner flag or ownership $\ge 25.0\%$).
    pub fn is_beneficial_owner(&self) -> bool {
        self.is_owner() || self.meets_fincen_cdd_ownership_threshold()
    }

    /// Returns `true` if the equity ownership meets or exceeds the FinCEN CDD 25% threshold.
    pub fn meets_fincen_cdd_ownership_threshold(&self) -> bool {
        self.percent_ownership.is_some_and(|pct| pct >= 25.0)
    }

    /// Returns `true` if the person falls under the FinCEN CDD control prong
    /// (representative, executive, or director).
    pub fn is_control_person(&self) -> bool {
        self.is_representative() || self.is_executive() || self.is_director()
    }
}

/// Verification status and details for a Person.
///
/// # Examples
///
/// ```
/// use some_stripe_types::person::PersonVerification;
///
/// let ver = PersonVerification {
///     status: "verified".into(),
///     ..Default::default()
/// };
/// assert!(ver.is_verified());
/// assert!(!ver.is_pending());
/// assert!(!ver.is_unverified());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PersonVerification {
    /// Verification status of the person (`unverified`, `pending`, `verified`).
    #[serde(default)]
    pub status: String,

    /// Human-readable explanation of verification failure, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,

    /// Machine-readable code explaining verification failure, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details_code: Option<String>,

    /// Primary identity document submitted for verification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<VerificationDocument>,

    /// Additional identity document submitted (e.g. proof of residential address).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_document: Option<VerificationDocument>,
}

impl PersonVerification {
    /// Returns `true` if the person is verified.
    pub fn is_verified(&self) -> bool {
        self.status == "verified"
    }

    /// Returns `true` if the person's verification is currently pending.
    pub fn is_pending(&self) -> bool {
        self.status == "pending"
    }

    /// Returns `true` if the person is unverified.
    pub fn is_unverified(&self) -> bool {
        self.status == "unverified" || self.status.is_empty()
    }
}

/// Details of an identity document submitted for verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VerificationDocument {
    /// The front-side file upload ID (`file_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub front: Option<String>,

    /// The back-side file upload ID (`file_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back: Option<String>,

    /// Description of the document failure if verification failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,

    /// Machine-readable failure code if document verification failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details_code: Option<String>,
}

/// Collection of documents uploaded for a Person (authorization, passport, visa).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PersonDocuments {
    /// Company authorization letter document files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company_authorization: Option<PersonDocumentFiles>,

    /// Passport document files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passport: Option<PersonDocumentFiles>,

    /// Visa document files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visa: Option<PersonDocumentFiles>,
}

/// Wrapper for a list of document file IDs (`file_...`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PersonDocumentFiles {
    /// List of file IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
}

/// Outstanding verification and KYC requirements for a Person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PersonRequirements {
    /// Fields currently due to keep the connected account enabled.
    #[serde(default)]
    pub currently_due: Vec<String>,

    /// Fields that will eventually be due.
    #[serde(default)]
    pub eventually_due: Vec<String>,

    /// Fields that are past due.
    #[serde(default)]
    pub past_due: Vec<String>,

    /// Fields that are currently undergoing verification by Stripe.
    #[serde(default)]
    pub pending_verification: Vec<String>,

    /// Reason why the person was disabled, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,

    /// Detailed error descriptions for collection issues.
    #[serde(default)]
    pub errors: Vec<RequirementError>,
}

/// Description of a specific requirement collection failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RequirementError {
    /// Machine-readable error code.
    pub code: String,

    /// Human-readable explanation.
    pub reason: String,

    /// The field name that failed collection.
    pub requirement: String,
}

/// Paginated list of Person objects (`object: "list"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonList {
    /// Object type, always `list`.
    #[serde(default = "default_list_object")]
    pub object: String,

    /// The list of person objects.
    pub data: Vec<Person>,

    /// Whether there are more items available.
    pub has_more: bool,

    /// The URL endpoint for this list.
    pub url: String,
}

fn default_list_object() -> String {
    "list".into()
}

/// KYC & Customer Due Diligence Compliance Errors (FinCEN CDD Rule 31 CFR § 1010.230 & EU DAC7, Gate G-Law).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KycComplianceError {
    /// Mandatory identification field missing for beneficial owner or control person.
    MissingField {
        /// Field name.
        field: &'static str,
        /// Reason for requirement.
        reason: &'static str,
    },
    /// Invalid ownership percentage (must be within 0.0 - 100.0).
    InvalidOwnershipPercentage {
        /// Provided percentage value as string.
        percentage: String,
        /// Error explanation.
        reason: &'static str,
    },
    /// Invalid date of birth values.
    InvalidDateOfBirth {
        /// Error explanation.
        reason: &'static str,
    },
    /// Government tax or national identification missing.
    MissingIdentification {
        /// Error explanation.
        reason: &'static str,
    },
}

impl std::fmt::Display for KycComplianceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField { field, reason } => {
                write!(f, "KYC compliance error: missing field '{field}': {reason}")
            }
            Self::InvalidOwnershipPercentage { percentage, reason } => {
                write!(
                    f,
                    "KYC compliance error: invalid ownership percentage '{percentage}': {reason}"
                )
            }
            Self::InvalidDateOfBirth { reason } => {
                write!(f, "KYC compliance error: invalid date of birth: {reason}")
            }
            Self::MissingIdentification { reason } => {
                write!(f, "KYC compliance error: missing identification: {reason}")
            }
        }
    }
}

impl std::error::Error for KycComplianceError {}

impl Person {
    /// Validates FinCEN CDD (Customer Due Diligence, 31 CFR § 1010.230) compliance for this person (Gate G-Law).
    ///
    /// If the person is identified as a beneficial owner (ownership $\ge 25\%$ or `owner = true`),
    /// or as a designated control person (`representative = true`, `executive = true`, or `director = true`):
    /// - Legal name (`first_name` and `last_name`) must be present and non-empty.
    /// - Date of birth (`dob`) must be present and valid.
    /// - Physical address (`address`) must be present with country and street address.
    /// - Identification information (`id_number_provided`, `ssn_last_4_provided`, or `id_number`) must be provided.
    ///
    /// # Examples
    ///
    /// ```
    /// use some_stripe_types::person::{Person, PersonRelationship, PersonDob};
    /// use some_stripe_types::customer::CustomerAddress;
    ///
    /// let mut person = Person::default();
    /// person.first_name = Some("Alice".into());
    /// person.last_name = Some("Smith".into());
    /// person.dob = Some(PersonDob::new(1980, 5, 12));
    /// person.address = Some(CustomerAddress {
    ///     country: Some("US".into()),
    ///     line1: Some("123 Market St".into()),
    ///     ..Default::default()
    /// });
    /// person.id_number_provided = Some(true);
    /// person.relationship = Some(PersonRelationship {
    ///     owner: Some(true),
    ///     percent_ownership: Some(35.0),
    ///     ..Default::default()
    /// });
    ///
    /// assert!(person.validate_fincen_cdd_compliance().is_ok());
    /// ```
    fn validate_legal_name(
        &self,
        first_name_reason: &'static str,
        last_name_reason: &'static str,
    ) -> Result<(), KycComplianceError> {
        if self.first_name.as_deref().unwrap_or("").trim().is_empty() {
            return Err(KycComplianceError::MissingField {
                field: "first_name",
                reason: first_name_reason,
            });
        }
        if self.last_name.as_deref().unwrap_or("").trim().is_empty() {
            return Err(KycComplianceError::MissingField {
                field: "last_name",
                reason: last_name_reason,
            });
        }
        Ok(())
    }

    fn validate_dob_present(
        &self,
        invalid_reason: &'static str,
        missing_reason: &'static str,
    ) -> Result<(), KycComplianceError> {
        match &self.dob {
            Some(dob) if dob.is_valid() => Ok(()),
            Some(_) => Err(KycComplianceError::InvalidDateOfBirth {
                reason: invalid_reason,
            }),
            None => Err(KycComplianceError::MissingField {
                field: "dob",
                reason: missing_reason,
            }),
        }
    }

    fn validate_address_fields(
        &self,
        missing_address_reason: &'static str,
        missing_country_reason: &'static str,
        missing_line1_reason: Option<&'static str>,
    ) -> Result<(), KycComplianceError> {
        match &self.address {
            Some(addr) => {
                if addr.country.as_deref().unwrap_or("").trim().is_empty() {
                    return Err(KycComplianceError::MissingField {
                        field: "address.country",
                        reason: missing_country_reason,
                    });
                }
                if let Some(line1_reason) = missing_line1_reason
                    && addr.line1.as_deref().unwrap_or("").trim().is_empty()
                {
                    return Err(KycComplianceError::MissingField {
                        field: "address.line1",
                        reason: line1_reason,
                    });
                }
                Ok(())
            }
            None => Err(KycComplianceError::MissingField {
                field: "address",
                reason: missing_address_reason,
            }),
        }
    }

    fn has_government_id(&self, allow_ssn_last_4: bool) -> bool {
        self.id_number_provided.unwrap_or(false)
            || self.id_number.is_some()
            || (allow_ssn_last_4
                && (self.ssn_last_4_provided.unwrap_or(false) || self.ssn_last_4.is_some()))
            || self
                .verification
                .as_ref()
                .is_some_and(|v| v.document.is_some())
    }

    pub fn validate_fincen_cdd_compliance(&self) -> Result<(), KycComplianceError> {
        let is_owner = self
            .relationship
            .as_ref()
            .is_some_and(|r| r.is_beneficial_owner());
        let is_control = self
            .relationship
            .as_ref()
            .is_some_and(|r| r.is_control_person());

        if !is_owner && !is_control {
            return Ok(());
        }

        // Validate Ownership Percentage bounds if set
        if let Some(rel) = &self.relationship
            && let Some(pct) = rel.percent_ownership
            && !(0.0..=100.0).contains(&pct)
        {
            return Err(KycComplianceError::InvalidOwnershipPercentage {
                percentage: pct.to_string(),
                reason: "Ownership percentage must be between 0.0 and 100.0%",
            });
        }

        self.validate_legal_name(
            "FinCEN CDD requires full legal first name for beneficial owners and control persons",
            "FinCEN CDD requires full legal last name for beneficial owners and control persons",
        )?;

        self.validate_dob_present(
            "FinCEN CDD requires a valid date of birth (day, month, year)",
            "FinCEN CDD requires date of birth for beneficial owners and control persons",
        )?;

        self.validate_address_fields(
            "FinCEN CDD requires physical residential address for beneficial owners and control persons",
            "FinCEN CDD requires country in residential address",
            Some("FinCEN CDD requires street address in residential address"),
        )?;

        if !self.has_government_id(true) {
            return Err(KycComplianceError::MissingIdentification {
                reason: "FinCEN CDD requires government identification number (SSN, passport, or national ID)",
            });
        }

        Ok(())
    }

    /// Validates EU DAC7 (Directive 2021/514) / AMLD5 compliance for reportable sellers (Gate G-Law).
    ///
    /// Under EU DAC7:
    /// - Digital platform operators must collect, verify, and report identifying information
    ///   for active sellers residing in EU member states.
    /// - Mandatory requirements for natural persons:
    ///   * First name and last name.
    ///   * Primary residential address.
    ///   * Date of birth.
    ///   * Tax Identification Number (TIN) or national ID number.
    ///
    /// # Examples
    ///
    /// ```
    /// use some_stripe_types::person::{Person, PersonDob};
    /// use some_stripe_types::customer::CustomerAddress;
    ///
    /// let mut person = Person::default();
    /// person.first_name = Some("Jean".into());
    /// person.last_name = Some("Dupont".into());
    /// person.dob = Some(PersonDob::new(1992, 11, 3));
    /// person.address = Some(CustomerAddress {
    ///     country: Some("FR".into()),
    ///     line1: Some("10 Rue de la Paix".into()),
    ///     city: Some("Paris".into()),
    ///     postal_code: Some("75001".into()),
    ///     ..Default::default()
    /// });
    /// person.id_number_provided = Some(true);
    ///
    /// assert!(person.validate_dac7_compliance().is_ok());
    /// ```
    pub fn validate_dac7_compliance(&self) -> Result<(), KycComplianceError> {
        self.validate_legal_name(
            "EU DAC7 requires first name for reportable natural persons",
            "EU DAC7 requires last name for reportable natural persons",
        )?;

        self.validate_dob_present(
            "EU DAC7 requires a valid date of birth (day, month, year)",
            "EU DAC7 requires date of birth for natural person sellers",
        )?;

        self.validate_address_fields(
            "EU DAC7 requires primary address for reportable sellers",
            "EU DAC7 requires country code in primary address",
            None,
        )?;

        if !self.has_government_id(false) {
            return Err(KycComplianceError::MissingIdentification {
                reason: "EU DAC7 requires Tax Identification Number (TIN) or national identification",
            });
        }

        Ok(())
    }
}

/// Request parameters for creating a new Person on a Connected Account (`POST /v1/accounts/:account/persons`).
///
/// Implements custom [`std::fmt::Debug`] to mask sensitive PII (`id_number`, `id_number_secondary`, `ssn_last_4`)
/// as `[REDACTED]` (Gate G-Law & Gate G2).
///
/// # Examples
///
/// ```
/// use some_stripe_types::person::{CreatePersonParams, PersonDob, PersonRelationship};
///
/// let params = CreatePersonParams::new()
///     .first_name("Alice")
///     .last_name("Wonderland")
///     .email("alice@example.com")
///     .id_number("123-45-6789")
///     .ssn_last_4("6789")
///     .dob(PersonDob::new(1988, 3, 14))
///     .relationship(PersonRelationship::new().owner(true).percent_ownership(30.0));
///
/// assert!(params.validate().is_ok());
///
/// // Ensure sensitive ID numbers are redacted in Debug logs
/// let debug_str = format!("{params:?}");
/// assert!(debug_str.contains("[REDACTED]"));
/// assert!(!debug_str.contains("123-45-6789"));
/// ```
#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CreatePersonParams {
    /// The person's legal first name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,

    /// The person's legal last name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,

    /// The person's maiden name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maiden_name: Option<String>,

    /// The person's primary email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// The person's primary phone number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,

    /// The person's date of birth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dob: Option<PersonDob>,

    /// The person's residential address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<CustomerAddress>,

    /// The person's registered address (if distinct from residential address).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registered_address: Option<CustomerAddress>,

    /// The person's relationship to the account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationship: Option<PersonRelationship>,

    /// Verification parameters for identity verification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<PersonVerificationParams>,

    /// Documents uploaded for this person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documents: Option<PersonDocuments>,

    /// The person's primary government identification number (SSN or national ID).
    /// Masked in Debug as `[REDACTED]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_number: Option<String>,

    /// The person's secondary government identification number.
    /// Masked in Debug as `[REDACTED]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_number_secondary: Option<String>,

    /// The last 4 digits of the person's Social Security number.
    /// Masked in Debug as `[REDACTED]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssn_last_4: Option<String>,

    /// The person's nationality (two-letter ISO country code).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nationality: Option<String>,

    /// The person's political exposure status (`existing`, `none`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub political_exposure: Option<String>,

    /// Key-value metadata attached to this person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Idempotency key for request deduplication (omitted from serialized request body).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl std::fmt::Debug for CreatePersonParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreatePersonParams")
            .field("first_name", &self.first_name)
            .field("last_name", &self.last_name)
            .field("maiden_name", &self.maiden_name)
            .field("email", &self.email)
            .field("phone", &self.phone)
            .field("dob", &self.dob)
            .field("address", &self.address)
            .field("registered_address", &self.registered_address)
            .field("relationship", &self.relationship)
            .field("verification", &self.verification)
            .field("documents", &self.documents)
            .field("id_number", &self.id_number.as_ref().map(|_| "[REDACTED]"))
            .field(
                "id_number_secondary",
                &self.id_number_secondary.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "ssn_last_4",
                &self.ssn_last_4.as_ref().map(|_| "[REDACTED]"),
            )
            .field("nationality", &self.nationality)
            .field("political_exposure", &self.political_exposure)
            .field("metadata", &self.metadata)
            .finish()
    }
}

impl CreatePersonParams {
    /// Creates a new, default [`CreatePersonParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the person's legal first name.
    pub fn first_name(mut self, name: impl Into<String>) -> Self {
        self.first_name = Some(name.into());
        self
    }

    /// Sets the person's legal last name.
    pub fn last_name(mut self, name: impl Into<String>) -> Self {
        self.last_name = Some(name.into());
        self
    }

    /// Sets the person's maiden name.
    pub fn maiden_name(mut self, name: impl Into<String>) -> Self {
        self.maiden_name = Some(name.into());
        self
    }

    /// Sets the person's email address.
    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Sets the person's phone number.
    pub fn phone(mut self, phone: impl Into<String>) -> Self {
        self.phone = Some(phone.into());
        self
    }

    /// Sets the person's date of birth.
    pub fn dob(mut self, dob: PersonDob) -> Self {
        self.dob = Some(dob);
        self
    }

    /// Sets the person's residential address.
    pub fn address(mut self, addr: CustomerAddress) -> Self {
        self.address = Some(addr);
        self
    }

    /// Sets the person's registered address.
    pub fn registered_address(mut self, addr: CustomerAddress) -> Self {
        self.registered_address = Some(addr);
        self
    }

    /// Sets the person's relationship to the account.
    pub fn relationship(mut self, rel: PersonRelationship) -> Self {
        self.relationship = Some(rel);
        self
    }

    /// Sets the person's verification parameters.
    pub fn verification(mut self, ver: PersonVerificationParams) -> Self {
        self.verification = Some(ver);
        self
    }

    /// Sets the person's uploaded documents.
    pub fn documents(mut self, docs: PersonDocuments) -> Self {
        self.documents = Some(docs);
        self
    }

    /// Sets the person's government identification number (SSN or national ID).
    pub fn id_number(mut self, id: impl Into<String>) -> Self {
        self.id_number = Some(id.into());
        self
    }

    /// Sets the person's secondary government identification number.
    pub fn id_number_secondary(mut self, id: impl Into<String>) -> Self {
        self.id_number_secondary = Some(id.into());
        self
    }

    /// Sets the last 4 digits of the person's SSN.
    pub fn ssn_last_4(mut self, ssn: impl Into<String>) -> Self {
        self.ssn_last_4 = Some(ssn.into());
        self
    }

    /// Sets the person's nationality.
    pub fn nationality(mut self, nat: impl Into<String>) -> Self {
        self.nationality = Some(nat.into());
        self
    }

    /// Sets the person's political exposure status.
    pub fn political_exposure(mut self, exp: impl Into<String>) -> Self {
        self.political_exposure = Some(exp.into());
        self
    }

    /// Attaches key-value metadata.
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets an idempotency key for this request.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates parameter bounds and constraints before submission.
    pub fn validate(&self) -> Result<(), KycComplianceError> {
        if let Some(rel) = &self.relationship
            && let Some(pct) = rel.percent_ownership
            && !(0.0..=100.0).contains(&pct)
        {
            return Err(KycComplianceError::InvalidOwnershipPercentage {
                percentage: pct.to_string(),
                reason: "Ownership percentage must be between 0.0 and 100.0%",
            });
        }
        if let Some(dob) = &self.dob {
            if let Some(m) = dob.month
                && !(1..=12).contains(&m)
            {
                return Err(KycComplianceError::InvalidDateOfBirth {
                    reason: "Month must be between 1 and 12",
                });
            }
            if let Some(d) = dob.day
                && !(1..=31).contains(&d)
            {
                return Err(KycComplianceError::InvalidDateOfBirth {
                    reason: "Day must be between 1 and 31",
                });
            }
            if let Some(y) = dob.year
                && !(1900..=2100).contains(&y)
            {
                return Err(KycComplianceError::InvalidDateOfBirth {
                    reason: "Year must be between 1900 and 2100",
                });
            }
        }
        Ok(())
    }
}

/// Request parameters for updating an existing Person on a Connected Account (`POST /v1/accounts/:account/persons/:person`).
///
/// Employs 3-state [`Nullable<T>`] fields for precise mutation semantics:
/// - [`Nullable::Value`]: Sets a new concrete value.
/// - [`Nullable::Null`]: Clears or unsets the remote field.
/// - [`Nullable::Omit`]: Skips the field during serialization.
///
/// Implements custom [`std::fmt::Debug`] to mask sensitive ID fields as `[REDACTED]` (Gate G-Law & Gate G2).
///
/// # Examples
///
/// ```
/// use some_stripe_types::person::UpdatePersonParams;
/// use some_stripe_types::Nullable;
///
/// // Update phone, clear maiden_name, omit email
/// let params = UpdatePersonParams::new()
///     .phone("+15551234567")
///     .clear_maiden_name()
///     .id_number("987-65-4321");
///
/// assert_eq!(params.phone, Nullable::Value("+15551234567".into()));
/// assert_eq!(params.maiden_name, Nullable::Null);
/// assert!(params.email.is_omit());
///
/// // Ensure sensitive ID numbers are redacted in Debug logs
/// let debug_str = format!("{params:?}");
/// assert!(debug_str.contains("[REDACTED]"));
/// assert!(!debug_str.contains("987-65-4321"));
/// ```
#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UpdatePersonParams {
    /// The person's legal first name.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub first_name: Nullable<String>,

    /// The person's legal last name.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub last_name: Nullable<String>,

    /// The person's maiden name.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub maiden_name: Nullable<String>,

    /// The person's primary email address.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub email: Nullable<String>,

    /// The person's primary phone number.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub phone: Nullable<String>,

    /// The person's date of birth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dob: Option<PersonDob>,

    /// The person's residential address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<CustomerAddress>,

    /// The person's registered address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registered_address: Option<CustomerAddress>,

    /// The person's relationship to the account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationship: Option<PersonRelationship>,

    /// Verification parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<PersonVerificationParams>,

    /// Uploaded documents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documents: Option<PersonDocuments>,

    /// Sensitive identification number. Masked in Debug as `[REDACTED]`.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub id_number: Nullable<String>,

    /// Sensitive secondary identification number. Masked in Debug as `[REDACTED]`.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub id_number_secondary: Nullable<String>,

    /// Sensitive SSN last 4 digits. Masked in Debug as `[REDACTED]`.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub ssn_last_4: Nullable<String>,

    /// The person's nationality (two-letter ISO code).
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub nationality: Nullable<String>,

    /// The person's political exposure status.
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    pub political_exposure: Nullable<String>,

    /// Key-value metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Idempotency key for request deduplication.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl std::fmt::Debug for UpdatePersonParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let redact_nullable = |n: &Nullable<String>| match n {
            Nullable::Value(_) => Nullable::Value("[REDACTED]"),
            Nullable::Null => Nullable::Null,
            Nullable::Omit => Nullable::Omit,
        };

        f.debug_struct("UpdatePersonParams")
            .field("first_name", &self.first_name)
            .field("last_name", &self.last_name)
            .field("maiden_name", &self.maiden_name)
            .field("email", &self.email)
            .field("phone", &self.phone)
            .field("dob", &self.dob)
            .field("address", &self.address)
            .field("registered_address", &self.registered_address)
            .field("relationship", &self.relationship)
            .field("verification", &self.verification)
            .field("documents", &self.documents)
            .field("id_number", &redact_nullable(&self.id_number))
            .field(
                "id_number_secondary",
                &redact_nullable(&self.id_number_secondary),
            )
            .field("ssn_last_4", &redact_nullable(&self.ssn_last_4))
            .field("nationality", &self.nationality)
            .field("political_exposure", &self.political_exposure)
            .field("metadata", &self.metadata)
            .finish()
    }
}

impl UpdatePersonParams {
    /// Creates a new, default [`UpdatePersonParams`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates first name.
    pub fn first_name(mut self, name: impl Into<String>) -> Self {
        self.first_name = Nullable::Value(name.into());
        self
    }

    /// Clears first name.
    pub fn clear_first_name(mut self) -> Self {
        self.first_name = Nullable::Null;
        self
    }

    /// Updates last name.
    pub fn last_name(mut self, name: impl Into<String>) -> Self {
        self.last_name = Nullable::Value(name.into());
        self
    }

    /// Clears last name.
    pub fn clear_last_name(mut self) -> Self {
        self.last_name = Nullable::Null;
        self
    }

    /// Updates maiden name.
    pub fn maiden_name(mut self, name: impl Into<String>) -> Self {
        self.maiden_name = Nullable::Value(name.into());
        self
    }

    /// Clears maiden name.
    pub fn clear_maiden_name(mut self) -> Self {
        self.maiden_name = Nullable::Null;
        self
    }

    /// Updates email address.
    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Nullable::Value(email.into());
        self
    }

    /// Clears email address.
    pub fn clear_email(mut self) -> Self {
        self.email = Nullable::Null;
        self
    }

    /// Updates phone number.
    pub fn phone(mut self, phone: impl Into<String>) -> Self {
        self.phone = Nullable::Value(phone.into());
        self
    }

    /// Clears phone number.
    pub fn clear_phone(mut self) -> Self {
        self.phone = Nullable::Null;
        self
    }

    /// Updates date of birth.
    pub fn dob(mut self, dob: PersonDob) -> Self {
        self.dob = Some(dob);
        self
    }

    /// Updates residential address.
    pub fn address(mut self, addr: CustomerAddress) -> Self {
        self.address = Some(addr);
        self
    }

    /// Updates registered address.
    pub fn registered_address(mut self, addr: CustomerAddress) -> Self {
        self.registered_address = Some(addr);
        self
    }

    /// Updates relationship.
    pub fn relationship(mut self, rel: PersonRelationship) -> Self {
        self.relationship = Some(rel);
        self
    }

    /// Updates verification parameters.
    pub fn verification(mut self, ver: PersonVerificationParams) -> Self {
        self.verification = Some(ver);
        self
    }

    /// Updates uploaded documents.
    pub fn documents(mut self, docs: PersonDocuments) -> Self {
        self.documents = Some(docs);
        self
    }

    /// Updates sensitive government identification number.
    pub fn id_number(mut self, id: impl Into<String>) -> Self {
        self.id_number = Nullable::Value(id.into());
        self
    }

    /// Clears government identification number.
    pub fn clear_id_number(mut self) -> Self {
        self.id_number = Nullable::Null;
        self
    }

    /// Updates secondary identification number.
    pub fn id_number_secondary(mut self, id: impl Into<String>) -> Self {
        self.id_number_secondary = Nullable::Value(id.into());
        self
    }

    /// Clears secondary identification number.
    pub fn clear_id_number_secondary(mut self) -> Self {
        self.id_number_secondary = Nullable::Null;
        self
    }

    /// Updates SSN last 4 digits.
    pub fn ssn_last_4(mut self, ssn: impl Into<String>) -> Self {
        self.ssn_last_4 = Nullable::Value(ssn.into());
        self
    }

    /// Clears SSN last 4 digits.
    pub fn clear_ssn_last_4(mut self) -> Self {
        self.ssn_last_4 = Nullable::Null;
        self
    }

    /// Updates nationality.
    pub fn nationality(mut self, nat: impl Into<String>) -> Self {
        self.nationality = Nullable::Value(nat.into());
        self
    }

    /// Clears nationality.
    pub fn clear_nationality(mut self) -> Self {
        self.nationality = Nullable::Null;
        self
    }

    /// Updates political exposure status.
    pub fn political_exposure(mut self, exp: impl Into<String>) -> Self {
        self.political_exposure = Nullable::Value(exp.into());
        self
    }

    /// Clears political exposure status.
    pub fn clear_political_exposure(mut self) -> Self {
        self.political_exposure = Nullable::Null;
        self
    }

    /// Attaches key-value metadata.
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets an idempotency key for this request.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates parameter bounds and constraints before submission.
    pub fn validate(&self) -> Result<(), KycComplianceError> {
        if let Some(rel) = &self.relationship
            && let Some(pct) = rel.percent_ownership
            && !(0.0..=100.0).contains(&pct)
        {
            return Err(KycComplianceError::InvalidOwnershipPercentage {
                percentage: pct.to_string(),
                reason: "Ownership percentage must be between 0.0 and 100.0%",
            });
        }
        if let Some(dob) = &self.dob {
            if let Some(m) = dob.month
                && !(1..=12).contains(&m)
            {
                return Err(KycComplianceError::InvalidDateOfBirth {
                    reason: "Month must be between 1 and 12",
                });
            }
            if let Some(d) = dob.day
                && !(1..=31).contains(&d)
            {
                return Err(KycComplianceError::InvalidDateOfBirth {
                    reason: "Day must be between 1 and 31",
                });
            }
            if let Some(y) = dob.year
                && !(1900..=2100).contains(&y)
            {
                return Err(KycComplianceError::InvalidDateOfBirth {
                    reason: "Year must be between 1900 and 2100",
                });
            }
        }
        Ok(())
    }
}

/// Request parameters for document verification on a Person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PersonVerificationParams {
    /// Primary identity document parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<VerificationDocumentParams>,

    /// Additional identity document parameters (e.g. proof of residential address).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_document: Option<VerificationDocumentParams>,
}

/// File IDs for an uploaded verification document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VerificationDocumentParams {
    /// Front side file ID (`file_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub front: Option<String>,

    /// Back side file ID (`file_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back: Option<String>,
}

/// Paginated list parameter block for listing Persons (`GET /v1/accounts/{account}/persons`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListPersonsParams {
    /// A limit on the number of objects to be returned, between 1 and 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// A cursor for use in pagination. An object ID defining your place in the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// A cursor for use in pagination. An object ID defining your place in the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListPersonsParams {
    /// Creates a new default parameter block for listing persons.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets limit.
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
}

/// Confirmation object returned upon person deletion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletedPerson {
    /// Unique identifier of deleted person (`person_...`).
    pub id: String,

    /// Object type, always `person`.
    #[serde(default = "default_person_object")]
    pub object: String,

    /// Always `true` if deleted.
    pub deleted: bool,
}

/// Function for paginator ID extraction.
pub fn person_id(person: &Person) -> &str {
    person.id.as_str()
}
