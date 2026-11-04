//! Request parameter schemas, builders, and minor currency validation for PaymentIntents.
//!
//! Enforces ISO 4217 minor currency integer rules (Gate G-Law) and PCI-DSS credential masking
//! (Gate G2 / PCI-DSS 4.0).

use serde::{Deserialize, Serialize};

/// Validation errors for PaymentIntent parameters and minor currency operations (Gate G-Law & ISO 4217).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentIntentValidationError {
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
    /// The specified capture amount is invalid; it must be a positive integer minor currency unit (> 0).
    InvalidCaptureAmount {
        /// The invalid capture amount supplied.
        amount: i64,
    },
    /// A general parameter validation failure.
    InvalidParameter {
        /// Field name.
        field: &'static str,
        /// Description of the error.
        reason: String,
    },
}

impl std::fmt::Display for PaymentIntentValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAmount { amount } => {
                write!(
                    f,
                    "PaymentIntent amount must be a positive integer minor currency unit (> 0), got: {amount}"
                )
            }
            Self::InvalidCurrency { currency } => {
                write!(
                    f,
                    "PaymentIntent currency must be a valid 3-letter ISO 4217 code (got: '{currency}')"
                )
            }
            Self::InvalidCaptureAmount { amount } => {
                write!(
                    f,
                    "PaymentIntent amount_to_capture must be a positive integer minor currency unit (> 0), got: {amount}"
                )
            }
            Self::InvalidParameter { field, reason } => {
                write!(f, "Invalid PaymentIntent parameter '{field}': {reason}")
            }
        }
    }
}

impl std::error::Error for PaymentIntentValidationError {}

/// Alias for [`PaymentIntentValidationError`] for broader API ergonomics.
pub type PaymentIntentError = PaymentIntentValidationError;

/// Validates that an amount is a positive integer minor currency unit (> 0).
///
/// In ISO 4217 minor currency units (e.g. cents, pence, yen), amounts must be strictly
/// greater than zero.
///
/// # Errors
/// Returns [`PaymentIntentValidationError::InvalidAmount`] if `amount <= 0`.
///
/// # Examples
/// ```
/// use some_stripe_types::validate_amount;
///
/// assert!(validate_amount(100).is_ok());
/// assert!(validate_amount(0).is_err());
/// assert!(validate_amount(-50).is_err());
/// ```
pub fn validate_amount(amount: i64) -> Result<i64, PaymentIntentValidationError> {
    if amount <= 0 {
        return Err(PaymentIntentValidationError::InvalidAmount { amount });
    }
    Ok(amount)
}

/// Validates that a capture amount is a positive integer minor currency unit (> 0).
///
/// # Errors
/// Returns [`PaymentIntentValidationError::InvalidCaptureAmount`] if `amount <= 0`.
///
/// # Examples
/// ```
/// use some_stripe_types::validate_capture_amount;
///
/// assert!(validate_capture_amount(500).is_ok());
/// assert!(validate_capture_amount(0).is_err());
/// assert!(validate_capture_amount(-1).is_err());
/// ```
pub fn validate_capture_amount(amount: i64) -> Result<i64, PaymentIntentValidationError> {
    if amount <= 0 {
        return Err(PaymentIntentValidationError::InvalidCaptureAmount { amount });
    }
    Ok(amount)
}

/// Validates that a currency code is a valid 3-letter ISO 4217 code and converts it to lowercase.
///
/// # Errors
/// Returns [`PaymentIntentValidationError::InvalidCurrency`] if the code does not have exactly
/// three ASCII alphabetic characters.
///
/// # Examples
/// ```
/// use some_stripe_types::validate_currency;
///
/// assert_eq!(validate_currency("usd").unwrap(), "usd");
/// assert_eq!(validate_currency("USD").unwrap(), "usd");
/// assert_eq!(validate_currency("EUR").unwrap(), "eur");
/// assert!(validate_currency("us").is_err());
/// assert!(validate_currency("usdt").is_err());
/// assert!(validate_currency("123").is_err());
/// ```
pub fn validate_currency(currency: &str) -> Result<String, PaymentIntentValidationError> {
    let trimmed = currency.trim();
    if trimmed.len() != 3 || !trimmed.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(PaymentIntentValidationError::InvalidCurrency {
            currency: currency.to_string(),
        });
    }
    Ok(trimmed.to_ascii_lowercase())
}

/// Capture method for a PaymentIntent.
pub use crate::CaptureMethod;

/// Predefined cancellation reasons for a PaymentIntent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancellationReason {
    /// Duplicate transaction.
    Duplicate,
    /// Fraudulent transaction.
    Fraudulent,
    /// Customer requested cancellation.
    RequestedByCustomer,
    /// Customer abandoned the checkout flow.
    Abandoned,
}

impl CancellationReason {
    /// Returns the string representation used by Stripe API.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Duplicate => "duplicate",
            Self::Fraudulent => "fraudulent",
            Self::RequestedByCustomer => "requested_by_customer",
            Self::Abandoned => "abandoned",
        }
    }
}

impl std::fmt::Display for CancellationReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Parameters for creating a Stripe PaymentIntent (`POST /v1/payment_intents`).
///
/// Invariants enforced:
/// - `amount` must be a positive integer in minor currency units (Gate G-Law / ISO 4217).
/// - `currency` must be a valid 3-letter ISO 4217 code stored in lowercase.
///
/// # Example
/// ```
/// use some_stripe_types::CreatePaymentIntentParams;
///
/// let params = CreatePaymentIntentParams::new(5000, "USD")
///     .unwrap()
///     .capture_method("manual")
///     .description("Order #9921")
///     .idempotency_key("idem_order_9921");
///
/// assert_eq!(params.amount, 5000);
/// assert_eq!(params.currency, "usd");
/// assert_eq!(params.capture_method.as_deref(), Some("manual"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePaymentIntentParams {
    /// Amount intended to be collected in minor currency units (e.g. cents, pence). Must be > 0.
    pub amount: i64,

    /// Three-letter ISO currency code stored in lowercase.
    pub currency: String,

    /// Capture method: `automatic` (default) or `manual`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_method: Option<String>,

    /// An arbitrary string attached to the object, often useful for displaying to users.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// ID of the payment method to attach and confirm (e.g. `pm_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<String>,

    /// ID of the Customer this PaymentIntent belongs to, if one exists (e.g. `cus_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,

    /// The email address that will receive the payment receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_email: Option<String>,

    /// The URL to redirect your customer back to after they authenticate on the payment method's app or site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_url: Option<String>,

    /// Set to `true` to attempt to confirm this PaymentIntent immediately upon creation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirm: Option<bool>,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,

    /// Unique idempotency key header value. Not serialized into the HTTP payload body.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreatePaymentIntentParams {
    /// Constructs a validated `CreatePaymentIntentParams`.
    ///
    /// Automatically normalizes `currency` to lowercase and ensures `amount > 0`.
    ///
    /// # Errors
    /// Returns [`PaymentIntentValidationError::InvalidAmount`] if `amount <= 0`.
    /// Returns [`PaymentIntentValidationError::InvalidCurrency`] if `currency` is not a 3-letter ASCII ISO code.
    pub fn new(
        amount: i64,
        currency: impl Into<String>,
    ) -> Result<Self, PaymentIntentValidationError> {
        let currency = validate_currency(&currency.into())?;
        let amount = validate_amount(amount)?;
        Ok(Self {
            amount,
            currency,
            capture_method: None,
            description: None,
            payment_method: None,
            customer: None,
            receipt_email: None,
            return_url: None,
            confirm: None,
            metadata: None,
            idempotency_key: None,
        })
    }

    /// Creates a fluent builder initialized with the mandatory `amount` and `currency`.
    pub fn builder(
        amount: i64,
        currency: impl Into<String>,
    ) -> Result<CreatePaymentIntentParamsBuilder, PaymentIntentValidationError> {
        CreatePaymentIntentParamsBuilder::new(amount, currency)
    }

    /// Sets the capture method via string (`"automatic"` or `"manual"`).
    pub fn capture_method(mut self, capture_method: impl Into<String>) -> Self {
        self.capture_method = Some(capture_method.into());
        self
    }

    /// Sets the capture method via typed [`CaptureMethod`] enum.
    pub fn capture_method_type(mut self, capture_method: CaptureMethod) -> Self {
        self.capture_method = Some(capture_method.to_string());
        self
    }

    /// Sets the payment description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Sets the payment method identifier.
    pub fn payment_method(mut self, pm: impl Into<String>) -> Self {
        self.payment_method = Some(pm.into());
        self
    }

    /// Sets the customer identifier.
    pub fn customer(mut self, cus: impl Into<String>) -> Self {
        self.customer = Some(cus.into());
        self
    }

    /// Sets the receipt email.
    pub fn receipt_email(mut self, email: impl Into<String>) -> Self {
        self.receipt_email = Some(email.into());
        self
    }

    /// Sets the return URL for redirect-based authentication.
    pub fn return_url(mut self, return_url: impl Into<String>) -> Self {
        self.return_url = Some(return_url.into());
        self
    }

    /// Sets whether to confirm the payment immediately.
    pub fn confirm(mut self, confirm: bool) -> Self {
        self.confirm = Some(confirm);
        self
    }

    /// Sets key-value metadata pairs.
    pub fn metadata<K: Into<String>, V: Into<String>>(
        mut self,
        iter: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        let map = iter
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self.metadata = Some(map);
        self
    }

    /// Attaches an idempotency key.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates all parameters and invariants against Stripe requirements.
    ///
    /// # Errors
    /// Returns [`PaymentIntentValidationError`] if amount <= 0 or currency is invalid.
    pub fn validate(&self) -> Result<(), PaymentIntentValidationError> {
        validate_amount(self.amount)?;
        validate_currency(&self.currency)?;
        Ok(())
    }
}

/// Builder for fluent construction of [`CreatePaymentIntentParams`].
#[derive(Debug, Clone)]
pub struct CreatePaymentIntentParamsBuilder {
    amount: i64,
    currency: String,
    capture_method: Option<String>,
    description: Option<String>,
    payment_method: Option<String>,
    customer: Option<String>,
    receipt_email: Option<String>,
    return_url: Option<String>,
    confirm: Option<bool>,
    metadata: Option<std::collections::HashMap<String, String>>,
    idempotency_key: Option<String>,
}

impl CreatePaymentIntentParamsBuilder {
    /// Creates a new builder with validated `amount` and `currency`.
    pub fn new(
        amount: i64,
        currency: impl Into<String>,
    ) -> Result<Self, PaymentIntentValidationError> {
        let currency = validate_currency(&currency.into())?;
        let amount = validate_amount(amount)?;
        Ok(Self {
            amount,
            currency,
            capture_method: None,
            description: None,
            payment_method: None,
            customer: None,
            receipt_email: None,
            return_url: None,
            confirm: None,
            metadata: None,
            idempotency_key: None,
        })
    }

    /// Sets the capture method via string.
    pub fn capture_method(mut self, capture_method: impl Into<String>) -> Self {
        self.capture_method = Some(capture_method.into());
        self
    }

    /// Sets the capture method via [`CaptureMethod`].
    pub fn capture_method_type(mut self, capture_method: CaptureMethod) -> Self {
        self.capture_method = Some(capture_method.to_string());
        self
    }

    /// Sets description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Sets payment method.
    pub fn payment_method(mut self, pm: impl Into<String>) -> Self {
        self.payment_method = Some(pm.into());
        self
    }

    /// Sets customer.
    pub fn customer(mut self, cus: impl Into<String>) -> Self {
        self.customer = Some(cus.into());
        self
    }

    /// Sets receipt email.
    pub fn receipt_email(mut self, email: impl Into<String>) -> Self {
        self.receipt_email = Some(email.into());
        self
    }

    /// Sets return URL.
    pub fn return_url(mut self, return_url: impl Into<String>) -> Self {
        self.return_url = Some(return_url.into());
        self
    }

    /// Sets confirm flag.
    pub fn confirm(mut self, confirm: bool) -> Self {
        self.confirm = Some(confirm);
        self
    }

    /// Sets metadata.
    pub fn metadata<K: Into<String>, V: Into<String>>(
        mut self,
        iter: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        let map = iter
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self.metadata = Some(map);
        self
    }

    /// Sets idempotency key.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Finalizes and returns the [`CreatePaymentIntentParams`].
    pub fn build(self) -> Result<CreatePaymentIntentParams, PaymentIntentValidationError> {
        let params = CreatePaymentIntentParams {
            amount: self.amount,
            currency: self.currency,
            capture_method: self.capture_method,
            description: self.description,
            payment_method: self.payment_method,
            customer: self.customer,
            receipt_email: self.receipt_email,
            return_url: self.return_url,
            confirm: self.confirm,
            metadata: self.metadata,
            idempotency_key: self.idempotency_key,
        };
        params.validate()?;
        Ok(params)
    }
}

/// Parameters for capturing an authorized PaymentIntent (`POST /v1/payment_intents/{id}/capture`).
///
/// Invariants enforced:
/// - If specified, `amount_to_capture` must be a positive integer in minor currency units (> 0).
///
/// # Example
/// ```
/// use some_stripe_types::CapturePaymentIntentParams;
///
/// let params = CapturePaymentIntentParams::with_amount(2500)
///     .unwrap()
///     .idempotency_key("idem_capture_123");
///
/// assert_eq!(params.amount_to_capture, Some(2500));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturePaymentIntentParams {
    /// The amount to capture from the authorization. If omitted, captures the full amount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_to_capture: Option<i64>,

    /// Unique idempotency key. Not serialized into the HTTP payload body.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CapturePaymentIntentParams {
    /// Creates a new default `CapturePaymentIntentParams` capturing the full authorized amount.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a `CapturePaymentIntentParams` with a validated partial amount to capture.
    ///
    /// # Errors
    /// Returns [`PaymentIntentValidationError::InvalidCaptureAmount`] if `amount_to_capture <= 0`.
    pub fn with_amount(amount_to_capture: i64) -> Result<Self, PaymentIntentValidationError> {
        let amount = validate_capture_amount(amount_to_capture)?;
        Ok(Self {
            amount_to_capture: Some(amount),
            idempotency_key: None,
        })
    }

    /// Sets the amount to capture.
    ///
    /// # Errors
    /// Returns [`PaymentIntentValidationError::InvalidCaptureAmount`] if `amount <= 0`.
    pub fn amount_to_capture(mut self, amount: i64) -> Result<Self, PaymentIntentValidationError> {
        let amount = validate_capture_amount(amount)?;
        self.amount_to_capture = Some(amount);
        Ok(self)
    }

    /// Attaches an idempotency key.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates the capture parameters.
    ///
    /// # Errors
    /// Returns [`PaymentIntentValidationError::InvalidCaptureAmount`] if `amount_to_capture <= 0`.
    pub fn validate(&self) -> Result<(), PaymentIntentValidationError> {
        if let Some(amt) = self.amount_to_capture {
            validate_capture_amount(amt)?;
        }
        Ok(())
    }
}

/// Parameters for canceling a PaymentIntent (`POST /v1/payment_intents/{id}/cancel`).
///
/// # Example
/// ```
/// use some_stripe_types::{CancelPaymentIntentParams, CancellationReason};
///
/// let params = CancelPaymentIntentParams::with_reason("duplicate")
///     .idempotency_key("idem_cancel_999");
///
/// assert_eq!(params.cancellation_reason.as_deref(), Some("duplicate"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelPaymentIntentParams {
    /// Reason for canceling this PaymentIntent: `duplicate`, `fraudulent`, `requested_by_customer`, or `abandoned`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_reason: Option<String>,

    /// Unique idempotency key. Not serialized into the HTTP payload body.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CancelPaymentIntentParams {
    /// Creates a new default `CancelPaymentIntentParams`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a `CancelPaymentIntentParams` with a cancellation reason.
    pub fn with_reason(reason: impl Into<String>) -> Self {
        Self {
            cancellation_reason: Some(reason.into()),
            idempotency_key: None,
        }
    }

    /// Sets the cancellation reason via string.
    pub fn cancellation_reason(mut self, reason: impl Into<String>) -> Self {
        self.cancellation_reason = Some(reason.into());
        self
    }

    /// Sets the cancellation reason via typed [`CancellationReason`] enum.
    pub fn reason(mut self, reason: CancellationReason) -> Self {
        self.cancellation_reason = Some(reason.to_string());
        self
    }

    /// Attaches an idempotency key.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates the cancellation parameters.
    pub fn validate(&self) -> Result<(), PaymentIntentValidationError> {
        Ok(())
    }
}

/// Parameters for confirming a PaymentIntent (`POST /v1/payment_intents/{id}/confirm`).
///
/// Implements custom `Debug` to mask `client_secret` as `[REDACTED]` (Gate G2 / PCI-DSS 4.0).
///
/// # Example
/// ```
/// use some_stripe_types::ConfirmPaymentIntentParams;
///
/// let params = ConfirmPaymentIntentParams::new()
///     .payment_method("pm_card_visa")
///     .return_url("https://example.com/checkout/return")
///     .idempotency_key("idem_confirm_123");
///
/// assert_eq!(params.payment_method.as_deref(), Some("pm_card_visa"));
/// ```
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmPaymentIntentParams {
    /// ID of the payment method used to confirm this PaymentIntent (e.g. `pm_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<String>,

    /// The URL to redirect your customer back to after they authenticate (Gate G-Law / 3DS).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_url: Option<String>,

    /// Email address that the receipt for the resulting payment will be sent to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_email: Option<String>,

    /// Indicates that you intend to make future payments with this PaymentIntent's payment method: `on_session` or `off_session`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup_future_usage: Option<String>,

    /// The client secret of the PaymentIntent, if confirming client-side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,

    /// Unique idempotency key. Not serialized into the HTTP payload body.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl ConfirmPaymentIntentParams {
    /// Creates a new default `ConfirmPaymentIntentParams`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the payment method ID.
    pub fn payment_method(mut self, pm: impl Into<String>) -> Self {
        self.payment_method = Some(pm.into());
        self
    }

    /// Sets the return URL for 3DS / SCA redirect authentication.
    pub fn return_url(mut self, url: impl Into<String>) -> Self {
        self.return_url = Some(url.into());
        self
    }

    /// Sets the receipt email address.
    pub fn receipt_email(mut self, email: impl Into<String>) -> Self {
        self.receipt_email = Some(email.into());
        self
    }

    /// Sets `setup_future_usage` (`"on_session"` or `"off_session"`).
    pub fn setup_future_usage(mut self, usage: impl Into<String>) -> Self {
        self.setup_future_usage = Some(usage.into());
        self
    }

    /// Sets the client secret (masked in `Debug` implementations).
    pub fn client_secret(mut self, secret: impl Into<String>) -> Self {
        self.client_secret = Some(secret.into());
        self
    }

    /// Attaches an idempotency key.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates the confirmation parameters.
    pub fn validate(&self) -> Result<(), PaymentIntentValidationError> {
        Ok(())
    }
}

impl std::fmt::Debug for ConfirmPaymentIntentParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfirmPaymentIntentParams")
            .field("payment_method", &self.payment_method)
            .field("return_url", &self.return_url)
            .field("receipt_email", &self.receipt_email)
            .field("setup_future_usage", &self.setup_future_usage)
            .field(
                "client_secret",
                &self.client_secret.as_ref().map(|_| "[REDACTED]"),
            )
            .field("idempotency_key", &self.idempotency_key)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_positive_amounts_accepted() {
        assert_eq!(validate_amount(1).unwrap(), 1);
        assert_eq!(validate_amount(100).unwrap(), 100);
        assert_eq!(validate_amount(5000).unwrap(), 5000);
        assert_eq!(validate_amount(100_000_000).unwrap(), 100_000_000);
    }

    #[test]
    fn test_zero_or_negative_amounts_rejected() {
        assert_eq!(
            validate_amount(0),
            Err(PaymentIntentValidationError::InvalidAmount { amount: 0 })
        );
        assert_eq!(
            validate_amount(-1),
            Err(PaymentIntentValidationError::InvalidAmount { amount: -1 })
        );
        assert_eq!(
            validate_amount(-5000),
            Err(PaymentIntentValidationError::InvalidAmount { amount: -5000 })
        );
    }

    #[test]
    fn test_invalid_currency_length_rejected() {
        assert_eq!(
            validate_currency(""),
            Err(PaymentIntentValidationError::InvalidCurrency {
                currency: "".to_string()
            })
        );
        assert_eq!(
            validate_currency("us"),
            Err(PaymentIntentValidationError::InvalidCurrency {
                currency: "us".to_string()
            })
        );
        assert_eq!(
            validate_currency("usdt"),
            Err(PaymentIntentValidationError::InvalidCurrency {
                currency: "usdt".to_string()
            })
        );
        assert_eq!(
            validate_currency("dollars"),
            Err(PaymentIntentValidationError::InvalidCurrency {
                currency: "dollars".to_string()
            })
        );
    }

    #[test]
    fn test_non_alphabetic_currency_rejected() {
        assert_eq!(
            validate_currency("123"),
            Err(PaymentIntentValidationError::InvalidCurrency {
                currency: "123".to_string()
            })
        );
        assert_eq!(
            validate_currency("u$d"),
            Err(PaymentIntentValidationError::InvalidCurrency {
                currency: "u$d".to_string()
            })
        );
        assert_eq!(
            validate_currency("us1"),
            Err(PaymentIntentValidationError::InvalidCurrency {
                currency: "us1".to_string()
            })
        );
        assert_eq!(
            validate_currency("   "),
            Err(PaymentIntentValidationError::InvalidCurrency {
                currency: "   ".to_string()
            })
        );
    }

    #[test]
    fn test_uppercase_currency_converted_to_lowercase() {
        assert_eq!(validate_currency("USD").unwrap(), "usd");
        assert_eq!(validate_currency("EUR").unwrap(), "eur");
        assert_eq!(validate_currency("GBP").unwrap(), "gbp");
        assert_eq!(validate_currency("jpy").unwrap(), "jpy");
        assert_eq!(validate_currency(" ChF ").unwrap(), "chf");
    }

    #[test]
    fn test_create_params_builder_chaining() {
        let params = CreatePaymentIntentParams::new(2000, "USD")
            .expect("Valid creation")
            .capture_method("manual")
            .description("Order #1001")
            .payment_method("pm_card_visa")
            .customer("cus_alpha")
            .receipt_email("test@example.com")
            .return_url("https://example.com/checkout/return")
            .confirm(true)
            .metadata([("order_id", "ord_1001"), ("tier", "gold")])
            .idempotency_key("idem_req_abc123");

        assert_eq!(params.amount, 2000);
        assert_eq!(params.currency, "usd");
        assert_eq!(params.capture_method.as_deref(), Some("manual"));
        assert_eq!(params.description.as_deref(), Some("Order #1001"));
        assert_eq!(params.payment_method.as_deref(), Some("pm_card_visa"));
        assert_eq!(params.customer.as_deref(), Some("cus_alpha"));
        assert_eq!(params.receipt_email.as_deref(), Some("test@example.com"));
        assert_eq!(
            params.return_url.as_deref(),
            Some("https://example.com/checkout/return")
        );
        assert_eq!(params.confirm, Some(true));
        assert_eq!(params.idempotency_key.as_deref(), Some("idem_req_abc123"));

        let meta = params.metadata.as_ref().unwrap();
        assert_eq!(meta.get("order_id").unwrap(), "ord_1001");
        assert_eq!(meta.get("tier").unwrap(), "gold");

        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_create_params_builder_struct() {
        let params = CreatePaymentIntentParams::builder(3500, "EUR")
            .expect("Valid builder")
            .capture_method_type(CaptureMethod::Manual)
            .description("Annual Subscription")
            .build()
            .expect("Build success");

        assert_eq!(params.amount, 3500);
        assert_eq!(params.currency, "eur");
        assert_eq!(params.capture_method.as_deref(), Some("manual"));
        assert_eq!(params.description.as_deref(), Some("Annual Subscription"));
    }

    #[test]
    fn test_capture_params_validation_and_builder() {
        // Default capture (full)
        let full = CapturePaymentIntentParams::new();
        assert!(full.amount_to_capture.is_none());
        assert!(full.validate().is_ok());

        // Partial capture (valid)
        let partial = CapturePaymentIntentParams::with_amount(1500)
            .expect("Valid partial amount")
            .idempotency_key("idem_cap_456");

        assert_eq!(partial.amount_to_capture, Some(1500));
        assert_eq!(partial.idempotency_key.as_deref(), Some("idem_cap_456"));
        assert!(partial.validate().is_ok());

        // Invalid zero or negative capture amount
        assert_eq!(
            CapturePaymentIntentParams::with_amount(0),
            Err(PaymentIntentValidationError::InvalidCaptureAmount { amount: 0 })
        );
        assert_eq!(
            CapturePaymentIntentParams::with_amount(-100),
            Err(PaymentIntentValidationError::InvalidCaptureAmount { amount: -100 })
        );

        // Invalid struct validate()
        let invalid_struct = CapturePaymentIntentParams {
            amount_to_capture: Some(0),
            ..Default::default()
        };
        assert_eq!(
            invalid_struct.validate(),
            Err(PaymentIntentValidationError::InvalidCaptureAmount { amount: 0 })
        );
    }

    #[test]
    fn test_cancel_params_builder() {
        let params = CancelPaymentIntentParams::new()
            .reason(CancellationReason::RequestedByCustomer)
            .idempotency_key("idem_cancel_789");

        assert_eq!(
            params.cancellation_reason.as_deref(),
            Some("requested_by_customer")
        );
        assert_eq!(params.idempotency_key.as_deref(), Some("idem_cancel_789"));
        assert!(params.validate().is_ok());

        let params_str = CancelPaymentIntentParams::with_reason("duplicate");
        assert_eq!(params_str.cancellation_reason.as_deref(), Some("duplicate"));
    }

    #[test]
    fn test_confirm_params_and_debug_redaction() {
        let params = ConfirmPaymentIntentParams::new()
            .payment_method("pm_card_visa")
            .return_url("https://example.com/return")
            .receipt_email("receipt@example.com")
            .setup_future_usage("off_session")
            .client_secret("pi_secret_sensitive_tok_998877")
            .idempotency_key("idem_conf_001");

        assert_eq!(params.payment_method.as_deref(), Some("pm_card_visa"));
        assert_eq!(
            params.return_url.as_deref(),
            Some("https://example.com/return")
        );
        assert_eq!(params.receipt_email.as_deref(), Some("receipt@example.com"));
        assert_eq!(params.setup_future_usage.as_deref(), Some("off_session"));
        assert_eq!(params.idempotency_key.as_deref(), Some("idem_conf_001"));
        assert!(params.validate().is_ok());

        let debug_str = format!("{params:?}");
        assert!(debug_str.contains("[REDACTED]"));
        assert!(!debug_str.contains("sensitive_tok_998877"));
    }

    #[test]
    fn test_serde_serialization_roundtrip() {
        let params = CreatePaymentIntentParams::new(4200, "usd")
            .unwrap()
            .capture_method("automatic")
            .description("Test charge")
            .idempotency_key("secret_skip_key");

        let json = serde_json::to_string(&params).expect("Serialization success");
        assert!(json.contains(r#""amount":4200"#));
        assert!(json.contains(r#""currency":"usd""#));
        assert!(json.contains(r#""capture_method":"automatic""#));
        assert!(json.contains(r#""description":"Test charge""#));
        // Idempotency key must be skipped from JSON body
        assert!(!json.contains("secret_skip_key"));
        assert!(!json.contains("idempotency_key"));

        // Roundtrip deserialization
        let deserialized: CreatePaymentIntentParams =
            serde_json::from_str(&json).expect("Deserialization success");
        assert_eq!(deserialized.amount, 4200);
        assert_eq!(deserialized.currency, "usd");
        assert_eq!(deserialized.capture_method.as_deref(), Some("automatic"));
        assert_eq!(deserialized.description.as_deref(), Some("Test charge"));
        assert!(deserialized.idempotency_key.is_none());
    }

    #[test]
    fn test_serde_capture_params_omits_none() {
        let capture = CapturePaymentIntentParams::new();
        let json = serde_json::to_string(&capture).expect("Serialization success");
        assert_eq!(json, "{}");

        let capture_with_amt = CapturePaymentIntentParams::with_amount(2000).unwrap();
        let json_with_amt =
            serde_json::to_string(&capture_with_amt).expect("Serialization success");
        assert_eq!(json_with_amt, r#"{"amount_to_capture":2000}"#);
    }
}
