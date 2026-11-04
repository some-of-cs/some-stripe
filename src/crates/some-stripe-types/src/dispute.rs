//! # Dispute and Evidence Domain Models
//!
//! Strongly typed domain models, schemas, and evidence submission locking logic for
//! Stripe disputes and chargebacks.
//!
//! ## Overview
//!
//! When a cardholder questions a payment with their card issuer, the card issuer initiates
//! a formal dispute or pre-dispute inquiry. Stripe represents these records via [`Dispute`].
//!
//! Key components:
//! - [`Dispute`]: Root dispute entity containing monetary amounts in ISO 4217 minor units (Gate G-Law),
//!   current [`DisputeStatus`], evidence, and deadline metadata.
//! - [`DisputeStatus`]: Formal dispute statuses (`warning_needs_response`, `warning_under_review`,
//!   `warning_closed`, `needs_response`, `under_review`, `charge_refunded`, `won`, `lost`).
//! - [`DisputeEvidence`]: Exhaustive 20-field evidence container covering customer communication,
//!   fulfillment proofs, service logs, duplicate charge records, and cancellation policies.
//! - [`DisputeEvidenceDetails`]: Evidence submission tracking, deadline timestamps (`due_by`),
//!   and past-due markers.
//! - [`UpdateDisputeParams`]: Request payload for uploading evidence or locking submissions
//!   via `submit: Option<bool>` and [`UpdateDisputeParams::is_submission_final`].
//!
//! ## Evidence Submission Locking Invariant
//!
//! Evidence submission becomes locked (immutable) under two conditions:
//! 1. The merchant explicitly finalizes the submission via [`UpdateDisputeParams`] with `submit: Some(true)`
//!    ([`UpdateDisputeParams::is_submission_final`] returning `true`).
//! 2. The dispute transitions out of the open response window into review or resolution
//!    ([`DisputeStatus::UnderReview`], `Won`, `Lost`, etc.), or the submission window expires
//!    ([`DisputeEvidenceDetails::past_due`] is `true`). Checked via [`Dispute::is_evidence_locked`].
//!
//! ## Example
//!
//! ```
//! use some_stripe_types::{Dispute, DisputeStatus, DisputeEvidence, UpdateDisputeParams};
//!
//! let params = UpdateDisputeParams {
//!     submit: Some(true),
//!     evidence: Some(DisputeEvidence {
//!         customer_name: Some("Jane Doe".to_string()),
//!         uncategorized_text: Some("Customer confirmed delivery via chat".to_string()),
//!         ..Default::default()
//!     }),
//!     ..Default::default()
//! };
//!
//! assert!(params.is_submission_final());
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// Strongly typed Stripe Dispute ID (`dp_...`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DisputeId(pub String);

impl fmt::Display for DisputeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Into<String>> From<T> for DisputeId {
    fn from(s: T) -> Self {
        Self(s.into())
    }
}

impl std::ops::Deref for DisputeId {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for DisputeId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Status of a Stripe Dispute throughout its lifecycle.
///
/// In accordance with Stripe's chargeback processing lifecycle:
/// - `warning_needs_response`: Pre-dispute inquiry or early fraud warning awaiting merchant response.
/// - `warning_under_review`: Pre-dispute inquiry evidence has been submitted and is under issuer review.
/// - `warning_closed`: Pre-dispute inquiry has been closed without escalating to a formal dispute.
/// - `needs_response`: Formal chargeback initiated; awaiting merchant evidence.
/// - `under_review`: Merchant has submitted evidence; awaiting issuer decision.
/// - `charge_refunded`: Dispute was resolved when the merchant accepted or refunded the charge.
/// - `won`: Card issuer ruled in favor of the merchant.
/// - `lost`: Card issuer ruled in favor of the cardholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisputeStatus {
    /// Early fraud warning or pre-dispute inquiry requiring merchant response.
    WarningNeedsResponse,
    /// Early fraud warning evidence submitted and currently under review.
    WarningUnderReview,
    /// Early fraud warning inquiry closed without escalating to formal chargeback.
    WarningClosed,
    /// Formal chargeback initiated; awaiting merchant response and evidence submission.
    NeedsResponse,
    /// Evidence submitted to the card network; awaiting issuer adjudication.
    UnderReview,
    /// Charge refunded by the merchant, closing the dispute.
    ChargeRefunded,
    /// Card issuer ruled in favor of the merchant; disputed funds returned.
    Won,
    /// Card issuer ruled in favor of the cardholder; dispute lost.
    Lost,
}

impl DisputeStatus {
    /// Returns the canonical Stripe API snake_case string for this status.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::WarningNeedsResponse => "warning_needs_response",
            Self::WarningUnderReview => "warning_under_review",
            Self::WarningClosed => "warning_closed",
            Self::NeedsResponse => "needs_response",
            Self::UnderReview => "under_review",
            Self::ChargeRefunded => "charge_refunded",
            Self::Won => "won",
            Self::Lost => "lost",
        }
    }

    /// Returns `true` if this status indicates that the dispute is open for merchant response.
    #[inline]
    pub const fn is_needs_response(&self) -> bool {
        matches!(self, Self::NeedsResponse | Self::WarningNeedsResponse)
    }

    /// Returns `true` if this status indicates that evidence submission is locked.
    #[inline]
    pub const fn is_evidence_locked(&self) -> bool {
        !self.is_needs_response()
    }
}

impl fmt::Display for DisputeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when parsing an unrecognized dispute status string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisputeStatusParseError(pub String);

impl fmt::Display for DisputeStatusParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid Stripe dispute status: '{}'", self.0)
    }
}

impl std::error::Error for DisputeStatusParseError {}

impl FromStr for DisputeStatus {
    type Err = DisputeStatusParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "warning_needs_response" => Ok(Self::WarningNeedsResponse),
            "warning_under_review" => Ok(Self::WarningUnderReview),
            "warning_closed" => Ok(Self::WarningClosed),
            "needs_response" => Ok(Self::NeedsResponse),
            "under_review" => Ok(Self::UnderReview),
            "charge_refunded" => Ok(Self::ChargeRefunded),
            "won" => Ok(Self::Won),
            "lost" => Ok(Self::Lost),
            other => Err(DisputeStatusParseError(other.to_string())),
        }
    }
}

/// Information about the evidence submission process, deadlines, and lock states.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisputeEvidenceDetails {
    /// Date by which evidence must be submitted in UNIX epoch seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_by: Option<i64>,

    /// Whether evidence has been submitted for this dispute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_evidence: Option<bool>,

    /// Whether the deadline for evidence submission has passed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub past_due: Option<bool>,

    /// Number of times evidence has been submitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submission_count: Option<u32>,
}

/// Exhaustive evidence container for defending against Stripe disputes.
///
/// Contains all 20 standard Stripe evidence fields submitted to card networks.
///
/// # Example
/// ```
/// use some_stripe_types::DisputeEvidence;
///
/// let evidence = DisputeEvidence {
///     customer_name: Some("Alice Smith".to_string()),
///     customer_email_address: Some("alice@example.com".to_string()),
///     shipping_tracking_number: Some("1Z9999999999999999".to_string()),
///     receipt: Some("file_receipt_123".to_string()),
///     ..Default::default()
/// };
///
/// assert_eq!(evidence.customer_name.as_deref(), Some("Alice Smith"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisputeEvidence {
    /// Any server or customer communication (such as email transcripts) showing customer agreement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer_communication: Option<String>,

    /// The email address of the customer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer_email_address: Option<String>,

    /// The name of the customer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer_name: Option<String>,

    /// The IP address from which the customer made the purchase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer_purchase_ip: Option<String>,

    /// The billing address provided by the customer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing_address: Option<String>,

    /// The address to which physical goods or merchandise were shipped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping_address: Option<String>,

    /// The date on which physical goods were shipped (e.g. `YYYY-MM-DD`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping_date: Option<String>,

    /// File ID of documentation showing proof of shipment or delivery (`file_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping_documentation: Option<String>,

    /// The tracking number for shipping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipping_tracking_number: Option<String>,

    /// The date on which service was provided (e.g. `YYYY-MM-DD`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_date: Option<String>,

    /// File ID of documentation showing proof that service was provided (`file_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_documentation: Option<String>,

    /// File ID of documentation explaining why a duplicate charge occurred (`file_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate_charge_documentation: Option<String>,

    /// Explanation of why a duplicate charge was valid or legitimate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate_charge_explanation: Option<String>,

    /// ID of the original charge that was duplicated (`ch_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate_charge_id: Option<String>,

    /// Justification for refusing a refund requested by the customer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refund_refusal_explanation: Option<String>,

    /// File ID showing the merchant cancellation policy presented to the customer (`file_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_policy: Option<String>,

    /// Explanation of how and when the customer was informed of the cancellation policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_policy_disclosure: Option<String>,

    /// File ID of the receipt or transaction invoice (`file_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<String>,

    /// File ID of any other uncategorized documentation supporting the merchant (`file_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncategorized_documentation: Option<String>,

    /// Any additional textual evidence or narrative supporting the merchant's claim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncategorized_text: Option<String>,
}

/// Represents a Stripe Dispute object (`dp_...`).
///
/// Encapsulates the disputed amount in integer minor units (Gate G-Law), dispute reason,
/// evidence details, and submission lock status.
///
/// # Example
/// ```
/// use some_stripe_types::{Dispute, DisputeStatus};
///
/// let json = r#"{
///     "id": "dp_1001",
///     "object": "dispute",
///     "amount": 4900,
///     "currency": "usd",
///     "status": "needs_response",
///     "reason": "fraudulent"
/// }"#;
///
/// let dispute: Dispute = serde_json::from_str(json).unwrap();
/// assert_eq!(dispute.id, "dp_1001");
/// assert_eq!(dispute.amount, 4900);
/// assert!(!dispute.is_evidence_locked());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dispute {
    /// Unique identifier for the dispute (`dp_...`).
    pub id: String,

    /// Object type, always `dispute`.
    #[serde(default = "default_dispute_object")]
    pub object: String,

    /// Disputed amount in minor currency units (e.g., cents).
    /// Enforces integer arithmetic (Gate G-Law).
    pub amount: i64,

    /// Three-letter ISO currency code in lowercase (e.g., `usd`, `eur`).
    pub currency: String,

    /// Current status of the dispute.
    pub status: DisputeStatus,

    /// Reason why the payment was disputed (e.g., `fraudulent`, `duplicate`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// ID of the charge that was disputed (`ch_...`), if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charge: Option<String>,

    /// ID of the PaymentIntent that was disputed (`pi_...`), if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_intent: Option<String>,

    /// Evidence submitted or collected for the dispute.
    #[serde(default)]
    pub evidence: DisputeEvidence,

    /// Information on the evidence submission process, deadlines, and lock state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_details: Option<DisputeEvidenceDetails>,

    /// Whether the charge can be refunded while the dispute is open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_charge_refundable: Option<bool>,

    /// Has the value `true` if the object exists in live mode or `false` in test mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub livemode: Option<bool>,

    /// Set of key-value pairs attached to this dispute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Time at which the object was created (UNIX epoch seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,

    /// List of zero or more balance transaction IDs associated with this dispute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance_transactions: Option<Vec<String>>,
}

fn default_dispute_object() -> String {
    "dispute".into()
}

impl Dispute {
    /// Returns `true` if evidence submission for this dispute is locked and cannot be updated.
    ///
    /// Evidence is locked when:
    /// - The dispute is in a final or non-response status (`under_review`, `warning_under_review`,
    ///   `charge_refunded`, `won`, `lost`, `warning_closed`).
    /// - Or the evidence submission window has expired (`past_due == Some(true)` in `evidence_details`).
    ///
    /// # Example
    /// ```
    /// use some_stripe_types::{Dispute, DisputeStatus, DisputeEvidenceDetails};
    ///
    /// let mut dispute = Dispute {
    ///     id: "dp_2002".into(),
    ///     object: "dispute".into(),
    ///     amount: 2500,
    ///     currency: "eur".into(),
    ///     status: DisputeStatus::NeedsResponse,
    ///     reason: None,
    ///     charge: None,
    ///     payment_intent: None,
    ///     evidence: Default::default(),
    ///     evidence_details: None,
    ///     is_charge_refundable: None,
    ///     livemode: None,
    ///     metadata: None,
    ///     created: None,
    ///     balance_transactions: None,
    /// };
    ///
    /// assert!(!dispute.is_evidence_locked());
    ///
    /// dispute.status = DisputeStatus::UnderReview;
    /// assert!(dispute.is_evidence_locked());
    ///
    /// dispute.status = DisputeStatus::NeedsResponse;
    /// dispute.evidence_details = Some(DisputeEvidenceDetails {
    ///     past_due: Some(true),
    ///     ..Default::default()
    /// });
    /// assert!(dispute.is_evidence_locked());
    /// ```
    #[inline]
    pub fn is_evidence_locked(&self) -> bool {
        if self
            .evidence_details
            .as_ref()
            .is_some_and(|details| details.past_due.unwrap_or(false))
        {
            return true;
        }
        self.status.is_evidence_locked()
    }
}

/// Parameters for updating a dispute or submitting evidence.
///
/// Supports submission locking via the `submit` flag.
///
/// # Example
/// ```
/// use some_stripe_types::{UpdateDisputeParams, DisputeEvidence};
///
/// let params = UpdateDisputeParams::new()
///     .with_evidence(DisputeEvidence {
///         customer_name: Some("Bob".to_string()),
///         ..Default::default()
///     })
///     .with_submit(true);
///
/// assert!(params.is_submission_final());
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateDisputeParams {
    /// Evidence to upload in response to the dispute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<DisputeEvidence>,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Whether to submit evidence immediately. If `true`, evidence is finalized
    /// and cannot be edited further.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submit: Option<bool>,

    /// Idempotency key for safe retries (passed via header, not body).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl UpdateDisputeParams {
    /// Creates an empty set of dispute update parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the evidence payload.
    pub fn with_evidence(mut self, evidence: DisputeEvidence) -> Self {
        self.evidence = Some(evidence);
        self
    }

    /// Sets the `submit` flag to lock/finalize evidence submission.
    pub fn with_submit(mut self, submit: bool) -> Self {
        self.submit = Some(submit);
        self
    }

    /// Sets the idempotency key for this update request.
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Returns `true` if this update request marks the evidence submission as final.
    ///
    /// When `submit` is `Some(true)`, evidence is formally submitted to the dispute
    /// resolution network and locked against subsequent modifications.
    #[inline]
    pub fn is_submission_final(&self) -> bool {
        self.submit.unwrap_or(false)
    }
}

/// Parameters for closing a dispute in the merchant's customer's favor.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CloseDisputeParams {
    /// Idempotency key for safe retries (passed via header, not body).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CloseDisputeParams {
    /// Creates a new empty close dispute parameters struct.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the idempotency key for this close request.
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for listing disputes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListDisputesParams {
    /// Only return disputes associated with the specified charge ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charge: Option<String>,

    /// Only return disputes associated with the specified PaymentIntent ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_intent: Option<String>,

    /// A cursor for pagination across multiple pages of results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// A cursor for pagination across multiple pages of results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,

    /// Limit on the number of objects returned (between 1 and 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl ListDisputesParams {
    /// Creates a new empty list disputes parameters struct.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispute_status_roundtrip_all_eight_variants() {
        let test_cases = [
            (
                DisputeStatus::WarningNeedsResponse,
                "warning_needs_response",
            ),
            (DisputeStatus::WarningUnderReview, "warning_under_review"),
            (DisputeStatus::WarningClosed, "warning_closed"),
            (DisputeStatus::NeedsResponse, "needs_response"),
            (DisputeStatus::UnderReview, "under_review"),
            (DisputeStatus::ChargeRefunded, "charge_refunded"),
            (DisputeStatus::Won, "won"),
            (DisputeStatus::Lost, "lost"),
        ];

        for (status, expected_str) in test_cases {
            assert_eq!(status.as_str(), expected_str);
            assert_eq!(status.to_string(), expected_str);

            let parsed: DisputeStatus = expected_str.parse().expect("Failed to parse status");
            assert_eq!(parsed, status);

            let serialized = serde_json::to_string(&status).expect("Serialization failed");
            assert_eq!(serialized, format!("\"{expected_str}\""));

            let deserialized: DisputeStatus =
                serde_json::from_str(&serialized).expect("Deserialization failed");
            assert_eq!(deserialized, status);
        }

        // Test invalid status parsing
        let invalid = "fraud_review".parse::<DisputeStatus>();
        assert!(invalid.is_err());
        assert_eq!(
            invalid.unwrap_err().to_string(),
            "Invalid Stripe dispute status: 'fraud_review'"
        );
    }

    #[test]
    fn test_dispute_evidence_all_20_fields_roundtrip() {
        let evidence = DisputeEvidence {
            customer_communication: Some("chat_transcript_123".into()),
            customer_email_address: Some("customer@example.com".into()),
            customer_name: Some("Jane Doe".into()),
            customer_purchase_ip: Some("192.0.2.1".into()),
            billing_address: Some("123 Main St, Springfield, IL".into()),
            shipping_address: Some("456 Elm St, Shelbyville, IL".into()),
            shipping_date: Some("2026-10-15".into()),
            shipping_documentation: Some("file_ship_proof_456".into()),
            shipping_tracking_number: Some("1Z999AA10123456784".into()),
            service_date: Some("2026-10-16".into()),
            service_documentation: Some("file_service_cert_789".into()),
            duplicate_charge_documentation: Some("file_dup_doc_101".into()),
            duplicate_charge_explanation: Some("Customer placed two distinct orders".into()),
            duplicate_charge_id: Some("ch_original_111".into()),
            refund_refusal_explanation: Some(
                "Item was custom manufactured to specification".into(),
            ),
            cancellation_policy: Some("file_policy_222".into()),
            cancellation_policy_disclosure: Some(
                "Explicit checkbox agreement at checkout step 2".into(),
            ),
            receipt: Some("file_receipt_333".into()),
            uncategorized_documentation: Some("file_extra_doc_444".into()),
            uncategorized_text: Some("Customer verified receipt via SMS confirmation".into()),
        };

        let json = serde_json::to_string(&evidence).expect("Evidence serialization failed");
        let deserialized: DisputeEvidence =
            serde_json::from_str(&json).expect("Evidence deserialization failed");

        assert_eq!(evidence, deserialized);
        assert_eq!(deserialized.customer_name.as_deref(), Some("Jane Doe"));
        assert_eq!(
            deserialized.shipping_tracking_number.as_deref(),
            Some("1Z999AA10123456784")
        );
        assert_eq!(
            deserialized.uncategorized_text.as_deref(),
            Some("Customer verified receipt via SMS confirmation")
        );
    }

    #[test]
    fn test_dispute_evidence_empty_defaults() {
        let evidence = DisputeEvidence::default();
        let json = serde_json::to_string(&evidence).expect("Serialization failed");
        assert_eq!(json, "{}");

        let deserialized: DisputeEvidence =
            serde_json::from_str("{}").expect("Deserialization failed");
        assert_eq!(deserialized, DisputeEvidence::default());
    }

    #[test]
    fn test_update_dispute_params_submission_locking() {
        let p_none = UpdateDisputeParams::new();
        assert!(!p_none.is_submission_final());

        let p_false = UpdateDisputeParams::new().with_submit(false);
        assert!(!p_false.is_submission_final());

        let p_true = UpdateDisputeParams::new()
            .with_submit(true)
            .with_idempotency_key("idem_evidence_001")
            .with_evidence(DisputeEvidence {
                customer_name: Some("Alice".into()),
                ..Default::default()
            });

        assert!(p_true.is_submission_final());
        assert_eq!(p_true.idempotency_key.as_deref(), Some("idem_evidence_001"));
        assert_eq!(
            p_true
                .evidence
                .as_ref()
                .and_then(|e| e.customer_name.as_deref()),
            Some("Alice")
        );

        // Check that idempotency key is skipped from JSON serialization
        let json = serde_json::to_string(&p_true).expect("Serialization failed");
        assert!(!json.contains("idempotency_key"));
        assert!(json.contains("\"submit\":true"));
    }

    #[test]
    fn test_dispute_is_evidence_locked_status_transitions() {
        let mut dispute = Dispute {
            id: "dp_test_123".into(),
            object: "dispute".into(),
            amount: 5000,
            currency: "usd".into(),
            status: DisputeStatus::NeedsResponse,
            reason: Some("fraudulent".into()),
            charge: Some("ch_charge_123".into()),
            payment_intent: Some("pi_intent_123".into()),
            evidence: DisputeEvidence::default(),
            evidence_details: None,
            is_charge_refundable: Some(false),
            livemode: Some(false),
            metadata: None,
            created: Some(1728345600),
            balance_transactions: Some(vec!["txn_123".into()]),
        };

        // NeedsResponse and WarningNeedsResponse are open
        assert!(!dispute.is_evidence_locked());

        dispute.status = DisputeStatus::WarningNeedsResponse;
        assert!(!dispute.is_evidence_locked());

        // All other statuses lock evidence
        let locked_statuses = [
            DisputeStatus::WarningUnderReview,
            DisputeStatus::WarningClosed,
            DisputeStatus::UnderReview,
            DisputeStatus::ChargeRefunded,
            DisputeStatus::Won,
            DisputeStatus::Lost,
        ];

        for status in locked_statuses {
            dispute.status = status;
            assert!(
                dispute.is_evidence_locked(),
                "Status {status:?} must be locked"
            );
        }
    }

    #[test]
    fn test_dispute_is_evidence_locked_past_due_override() {
        let mut dispute = Dispute {
            id: "dp_past_due_test".into(),
            object: "dispute".into(),
            amount: 7500,
            currency: "eur".into(),
            status: DisputeStatus::NeedsResponse,
            reason: Some("general".into()),
            charge: None,
            payment_intent: None,
            evidence: DisputeEvidence::default(),
            evidence_details: Some(DisputeEvidenceDetails {
                due_by: Some(1728300000),
                has_evidence: Some(false),
                past_due: Some(false),
                submission_count: Some(0),
            }),
            is_charge_refundable: None,
            livemode: None,
            metadata: None,
            created: None,
            balance_transactions: None,
        };

        // When past_due is false, still open
        assert!(!dispute.is_evidence_locked());

        // When past_due becomes true, evidence is locked even if status is still needs_response
        dispute.evidence_details = Some(DisputeEvidenceDetails {
            due_by: Some(1728300000),
            has_evidence: Some(false),
            past_due: Some(true),
            submission_count: Some(0),
        });
        assert!(dispute.is_evidence_locked());
    }

    #[test]
    fn test_dispute_json_deserialization_contract() {
        let json = serde_json::json!({
            "id": "dp_fixture_999",
            "object": "dispute",
            "amount": 12500,
            "currency": "usd",
            "status": "under_review",
            "reason": "product_not_received",
            "charge": "ch_999_test",
            "evidence": {
                "customer_name": "Carol Danvers",
                "customer_email_address": "carol@avengers.com",
                "shipping_tracking_number": "TRK999888"
            },
            "evidence_details": {
                "due_by": 1735689600,
                "has_evidence": true,
                "past_due": false,
                "submission_count": 1
            },
            "is_charge_refundable": false,
            "livemode": false
        });

        let dispute: Dispute = serde_json::from_value(json).expect("Deserialization failed");
        assert_eq!(dispute.id, "dp_fixture_999");
        assert_eq!(dispute.amount, 12500);
        assert_eq!(dispute.status, DisputeStatus::UnderReview);
        assert!(dispute.is_evidence_locked());
        assert_eq!(
            dispute.evidence.customer_name.as_deref(),
            Some("Carol Danvers")
        );
        let details = dispute.evidence_details.expect("Expected evidence_details");
        assert_eq!(details.submission_count, Some(1));
        assert_eq!(details.has_evidence, Some(true));
        assert_eq!(details.past_due, Some(false));
    }

    #[test]
    fn test_dispute_id_newtype() {
        let id = DisputeId::from("dp_test_abc123");
        assert_eq!(id.to_string(), "dp_test_abc123");
        assert_eq!(&*id, "dp_test_abc123");
        assert_eq!(id.as_ref(), "dp_test_abc123");

        let ser = serde_json::to_string(&id).unwrap();
        assert_eq!(ser, "\"dp_test_abc123\"");
        let de: DisputeId = serde_json::from_str(&ser).unwrap();
        assert_eq!(de, id);
    }
}
