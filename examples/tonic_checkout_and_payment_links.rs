//! # Tonic gRPC Example 3: Hosted Checkout Sessions & Payment Links (Module 4)
//!
//! Demonstrates how to integrate **Module 4 (Hosted Checkout & Payment Links)** of `some-stripe`
//! inside a **Tonic (`tonic`)** gRPC service.
//!
//! ## Services & gRPC Methods Covered
//! - `stripe.checkout().sessions()`: `create_checkout_session`, `retrieve_checkout_session`
//!   (with `is_fulfillment_ready`), `expire_checkout_session`, `list_checkout_session_line_items`,
//!   and `validate_redirect_url` open-redirect domain allowlist guard
//! - `stripe.payment_links()`: `create_payment_link`, `retrieve_payment_link`,
//!   `update_payment_link`, and `list_payment_link_line_items`
//! - Statutory compliance: `validate_omnibus_promotion` (`validate_omnibus_price_transparency` &
//!   `validate_tax_behavior_transparency`)
//!
//! ## Run the Example
//! ```bash
//! cargo run --example tonic_checkout_and_payment_links
//! ```

use prost::Message;
use serde::{Deserialize, Serialize};
use some_stripe::{
    CreateCheckoutSessionParams, CreatePaymentLinkParams, StripeClient, StripeError, TaxBehavior,
    UpdatePaymentLinkParams, is_fulfillment_ready, validate_omnibus_price_transparency,
    validate_redirect_url, validate_tax_behavior_transparency,
};
use tonic::{Request, Response, Status};

#[derive(Clone, PartialEq, Message)]
pub struct CheckoutIdRequest {
    #[prost(string, tag = "1")]
    pub id: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct CheckoutJsonRequest {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub json_params: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct CheckoutJsonResponse {
    #[prost(string, tag = "1")]
    pub json_data: String,
    #[prost(bool, tag = "2")]
    pub is_fulfillment_ready: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OmnibusPromotionInput {
    pub regular_price: i64,
    pub discounted_price: i64,
    pub prior_30d_lowest_price: Option<i64>,
    pub tax_behavior: TaxBehavior,
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

fn to_response<T: Serialize>(
    value: &T,
    ready: bool,
) -> Result<Response<CheckoutJsonResponse>, Status> {
    let json_data =
        serde_json::to_string(value).map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(CheckoutJsonResponse {
        json_data,
        is_fulfillment_ready: ready,
    }))
}

/// Tonic gRPC service trait for Module 4 (Hosted Checkout & Payment Links).
#[tonic::async_trait]
pub trait CheckoutLinksGrpcService: Send + Sync + 'static {
    async fn create_checkout_session(
        &self,
        request: Request<CheckoutJsonRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status>;

    async fn retrieve_checkout_session(
        &self,
        request: Request<CheckoutIdRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status>;

    async fn expire_checkout_session(
        &self,
        request: Request<CheckoutIdRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status>;

    async fn list_checkout_session_line_items(
        &self,
        request: Request<CheckoutIdRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status>;

    async fn create_payment_link(
        &self,
        request: Request<CheckoutJsonRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status>;

    async fn retrieve_payment_link(
        &self,
        request: Request<CheckoutIdRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status>;

    async fn update_payment_link(
        &self,
        request: Request<CheckoutJsonRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status>;

    async fn list_payment_link_line_items(
        &self,
        request: Request<CheckoutIdRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status>;

    async fn validate_omnibus_promotion(
        &self,
        request: Request<CheckoutJsonRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status>;
}

/// Tonic gRPC server implementation for Module 4.
#[derive(Clone, Debug)]
pub struct CheckoutLinksGrpcServer {
    pub stripe: StripeClient,
}

impl CheckoutLinksGrpcServer {
    /// Initializes the gRPC server using native `"$STRIPE_SECRET_KEY"` `.env*` resolution.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            stripe: StripeClient::new("$STRIPE_SECRET_KEY"),
        }
    }
}

#[tonic::async_trait]
impl CheckoutLinksGrpcService for CheckoutLinksGrpcServer {
    async fn create_checkout_session(
        &self,
        request: Request<CheckoutJsonRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status> {
        let params: CreateCheckoutSessionParams = parse_json(&request.into_inner().json_params)?;
        let allowed_domains = ["example.com", "checkout.example.com"];
        validate_redirect_url(&params.success_url, &allowed_domains)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let session = self
            .stripe
            .checkout()
            .sessions()
            .create(&params)
            .await
            .map_err(to_status)?;
        let ready = is_fulfillment_ready(&session);
        to_response(&session, ready)
    }

    async fn retrieve_checkout_session(
        &self,
        request: Request<CheckoutIdRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status> {
        let id = request.into_inner().id;
        let session = self
            .stripe
            .checkout()
            .sessions()
            .retrieve(&id)
            .await
            .map_err(to_status)?;
        let ready = is_fulfillment_ready(&session);
        to_response(&session, ready)
    }

    async fn expire_checkout_session(
        &self,
        request: Request<CheckoutIdRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status> {
        let id = request.into_inner().id;
        let session = self
            .stripe
            .checkout()
            .sessions()
            .expire(&id)
            .await
            .map_err(to_status)?;
        to_response(&session, false)
    }

    async fn list_checkout_session_line_items(
        &self,
        request: Request<CheckoutIdRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status> {
        let id = request.into_inner().id;
        let mut paginator = self.stripe.checkout().sessions().list_line_items_default(&id);
        let mut items = Vec::new();
        while let Some(item) = paginator.next_item().await.map_err(to_status)? {
            items.push(item);
            if items.len() >= 25 {
                break;
            }
        }
        to_response(&items, false)
    }

    async fn create_payment_link(
        &self,
        request: Request<CheckoutJsonRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status> {
        let params: CreatePaymentLinkParams = parse_json(&request.into_inner().json_params)?;
        let link = self
            .stripe
            .payment_links()
            .create(&params)
            .await
            .map_err(to_status)?;
        to_response(&link, false)
    }

    async fn retrieve_payment_link(
        &self,
        request: Request<CheckoutIdRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status> {
        let id = request.into_inner().id;
        let link = self
            .stripe
            .payment_links()
            .retrieve(&id)
            .await
            .map_err(to_status)?;
        to_response(&link, false)
    }

    async fn update_payment_link(
        &self,
        request: Request<CheckoutJsonRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status> {
        let req = request.into_inner();
        let params: UpdatePaymentLinkParams = parse_json(&req.json_params)?;
        let link = self
            .stripe
            .payment_links()
            .update(&req.id, &params)
            .await
            .map_err(to_status)?;
        to_response(&link, false)
    }

    async fn list_payment_link_line_items(
        &self,
        request: Request<CheckoutIdRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status> {
        let id = request.into_inner().id;
        let mut paginator = self.stripe.payment_links().list_line_items_default(&id);
        let mut items = Vec::new();
        while let Some(item) = paginator.next_item().await.map_err(to_status)? {
            items.push(item);
            if items.len() >= 25 {
                break;
            }
        }
        to_response(&items, false)
    }

    async fn validate_omnibus_promotion(
        &self,
        request: Request<CheckoutJsonRequest>,
    ) -> Result<Response<CheckoutJsonResponse>, Status> {
        let input: OmnibusPromotionInput = parse_json(&request.into_inner().json_params)?;
        validate_omnibus_price_transparency(
            input.regular_price,
            input.discounted_price,
            input.prior_30d_lowest_price,
        )
        .map_err(to_status)?;
        validate_tax_behavior_transparency(input.tax_behavior).map_err(to_status)?;

        to_response(
            &serde_json::json!({
                "compliant": true,
                "regular_price": input.regular_price,
                "discounted_price": input.discounted_price,
            }),
            true,
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = CheckoutLinksGrpcServer::from_env();
    println!(
        "Tonic gRPC Checkout & Payment Links service ready (api_base: {})",
        server.stripe.config.api_base
    );
    Ok(())
}
