//! Structured [`LastPaymentError`] deserialization, `PaymentIntent` lifecycle
//! classification, and `CheckoutSession` cancel/expire guards (US-9.1.0 Unit 3).
//!
//! # Key Guarantees
//!
//! - **PII Redaction**: [`LastPaymentError`]'s [`fmt::Debug`] implementation masks
//!   the free-text `message` field as `"[REDACTED]"` whenever present while keeping
//!   structured diagnostic codes (`error_type`, `code`, `decline_code`,
//!   `advice_code`, `network_decline_code`) visible in logs.
//! - **CheckoutSession Void Safety**: Stripe rejects `PaymentIntent` cancellation
//!   for `CheckoutSession`-backed intents unless the intent is in `"requires_capture"`.
//!   [`can_cancel_checkout_pi`] and [`resolve_checkout_void_action`] enforce this
//!   invariant so callers expire the `CheckoutSession` in all other states.
//! - **Official 7-State Lifecycle**: [`PaymentIntentLifecycleState`] and
//!   [`KNOWN_PI_STATUSES`] model Stripe's 7 official `PaymentIntent` statuses
//!   without ever checking a nonexistent `"failed"` status; failed attempts are
//!   detected via [`is_failed_attempt`].
//!
//! # Example
//!
//! ```
//! use some_stripe_types::payment_error_detail::{
//!     CheckoutVoidAction, LastPaymentError, PaymentIntentLifecycleState, is_failed_attempt,
//!     resolve_checkout_void_action,
//! };
//!
//! let payload = r#"{
//!     "type": "card_error",
//!     "code": "authentication_required",
//!     "decline_code": "authentication_required",
//!     "message": "Your card was declined for Jane Doe."
//! }"#;
//!
//! let err: LastPaymentError = serde_json::from_str(payload).unwrap();
//! assert!(err.is_three_ds_failure());
//! assert!(err.is_card_decline());
//! assert!(is_failed_attempt("requires_payment_method", Some(&err)));
//!
//! let debug_output = format!("{err:?}");
//! assert!(debug_output.contains("[REDACTED]"));
//! assert!(!debug_output.contains("Jane Doe"));
//!
//! assert_eq!(
//!     resolve_checkout_void_action(Some("requires_capture")),
//!     CheckoutVoidAction::CancelPaymentIntent
//! );
//! assert_eq!(
//!     resolve_checkout_void_action(Some("requires_payment_method")),
//!     CheckoutVoidAction::ExpireCheckoutSession
//! );
//! assert_eq!(
//!     PaymentIntentLifecycleState::from_wire_status("requires_capture"),
//!     Some(PaymentIntentLifecycleState::RequiresCapture)
//! );
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};

/// Stripe `code` and `decline_code` values indicating a 3D Secure (3DS)
/// authentication failure or unfulfilled authentication challenge.
pub const THREE_DS_FAILURE_CODES: &[&str] = &[
    "authentication_required",
    "payment_intent_authentication_failure",
    "setup_intent_authentication_failure",
    "three_d_secure_failed",
    "three_d_secure_authentication_failed",
    "off_session_authentication_required",
];

/// Structured representation of Stripe's `last_payment_error` payload on a
/// `PaymentIntent`.
///
/// The custom [`fmt::Debug`] implementation redacts [`Self::message`] as
/// `"[REDACTED]"` when present so cardholder or issuer free-text never leaks
/// into structured logs.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastPaymentError {
    /// Stripe error category (wire key `"type"`, e.g. `"card_error"`).
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    /// Machine-readable Stripe error code (e.g. `"card_declined"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Issuer decline reason code (e.g. `"insufficient_funds"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decline_code: Option<String>,
    /// Issuer merchant advice code (e.g. `"try_again_later"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advice_code: Option<String>,
    /// Raw card-network decline code returned by the acquirer or scheme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_decline_code: Option<String>,
    /// Stripe documentation URL describing the error code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_url: Option<String>,
    /// Human-readable error message (redacted in [`fmt::Debug`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Request parameter associated with the error, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
}

impl fmt::Debug for LastPaymentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let redacted_message = self.message.as_ref().map(|_| "[REDACTED]");
        f.debug_struct("LastPaymentError")
            .field("error_type", &self.error_type)
            .field("code", &self.code)
            .field("decline_code", &self.decline_code)
            .field("advice_code", &self.advice_code)
            .field("network_decline_code", &self.network_decline_code)
            .field("doc_url", &self.doc_url)
            .field("message", &redacted_message)
            .field("param", &self.param)
            .finish()
    }
}

impl LastPaymentError {
    /// Returns `true` when either [`Self::code`] or [`Self::decline_code`]
    /// matches a known 3D Secure (3DS) authentication failure code in
    /// [`THREE_DS_FAILURE_CODES`].
    ///
    /// # Example
    ///
    /// ```
    /// use some_stripe_types::payment_error_detail::LastPaymentError;
    ///
    /// let err = LastPaymentError {
    ///     code: Some("payment_intent_authentication_failure".to_string()),
    ///     ..LastPaymentError::default()
    /// };
    /// assert!(err.is_three_ds_failure());
    /// ```
    #[must_use]
    pub fn is_three_ds_failure(&self) -> bool {
        let code_matches = self
            .code
            .as_deref()
            .is_some_and(|code| THREE_DS_FAILURE_CODES.contains(&code));
        let decline_matches = self
            .decline_code
            .as_deref()
            .is_some_and(|code| THREE_DS_FAILURE_CODES.contains(&code));
        code_matches || decline_matches
    }

    /// Returns `true` when [`Self::error_type`] is `Some("card_error")` or
    /// [`Self::decline_code`] is populated.
    ///
    /// # Example
    ///
    /// ```
    /// use some_stripe_types::payment_error_detail::LastPaymentError;
    ///
    /// let err = LastPaymentError {
    ///     error_type: Some("card_error".to_string()),
    ///     ..LastPaymentError::default()
    /// };
    /// assert!(err.is_card_decline());
    /// ```
    #[must_use]
    pub fn is_card_decline(&self) -> bool {
        self.error_type.as_deref() == Some("card_error") || self.decline_code.is_some()
    }
}

/// Safe voiding strategy for a `CheckoutSession`-backed `PaymentIntent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutVoidAction {
    /// Cancel the `PaymentIntent` directly (permitted only in `"requires_capture"`).
    CancelPaymentIntent,
    /// Expire the owning `CheckoutSession` instead of canceling the `PaymentIntent`.
    ExpireCheckoutSession,
}

/// Returns `true` if and only if `pi_status` is `"requires_capture"`.
///
/// Stripe forbids canceling a `PaymentIntent` created by a `CheckoutSession`
/// unless the intent has reached `"requires_capture"` (manual-capture
/// authorization hold). In every other state, the caller must expire the
/// `CheckoutSession` instead.
///
/// # Example
///
/// ```
/// use some_stripe_types::payment_error_detail::can_cancel_checkout_pi;
///
/// assert!(can_cancel_checkout_pi("requires_capture"));
/// assert!(!can_cancel_checkout_pi("requires_payment_method"));
/// assert!(!can_cancel_checkout_pi("requires_action"));
/// ```
#[must_use]
pub fn can_cancel_checkout_pi(pi_status: &str) -> bool {
    pi_status == "requires_capture"
}

/// Resolves whether a `CheckoutSession`-backed payment should be voided by
/// canceling the `PaymentIntent` or expiring the `CheckoutSession`.
///
/// Returns [`CheckoutVoidAction::CancelPaymentIntent`] only when `pi_status` is
/// `Some("requires_capture")`; otherwise returns
/// [`CheckoutVoidAction::ExpireCheckoutSession`].
///
/// # Example
///
/// ```
/// use some_stripe_types::payment_error_detail::{
///     CheckoutVoidAction, resolve_checkout_void_action,
/// };
///
/// assert_eq!(
///     resolve_checkout_void_action(Some("requires_capture")),
///     CheckoutVoidAction::CancelPaymentIntent
/// );
/// assert_eq!(
///     resolve_checkout_void_action(None),
///     CheckoutVoidAction::ExpireCheckoutSession
/// );
/// ```
#[must_use]
pub fn resolve_checkout_void_action(pi_status: Option<&str>) -> CheckoutVoidAction {
    if pi_status.is_some_and(can_cancel_checkout_pi) {
        CheckoutVoidAction::CancelPaymentIntent
    } else {
        CheckoutVoidAction::ExpireCheckoutSession
    }
}

/// Stripe's 7 official `PaymentIntent` lifecycle statuses.
///
/// Stripe does not define a `"failed"` `PaymentIntent` status; failed payment
/// attempts return to [`Self::RequiresPaymentMethod`] with `last_payment_error`
/// populated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentIntentLifecycleState {
    /// Awaiting a payment method (or returned here after a failed attempt).
    RequiresPaymentMethod,
    /// Payment method attached; awaiting confirmation.
    RequiresConfirmation,
    /// Requires customer action such as 3DS authentication or redirect.
    RequiresAction,
    /// Asynchronous payment processing is underway.
    Processing,
    /// Funds authorized; awaiting manual capture or cancellation.
    RequiresCapture,
    /// PaymentIntent has been canceled.
    Canceled,
    /// PaymentIntent has succeeded and funds are captured.
    Succeeded,
}

/// Static lookup table mapping Stripe's 7 official `PaymentIntent` wire status
/// strings to [`PaymentIntentLifecycleState`].
pub const KNOWN_PI_STATUSES: &[(&str, PaymentIntentLifecycleState)] = &[
    (
        "requires_payment_method",
        PaymentIntentLifecycleState::RequiresPaymentMethod,
    ),
    (
        "requires_confirmation",
        PaymentIntentLifecycleState::RequiresConfirmation,
    ),
    (
        "requires_action",
        PaymentIntentLifecycleState::RequiresAction,
    ),
    ("processing", PaymentIntentLifecycleState::Processing),
    (
        "requires_capture",
        PaymentIntentLifecycleState::RequiresCapture,
    ),
    ("canceled", PaymentIntentLifecycleState::Canceled),
    ("succeeded", PaymentIntentLifecycleState::Succeeded),
];

impl PaymentIntentLifecycleState {
    /// Parses a Stripe wire status string using [`KNOWN_PI_STATUSES`].
    ///
    /// Returns `None` for any unknown or non-official status string.
    ///
    /// # Example
    ///
    /// ```
    /// use some_stripe_types::payment_error_detail::PaymentIntentLifecycleState;
    ///
    /// assert_eq!(
    ///     PaymentIntentLifecycleState::from_wire_status("processing"),
    ///     Some(PaymentIntentLifecycleState::Processing)
    /// );
    /// assert_eq!(
    ///     PaymentIntentLifecycleState::from_wire_status("unknown_status"),
    ///     None
    /// );
    /// ```
    #[must_use]
    pub fn from_wire_status(status: &str) -> Option<Self> {
        KNOWN_PI_STATUSES
            .iter()
            .find(|(wire, _)| *wire == status)
            .map(|(_, state)| *state)
    }

    /// Returns the canonical Stripe wire status string for this lifecycle state.
    ///
    /// # Example
    ///
    /// ```
    /// use some_stripe_types::payment_error_detail::PaymentIntentLifecycleState;
    ///
    /// assert_eq!(
    ///     PaymentIntentLifecycleState::RequiresCapture.as_wire_status(),
    ///     "requires_capture"
    /// );
    /// ```
    #[must_use]
    pub fn as_wire_status(self) -> &'static str {
        KNOWN_PI_STATUSES
            .iter()
            .find(|(_, state)| *state == self)
            .map_or("requires_payment_method", |(wire, _)| *wire)
    }
}

impl fmt::Display for PaymentIntentLifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_wire_status())
    }
}

/// Returns `true` when a `PaymentIntent` represents a failed payment attempt:
/// its wire status is `"requires_payment_method"` and `last_payment_error` is
/// populated.
///
/// # Example
///
/// ```
/// use some_stripe_types::payment_error_detail::{LastPaymentError, is_failed_attempt};
///
/// let err = LastPaymentError {
///     error_type: Some("card_error".to_string()),
///     decline_code: Some("insufficient_funds".to_string()),
///     ..LastPaymentError::default()
/// };
///
/// assert!(is_failed_attempt("requires_payment_method", Some(&err)));
/// assert!(!is_failed_attempt("requires_payment_method", None));
/// assert!(!is_failed_attempt("succeeded", Some(&err)));
/// ```
#[must_use]
pub fn is_failed_attempt(status: &str, last_payment_error: Option<&LastPaymentError>) -> bool {
    status == "requires_payment_method" && last_payment_error.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_payment_error_deserializes_full_stripe_payload() {
        let json = r#"{
            "type": "card_error",
            "code": "card_declined",
            "decline_code": "insufficient_funds",
            "advice_code": "try_again_later",
            "network_decline_code": "51",
            "doc_url": "https://stripe.com/docs/error-codes/card-declined",
            "message": "Your card has insufficient funds.",
            "param": "payment_method"
        }"#;

        let err: LastPaymentError =
            serde_json::from_str(json).expect("valid Stripe last_payment_error payload");
        assert_eq!(err.error_type.as_deref(), Some("card_error"));
        assert_eq!(err.code.as_deref(), Some("card_declined"));
        assert_eq!(err.decline_code.as_deref(), Some("insufficient_funds"));
        assert_eq!(err.advice_code.as_deref(), Some("try_again_later"));
        assert_eq!(err.network_decline_code.as_deref(), Some("51"));
        assert_eq!(
            err.doc_url.as_deref(),
            Some("https://stripe.com/docs/error-codes/card-declined")
        );
        assert_eq!(
            err.message.as_deref(),
            Some("Your card has insufficient funds.")
        );
        assert_eq!(err.param.as_deref(), Some("payment_method"));
    }

    #[test]
    fn last_payment_error_deserializes_sparse_payload_with_defaults() {
        let err: LastPaymentError =
            serde_json::from_str("{}").expect("empty object should deserialize to defaults");
        assert_eq!(err, LastPaymentError::default());
        assert!(!err.is_three_ds_failure());
        assert!(!err.is_card_decline());
    }

    #[test]
    fn last_payment_error_debug_redacts_message_and_preserves_codes() {
        let err = LastPaymentError {
            error_type: Some("card_error".to_string()),
            code: Some("card_declined".to_string()),
            decline_code: Some("do_not_honor".to_string()),
            advice_code: Some("do_not_try_again".to_string()),
            network_decline_code: Some("05".to_string()),
            doc_url: Some("https://stripe.com/docs/error-codes/card-declined".to_string()),
            message: Some("Cardholder John Smith ending in 4242 was declined.".to_string()),
            param: Some("payment_method".to_string()),
        };

        let rendered = format!("{err:?}");
        assert!(rendered.contains("[REDACTED]"));
        assert!(!rendered.contains("John Smith"));
        assert!(!rendered.contains("4242"));
        assert!(rendered.contains("card_error"));
        assert!(rendered.contains("card_declined"));
        assert!(rendered.contains("do_not_honor"));
        assert!(rendered.contains("do_not_try_again"));
        assert!(rendered.contains("05"));
    }

    #[test]
    fn last_payment_error_debug_keeps_none_message_as_none() {
        let err = LastPaymentError {
            error_type: Some("invalid_request_error".to_string()),
            code: Some("parameter_missing".to_string()),
            ..LastPaymentError::default()
        };

        let rendered = format!("{err:?}");
        assert!(!rendered.contains("[REDACTED]"));
        assert!(rendered.contains("message: None"));
    }

    #[test]
    fn is_three_ds_failure_matches_code_and_decline_code() {
        for &code in THREE_DS_FAILURE_CODES {
            let by_code = LastPaymentError {
                code: Some(code.to_string()),
                ..LastPaymentError::default()
            };
            assert!(
                by_code.is_three_ds_failure(),
                "expected 3DS match for {code}"
            );

            let by_decline = LastPaymentError {
                decline_code: Some(code.to_string()),
                ..LastPaymentError::default()
            };
            assert!(
                by_decline.is_three_ds_failure(),
                "expected 3DS decline match for {code}"
            );
        }

        let non_3ds = LastPaymentError {
            error_type: Some("card_error".to_string()),
            code: Some("card_declined".to_string()),
            decline_code: Some("insufficient_funds".to_string()),
            ..LastPaymentError::default()
        };
        assert!(!non_3ds.is_three_ds_failure());
    }

    #[test]
    fn is_card_decline_matches_card_error_or_decline_code() {
        let by_type = LastPaymentError {
            error_type: Some("card_error".to_string()),
            ..LastPaymentError::default()
        };
        assert!(by_type.is_card_decline());

        let by_decline_code = LastPaymentError {
            error_type: Some("api_error".to_string()),
            decline_code: Some("generic_decline".to_string()),
            ..LastPaymentError::default()
        };
        assert!(by_decline_code.is_card_decline());

        let non_card = LastPaymentError {
            error_type: Some("invalid_request_error".to_string()),
            code: Some("parameter_invalid_empty".to_string()),
            ..LastPaymentError::default()
        };
        assert!(!non_card.is_card_decline());
    }

    #[test]
    fn can_cancel_checkout_pi_allows_only_requires_capture() {
        assert!(can_cancel_checkout_pi("requires_capture"));
        for &(status, _) in KNOWN_PI_STATUSES {
            if status != "requires_capture" {
                assert!(
                    !can_cancel_checkout_pi(status),
                    "CheckoutSession PI must not be canceled in status {status}"
                );
            }
        }
        assert!(!can_cancel_checkout_pi(""));
        assert!(!can_cancel_checkout_pi("unknown"));
    }

    #[test]
    fn resolve_checkout_void_action_selects_cancel_only_for_requires_capture() {
        assert_eq!(
            resolve_checkout_void_action(Some("requires_capture")),
            CheckoutVoidAction::CancelPaymentIntent
        );
        assert_eq!(
            resolve_checkout_void_action(Some("requires_payment_method")),
            CheckoutVoidAction::ExpireCheckoutSession
        );
        assert_eq!(
            resolve_checkout_void_action(Some("requires_confirmation")),
            CheckoutVoidAction::ExpireCheckoutSession
        );
        assert_eq!(
            resolve_checkout_void_action(Some("requires_action")),
            CheckoutVoidAction::ExpireCheckoutSession
        );
        assert_eq!(
            resolve_checkout_void_action(Some("processing")),
            CheckoutVoidAction::ExpireCheckoutSession
        );
        assert_eq!(
            resolve_checkout_void_action(Some("canceled")),
            CheckoutVoidAction::ExpireCheckoutSession
        );
        assert_eq!(
            resolve_checkout_void_action(Some("succeeded")),
            CheckoutVoidAction::ExpireCheckoutSession
        );
        assert_eq!(
            resolve_checkout_void_action(None),
            CheckoutVoidAction::ExpireCheckoutSession
        );
    }

    #[test]
    fn known_pi_statuses_maps_all_seven_official_states_and_rejects_non_official() {
        assert_eq!(KNOWN_PI_STATUSES.len(), 7);
        for &(wire, expected) in KNOWN_PI_STATUSES {
            assert_eq!(
                PaymentIntentLifecycleState::from_wire_status(wire),
                Some(expected)
            );
            assert_eq!(expected.as_wire_status(), wire);
            assert_eq!(expected.to_string(), wire);
        }
        assert_eq!(PaymentIntentLifecycleState::from_wire_status("failed"), None);
        assert_eq!(PaymentIntentLifecycleState::from_wire_status(""), None);
    }

    #[test]
    fn payment_intent_lifecycle_state_roundtrips_serde() {
        for &(wire, expected) in KNOWN_PI_STATUSES {
            let quoted = format!("\"{wire}\"");
            let parsed: PaymentIntentLifecycleState =
                serde_json::from_str(&quoted).expect("official status should deserialize");
            assert_eq!(parsed, expected);

            let serialized =
                serde_json::to_string(&expected).expect("lifecycle state should serialize");
            assert_eq!(serialized, quoted);
        }
    }

    #[test]
    fn is_failed_attempt_requires_payment_method_with_last_payment_error() {
        let err = LastPaymentError {
            error_type: Some("card_error".to_string()),
            code: Some("card_declined".to_string()),
            decline_code: Some("generic_decline".to_string()),
            ..LastPaymentError::default()
        };

        assert!(is_failed_attempt("requires_payment_method", Some(&err)));
        assert!(!is_failed_attempt("requires_payment_method", None));
        assert!(!is_failed_attempt("requires_action", Some(&err)));
        assert!(!is_failed_attempt("requires_capture", Some(&err)));
        assert!(!is_failed_attempt("processing", Some(&err)));
        assert!(!is_failed_attempt("canceled", Some(&err)));
        assert!(!is_failed_attempt("succeeded", Some(&err)));
    }
}
