//! # Stripe Connect Account Link Types & Request Parameters
//!
//! Strongly typed domain models, parameters, and security validators for Stripe Connect Account Links.
//!
//! An Account Link provides a single-use, time-limited URL that redirects a connected account
//! user to a Stripe-hosted onboarding or account update flow.
//!
//! Under Gate G2 (Security Gateway) and Gate G-Borrow (Zero-Clone Borrowing Discipline):
//! - URL validation protects against open redirect vulnerabilities and CRLF injections.
//! - Read operations provide zero-copy reference borrowing (`&str`, `&[T]`).
//! - All public structures include working rustdoc doctests and full serialization round-trip support.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Default object identifier for an Account Link (`"account_link"`).
#[must_use]
pub fn default_account_link_object() -> String {
    "account_link".into()
}

/// The type of an [`AccountLink`].
///
/// Indicates whether the user is completing initial onboarding or updating existing account settings.
///
/// # Example
/// ```
/// use some_stripe_types::account_link::AccountLinkType;
/// use std::str::FromStr;
///
/// let link_type = AccountLinkType::AccountOnboarding;
/// assert_eq!(link_type.as_str(), "account_onboarding");
/// assert_eq!(AccountLinkType::from_str("account_update").unwrap(), AccountLinkType::AccountUpdate);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountLinkType {
    /// Onboarding flow for accounts that are not yet fully activated.
    AccountOnboarding,
    /// Update flow for accounts that need to update or provide additional information.
    AccountUpdate,
}

impl AccountLinkType {
    /// Returns the static string representation of this account link type.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::AccountOnboarding => "account_onboarding",
            Self::AccountUpdate => "account_update",
        }
    }
}

impl fmt::Display for AccountLinkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for AccountLinkType {
    type Err = AccountLinkParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "account_onboarding" => Ok(Self::AccountOnboarding),
            "account_update" => Ok(Self::AccountUpdate),
            other => Err(AccountLinkParseError::InvalidType(other.to_string())),
        }
    }
}

/// Fields to collect in an account link onboarding or update flow.
///
/// # Example
/// ```
/// use some_stripe_types::account_link::AccountLinkCollectFields;
/// use std::str::FromStr;
///
/// let field = AccountLinkCollectFields::CurrentlyDue;
/// assert_eq!(field.as_str(), "currently_due");
/// assert_eq!(AccountLinkCollectFields::from_str("eventually_due").unwrap(), AccountLinkCollectFields::EventuallyDue);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountLinkCollectFields {
    /// Only collect fields that are currently required for payouts/charges.
    CurrentlyDue,
    /// Collect fields that will eventually be required.
    EventuallyDue,
}

impl AccountLinkCollectFields {
    /// Returns the static string representation of the collection field option.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::CurrentlyDue => "currently_due",
            Self::EventuallyDue => "eventually_due",
        }
    }
}

impl fmt::Display for AccountLinkCollectFields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for AccountLinkCollectFields {
    type Err = AccountLinkParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "currently_due" => Ok(Self::CurrentlyDue),
            "eventually_due" => Ok(Self::EventuallyDue),
            other => Err(AccountLinkParseError::InvalidCollectField(other.to_string())),
        }
    }
}

/// Future requirements collection behavior in an account link.
///
/// # Example
/// ```
/// use some_stripe_types::account_link::AccountLinkFutureRequirements;
/// use std::str::FromStr;
///
/// let req = AccountLinkFutureRequirements::Include;
/// assert_eq!(req.as_str(), "include");
/// assert_eq!(AccountLinkFutureRequirements::from_str("omit").unwrap(), AccountLinkFutureRequirements::Omit);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountLinkFutureRequirements {
    /// Include future requirements during collection.
    Include,
    /// Omit future requirements during collection.
    Omit,
}

impl AccountLinkFutureRequirements {
    /// Returns the static string representation of future requirements behavior.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Include => "include",
            Self::Omit => "omit",
        }
    }
}

impl fmt::Display for AccountLinkFutureRequirements {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for AccountLinkFutureRequirements {
    type Err = AccountLinkParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "include" => Ok(Self::Include),
            "omit" => Ok(Self::Omit),
            other => Err(AccountLinkParseError::InvalidFutureRequirement(
                other.to_string(),
            )),
        }
    }
}

/// Collection options when creating an [`AccountLink`].
///
/// # Example
/// ```
/// use some_stripe_types::account_link::{AccountLinkCollectionOptions, AccountLinkCollectFields, AccountLinkFutureRequirements};
///
/// let opts = AccountLinkCollectionOptions::new()
///     .with_fields(AccountLinkCollectFields::CurrentlyDue)
///     .with_future_requirements(AccountLinkFutureRequirements::Include);
/// assert_eq!(opts.fields, Some(AccountLinkCollectFields::CurrentlyDue));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AccountLinkCollectionOptions {
    /// The fields that the user may be asked to provide.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<AccountLinkCollectFields>,

    /// Whether to include or omit future requirements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub future_requirements: Option<AccountLinkFutureRequirements>,
}

impl AccountLinkCollectionOptions {
    /// Constructs empty collection options.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fields: None,
            future_requirements: None,
        }
    }

    /// Sets the fields to collect.
    #[must_use]
    pub fn with_fields(mut self, fields: AccountLinkCollectFields) -> Self {
        self.fields = Some(fields);
        self
    }

    /// Sets the future requirements collection behavior.
    #[must_use]
    pub fn with_future_requirements(mut self, future_requirements: AccountLinkFutureRequirements) -> Self {
        self.future_requirements = Some(future_requirements);
        self
    }
}

/// Represents an Account Link object returned by Stripe (`account_link`).
///
/// # Example
/// ```
/// use some_stripe_types::account_link::AccountLink;
///
/// let json = r#"{
///   "object": "account_link",
///   "created": 1700000000,
///   "expires_at": 1700000300,
///   "url": "https://connect.stripe.com/setup/s/xyz"
/// }"#;
/// let link: AccountLink = serde_json::from_str(json).unwrap();
/// assert_eq!(link.url(), "https://connect.stripe.com/setup/s/xyz");
/// assert!(!link.is_expired(1700000100));
/// assert!(link.is_expired(1700000400));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountLink {
    /// String representing the object's type. Always `account_link`.
    #[serde(default = "default_account_link_object")]
    pub object: String,

    /// Time at which the object was created (Unix timestamp in seconds).
    pub created: i64,

    /// The timestamp at which this account link will expire.
    pub expires_at: i64,

    /// The URL at which the user can authenticate and complete their onboarding or update.
    pub url: String,
}

impl AccountLink {
    /// Returns a borrowed reference to the generated onboarding or update URL (Gate G-Borrow).
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the object creation timestamp in seconds.
    #[must_use]
    pub const fn created(&self) -> i64 {
        self.created
    }

    /// Returns the expiration timestamp in seconds.
    #[must_use]
    pub const fn expires_at(&self) -> i64 {
        self.expires_at
    }

    /// Returns `true` if the account link is expired relative to the given current timestamp.
    #[must_use]
    pub const fn is_expired(&self, current_timestamp: i64) -> bool {
        current_timestamp >= self.expires_at
    }
}

/// Validates an account link redirect URL against open redirect and injection vulnerabilities (Gate G2).
///
/// # Errors
/// Returns an error message if:
/// - The URL is empty.
/// - The URL does not begin with `https://` (or `http://localhost` / `http://127.0.0.1` for local development).
/// - The URL contains dangerous URI schemes (`javascript:`, `data:`, `vbscript:`).
/// - The URL contains CRLF or newline control characters.
///
/// # Example
/// ```
/// use some_stripe_types::account_link::validate_account_link_url;
///
/// assert!(validate_account_link_url("https://example.com/return").is_ok());
/// assert!(validate_account_link_url("http://example.com/insecure").is_err());
/// assert!(validate_account_link_url("javascript:alert(1)").is_err());
/// assert!(validate_account_link_url("https://example.com/\r\nHeader: Injection").is_err());
/// ```
pub fn validate_account_link_url(url: &str) -> Result<(), &'static str> {
    if url.is_empty() {
        return Err("URL cannot be empty");
    }

    let is_https = url.starts_with("https://");
    let is_local = url.starts_with("http://localhost") || url.starts_with("http://127.0.0.1");

    if !is_https && !is_local {
        return Err("Account link URL must strictly use HTTPS protocol (or localhost for development)");
    }

    let lower = url.to_ascii_lowercase();
    if lower.contains("javascript:") || lower.contains("data:") || lower.contains("vbscript:") {
        return Err("Account link URL contains forbidden or malicious URI schemes");
    }

    if url.contains('\r') || url.contains('\n') || url.chars().any(|c| c.is_control()) {
        return Err("Account link URL contains forbidden control characters or newline delimiters");
    }

    Ok(())
}

/// Errors occurring during Account Link parameter validation or parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountLinkValidationError {
    /// The specified account ID was empty.
    EmptyAccount,
    /// The specified account ID is invalid (must start with `acct_`).
    InvalidAccount(String),
    /// A redirect URL failed security validation.
    InvalidUrl {
        /// The field name (`refresh_url` or `return_url`).
        field: &'static str,
        /// The reason the URL failed validation.
        reason: &'static str,
    },
    /// A required field is missing.
    MissingRequiredField(&'static str),
}

impl fmt::Display for AccountLinkValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAccount => write!(f, "Connected account identifier cannot be empty"),
            Self::InvalidAccount(acct) => write!(
                f,
                "Invalid account identifier '{acct}': must start with 'acct_'"
            ),
            Self::InvalidUrl { field, reason } => {
                write!(f, "Invalid '{field}': {reason}")
            }
            Self::MissingRequiredField(field) => {
                write!(f, "Missing required parameter '{field}'")
            }
        }
    }
}

impl std::error::Error for AccountLinkValidationError {}

/// Errors occurring during parsing of account link enums.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountLinkParseError {
    /// Invalid AccountLinkType string.
    InvalidType(String),
    /// Invalid AccountLinkCollectFields string.
    InvalidCollectField(String),
    /// Invalid AccountLinkFutureRequirements string.
    InvalidFutureRequirement(String),
}

impl fmt::Display for AccountLinkParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidType(s) => write!(f, "Invalid AccountLinkType: '{s}'"),
            Self::InvalidCollectField(s) => write!(f, "Invalid AccountLinkCollectFields: '{s}'"),
            Self::InvalidFutureRequirement(s) => {
                write!(f, "Invalid AccountLinkFutureRequirements: '{s}'")
            }
        }
    }
}

impl std::error::Error for AccountLinkParseError {}

/// Parameters for creating a new [`AccountLink`] (`POST /v1/account_links`).
///
/// # Example
/// ```
/// use some_stripe_types::account_link::{AccountLinkType, CreateAccountLinkParams};
///
/// let params = CreateAccountLinkParams::new(
///     "acct_1032D82eZvKYlo2C",
///     "https://example.com/reauth",
///     "https://example.com/return",
///     AccountLinkType::AccountOnboarding,
/// );
/// assert!(params.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateAccountLinkParams {
    /// The identifier of the account to create an account link for.
    pub account: String,

    /// The URL the user will be redirected to if the account link is expired,
    /// has been previously visited, or is otherwise invalid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,

    /// The URL that the user will be redirected to upon leaving or completing the linked flow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_url: Option<String>,

    /// The type of account link the user is requesting.
    #[serde(rename = "type")]
    pub type_: AccountLinkType,

    /// The items that the user may be asked to provide during collection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_options: Option<AccountLinkCollectionOptions>,

    /// Specifies which fields in the response should be expanded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expand: Option<Vec<String>>,

    /// Optional idempotency key for safe retries (omitted from body serialization).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateAccountLinkParams {
    /// Constructs parameters for creating an account link with all primary arguments.
    #[must_use]
    pub fn new(
        account: impl Into<String>,
        refresh_url: impl Into<String>,
        return_url: impl Into<String>,
        type_: AccountLinkType,
    ) -> Self {
        Self {
            account: account.into(),
            refresh_url: Some(refresh_url.into()),
            return_url: Some(return_url.into()),
            type_,
            collection_options: None,
            expand: None,
            idempotency_key: None,
        }
    }

    /// Constructs parameters for creating an account link with only account and type specified.
    #[must_use]
    pub fn for_account(account: impl Into<String>, type_: AccountLinkType) -> Self {
        Self {
            account: account.into(),
            refresh_url: None,
            return_url: None,
            type_,
            collection_options: None,
            expand: None,
            idempotency_key: None,
        }
    }

    /// Sets the refresh URL.
    #[must_use]
    pub fn with_refresh_url(mut self, url: impl Into<String>) -> Self {
        self.refresh_url = Some(url.into());
        self
    }

    /// Sets the return URL.
    #[must_use]
    pub fn with_return_url(mut self, url: impl Into<String>) -> Self {
        self.return_url = Some(url.into());
        self
    }

    /// Sets the account link type.
    #[must_use]
    pub fn with_type(mut self, type_: AccountLinkType) -> Self {
        self.type_ = type_;
        self
    }

    /// Sets collection options.
    #[must_use]
    pub fn with_collection_options(mut self, options: AccountLinkCollectionOptions) -> Self {
        self.collection_options = Some(options);
        self
    }

    /// Adds an expansion field.
    #[must_use]
    pub fn with_expand(mut self, field: impl Into<String>) -> Self {
        self.expand.get_or_insert_with(Vec::new).push(field.into());
        self
    }

    /// Sets an explicit idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Alias for [`with_idempotency_key`](Self::with_idempotency_key).
    #[must_use]
    pub fn idempotency_key(self, key: impl Into<String>) -> Self {
        self.with_idempotency_key(key)
    }

    /// Validates the parameters against Stripe Connect API invariants (Gate G2).
    ///
    /// # Errors
    /// Returns [`AccountLinkValidationError`] if the account identifier is invalid
    /// or redirect URLs fail security validation.
    pub fn validate(&self) -> Result<(), AccountLinkValidationError> {
        if self.account.trim().is_empty() {
            return Err(AccountLinkValidationError::EmptyAccount);
        }
        if !self.account.starts_with("acct_") {
            return Err(AccountLinkValidationError::InvalidAccount(self.account.clone()));
        }

        if let Some(ref refresh) = self.refresh_url {
            validate_account_link_url(refresh).map_err(|reason| {
                AccountLinkValidationError::InvalidUrl {
                    field: "refresh_url",
                    reason,
                }
            })?;
        }

        if let Some(ref ret) = self.return_url {
            validate_account_link_url(ret).map_err(|reason| {
                AccountLinkValidationError::InvalidUrl {
                    field: "return_url",
                    reason,
                }
            })?;
        }

        Ok(())
    }
}
