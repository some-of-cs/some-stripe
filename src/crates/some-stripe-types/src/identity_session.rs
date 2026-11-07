//! # Stripe Identity Verification Session Types & Biometric Models
//!
//! Strongly typed domain models, session options, and parameter schemas for
//! Stripe Identity Verification Sessions (`/v1/identity/verification_sessions`).
//!
//! A VerificationSession represents an individual flow for collecting and verifying
//! identity evidence from a user, such as government-issued photo IDs and biometric
//! matching selfies.
//!
//! ## Biometric Privacy Invariant (Gate G2 & Gate G-Law)
//! Under GDPR Article 5(1)(c) (Data Minimization), GDPR Article 9 (Special Category Data),
//! and GDPR Article 17 (Right to Erasure):
//! - DTO models **MUST NOT** include raw images, raw camera streams, or biometric facial
//!   feature vector blobs.
//! - Stripe servers perform secure encrypted processing; this SDK exposes only verification
//!   metadata, session tokens, secure redirect URLs, and verified outputs.
//! - `client_secret` is masked as `[REDACTED]` in custom [`fmt::Debug`] implementations.
//! - Redaction endpoints (`POST /v1/identity/verification_sessions/{id}/redact`) allow
//!   irreversible expunging of collected biometric and personal data.
//!
//! ## Gate G-Borrow Zero-Clone Discipline
//! - Read operations provide zero-copy reference borrowing (`&str`, `&VerificationOptions`).
//! - No deep clones on entities or models.
//!
//! ## Working Rustdoc Example
//!
//! ```
//! use some_stripe_types::identity_session::{
//!     CreateVerificationSessionParams, DocumentOptions, VerificationOptions,
//!     VerificationSessionStatus, VerificationSessionType,
//! };
//!
//! let params = CreateVerificationSessionParams::document()
//!     .with_options(
//!         VerificationOptions::new().with_document(
//!             DocumentOptions::new().with_matching_selfie(true),
//!         ),
//!     )
//!     .with_return_url("https://platform.com/kyc/complete");
//!
//! assert_eq!(params.r#type, VerificationSessionType::Document);
//! assert_eq!(params.return_url.as_deref(), Some("https://platform.com/kyc/complete"));
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// Default object identifier for a Verification Session (`"identity.verification_session"`).
#[must_use]
pub fn default_verification_session_object() -> String {
    "identity.verification_session".into()
}

/// The status of a [`VerificationSession`].
///
/// Indicates the current stage in the identity verification lifecycle.
///
/// # Example
/// ```
/// use some_stripe_types::identity_session::VerificationSessionStatus;
/// use std::str::FromStr;
///
/// let status = VerificationSessionStatus::RequiresInput;
/// assert_eq!(status.as_str(), "requires_input");
/// assert_eq!(VerificationSessionStatus::from_str("verified").unwrap(), VerificationSessionStatus::Verified);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationSessionStatus {
    /// The session requires input from the user (document upload, selfie capture, etc.).
    RequiresInput,
    /// The verification session is actively being processed and analyzed.
    Processing,
    /// The identity verification succeeded and verified outputs are available.
    Verified,
    /// The verification session was canceled before completion.
    Canceled,
}

impl VerificationSessionStatus {
    /// Returns the static string representation of this verification session status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::RequiresInput => "requires_input",
            Self::Processing => "processing",
            Self::Verified => "verified",
            Self::Canceled => "canceled",
        }
    }
}

impl fmt::Display for VerificationSessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for VerificationSessionStatus {
    type Err = VerificationSessionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "requires_input" => Ok(Self::RequiresInput),
            "processing" => Ok(Self::Processing),
            "verified" => Ok(Self::Verified),
            "canceled" => Ok(Self::Canceled),
            other => Err(VerificationSessionParseError::InvalidStatus(
                other.to_string(),
            )),
        }
    }
}

/// The type of verification performed by a [`VerificationSession`].
///
/// # Example
/// ```
/// use some_stripe_types::identity_session::VerificationSessionType;
/// use std::str::FromStr;
///
/// let vtype = VerificationSessionType::Document;
/// assert_eq!(vtype.as_str(), "document");
/// assert_eq!(VerificationSessionType::from_str("id_number").unwrap(), VerificationSessionType::IdNumber);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationSessionType {
    /// Document-based verification (checks government ID, optional biometric selfie).
    Document,
    /// ID number verification (checks national identification number against databases).
    IdNumber,
}

impl VerificationSessionType {
    /// Returns the static string representation of this verification session type.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::IdNumber => "id_number",
        }
    }
}

impl fmt::Display for VerificationSessionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for VerificationSessionType {
    type Err = VerificationSessionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "document" => Ok(Self::Document),
            "id_number" => Ok(Self::IdNumber),
            other => Err(VerificationSessionParseError::InvalidType(
                other.to_string(),
            )),
        }
    }
}

/// Options for document-based identity verification.
///
/// Configures requirements for government-issued photo identity documents and
/// biometric selfie matching.
///
/// # Example
/// ```
/// use some_stripe_types::identity_session::DocumentOptions;
///
/// let opts = DocumentOptions::new()
///     .with_matching_selfie(true)
///     .with_live_capture(true)
///     .with_id_number(true);
///
/// assert_eq!(opts.require_matching_selfie, Some(true));
/// assert_eq!(opts.require_live_capture, Some(true));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentOptions {
    /// Array of allowed document types (e.g. `["driving_license", "passport", "id_card"]`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_types: Option<Vec<String>>,

    /// Collect an ID number and cross-verify with document data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_id_number: Option<bool>,

    /// Require live camera capture to prevent spoofing with pre-recorded media.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_live_capture: Option<bool>,

    /// Capture a selfie and perform biometric facial comparison against the photo ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_matching_selfie: Option<bool>,
}

impl DocumentOptions {
    /// Creates empty document verification options.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            allowed_types: None,
            require_id_number: None,
            require_live_capture: None,
            require_matching_selfie: None,
        }
    }

    /// Configures whether biometric selfie comparison is required.
    #[must_use]
    pub fn with_matching_selfie(mut self, require: bool) -> Self {
        self.require_matching_selfie = Some(require);
        self
    }

    /// Configures whether live video/photo capture is required.
    #[must_use]
    pub fn with_live_capture(mut self, require: bool) -> Self {
        self.require_live_capture = Some(require);
        self
    }

    /// Configures whether an ID number must be collected.
    #[must_use]
    pub fn with_id_number(mut self, require: bool) -> Self {
        self.require_id_number = Some(require);
        self
    }

    /// Configures allowed document types.
    #[must_use]
    pub fn with_allowed_types(mut self, allowed: Vec<String>) -> Self {
        self.allowed_types = Some(allowed);
        self
    }
}

/// Options for ID number verification.
///
/// # Example
/// ```
/// use some_stripe_types::identity_session::IdNumberOptions;
///
/// let opts = IdNumberOptions::new();
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdNumberOptions {}

impl IdNumberOptions {
    /// Creates a new empty `IdNumberOptions`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

/// Options configured for a [`VerificationSession`].
///
/// # Example
/// ```
/// use some_stripe_types::identity_session::{DocumentOptions, VerificationOptions};
///
/// let options = VerificationOptions::new()
///     .with_document(DocumentOptions::new().with_matching_selfie(true));
///
/// assert!(options.document.is_some());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationOptions {
    /// Configuration options for document-based verification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<DocumentOptions>,

    /// Configuration options for ID number verification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_number: Option<IdNumberOptions>,
}

impl VerificationOptions {
    /// Creates empty verification options.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            document: None,
            id_number: None,
        }
    }

    /// Creates verification options configured for document checks.
    #[must_use]
    pub fn for_document(document: DocumentOptions) -> Self {
        Self {
            document: Some(document),
            id_number: None,
        }
    }

    /// Creates verification options configured for ID number checks.
    #[must_use]
    pub fn for_id_number(id_number: IdNumberOptions) -> Self {
        Self {
            document: None,
            id_number: Some(id_number),
        }
    }

    /// Attaches document verification options.
    #[must_use]
    pub fn with_document(mut self, document: DocumentOptions) -> Self {
        self.document = Some(document);
        self
    }

    /// Attaches ID number verification options.
    #[must_use]
    pub fn with_id_number(mut self, id_number: IdNumberOptions) -> Self {
        self.id_number = Some(id_number);
        self
    }
}

/// Redaction information for a [`VerificationSession`] under GDPR Article 17.
///
/// When a session is redacted, personal data and biometric records are permanently
/// expunged from Stripe servers.
///
/// # Example
/// ```
/// use some_stripe_types::identity_session::VerificationSessionRedaction;
///
/// let redaction = VerificationSessionRedaction {
///     status: Some("redacted".into()),
/// };
/// assert!(redaction.is_redacted());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationSessionRedaction {
    /// Indicates whether the session's data has been redacted (`"redacted"` or `"processing"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

impl VerificationSessionRedaction {
    /// Returns `true` if the session's personal data has been permanently redacted.
    #[must_use]
    pub fn is_redacted(&self) -> bool {
        self.status.as_deref() == Some("redacted")
    }

    /// Returns `true` if redaction is currently being processed.
    #[must_use]
    pub fn is_processing(&self) -> bool {
        self.status.as_deref() == Some("processing")
    }
}

/// Last error encountered during verification execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationSessionLastError {
    /// Machine-readable error code explaining why verification failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// Human-readable explanation of why verification failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Verified date of birth extracted from identity documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationSessionDob {
    /// Day of the month.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day: Option<u8>,

    /// Month of the year (1-12).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month: Option<u8>,

    /// Four-digit year.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
}

/// Verified residential address extracted from identity documents.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationSessionAddress {
    /// City, district, or town.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,

    /// Two-letter country code (ISO 3166-1 alpha-2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Address line 1 (e.g. street, PO Box).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line1: Option<String>,

    /// Address line 2 (e.g. apartment, suite).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line2: Option<String>,

    /// ZIP or postal code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,

    /// State, county, province, or region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

/// Verified identity outputs extracted from user-submitted documents.
///
/// Note: Contains only verified textual metadata. Under Gate G2 and GDPR Article 5(1)(c),
/// raw document images, photos, and biometric templates are strictly excluded.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationSessionVerifiedOutputs {
    /// Verified residential address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<VerificationSessionAddress>,

    /// Verified date of birth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dob: Option<VerificationSessionDob>,

    /// Verified legal first name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,

    /// Verified legal last name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,

    /// Verified national identity number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_number: Option<String>,

    /// Type of national identity number (e.g. `us_ssn`, `br_cpf`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_number_type: Option<String>,
}

/// User details provided by the caller to pre-populate or match during verification.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationSessionProvidedDetails {
    /// Pre-populated email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Pre-populated phone number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
}

/// Represents a Stripe Identity Verification Session (`identity.verification_session`).
///
/// Implements custom `Debug` to mask `client_secret` as `[REDACTED]` (PCI-DSS 4.0 / Gate G2).
///
/// # Example
/// ```
/// use some_stripe_types::identity_session::{VerificationSession, VerificationSessionStatus};
///
/// let json = r#"{
///   "id": "vs_1234567890",
///   "object": "identity.verification_session",
///   "client_secret": "vcs_secret_token_123",
///   "created": 1700000000,
///   "livemode": false,
///   "status": "requires_input",
///   "type": "document",
///   "url": "https://verify.stripe.com/start/vs_1234567890"
/// }"#;
///
/// let session: VerificationSession = serde_json::from_str(json).unwrap();
/// assert_eq!(session.id(), "vs_1234567890");
/// assert_eq!(session.status(), VerificationSessionStatus::RequiresInput);
/// assert_eq!(session.client_secret(), Some("vcs_secret_token_123"));
/// assert_eq!(session.url(), Some("https://verify.stripe.com/start/vs_1234567890"));
///
/// // Verify client_secret redaction in Debug (Gate G2)
/// let debug_str = format!("{session:?}");
/// assert!(!debug_str.contains("vcs_secret_token_123"));
/// assert!(debug_str.contains("[REDACTED]"));
/// ```
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationSession {
    /// Unique identifier for the object (e.g. `vs_123456`).
    pub id: String,

    /// String representing the object's type. Always `identity.verification_session`.
    #[serde(default = "default_verification_session_object")]
    pub object: String,

    /// Short-lived client secret used by the client SDK to authenticate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,

    /// Time at which the object was created (Unix timestamp in seconds).
    pub created: i64,

    /// Time at which the verification session will expire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,

    /// Has the value `true` if the object exists in live mode or `false` if test mode.
    pub livemode: bool,

    /// Status of this verification session.
    pub status: VerificationSessionStatus,

    /// The type of verification performed.
    #[serde(rename = "type")]
    pub r#type: VerificationSessionType,

    /// The short-lived URL that you redirect your customer to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// The URL to which the customer is redirected after verification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_url: Option<String>,

    /// If present, details about the error that caused the verification to fail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<VerificationSessionLastError>,

    /// ID of the most recent `VerificationReport` generated for this session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_verification_report: Option<String>,

    /// Set of key-value pairs attached to the object.
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    /// Configuration options for this session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<VerificationOptions>,

    /// Redaction status of this session under GDPR Article 17.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redaction: Option<VerificationSessionRedaction>,

    /// Verified outputs extracted from user-submitted evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_outputs: Option<VerificationSessionVerifiedOutputs>,
}

impl VerificationSession {
    /// Returns a borrowed reference to the verification session identifier (Gate G-Borrow).
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns a borrowed reference to the client secret, if present (Gate G-Borrow).
    #[must_use]
    pub fn client_secret(&self) -> Option<&str> {
        self.client_secret.as_deref()
    }

    /// Returns the session status.
    #[must_use]
    pub const fn status(&self) -> VerificationSessionStatus {
        self.status
    }

    /// Returns the verification session type.
    #[must_use]
    pub const fn verification_type(&self) -> VerificationSessionType {
        self.r#type
    }

    /// Returns the redirect URL for customer onboarding, if set.
    #[must_use]
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Returns the customer return URL, if set.
    #[must_use]
    pub fn return_url(&self) -> Option<&str> {
        self.return_url.as_deref()
    }

    /// Returns whether this session is verified.
    #[must_use]
    pub fn is_verified(&self) -> bool {
        self.status == VerificationSessionStatus::Verified
    }

    /// Returns whether this session is canceled.
    #[must_use]
    pub fn is_canceled(&self) -> bool {
        self.status == VerificationSessionStatus::Canceled
    }

    /// Returns whether this session requires user input.
    #[must_use]
    pub fn requires_input(&self) -> bool {
        self.status == VerificationSessionStatus::RequiresInput
    }

    /// Returns whether this session has been redacted under GDPR Article 17.
    #[must_use]
    pub fn is_redacted(&self) -> bool {
        self.redaction
            .as_ref()
            .is_some_and(VerificationSessionRedaction::is_redacted)
    }

    /// Returns whether the session exists in live mode.
    #[must_use]
    pub const fn is_livemode(&self) -> bool {
        self.livemode
    }

    /// Returns the Unix creation timestamp in seconds.
    #[must_use]
    pub const fn created(&self) -> i64 {
        self.created
    }

    /// Returns the optional expiration timestamp in seconds.
    #[must_use]
    pub const fn expires_at(&self) -> Option<i64> {
        self.expires_at
    }

    /// Returns `true` if the session is expired relative to the given timestamp.
    #[must_use]
    pub fn is_expired(&self, current_timestamp: i64) -> bool {
        self.expires_at.is_some_and(|exp| current_timestamp >= exp)
    }
}

impl fmt::Debug for VerificationSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let masked_secret = self.client_secret.as_ref().map(|_| "[REDACTED]");
        f.debug_struct("VerificationSession")
            .field("id", &self.id)
            .field("object", &self.object)
            .field("client_secret", &masked_secret)
            .field("created", &self.created)
            .field("expires_at", &self.expires_at)
            .field("livemode", &self.livemode)
            .field("status", &self.status)
            .field("type", &self.r#type)
            .field("url", &self.url)
            .field("return_url", &self.return_url)
            .field("last_error", &self.last_error)
            .field("last_verification_report", &self.last_verification_report)
            .field("metadata", &self.metadata)
            .field("options", &self.options)
            .field("redaction", &self.redaction)
            .field("verified_outputs", &self.verified_outputs)
            .finish()
    }
}

/// Parameters for creating a new [`VerificationSession`] (`POST /v1/identity/verification_sessions`).
///
/// # Example
/// ```
/// use some_stripe_types::identity_session::{
///     CreateVerificationSessionParams, DocumentOptions, VerificationOptions,
/// };
///
/// let params = CreateVerificationSessionParams::document()
///     .with_options(
///         VerificationOptions::new().with_document(
///             DocumentOptions::new().with_matching_selfie(true),
///         ),
///     )
///     .with_return_url("https://platform.com/kyc/complete")
///     .with_idempotency_key("req_idem_123");
///
/// assert_eq!(params.return_url.as_deref(), Some("https://platform.com/kyc/complete"));
/// assert_eq!(params.idempotency_key.as_deref(), Some("req_idem_123"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateVerificationSessionParams {
    /// The type of verification session to create.
    #[serde(rename = "type")]
    pub r#type: VerificationSessionType,

    /// Options for the verification session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<VerificationOptions>,

    /// The URL to which the customer is redirected after verification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_url: Option<String>,

    /// A unique identifier for the customer within your system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_reference_id: Option<String>,

    /// Set of key-value pairs attached to the session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Details about the user provided by the caller to pre-populate flow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provided_details: Option<VerificationSessionProvidedDetails>,

    /// Optional request idempotency key (header injected, skipped from body).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateVerificationSessionParams {
    /// Creates a new verification session parameter struct with the specified type.
    #[must_use]
    pub const fn new(session_type: VerificationSessionType) -> Self {
        Self {
            r#type: session_type,
            options: None,
            return_url: None,
            client_reference_id: None,
            metadata: None,
            provided_details: None,
            idempotency_key: None,
        }
    }

    /// Creates parameters for a document-based verification session.
    #[must_use]
    pub const fn document() -> Self {
        Self::new(VerificationSessionType::Document)
    }

    /// Creates parameters for an ID number verification session.
    #[must_use]
    pub const fn id_number() -> Self {
        Self::new(VerificationSessionType::IdNumber)
    }

    /// Attaches verification options.
    #[must_use]
    pub fn with_options(mut self, options: VerificationOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// Attaches customer return URL.
    #[must_use]
    pub fn with_return_url(mut self, url: impl Into<String>) -> Self {
        self.return_url = Some(url.into());
        self
    }

    /// Attaches client reference ID.
    #[must_use]
    pub fn with_client_reference_id(mut self, client_ref_id: impl Into<String>) -> Self {
        self.client_reference_id = Some(client_ref_id.into());
        self
    }

    /// Attaches metadata mapping.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Adds a single metadata entry.
    #[must_use]
    pub fn with_metadata_entry(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut map = self.metadata.unwrap_or_default();
        map.insert(key.into(), value.into());
        self.metadata = Some(map);
        self
    }

    /// Attaches pre-populated provided details.
    #[must_use]
    pub fn with_provided_details(mut self, details: VerificationSessionProvidedDetails) -> Self {
        self.provided_details = Some(details);
        self
    }

    /// Sets the request idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for updating an existing [`VerificationSession`] (`POST /v1/identity/verification_sessions/{id}`).
///
/// # Example
/// ```
/// use some_stripe_types::identity_session::{
///     DocumentOptions, UpdateVerificationSessionParams, VerificationOptions,
/// };
///
/// let params = UpdateVerificationSessionParams::new()
///     .with_options(
///         VerificationOptions::new().with_document(
///             DocumentOptions::new().with_matching_selfie(false),
///         ),
///     )
///     .with_idempotency_key("req_idem_update_456");
///
/// assert!(params.options.is_some());
/// assert_eq!(params.idempotency_key.as_deref(), Some("req_idem_update_456"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateVerificationSessionParams {
    /// The type of verification session.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<VerificationSessionType>,

    /// Options for the verification session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<VerificationOptions>,

    /// Set of key-value pairs attached to the session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Details about the user provided by the caller to pre-populate flow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provided_details: Option<VerificationSessionProvidedDetails>,

    /// Optional request idempotency key (header injected, skipped from body).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl UpdateVerificationSessionParams {
    /// Creates empty update parameters.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            r#type: None,
            options: None,
            metadata: None,
            provided_details: None,
            idempotency_key: None,
        }
    }

    /// Sets the verification session type.
    #[must_use]
    pub fn with_type(mut self, session_type: VerificationSessionType) -> Self {
        self.r#type = Some(session_type);
        self
    }

    /// Attaches verification options.
    #[must_use]
    pub fn with_options(mut self, options: VerificationOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// Attaches metadata mapping.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Adds a single metadata entry.
    #[must_use]
    pub fn with_metadata_entry(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut map = self.metadata.unwrap_or_default();
        map.insert(key.into(), value.into());
        self.metadata = Some(map);
        self
    }

    /// Attaches pre-populated provided details.
    #[must_use]
    pub fn with_provided_details(mut self, details: VerificationSessionProvidedDetails) -> Self {
        self.provided_details = Some(details);
        self
    }

    /// Sets the request idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Errors occurring during verification session parsing or validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationSessionParseError {
    /// The status string could not be parsed.
    InvalidStatus(String),
    /// The verification type string could not be parsed.
    InvalidType(String),
    /// The redirect URL failed security checks.
    InvalidUrl(String),
}

impl fmt::Display for VerificationSessionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStatus(s) => write!(f, "Invalid verification session status: '{s}'"),
            Self::InvalidType(t) => write!(f, "Invalid verification session type: '{t}'"),
            Self::InvalidUrl(msg) => write!(f, "Invalid verification session redirect URL: {msg}"),
        }
    }
}

impl std::error::Error for VerificationSessionParseError {}

/// Checks if the URL uses an authorized protocol (HTTPS or local dev).
fn is_allowed_protocol(url: &str) -> bool {
    url.starts_with("https://")
        || url.starts_with("http://localhost")
        || url.starts_with("http://127.0.0.1")
}

/// Checks if the URL contains forbidden client-side scripting schemes.
fn has_malicious_scheme(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains("javascript:") || lower.contains("data:") || lower.contains("vbscript:")
}

/// Checks if the URL contains CRLF or ASCII control characters.
fn has_control_characters(url: &str) -> bool {
    url.contains('\r') || url.contains('\n') || url.chars().any(char::is_control)
}

/// Security validator for identity verification redirect URLs.
///
/// Enforces HTTPS protocol, rejects CRLF injection and malicious schemes.
///
/// # Errors
/// Returns [`VerificationSessionParseError::InvalidUrl`] if the URL is invalid.
///
/// # Example
/// ```
/// use some_stripe_types::identity_session::validate_identity_url;
///
/// assert!(validate_identity_url("https://platform.com/kyc/complete").is_ok());
/// assert!(validate_identity_url("javascript:alert(1)").is_err());
/// ```
pub fn validate_identity_url(url: &str) -> Result<(), VerificationSessionParseError> {
    if url.is_empty() {
        return Err(VerificationSessionParseError::InvalidUrl(
            "URL cannot be empty".into(),
        ));
    }

    if !is_allowed_protocol(url) {
        return Err(VerificationSessionParseError::InvalidUrl(
            "Redirect URL must strictly use HTTPS protocol (or localhost for development)".into(),
        ));
    }

    if has_malicious_scheme(url) {
        return Err(VerificationSessionParseError::InvalidUrl(
            "Redirect URL contains forbidden or malicious URI scheme".into(),
        ));
    }

    if has_control_characters(url) {
        return Err(VerificationSessionParseError::InvalidUrl(
            "Redirect URL contains forbidden control characters or newline delimiters".into(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_session_status_roundtrip() {
        for (str_val, enum_val) in [
            ("requires_input", VerificationSessionStatus::RequiresInput),
            ("processing", VerificationSessionStatus::Processing),
            ("verified", VerificationSessionStatus::Verified),
            ("canceled", VerificationSessionStatus::Canceled),
        ] {
            assert_eq!(enum_val.as_str(), str_val);
            assert_eq!(
                VerificationSessionStatus::from_str(str_val).unwrap(),
                enum_val
            );
            let json = serde_json::to_string(&enum_val).unwrap();
            assert_eq!(json, format!("\"{str_val}\""));
            let de: VerificationSessionStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(de, enum_val);
        }
        assert!(VerificationSessionStatus::from_str("invalid_status").is_err());
    }

    #[test]
    fn test_verification_session_type_roundtrip() {
        for (str_val, enum_val) in [
            ("document", VerificationSessionType::Document),
            ("id_number", VerificationSessionType::IdNumber),
        ] {
            assert_eq!(enum_val.as_str(), str_val);
            assert_eq!(
                VerificationSessionType::from_str(str_val).unwrap(),
                enum_val
            );
            let json = serde_json::to_string(&enum_val).unwrap();
            assert_eq!(json, format!("\"{str_val}\""));
            let de: VerificationSessionType = serde_json::from_str(&json).unwrap();
            assert_eq!(de, enum_val);
        }
        assert!(VerificationSessionType::from_str("unknown_type").is_err());
    }

    #[test]
    fn test_verification_session_redaction_helpers() {
        let red = VerificationSessionRedaction {
            status: Some("redacted".into()),
        };
        assert!(red.is_redacted());
        assert!(!red.is_processing());

        let proc = VerificationSessionRedaction {
            status: Some("processing".into()),
        };
        assert!(!proc.is_redacted());
        assert!(proc.is_processing());

        let none = VerificationSessionRedaction { status: None };
        assert!(!none.is_redacted());
        assert!(!none.is_processing());
    }

    #[test]
    fn test_verification_session_deserialization_and_debug_redaction() {
        let json = r#"{
            "id": "vs_test_abc123",
            "object": "identity.verification_session",
            "client_secret": "vcs_secret_very_sensitive_999",
            "created": 1700000000,
            "expires_at": 1700086400,
            "livemode": false,
            "status": "requires_input",
            "type": "document",
            "url": "https://verify.stripe.com/start/vs_test_abc123",
            "return_url": "https://myplatform.com/verified",
            "options": {
                "document": {
                    "require_matching_selfie": true,
                    "require_live_capture": true
                }
            },
            "redaction": {
                "status": "redacted"
            }
        }"#;

        let session: VerificationSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.id(), "vs_test_abc123");
        assert_eq!(
            session.client_secret(),
            Some("vcs_secret_very_sensitive_999")
        );
        assert_eq!(session.status(), VerificationSessionStatus::RequiresInput);
        assert_eq!(
            session.verification_type(),
            VerificationSessionType::Document
        );
        assert_eq!(
            session.url(),
            Some("https://verify.stripe.com/start/vs_test_abc123")
        );
        assert_eq!(
            session.return_url(),
            Some("https://myplatform.com/verified")
        );
        assert!(session.requires_input());
        assert!(!session.is_verified());
        assert!(!session.is_canceled());
        assert!(session.is_redacted());
        assert_eq!(session.created(), 1700000000);
        assert_eq!(session.expires_at(), Some(1700086400));
        assert!(!session.is_expired(1700000000));
        assert!(session.is_expired(1700086401));

        // Gate G2 Debug redaction check
        let debug = format!("{session:?}");
        assert!(!debug.contains("vcs_secret_very_sensitive_999"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn test_create_params_builder() {
        let params = CreateVerificationSessionParams::document()
            .with_options(VerificationOptions::for_document(
                DocumentOptions::new()
                    .with_matching_selfie(true)
                    .with_live_capture(true)
                    .with_id_number(true)
                    .with_allowed_types(vec!["passport".into(), "driving_license".into()]),
            ))
            .with_return_url("https://platform.com/complete")
            .with_client_reference_id("cust_user_42")
            .with_metadata_entry("tenant", "org_1")
            .with_provided_details(VerificationSessionProvidedDetails {
                email: Some("user@example.com".into()),
                phone: Some("+15551234567".into()),
            })
            .with_idempotency_key("req_key_1");

        assert_eq!(params.r#type, VerificationSessionType::Document);
        assert_eq!(
            params.return_url.as_deref(),
            Some("https://platform.com/complete")
        );
        assert_eq!(params.client_reference_id.as_deref(), Some("cust_user_42"));
        assert_eq!(
            params
                .metadata
                .as_ref()
                .unwrap()
                .get("tenant")
                .map(String::as_str),
            Some("org_1")
        );
        assert_eq!(params.idempotency_key.as_deref(), Some("req_key_1"));

        // Verify idempotency_key is skipped from serialization
        let json = serde_json::to_string(&params).unwrap();
        assert!(!json.contains("idempotency_key"));
        assert!(json.contains("require_matching_selfie"));
    }

    #[test]
    fn test_update_params_builder() {
        let params = UpdateVerificationSessionParams::new()
            .with_type(VerificationSessionType::IdNumber)
            .with_options(VerificationOptions::for_id_number(IdNumberOptions::new()))
            .with_metadata_entry("updated", "true")
            .with_idempotency_key("update_key_2");

        assert_eq!(params.r#type, Some(VerificationSessionType::IdNumber));
        assert!(params.options.is_some());
        assert_eq!(
            params
                .metadata
                .as_ref()
                .unwrap()
                .get("updated")
                .map(String::as_str),
            Some("true")
        );
        assert_eq!(params.idempotency_key.as_deref(), Some("update_key_2"));
    }

    #[test]
    fn test_validate_identity_url() {
        assert!(validate_identity_url("https://platform.com/kyc/complete").is_ok());
        assert!(validate_identity_url("http://localhost:3000/callback").is_ok());
        assert!(validate_identity_url("http://127.0.0.1:8080/callback").is_ok());

        // Insecure HTTP
        assert!(validate_identity_url("http://insecure.com/callback").is_err());
        // Empty
        assert!(validate_identity_url("").is_err());
        // Malicious schemes
        assert!(validate_identity_url("javascript:alert(1)").is_err());
        assert!(validate_identity_url("data:text/html,evil").is_err());
        assert!(validate_identity_url("vbscript:msgbox").is_err());
        // CRLF injection
        assert!(validate_identity_url("https://platform.com/ok\r\nInjected: Header").is_err());
        assert!(validate_identity_url("https://platform.com/ok\nInjected: Header").is_err());
    }
}
