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
    DEFAULT_TOLERANCE_SECS, verify_stripe_raw_webhook,
};

use crate::adapter::mappers::{
    map_checkout_session_json, map_payment_intent_json, map_refund_json, map_stripe_capabilities,
    map_stripe_error_payload, map_webhook_json_to_normalized, stripe_provider_id,
    validate_stripe_money_for_adapter,
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

    async fn send_request(
        &self,
        method: Method,
        path: &str,
        form: Option<&[(String, String)]>,
        idempotency_key: Option<&str>,
    ) -> Result<Value, PaymentError> {
        let url = format!("{}{}", self.api_base, path);
        let mut builder = self
            .http_client
            .request(method, &url)
            .bearer_auth(self.api_key.expose_secret())
            .header("Stripe-Version", &self.stripe_version);
        if let Some(key) = idempotency_key {
            builder = builder.header("Idempotency-Key", key);
        }
        if let Some(params) = form {
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
            return Err(map_stripe_error_payload(status, &body));
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
            .send_request(
                Method::POST,
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
            let encoded = encode_stripe_id(raw_id)?;
            let path = format!("/v1/payment_intents/{encoded}");
            let json = self.send_request(Method::GET, &path, None, None).await?;
            return map_payment_intent_json(&json);
        }
        if let Some(raw_cs) = reference.checkout_session_id() {
            let encoded = encode_stripe_id(raw_cs)?;
            let path = format!("/v1/checkout/sessions/{encoded}");
            let json = self.send_request(Method::GET, &path, None, None).await?;
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
            .send_request(
                Method::POST,
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
        let encoded = encode_stripe_id(raw_id)?;
        let mut params = Vec::new();
        if let Some(money) = &req.amount {
            validate_stripe_money_for_adapter(money)?;
            params.push(("amount_to_capture".to_owned(), money.amount().to_string()));
        }
        let form = (!params.is_empty()).then_some(params.as_slice());
        let path = format!("/v1/payment_intents/{encoded}/capture");
        let json = self
            .send_request(Method::POST, &path, form, req.idempotency_key.as_deref())
            .await?;
        map_payment_intent_json(&json)
    }

    async fn void_payment(&self, req: &VoidRequest) -> Result<PaymentResult, PaymentError> {
        req.validate()?;
        if let Some(raw_pi) = resolve_payment_intent_id(&req.reference) {
            let encoded = encode_stripe_id(raw_pi)?;
            let mut params = Vec::new();
            push_opt(&mut params, "cancellation_reason", req.reason.as_deref());
            let form = (!params.is_empty()).then_some(params.as_slice());
            let path = format!("/v1/payment_intents/{encoded}/cancel");
            let json = self
                .send_request(Method::POST, &path, form, req.idempotency_key.as_deref())
                .await?;
            return map_payment_intent_json(&json);
        }
        if let Some(raw_cs) = req.reference.checkout_session_id() {
            let encoded = encode_stripe_id(raw_cs)?;
            let path = format!("/v1/checkout/sessions/{encoded}/expire");
            let json = self
                .send_request(Method::POST, &path, None, req.idempotency_key.as_deref())
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
            .send_request(
                Method::POST,
                "/v1/refunds",
                Some(&params),
                req.idempotency_key.as_deref(),
            )
            .await?;
        map_refund_json(&json)
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
        let now = chrono::Utc::now().timestamp();
        let _ = verify_stripe_raw_webhook(
            &raw.raw_body,
            sig,
            &secret_slices,
            now,
            self.webhook_tolerance_secs,
        )
        .map_err(|err| PaymentError::WebhookVerificationFailed {
            provider: self.provider_id.clone(),
            reason: err.to_string(),
        })?;
        let payload: Value = serde_json::from_slice(&raw.raw_body).map_err(|err| {
            PaymentError::WebhookVerificationFailed {
                provider: self.provider_id.clone(),
                reason: format!("invalid Stripe webhook JSON payload: {err}"),
            }
        })?;
        map_webhook_json_to_normalized(&payload)
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

fn encode_stripe_id(id: &str) -> Result<String, PaymentError> {
    validate_and_encode_id(id).map_err(|err| PaymentError::validation(err.to_string()))
}

fn resolve_payment_intent_id(reference: &ProviderReference) -> Option<&str> {
    reference.payment_id().or_else(|| reference.intent_id())
}

fn require_payment_intent_id<'a>(
    reference: &'a ProviderReference,
    operation: &str,
) -> Result<&'a str, PaymentError> {
    resolve_payment_intent_id(reference).ok_or_else(|| {
        PaymentError::validation(format!("Stripe {operation} requires payment_id or intent_id"))
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

fn build_checkout_params(req: &CheckoutSessionRequest) -> Vec<(String, String)> {
    let product_name = req.description.as_deref().unwrap_or(&req.order_id);
    let mut params = vec![
        ("mode".to_owned(), "payment".to_owned()),
        ("success_url".to_owned(), req.success_url.clone()),
        ("client_reference_id".to_owned(), req.order_id.clone()),
        (
            "line_items[0][price_data][currency]".to_owned(),
            req.money.currency().to_lowercase(),
        ),
        (
            "line_items[0][price_data][unit_amount]".to_owned(),
            req.money.amount().to_string(),
        ),
        (
            "line_items[0][price_data][product_data][name]".to_owned(),
            product_name.to_owned(),
        ),
        ("line_items[0][quantity]".to_owned(), "1".to_owned()),
    ];
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

        let cfg_dbg = format!("{config:?}");
        assert!(cfg_dbg.contains("[REDACTED]"));
        assert!(!cfg_dbg.contains("sk_test_canary_api_key_999"));
        assert!(!cfg_dbg.contains("whsec_canary_primary_111"));
        assert!(!cfg_dbg.contains("whsec_canary_secondary_222"));

        let provider = StripeProvider::new(config).expect("valid config");
        let prov_dbg = format!("{provider:?}");
        assert!(prov_dbg.contains("[REDACTED]"));
        assert!(!prov_dbg.contains("sk_test_canary_api_key_999"));
        assert!(!prov_dbg.contains("whsec_canary_primary_111"));
        assert!(!prov_dbg.contains("whsec_canary_secondary_222"));
        assert_eq!(provider.id().as_str(), "stripe");
    }

    #[test]
    fn provider_new_rejects_invalid_configuration() {
        let empty_key = StripeProvider::new(StripeProviderConfig::new("   ")).unwrap_err();
        assert_eq!(empty_key.kind(), ErrorKind::Configuration);

        let bad_base = StripeProvider::new(
            StripeProviderConfig::new("sk_test_123").with_api_base("ftp://api.stripe.com"),
        )
        .unwrap_err();
        assert_eq!(bad_base.kind(), ErrorKind::Configuration);

        let empty_ver = StripeProvider::new(
            StripeProviderConfig::new("sk_test_123").with_stripe_version("  "),
        )
        .unwrap_err();
        assert_eq!(empty_ver.kind(), ErrorKind::Configuration);

        let neg_tol = StripeProvider::new(
            StripeProviderConfig::new("sk_test_123").with_webhook_tolerance_secs(-1),
        )
        .unwrap_err();
        assert_eq!(neg_tol.kind(), ErrorKind::Configuration);
    }

    #[test]
    fn param_builders_populate_expected_form_fields() {
        let money = Money::new(2_500, "USD").unwrap();
        let mut req = CreatePaymentRequest::new(money.clone(), "ORD-100")
            .with_capture_mode(CaptureMode::Manual);
        req.description = Some("Order 100".to_owned());
        req.metadata.insert("tenant".to_owned(), "acme".to_owned());

        let pi_params = build_create_payment_params(&req);
        assert!(pi_params.contains(&("amount".to_owned(), "2500".to_owned())));
        assert!(pi_params.contains(&("currency".to_owned(), "usd".to_owned())));
        assert!(pi_params.contains(&("capture_method".to_owned(), "manual".to_owned())));
        assert!(pi_params.contains(&("metadata[order_id]".to_owned(), "ORD-100".to_owned())));
        assert!(pi_params.contains(&("metadata[tenant]".to_owned(), "acme".to_owned())));

        let checkout_req = CheckoutSessionRequest {
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
        let cs_params = build_checkout_params(&checkout_req);
        assert!(cs_params.contains(&("mode".to_owned(), "payment".to_owned())));
        assert!(cs_params.contains(&(
            "payment_intent_data[capture_method]".to_owned(),
            "manual".to_owned()
        )));
        assert!(cs_params.contains(&(
            "line_items[0][price_data][product_data][name]".to_owned(),
            "Pro Plan".to_owned()
        )));
    }

    #[test]
    fn encode_stripe_id_blocks_path_traversal() {
        assert_eq!(encode_stripe_id("pi_12345").unwrap(), "pi_12345");
        let err = encode_stripe_id("../customers/cus_1").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Validation);
    }
}
