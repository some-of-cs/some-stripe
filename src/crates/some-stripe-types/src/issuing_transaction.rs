//! Stripe Issuing Transactions domain models, parameters, and ledger verification.
//!
//! Provides domain types for cardholder transaction ledger entries resulting from captures
//! or refunds on Stripe Issuing commercial payment cards (`/v1/issuing/transactions`).
//!
//! # Ledger Invariant & Type System
//!
//! In accordance with the Stripe Issuing Ledger Model:
//! - [`IssuingTransactionType::Capture`] represents funds leaving the cardholder balance (debit).
//! - [`IssuingTransactionType::Refund`] represents funds returning to the cardholder balance (credit).

use crate::issuing_authorization::AuthorizationMerchantData;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_transaction_object() -> String {
    "issuing.transaction".to_string()
}

fn default_list_object() -> String {
    "list".to_string()
}

/// The nature of the transaction (`capture` or `refund`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssuingTransactionType {
    /// A purchase capture debiting cardholder funds.
    Capture,
    /// A merchant refund crediting cardholder funds.
    Refund,
}

impl std::fmt::Display for IssuingTransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capture => write!(f, "capture"),
            Self::Refund => write!(f, "refund"),
        }
    }
}

/// Detailed breakdown of specific components of the transaction amount.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuingTransactionAmountDetails {
    /// ATM withdrawal fee included in the amount, in minor currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub atm_fee: Option<i64>,

    /// Cashback amount requested by cardholder, in minor currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cashback_amount: Option<i64>,
}

/// Additional merchant purchase details for receipt breakdown and travel data.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuingPurchaseDetails {
    /// Flight purchase details if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flight: Option<serde_json::Value>,

    /// Fuel purchase breakdown if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuel: Option<serde_json::Value>,

    /// Lodging reservation details if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lodging: Option<serde_json::Value>,

    /// Line item receipts associated with this transaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<Vec<serde_json::Value>>,

    /// Merchant-provided reference string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

/// A Stripe Issuing Transaction object (`issuing.transaction`).
///
/// An `IssuingTransaction` represents any use of an issued card that results in funds
/// entering or leaving the cardholder balance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuingTransaction {
    /// Unique identifier for the transaction (`ipi_...`).
    pub id: String,

    /// String representing the object's type, always `issuing.transaction`.
    #[serde(default = "default_transaction_object")]
    pub object: String,

    /// Transaction amount in minor currency units (e.g. cents).
    pub amount: i64,

    /// Detailed breakdown of amount components.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_details: Option<IssuingTransactionAmountDetails>,

    /// Associated [`IssuingAuthorization`](crate::IssuingAuthorization) ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization: Option<String>,

    /// Associated [`BalanceTransaction`](crate::BalanceTransaction) ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance_transaction: Option<String>,

    /// The ID of the card used for this transaction (`ic_...`).
    pub card: String,

    /// The ID of the cardholder associated with this transaction (`ich_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cardholder: Option<String>,

    /// Time at which the transaction was created (seconds since Unix epoch).
    pub created: i64,

    /// Three-letter ISO currency code (lowercase).
    pub currency: String,

    /// Associated dispute ID if this transaction was disputed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispute: Option<String>,

    /// True if the transaction exists in live mode, false in test mode.
    pub livemode: bool,

    /// The amount the merchant will receive in `merchant_currency`.
    pub merchant_amount: i64,

    /// The currency presented by the merchant.
    pub merchant_currency: String,

    /// Details about the seller/merchant.
    pub merchant_data: AuthorizationMerchantData,

    /// Set of key-value pairs for custom metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Additional purchase receipt and travel details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purchase_details: Option<IssuingPurchaseDetails>,

    /// The type of transaction (`capture` or `refund`).
    pub r#type: IssuingTransactionType,

    /// Digital wallet used (e.g. `apple_pay`, `google_pay`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wallet: Option<String>,
}

/// Parameters for updating an [`IssuingTransaction`] (`POST /v1/issuing/transactions/{id}`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateIssuingTransactionParams {
    /// Key-value metadata to attach to the transaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Unique idempotency key for safe execution.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl UpdateIssuingTransactionParams {
    /// Creates a new empty `UpdateIssuingTransactionParams`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the metadata key-value map.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Adds a single metadata entry.
    #[must_use]
    pub fn with_metadata_entry(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let map = self.metadata.get_or_insert_with(HashMap::new);
        map.insert(key.into(), value.into());
        self
    }

    /// Sets the idempotency key for this request.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for listing [`IssuingTransaction`] objects (`GET /v1/issuing/transactions`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListIssuingTransactionsParams {
    /// Only return transactions associated with this card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<String>,

    /// Only return transactions associated with this cardholder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cardholder: Option<String>,

    /// Exact creation timestamp filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,

    /// Only return transactions with the specified type (`capture` or `refund`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<IssuingTransactionType>,

    /// Limit on the number of objects to return (between 1 and 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,

    /// Cursor for forward pagination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// Cursor for backward pagination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListIssuingTransactionsParams {
    /// Creates a new empty `ListIssuingTransactionsParams`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Filters by card ID.
    #[must_use]
    pub fn with_card(mut self, card: impl Into<String>) -> Self {
        self.card = Some(card.into());
        self
    }

    /// Filters by cardholder ID.
    #[must_use]
    pub fn with_cardholder(mut self, cardholder: impl Into<String>) -> Self {
        self.cardholder = Some(cardholder.into());
        self
    }

    /// Filters by transaction type.
    #[must_use]
    pub fn with_type(mut self, r#type: IssuingTransactionType) -> Self {
        self.r#type = Some(r#type);
        self
    }

    /// Sets pagination limit.
    #[must_use]
    pub fn with_limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets cursor for forward pagination.
    #[must_use]
    pub fn with_starting_after(mut self, id: impl Into<String>) -> Self {
        self.starting_after = Some(id.into());
        self
    }

    /// Sets cursor for backward pagination.
    #[must_use]
    pub fn with_ending_before(mut self, id: impl Into<String>) -> Self {
        self.ending_before = Some(id.into());
        self
    }
}

/// Paginated list of [`IssuingTransaction`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuingTransactionList {
    /// String representing the object's type, always `list`.
    #[serde(default = "default_list_object")]
    pub object: String,

    /// Details about each object in the list.
    pub data: Vec<IssuingTransaction>,

    /// True if this list has another page of items after this one.
    pub has_more: bool,

    /// The URL where this list can be accessed.
    pub url: String,
}

/// Validates that an issuing transaction amount is mathematically consistent with its type.
///
/// In the Stripe Issuing ledger:
/// - Captures are debits with positive minor integer amounts ($> 0$).
/// - Refunds are credits with positive minor integer amounts ($> 0$).
///
/// # Errors
///
/// Returns an error message if the amount is non-positive or exceeds maximum allowable ledger limits.
pub fn validate_issuing_transaction_amount(
    amount: i64,
    _transaction_type: IssuingTransactionType,
) -> Result<(), &'static str> {
    if amount <= 0 {
        return Err("Issuing transaction amount must be a positive integer in minor units");
    }
    // Safeguard against unrealistic integer overflow (> $100,000,000.00)
    if amount > 10_000_000_000 {
        return Err("Issuing transaction amount exceeds maximum allowable ledger threshold");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_deserialization() {
        let json = r#"{
            "id": "ipi_test_123",
            "object": "issuing.transaction",
            "amount": 1000,
            "amount_details": {
                "atm_fee": null,
                "cashback_amount": null
            },
            "authorization": "iauth_auth_999",
            "balance_transaction": "txn_bal_888",
            "card": "ic_card_777",
            "cardholder": "ich_holder_666",
            "created": 1582200000,
            "currency": "usd",
            "dispute": null,
            "livemode": false,
            "merchant_amount": 1000,
            "merchant_currency": "usd",
            "merchant_data": {
                "name": "Grocery Store",
                "category": "grocery_stores",
                "category_code": "5411",
                "city": "San Francisco",
                "country": "US",
                "postal_code": "94107",
                "state": "CA"
            },
            "type": "capture",
            "wallet": "apple_pay"
        }"#;

        let txn: IssuingTransaction = serde_json::from_str(json).unwrap();
        assert_eq!(txn.id, "ipi_test_123");
        assert_eq!(txn.amount, 1000);
        assert_eq!(txn.r#type, IssuingTransactionType::Capture);
        assert_eq!(txn.authorization.as_deref(), Some("iauth_auth_999"));
        assert_eq!(txn.card, "ic_card_777");
        assert_eq!(txn.merchant_data.city.as_deref(), Some("San Francisco"));
        assert_eq!(txn.wallet.as_deref(), Some("apple_pay"));
    }

    #[test]
    fn test_update_issuing_transaction_params() {
        let params = UpdateIssuingTransactionParams::new()
            .with_metadata_entry("project", "travel_expense")
            .with_idempotency_key("idem_txn_update_1");

        assert_eq!(
            params.metadata.as_ref().unwrap().get("project").unwrap(),
            "travel_expense"
        );
        assert_eq!(params.idempotency_key.as_deref(), Some("idem_txn_update_1"));
    }

    #[test]
    fn test_list_issuing_transactions_params() {
        let params = ListIssuingTransactionsParams::new()
            .with_card("ic_card_1")
            .with_cardholder("ich_holder_1")
            .with_type(IssuingTransactionType::Refund)
            .with_limit(50)
            .with_ending_before("ipi_100");

        assert_eq!(params.card.as_deref(), Some("ic_card_1"));
        assert_eq!(params.cardholder.as_deref(), Some("ich_holder_1"));
        assert_eq!(params.r#type, Some(IssuingTransactionType::Refund));
        assert_eq!(params.limit, Some(50));
        assert_eq!(params.ending_before.as_deref(), Some("ipi_100"));
    }

    #[test]
    fn test_validate_issuing_transaction_amount() {
        assert!(validate_issuing_transaction_amount(500, IssuingTransactionType::Capture).is_ok());
        assert!(validate_issuing_transaction_amount(2500, IssuingTransactionType::Refund).is_ok());

        assert!(validate_issuing_transaction_amount(0, IssuingTransactionType::Capture).is_err());
        assert!(validate_issuing_transaction_amount(-100, IssuingTransactionType::Refund).is_err());
        assert!(
            validate_issuing_transaction_amount(99_000_000_000, IssuingTransactionType::Capture)
                .is_err()
        );
    }
}
