//! # Stripe Terminal POS Readers, Locations & Connection Tokens
//!
//! Strongly typed domain models, request parameters, and security validators for
//! Stripe Terminal in-person point-of-sale (POS) hardware and SDK integrations.
//!
//! ## Architectural & Compliance Highlights
//! - **AC-US-7.2.0-03**: Ephemeral connection token generation with location binding for smart reader hardware.
//! - **EMV Level 1 & 2 Certification**: Reader device compatibility validators ensuring contact (chip),
//!   contactless (NFC / Apple Pay / Google Pay), and magnetic stripe transaction security invariants.
//! - **Gate G-Borrow**: Zero-clone reference borrowing for identifiers, location bindings, and ephemeral secrets.
//! - **Gate G2 (Security Gateway)**: Strict registration code sanitization, location ID structure validation,
//!   and secret redaction in debug logs.
//! - **Gate G-Complexity**: Decomposed, table-driven validation functions with cyclomatic complexity $CC \le 5$.
//!
//! # Quickstart
//! ```
//! use some_stripe_types::terminal::{
//!     CreateConnectionTokenParams, CreateReaderParams, TerminalReader,
//! };
//!
//! // Configure connection token scoped to a physical store location
//! let token_params = CreateConnectionTokenParams::for_location("loc_123");
//! assert!(token_params.validate().is_ok());
//!
//! // Register a smart POS reader device
//! let reader_params = CreateReaderParams::new("simulated-wpos-1")
//!     .with_label("Front Register 1")
//!     .with_location("loc_123");
//! assert!(reader_params.validate().is_ok());
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Default object identifier for a Terminal Reader (`"terminal.reader"`).
#[must_use]
pub fn default_terminal_reader_object() -> String {
    "terminal.reader".into()
}

/// Default object identifier for a Terminal Location (`"terminal.location"`).
#[must_use]
pub fn default_terminal_location_object() -> String {
    "terminal.location".into()
}

/// Default object identifier for a Terminal Connection Token (`"terminal.connection_token"`).
#[must_use]
pub fn default_connection_token_object() -> String {
    "terminal.connection_token".into()
}

/// Default object identifier for a paginated list (`"list"`).
#[must_use]
fn default_list_object() -> String {
    "list".into()
}

/// Paginated list of [`TerminalReader`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalReaderList {
    /// String representing the object's type. Always `list`.
    #[serde(default = "default_list_object")]
    pub object: String,

    /// List of terminal readers.
    pub data: Vec<TerminalReader>,

    /// True if this list has another page of items.
    pub has_more: bool,

    /// The URL where this list can be accessed.
    #[serde(default)]
    pub url: String,
}

/// Paginated list of [`TerminalLocation`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalLocationList {
    /// String representing the object's type. Always `list`.
    #[serde(default = "default_list_object")]
    pub object: String,

    /// List of terminal locations.
    pub data: Vec<TerminalLocation>,

    /// True if this list has another page of items.
    pub has_more: bool,

    /// The URL where this list can be accessed.
    #[serde(default)]
    pub url: String,
}

/// Physical address for a Terminal Location.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalAddress {
    /// City, district, suburb, town, or village.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,

    /// Two-letter country code (ISO 3166-1 alpha-2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Address line 1 (e.g., street, PO Box, or company name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line1: Option<String>,

    /// Address line 2 (e.g., apartment, suite, unit, or building).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line2: Option<String>,

    /// ZIP or postal code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,

    /// State, county, province, or region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

impl TerminalAddress {
    /// Constructs a new empty [`TerminalAddress`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            city: None,
            country: None,
            line1: None,
            line2: None,
            postal_code: None,
            state: None,
        }
    }

    /// Sets the street address line 1.
    #[must_use]
    pub fn with_line1(mut self, line1: impl Into<String>) -> Self {
        self.line1 = Some(line1.into());
        self
    }

    /// Sets the city.
    #[must_use]
    pub fn with_city(mut self, city: impl Into<String>) -> Self {
        self.city = Some(city.into());
        self
    }

    /// Sets the country code.
    #[must_use]
    pub fn with_country(mut self, country: impl Into<String>) -> Self {
        self.country = Some(country.into());
        self
    }

    /// Sets the postal code.
    #[must_use]
    pub fn with_postal_code(mut self, postal_code: impl Into<String>) -> Self {
        self.postal_code = Some(postal_code.into());
        self
    }

    /// Sets the state.
    #[must_use]
    pub fn with_state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }
}

/// Represents a physical retail location where Terminal readers operate (`terminal.location`).
///
/// In-person readers must be registered to a location so that regional EMV compliance,
/// currency routing, and tax calculations are handled correctly.
///
/// # Example
/// ```
/// use some_stripe_types::terminal::TerminalLocation;
///
/// let json = r#"{
///   "id": "loc_123",
///   "object": "terminal.location",
///   "display_name": "Downtown Bakery",
///   "livemode": false
/// }"#;
/// let location: TerminalLocation = serde_json::from_str(json).unwrap();
/// assert_eq!(location.id(), "loc_123");
/// assert_eq!(location.display_name(), "Downtown Bakery");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalLocation {
    /// Unique identifier for the location (`loc_...`).
    pub id: String,

    /// String representing the object's type. Always `terminal.location`.
    #[serde(default = "default_terminal_location_object")]
    pub object: String,

    /// The physical address of the location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<TerminalAddress>,

    /// The friendly name of the location.
    pub display_name: String,

    /// Has the value `true` if the object exists in live mode or `false` if in test mode.
    #[serde(default)]
    pub livemode: bool,

    /// Set of key-value pairs attached to the location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl TerminalLocation {
    /// Returns a borrowed reference to the location ID (Gate G-Borrow).
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns a borrowed reference to the display name.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Returns a borrowed reference to the address if present.
    #[must_use]
    pub fn address(&self) -> Option<&TerminalAddress> {
        self.address.as_ref()
    }

    /// Returns `true` if this location is in livemode.
    #[must_use]
    pub const fn is_livemode(&self) -> bool {
        self.livemode
    }
}

/// Reader action details when processing in-person payment presentment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderAction {
    /// Type of action currently being performed on the reader (`process_payment_intent`, etc.).
    #[serde(rename = "type")]
    pub action_type: String,

    /// Status of the action (`in_progress`, `succeeded`, `failed`).
    pub status: String,

    /// Failure code if the action failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_code: Option<String>,

    /// Detailed failure message if the action failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_message: Option<String>,
}

/// Represents a physical or simulated point-of-sale card reader device (`terminal.reader`).
///
/// Under EMV Level 1/2 requirements, readers communicate with card chips and contactless
/// antennas to safely capture cardholder presentment data without exposing raw PANs.
///
/// # Example
/// ```
/// use some_stripe_types::terminal::TerminalReader;
///
/// let json = r#"{
///   "id": "tmr_123",
///   "object": "terminal.reader",
///   "device_type": "bbpos_wisepos_e",
///   "label": "Counter Register",
///   "livemode": false,
///   "location": "loc_123",
///   "serial_number": "WPOS-123456",
///   "status": "online"
/// }"#;
/// let reader: TerminalReader = serde_json::from_str(json).unwrap();
/// assert_eq!(reader.id(), "tmr_123");
/// assert!(reader.is_online());
/// assert!(reader.is_emv_certified());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalReader {
    /// Unique identifier for the reader (`tmr_...`).
    pub id: String,

    /// String representing the object's type. Always `terminal.reader`.
    #[serde(default = "default_terminal_reader_object")]
    pub object: String,

    /// Type of reader hardware (e.g., `bbpos_wisepos_e`, `verifone_P400`, `stripe_s700`).
    pub device_type: String,

    /// Current software version running on the reader device.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_sw_version: Option<String>,

    /// The local IP address of the reader on the local area network.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,

    /// Custom label assigned to the reader.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// Has the value `true` if the reader exists in live mode or `false` if in test mode.
    #[serde(default)]
    pub livemode: bool,

    /// The location identifier the reader is registered to (`loc_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    /// Serial number of the reader device.
    #[serde(default)]
    pub serial_number: String,

    /// Current connectivity status of the reader (`online` or `offline`).
    #[serde(default)]
    pub status: String,

    /// Information about the current action being processed by the reader.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<ReaderAction>,

    /// Set of key-value pairs attached to the reader.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl TerminalReader {
    /// Returns a borrowed reference to the reader ID (Gate G-Borrow).
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns a borrowed reference to the device type.
    #[must_use]
    pub fn device_type(&self) -> &str {
        &self.device_type
    }

    /// Returns a borrowed reference to the serial number.
    #[must_use]
    pub fn serial_number(&self) -> &str {
        &self.serial_number
    }

    /// Returns a borrowed reference to the status string (`online` or `offline`).
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    /// Returns `true` if the reader is currently online.
    #[must_use]
    pub fn is_online(&self) -> bool {
        self.status.eq_ignore_ascii_case("online")
    }

    /// Returns a borrowed reference to the location ID if registered to a location.
    #[must_use]
    pub fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }

    /// Returns a borrowed reference to the label if configured.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Verifies whether the hardware device meets EMV Level 1/2 contact and contactless standards.
    #[must_use]
    pub fn is_emv_certified(&self) -> bool {
        is_emv_compliant_device(&self.device_type)
    }
}

/// Represents an ephemeral connection token authorizing Terminal SDK reader discovery (`terminal.connection_token`).
///
/// Connection tokens grant short-lived authorization for a mobile app or web POS client
/// to connect to a Terminal reader without exposing primary API secret keys.
///
/// # Example
/// ```
/// use some_stripe_types::terminal::ConnectionToken;
///
/// let json = r#"{
///   "object": "terminal.connection_token",
///   "secret": "pst_test_secret_12345",
///   "location": "loc_123"
/// }"#;
/// let token: ConnectionToken = serde_json::from_str(json).unwrap();
/// assert_eq!(token.secret(), "pst_test_secret_12345");
/// assert!(token.is_bound_to_location("loc_123"));
/// ```
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionToken {
    /// String representing the object's type. Always `terminal.connection_token`.
    #[serde(default = "default_connection_token_object")]
    pub object: String,

    /// Ephemeral secret key used by the Terminal SDK to connect to readers.
    pub secret: String,

    /// The location identifier the connection token is bound to, if scoped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

impl ConnectionToken {
    /// Returns a borrowed reference to the ephemeral secret (Gate G-Borrow).
    #[must_use]
    pub fn secret(&self) -> &str {
        &self.secret
    }

    /// Returns a borrowed reference to the location ID if scoped.
    #[must_use]
    pub fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }

    /// Returns `true` if this token is bound to the given location ID.
    #[must_use]
    pub fn is_bound_to_location(&self, location_id: &str) -> bool {
        self.location.as_deref() == Some(location_id)
    }
}

impl fmt::Debug for ConnectionToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectionToken")
            .field("object", &self.object)
            .field("secret", &"[REDACTED]")
            .field("location", &self.location)
            .finish()
    }
}

/// Errors occurring during validation of Terminal domain parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalValidationError {
    /// The registration code was empty or missing.
    EmptyRegistrationCode,
    /// The registration code contained invalid characters or length.
    InvalidRegistrationCode(String),
    /// The location display name was empty.
    EmptyDisplayName,
    /// The location ID format is invalid (must start with `loc_`).
    InvalidLocationId(String),
    /// The address line or country was invalid.
    InvalidAddress(&'static str),
}

impl fmt::Display for TerminalValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRegistrationCode => {
                write!(f, "Terminal reader registration code cannot be empty")
            }
            Self::InvalidRegistrationCode(c) => write!(
                f,
                "Invalid registration code '{c}': must be 3-64 characters alphanumeric with hyphens"
            ),
            Self::EmptyDisplayName => write!(f, "Terminal location display_name cannot be empty"),
            Self::InvalidLocationId(id) => {
                write!(f, "Invalid location ID '{id}': must start with 'loc_'")
            }
            Self::InvalidAddress(reason) => write!(f, "Invalid terminal address: {reason}"),
        }
    }
}

impl std::error::Error for TerminalValidationError {}

/// Validates whether a device type is certified for EMV Level 1 and Level 2 card presentment.
///
/// Supported hardware includes:
/// - `bbpos_chipper2x`
/// - `bbpos_wisepad3`
/// - `bbpos_wisepos_e`
/// - `verifone_p400`
/// - `stripe_s700`
/// - `simulated_wisepos_e`
#[must_use]
pub fn is_emv_compliant_device(device_type: &str) -> bool {
    let normalized = device_type.to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "bbpos_chipper2x"
            | "bbpos_wisepad3"
            | "bbpos_wisepos_e"
            | "verifone_p400"
            | "stripe_s700"
            | "stripe_m2"
            | "simulated_wisepos_e"
            | "simulated_s700"
    )
}

/// Validates a Terminal reader pairing registration code.
///
/// Prevents command injection and malformed inputs during POS pairing.
///
/// # Errors
/// Returns [`TerminalValidationError`] if empty or invalid.
pub fn validate_registration_code(code: &str) -> Result<(), TerminalValidationError> {
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return Err(TerminalValidationError::EmptyRegistrationCode);
    }
    if trimmed.len() < 3 || trimmed.len() > 64 {
        return Err(TerminalValidationError::InvalidRegistrationCode(
            trimmed.to_string(),
        ));
    }
    let is_valid = trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !is_valid {
        return Err(TerminalValidationError::InvalidRegistrationCode(
            trimmed.to_string(),
        ));
    }
    Ok(())
}

/// Validates that an optional location ID begins with `loc_`.
///
/// # Errors
/// Returns [`TerminalValidationError::InvalidLocationId`] if malformed.
pub fn validate_location_id(location: Option<&str>) -> Result<(), TerminalValidationError> {
    if let Some(loc) = location {
        let trimmed = loc.trim();
        if !trimmed.starts_with("loc_") || trimmed.len() <= 4 {
            return Err(TerminalValidationError::InvalidLocationId(
                trimmed.to_string(),
            ));
        }
    }
    Ok(())
}

/// Parameters for registering a new [`TerminalReader`] (`POST /v1/terminal/readers`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateReaderParams {
    /// A code generated by the reader used for pairing.
    pub registration_code: String,

    /// Custom label given to the reader for identification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// The location identifier to register the reader to (`loc_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    /// Set of key-value pairs attached to the reader.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Optional idempotency key for safe retries (omitted from body serialization).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateReaderParams {
    /// Constructs parameters with the mandatory registration code.
    #[must_use]
    pub fn new(registration_code: impl Into<String>) -> Self {
        Self {
            registration_code: registration_code.into(),
            label: None,
            location: None,
            metadata: None,
            idempotency_key: None,
        }
    }

    /// Sets the reader label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Scopes the reader to a physical location (`loc_...`).
    #[must_use]
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Sets metadata key-value pairs.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Attaches an explicit idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates the reader creation parameters.
    ///
    /// # Errors
    /// Returns [`TerminalValidationError`] if registration code or location ID is invalid.
    pub fn validate(&self) -> Result<(), TerminalValidationError> {
        validate_registration_code(&self.registration_code)?;
        validate_location_id(self.location.as_deref())?;
        Ok(())
    }
}

/// Parameters for creating a [`TerminalLocation`] (`POST /v1/terminal/locations`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateLocationParams {
    /// A name for the location.
    pub display_name: String,

    /// The physical address of the location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<TerminalAddress>,

    /// Set of key-value pairs attached to the location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Optional idempotency key for safe retries (omitted from body serialization).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateLocationParams {
    /// Constructs parameters with the mandatory display name.
    #[must_use]
    pub fn new(display_name: impl Into<String>) -> Self {
        Self {
            display_name: display_name.into(),
            address: None,
            metadata: None,
            idempotency_key: None,
        }
    }

    /// Sets the physical address.
    #[must_use]
    pub fn with_address(mut self, address: TerminalAddress) -> Self {
        self.address = Some(address);
        self
    }

    /// Sets metadata key-value pairs.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Attaches an explicit idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates location creation parameters.
    ///
    /// # Errors
    /// Returns [`TerminalValidationError::EmptyDisplayName`] if display name is empty.
    pub fn validate(&self) -> Result<(), TerminalValidationError> {
        if self.display_name.trim().is_empty() {
            return Err(TerminalValidationError::EmptyDisplayName);
        }
        Ok(())
    }
}

/// Parameters for generating a [`ConnectionToken`] (`POST /v1/terminal/connection_tokens`).
///
/// Satisfies **AC-US-7.2.0-03**: Generating ephemeral connection token with optional location binding.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateConnectionTokenParams {
    /// Binds the connection token to a specific location (`loc_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    /// Optional idempotency key for safe retries (omitted from body serialization).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateConnectionTokenParams {
    /// Constructs empty parameters for an unscoped connection token.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            location: None,
            idempotency_key: None,
        }
    }

    /// Constructs parameters binding the token to a specific location (`loc_...`).
    #[must_use]
    pub fn for_location(location: impl Into<String>) -> Self {
        Self {
            location: Some(location.into()),
            idempotency_key: None,
        }
    }

    /// Sets the location binding.
    #[must_use]
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Attaches an explicit idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates connection token parameters.
    ///
    /// # Errors
    /// Returns [`TerminalValidationError::InvalidLocationId`] if location is malformed.
    pub fn validate(&self) -> Result<(), TerminalValidationError> {
        validate_location_id(self.location.as_deref())
    }
}

/// Filter parameters for listing Terminal readers (`GET /v1/terminal/readers`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListTerminalReadersParams {
    /// Filter readers by device type (e.g., `bbpos_wisepos_e`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_type: Option<String>,

    /// Filter readers registered to a specific location ID (`loc_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    /// Filter readers by status (`online` or `offline`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Maximum number of readers to return (1-100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Cursor for forward pagination (`tmr_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// Cursor for backward pagination (`tmr_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListTerminalReadersParams {
    /// Constructs empty list parameters.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            device_type: None,
            location: None,
            status: None,
            limit: None,
            starting_after: None,
            ending_before: None,
        }
    }

    /// Scopes list query to a location ID.
    #[must_use]
    pub fn for_location(location: impl Into<String>) -> Self {
        Self {
            location: Some(location.into()),
            ..Self::new()
        }
    }

    /// Sets limit of returned readers.
    #[must_use]
    pub const fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Filters by reader online status.
    #[must_use]
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }
}

/// Filter parameters for listing Terminal locations (`GET /v1/terminal/locations`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListTerminalLocationsParams {
    /// Maximum number of locations to return (1-100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Cursor for forward pagination (`loc_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// Cursor for backward pagination (`loc_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListTerminalLocationsParams {
    /// Constructs empty list parameters.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            limit: None,
            starting_after: None,
            ending_before: None,
        }
    }

    /// Sets limit of returned locations.
    #[must_use]
    pub const fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}
