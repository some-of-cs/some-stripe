//! # Stripe Refund Types & Statutory Compliance
//!
//! Strongly typed domain models, parameters, and statutory legal validators for Stripe Refunds,
//! adhering to EU Directive 2011/83/EU Article 13 consumer withdrawal rules and ISO 4217 integer minor currency units.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// The statutory withdrawal window duration in seconds under EU Directive 2011/83/EU Article 13
/// (14 calendar days = 14 * 86,400 seconds = 1,209,600 seconds).
pub const STATUTORY_WITHDRAWAL_WINDOW_SECONDS: i64 = 14 * 86_400;

/// Standard Stripe refund reasons.
///
/// Indicates the reason why a refund was initiated.
///
/// # Example
/// ```
/// use some_stripe_types::RefundReason;
/// use std::str::FromStr;
///
/// let reason = RefundReason::RequestedByCustomer;
/// assert_eq!(reason.as_str(), "requested_by_customer");
/// assert_eq!(reason.to_string(), "requested_by_customer");
///
/// let parsed = RefundReason::from_str("fraudulent").unwrap();
/// assert_eq!(parsed, RefundReason::Fraudulent);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefundReason {
    /// The charge was a duplicate.
    Duplicate,
    /// The charge was deemed fraudulent.
    Fraudulent,
    /// The refund was requested by the customer (e.g. exercising statutory withdrawal rights).
    RequestedByCustomer,
    /// The charge was an uncaptured authorization that has expired.
    ExpiredUncapturedCharge,
}

impl RefundReason {
    /// Returns the static string slice representation for this reason code.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Duplicate => "duplicate",
            Self::Fraudulent => "fraudulent",
            Self::RequestedByCustomer => "requested_by_customer",
            Self::ExpiredUncapturedCharge => "expired_uncaptured_charge",
        }
    }
}

impl std::fmt::Display for RefundReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for RefundReason {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for RefundReason {
    type Err = ParseRefundReasonError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "duplicate" => Ok(Self::Duplicate),
            "fraudulent" => Ok(Self::Fraudulent),
            "requested_by_customer" => Ok(Self::RequestedByCustomer),
            "expired_uncaptured_charge" => Ok(Self::ExpiredUncapturedCharge),
            _ => Err(ParseRefundReasonError(s.to_string())),
        }
    }
}

impl From<RefundReason> for &'static str {
    fn from(reason: RefundReason) -> Self {
        reason.as_str()
    }
}

impl From<RefundReason> for String {
    fn from(reason: RefundReason) -> Self {
        reason.as_str().to_string()
    }
}

/// Error returned when parsing an unrecognized refund reason string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseRefundReasonError(pub String);

impl std::fmt::Display for ParseRefundReasonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown Stripe refund reason: '{}'", self.0)
    }
}

impl std::error::Error for ParseRefundReasonError {}

/// Represents a Stripe Refund object.
///
/// Stripe refunds allow you to refund a charge that has previously been created but not yet refunded.
/// Funds will be refunded to the payment method that was originally charged.
///
/// Under EU Directive 2011/83/EU Article 13 (Gate G-Law), consumers exercising their statutory
/// 14-day right of withdrawal are entitled to a full refund without deductions or penalty fees.
///
/// # Example
/// ```
/// use some_stripe_types::Refund;
///
/// let json = r#"{
///     "id": "re_3Mtw123",
///     "object": "refund",
///     "amount": 2000,
///     "currency": "usd",
///     "status": "succeeded"
/// }"#;
/// let refund: Refund = serde_json::from_str(json).unwrap();
/// assert_eq!(refund.id, "re_3Mtw123");
/// assert_eq!(refund.amount, 2000);
/// assert_eq!(refund.currency, "usd");
/// assert_eq!(refund.status, "succeeded");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refund {
    /// Unique identifier for the refund (`re_...`).
    pub id: String,

    /// Object type, always `refund`.
    #[serde(default = "default_refund_object")]
    pub object: String,

    /// Amount in minor currency units (e.g., 2000 for $20.00 / €20.00) conforming to ISO 4217 integer arithmetic.
    pub amount: i64,

    /// ID of the charge that was refunded (`ch_...`), if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charge: Option<String>,

    /// Three-letter ISO currency code in lowercase (e.g., `usd`, `eur`).
    pub currency: String,

    /// An arbitrary string attached to the object, often useful for displaying to users.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// If the refund failed, the ID for the balance transaction that was created to balance the refund failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_balance_transaction: Option<String>,

    /// If the refund failed, the reason for refund failure if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// ID of the PaymentIntent that was refunded (`pi_...`), if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_intent: Option<String>,

    /// Reason for the refund, if provided (`duplicate`, `fraudulent`, `requested_by_customer`, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Receipt number for this refund.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_number: Option<String>,

    /// Status of the refund: `succeeded`, `pending`, `failed`, or `canceled`.
    pub status: String,

    /// Time at which the refund was created (UNIX epoch seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,

    /// ID of the balance transaction that describes the impact of this refund on your account balance (`txn_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance_transaction: Option<String>,
}

fn default_refund_object() -> String {
    "refund".into()
}

impl Refund {
    /// Returns the parsed [`RefundReason`] if the reason string matches a standard Stripe reason code.
    #[must_use]
    pub fn parsed_reason(&self) -> Option<RefundReason> {
        self.reason.as_deref().and_then(|r| r.parse().ok())
    }

    /// Checks if this refund is in `succeeded` status.
    #[must_use]
    pub fn is_succeeded(&self) -> bool {
        self.status == "succeeded"
    }

    /// Checks if this refund is in `pending` status.
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.status == "pending"
    }

    /// Checks if this refund is in `failed` status.
    #[must_use]
    pub fn is_failed(&self) -> bool {
        self.status == "failed"
    }

    /// Checks if this refund is in `canceled` status.
    #[must_use]
    pub fn is_canceled(&self) -> bool {
        self.status == "canceled"
    }

    /// Validates whether this refund satisfies the EU Directive 2011/83/EU Article 13 statutory
    /// 14-day withdrawal compliance for a given original charge amount and creation timestamp.
    ///
    /// If `self.created` is missing, `refund_requested_timestamp` defaults to `charge_created_timestamp`.
    #[must_use]
    pub fn is_statutory_withdrawal_compliant(
        &self,
        original_charge_amount: i64,
        charge_created_timestamp: i64,
    ) -> bool {
        let requested_ts = self.created.unwrap_or(charge_created_timestamp);
        is_statutory_withdrawal_compliant(
            self.amount,
            original_charge_amount,
            charge_created_timestamp,
            requested_ts,
        )
    }
}

/// Validates compliance with statutory consumer withdrawal refund requirements under
/// EU Directive 2011/83/EU Article 13 (Gate G-Law).
///
/// Under Article 13 of Directive 2011/83/EU:
/// 1. The consumer has a 14-calendar-day statutory right of withdrawal from the contract
///    date or delivery (`refund_requested_timestamp - charge_created_timestamp <= 14 * 86400`).
/// 2. The reimbursement must be for 100% of payments received without deductions, penalty fees,
///    or administrative discounts (`refund_amount == original_charge_amount`).
/// 3. Currency amounts must be positive ISO 4217 minor integer units.
///
/// # Arguments
/// * `refund_amount` - The refund amount in minor integer units.
/// * `original_charge_amount` - The original charge amount in minor integer units.
/// * `charge_created_timestamp` - The UNIX timestamp when the charge was created.
/// * `refund_requested_timestamp` - The UNIX timestamp when the refund was requested.
///
/// # Returns
/// `true` if the refund strictly satisfies the 14-day 100% full reimbursement statutory mandate; `false` otherwise.
///
/// # Example
/// ```
/// use some_stripe_types::is_statutory_withdrawal_compliant;
///
/// let charge_ts = 1_700_000_000;
/// // Refund requested 5 days later (within 14 days), for full amount of 5000 cents ($50.00)
/// let refund_ts = charge_ts + (5 * 86_400);
/// assert!(is_statutory_withdrawal_compliant(5000, 5000, charge_ts, refund_ts));
///
/// // Partial refund does not satisfy full statutory withdrawal restitution
/// assert!(!is_statutory_withdrawal_compliant(3000, 5000, charge_ts, refund_ts));
///
/// // Refund requested on day 15 (outside 14-day statutory window)
/// let late_ts = charge_ts + (15 * 86_400);
/// assert!(!is_statutory_withdrawal_compliant(5000, 5000, charge_ts, late_ts));
/// ```
#[must_use]
pub fn is_statutory_withdrawal_compliant(
    refund_amount: i64,
    original_charge_amount: i64,
    charge_created_timestamp: i64,
    refund_requested_timestamp: i64,
) -> bool {
    // 1. Amounts must be positive minor currency units
    if refund_amount <= 0 || original_charge_amount <= 0 {
        return false;
    }

    // 2. 100% full refund with zero deductions or penalties
    if refund_amount != original_charge_amount {
        return false;
    }

    // 3. Refund request cannot antedate the charge creation timestamp
    if refund_requested_timestamp < charge_created_timestamp {
        return false;
    }

    // 4. Request must occur within statutory 14 calendar days (1,209,600 seconds)
    let elapsed = refund_requested_timestamp - charge_created_timestamp;
    elapsed <= STATUTORY_WITHDRAWAL_WINDOW_SECONDS
}

/// Parameters for creating a Stripe Refund.
///
/// Supports either `charge` or `payment_intent` identifiers.
/// If `amount` is omitted, Stripe issues a full refund of the remaining unrefunded charge amount.
///
/// # Example
/// ```
/// use some_stripe_types::{CreateRefundParams, RefundReason};
///
/// let params = CreateRefundParams::for_charge("ch_12345")
///     .with_amount(2000)
///     .with_reason(RefundReason::RequestedByCustomer)
///     .with_idempotency_key("req_idem_123");
///
/// assert_eq!(params.charge.as_deref(), Some("ch_12345"));
/// assert_eq!(params.amount, Some(2000));
/// assert_eq!(params.reason.as_deref(), Some("requested_by_customer"));
/// assert_eq!(params.idempotency_key.as_deref(), Some("req_idem_123"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateRefundParams {
    /// The identifier of the charge to refund (`ch_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charge: Option<String>,

    /// The identifier of the PaymentIntent to refund (`pi_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_intent: Option<String>,

    /// A positive integer in minor units (e.g., 2000 for $20.00) representing how much of this charge to refund.
    /// If omitted, the entire remaining amount of the charge will be refunded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i64>,

    /// Three-letter ISO currency code in lowercase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    /// Customer ID whose charge is to be refunded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,

    /// String explaining the reason for the refund (`duplicate`, `fraudulent`, or `requested_by_customer`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Whether the application fee should be refunded when refunding this charge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refund_application_fee: Option<bool>,

    /// Whether the transfer should be reversed when refunding this charge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverse_transfer: Option<bool>,

    /// Optional instructions email for offline refund disbursement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions_email: Option<String>,

    /// Unique idempotency key for safe retries (Gate G2). Not serialized in request body.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateRefundParams {
    /// Creates a new empty [`CreateRefundParams`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience constructor for refunding a specific charge ID.
    #[must_use]
    pub fn for_charge(charge_id: impl Into<String>) -> Self {
        Self {
            charge: Some(charge_id.into()),
            ..Default::default()
        }
    }

    /// Convenience constructor for refunding a specific PaymentIntent ID.
    #[must_use]
    pub fn for_payment_intent(payment_intent_id: impl Into<String>) -> Self {
        Self {
            payment_intent: Some(payment_intent_id.into()),
            ..Default::default()
        }
    }

    /// Sets the refund amount in minor integer units.
    #[must_use]
    pub fn with_amount(mut self, amount: i64) -> Self {
        self.amount = Some(amount);
        self
    }

    /// Sets the refund reason from a string or [`RefundReason`].
    #[must_use]
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Sets the ISO currency code.
    #[must_use]
    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = Some(currency.into());
        self
    }

    /// Sets the customer ID.
    #[must_use]
    pub fn with_customer(mut self, customer: impl Into<String>) -> Self {
        self.customer = Some(customer.into());
        self
    }

    /// Sets the idempotency key for safe execution.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Sets metadata key-value pairs.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets whether to refund the application fee.
    #[must_use]
    pub fn with_refund_application_fee(mut self, refund_application_fee: bool) -> Self {
        self.refund_application_fee = Some(refund_application_fee);
        self
    }

    /// Sets whether to reverse the transfer.
    #[must_use]
    pub fn with_reverse_transfer(mut self, reverse_transfer: bool) -> Self {
        self.reverse_transfer = Some(reverse_transfer);
        self
    }
}

/// Parameters for updating an existing Stripe Refund.
///
/// In Stripe API, only `metadata` can be updated on a Refund.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateRefundParams {
    /// Set of key-value pairs to update on the refund.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Unique idempotency key for safe retries (Gate G2). Not serialized in request body.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl UpdateRefundParams {
    /// Creates a new empty [`UpdateRefundParams`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets metadata key-value pairs.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets the idempotency key for safe execution.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for canceling a pending Stripe Refund.
///
/// For certain payment methods (like ACH or WeChat Pay), refunds may take time to process
/// and can be canceled before completion.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelRefundParams {
    /// Unique idempotency key for safe retries (Gate G2). Not serialized in request body.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CancelRefundParams {
    /// Creates a new empty [`CancelRefundParams`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the idempotency key for safe execution.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refund_json_roundtrip_full() {
        let mut metadata = HashMap::new();
        metadata.insert("order_ref".to_string(), "ord_999".to_string());

        let refund = Refund {
            id: "re_test_12345".to_string(),
            object: "refund".to_string(),
            amount: 5000,
            charge: Some("ch_charge_123".to_string()),
            currency: "usd".to_string(),
            description: Some("Customer return".to_string()),
            failure_balance_transaction: None,
            failure_reason: None,
            metadata: Some(metadata),
            payment_intent: Some("pi_intent_456".to_string()),
            reason: Some("requested_by_customer".to_string()),
            receipt_number: Some("REC-789".to_string()),
            status: "succeeded".to_string(),
            created: Some(1728345600),
            balance_transaction: Some("txn_bal_999".to_string()),
        };

        let json = serde_json::to_string(&refund).expect("Serialization failed");
        let decoded: Refund = serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(decoded.id, "re_test_12345");
        assert_eq!(decoded.object, "refund");
        assert_eq!(decoded.amount, 5000);
        assert_eq!(decoded.charge.as_deref(), Some("ch_charge_123"));
        assert_eq!(decoded.currency, "usd");
        assert_eq!(decoded.description.as_deref(), Some("Customer return"));
        assert_eq!(decoded.payment_intent.as_deref(), Some("pi_intent_456"));
        assert_eq!(decoded.reason.as_deref(), Some("requested_by_customer"));
        assert_eq!(decoded.status, "succeeded");
        assert_eq!(decoded.balance_transaction.as_deref(), Some("txn_bal_999"));
        assert!(decoded.is_succeeded());
        assert!(!decoded.is_pending());
        assert!(!decoded.is_failed());
        assert!(!decoded.is_canceled());
        assert_eq!(
            decoded.parsed_reason(),
            Some(RefundReason::RequestedByCustomer)
        );
    }

    #[test]
    fn test_refund_default_object_omitted_in_json() {
        let json = r#"{
            "id": "re_minimal",
            "amount": 2500,
            "currency": "eur",
            "status": "pending"
        }"#;

        let refund: Refund =
            serde_json::from_str(json).expect("Minimal refund deserialization failed");
        assert_eq!(refund.id, "re_minimal");
        assert_eq!(refund.object, "refund");
        assert_eq!(refund.amount, 2500);
        assert_eq!(refund.currency, "eur");
        assert_eq!(refund.status, "pending");
        assert!(refund.is_pending());
        assert!(!refund.is_succeeded());
        assert!(refund.charge.is_none());
        assert!(refund.payment_intent.is_none());
        assert!(refund.metadata.is_none());
    }

    #[test]
    fn test_refund_reason_parsing_and_display() {
        let reasons = [
            (RefundReason::Duplicate, "duplicate"),
            (RefundReason::Fraudulent, "fraudulent"),
            (RefundReason::RequestedByCustomer, "requested_by_customer"),
            (
                RefundReason::ExpiredUncapturedCharge,
                "expired_uncaptured_charge",
            ),
        ];

        for (reason, s) in reasons {
            assert_eq!(reason.as_str(), s);
            assert_eq!(reason.to_string(), s);
            assert_eq!(reason.as_ref(), s);
            let parsed: RefundReason = s.parse().expect("Parse failed");
            assert_eq!(parsed, reason);

            let json = serde_json::to_string(&reason).expect("Serialize failed");
            assert_eq!(json, format!("\"{s}\""));
            let from_json: RefundReason = serde_json::from_str(&json).expect("Deserialize failed");
            assert_eq!(from_json, reason);
        }

        let err = "unknown_reason".parse::<RefundReason>().unwrap_err();
        assert!(err.to_string().contains("unknown_reason"));
    }

    #[test]
    fn test_statutory_withdrawal_compliance_law_matrix() {
        let charge_ts = 1_700_000_000;
        let original_amount = 8900; // $89.00 in cents

        // 1. Valid cases: full refund on day 0, day 7, day 14 (exact boundary)
        assert!(is_statutory_withdrawal_compliant(
            original_amount,
            original_amount,
            charge_ts,
            charge_ts
        ));
        assert!(is_statutory_withdrawal_compliant(
            original_amount,
            original_amount,
            charge_ts,
            charge_ts + (7 * 86_400)
        ));
        assert!(is_statutory_withdrawal_compliant(
            original_amount,
            original_amount,
            charge_ts,
            charge_ts + (14 * 86_400) // Exactly 14 days
        ));

        // 2. Beyond 14-day statutory window: 14 days + 1 second, 15 days
        assert!(!is_statutory_withdrawal_compliant(
            original_amount,
            original_amount,
            charge_ts,
            charge_ts + (14 * 86_400) + 1
        ));
        assert!(!is_statutory_withdrawal_compliant(
            original_amount,
            original_amount,
            charge_ts,
            charge_ts + (15 * 86_400)
        ));

        // 3. Deduction violations (partial refunds)
        assert!(!is_statutory_withdrawal_compliant(
            original_amount - 1,
            original_amount,
            charge_ts,
            charge_ts + (3 * 86_400)
        ));
        assert!(!is_statutory_withdrawal_compliant(
            original_amount / 2,
            original_amount,
            charge_ts,
            charge_ts + (3 * 86_400)
        ));

        // 4. Over-refund (cannot claim 100% statutory consideration restitution)
        assert!(!is_statutory_withdrawal_compliant(
            original_amount + 500,
            original_amount,
            charge_ts,
            charge_ts + (3 * 86_400)
        ));

        // 5. Zero and negative amounts
        assert!(!is_statutory_withdrawal_compliant(
            0,
            original_amount,
            charge_ts,
            charge_ts
        ));
        assert!(!is_statutory_withdrawal_compliant(
            -100,
            original_amount,
            charge_ts,
            charge_ts
        ));
        assert!(!is_statutory_withdrawal_compliant(
            original_amount,
            0,
            charge_ts,
            charge_ts
        ));
        assert!(!is_statutory_withdrawal_compliant(
            original_amount,
            -100,
            charge_ts,
            charge_ts
        ));

        // 6. Chronological skew (requested before charge created)
        assert!(!is_statutory_withdrawal_compliant(
            original_amount,
            original_amount,
            charge_ts,
            charge_ts - 100
        ));
    }

    #[test]
    fn test_refund_entity_statutory_method() {
        let charge_ts = 1_700_000_000;
        let refund = Refund {
            id: "re_statutory_check".to_string(),
            object: "refund".to_string(),
            amount: 4500,
            charge: Some("ch_compliant".to_string()),
            currency: "eur".to_string(),
            description: None,
            failure_balance_transaction: None,
            failure_reason: None,
            metadata: None,
            payment_intent: None,
            reason: Some("requested_by_customer".to_string()),
            receipt_number: None,
            status: "succeeded".to_string(),
            created: Some(charge_ts + (10 * 86_400)), // 10 days
            balance_transaction: None,
        };

        assert!(refund.is_statutory_withdrawal_compliant(4500, charge_ts));
        // Partial refund should fail
        assert!(!refund.is_statutory_withdrawal_compliant(5000, charge_ts));
    }

    #[test]
    fn test_create_refund_params_builder() {
        let mut meta = HashMap::new();
        meta.insert("source".to_string(), "dashboard".to_string());

        let params = CreateRefundParams::for_charge("ch_test_abc")
            .with_amount(1500)
            .with_currency("usd")
            .with_reason(RefundReason::Duplicate)
            .with_customer("cus_user_123")
            .with_idempotency_key("idem_refund_1")
            .with_metadata(meta)
            .with_refund_application_fee(true)
            .with_reverse_transfer(true);

        assert_eq!(params.charge.as_deref(), Some("ch_test_abc"));
        assert_eq!(params.amount, Some(1500));
        assert_eq!(params.currency.as_deref(), Some("usd"));
        assert_eq!(params.reason.as_deref(), Some("duplicate"));
        assert_eq!(params.customer.as_deref(), Some("cus_user_123"));
        assert_eq!(params.idempotency_key.as_deref(), Some("idem_refund_1"));
        assert_eq!(params.refund_application_fee, Some(true));
        assert_eq!(params.reverse_transfer, Some(true));

        // Ensure idempotency key is skipped in serialization
        let json = serde_json::to_string(&params).expect("Failed to serialize");
        assert!(!json.contains("idem_refund_1"));
        assert!(json.contains("ch_test_abc"));
        assert!(json.contains("1500"));
    }

    #[test]
    fn test_update_and_cancel_refund_params_builder() {
        let mut meta = HashMap::new();
        meta.insert("note".to_string(), "updated".to_string());

        let update = UpdateRefundParams::new()
            .with_metadata(meta)
            .with_idempotency_key("idem_update_1");

        assert_eq!(update.idempotency_key.as_deref(), Some("idem_update_1"));
        let update_json = serde_json::to_string(&update).expect("Serialize update failed");
        assert!(!update_json.contains("idem_update_1"));
        assert!(update_json.contains("updated"));

        let cancel = CancelRefundParams::new().with_idempotency_key("idem_cancel_1");
        assert_eq!(cancel.idempotency_key.as_deref(), Some("idem_cancel_1"));
        let cancel_json = serde_json::to_string(&cancel).expect("Serialize cancel failed");
        assert!(!cancel_json.contains("idem_cancel_1"));
    }

    #[test]
    fn test_gate_g_borrow_zero_clone_discipline() {
        let refund = Refund {
            id: "re_borrow_test".to_string(),
            object: "refund".to_string(),
            amount: 3000,
            charge: Some("ch_borrow".to_string()),
            currency: "usd".to_string(),
            description: None,
            failure_balance_transaction: None,
            failure_reason: None,
            metadata: None,
            payment_intent: None,
            reason: Some("requested_by_customer".to_string()),
            receipt_number: None,
            status: "succeeded".to_string(),
            created: Some(1728345600),
            balance_transaction: None,
        };

        // Borrowing tests: strictly references, no clone
        let borrowed_id: &str = &refund.id;
        let borrowed_currency: &str = &refund.currency;
        assert_eq!(borrowed_id, "re_borrow_test");
        assert_eq!(borrowed_currency, "usd");

        // RefundReason is Copy
        let reason = RefundReason::RequestedByCustomer;
        let copied = reason;
        assert_eq!(reason, copied);
    }
}
