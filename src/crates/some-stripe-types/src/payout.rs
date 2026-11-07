//! # Stripe Payout Types & Lifecycle Models
//!
//! Strongly typed domain models, parameters, and lifecycle states for Stripe Payouts,
//! adhering to ISO 4217 integer minor currency units and zero-clone borrowing disciplines.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// Status of a payout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayoutStatus {
    /// Payout has been successfully paid out.
    Paid,
    /// Payout is pending.
    Pending,
    /// Payout is in transit.
    InTransit,
    /// Payout has been canceled.
    Canceled,
    /// Payout has failed.
    Failed,
}

impl PayoutStatus {
    /// Returns the static string slice representation for this status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Paid => "paid",
            Self::Pending => "pending",
            Self::InTransit => "in_transit",
            Self::Canceled => "canceled",
            Self::Failed => "failed",
        }
    }
}

impl std::fmt::Display for PayoutStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for PayoutStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for PayoutStatus {
    type Err = ParsePayoutStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "paid" => Ok(Self::Paid),
            "pending" => Ok(Self::Pending),
            "in_transit" => Ok(Self::InTransit),
            "canceled" => Ok(Self::Canceled),
            "failed" => Ok(Self::Failed),
            _ => Err(ParsePayoutStatusError(s.to_string())),
        }
    }
}

/// Error returned when parsing an unrecognized payout status string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePayoutStatusError(pub String);

impl std::fmt::Display for ParsePayoutStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown Stripe payout status: '{}'", self.0)
    }
}

impl std::error::Error for ParsePayoutStatusError {}

/// Method of a payout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayoutMethod {
    /// Standard payout method.
    Standard,
    /// Instant payout method.
    Instant,
}

impl PayoutMethod {
    /// Returns the static string slice representation for this method.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Instant => "instant",
        }
    }
}

impl std::fmt::Display for PayoutMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for PayoutMethod {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for PayoutMethod {
    type Err = ParsePayoutMethodError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "standard" => Ok(Self::Standard),
            "instant" => Ok(Self::Instant),
            _ => Err(ParsePayoutMethodError(s.to_string())),
        }
    }
}

/// Error returned when parsing an unrecognized payout method string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePayoutMethodError(pub String);

impl std::fmt::Display for ParsePayoutMethodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown Stripe payout method: '{}'", self.0)
    }
}

impl std::error::Error for ParsePayoutMethodError {}

/// Type of a payout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayoutType {
    /// Bank account payout.
    BankAccount,
    /// Card payout.
    Card,
}

impl PayoutType {
    /// Returns the static string slice representation for this type.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::BankAccount => "bank_account",
            Self::Card => "card",
        }
    }
}

impl std::fmt::Display for PayoutType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for PayoutType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for PayoutType {
    type Err = ParsePayoutTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bank_account" => Ok(Self::BankAccount),
            "card" => Ok(Self::Card),
            _ => Err(ParsePayoutTypeError(s.to_string())),
        }
    }
}

/// Error returned when parsing an unrecognized payout type string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePayoutTypeError(pub String);

impl std::fmt::Display for ParsePayoutTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown Stripe payout type: '{}'", self.0)
    }
}

impl std::error::Error for ParsePayoutTypeError {}

/// Unique identifier for a Stripe payout (`po_...`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PayoutId(pub String);

impl std::fmt::Display for PayoutId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for PayoutId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for PayoutId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for PayoutId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// A Payout object when you payout funds to your bank account or card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Payout {
    /// Unique identifier for the object.
    pub id: String,
    /// String representing the object's type. Objects of the same type share the same value.
    #[serde(default = "default_payout_object")]
    pub object: String,
    /// Amount to be paid out in minor currency units (e.g., cents).
    pub amount: i64,
    /// Date the payout is expected to arrive in the bank, in UTC seconds.
    pub arrival_date: i64,
    /// Returns true if the payout was created by a Stripe scheduled payout, false if it was created manually.
    pub automatic: bool,
    /// ID of the balance transaction that describes the impact of this payout on your account balance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance_transaction: Option<String>,
    /// Time at which the object was created in UTC seconds.
    pub created: i64,
    /// Three-letter ISO currency code, in lowercase.
    pub currency: String,
    /// An arbitrary string attached to the object. Often useful for displaying to users.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// ID of the bank account or card the payout was sent to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    /// If the payout fails or is canceled, this is the ID of the balance transaction that reverses the initial balance impact.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_balance_transaction: Option<String>,
    /// Error code explaining reason for payout failure if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_code: Option<String>,
    /// Message explaining reason for payout failure if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_message: Option<String>,
    /// Has the value true if the object exists in live mode or the value false if the object exists in test mode.
    pub livemode: bool,
    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// The method used to send this payout (`standard` or `instant`).
    pub method: PayoutMethod,
    /// If the payout is reversed, this is the ID of the original payout.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_payout: Option<String>,
    /// If the payout is reversed, this is the ID of the reversal payout.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reversed_by: Option<String>,
    /// The source event type that caused the payout (`bank_account`, `card`, or `alipay_account`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    /// Extra information about a payout to be displayed on the user's bank statement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_descriptor: Option<String>,
    /// Current status of the payout (`paid`, `pending`, `in_transit`, `canceled`, or `failed`).
    pub status: PayoutStatus,
    /// The type of payout (`bank_account` or `card`).
    #[serde(rename = "type")]
    pub type_: PayoutType,
}

fn default_payout_object() -> String {
    "payout".into()
}

/// A list wrapper for Stripe payouts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayoutList {
    /// String representing the object's type. Always has the value "list".
    pub object: String,
    /// Array of payouts.
    pub data: Vec<Payout>,
    /// True if there are more items available in this list.
    pub has_more: bool,
    /// URL for the list endpoint.
    pub url: String,
}

/// Parameters for creating a payout.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePayoutParams {
    /// The amount to pay out, represented in minor currency units.
    pub amount: i64,
    /// Three-letter ISO currency code, in lowercase.
    pub currency: String,
    /// An arbitrary string attached to the object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The ID of a bank account or a card to send the payout to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// The method to use for this payout (`standard` or `instant`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<PayoutMethod>,
    /// The source type to send this payout from (`bank_account`, `card`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    /// Extra information to display on the user's bank statement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_descriptor: Option<String>,
}

impl CreatePayoutParams {
    /// Creates a new `CreatePayoutParams` with required amount and currency.
    pub fn new(amount: i64, currency: impl Into<String>) -> Self {
        Self {
            amount,
            currency: currency.into(),
            description: None,
            destination: None,
            metadata: None,
            method: None,
            source_type: None,
            statement_descriptor: None,
        }
    }

    /// Sets the method for this payout.
    pub fn with_method(mut self, method: PayoutMethod) -> Self {
        self.method = Some(method);
        self
    }

    /// Sets the statement descriptor for this payout.
    pub fn with_statement_descriptor(mut self, desc: impl Into<String>) -> Self {
        self.statement_descriptor = Some(desc.into());
        self
    }
}

/// Parameters for updating a payout.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatePayoutParams {
    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

/// Parameters for listing payouts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListPayoutsParams {
    /// Only return payouts that have the given status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<PayoutStatus>,
    /// A limit on the number of objects to be returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// A cursor for use in pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,
    /// A cursor for use in pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}
