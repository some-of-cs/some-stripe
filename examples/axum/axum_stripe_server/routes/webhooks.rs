//! Webhook processing routes with constant-time HMAC-SHA256 signature verification.
//!
//! # Security Invariants (Gate G2 & Module 2)
//! - **Raw Body Extraction**: Extracts raw bytes (`axum::body::Bytes`) before deserialization
//!   to ensure byte-exact cryptographic integrity.
//! - **Constant-Time Verification**: Validates the `Stripe-Signature` header using `subtle::ConstantTimeEq`
//!   to eliminate timing side-channel attacks.
//! - **Replay Attack Tolerance**: Enforces a strict 300-second timestamp drift tolerance (`Some(300)`).
//! - **Discriminated Event Union**: Dispatches events safely across [`EventObject`] variants without panic.
//! - **V2 Thin Event Hydration**: Routes lightweight V2 thin events through `construct_v2_thin_event`
//!   and hydrates them via [`ThinEventHydrator`].

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde_json::json;
use some_stripe::{is_thin_event, EventObject, ThinEventHydrator};

use crate::state::{AppError, AppState};

/// Creates the webhook router.
pub fn router() -> Router<AppState> {
    Router::new().route("/webhooks/stripe", post(handle_stripe_webhook))
}

/// Webhook ingestion handler verifying signatures in constant time.
///
/// # Invariants
/// - Never parses JSON before signature validation.
/// - Routes V2 thin events via `construct_v2_thin_event` and V1 snapshot events via `construct_event`.
/// - Enforces a 300-second tolerance window (`Some(300)`).
pub async fn handle_stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, AppError> {
    // 1. Extract Stripe-Signature header
    let sig_header = headers
        .get("Stripe-Signature")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing Stripe-Signature header".to_string()))?;

    // 2. Check if payload is an API V2 Thin Event notification and verify signature accordingly
    if is_thin_event(&body) {
        let thin_event = state.stripe.webhooks().construct_v2_thin_event(
            &body,
            sig_header,
            state.webhook_secret.expose_secret(),
            Some(300),
        )?;

        tracing::info!(
            event_id = %thin_event.id,
            event_type = %thin_event.event_type,
            "Verified Stripe API v2 thin event notification"
        );

        let hydrator = ThinEventHydrator::new(&state.stripe);
        let hydrated_id = if let Ok(v2_event) = hydrator.hydrate_thin_event(&thin_event).await {
            tracing::info!(
                v2_id = %v2_event.id,
                v2_type = %v2_event.r#type,
                "Successfully hydrated API v2 event"
            );
            Some(v2_event.id)
        } else {
            None
        };

        return Ok((
            StatusCode::OK,
            Json(json!({
                "received": true,
                "event_id": thin_event.id,
                "type": thin_event.event_type,
                "v2_thin_event": true,
                "hydrated_id": hydrated_id,
            })),
        ));
    }

    // 3. Verify signature and construct V1 snapshot event with constant-time equality and 300s tolerance
    let event = state.stripe.webhooks().construct_event(
        &body,
        sig_header,
        state.webhook_secret.expose_secret(),
        Some(300),
    )?;

    tracing::info!(
        event_id = %event.id,
        event_type = %event.event_type,
        "Received verified Stripe V1 webhook event"
    );

    // 4. Safely handle strongly typed event payload via EventObject union
    log_webhook_event_object(event.deserialize_object(), &event.event_type);

    Ok((
        StatusCode::OK,
        Json(json!({
            "received": true,
            "event_id": event.id,
            "type": event.event_type,
            "v2_thin_event": false,
        })),
    ))
}

/// Dispatches deserialized [`EventObject`] logging while keeping cyclomatic complexity <= 6.
fn log_webhook_event_object(
    parsed: Result<EventObject, serde_json::Error>,
    event_type: &str,
) {
    match parsed {
        Ok(obj) => log_verified_event_variant(&obj, event_type),
        Err(err) => {
            tracing::warn!(
                error = %err,
                "Failed to deserialize event payload object into EventObject union"
            );
        }
    }
}

/// Logs payment, checkout, and billing [`EventObject`] variants ($CC \le 5$).
fn log_verified_event_variant(obj: &EventObject, event_type: &str) {
    match obj {
        EventObject::PaymentIntent(pi) => {
            tracing::info!(
                id = %pi.id,
                amount = pi.amount,
                status = %pi.status,
                "Processed PaymentIntent webhook"
            );
        }
        EventObject::CheckoutSession(session) => {
            tracing::info!(
                id = %session.id,
                customer = ?session.customer,
                mode = ?session.mode,
                "Processed CheckoutSession webhook"
            );
        }
        EventObject::Invoice(inv) => {
            tracing::info!(
                id = ?inv.id,
                customer = ?inv.customer,
                amount_due = ?inv.amount_due,
                "Processed Invoice webhook"
            );
        }
        other => log_secondary_event_variant(other, event_type),
    }
}

/// Logs customer, dispute, refund, and fallback [`EventObject`] variants ($CC \le 5$).
fn log_secondary_event_variant(obj: &EventObject, event_type: &str) {
    match obj {
        EventObject::Customer(cust) => {
            tracing::info!(id = %cust.id, email = ?cust.email, "Processed Customer webhook");
        }
        EventObject::Dispute(disp) => {
            tracing::warn!(id = %disp.id, status = %disp.status, "Dispute created or updated");
        }
        EventObject::Refund(ref_obj) => {
            tracing::info!(id = %ref_obj.id, amount = ref_obj.amount, "Processed Refund webhook");
        }
        EventObject::Unknown => {
            tracing::debug!(event_type = %event_type, "Unhandled or custom webhook event");
        }
        _ => {}
    }
}
