//! # Stripe Transfer & Transfer Reversal Domain Entities
//!
//! Strongly typed domain models for Stripe Transfers and Reversals,
//! adhering to ISO 4217 integer minor currency units (Gate G-Law)
//! and Rust zero-clone borrowing discipline (Gate G-Borrow).
//!
//! ## Overview
//!
//! In Stripe Connect multi-party payment architectures, transfers move funds from your
//! platform's Stripe balance to a connected account's Stripe balance (`POST /v1/transfers`).
//! When funds must be returned from the connected account back to the platform, transfer
//! reversals are created (`POST /v1/transfers/{id}/reversals`).
//!
//! - [`Transfer`]: Represents a transfer of funds to a connected account.
//! - [`TransferReversal`]: Represents an individual reversal of a transfer.
//! - [`TransferReversalList`]: Paginated list of transfer reversals.
//! - [`TransferList`]: Paginated list of transfers.
//! - [`TransferId`]: Strongly typed identifier for a transfer (`tr_...`).
//! - [`TransferReversalId`]: Strongly typed identifier for a transfer reversal (`trr_...`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Strongly typed identifier for a Stripe Transfer (`tr_...`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransferId(pub String);

impl std::fmt::Display for TransferId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Into<String>> From<T> for TransferId {
    fn from(s: T) -> Self {
        Self(s.into())
    }
}

impl std::ops::Deref for TransferId {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for TransferId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TransferId {
    /// Returns the inner string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Strongly typed identifier for a Stripe Transfer Reversal (`trr_...`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransferReversalId(pub String);

impl std::fmt::Display for TransferReversalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Into<String>> From<T> for TransferReversalId {
    fn from(s: T) -> Self {
        Self(s.into())
    }
}

impl std::ops::Deref for TransferReversalId {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for TransferReversalId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TransferReversalId {
    /// Returns the inner string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn default_transfer_object() -> String {
    "transfer".into()
}

fn default_transfer_reversal_object() -> String {
    "transfer_reversal".into()
}

fn default_list_object() -> String {
    "list".into()
}

/// Represents a Stripe Transfer of funds to a connected account.
///
/// See <https://docs.stripe.com/api/transfers/object>.
///
/// # Example
/// ```
/// use some_stripe_types::Transfer;
///
/// let json = r#"{
///     "id": "tr_1MoBy52eZvKYlo2C088a8d11",
///     "object": "transfer",
///     "amount": 1100,
///     "amount_reversed": 0,
///     "balance_transaction": "txn_1MoBy52eZvKYlo2C7q01452a",
///     "created": 1679000000,
///     "currency": "usd",
///     "destination": "acct_1032D82eZvKYlo2C",
///     "livemode": false,
///     "reversed": false
/// }"#;
///
/// let transfer: Transfer = serde_json::from_str(json).unwrap();
/// assert_eq!(transfer.id, "tr_1MoBy52eZvKYlo2C088a8d11");
/// assert_eq!(transfer.amount, 1100);
/// assert_eq!(transfer.remaining_amount(), 1100);
/// assert!(!transfer.is_reversed());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transfer {
    /// Unique identifier for the object (`tr_...`).
    pub id: String,

    /// String representing the object's type. Objects of the same type share the same value (`transfer`).
    #[serde(default = "default_transfer_object")]
    pub object: String,

    /// Amount in minor currency units (e.g., 100 cents for $1.00 or 100 for ¥100) to be transferred.
    /// Enforces integer arithmetic (Gate G-Law).
    pub amount: i64,

    /// Amount in minor currency units that has been reversed.
    #[serde(default)]
    pub amount_reversed: i64,

    /// Balance transaction that describes the impact of this transfer on your account balance (`txn_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance_transaction: Option<String>,

    /// Time that this record was created (UNIX epoch seconds).
    pub created: i64,

    /// Three-letter ISO currency code, in lowercase. Must be a supported currency.
    pub currency: String,

    /// An arbitrary string which you can attach to a Transfer object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// ID of the connected account that is the recipient of the transfer (`acct_...`).
    pub destination: String,

    /// If the destination is a Stripe account, this will be the ID of the payment that the destination account received for the transfer (`py_...` or `ch_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_payment: Option<String>,

    /// Has the value `true` if the object exists in live mode or `false` if the object exists in test mode.
    #[serde(default)]
    pub livemode: bool,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// A list of reversals that have been applied to the transfer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reversals: Option<TransferReversalList>,

    /// Whether the transfer has been fully reversed. If the transfer is only partially reversed, this attribute will still be false.
    #[serde(default)]
    pub reversed: bool,

    /// ID of the charge (or other transaction) that was used to fund the transfer (`ch_...` or `py_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_transaction: Option<String>,

    /// The source balance this transfer came from: `card`, `fpx`, or `bank_account`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,

    /// A string that identifies this transfer as part of a group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfer_group: Option<String>,
}

impl Transfer {
    /// Returns `true` if the transfer is fully reversed (either marked as `reversed = true`
    /// or `amount_reversed >= amount` when amount > 0).
    #[must_use]
    pub fn is_reversed(&self) -> bool {
        self.reversed || (self.amount > 0 && self.amount_reversed >= self.amount)
    }

    /// Returns `true` if the transfer has been partially reversed (some amount reversed, but not all).
    #[must_use]
    pub fn is_partially_reversed(&self) -> bool {
        self.amount_reversed > 0 && self.amount_reversed < self.amount
    }

    /// Returns the net remaining unreversed transfer amount in minor currency units (Gate G-Law).
    #[must_use]
    pub fn remaining_amount(&self) -> i64 {
        self.amount.saturating_sub(self.amount_reversed)
    }

    /// Returns the number of reversals attached to this transfer, if present.
    #[must_use]
    pub fn reversal_count(&self) -> usize {
        self.reversals
            .as_ref()
            .map(|r| r.data.len())
            .unwrap_or(0)
    }
}

/// Represents an individual reversal of a Stripe Transfer.
///
/// See <https://docs.stripe.com/api/transfer_reversals/object>.
///
/// # Example
/// ```
/// use some_stripe_types::TransferReversal;
///
/// let json = r#"{
///     "id": "trr_1MoBy62eZvKYlo2C1234abcd",
///     "object": "transfer_reversal",
///     "amount": 500,
///     "balance_transaction": "txn_1MoBy62eZvKYlo2C9876efgh",
///     "created": 1679000100,
///     "currency": "usd",
///     "transfer": "tr_1MoBy52eZvKYlo2C088a8d11"
/// }"#;
///
/// let reversal: TransferReversal = serde_json::from_str(json).unwrap();
/// assert_eq!(reversal.id, "trr_1MoBy62eZvKYlo2C1234abcd");
/// assert_eq!(reversal.amount, 500);
/// assert_eq!(reversal.transfer, "tr_1MoBy52eZvKYlo2C088a8d11");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferReversal {
    /// Unique identifier for the object (`trr_...`).
    pub id: String,

    /// String representing the object's type. Objects of the same type share the same value (`transfer_reversal`).
    #[serde(default = "default_transfer_reversal_object")]
    pub object: String,

    /// Amount, in minor currency units (e.g., cents), reversed.
    /// Enforces integer arithmetic (Gate G-Law).
    pub amount: i64,

    /// Balance transaction that describes the impact of this reversal on your account balance (`txn_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance_transaction: Option<String>,

    /// Time that this record was created (UNIX epoch seconds).
    pub created: i64,

    /// Three-letter ISO currency code, in lowercase. Must be a supported currency.
    pub currency: String,

    /// An arbitrary string which you can attach to a Reversal object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// ID of the refund created for the payment on the destination account (`pyr_...` or `re_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_payment_refund: Option<String>,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// ID of the refund on the parent charge that funded the transfer, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_refund: Option<String>,

    /// ID of the transfer that was reversed (`tr_...`).
    pub transfer: String,
}

/// Paginated list of [`TransferReversal`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferReversalList {
    /// String representing the object's type, always `list`.
    #[serde(default = "default_list_object")]
    pub object: String,

    /// Details about each object in the list.
    pub data: Vec<TransferReversal>,

    /// True if this list has another page of items after this one that can be fetched.
    pub has_more: bool,

    /// The URL where this list can be accessed.
    pub url: String,
}

/// Paginated list of [`Transfer`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferList {
    /// String representing the object's type, always `list`.
    #[serde(default = "default_list_object")]
    pub object: String,

    /// Details about each object in the list.
    pub data: Vec<Transfer>,

    /// True if this list has another page of items after this one that can be fetched.
    pub has_more: bool,

    /// The URL where this list can be accessed.
    pub url: String,
}
