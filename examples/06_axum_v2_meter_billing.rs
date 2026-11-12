//! # 06: Axum API V2 & High-Throughput Meter Billing Service Example
//!
//! Next-Gen API V2 Services and AI / Usage Meter Ingestion Engine:
//! - API V2 Core Events & Event Destinations (EventBridge, Azure Event Grid, Webhooks).
//! - V2 Billing Meters CRUD with matched `"value"` payload key.
//! - High-Throughput `MeterStreamCollector` retained in `V2State` for live telemetry.
//!
//! Run with:
//! ```bash
//! export STRIPE_SECRET_KEY="sk_test_..."
//! cargo run --example 06_axum_v2_meter_billing
//! ```

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use some_stripe::{
    AggregationFormula, CreateBillingMeterParams, CreateEventDestinationParams,
    CreateMeterEventParams, CreateV2AccountParams, CustomerMappingSettings, MeterEventPayload,
    MeterPayload, MeterStreamCollector, StripeClient, ValueSettings,
};
use std::sync::Arc;
use std::time::Duration;

/// Shared state holding the lock-free [`StripeClient`] and active [`MeterStreamCollector`].
#[derive(Clone)]
pub struct V2State {
    /// Thread-safe Stripe client.
    pub stripe: Arc<StripeClient>,
    /// Shared active meter stream collector tracking service ingestion traffic.
    pub collector: Arc<MeterStreamCollector>,
}

impl V2State {
    /// Creates a new [`V2State`] from an existing [`StripeClient`].
    pub fn with_client(client: StripeClient) -> Self {
        let engine = client.v2().billing().create_ingestion_engine();
        let collector = engine.create_stream_collector(1000, Duration::from_millis(500));
        Self {
            stripe: Arc::new(client),
            collector: Arc::new(collector),
        }
    }
}

/// Builds the Axum [`Router`] for the API V2 & Meter Billing microservice.
pub fn build_router(state: V2State) -> Router {
    Router::new()
        .route("/v2/events", get(list_events))
        .route("/v2/destinations", post(create_destination))
        .route("/v2/accounts", post(create_account))
        .route("/v2/meters", post(create_meter))
        .route("/v2/meters/event", post(stream_event))
        .route("/v2/meters/engine-stats", get(get_engine_stats))
        .with_state(state)
}

/// Entry point for the API V2 & Meter Billing microservice.
#[allow(dead_code)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Automatically resolves $STRIPE_SECRET_KEY from process env, .env.local, .env.dev, .env, or .env.example
    let state = V2State::with_client(StripeClient::new("$STRIPE_SECRET_KEY"));
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3006").await?;
    tracing::info!("API V2 & Meter Billing service listening on http://0.0.0.0:3006");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Lists API V2 Core Events.
pub async fn list_events(
    State(state): State<V2State>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let page = state
        .stripe
        .v2()
        .core()
        .events()
        .list(None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::OK, Json(page)))
}

/// Request payload for creating a V2 Event Destination.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateDestReq {
    pub url: String,
    pub events: Vec<String>,
}

/// Creates a V2 Webhook Event Destination.
pub async fn create_destination(
    State(state): State<V2State>,
    Json(p): Json<CreateDestReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreateEventDestinationParams::webhook_endpoint(p.url, p.events)
        .with_description("Axum V2 Event Notification Destination");

    let dest = state
        .stripe
        .v2()
        .core()
        .event_destinations()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(dest)))
}

/// Request payload for creating a V2 Account.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateAcctReq {
    pub display_name: String,
}

/// Creates a V2 Core Account.
pub async fn create_account(
    State(state): State<V2State>,
    Json(p): Json<CreateAcctReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreateV2AccountParams {
        display_name: Some(p.display_name),
        contact_email: None,
        configuration: None,
        idempotency_key: None,
    };

    let acct = state
        .stripe
        .v2_accounts()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(acct)))
}

/// Request payload for creating a V2 Billing Meter.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateMeterReq {
    pub display_name: String,
    pub event_name: String,
}

/// Creates a V2 Billing Meter with customer ID mapping and `"value"` payload key.
pub async fn create_meter(
    State(state): State<V2State>,
    Json(p): Json<CreateMeterReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreateBillingMeterParams {
        display_name: p.display_name,
        event_name: p.event_name,
        aggregation_formula: AggregationFormula::Sum,
        value_settings: ValueSettings {
            event_payload_key: "value".to_string(),
        },
        customer_mapping: CustomerMappingSettings {
            event_payload_key: "stripe_customer_id".to_string(),
            mapping_type: "by_id".to_string(),
        },
        idempotency_key: None,
    };

    let meter = state
        .stripe
        .billing_meters()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(meter)))
}

/// Request payload for streaming a usage meter event.
#[derive(Debug, Deserialize, Serialize)]
pub struct StreamEventReq {
    pub event_name: String,
    pub customer_id: String,
    pub value: String,
}

/// Streams a single usage meter event to Stripe Billing and records it in the shared [`MeterStreamCollector`].
pub async fn stream_event(
    State(state): State<V2State>,
    Json(p): Json<StreamEventReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let collector_event = MeterEventPayload::new(
        p.event_name.clone(),
        p.customer_id.clone(),
        p.value.clone(),
    );
    let _ = state.collector.send(collector_event).await;

    let params = CreateMeterEventParams {
        event_name: p.event_name,
        payload: MeterPayload {
            stripe_customer_id: p.customer_id,
            value: p.value,
        },
        timestamp: None,
        identifier: None,
    };

    let event = state
        .stripe
        .meter_events()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(event)))
}

/// Returns live telemetry statistics from the shared [`MeterStreamCollector`] in [`V2State`].
pub async fn get_engine_stats(
    State(state): State<V2State>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let stats = state.collector.stats();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "events_received": stats.events_received,
            "events_flushed": stats.events_flushed,
            "batches_sent": stats.batches_sent,
            "duplicates_dropped": stats.duplicates_dropped,
        })),
    ))
}
