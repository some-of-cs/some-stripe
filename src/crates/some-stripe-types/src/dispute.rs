//! Stripe Dispute domain models, evidence schemas, and defense parameters.
//!
//! # Legal & Financial Compliance (Gate G-Law)
//! - Dispute amounts are tracked strictly as integer ISO 4217 minor currency units ([`i64`]).
//! - Card scheme compliance requires structured evidence submission within strict statutory deadlines.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a Stripe Dispute object (`dp_...`).
///
/// A dispute occurs when a customer questions your charge with their card issuer or bank.
///
/// # Example
/// ```
/// use some_stripe_types::Dispute;
///
/// let json = r#"{
///     "id": "dp_1MtwDemo456",
///     "object": "dispute",
///     "amount": 10000,
///     "currency": "usd",
///     "status": "needs_response",
///     "reason": "fraudulent"
/// }"#;
/// let dispute: Dispute = serde_json::from_str(json).unwrap();
/// assert_eq!(dispute.id, "dp_1MtwDemo456");
/// assert_eq!(dispute.amount, 10000);
/// assert_eq!(dispute.currency, "usd");
/// assert_eq!(dispute.status, "needs_response");
/// assert_eq!(dispute.reason.as_deref(), Some("fraudulent"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dispute {
    /// Unique identifier for the dispute (`dp_...`).
    pub id: String,

    /// Object type, always `dispute`.
    #[serde(default = "default_dispute_object")]
    pub object: String,

    /// Disputed amount in minor currency units (e.g. cents).
    pub amount: i64,

    /// Three-letter ISO currency code in lowercase.
    pub currency: String,

    /// Current status of the dispute: `warning_needs_response`, `warning_under_review`,
    /// `warning_closed`, `needs_response`, `under_review`, `charge_refunded`, `won`, or `lost`.
    pub status: String,

    /// Reason given by cardholder for dispute (e.g. `fraudulent`, `general`, `product_not_received`).
    pub reason: Option<String>,

    /// ID of the Charge that was disputed.
    pub charge: Option<String>,

    /// ID of the PaymentIntent that was disputed, if applicable.
    pub payment_intent: Option<String>,

    /// Evidence provided to refute or substantiate the dispute.
    pub evidence: Option<DisputeEvidence>,

    /// Information about the dispute's evidence submission details.
    pub evidence_details: Option<DisputeEvidenceDetails>,

    /// Whether the charge has been refunded.
    pub is_charge_refundable: Option<bool>,

    /// Has the value `true` if the object exists in live mode or the value `false` if the object exists in test mode.
    pub livemode: Option<bool>,

    /// Set of key-value pairs attached to the dispute object.
    pub metadata: Option<HashMap<String, String>>,

    /// Time at which the dispute was created (UNIX epoch seconds).
    pub created: Option<i64>,
}

fn default_dispute_object() -> String {
    "dispute".into()
}

/// Evidence submitted to defend against a chargeback or dispute.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisputeEvidence {
    /// The name of the customer who placed the order.
    pub customer_name: Option<String>,

    /// The customer's email address.
    pub customer_email_address: Option<String>,

    /// The billing address provided by the customer.
    pub billing_address: Option<String>,

    /// The shipping address where the product was delivered.
    pub shipping_address: Option<String>,

    /// Documentation showing proof of delivery.
    pub shipping_documentation: Option<String>,

    /// Description of the product or service provided.
    pub product_description: Option<String>,

    /// Uncategorized text evidence submitted to the issuer.
    pub uncategorized_text: Option<String>,
}

/// Metadata and deadline details regarding dispute evidence submission.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisputeEvidenceDetails {
    /// Date by which evidence must be submitted in UNIX epoch seconds.
    pub due_by: Option<i64>,

    /// Whether evidence has been submitted.
    pub has_evidence: Option<bool>,

    /// Whether the dispute has passed its submission deadline.
    pub past_due: Option<bool>,

    /// The number of times evidence has been submitted.
    pub submission_count: Option<u32>,
}

/// Request parameters for updating or submitting evidence for a Dispute (`POST /v1/disputes/{id}`).
///
/// # Example
/// ```
/// use some_stripe_types::{DisputeEvidenceParams, UpdateDisputeParams};
///
/// let params = UpdateDisputeParams {
///     evidence: Some(DisputeEvidenceParams {
///         customer_name: Some("Jane Doe".into()),
///         customer_email_address: Some("jane@example.com".into()),
///         product_description: Some("Software subscription".into()),
///         shipping_documentation: None,
///         billing_address: None,
///         shipping_address: None,
///         uncategorized_text: None,
///     }),
///     metadata: None,
///     submit: Some(true),
///     idempotency_key: Some("idem_disp_999".into()),
/// };
/// assert_eq!(params.submit, Some(true));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateDisputeParams {
    /// Evidence to upload in defense of the dispute.
    pub evidence: Option<DisputeEvidenceParams>,

    /// Key-value metadata pairs attached to the dispute.
    pub metadata: Option<HashMap<String, String>>,

    /// Whether to immediately submit evidence to the bank. When `Some(true)`, evidence is finalized.
    pub submit: Option<bool>,

    /// Unique idempotency key sent via the `Idempotency-Key` HTTP header.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

/// Parameters for evidence fields submitted in [`UpdateDisputeParams`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisputeEvidenceParams {
    /// The name of the customer who placed the order.
    pub customer_name: Option<String>,

    /// The customer's email address.
    pub customer_email_address: Option<String>,

    /// The billing address provided by the customer.
    pub billing_address: Option<String>,

    /// The shipping address where the product was delivered.
    pub shipping_address: Option<String>,

    /// Documentation showing proof of delivery.
    pub shipping_documentation: Option<String>,

    /// Description of the product or service provided.
    pub product_description: Option<String>,

    /// Uncategorized text evidence submitted to the issuer.
    pub uncategorized_text: Option<String>,
}
