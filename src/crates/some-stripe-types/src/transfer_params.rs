//! Request parameter schemas and builders for Stripe Transfers and Reversals API operations.
//!
//! Enforces ISO 4217 minor currency integer rules (Gate G-Law) and Rust zero-clone borrowing
//! discipline (Gate G-Borrow).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validation errors for Transfer parameters and minor currency operations (Gate G-Law & ISO 4217).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferValidationError {
    /// The specified amount is invalid; it must be a positive integer minor currency unit (> 0).
    InvalidAmount {
        /// The invalid amount supplied.
        amount: i64,
    },
    /// The specified currency is invalid; it must be a valid 3-letter ISO 4217 code.
    InvalidCurrency {
        /// The invalid currency string supplied.
        currency: String,
    },
    /// The destination account ID is invalid (must be non-empty and typically start with `acct_`).
    InvalidDestination {
        /// The invalid destination supplied.
        destination: String,
    },
    /// A general parameter validation failure.
    InvalidParameter {
        /// Field name.
        field: &'static str,
        /// Description of the error.
        reason: String,
    },
}

impl std::fmt::Display for TransferValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAmount { amount } => {
                write!(
                    f,
                    "Transfer amount must be a positive integer minor currency unit (> 0), got: {amount}"
                )
            }
            Self::InvalidCurrency { currency } => {
                write!(
                    f,
                    "Transfer currency must be a valid 3-letter ISO 4217 code, got: '{currency}'"
                )
            }
            Self::InvalidDestination { destination } => {
                write!(
                    f,
                    "Transfer destination must be a non-empty connected account ID (`acct_...`), got: '{destination}'"
                )
            }
            Self::InvalidParameter { field, reason } => {
                write!(f, "Invalid transfer parameter '{field}': {reason}")
            }
        }
    }
}

impl std::error::Error for TransferValidationError {}

/// Parameters for creating a new [`Transfer`](crate::Transfer) (`POST /v1/transfers`).
///
/// # Example
/// ```
/// use some_stripe_types::CreateTransferParams;
///
/// let params = CreateTransferParams::new(5000, "usd", "acct_1032D82eZvKYlo2C")
///     .with_description("Weekly payout")
///     .with_transfer_group("group_order_123");
///
/// assert!(params.validate().is_ok());
/// assert_eq!(params.amount, 5000);
/// assert_eq!(params.currency, "usd");
/// assert_eq!(params.destination, "acct_1032D82eZvKYlo2C");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTransferParams {
    /// A positive integer in minor currency units (e.g., cents) representing how much to transfer.
    pub amount: i64,

    /// 3-letter ISO code for currency in lowercase.
    pub currency: String,

    /// The ID of a connected Stripe account (`acct_...`).
    pub destination: String,

    /// An arbitrary string which you can attach to a Transfer object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// You can use this parameter to transfer funds from a charge before they are added to your available balance (`ch_...` or `py_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_transaction: Option<String>,

    /// The source balance this transfer should come from: `card`, `fpx`, or `bank_account`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,

    /// A string that identifies this transfer as part of a group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfer_group: Option<String>,

    /// Unique idempotency key for safe execution (Gate G2).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateTransferParams {
    /// Creates a new `CreateTransferParams` with required fields.
    #[must_use]
    pub fn new(amount: i64, currency: impl Into<String>, destination: impl Into<String>) -> Self {
        Self {
            amount,
            currency: currency.into().to_lowercase(),
            destination: destination.into(),
            description: None,
            metadata: None,
            source_transaction: None,
            source_type: None,
            transfer_group: None,
            idempotency_key: None,
        }
    }

    /// Sets the optional description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the optional description (alias for [`with_description`](Self::with_description)).
    #[must_use]
    pub fn description(self, description: impl Into<String>) -> Self {
        self.with_description(description)
    }

    /// Sets the optional metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets the optional metadata (alias for [`with_metadata`](Self::with_metadata)).
    #[must_use]
    pub fn metadata(self, metadata: HashMap<String, String>) -> Self {
        self.with_metadata(metadata)
    }

    /// Sets the optional source transaction ID.
    #[must_use]
    pub fn with_source_transaction(mut self, tx: impl Into<String>) -> Self {
        self.source_transaction = Some(tx.into());
        self
    }

    /// Sets the optional source transaction ID (alias for [`with_source_transaction`](Self::with_source_transaction)).
    #[must_use]
    pub fn source_transaction(self, tx: impl Into<String>) -> Self {
        self.with_source_transaction(tx)
    }

    /// Sets the optional source type.
    #[must_use]
    pub fn with_source_type(mut self, source_type: impl Into<String>) -> Self {
        self.source_type = Some(source_type.into());
        self
    }

    /// Sets the optional source type (alias for [`with_source_type`](Self::with_source_type)).
    #[must_use]
    pub fn source_type(self, source_type: impl Into<String>) -> Self {
        self.with_source_type(source_type)
    }

    /// Sets the optional transfer group.
    #[must_use]
    pub fn with_transfer_group(mut self, transfer_group: impl Into<String>) -> Self {
        self.transfer_group = Some(transfer_group.into());
        self
    }

    /// Sets the optional transfer group (alias for [`with_transfer_group`](Self::with_transfer_group)).
    #[must_use]
    pub fn transfer_group(self, transfer_group: impl Into<String>) -> Self {
        self.with_transfer_group(transfer_group)
    }

    /// Sets the idempotency key for this request.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Sets the idempotency key for this request (alias for [`with_idempotency_key`](Self::with_idempotency_key)).
    #[must_use]
    pub fn idempotency_key(self, key: impl Into<String>) -> Self {
        self.with_idempotency_key(key)
    }

    /// Validates parameters against ISO 4217 and Stripe statutory requirements (Gate G-Law).
    pub fn validate(&self) -> Result<(), TransferValidationError> {
        if self.amount <= 0 {
            return Err(TransferValidationError::InvalidAmount {
                amount: self.amount,
            });
        }
        let cur = self.currency.trim();
        if cur.len() != 3 || !cur.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(TransferValidationError::InvalidCurrency {
                currency: self.currency.clone(),
            });
        }
        let dest = self.destination.trim();
        if dest.is_empty() {
            return Err(TransferValidationError::InvalidDestination {
                destination: self.destination.clone(),
            });
        }
        Ok(())
    }
}

/// Parameters for updating an existing [`Transfer`](crate::Transfer) (`POST /v1/transfers/{id}`).
///
/// # Example
/// ```
/// use some_stripe_types::UpdateTransferParams;
/// use std::collections::HashMap;
///
/// let mut meta = HashMap::new();
/// meta.insert("department".into(), "engineering".into());
///
/// let params = UpdateTransferParams::new()
///     .with_description("Updated payout description")
///     .with_metadata(meta);
///
/// assert_eq!(params.description.as_deref(), Some("Updated payout description"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateTransferParams {
    /// An arbitrary string which you can attach to a Transfer object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Unique idempotency key for safe execution (Gate G2).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl UpdateTransferParams {
    /// Creates a new empty `UpdateTransferParams`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the optional description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the optional description (alias for [`with_description`](Self::with_description)).
    #[must_use]
    pub fn description(self, description: impl Into<String>) -> Self {
        self.with_description(description)
    }

    /// Sets the optional metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets the optional metadata (alias for [`with_metadata`](Self::with_metadata)).
    #[must_use]
    pub fn metadata(self, metadata: HashMap<String, String>) -> Self {
        self.with_metadata(metadata)
    }

    /// Sets the idempotency key for this request.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Sets the idempotency key for this request (alias for [`with_idempotency_key`](Self::with_idempotency_key)).
    #[must_use]
    pub fn idempotency_key(self, key: impl Into<String>) -> Self {
        self.with_idempotency_key(key)
    }

    /// Validates parameters.
    pub fn validate(&self) -> Result<(), TransferValidationError> {
        Ok(())
    }
}

/// Parameters for creating a [`TransferReversal`](crate::TransferReversal) (`POST /v1/transfers/{id}/reversals`).
///
/// # Example
/// ```
/// use some_stripe_types::CreateTransferReversalParams;
///
/// let params = CreateTransferReversalParams::new()
///     .with_amount(2500)
///     .with_description("Return unneeded advance")
///     .with_refund_application_fee(true);
///
/// assert!(params.validate().is_ok());
/// assert_eq!(params.amount, Some(2500));
/// assert_eq!(params.refund_application_fee, Some(true));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTransferReversalParams {
    /// A positive integer in minor currency units (e.g., cents) representing how much of this transfer to reverse.
    /// Can only reverse up to the unreversed amount approved for your account. If omitted, the full unreversed amount is reversed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i64>,

    /// An arbitrary string which you can attach to a Reversal object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Boolean indicating whether the application fee should be refunded when reversing this transfer.
    /// If a full transfer reversal is given, the full application fee will be refunded.
    /// Otherwise, the application fee will be refunded with an amount proportional to the amount being reversed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refund_application_fee: Option<bool>,

    /// Unique idempotency key for safe execution (Gate G2).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateTransferReversalParams {
    /// Creates a new empty `CreateTransferReversalParams`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the optional amount to reverse in minor currency units.
    #[must_use]
    pub fn with_amount(mut self, amount: i64) -> Self {
        self.amount = Some(amount);
        self
    }

    /// Sets the optional amount to reverse (alias for [`with_amount`](Self::with_amount)).
    #[must_use]
    pub fn amount(self, amount: i64) -> Self {
        self.with_amount(amount)
    }

    /// Sets the optional description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the optional description (alias for [`with_description`](Self::with_description)).
    #[must_use]
    pub fn description(self, description: impl Into<String>) -> Self {
        self.with_description(description)
    }

    /// Sets the optional metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets the optional metadata (alias for [`with_metadata`](Self::with_metadata)).
    #[must_use]
    pub fn metadata(self, metadata: HashMap<String, String>) -> Self {
        self.with_metadata(metadata)
    }

    /// Sets whether to refund the proportional application fee.
    #[must_use]
    pub fn with_refund_application_fee(mut self, refund: bool) -> Self {
        self.refund_application_fee = Some(refund);
        self
    }

    /// Sets whether to refund the proportional application fee (alias for [`with_refund_application_fee`](Self::with_refund_application_fee)).
    #[must_use]
    pub fn refund_application_fee(self, refund: bool) -> Self {
        self.with_refund_application_fee(refund)
    }

    /// Sets the idempotency key for this request.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Sets the idempotency key for this request (alias for [`with_idempotency_key`](Self::with_idempotency_key)).
    #[must_use]
    pub fn idempotency_key(self, key: impl Into<String>) -> Self {
        self.with_idempotency_key(key)
    }

    /// Validates parameters against ISO 4217 integer rules (Gate G-Law).
    pub fn validate(&self) -> Result<(), TransferValidationError> {
        if let Some(amount) = self.amount {
            if amount <= 0 {
                return Err(TransferValidationError::InvalidAmount { amount });
            }
        }
        Ok(())
    }
}

/// Parameters for listing [`Transfer`](crate::Transfer) objects (`GET /v1/transfers`).
///
/// # Example
/// ```
/// use some_stripe_types::ListTransfersParams;
///
/// let params = ListTransfersParams::new()
///     .with_destination("acct_1032D82eZvKYlo2C")
///     .with_transfer_group("group_abc_123")
///     .with_limit(25);
///
/// assert_eq!(params.destination.as_deref(), Some("acct_1032D82eZvKYlo2C"));
/// assert_eq!(params.limit, Some(25));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListTransfersParams {
    /// Only return transfers for the destination specified by this account ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,

    /// Only return transfers with the given transfer_group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfer_group: Option<String>,

    /// A limit on the number of objects to be returned. The default is 10, and maximum is 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// A cursor for use in pagination. `starting_after` is an object ID defining your place in the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// A cursor for use in pagination. `ending_before` is an object ID defining your place in the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListTransfersParams {
    /// Creates a new empty `ListTransfersParams`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the destination filter.
    #[must_use]
    pub fn with_destination(mut self, destination: impl Into<String>) -> Self {
        self.destination = Some(destination.into());
        self
    }

    /// Sets the destination filter (alias for [`with_destination`](Self::with_destination)).
    #[must_use]
    pub fn destination(self, destination: impl Into<String>) -> Self {
        self.with_destination(destination)
    }

    /// Sets the transfer group filter.
    #[must_use]
    pub fn with_transfer_group(mut self, transfer_group: impl Into<String>) -> Self {
        self.transfer_group = Some(transfer_group.into());
        self
    }

    /// Sets the transfer group filter (alias for [`with_transfer_group`](Self::with_transfer_group)).
    #[must_use]
    pub fn transfer_group(self, transfer_group: impl Into<String>) -> Self {
        self.with_transfer_group(transfer_group)
    }

    /// Sets the page limit.
    #[must_use]
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the page limit (alias for [`with_limit`](Self::with_limit)).
    #[must_use]
    pub fn limit(self, limit: u32) -> Self {
        self.with_limit(limit)
    }

    /// Sets the `starting_after` cursor.
    #[must_use]
    pub fn with_starting_after(mut self, id: impl Into<String>) -> Self {
        self.starting_after = Some(id.into());
        self
    }

    /// Sets the `starting_after` cursor (alias for [`with_starting_after`](Self::with_starting_after)).
    #[must_use]
    pub fn starting_after(self, id: impl Into<String>) -> Self {
        self.with_starting_after(id)
    }

    /// Sets the `ending_before` cursor.
    #[must_use]
    pub fn with_ending_before(mut self, id: impl Into<String>) -> Self {
        self.ending_before = Some(id.into());
        self
    }

    /// Sets the `ending_before` cursor (alias for [`with_ending_before`](Self::with_ending_before)).
    #[must_use]
    pub fn ending_before(self, id: impl Into<String>) -> Self {
        self.with_ending_before(id)
    }

    /// Validates list parameters.
    pub fn validate(&self) -> Result<(), TransferValidationError> {
        if let Some(limit) = self.limit {
            if limit == 0 || limit > 100 {
                return Err(TransferValidationError::InvalidParameter {
                    field: "limit",
                    reason: "Limit must be between 1 and 100".into(),
                });
            }
        }
        Ok(())
    }
}

/// Parameters for listing [`TransferReversal`](crate::TransferReversal) objects (`GET /v1/transfers/{id}/reversals`).
///
/// # Example
/// ```
/// use some_stripe_types::ListTransferReversalsParams;
///
/// let params = ListTransferReversalsParams::new()
///     .with_limit(15)
///     .with_starting_after("trr_prev_reversal");
///
/// assert_eq!(params.limit, Some(15));
/// assert_eq!(params.starting_after.as_deref(), Some("trr_prev_reversal"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListTransferReversalsParams {
    /// A limit on the number of objects to be returned. The default is 10, and maximum is 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// A cursor for use in pagination. `starting_after` is an object ID defining your place in the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// A cursor for use in pagination. `ending_before` is an object ID defining your place in the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListTransferReversalsParams {
    /// Creates a new empty `ListTransferReversalsParams`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the page limit.
    #[must_use]
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the page limit (alias for [`with_limit`](Self::with_limit)).
    #[must_use]
    pub fn limit(self, limit: u32) -> Self {
        self.with_limit(limit)
    }

    /// Sets the `starting_after` cursor.
    #[must_use]
    pub fn with_starting_after(mut self, id: impl Into<String>) -> Self {
        self.starting_after = Some(id.into());
        self
    }

    /// Sets the `starting_after` cursor (alias for [`with_starting_after`](Self::with_starting_after)).
    #[must_use]
    pub fn starting_after(self, id: impl Into<String>) -> Self {
        self.with_starting_after(id)
    }

    /// Sets the `ending_before` cursor.
    #[must_use]
    pub fn with_ending_before(mut self, id: impl Into<String>) -> Self {
        self.ending_before = Some(id.into());
        self
    }

    /// Sets the `ending_before` cursor (alias for [`with_ending_before`](Self::with_ending_before)).
    #[must_use]
    pub fn ending_before(self, id: impl Into<String>) -> Self {
        self.with_ending_before(id)
    }

    /// Validates list parameters.
    pub fn validate(&self) -> Result<(), TransferValidationError> {
        if let Some(limit) = self.limit {
            if limit == 0 || limit > 100 {
                return Err(TransferValidationError::InvalidParameter {
                    field: "limit",
                    reason: "Limit must be between 1 and 100".into(),
                });
            }
        }
        Ok(())
    }
}
