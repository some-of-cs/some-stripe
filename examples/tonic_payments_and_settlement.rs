//! # Tonic gRPC Example 2: Payments, Refunds, Disputes & Balance Settlement (Module 3)
//!
//! Demonstrates how to integrate **Module 3 (Payments & Settlement)** of `some-stripe`
//! inside a **Tonic (`tonic`)** gRPC service, including server-streaming pagination.
//!
//! ## Services & gRPC Methods Covered
//! - `stripe.payment_intents()`: `create_payment_intent`, `retrieve_payment_intent`,
//!   `confirm_payment_intent` (with PSD2 SCA 3DS2 redirect detection),
//!   `capture_payment_intent`, and `cancel_payment_intent`
//! - `stripe.refunds()`: `create_refund`, `retrieve_refund`, `update_refund`, `cancel_refund`
//! - `stripe.disputes()`: `retrieve_dispute`, `update_dispute`, `close_dispute`
//! - `stripe.balance()` & `stripe.balance_transactions()`: `retrieve_balance`,
//!   `retrieve_balance_transaction`, and `list_balance_transactions`
//!
//! ## Run the Example
//! ```bash
//! cargo run --example tonic_payments_and_settlement
//! ```

use prost::Message;
use serde::{Deserialize, Serialize};
use some_stripe::{
    CancelPaymentIntentParams, CancelRefundParams, CapturePaymentIntentParams,
    ConfirmPaymentIntentParams, CreatePaymentIntentParams, CreateRefundParams, DisputeEvidence,
    StripeClient, StripeError, UpdateDisputeParams, UpdateRefundParams,
};
use tonic::{Request, Response, Status};

/// Protobuf message carrying a single resource ID (`pi_...`, `re_...`, `dp_...`, `txn_...`).
#[derive(Clone, PartialEq, Message)]
pub struct ResourceIdRequest {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(uint32, optional, tag = "2")]
    pub limit: Option<u32>,
}

/// Protobuf message carrying a resource ID and JSON-encoded request parameters.
#[derive(Clone, PartialEq, Message)]
pub struct JsonPayloadRequest {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub json_params: String,
}

/// Protobuf message returning JSON-encoded Stripe domain models.
#[derive(Clone, PartialEq, Message)]
pub struct JsonPayloadResponse {
    #[prost(string, tag = "1")]
    pub json_data: String,
    #[prost(bool, tag = "2")]
    pub requires_3ds2_redirect: bool,
}

/// Typed evidence payload for `update_dispute`.
#[derive(Debug, Deserialize, Serialize)]
pub struct SubmitDisputeEvidenceInput {
    pub customer_name: Option<String>,
    pub product_description: Option<String>,
    pub uncategorized_text: Option<String>,
    pub submit: Option<bool>,
}

/// Maps [`StripeError`] into [`tonic::Status`].
#[must_use]
pub fn to_status(err: StripeError) -> Status {
    match err {
        StripeError::InvalidRequest(msg) => Status::invalid_argument(msg),
        other => Status::unavailable(other.to_string()),
    }
}

/// Deserializes a JSON parameter string into `T` or returns `Status::invalid_argument`.
pub fn parse_json<T: serde::de::DeserializeOwned>(raw: &str) -> Result<T, Status> {
    serde_json::from_str(raw).map_err(|e| Status::invalid_argument(e.to_string()))
}

/// Serializes `value` into a [`JsonPayloadResponse`].
pub fn to_json_response<T: Serialize>(
    value: &T,
    requires_3ds2_redirect: bool,
) -> Result<Response<JsonPayloadResponse>, Status> {
    let json_data =
        serde_json::to_string(value).map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(JsonPayloadResponse {
        json_data,
        requires_3ds2_redirect,
    }))
}

/// Tonic gRPC service trait for Module 3 (Payments, Refunds, Disputes & Balance Settlement).
#[tonic::async_trait]
pub trait PaymentsSettlementGrpcService: Send + Sync + 'static {
    async fn create_payment_intent(
        &self,
        request: Request<JsonPayloadRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status>;

    async fn retrieve_payment_intent(
        &self,
        request: Request<ResourceIdRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status>;

    async fn confirm_payment_intent(
        &self,
        request: Request<JsonPayloadRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status>;

    async fn capture_payment_intent(
        &self,
        request: Request<JsonPayloadRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status>;

    async fn cancel_payment_intent(
        &self,
        request: Request<JsonPayloadRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status>;

    async fn create_refund(
        &self,
        request: Request<JsonPayloadRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status>;

    async fn retrieve_refund(
        &self,
        request: Request<ResourceIdRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status>;

    async fn update_refund(
        &self,
        request: Request<JsonPayloadRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status>;

    async fn cancel_refund(
        &self,
        request: Request<JsonPayloadRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status>;

    async fn retrieve_dispute(
        &self,
        request: Request<ResourceIdRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status>;

    async fn update_dispute(
        &self,
        request: Request<JsonPayloadRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status>;

    async fn close_dispute(
        &self,
        request: Request<ResourceIdRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status>;

    async fn retrieve_balance(
        &self,
        request: Request<ResourceIdRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status>;

    async fn retrieve_balance_transaction(
        &self,
        request: Request<ResourceIdRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status>;

    async fn list_balance_transactions(
        &self,
        request: Request<ResourceIdRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status>;
}

/// Tonic gRPC server implementation for Module 3.
#[derive(Clone, Debug)]
pub struct PaymentsSettlementGrpcServer {
    pub stripe: StripeClient,
}

impl PaymentsSettlementGrpcServer {
    /// Initializes the gRPC server using native `"$STRIPE_SECRET_KEY"` `.env*` resolution.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            stripe: StripeClient::new("$STRIPE_SECRET_KEY"),
        }
    }
}

#[tonic::async_trait]
impl PaymentsSettlementGrpcService for PaymentsSettlementGrpcServer {
    async fn create_payment_intent(
        &self,
        request: Request<JsonPayloadRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status> {
        let params: CreatePaymentIntentParams = parse_json(&request.into_inner().json_params)?;
        let intent = self
            .stripe
            .payment_intents()
            .create(&params)
            .await
            .map_err(to_status)?;
        to_json_response(&intent, false)
    }

    async fn retrieve_payment_intent(
        &self,
        request: Request<ResourceIdRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status> {
        let id = request.into_inner().id;
        let intent = self
            .stripe
            .payment_intents()
            .retrieve(&id)
            .await
            .map_err(to_status)?;
        to_json_response(&intent, false)
    }

    async fn confirm_payment_intent(
        &self,
        request: Request<JsonPayloadRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status> {
        let req = request.into_inner();
        let params: ConfirmPaymentIntentParams = parse_json(&req.json_params)?;
        let intent = self
            .stripe
            .payment_intents()
            .confirm(&req.id, Some(&params))
            .await
            .map_err(to_status)?;
        let requires_3ds2 = intent.status == "requires_action";
        to_json_response(&intent, requires_3ds2)
    }

    async fn capture_payment_intent(
        &self,
        request: Request<JsonPayloadRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status> {
        let req = request.into_inner();
        let params: CapturePaymentIntentParams = parse_json(&req.json_params)?;
        let intent = self
            .stripe
            .payment_intents()
            .capture(&req.id, Some(&params))
            .await
            .map_err(to_status)?;
        to_json_response(&intent, false)
    }

    async fn cancel_payment_intent(
        &self,
        request: Request<JsonPayloadRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status> {
        let req = request.into_inner();
        let params: CancelPaymentIntentParams = parse_json(&req.json_params)?;
        let intent = self
            .stripe
            .payment_intents()
            .cancel(&req.id, Some(&params))
            .await
            .map_err(to_status)?;
        to_json_response(&intent, false)
    }

    async fn create_refund(
        &self,
        request: Request<JsonPayloadRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status> {
        let params: CreateRefundParams = parse_json(&request.into_inner().json_params)?;
        let refund = self.stripe.refunds().create(&params).await.map_err(to_status)?;
        to_json_response(&refund, false)
    }

    async fn retrieve_refund(
        &self,
        request: Request<ResourceIdRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status> {
        let id = request.into_inner().id;
        let refund = self.stripe.refunds().retrieve(&id).await.map_err(to_status)?;
        to_json_response(&refund, false)
    }

    async fn update_refund(
        &self,
        request: Request<JsonPayloadRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status> {
        let req = request.into_inner();
        let params: UpdateRefundParams = parse_json(&req.json_params)?;
        let refund = self
            .stripe
            .refunds()
            .update(&req.id, &params)
            .await
            .map_err(to_status)?;
        to_json_response(&refund, false)
    }

    async fn cancel_refund(
        &self,
        request: Request<JsonPayloadRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status> {
        let req = request.into_inner();
        let params: CancelRefundParams = parse_json(&req.json_params)?;
        let refund = self
            .stripe
            .refunds()
            .cancel(&req.id, Some(&params))
            .await
            .map_err(to_status)?;
        to_json_response(&refund, false)
    }

    async fn retrieve_dispute(
        &self,
        request: Request<ResourceIdRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status> {
        let id = request.into_inner().id;
        let dispute = self.stripe.disputes().retrieve(&id).await.map_err(to_status)?;
        to_json_response(&dispute, false)
    }

    async fn update_dispute(
        &self,
        request: Request<JsonPayloadRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status> {
        let req = request.into_inner();
        let input: SubmitDisputeEvidenceInput = parse_json(&req.json_params)?;
        let params = UpdateDisputeParams {
            evidence: Some(DisputeEvidence {
                customer_name: input.customer_name,
                product_description: input.product_description,
                uncategorized_text: input.uncategorized_text,
                ..Default::default()
            }),
            metadata: None,
            submit: input.submit,
            idempotency_key: None,
        };
        let dispute = self
            .stripe
            .disputes()
            .update(&req.id, &params)
            .await
            .map_err(to_status)?;
        to_json_response(&dispute, false)
    }

    async fn close_dispute(
        &self,
        request: Request<ResourceIdRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status> {
        let id = request.into_inner().id;
        let dispute = self.stripe.disputes().close(&id).await.map_err(to_status)?;
        to_json_response(&dispute, false)
    }

    async fn retrieve_balance(
        &self,
        _request: Request<ResourceIdRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status> {
        let balance = self.stripe.balance().retrieve().await.map_err(to_status)?;
        to_json_response(&balance, false)
    }

    async fn retrieve_balance_transaction(
        &self,
        request: Request<ResourceIdRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status> {
        let id = request.into_inner().id;
        let tx = self
            .stripe
            .balance_transactions()
            .retrieve(&id)
            .await
            .map_err(to_status)?;
        to_json_response(&tx, false)
    }

    async fn list_balance_transactions(
        &self,
        request: Request<ResourceIdRequest>,
    ) -> Result<Response<JsonPayloadResponse>, Status> {
        let limit = request.into_inner().limit.unwrap_or(10);
        let mut paginator = self.stripe.balance_transactions().list(Some(limit));
        let mut items = Vec::new();
        while let Some(item) = paginator.next_item().await.map_err(to_status)? {
            items.push(item);
            if items.len() >= limit as usize {
                break;
            }
        }
        to_json_response(&items, false)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = PaymentsSettlementGrpcServer::from_env();
    println!(
        "Tonic gRPC Payments & Settlement service ready (api_base: {})",
        server.stripe.config.api_base
    );
    Ok(())
}
