//! # Tonic gRPC Example 5: Invoices, Credit Notes, Quotes & Billing Portal (Module 5 Part B)
//!
//! Demonstrates how to integrate **Invoices, Credit Notes, Quotes, and Customer Billing Portal**
//! from `some-stripe` inside a **Tonic (`tonic`)** gRPC service, including binary PDF responses.
//!
//! ## Services & gRPC Methods Covered
//! - `stripe.invoices()`: `create_invoice`, `retrieve_invoice`, `update_invoice`,
//!   `delete_invoice`, `finalize_invoice`, `pay_invoice`, `void_invoice`,
//!   `mark_invoice_uncollectible`, `send_invoice`, `list_invoices`, `retrieve_upcoming_invoice`,
//!   `add_invoice_lines`, `update_invoice_line`, `remove_invoice_lines`, `download_invoice_pdf`
//! - `stripe.credit_notes()`: `create_credit_note`, `retrieve_credit_note`,
//!   `update_credit_note`, `void_credit_note`, `preview_credit_note`, `list_credit_notes`,
//!   `validate_credit_deduction`
//! - `stripe.quotes()`: `create_quote`, `retrieve_quote`, `update_quote`, `finalize_quote`,
//!   `accept_quote`, `cancel_quote`, `download_quote_pdf`, `list_quotes`
//! - `stripe.billing_portal()`: `create_portal_session`, `create_portal_configuration`,
//!   `retrieve_portal_configuration`, `update_portal_configuration`, `list_portal_configurations`
//!
//! ## Run the Example
//! ```bash
//! cargo run --example tonic_invoicing_and_billing_portal
//! ```

use prost::Message;
use serde::{Deserialize, Serialize};
use some_stripe::{
    AcceptQuoteParams, AddInvoiceLinesParams, CancelQuoteParams, CreateCreditNoteParams,
    CreateInvoiceParams, CreatePortalConfigurationParams, CreatePortalSessionParams,
    CreateQuoteParams, FinalizeInvoiceParams, FinalizeQuoteParams, InvoiceUpcomingParams,
    ListCreditNotesParams, ListInvoicesParams, ListQuotesParams, PayInvoiceParams,
    PreviewCreditNoteParams, StripeClient, StripeError, UpdateCreditNoteParams,
    UpdateInvoiceLineParams, UpdateInvoiceParams, UpdatePortalConfigurationParams,
    UpdateQuoteParams, VoidCreditNoteParams, VoidInvoiceParams,
};
use tonic::{Request, Response, Status};

#[derive(Clone, PartialEq, Message)]
pub struct InvoicingRpcRequest {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub sub_id: String,
    #[prost(string, tag = "3")]
    pub json_params: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct InvoicingRpcResponse {
    #[prost(string, tag = "1")]
    pub json_data: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct BinaryPdfResponse {
    #[prost(bytes = "vec", tag = "1")]
    pub pdf_bytes: Vec<u8>,
    #[prost(string, tag = "2")]
    pub content_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RemoveLinesInput {
    pub line_ids: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ValidateCreditDeductionInput {
    pub credit_amount: i64,
    pub invoice_paid_amount: i64,
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

fn to_response<T: Serialize>(value: &T) -> Result<Response<InvoicingRpcResponse>, Status> {
    let json_data =
        serde_json::to_string(value).map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(InvoicingRpcResponse { json_data }))
}

/// Tonic gRPC service trait for Invoices, Credit Notes, Quotes & Customer Billing Portal.
#[tonic::async_trait]
pub trait InvoicingPortalGrpcService: Send + Sync + 'static {
    async fn create_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn retrieve_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn update_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn delete_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn finalize_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn pay_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn void_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn mark_invoice_uncollectible(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn send_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn list_invoices(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn retrieve_upcoming_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn add_invoice_lines(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn update_invoice_line(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn remove_invoice_lines(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn download_invoice_pdf(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<BinaryPdfResponse>, Status>;

    async fn create_credit_note(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn retrieve_credit_note(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn update_credit_note(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn void_credit_note(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn preview_credit_note(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn list_credit_notes(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn validate_credit_deduction(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;

    async fn create_quote(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn retrieve_quote(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn update_quote(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn finalize_quote(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn accept_quote(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn cancel_quote(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn download_quote_pdf(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<BinaryPdfResponse>, Status>;
    async fn list_quotes(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;

    async fn create_portal_session(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn create_portal_configuration(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn retrieve_portal_configuration(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn update_portal_configuration(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
    async fn list_portal_configurations(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status>;
}

/// Tonic gRPC server implementation for Invoicing & Billing Portal.
#[derive(Clone, Debug)]
pub struct InvoicingPortalGrpcServer {
    pub stripe: StripeClient,
}

impl InvoicingPortalGrpcServer {
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            stripe: StripeClient::new("$STRIPE_SECRET_KEY"),
        }
    }
}

#[tonic::async_trait]
impl InvoicingPortalGrpcService for InvoicingPortalGrpcServer {
    async fn create_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let params: CreateInvoiceParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.invoices().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let res = self.stripe.invoices().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn update_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateInvoiceParams = parse_json(&r.json_params)?;
        let res = self.stripe.invoices().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn delete_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let res = self.stripe.invoices().del(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn finalize_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<FinalizeInvoiceParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.invoices().finalize_invoice(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn pay_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<PayInvoiceParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.invoices().pay(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn void_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<VoidInvoiceParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.invoices().void_invoice(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn mark_invoice_uncollectible(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let res = self.stripe.invoices().mark_uncollectible(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn send_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let res = self.stripe.invoices().send_invoice(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn list_invoices(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let params: Option<ListInvoicesParams> = parse_opt_json(&req.into_inner().json_params)?;
        let res = self.stripe.invoices().list(params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_upcoming_invoice(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let params: InvoiceUpcomingParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.invoices().upcoming(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn add_invoice_lines(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: AddInvoiceLinesParams = parse_json(&r.json_params)?;
        let res = self.stripe.invoices().add_lines(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn update_invoice_line(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateInvoiceLineParams = parse_json(&r.json_params)?;
        let res = self.stripe.invoices().update_lines(&r.id, &r.sub_id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn remove_invoice_lines(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let r = req.into_inner();
        let input: RemoveLinesInput = parse_json(&r.json_params)?;
        let refs: Vec<&str> = input.line_ids.iter().map(String::as_str).collect();
        let res = self.stripe.invoices().remove_lines(&r.id, &refs).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn download_invoice_pdf(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<BinaryPdfResponse>, Status> {
        let bytes = self.stripe.invoices().pdf(&req.into_inner().id).await.map_err(to_status)?;
        Ok(Response::new(BinaryPdfResponse {
            pdf_bytes: bytes.to_vec(),
            content_type: "application/pdf".to_string(),
        }))
    }

    async fn create_credit_note(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let params: CreateCreditNoteParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.credit_notes().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_credit_note(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let res = self.stripe.credit_notes().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn update_credit_note(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateCreditNoteParams = parse_json(&r.json_params)?;
        let res = self.stripe.credit_notes().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn void_credit_note(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<VoidCreditNoteParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.credit_notes().void_credit_note(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn preview_credit_note(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let params: PreviewCreditNoteParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.credit_notes().preview(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn list_credit_notes(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let params: Option<ListCreditNotesParams> = parse_opt_json(&req.into_inner().json_params)?;
        let res = self.stripe.credit_notes().list(params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn validate_credit_deduction(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let input: ValidateCreditDeductionInput = parse_json(&req.into_inner().json_params)?;
        self.stripe
            .credit_notes()
            .validate_balance_deduction(input.credit_amount, input.invoice_paid_amount)
            .map_err(to_status)?;
        to_response(&serde_json::json!({ "valid": true }))
    }

    async fn create_quote(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let params: CreateQuoteParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.quotes().create(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_quote(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let res = self.stripe.quotes().retrieve(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn update_quote(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdateQuoteParams = parse_json(&r.json_params)?;
        let res = self.stripe.quotes().update(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn finalize_quote(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<FinalizeQuoteParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.quotes().finalize_quote(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn accept_quote(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<AcceptQuoteParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.quotes().accept(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn cancel_quote(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: Option<CancelQuoteParams> = parse_opt_json(&r.json_params)?;
        let res = self.stripe.quotes().cancel(&r.id, params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn download_quote_pdf(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<BinaryPdfResponse>, Status> {
        let bytes = self.stripe.quotes().pdf(&req.into_inner().id).await.map_err(to_status)?;
        Ok(Response::new(BinaryPdfResponse {
            pdf_bytes: bytes.to_vec(),
            content_type: "application/pdf".to_string(),
        }))
    }

    async fn list_quotes(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let params: Option<ListQuotesParams> = parse_opt_json(&req.into_inner().json_params)?;
        let res = self.stripe.quotes().list(params.as_ref()).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn create_portal_session(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let params: CreatePortalSessionParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.billing_portal().create_session(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn create_portal_configuration(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let params: CreatePortalConfigurationParams = parse_json(&req.into_inner().json_params)?;
        let res = self.stripe.billing_portal().create_configuration(&params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn retrieve_portal_configuration(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let res = self.stripe.billing_portal().retrieve_configuration(&req.into_inner().id).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn update_portal_configuration(&self, req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let r = req.into_inner();
        let params: UpdatePortalConfigurationParams = parse_json(&r.json_params)?;
        let res = self.stripe.billing_portal().update_configuration(&r.id, &params).await.map_err(to_status)?;
        to_response(&res)
    }

    async fn list_portal_configurations(&self, _req: Request<InvoicingRpcRequest>) -> Result<Response<InvoicingRpcResponse>, Status> {
        let res = self.stripe.billing_portal().list_configurations(Some(10)).await.map_err(to_status)?;
        to_response(&res)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = InvoicingPortalGrpcServer::from_env();
    println!(
        "Tonic gRPC Invoicing & Billing Portal service ready (api_base: {})",
        server.stripe.config.api_base
    );
    Ok(())
}
