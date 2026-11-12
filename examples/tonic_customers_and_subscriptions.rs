//! # Tonic gRPC Example 4: Customers, Subscriptions, Items & Schedules (Module 5 Part A)
//!
//! Demonstrates how to integrate **Customers, Subscriptions, SubscriptionItems, and
//! SubscriptionSchedules** from `some-stripe` inside a **Tonic (`tonic`)** gRPC service.
//!
//! ## Services & gRPC Methods Covered
//! - `stripe.customers()`: `create_customer`, `retrieve_customer`, `update_customer`
//!   (`Nullable<String>`), `delete_customer` (GDPR Art. 17 erasure), `list_customers`,
//!   `search_customers`, `retrieve_customer_cash_balance`, `update_customer_cash_balance`
//! - `stripe.subscriptions()`: `create_subscription`, `retrieve_subscription`,
//!   `update_subscription`, `cancel_subscription`, `resume_subscription`,
//!   `pause_subscription_collection`, `resume_subscription_collection`,
//!   `preview_subscription_proration`, `search_subscriptions`, `list_subscriptions`
//! - `stripe.subscriptions().items()`: `create_subscription_item`, `retrieve_subscription_item`,
//!   `update_subscription_item`, `delete_subscription_item`, `list_subscription_items`,
//!   `create_usage_record`
//! - `stripe.subscription_schedules()`: `create_subscription_schedule`,
//!   `retrieve_subscription_schedule`, `update_subscription_schedule`,
//!   `release_subscription_schedule`, `cancel_subscription_schedule`,
//!   `list_subscription_schedules`
//!
//! ## Run the Example
//! ```bash
//! cargo run --example tonic_customers_and_subscriptions
//! ```

use prost::Message;
use serde::Serialize;
use some_stripe::{
    CancelSubscriptionParams, CancelSubscriptionScheduleParams, CreateCustomerParams,
    CreateSubscriptionItemParams, CreateSubscriptionParams, CreateSubscriptionScheduleParams,
    CreateUsageRecordParams, DeleteSubscriptionItemParams, ListCustomersParams,
    ListSubscriptionItemsParams, ListSubscriptionSchedulesParams, ListSubscriptionsParams,
    PauseCollectionParams, PreviewProrationParams, ReleaseSubscriptionScheduleParams,
    ResumeSubscriptionParams, SearchCustomersParams, SearchSubscriptionsParams, StripeClient,
    StripeError, UpdateCustomerCashBalanceParams, UpdateCustomerParams,
    UpdateSubscriptionItemParams, UpdateSubscriptionParams, UpdateSubscriptionScheduleParams,
};
use tonic::{Request, Response, Status};

#[derive(Clone, PartialEq, Message)]
pub struct BillingRpcRequest {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub json_params: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct BillingRpcResponse {
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

fn parse_json<T: serde::de::DeserializeOwned>(raw: &str) -> Result<T, Status> {
    serde_json::from_str(raw).map_err(|e| Status::invalid_argument(e.to_string()))
}

fn to_response<T: Serialize>(value: &T) -> Result<Response<BillingRpcResponse>, Status> {
    let json_data =
        serde_json::to_string(value).map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(BillingRpcResponse { json_data }))
}

/// Tonic gRPC service trait for Customers, Subscriptions, SubscriptionItems & SubscriptionSchedules.
#[tonic::async_trait]
pub trait CustomersSubscriptionsGrpcService: Send + Sync + 'static {
    async fn create_customer(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn retrieve_customer(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn update_customer(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn delete_customer(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn list_customers(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn search_customers(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn retrieve_customer_cash_balance(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn update_customer_cash_balance(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;

    async fn create_subscription(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn retrieve_subscription(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn update_subscription(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn cancel_subscription(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn resume_subscription(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn pause_subscription_collection(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn resume_subscription_collection(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn preview_subscription_proration(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn search_subscriptions(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn list_subscriptions(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;

    async fn create_subscription_item(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn retrieve_subscription_item(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn update_subscription_item(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn delete_subscription_item(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn list_subscription_items(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn create_usage_record(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;

    async fn create_subscription_schedule(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn retrieve_subscription_schedule(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn update_subscription_schedule(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn release_subscription_schedule(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn cancel_subscription_schedule(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
    async fn list_subscription_schedules(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status>;
}

/// Tonic gRPC server implementation for Customers & Subscriptions.
#[derive(Clone, Debug)]
pub struct CustomersSubscriptionsGrpcServer {
    pub stripe: StripeClient,
}

impl CustomersSubscriptionsGrpcServer {
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            stripe: StripeClient::new("$STRIPE_SECRET_KEY"),
        }
    }
}

#[tonic::async_trait]
impl CustomersSubscriptionsGrpcService for CustomersSubscriptionsGrpcServer {
    async fn create_customer(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let params: CreateCustomerParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.customers().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_customer(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let res = self.stripe.customers().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn update_customer(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateCustomerParams = parse_json(&r.json_params)?;
        let res = self.stripe.customers().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn delete_customer(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let res = self.stripe.customers().delete(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn list_customers(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let params: ListCustomersParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.customers().list(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn search_customers(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let params: SearchCustomersParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.customers().search(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_customer_cash_balance(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let res = self.stripe.customers().retrieve_cash_balance(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn update_customer_cash_balance(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateCustomerCashBalanceParams = parse_json(&r.json_params)?;
        let res = self.stripe.customers().update_cash_balance(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn create_subscription(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let params: CreateSubscriptionParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.subscriptions().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_subscription(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let res = self.stripe.subscriptions().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn update_subscription(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateSubscriptionParams = parse_json(&r.json_params)?;
        let res = self.stripe.subscriptions().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn cancel_subscription(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: CancelSubscriptionParams = parse_json(&r.json_params)?;
        let res = self.stripe.subscriptions().cancel(&r.id, Some(&params)).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn resume_subscription(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: ResumeSubscriptionParams = parse_json(&r.json_params)?;
        let res = self.stripe.subscriptions().resume(&r.id, Some(&params)).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn pause_subscription_collection(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: PauseCollectionParams = parse_json(&r.json_params)?;
        let res = self.stripe.subscriptions().pause_collection(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn resume_subscription_collection(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let res = self.stripe.subscriptions().resume_collection(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn preview_subscription_proration(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: PreviewProrationParams = parse_json(&r.json_params)?;
        let res = self.stripe.subscriptions().preview_proration(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn search_subscriptions(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let params: SearchSubscriptionsParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.subscriptions().search_with_params(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn list_subscriptions(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let params: ListSubscriptionsParams = parse_json(&req.into_inner().json_params)?;
        let mut paginator = self.stripe.subscriptions().list(&params);
        let mut items = Vec::new();
        while let Some(item) = paginator.next_item().await.map_err(to_status)? {
            items.push(item);
            if items.len() >= 25 {
                break;
            }
        }
        to_response(&items)
    }

    async fn create_subscription_item(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let params: CreateSubscriptionItemParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.subscriptions().items().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_subscription_item(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let res = self.stripe.subscriptions().items().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn update_subscription_item(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateSubscriptionItemParams = parse_json(&r.json_params)?;
        let res = self.stripe.subscriptions().items().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn delete_subscription_item(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<DeleteSubscriptionItemParams> = if r.json_params.trim().is_empty() {
            None
        } else {
            Some(parse_json(&r.json_params)?)
        };
        let res = self.stripe.subscriptions().items().del(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn list_subscription_items(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let params: ListSubscriptionItemsParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.subscriptions().items().list(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn create_usage_record(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: CreateUsageRecordParams = parse_json(&r.json_params)?;
        let res = self.stripe.subscriptions().items().create_usage_record(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn create_subscription_schedule(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let params: CreateSubscriptionScheduleParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.subscription_schedules().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_subscription_schedule(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let res = self.stripe.subscription_schedules().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn update_subscription_schedule(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateSubscriptionScheduleParams = parse_json(&r.json_params)?;
        let res = self.stripe.subscription_schedules().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn release_subscription_schedule(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: ReleaseSubscriptionScheduleParams = parse_json(&r.json_params)?;
        let res = self.stripe.subscription_schedules().release(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn cancel_subscription_schedule(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: CancelSubscriptionScheduleParams = parse_json(&r.json_params)?;
        let res = self.stripe.subscription_schedules().cancel(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn list_subscription_schedules(&self, req: Request<BillingRpcRequest>) -> Result<Response<BillingRpcResponse>, Status> {
        let params: ListSubscriptionSchedulesParams = parse_json(&req.into_inner().json_params)?;
        let mut paginator = self.stripe.subscription_schedules().list(&params);
        let mut items = Vec::new();
        while let Some(item) = paginator.next_item().await.map_err(to_status)? {
            items.push(item);
            if items.len() >= 25 {
                break;
            }
        }
        to_response(&items)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = CustomersSubscriptionsGrpcServer::from_env();
    println!(
        "Tonic gRPC Customers & Subscriptions service ready (api_base: {})",
        server.stripe.config.api_base
    );
    Ok(())
}
