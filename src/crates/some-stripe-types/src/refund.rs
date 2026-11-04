//! Stripe Refund domain models, request parameters, and statutory withdrawal schemas.
//!
//! # Legal Compliance (Gate G-Law)
//! - **EU Directive 2011/83/EU Article 13**: Statutory 14-day consumer withdrawal compliance
//!   mandating 100% full refund without penalty or deductions. When `amount` in [`CreateRefundParams`]
//!   is omitted (`None`), Stripe issues a 100% full reimbursement.
//! - **ISO 4217**: Monetary amounts are strictly integer minor currency units (e.g., cents, pence)
//!   represented as [`i64`]. Floating-point representations are strictly prohibited.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a Stripe Refund object (`re_...`).
///
/// Refers to funds returned to a customer from a previously settled Charge or PaymentIntent.
/// All monetary amounts use ISO 4217 minor currency units (integer cents).
///
/// # Example
/// ```
/// use some_stripe_types::Refund;
///
/// let json = r#"{
///     "id": "re_3MtwDemo123",
///     "object": "refund",
///     "amount": 2500,
///     "currency": "eur",
///     "status": "succeeded",
///     "payment_intent": "pi_3MtwDemo123"
/// }"#;
/// let refund: Refund = serde_json::from_str(json).unwrap();
/// assert_eq!(refund.id, "re_3MtwDemo123");
/// assert_eq!(refund.amount, 2500);
/// assert_eq!(refund.currency, "eur");
/// assert_eq!(refund.status, "succeeded");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Refund {
    /// Unique identifier for the refund (`re_...`).
    pub id: String,

    /// Object type, always `refund`.
    #[serde(default = "default_refund_object")]
    pub object: String,

    /// Amount refunded in ISO 4217 minor currency units (e.g. cents).
    pub amount: i64,

    /// Three-letter ISO currency code in lowercase.
    pub currency: String,

    /// Status of the refund: `pending`, `requires_action`, `succeeded`, `failed`, or `canceled`.
    pub status: String,

    /// ID of the Charge that was refunded, if applicable.
    pub charge: Option<String>,

    /// ID of the PaymentIntent that was refunded, if applicable.
    pub payment_intent: Option<String>,

    /// Reason for the refund, if provided (e.g. `duplicate`, `fraudulent`, `requested_by_customer`).
    pub reason: Option<String>,

    /// Transaction receipt number for the refund.
    pub receipt_number: Option<String>,

    /// An arbitrary string attached to the object (often for internal notes).
    pub description: Option<String>,

    /// If the refund failed, the reason for the failure.
    pub failure_reason: Option<String>,

    /// Set of key-value pairs attached to the refund object.
    pub metadata: Option<HashMap<String, String>>,

    /// Time at which the refund was created (UNIX epoch seconds).
    pub created: Option<i64>,
}

fn default_refund_object() -> String {
    "refund".into()
}

/// Request parameters for creating a new Stripe Refund (`POST /v1/refunds`).
///
/// # Statutory 14-Day Right of Withdrawal (EU Directive 2011/83/EU Article 13)
/// If `amount` is omitted (`None`), Stripe automatically refunds the full unrefunded amount
/// of the original charge or PaymentIntent, satisfying statutory 100% reimbursement requirements.
///
/// # Example
/// ```
/// use some_stripe_types::CreateRefundParams;
///
/// let params = CreateRefundParams {
///     payment_intent: Some("pi_12345".into()),
///     charge: None,
///     amount: Some(5000), // $50.00 in minor units
///     reason: Some("requested_by_customer".into()),
///     refund_application_fee: None,
///     reverse_transfer: None,
///     metadata: None,
///     idempotency_key: Some("idem_ref_abc".into()),
/// };
/// assert_eq!(params.amount, Some(5000));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateRefundParams {
    /// ID of the PaymentIntent to refund.
    pub payment_intent: Option<String>,

    /// ID of the Charge to refund.
    pub charge: Option<String>,

    /// Amount to refund in minor units. If omitted, the entire charge or intent is refunded
    /// in accordance with statutory consumer withdrawal rights.
    pub amount: Option<i64>,

    /// Reason for the refund (`duplicate`, `fraudulent`, or `requested_by_customer`).
    pub reason: Option<String>,

    /// Boolean indicating whether the application fee should be refunded when refunding an application fee.
    pub refund_application_fee: Option<bool>,

    /// Boolean indicating whether the transfer should be reversed when refunding a transfer.
    pub reverse_transfer: Option<bool>,

    /// Key-value metadata pairs attached to the refund.
    pub metadata: Option<HashMap<String, String>>,

    /// Unique idempotency key sent via the `Idempotency-Key` HTTP header.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

/// Request parameters for updating an existing Stripe Refund (`POST /v1/refunds/{id}`).
///
/// Allows updating metadata or notes associated with a refund.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateRefundParams {
    /// Set of key-value pairs to store with the refund object.
    pub metadata: Option<HashMap<String, String>>,

    /// Unique idempotency key sent via the `Idempotency-Key` HTTP header.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

/// Request parameters for canceling an existing Stripe Refund (`POST /v1/refunds/{id}/cancel`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CancelRefundParams {
    /// Unique idempotency key sent via the `Idempotency-Key` HTTP header.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}
