//! Five-status Stripe Refund lifecycle model and cancel endpoint descriptor (US-9.1.0 Unit 4).
//!
//! Implements Stripe's full 5-status Refund lifecycle (`pending`, `requires_action`, `succeeded`,
//! `failed`, `canceled`), structured `next_action` payloads for `requires_action` refunds, and a
//! validated endpoint descriptor builder for `POST /v1/refunds/{refund_id}/cancel` with strict
//! path-traversal guardrails.
//!
//! # Example
//!
//! ```
//! use some_stripe_types::refund_lifecycle::{RefundRecord, RefundStatus, cancel_refund};
//!
//! let payload = r#"{
//!     "id": "re_1N3T00LkdIwHu7ix",
//!     "amount": 2500,
//!     "currency": "usd",
//!     "status": "requires_action",
//!     "payment_intent": "pi_3MtwBwLkdIwHu7ix",
//!     "next_action": {
//!         "type": "display_details",
//!         "display_details": {
//!             "email_sent": { "email_sent_to": "customer@example.com" }
//!         }
//!     }
//! }"#;
//!
//! let record: RefundRecord = serde_json::from_str(payload).unwrap();
//! assert_eq!(record.status, RefundStatus::RequiresAction);
//! assert!(record.is_cancellable());
//!
//! let req = cancel_refund(&record.id).unwrap();
//! assert_eq!(req.method, "POST");
//! assert_eq!(req.path, "/v1/refunds/re_1N3T00LkdIwHu7ix/cancel");
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Canonical mapping from Stripe wire-format refund status slugs to [`RefundStatus`].
pub const KNOWN_REFUND_STATUSES: &[(&str, RefundStatus)] = &[
    ("pending", RefundStatus::Pending),
    ("requires_action", RefundStatus::RequiresAction),
    ("succeeded", RefundStatus::Succeeded),
    ("failed", RefundStatus::Failed),
    ("canceled", RefundStatus::Canceled),
];

/// Full 5-status lifecycle for a Stripe Refund resource.
///
/// # Example
///
/// ```
/// use some_stripe_types::refund_lifecycle::RefundStatus;
///
/// assert_eq!(
///     RefundStatus::from_slug("requires_action"),
///     Some(RefundStatus::RequiresAction)
/// );
/// assert_eq!(RefundStatus::RequiresAction.as_str(), "requires_action");
/// assert!(RefundStatus::RequiresAction.is_cancellable());
/// assert!(!RefundStatus::Succeeded.is_cancellable());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefundStatus {
    /// Refund has been created and is awaiting settlement.
    Pending,
    /// Customer or merchant action is required before the refund can complete.
    RequiresAction,
    /// Refund completed successfully.
    Succeeded,
    /// Refund failed; see `failure_reason` on [`RefundRecord`].
    Failed,
    /// Refund was canceled before completion.
    Canceled,
}

impl RefundStatus {
    /// Returns the canonical Stripe `snake_case` wire slug for this status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::RequiresAction => "requires_action",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }

    /// Parses a Stripe `snake_case` status slug into a [`RefundStatus`] using
    /// [`KNOWN_REFUND_STATUSES`].
    #[must_use]
    pub fn from_slug(slug: &str) -> Option<Self> {
        KNOWN_REFUND_STATUSES
            .iter()
            .find(|(name, _)| *name == slug)
            .map(|(_, status)| *status)
    }

    /// Returns `true` if a refund in this status can be canceled via Stripe's cancel endpoint.
    ///
    /// Stripe allows cancellation for refunds in [`RefundStatus::RequiresAction`] or
    /// [`RefundStatus::Pending`].
    #[must_use]
    pub const fn is_cancellable(&self) -> bool {
        matches!(self, Self::RequiresAction | Self::Pending)
    }
}

impl fmt::Display for RefundStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Structured `next_action` metadata returned on a `requires_action` Stripe Refund.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RefundNextAction {
    /// Type of next action required (serialized as `"type"` on the Stripe wire format).
    #[serde(rename = "type", alias = "action_type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_type: Option<String>,
    /// Arbitrary JSON details describing the required customer or merchant action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_details: Option<serde_json::Value>,
}

/// Deserializable representation of a Stripe Refund resource across all 5 lifecycle statuses.
///
/// Amounts are stored strictly as signed 64-bit integers in the currency's smallest unit
/// (zero floating-point arithmetic).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefundRecord {
    /// Unique Stripe refund identifier (`re_...`).
    pub id: String,
    /// Refund amount in minor currency units (e.g., cents).
    pub amount: i64,
    /// Three-letter lowercase ISO 4217 currency code.
    pub currency: String,
    /// Current lifecycle status of the refund.
    pub status: RefundStatus,
    /// Associated PaymentIntent identifier (`pi_...`), if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_intent: Option<String>,
    /// Associated Charge identifier (`ch_...` or `py_...`), if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charge: Option<String>,
    /// Reason supplied when creating the refund (e.g., `"requested_by_customer"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Explanation for why a refund is in [`RefundStatus::Pending`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_reason: Option<String>,
    /// Explanation for why a refund transitioned to [`RefundStatus::Failed`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    /// Action details when the refund is in [`RefundStatus::RequiresAction`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_action: Option<RefundNextAction>,
}

impl RefundRecord {
    /// Returns `true` when this refund's current status permits cancellation.
    #[must_use]
    pub const fn is_cancellable(&self) -> bool {
        self.status.is_cancellable()
    }
}

/// Errors returned when validating refund lifecycle operations and identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RefundLifecycleError {
    /// Refund identifier does not begin with Stripe's required `"re_"` prefix.
    #[error("invalid refund identifier: must start with 're_'")]
    InvalidPrefix,
    /// Refund identifier has no characters following the `"re_"` prefix.
    #[error("invalid refund identifier: suffix after 're_' must not be empty")]
    EmptySuffix,
    /// Refund identifier suffix contains a character outside ASCII alphanumeric and `'_'`.
    #[error("invalid refund identifier: disallowed character {0:?}")]
    InvalidCharacter(char),
}

/// Validated Stripe HTTP endpoint descriptor for cancelling a refund
/// (`POST /v1/refunds/{refund_id}/cancel`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CancelRefundRequest {
    /// Validated Stripe refund identifier (`re_...`).
    pub refund_id: String,
    /// HTTP method (`"POST"`).
    pub method: &'static str,
    /// Stripe API path (`/v1/refunds/{refund_id}/cancel`).
    pub path: String,
}

#[inline]
const fn is_valid_id_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn validate_refund_id(refund_id: &str) -> Result<(), RefundLifecycleError> {
    let suffix = refund_id
        .strip_prefix("re_")
        .ok_or(RefundLifecycleError::InvalidPrefix)?;
    if suffix.is_empty() {
        return Err(RefundLifecycleError::EmptySuffix);
    }
    if let Some(bad) = suffix.chars().find(|ch| !is_valid_id_char(*ch)) {
        return Err(RefundLifecycleError::InvalidCharacter(bad));
    }
    Ok(())
}

/// Validates `refund_id` and constructs the Stripe cancel-refund endpoint descriptor
/// (`POST /v1/refunds/{refund_id}/cancel`).
///
/// The identifier must begin with `"re_"` followed by a non-empty ASCII alphanumeric or
/// underscore suffix, blocking path traversal (`/`, `\`, `..`) and URL injection (`?`, `#`).
///
/// # Errors
///
/// Returns [`RefundLifecycleError`] if `refund_id` lacks the `"re_"` prefix, has an empty
/// suffix, or contains any character outside `[A-Za-z0-9_]`.
///
/// # Example
///
/// ```
/// use some_stripe_types::refund_lifecycle::{RefundLifecycleError, cancel_refund};
///
/// let req = cancel_refund("re_1N3T00LkdIwHu7ix").unwrap();
/// assert_eq!(req.method, "POST");
/// assert_eq!(req.path, "/v1/refunds/re_1N3T00LkdIwHu7ix/cancel");
///
/// assert_eq!(
///     cancel_refund("re_../admin"),
///     Err(RefundLifecycleError::InvalidCharacter('.'))
/// );
/// ```
pub fn cancel_refund(refund_id: &str) -> Result<CancelRefundRequest, RefundLifecycleError> {
    validate_refund_id(refund_id)?;
    Ok(CancelRefundRequest {
        refund_id: refund_id.to_owned(),
        method: "POST",
        path: format!("/v1/refunds/{refund_id}/cancel"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tc_9_1_4_all_five_statuses_deserialize_and_roundtrip() {
        let cases = [
            ("pending", RefundStatus::Pending),
            ("requires_action", RefundStatus::RequiresAction),
            ("succeeded", RefundStatus::Succeeded),
            ("failed", RefundStatus::Failed),
            ("canceled", RefundStatus::Canceled),
        ];

        assert_eq!(KNOWN_REFUND_STATUSES.len(), 5);
        for (slug, expected) in cases {
            let quoted = format!("\"{slug}\"");
            let parsed: RefundStatus = serde_json::from_str(&quoted).unwrap();
            assert_eq!(parsed, expected);
            assert_eq!(serde_json::to_string(&parsed).unwrap(), quoted);
            assert_eq!(parsed.as_str(), slug);
            assert_eq!(parsed.to_string(), slug);
            assert_eq!(RefundStatus::from_slug(slug), Some(expected));
        }

        assert_eq!(RefundStatus::from_slug("unknown_status"), None);
        assert_eq!(RefundStatus::from_slug(""), None);
        assert!(serde_json::from_str::<RefundStatus>("\"invalid\"").is_err());
    }

    #[test]
    fn tc_9_1_4_cancellable_statuses() {
        assert!(RefundStatus::Pending.is_cancellable());
        assert!(RefundStatus::RequiresAction.is_cancellable());
        assert!(!RefundStatus::Succeeded.is_cancellable());
        assert!(!RefundStatus::Failed.is_cancellable());
        assert!(!RefundStatus::Canceled.is_cancellable());
    }

    #[test]
    fn tc_9_1_4_refund_record_with_pending_reason() {
        let json = r#"{
            "id": "re_pending_001",
            "amount": 1500,
            "currency": "eur",
            "status": "pending",
            "payment_intent": "pi_001",
            "charge": "ch_001",
            "reason": "requested_by_customer",
            "pending_reason": "processing"
        }"#;

        let record: RefundRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.id, "re_pending_001");
        assert_eq!(record.amount, 1500);
        assert_eq!(record.currency, "eur");
        assert_eq!(record.status, RefundStatus::Pending);
        assert_eq!(record.payment_intent.as_deref(), Some("pi_001"));
        assert_eq!(record.charge.as_deref(), Some("ch_001"));
        assert_eq!(record.reason.as_deref(), Some("requested_by_customer"));
        assert_eq!(record.pending_reason.as_deref(), Some("processing"));
        assert!(record.failure_reason.is_none());
        assert!(record.next_action.is_none());
        assert!(record.is_cancellable());
    }

    #[test]
    fn tc_9_1_4_refund_record_with_requires_action_and_next_action() {
        let json = r#"{
            "id": "re_action_002",
            "amount": 4200,
            "currency": "usd",
            "status": "requires_action",
            "next_action": {
                "type": "display_details",
                "display_details": {
                    "email_sent": {
                        "email_sent_at": 1700000000,
                        "email_sent_to": "buyer@example.com"
                    },
                    "expires_at": 1700604800
                }
            }
        }"#;

        let record: RefundRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.status, RefundStatus::RequiresAction);
        assert!(record.is_cancellable());

        let next = record.next_action.as_ref().unwrap();
        assert_eq!(next.action_type.as_deref(), Some("display_details"));
        let details = next.display_details.as_ref().unwrap();
        assert_eq!(details["expires_at"], 1_700_604_800);
    }

    #[test]
    fn tc_9_1_4_refund_record_with_failed_and_failure_reason() {
        let json = r#"{
            "id": "re_failed_003",
            "amount": 999,
            "currency": "gbp",
            "status": "failed",
            "failure_reason": "expired_or_canceled_card"
        }"#;

        let record: RefundRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.status, RefundStatus::Failed);
        assert_eq!(
            record.failure_reason.as_deref(),
            Some("expired_or_canceled_card")
        );
        assert!(!record.is_cancellable());
    }

    #[test]
    fn tc_9_1_4_refund_record_succeeded_and_canceled() {
        let succeeded_json = r#"{
            "id": "re_ok_004",
            "amount": 5000,
            "currency": "jpy",
            "status": "succeeded"
        }"#;
        let succeeded: RefundRecord = serde_json::from_str(succeeded_json).unwrap();
        assert_eq!(succeeded.status, RefundStatus::Succeeded);
        assert!(!succeeded.is_cancellable());

        let canceled_json = r#"{
            "id": "re_cancel_005",
            "amount": 5000,
            "currency": "jpy",
            "status": "canceled"
        }"#;
        let canceled: RefundRecord = serde_json::from_str(canceled_json).unwrap();
        assert_eq!(canceled.status, RefundStatus::Canceled);
        assert!(!canceled.is_cancellable());
    }

    #[test]
    fn tc_9_1_4_cancel_refund_valid_ids() {
        let req = cancel_refund("re_1N3T00LkdIwHu7ix").unwrap();
        assert_eq!(req.refund_id, "re_1N3T00LkdIwHu7ix");
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/v1/refunds/re_1N3T00LkdIwHu7ix/cancel");

        let underscore_req = cancel_refund("re_test_2026_01").unwrap();
        assert_eq!(underscore_req.method, "POST");
        assert_eq!(underscore_req.path, "/v1/refunds/re_test_2026_01/cancel");
    }

    #[test]
    fn tc_9_1_4_cancel_refund_rejects_invalid_prefix_and_empty_suffix() {
        assert_eq!(cancel_refund(""), Err(RefundLifecycleError::InvalidPrefix));
        assert_eq!(cancel_refund("pi_12345"), Err(RefundLifecycleError::InvalidPrefix));
        assert_eq!(cancel_refund("ch_12345"), Err(RefundLifecycleError::InvalidPrefix));
        assert_eq!(cancel_refund("re_"), Err(RefundLifecycleError::EmptySuffix));
    }

    #[test]
    fn tc_9_1_4_cancel_refund_rejects_path_traversal_and_special_chars() {
        let bad_cases = [
            ("re_123/cancel", '/'),
            ("re_123/../admin", '/'),
            ("re_..secret", '.'),
            ("re_123\\evil", '\\'),
            ("re_123?expand=charge", '?'),
            ("re_123#fragment", '#'),
            ("re_123%2f456", '%'),
            ("re_123 456", ' '),
        ];

        for (input, expected_char) in bad_cases {
            assert_eq!(
                cancel_refund(input),
                Err(RefundLifecycleError::InvalidCharacter(expected_char)),
                "expected rejection for input {input:?}"
            );
        }
    }
}
