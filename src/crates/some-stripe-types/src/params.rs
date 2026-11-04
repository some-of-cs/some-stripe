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

    /// Sets the capture method (alias for [`Self::capture_method`]).
    pub fn with_capture_method(self, capture_method: impl Into<String>) -> Self {
        self.capture_method(capture_method)
    }

    /// Sets the payment description (alias for [`Self::description`]).
    pub fn with_description(self, desc: impl Into<String>) -> Self {
        self.description(desc)
    }

    /// Sets the payment method identifier (alias for [`Self::payment_method`]).
    pub fn with_payment_method(self, pm: impl Into<String>) -> Self {
        self.payment_method(pm)
    }

    /// Sets the customer identifier (alias for [`Self::customer`]).
    pub fn with_customer(self, cus: impl Into<String>) -> Self {
        self.customer(cus)
    }

    /// Sets the receipt email (alias for [`Self::receipt_email`]).
    pub fn with_receipt_email(self, email: impl Into<String>) -> Self {
        self.receipt_email(email)
    }

    /// Sets the return URL (alias for [`Self::return_url`]).
    pub fn with_return_url(self, return_url: impl Into<String>) -> Self {
        self.return_url(return_url)
    }

    /// Sets whether to confirm the payment immediately (alias for [`Self::confirm`]).
    pub fn with_confirm(self, confirm: bool) -> Self {
        self.confirm(confirm)
    }

    /// Attaches an idempotency key (alias for [`Self::idempotency_key`]).
    pub fn with_idempotency_key(self, key: impl Into<String>) -> Self {
        self.idempotency_key(key)
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

/// Default payment mode for checkout sessions (`payment`).
fn default_payment_mode() -> String {
    "payment".into()
}

/// Parameters for the product associated with a line item's price data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LineItemProductDataParams {
    /// The product's name, meant to be displayable to the customer.
    pub name: String,

    /// The product's description, meant to be displayable to the customer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// A list of up to 8 URLs of images for this product.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

impl LineItemProductDataParams {
    /// Creates a new `LineItemProductDataParams` with the specified product name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            images: None,
        }
    }

    /// Sets the product description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the product image URLs.
    pub fn images(mut self, images: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.images = Some(images.into_iter().map(Into::into).collect());
        self
    }

    /// Appends an image URL to the product.
    pub fn add_image(mut self, image_url: impl Into<String>) -> Self {
        self.images
            .get_or_insert_with(Vec::new)
            .push(image_url.into());
        self
    }
}

/// Data used to generate a new Price object inline for a line item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineItemPriceDataParams {
    /// Three-letter ISO currency code, in lowercase. Must be a valid ISO 4217 code.
    pub currency: String,

    /// A non-negative integer in cents (or minor currency units) representing how much to charge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_amount: Option<i64>,

    /// Data used to generate a new product object inline.
    pub product_data: LineItemProductDataParams,
}

impl LineItemPriceDataParams {
    /// Creates a new `LineItemPriceDataParams` with currency and product data.
    pub fn new(currency: impl Into<String>, product_data: LineItemProductDataParams) -> Self {
        Self {
            currency: currency.into().to_ascii_lowercase(),
            unit_amount: None,
            product_data,
        }
    }

    /// Creates a new `LineItemPriceDataParams` with currency, unit amount, and product data.
    pub fn with_amount(
        currency: impl Into<String>,
        unit_amount: i64,
        product_data: LineItemProductDataParams,
    ) -> Self {
        Self {
            currency: currency.into().to_ascii_lowercase(),
            unit_amount: Some(unit_amount),
            product_data,
        }
    }

    /// Sets the unit amount in minor currency units.
    pub fn unit_amount(mut self, amount: i64) -> Self {
        self.unit_amount = Some(amount);
        self
    }

    /// Sets the currency code.
    pub fn currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into().to_ascii_lowercase();
        self
    }

    /// Sets the inline product data.
    pub fn product_data(mut self, product_data: LineItemProductDataParams) -> Self {
        self.product_data = product_data;
        self
    }
}

/// A line item being purchased in a checkout session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LineItemParams {
    /// The ID of the Price or Plan object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,

    /// The quantity of the line item being purchased.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<u32>,

    /// Data used to generate a new Price object inline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_data: Option<LineItemPriceDataParams>,
}

impl LineItemParams {
    /// Creates an empty `LineItemParams`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a `LineItemParams` referencing an existing price ID with a specified quantity.
    pub fn from_price(price: impl Into<String>, quantity: u32) -> Self {
        Self {
            price: Some(price.into()),
            quantity: Some(quantity),
            price_data: None,
        }
    }

    /// Creates a `LineItemParams` with inline price data and a specified quantity.
    pub fn from_price_data(price_data: LineItemPriceDataParams, quantity: u32) -> Self {
        Self {
            price: None,
            quantity: Some(quantity),
            price_data: Some(price_data),
        }
    }

    /// Sets the price ID.
    pub fn price(mut self, price: impl Into<String>) -> Self {
        self.price = Some(price.into());
        self
    }

    /// Sets the quantity.
    pub fn quantity(mut self, qty: u32) -> Self {
        self.quantity = Some(qty);
        self
    }

    /// Sets the price data.
    pub fn price_data(mut self, price_data: LineItemPriceDataParams) -> Self {
        self.price_data = Some(price_data);
        self
    }
}

/// Parameters for creating a Stripe Checkout Session (`POST /v1/checkout/sessions`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateCheckoutSessionParams {
    /// The URL the customer will be directed to upon successful payment.
    pub success_url: String,

    /// The URL the customer will be directed to if they decide to cancel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancel_url: Option<String>,

    /// The mode of the checkout session (`payment`, `setup`, or `subscription`). Default: `payment`.
    #[serde(default = "default_payment_mode")]
    pub mode: String,

    /// Unique string to reference the Checkout Session on your system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_reference_id: Option<String>,

    /// The ID of an existing customer for this session (`cus_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,

    /// If provided, this value will be used when the Customer object is created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer_email: Option<String>,

    /// Epoch timestamp at which the session expires (must be between 30 minutes and 24 hours).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,

    /// Idempotency key for safe retries. Not serialized into request body.
    #[serde(skip)]
    pub idempotency_key: Option<String>,

    /// A list of items the customer is purchasing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_items: Option<Vec<LineItemParams>>,

    /// Currency code (for single-item sessions; preserved for backward compatibility).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    /// The amount to be collected per unit in minor currency units (for single-item sessions; preserved for backward compatibility).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_amount: Option<i64>,

    /// Name of the product or service displayed at checkout (for single-item sessions; preserved for backward compatibility).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_name: Option<String>,

    /// Quantity of the item being purchased (for single-item sessions; preserved for backward compatibility).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<u32>,
}

impl Default for CreateCheckoutSessionParams {
    fn default() -> Self {
        Self {
            success_url: String::new(),
            cancel_url: None,
            mode: default_payment_mode(),
            client_reference_id: None,
            customer: None,
            customer_email: None,
            expires_at: None,
            metadata: None,
            idempotency_key: None,
            line_items: None,
            currency: None,
            unit_amount: None,
            product_name: None,
            quantity: None,
        }
    }
}

impl CreateCheckoutSessionParams {
    /// Creates a new `CreateCheckoutSessionParams` with the specified success URL.
    /// Mode defaults to `"payment"`.
    pub fn new(success_url: impl Into<String>) -> Self {
        Self {
            success_url: success_url.into(),
            ..Default::default()
        }
    }

    /// Backward-compatible constructor for creating a single-item Checkout Session.
    pub fn single_item(
        success_url: impl Into<String>,
        currency: impl Into<String>,
        unit_amount: i64,
        product_name: impl Into<String>,
        quantity: u32,
    ) -> Self {
        let curr = currency.into().to_ascii_lowercase();
        let prod = product_name.into();
        Self {
            success_url: success_url.into(),
            currency: Some(curr.clone()),
            unit_amount: Some(unit_amount),
            product_name: Some(prod.clone()),
            quantity: Some(quantity),
            line_items: Some(vec![LineItemParams {
                price: None,
                quantity: Some(quantity),
                price_data: Some(LineItemPriceDataParams {
                    currency: curr,
                    unit_amount: Some(unit_amount),
                    product_data: LineItemProductDataParams {
                        name: prod,
                        description: None,
                        images: None,
                    },
                }),
            }]),
            ..Default::default()
        }
    }

    /// Sets the cancel URL.
    pub fn cancel_url(mut self, url: impl Into<String>) -> Self {
        self.cancel_url = Some(url.into());
        self
    }

    /// Sets the session mode (`payment`, `setup`, or `subscription`).
    pub fn mode(mut self, mode: impl Into<String>) -> Self {
        self.mode = mode.into();
        self
    }

    /// Sets the client reference ID.
    pub fn client_reference_id(mut self, id: impl Into<String>) -> Self {
        self.client_reference_id = Some(id.into());
        self
    }

    /// Sets the customer ID.
    pub fn customer(mut self, customer: impl Into<String>) -> Self {
        self.customer = Some(customer.into());
        self
    }

    /// Sets the customer email.
    pub fn customer_email(mut self, email: impl Into<String>) -> Self {
        self.customer_email = Some(email.into());
        self
    }

    /// Sets the expiration timestamp (Unix epoch seconds).
    pub fn expires_at(mut self, expires_at: i64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Sets the metadata dictionary.
    pub fn metadata(
        mut self,
        metadata: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.metadata = Some(
            metadata
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
        self
    }

    /// Adds a single key-value metadata entry.
    pub fn add_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata
            .get_or_insert_with(std::collections::HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    /// Sets the idempotency key for safe retries.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Sets the list of line items.
    pub fn line_items(mut self, items: Vec<LineItemParams>) -> Self {
        self.line_items = Some(items);
        self
    }

    /// Appends a line item to the checkout session.
    pub fn add_line_item(mut self, item: LineItemParams) -> Self {
        self.line_items.get_or_insert_with(Vec::new).push(item);
        self
    }

    /// Backward-compatibility setter for currency.
    pub fn currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = Some(currency.into().to_ascii_lowercase());
        self
    }

    /// Backward-compatibility setter for unit_amount.
    pub fn unit_amount(mut self, amount: i64) -> Self {
        self.unit_amount = Some(amount);
        self
    }

    /// Backward-compatibility setter for product_name.
    pub fn product_name(mut self, name: impl Into<String>) -> Self {
        self.product_name = Some(name.into());
        self
    }

    /// Backward-compatibility setter for quantity.
    pub fn quantity(mut self, quantity: u32) -> Self {
        self.quantity = Some(quantity);
        self
    }

    /// Alias for [`cancel_url`].
    pub fn with_cancel_url(self, cancel_url: impl Into<String>) -> Self {
        self.cancel_url(cancel_url)
    }

    /// Alias for [`mode`].
    pub fn with_mode(self, mode: impl Into<String>) -> Self {
        self.mode(mode)
    }

    /// Alias for [`client_reference_id`].
    pub fn with_client_reference_id(self, id: impl Into<String>) -> Self {
        self.client_reference_id(id)
    }

    /// Alias for [`customer`].
    pub fn with_customer(self, customer: impl Into<String>) -> Self {
        self.customer(customer)
    }

    /// Alias for [`customer_email`].
    pub fn with_customer_email(self, email: impl Into<String>) -> Self {
        self.customer_email(email)
    }

    /// Alias for [`quantity`].
    pub fn with_quantity(self, quantity: u32) -> Self {
        self.quantity(quantity)
    }

    /// Alias for [`expires_at`].
    pub fn with_expires_at(self, expires_at: i64) -> Self {
        self.expires_at(expires_at)
    }

    /// Alias for [`idempotency_key`].
    pub fn with_idempotency_key(self, key: impl Into<String>) -> Self {
        self.idempotency_key(key)
    }

    /// Alias for [`line_items`].
    pub fn with_line_items(self, items: Vec<LineItemParams>) -> Self {
        self.line_items(items)
    }

    /// Alias for [`metadata`].
    pub fn with_metadata(
        self,
        metadata: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.metadata(metadata)
    }

    /// Validates the checkout session parameters according to Stripe requirements and Gate G-Law ISO 4217 rules.
    pub fn validate(&self) -> Result<(), PaymentIntentValidationError> {
        if self.success_url.trim().is_empty() {
            return Err(PaymentIntentValidationError::InvalidParameter {
                field: "success_url",
                reason: "success_url cannot be empty".to_string(),
            });
        }

        if let Some(ref items) = self.line_items {
            for item in items {
                if item.quantity == Some(0) {
                    return Err(PaymentIntentValidationError::InvalidParameter {
                        field: "line_items.quantity",
                        reason: "line item quantity must be greater than 0".to_string(),
                    });
                }
                if let Some(ref price_data) = item.price_data {
                    validate_currency(&price_data.currency)?;
                    if let Some(amt) = price_data.unit_amount {
                        validate_amount(amt)?;
                    }
                    if price_data.product_data.name.trim().is_empty() {
                        return Err(PaymentIntentValidationError::InvalidParameter {
                            field: "product_data.name",
                            reason: "product name cannot be empty".to_string(),
                        });
                    }
                }
            }
        }

        if let Some(ref curr) = self.currency {
            validate_currency(curr)?;
        }
        if let Some(amt) = self.unit_amount {
            validate_amount(amt)?;
        }
        if self.quantity == Some(0) {
            return Err(PaymentIntentValidationError::InvalidParameter {
                field: "quantity",
                reason: "quantity must be greater than 0".to_string(),
            });
        }

        Ok(())
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
