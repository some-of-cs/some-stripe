//! API V2 Next-Gen Services & High-Throughput Meter Billing (Module 8).
//!
//! # Features
//! - API V2 Core Events & Event Destinations (Amazon EventBridge, Azure Event Grid, Webhooks).
//! - V2 Billing Meters CRUD & usage event streaming.
//! - High-Throughput `MeterStreamCollector` retained in `AppState` for live telemetry.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use some_stripe::{
    AggregationFormula, CreateBillingMeterParams, CreateEventDestinationParams,
    CreateMeterEventAdjustmentParams, CreateMeterEventParams, CreateV2AccountParams,
    CustomerMappingSettings, MeterEventPayload, MeterPayload, ValueSettings,
};

use crate::state::{AppError, AppState};

/// Creates the API V2 next-gen services router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v2/events", get(list_v2_events))
        .route("/v2/events/{id}", get(get_v2_event))
        .route("/v2/event-destinations", post(create_event_destination))
        .route("/v2/accounts", post(create_v2_account))
        .route("/v2/billing/meters", post(create_billing_meter))
        .route("/v2/billing/meters/{id}/events", post(stream_meter_event))
        .route("/v2/billing/meters/adjustments", post(adjust_meter_events))
        .route("/v2/billing/meter-ingestion/stream-demo", post(meter_ingestion_demo))
}

/// List API v2 core events.
pub async fn list_v2_events(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let page = state.stripe.v2().core().events().list(None).await?;
    Ok((StatusCode::OK, Json(page)))
}

/// Retrieve a specific API v2 event by ID.
pub async fn get_v2_event(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let event = state.stripe.v2().core().events().retrieve(&id).await?;
    Ok((StatusCode::OK, Json(event)))
}

/// Request body for creating an Event Destination.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateDestinationRequest {
    pub url: String,
    pub events: Vec<String>,
}

/// Create an event destination (Webhook endpoint).
pub async fn create_event_destination(
    State(state): State<AppState>,
    Json(payload): Json<CreateDestinationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateEventDestinationParams::webhook_endpoint(payload.url, payload.events)
        .with_description("Axum API V2 Event Destination");

    let destination = state.stripe.v2().core().event_destinations().create(&params).await?;
    Ok((StatusCode::CREATED, Json(destination)))
}

/// Request body for creating a V2 Account.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateV2AccountRequest {
    pub display_name: Option<String>,
    pub contact_email: Option<String>,
}

/// Create a V2 core account.
pub async fn create_v2_account(
    State(state): State<AppState>,
    Json(payload): Json<CreateV2AccountRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateV2AccountParams {
        display_name: payload.display_name,
        contact_email: payload.contact_email,
        configuration: None,
        idempotency_key: None,
    };

    let account = state.stripe.v2_accounts().create(&params).await?;
    Ok((StatusCode::CREATED, Json(account)))
}

/// Request body for creating a V2 Billing Meter.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateBillingMeterRequest {
    pub display_name: String,
    pub event_name: String,
}

/// Create a V2 billing usage meter.
pub async fn create_billing_meter(
    State(state): State<AppState>,
    Json(payload): Json<CreateBillingMeterRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateBillingMeterParams {
        display_name: payload.display_name,
        event_name: payload.event_name,
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

    let meter = state.stripe.billing_meters().create(&params).await?;
    Ok((StatusCode::CREATED, Json(meter)))
}

/// Request body for streaming a single meter event.
#[derive(Debug, Deserialize, Serialize)]
pub struct StreamMeterEventRequest {
    pub event_name: String,
    pub customer_id: String,
    pub value: String,
}

/// Stream a real-time usage meter event and record it in the shared [`MeterStreamCollector`].
pub async fn stream_meter_event(
    State(state): State<AppState>,
    Path(_id): Path<String>,
    Json(payload): Json<StreamMeterEventRequest>,
) -> Result<impl IntoResponse, AppError> {
    let collector_event = MeterEventPayload::new(
        payload.event_name.clone(),
        payload.customer_id.clone(),
        payload.value.clone(),
    );
    let _ = state.meter_collector.send(collector_event).await;

    let params = CreateMeterEventParams {
        event_name: payload.event_name,
        payload: MeterPayload {
            stripe_customer_id: payload.customer_id,
            value: payload.value,
        },
        timestamp: None,
        identifier: None,
    };

    let event = state.stripe.meter_events().create(&params).await?;
    Ok((StatusCode::CREATED, Json(event)))
}

/// Request body for adjusting historical meter events.
#[derive(Debug, Deserialize, Serialize)]
pub struct AdjustMeterEventRequest {
    pub event_name: String,
}

/// Adjust historical meter event readings.
pub async fn adjust_meter_events(
    State(state): State<AppState>,
    Json(payload): Json<AdjustMeterEventRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateMeterEventAdjustmentParams {
        event_name: payload.event_name,
        adjustment_type: "cancel".to_string(),
        cancel: None,
        reason: None,
        idempotency_key: None,
    };

    let adjustment = state.stripe.meter_event_adjustments().create(&params).await?;
    Ok((StatusCode::CREATED, Json(adjustment)))
}

/// Returns live telemetry statistics from the shared [`MeterStreamCollector`] in [`AppState`].
pub async fn meter_ingestion_demo(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let stats = state.meter_collector.stats();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ready",
            "events_received": stats.events_received,
            "events_flushed": stats.events_flushed,
            "batches_sent": stats.batches_sent,
            "duplicates_dropped": stats.duplicates_dropped,
        })),
    ))
}
