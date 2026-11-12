//! # Actix-Web Example 8: Stripe API v2 Core, Event Destinations, Thin Hydration & Meter Billing (Module 8)
//!
//! Demonstrates how to integrate **Module 8 (Stripe API v2 Next-Gen Services)** of `some-stripe`
//! inside an `actix-web` application.
//!
//! ## Services Covered
//! - `stripe.v2().core()`:
//!   - `events()`: `retrieve` and `list_events`
//!   - `event_destinations()`: `create`, `retrieve`, `update`, `delete`, `list`, `enable`,
//!     `disable`, and `ping` (Amazon EventBridge, Azure Event Grid, Webhooks)
//!   - `accounts()`: `create`, `retrieve`, `update`, `del`, `list`, and `create_token`
//!   - `hydrator()`: `hydrate_from_slice` / `stripe.hydrate_thin_event()` with 300s freshness guard
//! - `stripe.v2().billing()`:
//!   - `meters()`: `create`, `retrieve`, `update`, `deactivate`, `reactivate`, and `list`
//!   - `meter_event_session()`: `create`, `retrieve`, and `expire`
//!   - `meter_event_stream()`: `create_batch`
//!   - `meter_event_adjustments()`: `create`, `retrieve`, and `list`
//!   - `stripe.meter_events()`: `create` (synchronous meter ingestion)
//!   - `MeterIngestionEngine`: High-throughput buffered AI token / usage meter ingestion with
//!     ephemeral session bearer token caching (`ZeroizeOnDrop`) and deduplication
//!
//! ## Run the Example
//! ```bash
//! STRIPE_SECRET_KEY=sk_test_123 cargo run --example actix_v2_next_gen_services
//! ```

use actix_web::{App, HttpResponse, HttpServer, ResponseError, delete, get, post, web};
use some_stripe::{
    CreateBillingMeterParams, CreateEventDestinationParams, CreateMeterEventAdjustmentParams,
    CreateMeterEventParams, CreateMeterEventSessionParams, CreateMeterEventStreamParams,
    CreateV2AccountParams, CreateV2AccountTokenParams, HydrationError, ListBillingMetersParams,
    ListMeterEventAdjustmentsParams, ListV2AccountsParams, ListV2EventsParams, MeterEventPayload,
    MeterIngestionEngine, SecretKey, StripeClient, StripeError, UpdateBillingMeterParams,
    UpdateEventDestinationParams, UpdateV2AccountParams,
};
use std::fmt;

/// Shared state holding both `StripeClient` and the high-throughput `MeterIngestionEngine`.
pub struct V2AppState {
    pub stripe: StripeClient,
    pub ingestion_engine: MeterIngestionEngine,
}

/// Unified Actix-Web error wrapper for API v2 and `ThinEventHydrator` operations.
#[derive(Debug)]
pub enum V2ApiError {
    Stripe(StripeError),
    Hydration(HydrationError),
}

impl fmt::Display for V2ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stripe(err) => write!(f, "{err}"),
            Self::Hydration(err) => write!(f, "{err}"),
        }
    }
}

impl From<StripeError> for V2ApiError {
    fn from(err: StripeError) -> Self {
        Self::Stripe(err)
    }
}

impl From<HydrationError> for V2ApiError {
    fn from(err: HydrationError) -> Self {
        Self::Hydration(err)
    }
}

impl ResponseError for V2ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            Self::Stripe(err) => HttpResponse::BadGateway().json(serde_json::json!({
                "error_type": "stripe_v2_error",
                "message": err.to_string(),
            })),
            Self::Hydration(err) => HttpResponse::BadRequest().json(serde_json::json!({
                "error_type": "thin_event_hydration_error",
                "message": err.to_string(),
            })),
        }
    }
}

// ============================================================================
// 1. V2 Core Events & Thin Event Hydration (`stripe.v2().core()`)
// ============================================================================

/// `GET /v2/core/events/{id}` — Retrieves a full v2 core event via
/// `stripe.v2().core().events().retrieve()`.
#[get("/v2/core/events/{id}")]
pub async fn retrieve_v2_event(
    state: web::Data<V2AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, V2ApiError> {
    let event = state
        .stripe
        .v2()
        .core()
        .events()
        .retrieve(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(event))
}

/// `GET /v2/core/events` — Lists v2 core events via `stripe.v2().core().events().list_events()`.
#[get("/v2/core/events")]
pub async fn list_v2_events(
    state: web::Data<V2AppState>,
    query: web::Query<ListV2EventsParams>,
) -> Result<HttpResponse, V2ApiError> {
    let list = state
        .stripe
        .v2()
        .core()
        .events()
        .list_events(Some(&query))
        .await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /v2/core/hydrate` — Parses a raw v2 Thin Event payload and hydrates the full event
/// with 300-second freshness verification via `stripe.v2().core().hydrator().hydrate_from_slice()`.
#[post("/v2/core/hydrate")]
pub async fn hydrate_v2_thin_event(
    state: web::Data<V2AppState>,
    payload: web::Bytes,
) -> Result<HttpResponse, V2ApiError> {
    let hydrated = state
        .stripe
        .v2()
        .core()
        .hydrator()
        .hydrate_from_slice(&payload)
        .await?;
    Ok(HttpResponse::Ok().json(hydrated))
}

// ============================================================================
// 2. V2 Event Destinations (`stripe.v2().core().event_destinations()`)
// ============================================================================

/// `POST /v2/core/event-destinations` — Creates an EventDestination (Webhook, AWS EventBridge,
/// or Azure Event Grid) via `stripe.v2().core().event_destinations().create()`.
#[post("/v2/core/event-destinations")]
pub async fn create_event_destination(
    state: web::Data<V2AppState>,
    params: web::Json<CreateEventDestinationParams>,
) -> Result<HttpResponse, V2ApiError> {
    let dest = state
        .stripe
        .v2()
        .core()
        .event_destinations()
        .create(&params)
        .await?;
    Ok(HttpResponse::Created().json(dest))
}

/// `GET /v2/core/event-destinations/{id}` — Retrieves an EventDestination via
/// `stripe.v2().core().event_destinations().retrieve()`.
#[get("/v2/core/event-destinations/{id}")]
pub async fn retrieve_event_destination(
    state: web::Data<V2AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, V2ApiError> {
    let dest = state
        .stripe
        .v2()
        .core()
        .event_destinations()
        .retrieve(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(dest))
}

/// `POST /v2/core/event-destinations/{id}` — Updates an EventDestination via
/// `stripe.v2().core().event_destinations().update()`.
#[post("/v2/core/event-destinations/{id}")]
pub async fn update_event_destination(
    state: web::Data<V2AppState>,
    id: web::Path<String>,
    params: web::Json<UpdateEventDestinationParams>,
) -> Result<HttpResponse, V2ApiError> {
    let dest = state
        .stripe
        .v2()
        .core()
        .event_destinations()
        .update(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(dest))
}

/// `DELETE /v2/core/event-destinations/{id}` — Deletes an EventDestination via
/// `stripe.v2().core().event_destinations().delete()`.
#[delete("/v2/core/event-destinations/{id}")]
pub async fn delete_event_destination(
    state: web::Data<V2AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, V2ApiError> {
    let dest = state
        .stripe
        .v2()
        .core()
        .event_destinations()
        .delete(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(dest))
}

/// `GET /v2/core/event-destinations` — Lists EventDestinations via
/// `stripe.v2().core().event_destinations().list()`.
#[get("/v2/core/event-destinations")]
pub async fn list_event_destinations(
    state: web::Data<V2AppState>,
) -> Result<HttpResponse, V2ApiError> {
    let list = state.stripe.v2().core().event_destinations().list().await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /v2/core/event-destinations/{id}/enable` — Enables an EventDestination via
/// `stripe.v2().core().event_destinations().enable()`.
#[post("/v2/core/event-destinations/{id}/enable")]
pub async fn enable_event_destination(
    state: web::Data<V2AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, V2ApiError> {
    let dest = state
        .stripe
        .v2()
        .core()
        .event_destinations()
        .enable(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(dest))
}

/// `POST /v2/core/event-destinations/{id}/disable` — Disables an EventDestination via
/// `stripe.v2().core().event_destinations().disable()`.
#[post("/v2/core/event-destinations/{id}/disable")]
pub async fn disable_event_destination(
    state: web::Data<V2AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, V2ApiError> {
    let dest = state
        .stripe
        .v2()
        .core()
        .event_destinations()
        .disable(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(dest))
}

/// `POST /v2/core/event-destinations/{id}/ping` — Sends a test ping event to an EventDestination
/// via `stripe.v2().core().event_destinations().ping()`.
#[post("/v2/core/event-destinations/{id}/ping")]
pub async fn ping_event_destination(
    state: web::Data<V2AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, V2ApiError> {
    let dest = state
        .stripe
        .v2()
        .core()
        .event_destinations()
        .ping(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(dest))
}

// ============================================================================
// 3. V2 Core Accounts (`stripe.v2().core().accounts()`)
// ============================================================================

/// `POST /v2/core/accounts` — Creates a v2 Core Account via `stripe.v2().core().accounts().create()`.
#[post("/v2/core/accounts")]
pub async fn create_v2_account(
    state: web::Data<V2AppState>,
    params: web::Json<CreateV2AccountParams>,
) -> Result<HttpResponse, V2ApiError> {
    let acct = state.stripe.v2().core().accounts().create(&params).await?;
    Ok(HttpResponse::Created().json(acct))
}

/// `GET /v2/core/accounts/{id}` — Retrieves a v2 Core Account via
/// `stripe.v2().core().accounts().retrieve()`.
#[get("/v2/core/accounts/{id}")]
pub async fn retrieve_v2_account(
    state: web::Data<V2AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, V2ApiError> {
    let acct = state
        .stripe
        .v2()
        .core()
        .accounts()
        .retrieve(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(acct))
}

/// `POST /v2/core/accounts/{id}` — Updates a v2 Core Account via
/// `stripe.v2().core().accounts().update()`.
#[post("/v2/core/accounts/{id}")]
pub async fn update_v2_account(
    state: web::Data<V2AppState>,
    id: web::Path<String>,
    params: web::Json<UpdateV2AccountParams>,
) -> Result<HttpResponse, V2ApiError> {
    let acct = state
        .stripe
        .v2()
        .core()
        .accounts()
        .update(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(acct))
}

/// `DELETE /v2/core/accounts/{id}` — Deletes a v2 Core Account via
/// `stripe.v2().core().accounts().del()`.
#[delete("/v2/core/accounts/{id}")]
pub async fn delete_v2_account(
    state: web::Data<V2AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, V2ApiError> {
    let acct = state.stripe.v2().core().accounts().del(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(acct))
}

/// `GET /v2/core/accounts` — Lists v2 Core Accounts via `stripe.v2().core().accounts().list_page()`.
#[get("/v2/core/accounts")]
pub async fn list_v2_accounts(
    state: web::Data<V2AppState>,
    query: web::Query<ListV2AccountsParams>,
) -> Result<HttpResponse, V2ApiError> {
    let page = state
        .stripe
        .v2()
        .core()
        .accounts()
        .list_page(Some(&query))
        .await?;
    Ok(HttpResponse::Ok().json(page))
}

/// `POST /v2/core/accounts/{id}/tokens` — Mints an ephemeral v2 Account Access Token via
/// `stripe.v2().core().accounts().create_token()`.
#[post("/v2/core/accounts/{id}/tokens")]
pub async fn create_v2_account_token(
    state: web::Data<V2AppState>,
    id: web::Path<String>,
    params: web::Json<CreateV2AccountTokenParams>,
) -> Result<HttpResponse, V2ApiError> {
    let token = state
        .stripe
        .v2()
        .core()
        .accounts()
        .create_token(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Created().json(serde_json::json!({
        "account_id": id.as_str(),
        "expires_at": token.expires_at,
        "token_debug_redacted": format!("{token:?}"),
    })))
}

// ============================================================================
// 4. V2 Billing Meters, Sessions, Streams, Adjustments & Ingestion Engine
// ============================================================================

/// `POST /v2/billing/meters` — Creates a v2 Billing Meter via `stripe.v2().billing().meters().create()`.
#[post("/v2/billing/meters")]
pub async fn create_billing_meter(
    state: web::Data<V2AppState>,
    params: web::Json<CreateBillingMeterParams>,
) -> Result<HttpResponse, V2ApiError> {
    let meter = state.stripe.v2().billing().meters().create(&params).await?;
    Ok(HttpResponse::Created().json(meter))
}

/// `GET /v2/billing/meters/{id}` — Retrieves a v2 Billing Meter via
/// `stripe.v2().billing().meters().retrieve()`.
#[get("/v2/billing/meters/{id}")]
pub async fn retrieve_billing_meter(
    state: web::Data<V2AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, V2ApiError> {
    let meter = state
        .stripe
        .v2()
        .billing()
        .meters()
        .retrieve(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(meter))
}

/// `POST /v2/billing/meters/{id}` — Updates a v2 Billing Meter via
/// `stripe.v2().billing().meters().update()`.
#[post("/v2/billing/meters/{id}")]
pub async fn update_billing_meter(
    state: web::Data<V2AppState>,
    id: web::Path<String>,
    params: web::Json<UpdateBillingMeterParams>,
) -> Result<HttpResponse, V2ApiError> {
    let meter = state
        .stripe
        .v2()
        .billing()
        .meters()
        .update(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(meter))
}

/// `POST /v2/billing/meters/{id}/deactivate` — Deactivates a v2 Billing Meter via
/// `stripe.v2().billing().meters().deactivate()`.
#[post("/v2/billing/meters/{id}/deactivate")]
pub async fn deactivate_billing_meter(
    state: web::Data<V2AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, V2ApiError> {
    let meter = state
        .stripe
        .v2()
        .billing()
        .meters()
        .deactivate(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(meter))
}

/// `POST /v2/billing/meters/{id}/reactivate` — Reactivates a v2 Billing Meter via
/// `stripe.v2().billing().meters().reactivate()`.
#[post("/v2/billing/meters/{id}/reactivate")]
pub async fn reactivate_billing_meter(
    state: web::Data<V2AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, V2ApiError> {
    let meter = state
        .stripe
        .v2()
        .billing()
        .meters()
        .reactivate(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(meter))
}

/// `GET /v2/billing/meters` — Lists v2 Billing Meters via `stripe.v2().billing().meters().list()`.
#[get("/v2/billing/meters")]
pub async fn list_billing_meters(
    state: web::Data<V2AppState>,
    query: web::Query<ListBillingMetersParams>,
) -> Result<HttpResponse, V2ApiError> {
    let list = state.stripe.v2().billing().meters().list(&query).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /v2/billing/meter-event-sessions` — Creates an ephemeral meter event session via
/// `stripe.v2().billing().meter_event_session().create()`.
#[post("/v2/billing/meter-event-sessions")]
pub async fn create_meter_event_session(
    state: web::Data<V2AppState>,
    params: Option<web::Json<CreateMeterEventSessionParams>>,
) -> Result<HttpResponse, V2ApiError> {
    let session = state
        .stripe
        .v2()
        .billing()
        .meter_event_session()
        .create(params.as_deref())
        .await?;
    Ok(HttpResponse::Created().json(serde_json::json!({
        "id": session.id,
        "expires_at": session.expires_at,
        "token_redacted": format!("{:?}", session.authentication_token),
    })))
}

/// `POST /v2/billing/meter-events` — Synchronously ingests a single meter event via
/// `stripe.meter_events().create()`.
#[post("/v2/billing/meter-events")]
pub async fn create_synchronous_meter_event(
    state: web::Data<V2AppState>,
    params: web::Json<CreateMeterEventParams>,
) -> Result<HttpResponse, V2ApiError> {
    let event = state.stripe.meter_events().create(&params).await?;
    Ok(HttpResponse::Created().json(event))
}

/// `POST /v2/billing/meter-event-stream` — Streams a batch of meter events via
/// `stripe.v2().billing().meter_event_stream().create()`.
#[post("/v2/billing/meter-event-stream")]
pub async fn create_meter_event_stream_batch(
    state: web::Data<V2AppState>,
    params: web::Json<CreateMeterEventStreamParams>,
) -> Result<HttpResponse, V2ApiError> {
    let resp = state
        .stripe
        .v2()
        .billing()
        .meter_event_stream()
        .create(&params)
        .await?;
    Ok(HttpResponse::Accepted().json(resp))
}

/// `POST /v2/billing/ingestion-engine/enqueue` — Enqueues a usage event into the buffered
/// `MeterIngestionEngine` with client-side deduplication and flushes when ready.
#[post("/v2/billing/ingestion-engine/enqueue")]
pub async fn enqueue_meter_event(
    state: web::Data<V2AppState>,
    event: web::Json<MeterEventPayload>,
) -> Result<HttpResponse, V2ApiError> {
    let flush_result = state
        .ingestion_engine
        .push_event(event.into_inner())
        .await?;
    let pending = state.ingestion_engine.pending_count().await;
    Ok(HttpResponse::Accepted().json(serde_json::json!({
        "pending_events": pending,
        "auto_flushed": flush_result,
    })))
}

/// `POST /v2/billing/ingestion-engine/flush` — Explicitly flushes all buffered events in
/// `MeterIngestionEngine`.
#[post("/v2/billing/ingestion-engine/flush")]
pub async fn flush_ingestion_engine(
    state: web::Data<V2AppState>,
) -> Result<HttpResponse, V2ApiError> {
    let result = state.ingestion_engine.flush().await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "flushed": result,
    })))
}

/// `POST /v2/billing/meter-event-adjustments` — Cancels or adjusts a previously ingested meter
/// event via `stripe.v2().billing().meter_event_adjustments().create()`.
#[post("/v2/billing/meter-event-adjustments")]
pub async fn create_meter_event_adjustment(
    state: web::Data<V2AppState>,
    params: web::Json<CreateMeterEventAdjustmentParams>,
) -> Result<HttpResponse, V2ApiError> {
    let adj = state
        .stripe
        .v2()
        .billing()
        .meter_event_adjustments()
        .create(&params)
        .await?;
    Ok(HttpResponse::Created().json(adj))
}

/// `GET /v2/billing/meter-event-adjustments/{id}` — Retrieves a meter event adjustment via
/// `stripe.v2().billing().meter_event_adjustments().retrieve()`.
#[get("/v2/billing/meter-event-adjustments/{id}")]
pub async fn retrieve_meter_event_adjustment(
    state: web::Data<V2AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, V2ApiError> {
    let adj = state
        .stripe
        .v2()
        .billing()
        .meter_event_adjustments()
        .retrieve(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(adj))
}

/// `GET /v2/billing/meter-event-adjustments` — Lists meter event adjustments via
/// `stripe.v2().billing().meter_event_adjustments().list()`.
#[get("/v2/billing/meter-event-adjustments")]
pub async fn list_meter_event_adjustments(
    state: web::Data<V2AppState>,
    query: web::Query<ListMeterEventAdjustmentsParams>,
) -> Result<HttpResponse, V2ApiError> {
    let list = state
        .stripe
        .v2()
        .billing()
        .meter_event_adjustments()
        .list(&query)
        .await?;
    Ok(HttpResponse::Ok().json(list))
}

/// Mounts all Module 8 (Stripe API v2 Core & Billing) routes onto an Actix-Web [`web::ServiceConfig`].
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(retrieve_v2_event)
        .service(list_v2_events)
        .service(hydrate_v2_thin_event)
        .service(create_event_destination)
        .service(retrieve_event_destination)
        .service(update_event_destination)
        .service(delete_event_destination)
        .service(list_event_destinations)
        .service(enable_event_destination)
        .service(disable_event_destination)
        .service(ping_event_destination)
        .service(create_v2_account)
        .service(retrieve_v2_account)
        .service(update_v2_account)
        .service(delete_v2_account)
        .service(list_v2_accounts)
        .service(create_v2_account_token)
        .service(create_billing_meter)
        .service(retrieve_billing_meter)
        .service(update_billing_meter)
        .service(deactivate_billing_meter)
        .service(reactivate_billing_meter)
        .service(list_billing_meters)
        .service(create_meter_event_session)
        .service(create_synchronous_meter_event)
        .service(create_meter_event_stream_batch)
        .service(enqueue_meter_event)
        .service(flush_ingestion_engine)
        .service(create_meter_event_adjustment)
        .service(retrieve_meter_event_adjustment)
        .service(list_meter_event_adjustments);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Automatically resolves $STRIPE_SECRET_KEY from process env, .env.local, .env.dev, .env, or .env.example
    let stripe = StripeClient::new("$STRIPE_SECRET_KEY");
    let ingestion_engine = stripe.v2().billing().create_ingestion_engine();

    let state = web::Data::new(V2AppState {
        stripe,
        ingestion_engine,
    });

    println!("Starting Actix-Web Stripe API v2 Next-Gen Services example on http://127.0.0.1:8088");
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .configure(configure_routes)
    })
    .bind(("127.0.0.1", 8088))?
    .run()
    .await
}
