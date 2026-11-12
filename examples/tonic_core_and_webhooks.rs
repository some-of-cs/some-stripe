//! # Tonic gRPC Example 1: Microkernel Core, Dual-Protocol Serialization & Cryptographic Webhooks
//!
//! Demonstrates how to integrate **Module 1 (Microkernel Core Runtime)** and **Module 2
//! (Cryptographic Webhooks)** of `some-stripe` inside a **Tonic (`tonic`)** gRPC server.
//!
//! ## Services & gRPC Methods Covered
//! - `get_health`: Reports `StripeClient` configuration health with `[REDACTED]` secret formatting
//!   (`ZeroizeOnDrop`)
//! - `preview_dual_protocol`: Demonstrates 3-state `Nullable<T>` (`Value`, `Null`, `Omit`)
//!   across `serialize_v1_form` and `serialize_v2_json` plus `RequestOptions`
//! - `verify_v1_webhook`: Extracts `stripe-signature` from gRPC `MetadataMap` (or request field)
//!   and verifies HMAC-SHA256 in constant time (`StripeWebhookWrapper::from_raw` &
//!   `construct_event_with_config`) with 300s replay drift tolerance and 1 MB payload ceiling
//! - `verify_v2_thin_webhook`: Verifies and parses lightweight API v2 `V2ThinEvent` notifications
//!
//! ## Run the Example
//! ```bash
//! cargo run --example tonic_core_and_webhooks
//! ```

use prost::Message;
use serde::Serialize;
use some_stripe::core::patterns::{Nullable, serialize_v1_form, serialize_v2_json};
use some_stripe::core::workflow::CustomErrorWorkflowBuilder;
use some_stripe::{
    EventObject, MAX_WEBHOOK_PAYLOAD_BYTES, RequestOptions, SecretKey, StripeClient, StripeConfig,
    StripeError, StripeWebhookError, StripeWebhookWrapper, ToleranceConfig, is_thin_event,
};
use std::time::Duration;
use tonic::{Request, Response, Status};

/// Protobuf request message for `GetHealth`.
#[derive(Clone, PartialEq, Message)]
pub struct HealthCheckRequest {}

/// Protobuf response message for `GetHealth`.
#[derive(Clone, PartialEq, Message)]
pub struct HealthCheckResponse {
    #[prost(string, tag = "1")]
    pub status: String,
    #[prost(string, tag = "2")]
    pub service: String,
    #[prost(string, tag = "3")]
    pub api_base: String,
    #[prost(string, tag = "4")]
    pub secret_debug_redacted: String,
}

/// Protobuf request message for `PreviewDualProtocol`.
#[derive(Clone, PartialEq, Message)]
pub struct DualProtocolPreviewRequest {
    #[prost(string, optional, tag = "1")]
    pub email: Option<String>,
    #[prost(bool, tag = "2")]
    pub clear_description: bool,
    #[prost(string, optional, tag = "3")]
    pub stripe_account: Option<String>,
    #[prost(string, optional, tag = "4")]
    pub idempotency_key: Option<String>,
}

/// Protobuf response message for `PreviewDualProtocol`.
#[derive(Clone, PartialEq, Message)]
pub struct DualProtocolPreviewResponse {
    #[prost(string, tag = "1")]
    pub v1_form_urlencoded: String,
    #[prost(string, tag = "2")]
    pub v2_json_payload: String,
    #[prost(string, tag = "3")]
    pub stripe_account: String,
    #[prost(string, tag = "4")]
    pub idempotency_key: String,
    #[prost(uint64, tag = "5")]
    pub max_webhook_payload_bytes: u64,
}

/// Protobuf request message for `VerifyV1Webhook` and `VerifyV2ThinWebhook`.
#[derive(Clone, PartialEq, Message)]
pub struct WebhookVerifyRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub payload: Vec<u8>,
    #[prost(string, tag = "2")]
    pub signature_header: String,
}

/// Protobuf response message for `VerifyV1Webhook`.
#[derive(Clone, PartialEq, Message)]
pub struct V1WebhookVerifyResponse {
    #[prost(bool, tag = "1")]
    pub verified: bool,
    #[prost(string, tag = "2")]
    pub event_id: String,
    #[prost(string, tag = "3")]
    pub event_type: String,
    #[prost(bool, tag = "4")]
    pub is_checkout_completed: bool,
    #[prost(string, tag = "5")]
    pub object_variant: String,
}

/// Protobuf response message for `VerifyV2ThinWebhook`.
#[derive(Clone, PartialEq, Message)]
pub struct V2ThinWebhookVerifyResponse {
    #[prost(bool, tag = "1")]
    pub verified: bool,
    #[prost(bool, tag = "2")]
    pub thin_detected: bool,
    #[prost(string, tag = "3")]
    pub event_id: String,
    #[prost(string, tag = "4")]
    pub event_type: String,
    #[prost(string, tag = "5")]
    pub related_object_id: String,
}

#[derive(Serialize)]
struct WirePatchPayload<'a> {
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    email: Nullable<&'a str>,
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    description: Nullable<&'a str>,
}

/// Converts a [`StripeError`] into an idiomatic [`tonic::Status`].
#[must_use]
pub fn map_stripe_err(err: StripeError) -> Status {
    match err {
        StripeError::InvalidRequest(msg) => Status::invalid_argument(msg),
        StripeError::Webhook(w) => Status::unauthenticated(w.to_string()),
        other => Status::unavailable(other.to_string()),
    }
}

/// Converts a [`StripeWebhookError`] into an idiomatic [`tonic::Status`].
#[must_use]
pub fn map_webhook_err(err: StripeWebhookError) -> Status {
    match err {
        StripeWebhookError::PayloadTooLarge { .. } | StripeWebhookError::EmptyPayload => {
            Status::invalid_argument(err.to_string())
        }
        _ => Status::unauthenticated(err.to_string()),
    }
}

/// Builds a [`StripeClient`] configured with retry policies and security telemetry hooks.
///
/// Passing `"$STRIPE_SECRET_KEY"` automatically resolves the secret from process env,
/// `.env.local`, `.env.dev`, `.env`, or `.env.example`.
#[must_use]
pub fn build_configured_client(secret_key: &str, api_base: Option<&str>) -> StripeClient {
    let workflow = CustomErrorWorkflowBuilder::new()
        .max_retries(3)
        .initial_delay(Duration::from_millis(250))
        .on_security_incident(|err, payload_size| {
            eprintln!("gRPC SECURITY INCIDENT ({payload_size} bytes): {err:?}");
        })
        .on_api_error(|status, code, message| {
            eprintln!("gRPC Stripe API error [HTTP {status} / {code}]: {message}");
        })
        .build();

    let mut config = StripeConfig::new(SecretKey::new(secret_key))
        .with_timeout(Duration::from_secs(15))
        .with_max_retries(3)
        .with_error_workflow(workflow);

    if let Some(base) = api_base {
        config = config.with_api_base(base);
    }

    StripeClient::with_config(config)
}

/// Classifies a deserialized [`EventObject`] discriminated union without cloning.
#[must_use]
pub fn describe_event_object(object: &EventObject) -> &'static str {
    match object {
        EventObject::CheckoutSession(_) => "checkout.session",
        EventObject::PaymentIntent(_) => "payment_intent",
        EventObject::Invoice(_) => "invoice",
        EventObject::Dispute(_) => "dispute",
        EventObject::Refund(_) => "refund",
        EventObject::PaymentLink(_) => "payment_link",
        EventObject::Unknown => "unknown",
    }
}

/// Extracts the `stripe-signature` from gRPC request metadata or falls back to the message field.
fn resolve_signature<'a>(
    metadata: &'a tonic::metadata::MetadataMap,
    field_sig: &'a str,
) -> Result<&'a str, Status> {
    if let Some(meta_val) = metadata.get("stripe-signature")
        && let Ok(s) = meta_val.to_str()
        && !s.trim().is_empty()
    {
        return Ok(s);
    }
    if !field_sig.trim().is_empty() {
        return Ok(field_sig);
    }
    Err(Status::unauthenticated(
        "Missing stripe-signature in gRPC metadata or request message",
    ))
}

/// Tonic gRPC service trait for Module 1 (Core Runtime) & Module 2 (Cryptographic Webhooks).
#[tonic::async_trait]
pub trait CoreWebhooksGrpcService: Send + Sync + 'static {
    async fn get_health(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status>;

    async fn preview_dual_protocol(
        &self,
        request: Request<DualProtocolPreviewRequest>,
    ) -> Result<Response<DualProtocolPreviewResponse>, Status>;

    async fn verify_v1_webhook(
        &self,
        request: Request<WebhookVerifyRequest>,
    ) -> Result<Response<V1WebhookVerifyResponse>, Status>;

    async fn verify_v2_thin_webhook(
        &self,
        request: Request<WebhookVerifyRequest>,
    ) -> Result<Response<V2ThinWebhookVerifyResponse>, Status>;
}

/// Concrete Tonic gRPC service implementation holding a thread-safe [`StripeClient`] and [`SecretKey`].
#[derive(Clone, Debug)]
pub struct CoreWebhooksGrpcServer {
    pub stripe: StripeClient,
    pub webhook_secret: SecretKey,
}

impl CoreWebhooksGrpcServer {
    /// Creates a new `CoreWebhooksGrpcServer` using native `"$STRIPE_SECRET_KEY"` and
    /// `"$STRIPE_WEBHOOK_SECRET"` `.env*` resolution.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            stripe: build_configured_client("$STRIPE_SECRET_KEY", None),
            webhook_secret: SecretKey::new("$STRIPE_WEBHOOK_SECRET"),
        }
    }
}

#[tonic::async_trait]
impl CoreWebhooksGrpcService for CoreWebhooksGrpcServer {
    async fn get_health(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {
            status: "SERVING".to_string(),
            service: "tonic_core_and_webhooks".to_string(),
            api_base: self.stripe.config.api_base.clone(),
            secret_debug_redacted: format!("{:?}", self.webhook_secret),
        }))
    }

    async fn preview_dual_protocol(
        &self,
        request: Request<DualProtocolPreviewRequest>,
    ) -> Result<Response<DualProtocolPreviewResponse>, Status> {
        let req = request.into_inner();
        let email_nullable = match req.email.as_deref() {
            Some(val) => Nullable::Value(val),
            None => Nullable::Omit,
        };
        let description_nullable = if req.clear_description {
            Nullable::Null
        } else {
            Nullable::Omit
        };

        let patch = WirePatchPayload {
            email: email_nullable,
            description: description_nullable,
        };

        let v1_form = serialize_v1_form(&patch).map_err(map_stripe_err)?;
        let v2_json = serialize_v2_json(&patch).map_err(map_stripe_err)?;

        let mut options = RequestOptions::new();
        if let Some(acct) = req.stripe_account.as_deref() {
            options = options.stripe_account(acct);
        }
        if let Some(idem) = req.idempotency_key.as_deref() {
            options = options.idempotency_key(idem);
        }

        Ok(Response::new(DualProtocolPreviewResponse {
            v1_form_urlencoded: v1_form,
            v2_json_payload: v2_json,
            stripe_account: options.stripe_account.unwrap_or_default(),
            idempotency_key: options.idempotency_key.unwrap_or_default(),
            max_webhook_payload_bytes: MAX_WEBHOOK_PAYLOAD_BYTES as u64,
        }))
    }

    async fn verify_v1_webhook(
        &self,
        request: Request<WebhookVerifyRequest>,
    ) -> Result<Response<V1WebhookVerifyResponse>, Status> {
        let (metadata, _, msg) = request.into_parts();
        let sig_header = resolve_signature(&metadata, &msg.signature_header)?;
        let secret = self.webhook_secret.as_str();

        let wrapper = StripeWebhookWrapper::from_raw(&msg.payload, sig_header, secret, None)
            .map_err(map_webhook_err)?;
        let tolerance = ToleranceConfig::default();
        let event = self
            .stripe
            .webhooks()
            .construct_event_with_config(&msg.payload, sig_header, secret, &tolerance)
            .map_err(map_webhook_err)?;
        let typed = event.data.deserialize_object().unwrap_or(EventObject::Unknown);

        Ok(Response::new(V1WebhookVerifyResponse {
            verified: true,
            event_id: wrapper.event_id().to_string(),
            event_type: wrapper.event_type().to_string(),
            is_checkout_completed: wrapper.is_checkout_completed(),
            object_variant: describe_event_object(&typed).to_string(),
        }))
    }

    async fn verify_v2_thin_webhook(
        &self,
        request: Request<WebhookVerifyRequest>,
    ) -> Result<Response<V2ThinWebhookVerifyResponse>, Status> {
        let (metadata, _, msg) = request.into_parts();
        let sig_header = resolve_signature(&metadata, &msg.signature_header)?;
        let thin_detected = is_thin_event(&msg.payload);
        let thin = self
            .stripe
            .webhooks()
            .construct_v2_thin_event(&msg.payload, sig_header, self.webhook_secret.as_str(), None)
            .map_err(map_webhook_err)?;

        Ok(Response::new(V2ThinWebhookVerifyResponse {
            verified: true,
            thin_detected,
            event_id: thin.id,
            event_type: thin.r#type,
            related_object_id: thin
                .related_object
                .map(|r| r.id)
                .unwrap_or_default(),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = CoreWebhooksGrpcServer::from_env();
    let health = server.get_health(Request::new(HealthCheckRequest {})).await?;
    println!(
        "Tonic gRPC Core & Webhooks service initialized: {:?}",
        health.into_inner()
    );
    Ok(())
}
