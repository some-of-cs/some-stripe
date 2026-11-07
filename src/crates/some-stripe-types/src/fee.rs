//! # Application Fees & Fee Refunds Domain Entities
//!
//! Strongly typed domain models, lists, and parameter structs for Stripe Application Fees and Fee Refunds.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Strongly typed ApplicationFee identifier (`fee_...`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApplicationFeeId(pub String);

impl From<String> for ApplicationFeeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ApplicationFeeId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for ApplicationFeeId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ApplicationFeeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Strongly typed FeeRefund identifier (`fr_...`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FeeRefundId(pub String);

impl From<String> for FeeRefundId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for FeeRefundId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for FeeRefundId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FeeRefundId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn default_fee_refund_object() -> String {
    "fee_refund".into()
}

fn default_application_fee_object() -> String {
    "application_fee".into()
}

/// A refund of an application fee (`fee_refund`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeeRefund {
    pub id: String,
    #[serde(default = "default_fee_refund_object")]
    pub object: String, // "fee_refund"
    pub amount: i64,
    pub balance_transaction: Option<String>,
    pub created: i64,
    pub currency: String,
    pub fee: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

/// A list container for application fee refunds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeeRefundList {
    pub object: String, // "list"
    pub data: Vec<FeeRefund>,
    pub has_more: bool,
    pub url: String,
}

/// An application fee (`application_fee`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationFee {
    pub id: String,
    #[serde(default = "default_application_fee_object")]
    pub object: String, // "application_fee"
    pub account: String,
    pub amount: i64,
    pub amount_refunded: i64,
    pub application: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance_transaction: Option<String>,
    pub charge: String,
    pub created: i64,
    pub currency: String,
    pub livemode: bool,
    pub refunded: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refunds: Option<FeeRefundList>,
}

/// A list container for application fees.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationFeeList {
    pub object: String, // "list"
    pub data: Vec<ApplicationFee>,
    pub has_more: bool,
    pub url: String,
}

/// Parameters for creating a fee refund (`CreateFeeRefundParams`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CreateFeeRefundParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

impl CreateFeeRefundParams {
    /// Creates a new default parameter block for fee refunds.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the refund amount.
    pub fn with_amount(mut self, amount: i64) -> Self {
        self.amount = Some(amount);
        self
    }

    /// Sets the idempotency key for safe request retries.
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for listing application fees (`ListApplicationFeesParams`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ListApplicationFeesParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charge: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

/// Parameters for listing fee refunds (`ListFeeRefundsParams`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ListFeeRefundsParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}
