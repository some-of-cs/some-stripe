//! # 01: Axum Webhooks Service Example
//!
//! Production-grade Stripe webhook endpoint implementation in Axum featuring:
//! - Raw `Bytes` body streaming to prevent pre-parsing tampering.
//! - Constant-time HMAC-SHA256 signature verification (`subtle::ConstantTimeEq`).
//! - 300-second replay attack protection tolerance (`Some(300)`).
//! - API V2 Thin Event detection and routing via `construct_v2_thin_event` + `ThinEventHydrator`.
//! - Discriminated union `EventObject` pattern matching for V1 events.
//!
//! Run with:
//! ```bash
//! export STRIPE_SECRET_KEY="sk_test_..."
//! export STRIPE_WEBHOOK_SECRET="whsec_..."
//! cargo run --example 01_axum_webhooks
//! ```

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde_json::json;
use some_stripe::{is_thin_event, EventObject, SecretKey, StripeClient, ThinEventHydrator};
use std::sync::Arc;

/// Shared webhook state holding the lock-free [`StripeClient`] and [`SecretKey`] webhook signing secret.
#[derive(Clone)]
pub struct WebhookState {
    /// Thread-safe Stripe client.
    pub stripe: Arc<StripeClient>,
    /// Webhook endpoint signing secret (`SecretKey` with automatic `$VAR_NAME` resolution & zeroization on drop).
    pub webhook_secret: SecretKey,
}

impl WebhookState {
    /// Creates a new [`WebhookState`] from an existing [`StripeClient`] and signing secret
    /// (supports both literal `"whsec_..."` and `"$STRIPE_WEBHOOK_SECRET"`).
    pub fn with_client(client: StripeClient, webhook_secret: impl Into<SecretKey>) -> Self {
        Self {
            stripe: Arc::new(client),
            webhook_secret: webhook_secret.into(),
        }
    }
}

/// Builds the Axum [`Router`] for the Webhooks microservice.
pub fn build_router(state: WebhookState) -> Router {
    Router::new()
        .route("/webhooks/stripe", post(handle_webhook))
        .with_state(state)
}

/// Entry point for the standalone Webhooks microservice.
#[allow(dead_code)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Automatically resolves $STRIPE_SECRET_KEY and $STRIPE_WEBHOOK_SECRET from env, .env.local, .env.dev, .env, or .env.example
    let state = WebhookState::with_client(
        StripeClient::new("$STRIPE_SECRET_KEY"),
        SecretKey::new("$STRIPE_WEBHOOK_SECRET"),
    );
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    tracing::info!("Webhook microservice listening on http://0.0.0.0:3001/webhooks/stripe");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Verifies the `Stripe-Signature` header in constant time and dispatches V1 or V2 thin events.
pub async fn handle_webhook(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let sig_header = headers
        .get("Stripe-Signature")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing Stripe-Signature header".to_string()))?;

    // 1. Route API V2 Thin Event notifications through construct_v2_thin_event before V1 parsing
    if is_thin_event(&body) {
        let thin_event = state
            .stripe
            .webhooks()
            .construct_v2_thin_event(&body, sig_header, state.webhook_secret.expose_secret(), Some(300))
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Webhook verification failed: {e}")))?;

        tracing::info!(
            id = %thin_event.id,
            r#type = %thin_event.event_type,
            "Verified Stripe V2 thin event"
        );

        let hydrator = ThinEventHydrator::new(&state.stripe);
        let hydrated_id = if let Ok(v2_event) = hydrator.hydrate_thin_event(&thin_event).await {
            tracing::info!(v2_id = %v2_event.id, "Hydrated V2 thin event");
            Some(v2_event.id)
        } else {
            None
        };

        return Ok((
            StatusCode::OK,
            Json(json!({
                "received": true,
                "event_id": thin_event.id,
                "v2_thin_event": true,
                "hydrated_id": hydrated_id,
            })),
        ));
    }

    // 2. Verify cryptographic signature for V1 snapshot events in constant time
    let event = state
        .stripe
        .webhooks()
        .construct_event(&body, sig_header, state.webhook_secret.expose_secret(), Some(300))
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Webhook verification failed: {e}")))?;

    tracing::info!(id = %event.id, r#type = %event.event_type, "Verified Stripe V1 event");

    // 3. Dispatch payload
    log_v1_event_object(event.deserialize_object());

    Ok((
        StatusCode::OK,
        Json(json!({
            "received": true,
            "event_id": event.id,
            "v2_thin_event": false,
        })),
    ))
}

/// Logs the deserialized [`EventObject`] variant with low cyclomatic complexity ($CC \le 5$).
fn log_v1_event_object(parsed: Result<EventObject, serde_json::Error>) {
    match parsed {
        Ok(EventObject::PaymentIntent(pi)) => {
            tracing::info!(id = %pi.id, amount = pi.amount, "PaymentIntent status: {}", pi.status);
        }
        Ok(EventObject::CheckoutSession(cs)) => {
            tracing::info!(id = %cs.id, "CheckoutSession mode: {:?}", cs.mode);
        }
        Ok(EventObject::Invoice(inv)) => {
            tracing::info!(id = ?inv.id, "Invoice amount: {:?}", inv.amount_due);
        }
        _ => {}
    }
}
