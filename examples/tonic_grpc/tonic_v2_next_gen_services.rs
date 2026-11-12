//! # Tonic gRPC Example 8: Stripe API v2 Core, Hydration & Billing Meter Ingestion (Module 8)
//!
//! Demonstrates how to integrate **Module 8 (Stripe API v2 Next-Gen Services)** of `some-stripe`
//! inside a **Tonic (`tonic`)** gRPC service.
//!
//! ## Services & gRPC Methods Covered
//! - `stripe.v2().core()`: `retrieve_v2_event`, `list_v2_events`, `hydrate_v2_thin_event`
//!   (300s freshness guard), `create_event_destination`, `retrieve_event_destination`,
//!   `update_event_destination`, `delete_event_destination`, `list_event_destinations`,
//!   `enable_event_destination`, `disable_event_destination`, `ping_event_destination`,
//!   `create_v2_account`, `retrieve_v2_account`, `update_v2_account`, `delete_v2_account`,
//!   `list_v2_accounts`, `create_v2_account_token` (`[REDACTED]` token formatting)
//! - `stripe.v2().billing()`: `create_billing_meter`, `retrieve_billing_meter`,
//!   `update_billing_meter`, `deactivate_billing_meter`, `reactivate_billing_meter`,
//!   `list_billing_meters`, `create_meter_event_session`, `create_synchronous_meter_event`,
//!   `create_meter_event_stream_batch`, `enqueue_meter_event`, `flush_ingestion_engine`,
//!   `create_meter_event_adjustment`, `retrieve_meter_event_adjustment`,
//!   `list_meter_event_adjustments`
//!
//! ## Run the Example
//! ```bash
//! cargo run --example tonic_v2_next_gen_services
//! ```

use prost::Message;
use serde::Serialize;
use some_stripe::{
    CreateBillingMeterParams, CreateEventDestinationParams, CreateMeterEventAdjustmentParams,
    CreateMeterEventParams, CreateMeterEventSessionParams, CreateMeterEventStreamParams,
    CreateV2AccountParams, CreateV2AccountTokenParams, HydrationError, ListBillingMetersParams,
    ListMeterEventAdjustmentsParams, ListV2AccountsParams, ListV2EventsParams, MeterEventPayload,
    MeterIngestionEngine, StripeClient, StripeError, UpdateBillingMeterParams,
    UpdateEventDestinationParams, UpdateV2AccountParams,
};
use tonic::{Request, Response, Status};

#[derive(Clone, PartialEq, Message)]
pub struct V2RpcRequest {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub json_params: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct V2RpcResponse {
    #[prost(string, tag = "1")]
    pub json_data: String,
}

#[must_use]
pub fn to_status(err: StripeError) -> Status {
    match err {
        StripeError::InvalidRequest(msg) => Status::invalid_argument(msg),
        other => Status::unavailable(other.to_string()),
    }
}

#[must_use]
pub fn map_hydration_err(err: HydrationError) -> Status {
    Status::invalid_argument(err.to_string())
}

fn parse_json<T: serde::de::DeserializeOwned>(raw: &str) -> Result<T, Status> {
    serde_json::from_str(raw).map_err(|e| Status::invalid_argument(e.to_string()))
}

fn parse_opt_json<T: serde::de::DeserializeOwned>(raw: &str) -> Result<Option<T>, Status> {
    if raw.trim().is_empty() {
        Ok(None)
    } else {
        parse_json(raw).map(Some)
    }
}

fn to_response<T: Serialize>(value: &T) -> Result<Response<V2RpcResponse>, Status> {
    let json_data =
        serde_json::to_string(value).map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(V2RpcResponse { json_data }))
}

/// Tonic gRPC service trait for Module 8 (Stripe API v2 Core & Billing).
#[tonic::async_trait]
pub trait V2NextGenGrpcService: Send + Sync + 'static {
    async fn retrieve_v2_event(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn list_v2_events(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn hydrate_v2_thin_event(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;

    async fn create_event_destination(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn retrieve_event_destination(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn update_event_destination(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn delete_event_destination(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn list_event_destinations(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn enable_event_destination(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn disable_event_destination(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn ping_event_destination(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;

    async fn create_v2_account(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn retrieve_v2_account(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn update_v2_account(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn delete_v2_account(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn list_v2_accounts(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn create_v2_account_token(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;

    async fn create_billing_meter(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn retrieve_billing_meter(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn update_billing_meter(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn deactivate_billing_meter(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn reactivate_billing_meter(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn list_billing_meters(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;

    async fn create_meter_event_session(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn create_synchronous_meter_event(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn create_meter_event_stream_batch(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn enqueue_meter_event(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn flush_ingestion_engine(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn create_meter_event_adjustment(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn retrieve_meter_event_adjustment(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
    async fn list_meter_event_adjustments(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status>;
}

/// Tonic gRPC server implementation for Module 8.
#[derive(Clone)]
pub struct V2NextGenGrpcServer {
    pub stripe: StripeClient,
    pub ingestion_engine: MeterIngestionEngine,
}

impl V2NextGenGrpcServer {
    #[must_use]
    pub fn from_env() -> Self {
        let stripe = StripeClient::new("$STRIPE_SECRET_KEY");
        let ingestion_engine = stripe.v2().billing().create_ingestion_engine();
        Self {
            stripe,
            ingestion_engine,
        }
    }
}

#[tonic::async_trait]
impl V2NextGenGrpcService for V2NextGenGrpcServer {
    async fn retrieve_v2_event(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let res = self.stripe.v2().core().events().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_v2_events(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let params: Option<ListV2EventsParams> = parse_opt_json(&req.into_inner().json_params)?;
        let res = self.stripe.v2().core().events().list_events(params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn hydrate_v2_thin_event(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let res = self
            .stripe
            .v2()
            .core()
            .hydrator()
            .hydrate_from_json(&req.into_inner().json_params)
            .await
            .map_err(map_hydration_err)?;
        to_response(&res)
    }

    async fn create_event_destination(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let params: CreateEventDestinationParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.v2().core().event_destinations().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_event_destination(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let res = self.stripe.v2().core().event_destinations().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn update_event_destination(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateEventDestinationParams = parse_json(&r.json_params)?;
        let res = self.stripe.v2().core().event_destinations().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn delete_event_destination(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let res = self.stripe.v2().core().event_destinations().delete(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_event_destinations(&self, _req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let res = self.stripe.v2().core().event_destinations().list().await.map_err(to_status)?;
        to_response(&res)
    }
    async fn enable_event_destination(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let res = self.stripe.v2().core().event_destinations().enable(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn disable_event_destination(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let res = self.stripe.v2().core().event_destinations().disable(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn ping_event_destination(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let res = self.stripe.v2().core().event_destinations().ping(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn create_v2_account(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let params: CreateV2AccountParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.v2().core().accounts().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_v2_account(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let res = self.stripe.v2().core().accounts().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn update_v2_account(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateV2AccountParams = parse_json(&r.json_params)?;
        let res = self.stripe.v2().core().accounts().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn delete_v2_account(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let res = self.stripe.v2().core().accounts().del(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_v2_accounts(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let params: Option<ListV2AccountsParams> = parse_opt_json(&req.into_inner().json_params)?;
        let res = self.stripe.v2().core().accounts().list_page(params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn create_v2_account_token(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let r = req.into_inner();
        let params: CreateV2AccountTokenParams = parse_json(&r.json_params)?;
        let token = self.stripe.v2().core().accounts().create_token(&r.id, &params).await.map_err(to_status)?;
        to_response(&serde_json::json!({
            "account_id": r.id,
            "expires_at": token.expires_at,
            "token_debug_redacted": format!("{token:?}"),
        }))
    }

    async fn create_billing_meter(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let params: CreateBillingMeterParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.v2().billing().meters().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_billing_meter(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let res = self.stripe.v2().billing().meters().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn update_billing_meter(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateBillingMeterParams = parse_json(&r.json_params)?;
        let res = self.stripe.v2().billing().meters().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn deactivate_billing_meter(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let res = self.stripe.v2().billing().meters().deactivate(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn reactivate_billing_meter(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let res = self.stripe.v2().billing().meters().reactivate(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_billing_meters(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let params: ListBillingMetersParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.v2().billing().meters().list(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn create_meter_event_session(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let params: Option<CreateMeterEventSessionParams> = parse_opt_json(&req.into_inner().json_params)?;
        let session = self.stripe.v2().billing().meter_event_session().create(params.as_ref()).await.map_err(to_status)?;
        to_response(&serde_json::json!({
            "id": session.id,
            "expires_at": session.expires_at,
            "token_redacted": format!("{:?}", session.authentication_token),
        }))
    }
    async fn create_synchronous_meter_event(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let params: CreateMeterEventParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.meter_events().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn create_meter_event_stream_batch(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let params: CreateMeterEventStreamParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.v2().billing().meter_event_stream().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn enqueue_meter_event(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let event: MeterEventPayload = parse_json(&req.into_inner().json_params)?;
        let flushed = self.ingestion_engine.push_event(event).await.map_err(to_status)?;
        let pending = self.ingestion_engine.pending_count().await;
        to_response(&serde_json::json!({
            "pending_events": pending,
            "auto_flushed": flushed,
        }))
    }
    async fn flush_ingestion_engine(&self, _req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let res = self.ingestion_engine.flush().await.map_err(to_status)?;
        to_response(&serde_json::json!({ "flushed": res }))
    }
    async fn create_meter_event_adjustment(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let params: CreateMeterEventAdjustmentParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.v2().billing().meter_event_adjustments().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_meter_event_adjustment(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let res = self.stripe.v2().billing().meter_event_adjustments().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_meter_event_adjustments(&self, req: Request<V2RpcRequest>) -> Result<Response<V2RpcResponse>, Status> {
        let params: ListMeterEventAdjustmentsParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.v2().billing().meter_event_adjustments().list(&params).await.map_err(to_status)?;
        to_response(&res)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = V2NextGenGrpcServer::from_env();
    println!(
        "Tonic gRPC Stripe API v2 Next-Gen Services ready (api_base: {})",
        server.stripe.config.api_base
    );
    Ok(())
}
