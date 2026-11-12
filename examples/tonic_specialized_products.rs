//! # Tonic gRPC Example 7: Tax, Identity, Issuing, Treasury & Terminal (Module 7)
//!
//! Demonstrates how to integrate **Module 7 (Specialized Product Namespaces)** of `some-stripe`
//! inside a **Tonic (`tonic`)** gRPC service.
//!
//! ## Services & gRPC Methods Covered
//! - `stripe.tax()`: `create_tax_calculation`, `retrieve_tax_calculation`,
//!   `list_tax_calculation_line_items`, `create_tax_transaction_from_calculation`,
//!   `create_tax_transaction_reversal`, `retrieve_tax_transaction`,
//!   `list_tax_transaction_line_items`, `create_tax_registration`, `retrieve_tax_registration`,
//!   `update_tax_registration`, `list_tax_registrations`, `retrieve_tax_settings`,
//!   `update_tax_settings`
//! - `stripe.identity()`: `create_verification_session`, `retrieve_verification_session`,
//!   `update_verification_session`, `cancel_verification_session`,
//!   `redact_verification_session` (GDPR Art. 17 PII scrub), `retrieve_verification_report`,
//!   `list_verification_reports`
//! - `stripe.issuing()`: `create_issuing_cardholder`, `retrieve_issuing_cardholder`,
//!   `update_issuing_cardholder`, `list_issuing_cardholders`, `create_issuing_card`,
//!   `retrieve_issuing_card`, `update_issuing_card`, `list_issuing_cards`,
//!   `retrieve_issuing_authorization`, `update_issuing_authorization`,
//!   `approve_issuing_authorization`, `decline_issuing_authorization`,
//!   `list_issuing_authorizations`, `retrieve_issuing_transaction`,
//!   `update_issuing_transaction`, `list_issuing_transactions`
//! - `stripe.treasury()`: `create_financial_account`, `retrieve_financial_account`,
//!   `update_financial_account`, `list_financial_accounts`, `close_financial_account`,
//!   `create_outbound_transfer`, `retrieve_outbound_transfer`, `cancel_outbound_transfer`,
//!   `list_outbound_transfers`, `create_outbound_payment`, `retrieve_outbound_payment`,
//!   `cancel_outbound_payment`, `list_outbound_payments`, `retrieve_received_credit`,
//!   `list_received_credits`
//! - `stripe.terminal()`: `create_terminal_location`, `retrieve_terminal_location`,
//!   `list_terminal_locations`, `create_terminal_reader`, `retrieve_terminal_reader`,
//!   `list_terminal_readers`, `cancel_terminal_reader_action`, `create_terminal_connection_token`
//!
//! ## Run the Example
//! ```bash
//! cargo run --example tonic_specialized_products
//! ```

use prost::Message;
use serde::Serialize;
use some_stripe::{
    ApproveAuthorizationParams, CancelOutboundPaymentParams, CancelOutboundTransferParams,
    CloseFinancialAccountParams, CreateCardParams, CreateCardholderParams,
    CreateConnectionTokenParams, CreateFinancialAccountParams, CreateLocationParams,
    CreateOutboundPaymentParams, CreateOutboundTransferParams, CreateReaderParams,
    CreateTaxCalculationParams, CreateTaxRegistrationParams,
    CreateTaxTransactionFromCalculationParams, CreateTaxTransactionReversalParams,
    CreateVerificationSessionParams, DeclineAuthorizationParams, ListFinancialAccountsParams,
    ListIssuingAuthorizationsParams, ListIssuingCardholdersParams, ListIssuingCardsParams,
    ListIssuingTransactionsParams, ListOutboundPaymentsParams, ListOutboundTransfersParams,
    ListReceivedCreditsParams, ListTaxCalculationLineItemsParams, ListTaxRegistrationsParams,
    ListTerminalLocationsParams, ListTerminalReadersParams, StripeClient, StripeError,
    UpdateCardParams, UpdateCardholderParams, UpdateFinancialAccountParams,
    UpdateIssuingAuthorizationParams, UpdateIssuingTransactionParams, UpdateTaxRegistrationParams,
    UpdateTaxSettingsParams, UpdateVerificationSessionParams,
};
use tonic::{Request, Response, Status};

#[derive(Clone, PartialEq, Message)]
pub struct SpecializedRpcRequest {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub json_params: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct SpecializedRpcResponse {
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

fn to_response<T: Serialize>(value: &T) -> Result<Response<SpecializedRpcResponse>, Status> {
    let json_data =
        serde_json::to_string(value).map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(SpecializedRpcResponse { json_data }))
}

/// Tonic gRPC service trait for Module 7 (Tax, Identity, Issuing, Treasury & Terminal).
#[tonic::async_trait]
pub trait SpecializedProductsGrpcService: Send + Sync + 'static {
    async fn create_tax_calculation(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn retrieve_tax_calculation(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn list_tax_calculation_line_items(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn create_tax_transaction_from_calculation(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn create_tax_transaction_reversal(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn retrieve_tax_transaction(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn list_tax_transaction_line_items(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn create_tax_registration(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn retrieve_tax_registration(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn update_tax_registration(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn list_tax_registrations(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn retrieve_tax_settings(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn update_tax_settings(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;

    async fn create_verification_session(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn retrieve_verification_session(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn update_verification_session(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn cancel_verification_session(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn redact_verification_session(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn retrieve_verification_report(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn list_verification_reports(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;

    async fn create_issuing_cardholder(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn retrieve_issuing_cardholder(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn update_issuing_cardholder(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn list_issuing_cardholders(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn create_issuing_card(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn retrieve_issuing_card(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn update_issuing_card(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn list_issuing_cards(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn retrieve_issuing_authorization(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn update_issuing_authorization(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn approve_issuing_authorization(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn decline_issuing_authorization(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn list_issuing_authorizations(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn retrieve_issuing_transaction(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn update_issuing_transaction(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn list_issuing_transactions(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;

    async fn create_financial_account(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn retrieve_financial_account(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn update_financial_account(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn list_financial_accounts(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn close_financial_account(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn create_outbound_transfer(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn retrieve_outbound_transfer(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn cancel_outbound_transfer(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn list_outbound_transfers(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn create_outbound_payment(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn retrieve_outbound_payment(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn cancel_outbound_payment(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn list_outbound_payments(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn retrieve_received_credit(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn list_received_credits(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;

    async fn create_terminal_location(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn retrieve_terminal_location(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn list_terminal_locations(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn create_terminal_reader(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn retrieve_terminal_reader(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn list_terminal_readers(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn cancel_terminal_reader_action(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
    async fn create_terminal_connection_token(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status>;
}

/// Tonic gRPC server implementation for Module 7.
#[derive(Clone, Debug)]
pub struct SpecializedProductsGrpcServer {
    pub stripe: StripeClient,
}

impl SpecializedProductsGrpcServer {
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            stripe: StripeClient::new("$STRIPE_SECRET_KEY"),
        }
    }
}

#[tonic::async_trait]
impl SpecializedProductsGrpcService for SpecializedProductsGrpcServer {
    async fn create_tax_calculation(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: CreateTaxCalculationParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.tax().calculations().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_tax_calculation(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.tax().calculations().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_tax_calculation_line_items(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<ListTaxCalculationLineItemsParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.tax().calculations().list_line_items_page(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn create_tax_transaction_from_calculation(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: CreateTaxTransactionFromCalculationParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.tax().transactions().create_from_calculation(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn create_tax_transaction_reversal(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: CreateTaxTransactionReversalParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.tax().transactions().create_reversal(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_tax_transaction(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.tax().transactions().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_tax_transaction_line_items(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let mut paginator = self.stripe.tax().transactions().list_line_items_default(&req.into_inner().id);
        let mut items = Vec::new();
        while let Some(item) = paginator.next_item().await.map_err(to_status)? {
            items.push(item);
            if items.len() >= 25 { break; }
        }
        to_response(&items)
    }
    async fn create_tax_registration(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: CreateTaxRegistrationParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.tax().registrations().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_tax_registration(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.tax().registrations().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn update_tax_registration(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateTaxRegistrationParams = parse_json(&r.json_params)?;
        let res = self.stripe.tax().registrations().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_tax_registrations(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: Option<ListTaxRegistrationsParams> = parse_opt_json(&req.into_inner().json_params)?;
        let res = self.stripe.tax().registrations().list_page(params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_tax_settings(&self, _req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.tax().settings().retrieve().await.map_err(to_status)?;
        to_response(&res)
    }
    async fn update_tax_settings(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: UpdateTaxSettingsParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.tax().settings().update(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn create_verification_session(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: CreateVerificationSessionParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.identity().verification_sessions().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_verification_session(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.identity().verification_sessions().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn update_verification_session(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateVerificationSessionParams = parse_json(&r.json_params)?;
        let res = self.stripe.identity().verification_sessions().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn cancel_verification_session(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.identity().verification_sessions().cancel(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn redact_verification_session(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.identity().verification_sessions().redact(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_verification_report(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.identity().verification_reports().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_verification_reports(&self, _req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let mut paginator = self.stripe.identity().verification_reports().list_default();
        let mut items = Vec::new();
        while let Some(item) = paginator.next_item().await.map_err(to_status)? {
            items.push(item);
            if items.len() >= 25 { break; }
        }
        to_response(&items)
    }

    async fn create_issuing_cardholder(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: CreateCardholderParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.issuing().cardholders().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_issuing_cardholder(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.issuing().cardholders().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn update_issuing_cardholder(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateCardholderParams = parse_json(&r.json_params)?;
        let res = self.stripe.issuing().cardholders().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_issuing_cardholders(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: Option<ListIssuingCardholdersParams> = parse_opt_json(&req.into_inner().json_params)?;
        let res = self.stripe.issuing().cardholders().list(params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn create_issuing_card(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: CreateCardParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.issuing().cards().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_issuing_card(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.issuing().cards().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn update_issuing_card(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateCardParams = parse_json(&r.json_params)?;
        let res = self.stripe.issuing().cards().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_issuing_cards(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: Option<ListIssuingCardsParams> = parse_opt_json(&req.into_inner().json_params)?;
        let res = self.stripe.issuing().cards().list(params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_issuing_authorization(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.issuing().authorizations().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn update_issuing_authorization(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateIssuingAuthorizationParams = parse_json(&r.json_params)?;
        let res = self.stripe.issuing().authorizations().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn approve_issuing_authorization(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<ApproveAuthorizationParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.issuing().authorizations().approve(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn decline_issuing_authorization(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<DeclineAuthorizationParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.issuing().authorizations().decline(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_issuing_authorizations(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: Option<ListIssuingAuthorizationsParams> = parse_opt_json(&req.into_inner().json_params)?;
        let res = self.stripe.issuing().authorizations().list(params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_issuing_transaction(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.issuing().transactions().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn update_issuing_transaction(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateIssuingTransactionParams = parse_json(&r.json_params)?;
        let res = self.stripe.issuing().transactions().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_issuing_transactions(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: Option<ListIssuingTransactionsParams> = parse_opt_json(&req.into_inner().json_params)?;
        let res = self.stripe.issuing().transactions().list(params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn create_financial_account(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: CreateFinancialAccountParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.treasury().financial_accounts().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_financial_account(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.treasury().financial_accounts().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn update_financial_account(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateFinancialAccountParams = parse_json(&r.json_params)?;
        let res = self.stripe.treasury().financial_accounts().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_financial_accounts(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: Option<ListFinancialAccountsParams> = parse_opt_json(&req.into_inner().json_params)?;
        let res = self.stripe.treasury().financial_accounts().list(params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn close_financial_account(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<CloseFinancialAccountParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.treasury().financial_accounts().close(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn create_outbound_transfer(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: CreateOutboundTransferParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.treasury().outbound_transfers().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_outbound_transfer(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.treasury().outbound_transfers().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn cancel_outbound_transfer(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<CancelOutboundTransferParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.treasury().outbound_transfers().cancel(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_outbound_transfers(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: ListOutboundTransfersParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.treasury().outbound_transfers().list(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn create_outbound_payment(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: CreateOutboundPaymentParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.treasury().outbound_payments().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_outbound_payment(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.treasury().outbound_payments().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn cancel_outbound_payment(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<CancelOutboundPaymentParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.treasury().outbound_payments().cancel(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_outbound_payments(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: ListOutboundPaymentsParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.treasury().outbound_payments().list(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_received_credit(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.treasury().received_credits().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_received_credits(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: ListReceivedCreditsParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.treasury().received_credits().list(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn create_terminal_location(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: CreateLocationParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.terminal().locations().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_terminal_location(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.terminal().locations().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_terminal_locations(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: ListTerminalLocationsParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.terminal().locations().list(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn create_terminal_reader(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: CreateReaderParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.terminal().readers().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn retrieve_terminal_reader(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.terminal().readers().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn list_terminal_readers(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: ListTerminalReadersParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.terminal().readers().list(&params).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn cancel_terminal_reader_action(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let res = self.stripe.terminal().readers().cancel_action(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }
    async fn create_terminal_connection_token(&self, req: Request<SpecializedRpcRequest>) -> Result<Response<SpecializedRpcResponse>, Status> {
        let params: CreateConnectionTokenParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.terminal().connection_tokens().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = SpecializedProductsGrpcServer::from_env();
    println!(
        "Tonic gRPC Specialized Products service ready (api_base: {})",
        server.stripe.config.api_base
    );
    Ok(())
}
