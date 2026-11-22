//! Pure, side-effect-free mappers between `some-stripe` and `some-payment-core` types.
//!
//! All multi-branch status and error classifications use static lookup tables
//! to keep cyclomatic complexity bounded (`CC <= 6`) and guarantee panic-free
//! execution on untrusted API or webhook payloads.

use serde_json::Value;
use some_payment_core::{
    Money, NextAction, NormalizedWebhookEvent, PaymentCapabilities, PaymentError, PaymentResult,
    PaymentStatus, ProviderId, ProviderReference, RefundResult, RefundStatus,
};
use some_stripe_core::StripeError;
use some_stripe_types::currency_guardrails::validate_stripe_money;
use some_stripe_types::payment_error_detail::THREE_DS_FAILURE_CODES;

/// Canonical lowercase provider slug identifying Stripe inside `some-payment`.
pub const STRIPE_PROVIDER_SLUG: &str = "stripe";

const KNOWN_PI_STATUSES: &[(&str, PaymentStatus)] = &[
    ("requires_payment_method", PaymentStatus::Initiated),
    ("requires_confirmation", PaymentStatus::Initiated),
    ("requires_action", PaymentStatus::RequiresAction),
    ("processing", PaymentStatus::Processing),
    ("requires_capture", PaymentStatus::Authorized),
    ("canceled", PaymentStatus::Canceled),
    ("succeeded", PaymentStatus::Paid),
];

const KNOWN_CHECKOUT_SESSION_STATUSES: &[(&str, PaymentStatus)] = &[
    ("open", PaymentStatus::Initiated),
    ("complete", PaymentStatus::Paid),
    ("expired", PaymentStatus::Canceled),
];

const KNOWN_CHECKOUT_PAYMENT_STATUSES: &[(&str, PaymentStatus)] = &[
    ("paid", PaymentStatus::Paid),
    ("no_payment_required", PaymentStatus::Paid),
    ("unpaid", PaymentStatus::Initiated),
];

const KNOWN_REFUND_STATUSES: &[(&str, RefundStatus)] = &[
    ("pending", RefundStatus::Pending),
    ("requires_action", RefundStatus::RequiresAction),
    ("succeeded", RefundStatus::Succeeded),
    ("failed", RefundStatus::Failed),
    ("canceled", RefundStatus::Canceled),
];

const KNOWN_WEBHOOK_EVENT_STATUSES: &[(&str, PaymentStatus)] = &[
    ("payment_intent.succeeded", PaymentStatus::Paid),
    (
        "payment_intent.amount_capturable_updated",
        PaymentStatus::Authorized,
    ),
    ("payment_intent.processing", PaymentStatus::Processing),
    (
        "payment_intent.requires_action",
        PaymentStatus::RequiresAction,
    ),
    ("payment_intent.canceled", PaymentStatus::Canceled),
    ("payment_intent.payment_failed", PaymentStatus::Failed),
    ("checkout.session.completed", PaymentStatus::Paid),
    (
        "checkout.session.async_payment_succeeded",
        PaymentStatus::Paid,
    ),
    (
        "checkout.session.async_payment_failed",
        PaymentStatus::Failed,
    ),
    ("checkout.session.expired", PaymentStatus::Canceled),
    ("charge.succeeded", PaymentStatus::Paid),
    ("charge.captured", PaymentStatus::Captured),
    ("charge.failed", PaymentStatus::Failed),
    ("charge.refunded", PaymentStatus::Refunded),
];

const TERMINAL_FAILURE_WEBHOOK_EVENTS: &[&str] = &[
    "checkout.session.async_payment_failed",
    "payment_intent.payment_failed",
];

const SENSITIVE_KEY_SUBSTRINGS: &[&str] = &[
    "secret",
    "token",
    "password",
    "api_key",
    "apikey",
    "signature",
    "authorization",
    "bearer",
    "credential",
    "private_key",
    "passphrase",
];

/// Metadata describing a verified Stripe webhook event envelope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebhookEventMeta<'a> {
    /// Provider-unique event identifier (e.g., `"evt_..."`).
    pub event_id: &'a str,
    /// Raw Stripe event type (e.g., `"payment_intent.succeeded"`).
    pub event_type: &'a str,
    /// Whether the event originated in live mode (`false` for test mode).
    pub livemode: bool,
    /// Whether this is a thin v2 event requiring API hydration.
    pub is_thin_event: bool,
}

struct StripeHttpErrorFields<'a> {
    status: u16,
    code: Option<&'a str>,
    decline_code: Option<&'a str>,
    advice_code: Option<&'a str>,
    retry_after_secs: Option<u64>,
    message: String,
}

type EventExtractedFields = (
    Option<ProviderReference>,
    Option<PaymentStatus>,
    Option<Money>,
);

/// Returns the canonical [`ProviderId`] for Stripe (`"stripe"`).
///
/// # Examples
///
/// ```
/// use some_stripe::adapter::mappers::{STRIPE_PROVIDER_SLUG, stripe_provider_id};
///
/// assert_eq!(stripe_provider_id().as_str(), STRIPE_PROVIDER_SLUG);
/// ```
pub fn stripe_provider_id() -> ProviderId {
    ProviderId::new(STRIPE_PROVIDER_SLUG)
}

/// Returns the fail-closed [`PaymentCapabilities`] bitmask supported by Stripe.
///
/// # Examples
///
/// ```
/// use some_payment_core::PaymentCapabilities;
/// use some_stripe::adapter::mappers::map_stripe_capabilities;
///
/// let caps = map_stripe_capabilities();
/// assert!(caps.contains(PaymentCapabilities::EMBEDDED_INTENT));
/// assert!(caps.contains(PaymentCapabilities::NATIVE_MUTATION_IDEMPOTENCY));
/// assert!(!caps.contains(PaymentCapabilities::WEBHOOK_VERIFICATION_HANDSHAKE));
/// ```
pub fn map_stripe_capabilities() -> PaymentCapabilities {
    PaymentCapabilities::DIRECT_PAYMENT
        | PaymentCapabilities::EMBEDDED_INTENT
        | PaymentCapabilities::HOSTED_CHECKOUT
        | PaymentCapabilities::AUTH_CAPTURE
        | PaymentCapabilities::PARTIAL_CAPTURE
        | PaymentCapabilities::VOID_AUTHORIZED
        | PaymentCapabilities::REFUND
        | PaymentCapabilities::PARTIAL_REFUND
        | PaymentCapabilities::NATIVE_CREATE_IDEMPOTENCY
        | PaymentCapabilities::NATIVE_MUTATION_IDEMPOTENCY
        | PaymentCapabilities::SAVED_CARDS
}

fn lookup_table<T: Copy>(table: &[(&str, T)], raw: &str) -> Option<T> {
    let normalized = raw.trim().to_ascii_lowercase();
    table
        .iter()
        .find(|(key, _)| *key == normalized)
        .map(|(_, value)| *value)
}

fn is_ci_eq(value: Option<&str>, expected: &str) -> bool {
    value.is_some_and(|raw| raw.trim().eq_ignore_ascii_case(expected))
}

/// Maps a Stripe `PaymentIntent` status string to a canonical [`PaymentStatus`].
///
/// When Stripe reports `"requires_payment_method"` alongside a non-null
/// `last_payment_error`, the previous attempt failed and this function returns
/// [`PaymentStatus::Failed`].
///
/// # Examples
///
/// ```
/// use some_payment_core::PaymentStatus;
/// use some_stripe::adapter::mappers::map_payment_intent_status;
///
/// assert_eq!(
///     map_payment_intent_status("requires_capture", false),
///     PaymentStatus::Authorized
/// );
/// assert_eq!(
///     map_payment_intent_status("requires_payment_method", false),
///     PaymentStatus::Initiated
/// );
/// assert_eq!(
///     map_payment_intent_status("requires_payment_method", true),
///     PaymentStatus::Failed
/// );
/// ```
pub fn map_payment_intent_status(status: &str, has_last_payment_error: bool) -> PaymentStatus {
    let normalized = status.trim().to_ascii_lowercase();
    if normalized == "requires_payment_method" && has_last_payment_error {
        return PaymentStatus::Failed;
    }
    lookup_table(KNOWN_PI_STATUSES, &normalized).unwrap_or(PaymentStatus::Initiated)
}

/// Maps Stripe Checkout Session `status` and `payment_status` fields to [`PaymentStatus`].
///
/// # Examples
///
/// ```
/// use some_payment_core::PaymentStatus;
/// use some_stripe::adapter::mappers::map_checkout_status;
///
/// assert_eq!(
///     map_checkout_status(Some("expired"), Some("unpaid")),
///     PaymentStatus::Canceled
/// );
/// assert_eq!(
///     map_checkout_status(Some("complete"), Some("paid")),
///     PaymentStatus::Paid
/// );
/// assert_eq!(
///     map_checkout_status(Some("complete"), Some("unpaid")),
///     PaymentStatus::Processing
/// );
/// ```
pub fn map_checkout_status(status: Option<&str>, payment_status: Option<&str>) -> PaymentStatus {
    if is_ci_eq(status, "expired") {
        return PaymentStatus::Canceled;
    }
    let payment_mapped =
        payment_status.and_then(|s| lookup_table(KNOWN_CHECKOUT_PAYMENT_STATUSES, s));
    if payment_mapped == Some(PaymentStatus::Paid) {
        return PaymentStatus::Paid;
    }
    if is_ci_eq(status, "complete") && is_ci_eq(payment_status, "unpaid") {
        return PaymentStatus::Processing;
    }
    status
        .and_then(|s| lookup_table(KNOWN_CHECKOUT_SESSION_STATUSES, s))
        .or(payment_mapped)
        .unwrap_or(PaymentStatus::Initiated)
}

/// Maps a Stripe refund status string to a canonical [`RefundStatus`].
///
/// Covers all five Stripe refund states (`pending`, `requires_action`,
/// `succeeded`, `failed`, `canceled`).
///
/// # Examples
///
/// ```
/// use some_payment_core::RefundStatus;
/// use some_stripe::adapter::mappers::map_refund_status;
///
/// assert_eq!(map_refund_status("succeeded"), RefundStatus::Succeeded);
/// assert_eq!(
///     map_refund_status("requires_action"),
///     RefundStatus::RequiresAction
/// );
/// ```
pub fn map_refund_status(status: &str) -> RefundStatus {
    lookup_table(KNOWN_REFUND_STATUSES, status).unwrap_or(RefundStatus::Pending)
}

fn non_empty_str(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn is_three_ds_code(code: &str) -> bool {
    let lower = code.trim().to_ascii_lowercase();
    THREE_DS_FAILURE_CODES.contains(&lower.as_str()) || lower.contains("3ds")
}

fn map_card_or_three_ds_error(fields: StripeHttpErrorFields<'_>) -> PaymentError {
    let provider = stripe_provider_id();
    let is_three_ds = fields.code.is_some_and(is_three_ds_code)
        || fields.decline_code.is_some_and(is_three_ds_code);
    if is_three_ds {
        return PaymentError::ThreeDsFailed {
            provider,
            code: non_empty_str(fields.code.or(fields.decline_code)),
            message: fields.message,
        };
    }
    PaymentError::CardDeclined {
        provider,
        decline_code: non_empty_str(fields.decline_code.or(fields.code)),
        advice_code: non_empty_str(fields.advice_code),
        message: fields.message,
    }
}

fn map_stripe_http_error(fields: StripeHttpErrorFields<'_>) -> PaymentError {
    let provider = stripe_provider_id();
    match fields.status {
        401 => PaymentError::AuthenticationError {
            provider,
            message: fields.message,
        },
        402 => map_card_or_three_ds_error(fields),
        429 => PaymentError::RateLimited {
            provider,
            retry_after_secs: fields.retry_after_secs,
        },
        408 | 500..=599 => PaymentError::ProviderUnavailable {
            provider,
            http_status: Some(fields.status),
            indeterminate: matches!(fields.status, 408 | 504),
            message: fields.message,
        },
        _ => PaymentError::ProviderError {
            provider,
            http_status: Some(fields.status),
            vendor_code: non_empty_str(fields.code),
            message: fields.message,
        },
    }
}

fn map_stripe_client_error(err: &StripeError) -> Option<PaymentError> {
    match err {
        StripeError::InvalidRequest(msg) => Some(PaymentError::validation(msg.clone())),
        StripeError::Config(msg) => Some(PaymentError::configuration(msg.clone())),
        StripeError::Webhook(wh_err) => Some(PaymentError::WebhookVerificationFailed {
            provider: stripe_provider_id(),
            reason: wh_err.to_string(),
        }),
        StripeError::Redirect(redir_err) => Some(PaymentError::validation(redir_err.to_string())),
        _ => None,
    }
}

fn map_stripe_transport_or_api_error(err: &StripeError) -> PaymentError {
    match err {
        StripeError::Api {
            status,
            code,
            message,
            ..
        } => map_stripe_http_error(StripeHttpErrorFields {
            status: *status,
            code: Some(code.as_str()),
            decline_code: None,
            advice_code: None,
            retry_after_secs: None,
            message: message.clone(),
        }),
        StripeError::Http(http_err) => PaymentError::ProviderUnavailable {
            provider: stripe_provider_id(),
            http_status: http_err.status().map(|s| s.as_u16()),
            indeterminate: true,
            message: http_err.to_string(),
        },
        StripeError::ConnectionError(msg) => PaymentError::ProviderUnavailable {
            provider: stripe_provider_id(),
            http_status: None,
            indeterminate: true,
            message: msg.clone(),
        },
        StripeError::Json(json_err) => PaymentError::ProviderError {
            provider: stripe_provider_id(),
            http_status: None,
            vendor_code: Some("json_decode_error".to_owned()),
            message: json_err.to_string(),
        },
        _ => PaymentError::ProviderError {
            provider: stripe_provider_id(),
            http_status: None,
            vendor_code: Some("stripe_client_error".to_owned()),
            message: err.to_string(),
        },
    }
}

/// Maps a [`StripeError`] into a canonical, secret-redacted [`PaymentError`].
///
/// # Examples
///
/// ```
/// use some_payment_core::ErrorKind;
/// use some_stripe::adapter::mappers::map_stripe_error;
/// use some_stripe_core::StripeError;
///
/// let err = StripeError::InvalidRequest("missing currency".into());
/// assert_eq!(map_stripe_error(&err).kind(), ErrorKind::Validation);
/// ```
pub fn map_stripe_error(err: &StripeError) -> PaymentError {
    map_stripe_client_error(err).unwrap_or_else(|| map_stripe_transport_or_api_error(err))
}

/// Maps an HTTP status code and Stripe JSON error body into a [`PaymentError`].
///
/// Handles `401` (`AuthenticationError`), `402` (`ThreeDsFailed` for 3DS codes
/// or `CardDeclined` with `decline_code` and `advice_code`), `429`
/// (`RateLimited`), `408`/`5xx` (`ProviderUnavailable`), and other statuses
/// (`ProviderError`).
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use some_payment_core::ErrorKind;
/// use some_stripe::adapter::mappers::map_stripe_error_payload;
///
/// let body = json!({
///     "error": {
///         "code": "card_declined",
///         "decline_code": "insufficient_funds",
///         "advice_code": "try_again_later",
///         "message": "Your card has insufficient funds."
///     }
/// });
/// let err = map_stripe_error_payload(402, &body);
/// assert_eq!(err.kind(), ErrorKind::CardDeclined);
/// assert_eq!(err.decline_code(), Some("insufficient_funds"));
/// ```
pub fn map_stripe_error_payload(status: u16, body: &Value) -> PaymentError {
    map_stripe_error_payload_with_retry_after(status, body, None)
}

/// Maps an HTTP status code, Stripe JSON error body, and optional HTTP
/// `Retry-After` duration (in seconds) into a [`PaymentError`].
///
/// Prefers `retry_after_secs` from the HTTP `Retry-After` header over any
/// JSON body fallback when constructing [`PaymentError::RateLimited`].
pub fn map_stripe_error_payload_with_retry_after(
    status: u16,
    body: &Value,
    retry_after_secs: Option<u64>,
) -> PaymentError {
    let err_obj = body.get("error").unwrap_or(body);
    let fields = StripeHttpErrorFields {
        status,
        code: err_obj.get("code").and_then(Value::as_str),
        decline_code: err_obj.get("decline_code").and_then(Value::as_str),
        advice_code: err_obj.get("advice_code").and_then(Value::as_str),
        retry_after_secs: retry_after_secs.or_else(|| {
            body.get("retry_after_secs")
                .or_else(|| err_obj.get("retry_after_secs"))
                .and_then(Value::as_u64)
        }),
        message: err_obj
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Stripe API error")
            .to_owned(),
    };
    map_stripe_http_error(fields)
}

fn non_empty_trimmed(raw: Option<String>) -> Option<String> {
    raw.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

/// Maps optional Stripe `client_secret` and redirect URL values into a [`NextAction`].
///
/// Redirect actions take precedence over embedded client secrets when both are
/// present (e.g., 3DS redirect challenges on a `PaymentIntent`).
///
/// # Examples
///
/// ```
/// use some_payment_core::NextAction;
/// use some_stripe::adapter::mappers::map_next_action;
///
/// let action = map_next_action(Some("pi_123_secret_xyz".into()), None).unwrap();
/// assert!(matches!(action, NextAction::EmbeddedClientSecret { .. }));
/// assert!(!format!("{action:?}").contains("pi_123_secret_xyz"));
/// ```
pub fn map_next_action(
    client_secret: Option<String>,
    redirect_url: Option<String>,
) -> Option<NextAction> {
    if let Some(url) = non_empty_trimmed(redirect_url) {
        return Some(NextAction::Redirect { url });
    }
    non_empty_trimmed(client_secret)
        .map(|client_secret| NextAction::EmbeddedClientSecret { client_secret })
}

fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    SENSITIVE_KEY_SUBSTRINGS
        .iter()
        .any(|needle| lower.contains(needle))
}

fn scrub_object_entry((key, val): (&String, &Value)) -> (String, Value) {
    let scrubbed = if is_sensitive_key(key) {
        Value::String("[REDACTED]".to_owned())
    } else {
        scrub_webhook_payload(val)
    };
    (key.clone(), scrubbed)
}

/// Recursively scrubs sensitive fields from a webhook JSON payload.
///
/// Any JSON object entry whose key contains `secret`, `token`, `password`,
/// `api_key`, or `signature` (case-insensitively) has its value replaced with
/// `"[REDACTED]"`.
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use some_stripe::adapter::mappers::scrub_webhook_payload;
///
/// let scrubbed = scrub_webhook_payload(&json!({
///     "id": "evt_1",
///     "client_secret": "pi_secret_123"
/// }));
/// assert_eq!(scrubbed["id"], "evt_1");
/// assert_eq!(scrubbed["client_secret"], "[REDACTED]");
/// ```
pub fn scrub_webhook_payload(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(map.iter().map(scrub_object_entry).collect()),
        Value::Array(items) => Value::Array(items.iter().map(scrub_webhook_payload).collect()),
        _ => value.clone(),
    }
}

fn required_str<'a>(val: &'a Value, field: &str) -> Result<&'a str, PaymentError> {
    val.get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| PaymentError::validation(format!("missing required Stripe field '{field}'")))
}

fn required_u64(val: &Value, field: &str) -> Result<u64, PaymentError> {
    val.get(field).and_then(Value::as_u64).ok_or_else(|| {
        PaymentError::validation(format!("missing required Stripe integer '{field}'"))
    })
}

fn extract_id_or_str(val: &Value) -> Option<&str> {
    val.as_str()
        .or_else(|| val.get("id").and_then(Value::as_str))
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// Maps a Stripe `PaymentIntent` JSON object into a canonical [`PaymentResult`].
///
/// # Errors
///
/// Returns [`PaymentError::ValidationError`] if `id`, `amount`, or `currency`
/// is missing or invalid.
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use some_payment_core::PaymentStatus;
/// use some_stripe::adapter::mappers::map_payment_intent_json;
///
/// let pi = json!({
///     "id": "pi_3ABC",
///     "amount": 5000,
///     "currency": "usd",
///     "status": "succeeded",
///     "amount_received": 5000,
///     "livemode": false
/// });
/// let res = map_payment_intent_json(&pi).unwrap();
/// assert_eq!(res.status, PaymentStatus::Paid);
/// assert_eq!(res.reference.intent_id(), Some("pi_3ABC"));
/// ```
pub fn map_payment_intent_json(val: &Value) -> Result<PaymentResult, PaymentError> {
    let id = required_str(val, "id")?;
    let amount = required_u64(val, "amount")?;
    let currency = required_str(val, "currency")?;
    let money = Money::new(amount, currency)?;
    let reference = ProviderReference::builder(stripe_provider_id())
        .payment_id(id)
        .intent_id(id)
        .build()?;
    let raw_status = val
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("requires_payment_method");
    let has_last_error = val.get("last_payment_error").is_some_and(|v| !v.is_null());
    let status = map_payment_intent_status(raw_status, has_last_error);
    let captured_amount = val.get("amount_received").and_then(Value::as_u64);
    let refunded_amount = val
        .get("amount_refunded")
        .and_then(Value::as_u64)
        .or_else(|| {
            val.pointer("/latest_charge/amount_refunded")
                .and_then(Value::as_u64)
        });
    let client_secret = val
        .get("client_secret")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let redirect_url = val
        .pointer("/next_action/redirect_to_url/url")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let livemode = val
        .get("livemode")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Ok(PaymentResult {
        reference,
        status,
        money,
        captured_amount,
        refunded_amount,
        next_action: map_next_action(client_secret, redirect_url),
        checkout_url: None,
        livemode,
    })
}

/// Maps a Stripe Checkout Session JSON object into a canonical [`PaymentResult`].
///
/// # Errors
///
/// Returns [`PaymentError::ValidationError`] if `id`, `amount_total` (or
/// `amount_subtotal`), or `currency` is missing or invalid.
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use some_payment_core::PaymentStatus;
/// use some_stripe::adapter::mappers::map_checkout_session_json;
///
/// let cs = json!({
///     "id": "cs_test_123",
///     "amount_total": 2500,
///     "currency": "usd",
///     "status": "open",
///     "payment_status": "unpaid",
///     "url": "https://checkout.stripe.com/c/pay/cs_test_123",
///     "livemode": false
/// });
/// let res = map_checkout_session_json(&cs).unwrap();
/// assert_eq!(res.status, PaymentStatus::Initiated);
/// assert_eq!(res.reference.checkout_session_id(), Some("cs_test_123"));
/// ```
pub fn map_checkout_session_json(val: &Value) -> Result<PaymentResult, PaymentError> {
    let session_id = required_str(val, "id")?;
    let amount = val
        .get("amount_total")
        .or_else(|| val.get("amount_subtotal"))
        .and_then(Value::as_u64)
        .ok_or_else(|| PaymentError::validation("missing required Checkout Session amount"))?;
    let currency = required_str(val, "currency")?;
    let money = Money::new(amount, currency)?;
    let mut ref_builder =
        ProviderReference::builder(stripe_provider_id()).checkout_session_id(session_id);
    if let Some(pi_id) = val.get("payment_intent").and_then(extract_id_or_str) {
        ref_builder = ref_builder.payment_id(pi_id).intent_id(pi_id);
    }
    if let Some(order_id) = val.get("client_reference_id").and_then(Value::as_str) {
        ref_builder = ref_builder.order_id(order_id);
    }
    let reference = ref_builder.build()?;
    let status = map_checkout_status(
        val.get("status").and_then(Value::as_str),
        val.get("payment_status").and_then(Value::as_str),
    );
    let checkout_url = non_empty_str(val.get("url").and_then(Value::as_str));
    let captured_amount = (status == PaymentStatus::Paid).then_some(money.amount());
    let livemode = val
        .get("livemode")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Ok(PaymentResult {
        reference,
        status,
        money,
        captured_amount,
        refunded_amount: None,
        next_action: map_next_action(None, checkout_url.clone()),
        checkout_url,
        livemode,
    })
}

/// Maps a Stripe Refund JSON object into a canonical [`RefundResult`].
///
/// # Errors
///
/// Returns [`PaymentError::ValidationError`] if `id`, `amount`, or `currency`
/// is missing or invalid.
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use some_payment_core::{ProviderReference, RefundStatus};
/// use some_stripe::adapter::mappers::{map_refund_json, stripe_provider_id};
///
/// let payment_ref = ProviderReference::builder(stripe_provider_id())
///     .payment_id("pi_123")
///     .build()
///     .unwrap();
/// let refund = json!({
///     "id": "re_123",
///     "amount": 1000,
///     "currency": "usd",
///     "status": "succeeded",
///     "reason": "requested_by_customer"
/// });
/// let res = map_refund_json(&refund, &payment_ref).unwrap();
/// assert_eq!(res.status, RefundStatus::Succeeded);
/// assert_eq!(res.refund_id, "re_123");
/// ```
pub fn map_refund_json(
    val: &Value,
    payment_ref: &ProviderReference,
) -> Result<RefundResult, PaymentError> {
    let refund_id = required_str(val, "id")?.to_owned();
    let amount = required_u64(val, "amount")?;
    let currency = required_str(val, "currency")?;
    let money = Money::new(amount, currency)?;
    let raw_status = val
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("pending");
    let reason_detail = non_empty_str(
        val.get("failure_reason")
            .or_else(|| val.get("pending_reason"))
            .or_else(|| val.get("reason"))
            .and_then(Value::as_str),
    );
    Ok(RefundResult {
        refund_id,
        payment_reference: payment_ref.clone(),
        status: map_refund_status(raw_status),
        money,
        reason_detail,
    })
}

fn extract_checkout_webhook_ref(obj: &Value, id: Option<&str>) -> Option<ProviderReference> {
    let mut builder = ProviderReference::builder(stripe_provider_id());
    if let Some(cs_id) = id {
        builder = builder.checkout_session_id(cs_id);
    }
    if let Some(pi_id) = obj.get("payment_intent").and_then(extract_id_or_str) {
        builder = builder.payment_id(pi_id).intent_id(pi_id);
    }
    builder.build().ok()
}

fn extract_webhook_reference(obj: &Value) -> Option<ProviderReference> {
    let object_kind = obj.get("object").and_then(Value::as_str).unwrap_or("");
    let id = obj.get("id").and_then(Value::as_str);
    if object_kind == "checkout.session" {
        return extract_checkout_webhook_ref(obj, id);
    }
    let mut builder = ProviderReference::builder(stripe_provider_id());
    if let Some(pi_id) = obj.get("payment_intent").and_then(extract_id_or_str) {
        builder = builder.payment_id(pi_id).intent_id(pi_id);
    } else if let Some(primary_id) = id {
        builder = builder.payment_id(primary_id);
        if object_kind == "payment_intent" || primary_id.starts_with("pi_") {
            builder = builder.intent_id(primary_id);
        }
    }
    builder.build().ok()
}

fn extract_webhook_status(event_type: &str, obj: &Value) -> Option<PaymentStatus> {
    let normalized_event = event_type.trim().to_ascii_lowercase();
    if TERMINAL_FAILURE_WEBHOOK_EVENTS.contains(&normalized_event.as_str()) {
        return Some(PaymentStatus::Failed);
    }
    let object_kind = obj.get("object").and_then(Value::as_str).unwrap_or("");
    if object_kind == "payment_intent" {
        let raw_status = obj.get("status").and_then(Value::as_str)?;
        let has_err = obj.get("last_payment_error").is_some_and(|v| !v.is_null());
        return Some(map_payment_intent_status(raw_status, has_err));
    }
    if object_kind == "checkout.session" {
        return Some(map_checkout_status(
            obj.get("status").and_then(Value::as_str),
            obj.get("payment_status").and_then(Value::as_str),
        ));
    }
    lookup_table(KNOWN_WEBHOOK_EVENT_STATUSES, event_type)
}

fn extract_webhook_money(obj: &Value) -> Option<Money> {
    let amount = obj
        .get("amount")
        .or_else(|| obj.get("amount_total"))
        .or_else(|| obj.get("amount_subtotal"))
        .and_then(Value::as_u64)?;
    let currency = obj.get("currency").and_then(Value::as_str)?;
    Money::new(amount, currency).ok()
}

fn extract_thin_reference(payload: &Value) -> Option<ProviderReference> {
    let related_id = payload
        .pointer("/related_object/id")
        .and_then(Value::as_str)?;
    ProviderReference::builder(stripe_provider_id())
        .payment_id(related_id)
        .build()
        .ok()
}

fn extract_event_fields(meta: &WebhookEventMeta<'_>, payload: &Value) -> EventExtractedFields {
    if meta.is_thin_event {
        return (extract_thin_reference(payload), None, None);
    }
    let obj = payload.pointer("/data/object");
    let reference = obj.and_then(extract_webhook_reference);
    let status = obj
        .and_then(|o| extract_webhook_status(meta.event_type, o))
        .or_else(|| lookup_table(KNOWN_WEBHOOK_EVENT_STATUSES, meta.event_type));
    let money = obj.and_then(extract_webhook_money);
    (reference, status, money)
}

/// Maps a verified Stripe webhook JSON payload into a [`NormalizedWebhookEvent`].
///
/// Always scrubs sensitive keys from `raw_redacted`. Thin events set
/// `requires_hydration = true` and leave `status` and `money` empty pending
/// API hydration.
///
/// # Errors
///
/// Returns [`PaymentError::ValidationError`] if `meta.event_id` or
/// `meta.event_type` is empty or whitespace-only.
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use some_payment_core::PaymentStatus;
/// use some_stripe::adapter::mappers::{WebhookEventMeta, map_webhook_json_to_normalized};
///
/// let payload = json!({
///     "data": {
///         "object": {
///             "object": "payment_intent",
///             "id": "pi_wh_1",
///             "amount": 4200,
///             "currency": "usd",
///             "status": "succeeded",
///             "client_secret": "pi_wh_1_secret_hidden"
///         }
///     }
/// });
/// let meta = WebhookEventMeta {
///     event_id: "evt_1",
///     event_type: "payment_intent.succeeded",
///     livemode: false,
///     is_thin_event: false,
/// };
/// let event = map_webhook_json_to_normalized(&meta, &payload).unwrap();
/// assert_eq!(event.status, Some(PaymentStatus::Paid));
/// assert_eq!(
///     event.raw_redacted["data"]["object"]["client_secret"],
///     "[REDACTED]"
/// );
/// ```
pub fn map_webhook_json_to_normalized(
    meta: &WebhookEventMeta<'_>,
    payload: &Value,
) -> Result<NormalizedWebhookEvent, PaymentError> {
    if meta.event_id.trim().is_empty() {
        return Err(PaymentError::validation(
            "webhook event_id must not be empty",
        ));
    }
    if meta.event_type.trim().is_empty() {
        return Err(PaymentError::validation(
            "webhook event_type must not be empty",
        ));
    }
    let (reference, status, money) = extract_event_fields(meta, payload);
    Ok(NormalizedWebhookEvent {
        provider_id: stripe_provider_id(),
        event_id: meta.event_id.to_owned(),
        event_type: meta.event_type.to_owned(),
        livemode: meta.livemode,
        requires_hydration: meta.is_thin_event,
        reference,
        status,
        money,
        verification_challenge: None,
        raw_redacted: scrub_webhook_payload(payload),
    })
}

/// Validates [`Money`] against Stripe's currency-specific minor-unit divisibility
/// rules and returns `(amount, lowercase_currency)`.
///
/// Stripe requires two-decimal legacy currencies (`isk`, `ugx`) to be divisible
/// by `100` and three-decimal currencies (`bhd`, `jod`, `kwd`, `omr`, `tnd`) to
/// be divisible by `10` (last digit zero).
///
/// # Errors
///
/// Returns [`PaymentError::ValidationError`] when a guarded currency violates
/// Stripe's minor-unit divisibility invariant.
///
/// # Examples
///
/// ```
/// use some_payment_core::Money;
/// use some_stripe::adapter::mappers::validate_stripe_money_for_adapter;
///
/// let kwd_ok = Money::new(1500, "KWD").unwrap();
/// assert_eq!(
///     validate_stripe_money_for_adapter(&kwd_ok).unwrap(),
///     (1500, "kwd".into())
/// );
///
/// let kwd_bad = Money::new(1505, "KWD").unwrap();
/// assert!(validate_stripe_money_for_adapter(&kwd_bad).is_err());
/// ```
pub fn validate_stripe_money_for_adapter(money: &Money) -> Result<(u64, String), PaymentError> {
    let amount_i64 = i64::try_from(money.amount()).map_err(|_| {
        PaymentError::validation(format!(
            "Stripe minor-unit amount {} exceeds i64::MAX",
            money.amount()
        ))
    })?;
    let validated = validate_stripe_money(amount_i64, money.currency().as_str())
        .map_err(|err| PaymentError::validation(err.to_string()))?;
    Ok((money.amount(), validated.currency))
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use some_payment_core::{
        ErrorKind, Money, NextAction, PaymentCapabilities, PaymentError, PaymentStatus,
        RefundStatus,
    };
    use some_stripe_core::{RedirectError, StripeError, StripeWebhookError};

    use super::*;

    fn sample_api_error(status: u16, code: &str) -> StripeError {
        StripeError::Api {
            status,
            code: code.into(),
            message: "stripe api failure".into(),
            error_type: "api_error".into(),
            doc_url: None,
        }
    }

    fn assert_currency_guardrail(currencies: &[&str], valid_amount: u64, invalid_amount: u64) {
        for currency in currencies {
            let valid = Money::new(valid_amount, currency).unwrap();
            assert_eq!(
                validate_stripe_money_for_adapter(&valid).unwrap(),
                (valid_amount, currency.to_ascii_lowercase())
            );
            let invalid = Money::new(invalid_amount, currency).unwrap();
            assert_eq!(
                validate_stripe_money_for_adapter(&invalid)
                    .unwrap_err()
                    .kind(),
                ErrorKind::Validation
            );
        }
    }

    #[test]
    fn tc_9_2_2_provider_id_and_capabilities() {
        assert_eq!(STRIPE_PROVIDER_SLUG, "stripe");
        assert_eq!(stripe_provider_id().as_str(), "stripe");
        let caps = map_stripe_capabilities();
        let expected = PaymentCapabilities::DIRECT_PAYMENT
            | PaymentCapabilities::EMBEDDED_INTENT
            | PaymentCapabilities::HOSTED_CHECKOUT
            | PaymentCapabilities::AUTH_CAPTURE
            | PaymentCapabilities::PARTIAL_CAPTURE
            | PaymentCapabilities::VOID_AUTHORIZED
            | PaymentCapabilities::REFUND
            | PaymentCapabilities::PARTIAL_REFUND
            | PaymentCapabilities::NATIVE_CREATE_IDEMPOTENCY
            | PaymentCapabilities::NATIVE_MUTATION_IDEMPOTENCY
            | PaymentCapabilities::SAVED_CARDS;
        assert_eq!(caps, expected);
        assert!(!caps.contains(PaymentCapabilities::SPLITS));
        assert!(!caps.contains(PaymentCapabilities::VOID_SETTLED_WINDOW));
        assert!(!caps.contains(PaymentCapabilities::WEBHOOK_VERIFICATION_HANDSHAKE));
    }

    #[test]
    fn tc_9_2_2_all_payment_intent_statuses_and_last_payment_error() {
        let cases = [
            ("requires_payment_method", false, PaymentStatus::Initiated),
            ("requires_payment_method", true, PaymentStatus::Failed),
            ("REQUIRES_PAYMENT_METHOD", true, PaymentStatus::Failed),
            ("requires_confirmation", false, PaymentStatus::Initiated),
            ("requires_action", false, PaymentStatus::RequiresAction),
            ("processing", false, PaymentStatus::Processing),
            ("requires_capture", false, PaymentStatus::Authorized),
            ("canceled", false, PaymentStatus::Canceled),
            ("succeeded", false, PaymentStatus::Paid),
            ("succeeded", true, PaymentStatus::Paid),
            ("unknown_future_status", false, PaymentStatus::Initiated),
        ];
        for (raw, has_err, expected) in cases {
            assert_eq!(map_payment_intent_status(raw, has_err), expected);
        }
    }

    #[test]
    fn tc_9_2_2_checkout_session_statuses() {
        let cases = [
            (Some("open"), Some("unpaid"), PaymentStatus::Initiated),
            (Some("complete"), Some("paid"), PaymentStatus::Paid),
            (
                Some("complete"),
                Some("no_payment_required"),
                PaymentStatus::Paid,
            ),
            (Some("complete"), Some("unpaid"), PaymentStatus::Processing),
            (Some("expired"), Some("unpaid"), PaymentStatus::Canceled),
            (None, None, PaymentStatus::Initiated),
        ];
        for (status, payment_status, expected) in cases {
            assert_eq!(map_checkout_status(status, payment_status), expected);
        }
    }

    #[test]
    fn tc_9_2_2_all_five_refund_statuses() {
        let cases = [
            ("pending", RefundStatus::Pending),
            ("requires_action", RefundStatus::RequiresAction),
            ("succeeded", RefundStatus::Succeeded),
            ("failed", RefundStatus::Failed),
            ("canceled", RefundStatus::Canceled),
            ("  SUCCEEDED ", RefundStatus::Succeeded),
            ("unknown", RefundStatus::Pending),
        ];
        for (raw, expected) in cases {
            assert_eq!(map_refund_status(raw), expected);
        }
    }

    #[test]
    fn tc_9_2_2_next_action_mapping_and_secret_redaction() {
        let embedded = map_next_action(Some("pi_123_secret_supersecret".into()), None);
        assert_eq!(
            embedded,
            Some(NextAction::EmbeddedClientSecret {
                client_secret: "pi_123_secret_supersecret".into(),
            })
        );
        let debug_repr = format!("{embedded:?}");
        assert!(debug_repr.contains("[REDACTED]"));
        assert!(!debug_repr.contains("pi_123_secret_supersecret"));

        let redirect = map_next_action(
            Some("pi_123_secret_ignored".into()),
            Some("https://hooks.stripe.com/3d_secure_2/hosted".into()),
        );
        assert_eq!(
            redirect,
            Some(NextAction::Redirect {
                url: "https://hooks.stripe.com/3d_secure_2/hosted".into(),
            })
        );
        assert_eq!(map_next_action(Some("   ".into()), None), None);
        assert_eq!(map_next_action(None, None), None);
    }

    #[test]
    fn tc_9_2_2_http_error_payload_auth_and_card() {
        let auth = map_stripe_error_payload(401, &json!({"error": {"message": "Invalid API Key"}}));
        assert_eq!(auth.kind(), ErrorKind::Authentication);

        let declined = map_stripe_error_payload(
            402,
            &json!({
                "error": {
                    "code": "card_declined",
                    "decline_code": "insufficient_funds",
                    "advice_code": "try_again_later",
                    "message": "Your card has insufficient funds."
                }
            }),
        );
        assert!(matches!(
            declined,
            PaymentError::CardDeclined {
                decline_code: Some(ref dc),
                advice_code: Some(ref ac),
                ..
            } if dc == "insufficient_funds" && ac == "try_again_later"
        ));

        let three_ds = map_stripe_error_payload(
            402,
            &json!({
                "error": {
                    "code": "payment_intent_authentication_failure",
                    "message": "3DS authentication failed"
                }
            }),
        );
        assert!(matches!(
            three_ds,
            PaymentError::ThreeDsFailed {
                code: Some(ref c),
                ..
            } if c == "payment_intent_authentication_failure"
        ));
    }

    #[test]
    fn tc_9_2_2_http_error_payload_transport_and_generic() {
        let rate_limited = map_stripe_error_payload(429, &json!({"retry_after_secs": 15}));
        assert!(matches!(
            rate_limited,
            PaymentError::RateLimited {
                retry_after_secs: Some(15),
                ..
            }
        ));

        let unavailable = map_stripe_error_payload(503, &json!({"error": {"message": "down"}}));
        assert!(matches!(
            unavailable,
            PaymentError::ProviderUnavailable {
                http_status: Some(503),
                indeterminate: false,
                ..
            }
        ));

        let timeout = map_stripe_error_payload(408, &json!({"error": {"message": "timeout"}}));
        assert!(matches!(
            timeout,
            PaymentError::ProviderUnavailable {
                http_status: Some(408),
                indeterminate: true,
                ..
            }
        ));

        let generic = map_stripe_error_payload(
            400,
            &json!({"error": {"code": "parameter_invalid_empty", "message": "bad param"}}),
        );
        assert!(matches!(
            generic,
            PaymentError::ProviderError {
                http_status: Some(400),
                vendor_code: Some(ref vc),
                ..
            } if vc == "parameter_invalid_empty"
        ));
    }

    #[test]
    fn tc_9_2_2_stripe_error_enum_mapping() {
        let cases = [
            (
                sample_api_error(401, "invalid_key"),
                ErrorKind::Authentication,
            ),
            (sample_api_error(429, "rate_limit"), ErrorKind::RateLimited),
            (
                StripeError::InvalidRequest("bad amount".into()),
                ErrorKind::Validation,
            ),
            (
                StripeError::Config("missing secret".into()),
                ErrorKind::Configuration,
            ),
            (
                StripeError::Webhook(StripeWebhookError::BadSignature),
                ErrorKind::WebhookVerificationFailed,
            ),
            (
                StripeError::Redirect(RedirectError::MissingHost {
                    url: "https://".into(),
                }),
                ErrorKind::Validation,
            ),
            (
                StripeError::ConnectionError("reset by peer".into()),
                ErrorKind::ProviderUnavailable,
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(map_stripe_error(&err).kind(), expected);
        }
    }

    #[test]
    fn tc_9_2_2_recursive_secret_scrubbing() {
        let payload = json!({
            "id": "evt_123",
            "client_secret": "pi_123_secret_abc",
            "nested": {
                "Access_Token": "tok_live_999",
                "userPassword": "hunter2",
                "stripe_api_key": "sk_live_secret",
                "webhook_signature": "v1=deadbeef",
                "safe_field": "visible"
            },
            "items": [{"token_id": "tok_1", "amount": 500}]
        });
        let scrubbed = scrub_webhook_payload(&payload);
        assert_eq!(scrubbed["id"], "evt_123");
        assert_eq!(scrubbed["client_secret"], "[REDACTED]");
        assert_eq!(scrubbed["nested"]["Access_Token"], "[REDACTED]");
        assert_eq!(scrubbed["nested"]["userPassword"], "[REDACTED]");
        assert_eq!(scrubbed["nested"]["stripe_api_key"], "[REDACTED]");
        assert_eq!(scrubbed["nested"]["webhook_signature"], "[REDACTED]");
        assert_eq!(scrubbed["nested"]["safe_field"], "visible");
        assert_eq!(scrubbed["items"][0]["token_id"], "[REDACTED]");
        assert_eq!(scrubbed["items"][0]["amount"], 500);
    }

    #[test]
    fn tc_9_2_2_payment_intent_and_checkout_json_mappers() {
        let pi = json!({
            "id": "pi_failed_1",
            "amount": 10000,
            "currency": "usd",
            "status": "requires_payment_method",
            "last_payment_error": {"code": "card_declined"},
            "client_secret": "pi_failed_1_secret_xyz",
            "amount_received": 0,
            "livemode": true
        });
        let pi_res = map_payment_intent_json(&pi).unwrap();
        assert_eq!(pi_res.status, PaymentStatus::Failed);
        assert_eq!(pi_res.money.amount(), 10000);
        assert!(pi_res.livemode);

        let cs = json!({
            "id": "cs_live_1",
            "payment_intent": "pi_from_cs_1",
            "client_reference_id": "ORD-99",
            "amount_total": 7500,
            "currency": "eur",
            "status": "complete",
            "payment_status": "paid",
            "url": "https://checkout.stripe.com/c/pay/cs_live_1",
            "livemode": false
        });
        let cs_res = map_checkout_session_json(&cs).unwrap();
        assert_eq!(cs_res.status, PaymentStatus::Paid);
        assert_eq!(cs_res.reference.checkout_session_id(), Some("cs_live_1"));
        assert_eq!(cs_res.reference.intent_id(), Some("pi_from_cs_1"));
        assert_eq!(cs_res.reference.order_id(), Some("ORD-99"));
    }

    #[test]
    fn tc_9_2_2_refund_and_thin_webhook_normalization() {
        let payment_ref = ProviderReference::builder(stripe_provider_id())
            .payment_id("pi_from_cs_1")
            .build()
            .unwrap();
        let refund_res = map_refund_json(
            &json!({
                "id": "re_55",
                "amount": 2500,
                "currency": "eur",
                "status": "requires_action",
                "pending_reason": "processing"
            }),
            &payment_ref,
        )
        .unwrap();
        assert_eq!(refund_res.status, RefundStatus::RequiresAction);
        assert_eq!(refund_res.reason_detail.as_deref(), Some("processing"));

        let meta = WebhookEventMeta {
            event_id: "evt_thin_1",
            event_type: "v2.core.event_destination.ping",
            livemode: false,
            is_thin_event: true,
        };
        let thin = map_webhook_json_to_normalized(
            &meta,
            &json!({
                "related_object": {"id": "pi_thin_99"},
                "secret_token": "whsec_leak"
            }),
        )
        .unwrap();
        assert!(thin.requires_hydration);
        assert_eq!(thin.status, None);
        assert_eq!(thin.money, None);
        assert_eq!(thin.reference.unwrap().payment_id(), Some("pi_thin_99"));
        assert_eq!(thin.raw_redacted["secret_token"], "[REDACTED]");
    }

    #[test]
    fn tc_9_2_2_currency_divisibility_guardrails() {
        assert_currency_guardrail(&["ISK", "UGX"], 1500, 1550);
        assert_currency_guardrail(&["BHD", "JOD", "KWD", "OMR", "TND"], 1250, 1255);

        let usd = Money::new(1999, "USD").unwrap();
        assert_eq!(
            validate_stripe_money_for_adapter(&usd).unwrap(),
            (1999, "usd".into())
        );

        assert!(
            Money::new(0, "USD")
                .and_then(|m| validate_stripe_money_for_adapter(&m))
                .is_err()
        );
        let overflow = Money::new((i64::MAX as u64) + 1, "USD").unwrap();
        assert_eq!(
            validate_stripe_money_for_adapter(&overflow)
                .unwrap_err()
                .kind(),
            ErrorKind::Validation
        );
        assert!(validate_stripe_money(0, "USD").is_err());
        assert!(validate_stripe_money(100, "U1D").is_err());
    }

    #[test]
    fn scrub_webhook_payload_redacts_expanded_sensitive_keys() {
        let scrubbed = scrub_webhook_payload(&json!({
            "nested": {
                "authorization": "Bearer sk_live_hidden",
                "apikey": "key_123",
                "bearer_header": "tok_456",
                "credential_id": "cred_789",
                "private_key": "-----BEGIN KEY-----",
                "passphrase": "hunter2",
                "safe_field": "visible"
            }
        }));
        let nested = &scrubbed["nested"];
        for key in [
            "authorization",
            "apikey",
            "bearer_header",
            "credential_id",
            "private_key",
            "passphrase",
        ] {
            assert_eq!(nested[key], "[REDACTED]", "expected {key} to be redacted");
        }
        assert_eq!(nested["safe_field"], "visible");
    }

    #[test]
    fn async_payment_failed_checkout_webhook_maps_to_failed() {
        let meta = WebhookEventMeta {
            event_id: "evt_async_fail_1",
            event_type: "checkout.session.async_payment_failed",
            livemode: false,
            is_thin_event: false,
        };
        let payload = json!({
            "data": {
                "object": {
                    "object": "checkout.session",
                    "id": "cs_test_async_fail",
                    "status": "complete",
                    "payment_status": "unpaid",
                    "amount_total": 4200,
                    "currency": "usd"
                }
            }
        });
        let event = map_webhook_json_to_normalized(&meta, &payload).unwrap();
        assert_eq!(event.status, Some(PaymentStatus::Failed));
    }

    #[test]
    fn three_ds_failure_codes_cover_shared_core_catalog() {
        for code in [
            "three_d_secure_authentication_failed",
            "off_session_authentication_required",
        ] {
            let err = map_stripe_error_payload(402, &json!({"error": {"code": code}}));
            assert_eq!(err.kind(), ErrorKind::ThreeDsFailed);
        }
    }

    #[test]
    fn retry_after_header_populates_rate_limited_error() {
        let err = map_stripe_error_payload_with_retry_after(
            429,
            &json!({"error": {"message": "Too many requests"}}),
            Some(42),
        );
        assert!(matches!(
            err,
            PaymentError::RateLimited {
                retry_after_secs: Some(42),
                ..
            }
        ));
    }
}
