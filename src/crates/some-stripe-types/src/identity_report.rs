//! # Stripe Identity Verification Report Types
//!
//! Strongly typed domain models, report options, and list parameters for
//! Stripe Identity Verification Reports (`identity.verification_report`).
//!
//! # Architecture & Quality Invariants:
//! - **Gate G-Borrow**: Zero deep `.clone()` calls on domain entities.
//! - **Gate G-Docs**: Comprehensive rustdoc documentation and runnable doc-tests.
//! - **Gate G-Complexity**: Cyclomatic complexity <= 6 across all methods.
//! - **Gate G-Law & G2**: Sensitive PII minimization and GDPR Article 17 alignment.
//!
//! # Example
//! ```
//! use some_stripe_types::{VerificationReport, VerificationReportStatus, VerificationReportType};
//!
//! let report = VerificationReport {
//!     id: "vr_123456789".to_string(),
//!     object: "identity.verification_report".to_string(),
//!     created: 1728345600,
//!     livemode: false,
//!     report_type: VerificationReportType::Document,
//!     status: VerificationReportStatus::Verified,
//!     verification_session: Some("vs_test_session_123".to_string()),
//!     client_reference_id: Some("user_ref_001".to_string()),
//!     document: None,
//!     id_number: None,
//!     selfie: None,
//!     options: None,
//! };
//!
//! assert_eq!(report.id(), "vr_123456789");
//! assert!(report.is_verified());
//! ```

use crate::customer::CustomerAddress;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Discriminator type for a verification report.
///
/// Indicates whether the report verified an identity document or an ID number.
///
/// # Example
/// ```
/// use some_stripe_types::VerificationReportType;
/// use std::str::FromStr;
///
/// let report_type = VerificationReportType::Document;
/// assert_eq!(report_type.as_str(), "document");
/// assert_eq!(report_type.to_string(), "document");
///
/// let parsed = VerificationReportType::from_str("id_number").unwrap();
/// assert_eq!(parsed, VerificationReportType::IdNumber);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VerificationReportType {
    /// Document-based identity check (passport, driving license, ID card).
    #[default]
    Document,
    /// ID number lookup verification.
    IdNumber,
}

impl VerificationReportType {
    /// Returns the static string slice representation of this report type.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::IdNumber => "id_number",
        }
    }
}

impl fmt::Display for VerificationReportType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for VerificationReportType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for VerificationReportType {
    type Err = ParseReportTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "document" => Ok(Self::Document),
            "id_number" => Ok(Self::IdNumber),
            _ => Err(ParseReportTypeError(s.to_string())),
        }
    }
}

/// Error returned when parsing an unrecognized [`VerificationReportType`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseReportTypeError(pub String);

impl fmt::Display for ParseReportTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown verification report type: '{}'", self.0)
    }
}

impl std::error::Error for ParseReportTypeError {}

/// Status of an identity verification report.
///
/// # Example
/// ```
/// use some_stripe_types::VerificationReportStatus;
/// use std::str::FromStr;
///
/// let status = VerificationReportStatus::Verified;
/// assert_eq!(status.as_str(), "verified");
/// assert_eq!(status.to_string(), "verified");
///
/// let unverified = VerificationReportStatus::from_str("unverified").unwrap();
/// assert_eq!(unverified, VerificationReportStatus::Unverified);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VerificationReportStatus {
    /// The report verification succeeded.
    Verified,
    /// The report verification is pending or failed.
    #[default]
    Unverified,
}

impl VerificationReportStatus {
    /// Returns the static string slice representation of this status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Unverified => "unverified",
        }
    }
}

impl fmt::Display for VerificationReportStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for VerificationReportStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for VerificationReportStatus {
    type Err = ParseReportStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "verified" => Ok(Self::Verified),
            "unverified" => Ok(Self::Unverified),
            _ => Err(ParseReportStatusError(s.to_string())),
        }
    }
}

/// Error returned when parsing an unrecognized [`VerificationReportStatus`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseReportStatusError(pub String);

impl fmt::Display for ParseReportStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown verification report status: '{}'", self.0)
    }
}

impl std::error::Error for ParseReportStatusError {}

/// Date representation for document issuance, expiration, or date of birth.
///
/// # Example
/// ```
/// use some_stripe_types::DocumentDate;
///
/// let date = DocumentDate::new(1990, 5, 15);
/// assert_eq!(date.year, Some(1990));
/// assert_eq!(date.month, Some(5));
/// assert_eq!(date.day, Some(15));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct DocumentDate {
    /// The day of the month (1-31).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day: Option<u8>,

    /// The month (1-12).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month: Option<u8>,

    /// The four-digit year.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
}

impl DocumentDate {
    /// Constructs a new [`DocumentDate`] with the specified year, month, and day.
    pub const fn new(year: i32, month: u8, day: u8) -> Self {
        Self {
            year: Some(year),
            month: Some(month),
            day: Some(day),
        }
    }
}

/// Error details for an unverified document check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DocumentReportError {
    /// A short machine-readable string describing the verification failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// A human-readable message explaining the reason for the failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Document-specific verification results.
///
/// # Example
/// ```
/// use some_stripe_types::{DocumentDate, DocumentReport, VerificationReportStatus};
///
/// let doc_report = DocumentReport {
///     status: VerificationReportStatus::Verified,
///     error: None,
///     expiration_date: Some(DocumentDate::new(2030, 1, 1)),
///     files: Some(vec!["file_front_123".to_string()]),
///     first_name: Some("Jane".to_string()),
///     last_name: Some("Doe".to_string()),
///     dob: Some(DocumentDate::new(1988, 12, 10)),
///     issued_date: Some(DocumentDate::new(2020, 1, 1)),
///     issuing_country: Some("US".to_string()),
///     document_type: Some("driving_license".to_string()),
///     number: Some("DL12345678".to_string()),
///     address: None,
/// };
///
/// assert_eq!(doc_report.status, VerificationReportStatus::Verified);
/// assert_eq!(doc_report.first_name.as_deref(), Some("Jane"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DocumentReport {
    /// Status of the document verification check.
    pub status: VerificationReportStatus,

    /// Details on why document verification failed, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<DocumentReportError>,

    /// Expiration date of the document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<DocumentDate>,

    /// Array of File IDs of submitted document images.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,

    /// First name extracted from the document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,

    /// Last name extracted from the document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,

    /// Date of birth extracted from the document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dob: Option<DocumentDate>,

    /// Issued date of the document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issued_date: Option<DocumentDate>,

    /// Two-letter ISO country code of the issuing country.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuing_country: Option<String>,

    /// Type of the document (e.g. `passport`, `driving_license`, `id_card`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_type: Option<String>,

    /// Document number extracted from the document (may be partially redacted).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,

    /// Address extracted from the document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<CustomerAddress>,
}

/// Error details for an unverified selfie check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SelfieReportError {
    /// A short machine-readable string describing the verification failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// A human-readable message explaining the reason for the failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Selfie-specific biometric verification results.
///
/// # Example
/// ```
/// use some_stripe_types::{SelfieReport, VerificationReportStatus};
///
/// let selfie = SelfieReport {
///     status: VerificationReportStatus::Verified,
///     error: None,
///     selfie: Some("file_selfie_987".to_string()),
///     document: Some("file_face_crop_987".to_string()),
/// };
/// assert_eq!(selfie.status, VerificationReportStatus::Verified);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SelfieReport {
    /// Status of the selfie biometric match check.
    pub status: VerificationReportStatus,

    /// Details on why selfie verification failed, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<SelfieReportError>,

    /// ID of the File containing the selfie image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selfie: Option<String>,

    /// ID of the File containing the cropped face from the document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,
}

/// Error details for an unverified ID number check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IdNumberReportError {
    /// A short machine-readable string describing the verification failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// A human-readable message explaining the reason for the failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// ID number verification check results.
///
/// # Example
/// ```
/// use some_stripe_types::{DocumentDate, IdNumberReport, VerificationReportStatus};
///
/// let id_report = IdNumberReport {
///     status: VerificationReportStatus::Verified,
///     error: None,
///     dob: Some(DocumentDate::new(1985, 4, 20)),
///     first_name: Some("John".to_string()),
///     last_name: Some("Smith".to_string()),
///     id_number: Some("*********1234".to_string()),
///     id_number_type: Some("us_ssn".to_string()),
/// };
/// assert_eq!(id_report.status, VerificationReportStatus::Verified);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IdNumberReport {
    /// Status of the ID number verification check.
    pub status: VerificationReportStatus,

    /// Details on why ID number verification failed, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<IdNumberReportError>,

    /// Date of birth associated with the ID number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dob: Option<DocumentDate>,

    /// First name associated with the ID number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,

    /// Last name associated with the ID number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,

    /// The verified ID number (may be masked for security).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_number: Option<String>,

    /// The type of ID number verified (e.g. `us_ssn`, `br_cpf`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_number_type: Option<String>,
}

/// Options configured when generating the verification report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VerificationReportOptions {
    /// Options specific to document checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<DocumentReportOptions>,

    /// Options specific to ID number checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_number: Option<IdNumberReportOptions>,
}

/// Document verification options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DocumentReportOptions {
    /// Array of allowed document types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_types: Option<Vec<String>>,

    /// Whether a matching selfie was required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_matching_selfie: Option<bool>,
}

/// ID number verification options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IdNumberReportOptions {}

/// A Stripe Identity Verification Report.
///
/// Verification reports are created when a verification session completes,
/// providing cryptographic and biometric verification evidence.
///
/// # Example
/// ```
/// use some_stripe_types::{VerificationReport, VerificationReportStatus, VerificationReportType};
///
/// let json = r#"{
///     "id": "vr_test_001",
///     "object": "identity.verification_report",
///     "created": 1728345600,
///     "livemode": false,
///     "type": "document",
///     "status": "verified",
///     "verification_session": "vs_test_001"
/// }"#;
///
/// let report: VerificationReport = serde_json::from_str(json).unwrap();
/// assert_eq!(report.id(), "vr_test_001");
/// assert_eq!(report.report_type, VerificationReportType::Document);
/// assert_eq!(report.status, VerificationReportStatus::Verified);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Unique identifier for the verification report (`vr_...`).
    pub id: String,

    /// String representing the object's type, always `"identity.verification_report"`.
    #[serde(default = "default_verification_report_object")]
    pub object: String,

    /// Time at which the object was created (Unix timestamp in seconds).
    pub created: i64,

    /// Has the value `true` if the object exists in live mode or `false` for test mode.
    pub livemode: bool,

    /// Type of verification report: `document` or `id_number`.
    #[serde(rename = "type")]
    pub report_type: VerificationReportType,

    /// Status of the verification report.
    pub status: VerificationReportStatus,

    /// ID of the VerificationSession that created this report (`vs_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_session: Option<String>,

    /// A string to reference this user verification attempt across internal systems.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_reference_id: Option<String>,

    /// Results of the document check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<DocumentReport>,

    /// Results of the ID number check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_number: Option<IdNumberReport>,

    /// Results of the selfie check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selfie: Option<SelfieReport>,

    /// Options used when creating the verification report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<VerificationReportOptions>,
}

fn default_verification_report_object() -> String {
    "identity.verification_report".to_string()
}

impl VerificationReport {
    /// Returns the unique identifier of this verification report.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns `true` if this report achieved successful verification status.
    #[must_use]
    pub fn is_verified(&self) -> bool {
        self.status == VerificationReportStatus::Verified
    }
}

/// Key extractor function for [`VerificationReport`] in paginated streams.
///
/// # Example
/// ```
/// use some_stripe_types::{VerificationReport, VerificationReportStatus, VerificationReportType, verification_report_id};
///
/// let report = VerificationReport {
///     id: "vr_stream_123".to_string(),
///     object: "identity.verification_report".to_string(),
///     created: 1728345600,
///     livemode: false,
///     report_type: VerificationReportType::Document,
///     status: VerificationReportStatus::Verified,
///     verification_session: None,
///     client_reference_id: None,
///     document: None,
///     id_number: None,
///     selfie: None,
///     options: None,
/// };
///
/// assert_eq!(verification_report_id(&report), "vr_stream_123");
/// ```
#[must_use]
pub fn verification_report_id(report: &VerificationReport) -> &str {
    &report.id
}

/// Parameters for listing Stripe Identity Verification Reports (`GET /v1/identity/verification_reports`).
///
/// # Example
/// ```
/// use some_stripe_types::{ListVerificationReportsParams, VerificationReportStatus, VerificationReportType};
///
/// let params = ListVerificationReportsParams::new()
///     .verification_session("vs_test_123")
///     .status(VerificationReportStatus::Verified)
///     .report_type(VerificationReportType::Document)
///     .limit(20);
///
/// assert_eq!(params.verification_session.as_deref(), Some("vs_test_123"));
/// assert_eq!(params.status, Some(VerificationReportStatus::Verified));
/// assert_eq!(params.limit, Some(20));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListVerificationReportsParams {
    /// Filter reports created by a specific verification session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_session: Option<String>,

    /// Filter reports by client reference ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_reference_id: Option<String>,

    /// Filter reports by status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<VerificationReportStatus>,

    /// Filter reports by type (`document` or `id_number`).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub report_type: Option<VerificationReportType>,

    /// Filter reports created on or around a specific Unix timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,

    /// Cursor for backward pagination: an object ID defining your place in the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,

    /// Cursor for forward pagination: an object ID defining your place in the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// A limit on the number of objects to be returned (1 to 100). Default is 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl ListVerificationReportsParams {
    /// Creates empty, default [`ListVerificationReportsParams`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by verification session ID.
    #[must_use]
    pub fn verification_session(mut self, vs: impl Into<String>) -> Self {
        self.verification_session = Some(vs.into());
        self
    }

    /// Filter by client reference ID.
    #[must_use]
    pub fn client_reference_id(mut self, crid: impl Into<String>) -> Self {
        self.client_reference_id = Some(crid.into());
        self
    }

    /// Filter by report status.
    #[must_use]
    pub fn status(mut self, status: VerificationReportStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Filter by report type (`document` or `id_number`).
    #[must_use]
    pub fn report_type(mut self, report_type: VerificationReportType) -> Self {
        self.report_type = Some(report_type);
        self
    }

    /// Filter by creation timestamp.
    #[must_use]
    pub fn created(mut self, created: i64) -> Self {
        self.created = Some(created);
        self
    }

    /// Sets pagination cursor for items immediately following this ID.
    #[must_use]
    pub fn starting_after(mut self, cursor: impl Into<String>) -> Self {
        self.starting_after = Some(cursor.into());
        self
    }

    /// Sets pagination cursor for items immediately preceding this ID.
    #[must_use]
    pub fn ending_before(mut self, cursor: impl Into<String>) -> Self {
        self.ending_before = Some(cursor.into());
        self
    }

    /// Sets the maximum number of items to return (clamped between 1 and 100).
    #[must_use]
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit.clamp(1, 100));
        self
    }
}
