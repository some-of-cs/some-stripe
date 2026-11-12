//! Stripe API V2 Next-Gen Services routes (Module 8).
//!
//! Exposes endpoints for:
//! - V2 Core Events (`stripe.v2().core().events()`).
//! - V2 Event Destinations (`stripe.v2().core().event_destinations()`).
//! - V2 Merchant Accounts (`stripe.v2().core().accounts()`).
//! - Thin Event Hydration (`stripe.hydrate_thin_event(&thin_event)`).
//! - V2 High-Throughput Usage Meter Billing (`stripe.v2().billing().meter_events()`).

use rocket::serde::json::Json;
use rocket::State;
use some_stripe::{
    CreateMeterEventParams, EventDestination, V2Account, V2Event, V2MeterEvent, V2ThinEvent,
};

use crate::config::AppConfig;
use crate::models::{ApiResponse, CreateMeterEventRequest, ErrorResponse};

/// Retrieves a Stripe API V2 Core Event by ID.
///
/// Route: `GET /api/v2/events/<id>`
#[rocket::get("/v2/events/<id>")]
pub async fn get_v2_event(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<V2Event>>, ErrorResponse> {
    let event = config
        .client
        .v2()
        .core()
        .events()
        .retrieve(id)
        .await
        .map_err(|e| ErrorResponse::not_found(format!("V2 Event '{id}' not found: {e}")))?;

    Ok(Json(ApiResponse::new(event)))
}

/// Retrieves a V2 Event Destination (Webhook, EventBridge, Azure Event Grid) by ID.
///
/// Route: `GET /api/v2/destinations/<id>`
#[rocket::get("/v2/destinations/<id>")]
pub async fn get_v2_destination(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<EventDestination>>, ErrorResponse> {
    let dest = config
        .client
        .v2()
        .core()
        .event_destinations()
        .retrieve(id)
        .await
        .map_err(|e| ErrorResponse::not_found(format!("EventDestination '{id}' not found: {e}")))?;

    Ok(Json(ApiResponse::new(dest)))
}

/// Retrieves a Stripe API V2 next-generation account.
///
/// Route: `GET /api/v2/accounts/<id>`
#[rocket::get("/v2/accounts/<id>")]
pub async fn get_v2_account(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<V2Account>>, ErrorResponse> {
    let acct = config
        .client
        .v2()
        .core()
        .accounts()
        .retrieve(id)
        .await
        .map_err(|e| ErrorResponse::not_found(format!("V2 Account '{id}' not found: {e}")))?;

    Ok(Json(ApiResponse::new(acct)))
}

/// Hydrates an inbound V2 thin event notification into the full live V2 event.
///
/// Route: `POST /api/v2/events/hydrate`
#[rocket::post("/v2/events/hydrate", data = "<thin_event>")]
pub async fn hydrate_v2_event(
    config: &State<AppConfig>,
    thin_event: Json<V2ThinEvent>,
) -> Result<Json<ApiResponse<V2Event>>, ErrorResponse> {
    let full_event = config
        .client
        .hydrate_thin_event(&thin_event)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to hydrate thin event: {e}")))?;

    Ok(Json(ApiResponse::new(full_event)))
}

/// Ingests a high-throughput usage meter event for AI tokens or API usage.
///
/// Route: `POST /api/v2/billing/meter-events`
#[rocket::post("/v2/billing/meter-events", data = "<req>")]
pub async fn record_meter_event(
    config: &State<AppConfig>,
    req: Json<CreateMeterEventRequest>,
) -> Result<Json<ApiResponse<V2MeterEvent>>, ErrorResponse> {
    let params = CreateMeterEventParams::new(
        &req.event_name,
        &req.stripe_customer_id,
        &req.value,
    );

    let meter_event = config
        .client
        .v2()
        .billing()
        .meter_events()
        .create(&params)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to record meter event: {e}")))?;

    Ok(Json(ApiResponse::new(meter_event)))
}
