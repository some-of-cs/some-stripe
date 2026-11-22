//! `StripeProvider` implementing [`some_payment_core::PaymentProvider`] (US-9.2.0 Unit 3).
//!
//! Provides a feature-gated adapter (`some-payment-adapter`) bridging Stripe's REST API
//! and multi-`v1` webhook verifier to the canonical `some-payment-core` SPI.

use std::collections::BTreeMap;
use std::fmt;

use async_trait::async_trait;
use reqwest::Method;
use serde_json::Value;
use some_payment_core::{
    CaptureMode, CaptureRequest, CheckoutSessionRequest, CreatePaymentRequest,
    NormalizedWebhookEvent, PaymentCapabilities, PaymentError, PaymentProvider, PaymentResult,
    ProviderId, ProviderReference, RawWebhookRequest, RefundRequest, RefundResult, VoidRequest,
};
use some_stripe_core::{SecretKey, validate_and_encode_id};
use some_stripe_webhooks::multi_v1_verifier::{
    DEFAULT_TOLERANCE_SECS, WebhookVerificationOptions, verify_stripe_raw_webhook,
};

use crate::adapter::mappers::{
    WebhookEventMeta, map_checkout_session_json, map_payment_intent_json, map_refund_json,
    map_stripe_capabilities, map_stripe_error_payload, map_webhook_json_to_normalized,
    stripe_provider_id, validate_stripe_money_for_adapter,
};

/// Default base URL for Stripe's REST API.
pub const DEFAULT_STRIPE_API_BASE: &str = "https://api.stripe.com";

/// Default `Stripe-Version` API header sent by [`StripeProvider`].
pub const DEFAULT_STRIPE_API_VERSION: &str = "2026-11-18.endive";

/// Configuration builder for [`StripeProvider`].
///
/// Implements a custom [`fmt::Debug`] that always redacts `api_key` and
/// `webhook_secrets` as `"[REDACTED]"`.
///
/// # Examples
///
/// ```
/// use some_stripe::adapter::provider::{
///     DEFAULT_STRIPE_API_BASE, DEFAULT_STRIPE_API_VERSION, StripeProviderConfig,
/// };
///
/// let config = StripeProviderConfig::new("sk_test_example")
///     .with_webhook_secret("whsec_example")
///     .with_api_base(DEFAULT_STRIPE_API_BASE)
///     .with_stripe_version(DEFAULT_STRIPE_API_VERSION);
///
/// let debug = format!("{config:?}");
/// assert!(debug.contains("[REDACTED]"));
/// assert!(!debug.contains("sk_test_example"));
/// assert!(!debug.contains("whsec_example"));
/// ```
pub struct StripeProviderConfig {
    api_key: SecretKey,
    webhook_secrets: Vec<SecretKey>,
    api_base: String,
    stripe_version: String,
    http_client: Option<reqwest::Client>,
    webhook_tolerance_secs: i64,
}

impl fmt::Debug for StripeProviderConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StripeProviderConfig")
            .field("api_key", &"[REDACTED]")
            .field("webhook_secrets", &"[REDACTED]")
            .field("api_base", &self.api_base)
            .field("stripe_version", &self.stripe_version)
            .field("has_http_client", &self.http_client.is_some())
            .field("webhook_tolerance_secs", &self.webhook_tolerance_secs)
            .finish()
    }
}

impl StripeProviderConfig {
    /// Creates a new [`StripeProviderConfig`] with the given Stripe secret API key
    /// and default API base, version, and webhook timestamp tolerance.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: SecretKey::new(api_key),
            webhook_secrets: Vec::new(),
            api_base: DEFAULT_STRIPE_API_BASE.to_owned(),
            stripe_version: DEFAULT_STRIPE_API_VERSION.to_owned(),
            http_client: None,
            webhook_tolerance_secs: DEFAULT_TOLERANCE_SECS,
        }
    }

    /// Appends a webhook signing secret (`whsec_...`) for multi-secret rotation.
    pub fn with_webhook_secret(mut self, secret: impl Into<String>) -> Self {
        self.webhook_secrets.push(SecretKey::new(secret));
        self
    }

    /// Replaces the configured webhook signing secrets with the provided list.
    pub fn with_webhook_secrets(mut self, secrets: Vec<String>) -> Self {
        self.webhook_secrets = secrets.into_iter().map(SecretKey::new).collect();
        self
    }

    /// Overrides the Stripe API base URL (must be `http://` or `https://`).
    pub fn with_api_base(mut self, base: impl Into<String>) -> Self {
        self.api_base = base.into();
        self
    }

    /// Overrides the `Stripe-Version` HTTP header value.
    pub fn with_stripe_version(mut self, ver: impl Into<String>) -> Self {
        self.stripe_version = ver.into();
        self
    }

    /// Injects a custom per-instance [`reqwest::Client`].
    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Sets the webhook timestamp replay tolerance in seconds.
    pub fn with_webhook_tolerance_secs(mut self, secs: i64) -> Self {
        self.webhook_tolerance_secs = secs;
        self
    }
}

struct StripeHttpRequest<'a> {
    method: Method,
    path: &'a str,
    form: Option<&'a [(String, String)]>,
    idempotency_key: Option<&'a str>,
}

/// Stripe gateway adapter implementing [`some_payment_core::PaymentProvider`].
///
/// Implements a custom [`fmt::Debug`] that always redacts `api_key` and
/// `webhook_secrets` as `"[REDACTED]"`.
///
/// # Examples
///
/// ```
/// use some_payment_core::PaymentProvider;
/// use some_stripe::adapter::provider::{StripeProvider, StripeProviderConfig};
///
/// let config = StripeProviderConfig::new("sk_test_example")
///     .with_webhook_secret("whsec_example");
/// let provider = StripeProvider::new(config).expect("valid Stripe provider config");
///
/// assert_eq!(provider.id().as_str(), "stripe");
/// let debug = format!("{provider:?}");
/// assert!(debug.contains("[REDACTED]"));
/// assert!(!debug.contains("sk_test_example"));
/// assert!(!debug.contains("whsec_example"));
/// ```
pub struct StripeProvider {
    provider_id: ProviderId,
    http_client: reqwest::Client,
    api_key: SecretKey,
    webhook_secrets: Vec<SecretKey>,
    api_base: String,
    stripe_version: String,
    webhook_tolerance_secs: i64,
}

impl fmt::Debug for StripeProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StripeProvider")
            .field("provider_id", &self.provider_id)
            .field("api_key", &"[REDACTED]")
            .field("webhook_secrets", &"[REDACTED]")
            .field("api_base", &self.api_base)
            .field("stripe_version", &self.stripe_version)
            .field("webhook_tolerance_secs", &self.webhook_tolerance_secs)
            .finish()
    }
}

impl StripeProvider {
    /// Validates `config` and constructs a new [`StripeProvider`].
    ///
    /// # Errors
    ///
    /// Returns [`PaymentError::ConfigurationError`] if `api_key` is empty,
    /// `api_base` is not a valid `http://` or `https://` URL, `stripe_version`
    /// is empty, `webhook_tolerance_secs` is negative, or the default HTTP
    /// client fails to initialize.
    pub fn new(config: StripeProviderConfig) -> Result<Self, PaymentError> {
        validate_non_empty(config.api_key.expose_secret(), "api_key")?;
        let api_base = validate_api_base(&config.api_base)?;
        let stripe_version = validate_non_empty(&config.stripe_version, "stripe_version")?;
        if config.webhook_tolerance_secs < 0 {
            return Err(PaymentError::configuration(
                "Stripe webhook_tolerance_secs must be non-negative",
            ));
        }
        let http_client = match config.http_client {
            Some(client) => client,
            None => build_default_http_client()?,
        };
        Ok(Self {
            provider_id: stripe_provider_id(),
            http_client,
            api_key: config.api_key,
            webhook_secrets: config.webhook_secrets,
            api_base,
            stripe_version,
            webhook_tolerance_secs: config.webhook_tolerance_secs,
        })
    }

    async fn get_json(&self, path: &str) -> Result<Value, PaymentError> {
        self.send_request(StripeHttpRequest {
            method: Method::GET,
            path,
            form: None,
            idempotency_key: None,
        })
        .await
    }

    async fn post_form(
        &self,
        path: &str,
        form: Option<&[(String, String)]>,
        idempotency_key: Option<&str>,
    ) -> Result<Value, PaymentError> {
        self.send_request(StripeHttpRequest {
            method: Method::POST,
            path,
            form,
            idempotency_key,
        })
        .await
    }

    async fn send_request(&self, req: StripeHttpRequest<'_>) -> Result<Value, PaymentError> {
        let url = format!("{}{}", self.api_base, req.path);
        let mut builder = self
            .http_client
            .request(req.method, &url)
            .bearer_auth(self.api_key.expose_secret())
            .header("Stripe-Version", &self.stripe_version);
        if let Some(key) = req.idempotency_key {
            builder = builder.header("Idempotency-Key", key);
        }
        if let Some(params) = req.form {
            builder = builder.form(params);
        }
        let response = builder
            .send()
            .await
            .map_err(|err| self.transport_error(err))?;
        self.decode_response(response).await
    }

    async fn decode_response(&self, response: reqwest::Response) -> Result<Value, PaymentError> {
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .map_err(|err| self.transport_error(err))?;
        if !(200..300).contains(&status) {
            let body_json = parse_error_json(&body);
            return Err(map_stripe_error_payload(status, &body_json));
        }
        serde_json::from_str(&body).map_err(|err| PaymentError::ProviderError {
            provider: self.provider_id.clone(),
            http_status: Some(status),
            vendor_code: None,
            message: format!("failed to parse Stripe JSON response: {err}"),
        })
    }

    fn transport_error(&self, err: reqwest::Error) -> PaymentError {
        PaymentError::ProviderUnavailable {
            provider: self.provider_id.clone(),
            http_status: err.status().map(|s| s.as_u16()),
            indeterminate: true,
            message: format!("Stripe HTTP transport error: {err}"),
        }
    }
}

#[async_trait]
impl PaymentProvider for StripeProvider {
    fn id(&self) -> &ProviderId {
        &self.provider_id
    }

    fn capabilities(&self) -> PaymentCapabilities {
        map_stripe_capabilities()
    }

    async fn create_payment(
        &self,
        req: &CreatePaymentRequest,
    ) -> Result<PaymentResult, PaymentError> {
        req.validate()?;
        validate_stripe_money_for_adapter(&req.money)?;
        let params = build_create_payment_params(req);
        let json = self
            .post_form(
                "/v1/payment_intents",
                Some(&params),
                req.idempotency_key.as_deref(),
            )
            .await?;
        map_payment_intent_json(&json)
    }

    async fn get_payment(
        &self,
        reference: &ProviderReference,
    ) -> Result<PaymentResult, PaymentError> {
        if let Some(raw_id) = resolve_payment_intent_id(reference) {
            let path = encoded_resource_path("/v1/payment_intents", raw_id, "")?;
            let json = self.get_json(&path).await?;
            return map_payment_intent_json(&json);
        }
        if let Some(raw_cs) = reference.checkout_session_id() {
            let path = encoded_resource_path("/v1/checkout/sessions", raw_cs, "")?;
            let json = self.get_json(&path).await?;
            return map_checkout_session_json(&json);
        }
        Err(PaymentError::validation(
            "Stripe get_payment requires payment_id, intent_id, or checkout_session_id",
        ))
    }

    async fn create_checkout(
        &self,
        req: &CheckoutSessionRequest,
    ) -> Result<PaymentResult, PaymentError> {
        req.validate()?;
        validate_stripe_money_for_adapter(&req.money)?;
        let params = build_checkout_params(req);
        let json = self
            .post_form(
                "/v1/checkout/sessions",
                Some(&params),
                req.idempotency_key.as_deref(),
            )
            .await?;
        map_checkout_session_json(&json)
    }

    async fn capture(&self, req: &CaptureRequest) -> Result<PaymentResult, PaymentError> {
        req.validate()?;
        let raw_id = require_payment_intent_id(&req.reference, "capture")?;
        let path = encoded_resource_path("/v1/payment_intents", raw_id, "/capture")?;
        let mut params = Vec::new();
        if let Some(money) = &req.amount {
            validate_stripe_money_for_adapter(money)?;
            params.push(("amount_to_capture".to_owned(), money.amount().to_string()));
        }
        let form = (!params.is_empty()).then_some(params.as_slice());
        let json = self
            .post_form(&path, form, req.idempotency_key.as_deref())
            .await?;
        map_payment_intent_json(&json)
    }

    async fn void_payment(&self, req: &VoidRequest) -> Result<PaymentResult, PaymentError> {
        req.validate()?;
        if let Some(raw_pi) = resolve_payment_intent_id(&req.reference) {
            let path = encoded_resource_path("/v1/payment_intents", raw_pi, "/cancel")?;
            let mut params = Vec::new();
            push_opt(&mut params, "cancellation_reason", req.reason.as_deref());
            let form = (!params.is_empty()).then_some(params.as_slice());
            let json = self
                .post_form(&path, form, req.idempotency_key.as_deref())
                .await?;
            return map_payment_intent_json(&json);
        }
        if let Some(raw_cs) = req.reference.checkout_session_id() {
            let path = encoded_resource_path("/v1/checkout/sessions", raw_cs, "/expire")?;
            let json = self
                .post_form(&path, None, req.idempotency_key.as_deref())
                .await?;
            return map_checkout_session_json(&json);
        }
        Err(PaymentError::validation(
            "Stripe void_payment requires payment_id, intent_id, or checkout_session_id",
        ))
    }

    async fn refund(&self, req: &RefundRequest) -> Result<RefundResult, PaymentError> {
        req.validate()?;
        let raw_id = require_payment_intent_id(&req.reference, "refund")?;
        let _ = encode_stripe_id(raw_id)?;
        let mut params = vec![("payment_intent".to_owned(), raw_id.trim().to_owned())];
        if let Some(money) = &req.amount {
            validate_stripe_money_for_adapter(money)?;
            params.push(("amount".to_owned(), money.amount().to_string()));
        }
        push_opt(&mut params, "reason", req.reason.as_deref());
        let json = self
            .post_form("/v1/refunds", Some(&params), req.idempotency_key.as_deref())
            .await?;
        map_refund_json(&json, &req.reference)
    }

    async fn parse_and_verify_webhook(
        &self,
        raw: &RawWebhookRequest,
    ) -> Result<NormalizedWebhookEvent, PaymentError> {
        if self.webhook_secrets.is_empty() {
            return Err(PaymentError::configuration(
                "at least one Stripe webhook secret must be configured",
            ));
        }
        let sig = raw.header("stripe-signature").ok_or_else(|| {
            PaymentError::WebhookVerificationFailed {
                provider: self.provider_id.clone(),
                reason: "missing Stripe-Signature header".to_owned(),
            }
        })?;
        let secret_slices: Vec<&str> = self
            .webhook_secrets
            .iter()
            .map(SecretKey::expose_secret)
            .collect();
        let options = WebhookVerificationOptions {
            now_unix_secs: chrono::Utc::now().timestamp(),
            tolerance_secs: self.webhook_tolerance_secs,
        };
        let verified = verify_stripe_raw_webhook(&raw.raw_body, sig, &secret_slices, options)
            .map_err(|err| PaymentError::WebhookVerificationFailed {
                provider: self.provider_id.clone(),
                reason: err.to_string(),
            })?;
        let meta = WebhookEventMeta {
            event_id: &verified.event_id,
            event_type: &verified.event_type,
            livemode: verified.livemode,
            is_thin_event: verified.is_thin_event,
        };
        map_webhook_json_to_normalized(&meta, &verified.payload)
    }
}

fn validate_non_empty(value: &str, field: &str) -> Result<String, PaymentError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(PaymentError::configuration(format!(
            "Stripe {field} must not be empty"
        )));
    }
    Ok(trimmed.to_owned())
}

fn validate_api_base(raw: &str) -> Result<String, PaymentError> {
    let trimmed = raw.trim().trim_end_matches('/');
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"));
    let is_valid = rest.is_some_and(|r| {
        !r.is_empty() && r.chars().all(|c| c.is_ascii() && !c.is_ascii_whitespace())
    });
    if !is_valid {
        return Err(PaymentError::configuration(
            "Stripe api_base must be a valid http:// or https:// URL",
        ));
    }
    Ok(trimmed.to_owned())
}

fn build_default_http_client() -> Result<reqwest::Client, PaymentError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|err| {
            PaymentError::configuration(format!("failed to build Stripe HTTP client: {err}"))
        })
}

fn parse_error_json(body: &str) -> Value {
    serde_json::from_str(body).unwrap_or_else(|_| serde_json::json!({}))
}

fn encode_stripe_id(id: &str) -> Result<String, PaymentError> {
    validate_and_encode_id(id).map_err(|err| PaymentError::validation(err.to_string()))
}

fn encoded_resource_path(
    prefix: &str,
    raw_id: &str,
    suffix: &str,
) -> Result<String, PaymentError> {
    let encoded = encode_stripe_id(raw_id)?;
    Ok(format!("{prefix}/{encoded}{suffix}"))
}

fn resolve_payment_intent_id(reference: &ProviderReference) -> Option<&str> {
    reference.payment_id().or_else(|| reference.intent_id())
}

fn require_payment_intent_id<'a>(
    reference: &'a ProviderReference,
    operation: &str,
) -> Result<&'a str, PaymentError> {
    resolve_payment_intent_id(reference).ok_or_else(|| {
        PaymentError::validation(format!(
            "Stripe {operation} requires payment_id or intent_id"
        ))
    })
}

fn capture_method_str(mode: CaptureMode) -> &'static str {
    match mode {
        CaptureMode::Automatic => "automatic",
        CaptureMode::Manual => "manual",
    }
}

fn push_opt(params: &mut Vec<(String, String)>, key: &str, value: Option<&str>) {
    if let Some(val) = value {
        params.push((key.to_owned(), val.to_owned()));
    }
}

fn append_metadata_params(
    params: &mut Vec<(String, String)>,
    metadata: &BTreeMap<String, String>,
    order_id: &str,
) {
    params.push(("metadata[order_id]".to_owned(), order_id.to_owned()));
    for (key, value) in metadata {
        if key != "order_id" {
            params.push((format!("metadata[{key}]"), value.clone()));
        }
    }
}

fn build_create_payment_params(req: &CreatePaymentRequest) -> Vec<(String, String)> {
    let mut params = vec![
        ("amount".to_owned(), req.money.amount().to_string()),
        ("currency".to_owned(), req.money.currency().to_lowercase()),
        (
            "capture_method".to_owned(),
            capture_method_str(req.capture_mode).to_owned(),
        ),
    ];
    push_opt(&mut params, "description", req.description.as_deref());
    push_opt(&mut params, "receipt_email", req.customer_email.as_deref());
    push_opt(&mut params, "payment_method", req.payment_token.as_deref());
    push_opt(&mut params, "return_url", req.callback_url.as_deref());
    append_metadata_params(&mut params, &req.metadata, &req.order_id);
    params
}

fn append_checkout_line_item(params: &mut Vec<(String, String)>, req: &CheckoutSessionRequest) {
    let product_name = req.description.as_deref().unwrap_or(&req.order_id);
    params.push((
        "line_items[0][price_data][currency]".to_owned(),
        req.money.currency().to_lowercase(),
    ));
    params.push((
        "line_items[0][price_data][unit_amount]".to_owned(),
        req.money.amount().to_string(),
    ));
    params.push((
        "line_items[0][price_data][product_data][name]".to_owned(),
        product_name.to_owned(),
    ));
    params.push(("line_items[0][quantity]".to_owned(), "1".to_owned()));
}

fn build_checkout_params(req: &CheckoutSessionRequest) -> Vec<(String, String)> {
    let mut params = vec![
        ("mode".to_owned(), "payment".to_owned()),
        ("success_url".to_owned(), req.success_url.clone()),
        ("client_reference_id".to_owned(), req.order_id.clone()),
    ];
    append_checkout_line_item(&mut params, req);
    push_opt(&mut params, "cancel_url", req.cancel_url.as_deref());
    push_opt(&mut params, "customer_email", req.customer_email.as_deref());
    if req.capture_mode == CaptureMode::Manual {
        params.push((
            "payment_intent_data[capture_method]".to_owned(),
            "manual".to_owned(),
        ));
    }
    append_metadata_params(&mut params, &req.metadata, &req.order_id);
    params
}

#[cfg(test)]
mod tests {
    use super::*;
    use some_payment_core::{ErrorKind, Money};

    fn assert_secrets_redacted(debug: &str) {
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("sk_test_canary_api_key_999"));
        assert!(!debug.contains("whsec_canary_primary_111"));
        assert!(!debug.contains("whsec_canary_secondary_222"));
    }

    fn has_param(params: &[(String, String)], key: &str, value: &str) -> bool {
        params.iter().any(|(k, v)| k == key && v == value)
    }

    #[test]
    fn config_and_provider_debug_redact_all_secrets() {
        let config = StripeProviderConfig::new("sk_test_canary_api_key_999")
            .with_webhook_secrets(vec![
                "whsec_canary_primary_111".to_owned(),
                "whsec_canary_secondary_222".to_owned(),
            ])
            .with_api_base("https://api.stripe.com/")
            .with_stripe_version("2026-11-18.endive")
            .with_webhook_tolerance_secs(120);

        assert_secrets_redacted(&format!("{config:?}"));

        let provider = StripeProvider::new(config).expect("valid config");
        assert_secrets_redacted(&format!("{provider:?}"));
        assert_eq!(provider.id().as_str(), "stripe");
    }

    #[test]
    fn provider_new_rejects_invalid_configuration() {
        let invalid_configs = [
            StripeProviderConfig::new("   "),
            StripeProviderConfig::new("sk_test_123").with_api_base("ftp://api.stripe.com"),
            StripeProviderConfig::new("sk_test_123").with_stripe_version("  "),
            StripeProviderConfig::new("sk_test_123").with_webhook_tolerance_secs(-1),
        ];
        for cfg in invalid_configs {
            assert_eq!(StripeProvider::new(cfg).unwrap_err().kind(), ErrorKind::Configuration);
        }
    }

    #[test]
    fn create_payment_params_populate_expected_fields() {
        let money = Money::new(2_500, "USD").unwrap();
        let mut req = CreatePaymentRequest::new(money, "ORD-100")
            .with_capture_mode(CaptureMode::Manual);
        req.description = Some("Order 100".to_owned());
        req.metadata.insert("tenant".to_owned(), "acme".to_owned());

        let params = build_create_payment_params(&req);
        assert!(has_param(&params, "amount", "2500"));
        assert!(has_param(&params, "currency", "usd"));
        assert!(has_param(&params, "capture_method", "manual"));
        assert!(has_param(&params, "metadata[order_id]", "ORD-100"));
        assert!(has_param(&params, "metadata[tenant]", "acme"));
    }

    #[test]
    fn checkout_params_populate_expected_fields() {
        let money = Money::new(2_500, "USD").unwrap();
        let req = CheckoutSessionRequest {
            provider_id: None,
            money,
            order_id: "ORD-200".to_owned(),
            success_url: "https://example.com/ok".to_owned(),
            cancel_url: Some("https://example.com/cancel".to_owned()),
            description: Some("Pro Plan".to_owned()),
            customer_email: None,
            customer_phone: None,
            capture_mode: CaptureMode::Manual,
            idempotency_key: None,
            metadata: BTreeMap::new(),
        };

        let params = build_checkout_params(&req);
        assert!(has_param(&params, "mode", "payment"));
        assert!(has_param(&params, "payment_intent_data[capture_method]", "manual"));
        assert!(has_param(&params, "line_items[0][price_data][product_data][name]", "Pro Plan"));
    }

    #[test]
    fn encode_stripe_id_blocks_path_traversal() {
        assert_eq!(encode_stripe_id("pi_12345").unwrap(), "pi_12345");
        let err = encode_stripe_id("../customers/cus_1").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Validation);
    }
}
