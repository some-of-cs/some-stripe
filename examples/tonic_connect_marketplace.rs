//! # Tonic gRPC Example 6: Connect & Multi-Party Marketplaces (Module 6)
//!
//! Demonstrates how to integrate **Module 6 (Connect & Marketplaces)** of `some-stripe`
//! inside a **Tonic (`tonic`)** gRPC service.
//!
//! ## Services & gRPC Methods Covered
//! - `stripe.accounts()`: `create_account`, `retrieve_current_account`, `retrieve_account_by_id`,
//!   `update_account`, `delete_account`, `reject_account`, `list_accounts`
//! - `stripe.account_links()` & `stripe.account_sessions()`: `create_account_link`,
//!   `create_account_session`
//! - `stripe.persons()`: `create_person`, `retrieve_person`, `update_person`, `delete_person`,
//!   `list_persons`
//! - `stripe.transfers()`: `create_transfer`, `retrieve_transfer`, `update_transfer`,
//!   `reverse_transfer`, `list_transfers`, `list_transfer_reversals`
//! - `stripe.application_fees()`: `retrieve_application_fee`, `list_application_fees`,
//!   `create_application_fee_refund`, `retrieve_application_fee_refund`,
//!   `list_application_fee_refunds`
//! - `stripe.payouts()`: `create_payout`, `retrieve_payout`, `update_payout`, `cancel_payout`,
//!   `reverse_payout`, `list_payouts`
//!
//! ## Run the Example
//! ```bash
//! cargo run --example tonic_connect_marketplace
//! ```

use prost::Message;
use serde::Serialize;
use some_stripe::{
    CreateAccountLinkParams, CreateAccountParams, CreateAccountSessionParams,
    CreateFeeRefundParams, CreatePayoutParams, CreatePersonParams, CreateTransferParams,
    CreateTransferReversalParams, ListAccountsParams, ListApplicationFeesParams,
    ListFeeRefundsParams, ListPayoutsParams, ListPersonsParams, ListTransferReversalsParams,
    ListTransfersParams, RejectAccountParams, StripeClient, StripeError, UpdateAccountParams,
    UpdatePayoutParams, UpdatePersonParams, UpdateTransferParams,
};
use tonic::{Request, Response, Status};

#[derive(Clone, PartialEq, Message)]
pub struct ConnectRpcRequest {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub sub_id: String,
    #[prost(string, tag = "3")]
    pub json_params: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ConnectRpcResponse {
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

fn parse_opt_json<T: serde::de::DeserializeOwned>(raw: &str) -> Result<Option<T>, Status> {
    if raw.trim().is_empty() {
        Ok(None)
    } else {
        parse_json(raw).map(Some)
    }
}

fn to_response<T: Serialize>(value: &T) -> Result<Response<ConnectRpcResponse>, Status> {
    let json_data =
        serde_json::to_string(value).map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(ConnectRpcResponse { json_data }))
}

/// Tonic gRPC service trait for Module 6 (Connect & Multi-Party Marketplaces).
#[tonic::async_trait]
pub trait ConnectMarketplaceGrpcService: Send + Sync + 'static {
    async fn create_account(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn retrieve_current_account(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn retrieve_account_by_id(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn update_account(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn delete_account(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn reject_account(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn list_accounts(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn create_account_link(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn create_account_session(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;

    async fn create_person(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn retrieve_person(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn update_person(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn delete_person(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn list_persons(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;

    async fn create_transfer(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn retrieve_transfer(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn update_transfer(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn reverse_transfer(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn list_transfers(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn list_transfer_reversals(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;

    async fn retrieve_application_fee(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn list_application_fees(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn create_application_fee_refund(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn retrieve_application_fee_refund(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn list_application_fee_refunds(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;

    async fn create_payout(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn retrieve_payout(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn update_payout(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn cancel_payout(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn reverse_payout(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
    async fn list_payouts(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status>;
}

/// Tonic gRPC server implementation for Module 6.
#[derive(Clone, Debug)]
pub struct ConnectMarketplaceGrpcServer {
    pub stripe: StripeClient,
}

impl ConnectMarketplaceGrpcServer {
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            stripe: StripeClient::new("$STRIPE_SECRET_KEY"),
        }
    }
}

#[tonic::async_trait]
impl ConnectMarketplaceGrpcService for ConnectMarketplaceGrpcServer {
    async fn create_account(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let params: CreateAccountParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.accounts().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_current_account(&self, _req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let res = self.stripe.accounts().retrieve().await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_account_by_id(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let res = self.stripe.accounts().retrieve_by_id(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn update_account(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateAccountParams = parse_json(&r.json_params)?;
        let res = self.stripe.accounts().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn delete_account(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let res = self.stripe.accounts().delete(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn reject_account(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let r = req.into_inner();
        let params: RejectAccountParams = parse_json(&r.json_params)?;
        let res = self.stripe.accounts().reject(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn list_accounts(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let params: ListAccountsParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.accounts().list(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn create_account_link(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let params: CreateAccountLinkParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.account_links().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn create_account_session(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let params: CreateAccountSessionParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.account_sessions().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn create_person(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let r = req.into_inner();
        let params: CreatePersonParams = parse_json(&r.json_params)?;
        let res = self.stripe.persons().create(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_person(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let r = req.into_inner();
        let res = self.stripe.persons().retrieve(&r.id, &r.sub_id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn update_person(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdatePersonParams = parse_json(&r.json_params)?;
        let res = self.stripe.persons().update(&r.id, &r.sub_id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn delete_person(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let r = req.into_inner();
        let res = self.stripe.persons().delete(&r.id, &r.sub_id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn list_persons(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let r = req.into_inner();
        let params: ListPersonsParams = parse_json(&r.json_params)?;
        let res = self.stripe.persons().list(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn create_transfer(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let params: CreateTransferParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.transfers().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_transfer(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let res = self.stripe.transfers().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn update_transfer(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateTransferParams = parse_json(&r.json_params)?;
        let res = self.stripe.transfers().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn reverse_transfer(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<CreateTransferReversalParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.transfers().reverse(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn list_transfers(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let params: Option<ListTransfersParams> = parse_opt_json(&req.into_inner().json_params)?;
        let res = self.stripe.transfers().list(params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn list_transfer_reversals(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<ListTransferReversalsParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.transfers().list_reversals(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_application_fee(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let res = self.stripe.application_fees().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn list_application_fees(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let params: Option<ListApplicationFeesParams> = parse_opt_json(&req.into_inner().json_params)?;
        let res = self.stripe.application_fees().list(params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn create_application_fee_refund(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<CreateFeeRefundParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.application_fees().create_refund(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_application_fee_refund(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let r = req.into_inner();
        let res = self.stripe.application_fees().retrieve_refund(&r.id, &r.sub_id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn list_application_fee_refunds(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<ListFeeRefundsParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.application_fees().list_refunds(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn create_payout(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let params: CreatePayoutParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.payouts().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_payout(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let res = self.stripe.payouts().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn update_payout(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdatePayoutParams = parse_json(&r.json_params)?;
        let res = self.stripe.payouts().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn cancel_payout(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let res = self.stripe.payouts().cancel(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn reverse_payout(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let res = self.stripe.payouts().reverse(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn list_payouts(&self, req: Request<ConnectRpcRequest>) -> Result<Response<ConnectRpcResponse>, Status> {
        let params: Option<ListPayoutsParams> = parse_opt_json(&req.into_inner().json_params)?;
        let res = self.stripe.payouts().list(params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = ConnectMarketplaceGrpcServer::from_env();
    println!(
        "Tonic gRPC Connect Marketplace service ready (api_base: {})",
        server.stripe.config.api_base
    );
    Ok(())
}
