//! # Actix-Web Example 5: Invoices, Credit Notes, Quotes & Billing Portal (Module 5 Part B)
//!
//! Demonstrates how to integrate **Invoices, Credit Notes, Quotes, and Customer Billing Portal**
//! from `some-stripe` inside an `actix-web` application.
//!
//! ## Services Covered
//! - `stripe.invoices()`: `create`, `retrieve`, `update`, `del`, `finalize_invoice`, `pay`,
//!   `void_invoice`, `mark_uncollectible`, `send_invoice`, `list`, `upcoming`, `add_lines`,
//!   `update_lines`, `remove_lines`, `list_line_items_default`, and `pdf` (zero-copy `bytes::Bytes`)
//! - `stripe.credit_notes()`: `create`, `retrieve`, `update`, `void_credit_note`, `preview`,
//!   `list`, `list_default`, and `validate_balance_deduction`
//! - `stripe.quotes()`: `create`, `retrieve`, `update`, `finalize_quote`, `accept`, `cancel`,
//!   `pdf`, `list`, and `list_default`
//! - `stripe.billing_portal()`: `create_session`, `create_configuration`,
//!   `retrieve_configuration`, `update_configuration`, and `list_configurations`
//!
//! ## Run the Example
//! ```bash
//! STRIPE_SECRET_KEY=sk_test_123 cargo run --example actix_invoicing_and_billing_portal
//! ```

use actix_web::{App, HttpResponse, HttpServer, ResponseError, delete, get, post, web};
use serde::{Deserialize, Serialize};
use some_stripe::{
    AcceptQuoteParams, AddInvoiceLinesParams, CancelQuoteParams, CreateCreditNoteParams,
    CreateInvoiceParams, CreatePortalConfigurationParams, CreatePortalSessionParams,
    CreateQuoteParams, FinalizeInvoiceParams, FinalizeQuoteParams, InvoiceUpcomingParams,
    ListCreditNotesParams, ListInvoicesParams, ListQuotesParams, PayInvoiceParams,
    PreviewCreditNoteParams, SecretKey, StripeClient, StripeError, UpdateCreditNoteParams,
    UpdateInvoiceLineParams, UpdateInvoiceParams, UpdatePortalConfigurationParams,
    UpdateQuoteParams, VoidCreditNoteParams, VoidInvoiceParams,
};
use std::fmt;

/// Error wrapper converting `StripeError` into structured Actix-Web HTTP responses.
#[derive(Debug)]
pub struct InvoicingError(pub StripeError);

impl fmt::Display for InvoicingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<StripeError> for InvoicingError {
    fn from(err: StripeError) -> Self {
        Self(err)
    }
}

impl ResponseError for InvoicingError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::BadGateway().json(serde_json::json!({
            "error": self.0.to_string(),
        }))
    }
}

/// Request payload for `POST /invoices/{id}/remove-lines`.
#[derive(Debug, Deserialize, Serialize)]
pub struct RemoveLinesRequest {
    pub line_ids: Vec<String>,
}

/// Request payload for `POST /credit-notes/validate-deduction`.
#[derive(Debug, Deserialize, Serialize)]
pub struct ValidateCreditDeductionRequest {
    pub credit_amount: i64,
    pub invoice_paid_amount: i64,
}

/// `POST /invoices` — Creates a draft invoice via `stripe.invoices().create()`.
#[post("/invoices")]
pub async fn create_invoice(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateInvoiceParams>,
) -> Result<HttpResponse, InvoicingError> {
    let invoice = stripe.invoices().create(&params).await?;
    Ok(HttpResponse::Created().json(invoice))
}

/// `GET /invoices/{id}` — Retrieves an invoice via `stripe.invoices().retrieve()`.
#[get("/invoices/{id}")]
pub async fn retrieve_invoice(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, InvoicingError> {
    let invoice = stripe.invoices().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(invoice))
}

/// `POST /invoices/{id}` — Updates an invoice via `stripe.invoices().update()`.
#[post("/invoices/{id}")]
pub async fn update_invoice(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateInvoiceParams>,
) -> Result<HttpResponse, InvoicingError> {
    let invoice = stripe.invoices().update(id.as_str(), &params).await?;
    Ok(HttpResponse::Ok().json(invoice))
}

/// `DELETE /invoices/{id}` — Deletes a draft invoice via `stripe.invoices().del()`.
#[delete("/invoices/{id}")]
pub async fn delete_invoice(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, InvoicingError> {
    let deleted = stripe.invoices().del(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(deleted))
}

/// `POST /invoices/{id}/finalize` — Finalizes a draft invoice via `stripe.invoices().finalize_invoice()`.
#[post("/invoices/{id}/finalize")]
pub async fn finalize_invoice(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: Option<web::Json<FinalizeInvoiceParams>>,
) -> Result<HttpResponse, InvoicingError> {
    let invoice = stripe
        .invoices()
        .finalize_invoice(id.as_str(), params.as_deref())
        .await?;
    Ok(HttpResponse::Ok().json(invoice))
}

/// `POST /invoices/{id}/pay` — Pays an open invoice via `stripe.invoices().pay()`.
#[post("/invoices/{id}/pay")]
pub async fn pay_invoice(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: Option<web::Json<PayInvoiceParams>>,
) -> Result<HttpResponse, InvoicingError> {
    let invoice = stripe.invoices().pay(id.as_str(), params.as_deref()).await?;
    Ok(HttpResponse::Ok().json(invoice))
}

/// `POST /invoices/{id}/void` — Voids an invoice via `stripe.invoices().void_invoice()`.
#[post("/invoices/{id}/void")]
pub async fn void_invoice(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: Option<web::Json<VoidInvoiceParams>>,
) -> Result<HttpResponse, InvoicingError> {
    let invoice = stripe
        .invoices()
        .void_invoice(id.as_str(), params.as_deref())
        .await?;
    Ok(HttpResponse::Ok().json(invoice))
}

/// `POST /invoices/{id}/mark-uncollectible` — Marks an invoice uncollectible via
/// `stripe.invoices().mark_uncollectible()`.
#[post("/invoices/{id}/mark-uncollectible")]
pub async fn mark_invoice_uncollectible(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, InvoicingError> {
    let invoice = stripe.invoices().mark_uncollectible(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(invoice))
}

/// `POST /invoices/{id}/send` — Sends an invoice to the customer via `stripe.invoices().send_invoice()`.
#[post("/invoices/{id}/send")]
pub async fn send_invoice(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, InvoicingError> {
    let invoice = stripe.invoices().send_invoice(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(invoice))
}

/// `GET /invoices` — Lists invoices via `stripe.invoices().list()`.
#[get("/invoices")]
pub async fn list_invoices(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListInvoicesParams>,
) -> Result<HttpResponse, InvoicingError> {
    let list = stripe.invoices().list(Some(&query)).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /invoices/upcoming` — Retrieves an upcoming invoice preview (`id: None`) via
/// `stripe.invoices().upcoming()`.
#[post("/invoices/upcoming")]
pub async fn retrieve_upcoming_invoice(
    stripe: web::Data<StripeClient>,
    params: web::Json<InvoiceUpcomingParams>,
) -> Result<HttpResponse, InvoicingError> {
    let upcoming = stripe.invoices().upcoming(&params).await?;
    Ok(HttpResponse::Ok().json(upcoming))
}

/// `POST /invoices/{id}/lines` — Adds line items to an invoice via `stripe.invoices().add_lines()`.
#[post("/invoices/{id}/lines")]
pub async fn add_invoice_lines(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<AddInvoiceLinesParams>,
) -> Result<HttpResponse, InvoicingError> {
    let invoice = stripe.invoices().add_lines(id.as_str(), &params).await?;
    Ok(HttpResponse::Ok().json(invoice))
}

/// `POST /invoices/{id}/lines/{line_id}` — Updates an invoice line item via
/// `stripe.invoices().update_lines()`.
#[post("/invoices/{id}/lines/{line_id}")]
pub async fn update_invoice_line(
    stripe: web::Data<StripeClient>,
    path: web::Path<(String, String)>,
    params: web::Json<UpdateInvoiceLineParams>,
) -> Result<HttpResponse, InvoicingError> {
    let (invoice_id, line_id) = path.into_inner();
    let item = stripe
        .invoices()
        .update_lines(&invoice_id, &line_id, &params)
        .await?;
    Ok(HttpResponse::Ok().json(item))
}

/// `POST /invoices/{id}/remove-lines` — Removes line items from an invoice via
/// `stripe.invoices().remove_lines()` using borrowed `&str` slices.
#[post("/invoices/{id}/remove-lines")]
pub async fn remove_invoice_lines(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    body: web::Json<RemoveLinesRequest>,
) -> Result<HttpResponse, InvoicingError> {
    let refs: Vec<&str> = body.line_ids.iter().map(String::as_str).collect();
    let invoice = stripe.invoices().remove_lines(id.as_str(), &refs).await?;
    Ok(HttpResponse::Ok().json(invoice))
}

/// `GET /invoices/{id}/pdf` — Downloads the rendered invoice PDF as zero-copy `bytes::Bytes`
/// via `stripe.invoices().pdf()`.
#[get("/invoices/{id}/pdf")]
pub async fn download_invoice_pdf(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, InvoicingError> {
    let pdf_bytes = stripe.invoices().pdf(id.as_str()).await?;
    Ok(HttpResponse::Ok()
        .content_type("application/pdf")
        .body(pdf_bytes))
}

/// `POST /credit-notes` — Issues a credit note via `stripe.credit_notes().create()`.
#[post("/credit-notes")]
pub async fn create_credit_note(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateCreditNoteParams>,
) -> Result<HttpResponse, InvoicingError> {
    let cn = stripe.credit_notes().create(&params).await?;
    Ok(HttpResponse::Created().json(cn))
}

/// `GET /credit-notes/{id}` — Retrieves a credit note via `stripe.credit_notes().retrieve()`.
#[get("/credit-notes/{id}")]
pub async fn retrieve_credit_note(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, InvoicingError> {
    let cn = stripe.credit_notes().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(cn))
}

/// `POST /credit-notes/{id}` — Updates a credit note via `stripe.credit_notes().update()`.
#[post("/credit-notes/{id}")]
pub async fn update_credit_note(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateCreditNoteParams>,
) -> Result<HttpResponse, InvoicingError> {
    let cn = stripe.credit_notes().update(id.as_str(), &params).await?;
    Ok(HttpResponse::Ok().json(cn))
}

/// `POST /credit-notes/{id}/void` — Voids a credit note via `stripe.credit_notes().void_credit_note()`.
#[post("/credit-notes/{id}/void")]
pub async fn void_credit_note(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: Option<web::Json<VoidCreditNoteParams>>,
) -> Result<HttpResponse, InvoicingError> {
    let cn = stripe
        .credit_notes()
        .void_credit_note(id.as_str(), params.as_deref())
        .await?;
    Ok(HttpResponse::Ok().json(cn))
}

/// `POST /credit-notes/preview` — Previews a credit note via `stripe.credit_notes().preview()`.
#[post("/credit-notes/preview")]
pub async fn preview_credit_note(
    stripe: web::Data<StripeClient>,
    params: web::Json<PreviewCreditNoteParams>,
) -> Result<HttpResponse, InvoicingError> {
    let preview = stripe.credit_notes().preview(&params).await?;
    Ok(HttpResponse::Ok().json(preview))
}

/// `GET /credit-notes` — Lists credit notes via `stripe.credit_notes().list()`.
#[get("/credit-notes")]
pub async fn list_credit_notes(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListCreditNotesParams>,
) -> Result<HttpResponse, InvoicingError> {
    let list = stripe.credit_notes().list(Some(&query)).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /credit-notes/validate-deduction` — Validates credit note balance deduction invariants
/// via `stripe.credit_notes().validate_balance_deduction()`.
#[post("/credit-notes/validate-deduction")]
pub async fn validate_credit_deduction(
    stripe: web::Data<StripeClient>,
    body: web::Json<ValidateCreditDeductionRequest>,
) -> Result<HttpResponse, InvoicingError> {
    stripe
        .credit_notes()
        .validate_balance_deduction(body.credit_amount, body.invoice_paid_amount)?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "valid": true })))
}

/// `POST /quotes` — Creates a commercial quote via `stripe.quotes().create()`.
#[post("/quotes")]
pub async fn create_quote(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateQuoteParams>,
) -> Result<HttpResponse, InvoicingError> {
    let quote = stripe.quotes().create(&params).await?;
    Ok(HttpResponse::Created().json(quote))
}

/// `GET /quotes/{id}` — Retrieves a quote via `stripe.quotes().retrieve()`.
#[get("/quotes/{id}")]
pub async fn retrieve_quote(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, InvoicingError> {
    let quote = stripe.quotes().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(quote))
}

/// `POST /quotes/{id}` — Updates a quote via `stripe.quotes().update()`.
#[post("/quotes/{id}")]
pub async fn update_quote(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateQuoteParams>,
) -> Result<HttpResponse, InvoicingError> {
    let quote = stripe.quotes().update(id.as_str(), &params).await?;
    Ok(HttpResponse::Ok().json(quote))
}

/// `POST /quotes/{id}/finalize` — Finalizes a quote via `stripe.quotes().finalize_quote()`.
#[post("/quotes/{id}/finalize")]
pub async fn finalize_quote(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: Option<web::Json<FinalizeQuoteParams>>,
) -> Result<HttpResponse, InvoicingError> {
    let quote = stripe
        .quotes()
        .finalize_quote(id.as_str(), params.as_deref())
        .await?;
    Ok(HttpResponse::Ok().json(quote))
}

/// `POST /quotes/{id}/accept` — Accepts a finalized quote via `stripe.quotes().accept()`.
#[post("/quotes/{id}/accept")]
pub async fn accept_quote(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: Option<web::Json<AcceptQuoteParams>>,
) -> Result<HttpResponse, InvoicingError> {
    let quote = stripe
        .quotes()
        .accept(id.as_str(), params.as_deref())
        .await?;
    Ok(HttpResponse::Ok().json(quote))
}

/// `POST /quotes/{id}/cancel` — Cancels a quote via `stripe.quotes().cancel()`.
#[post("/quotes/{id}/cancel")]
pub async fn cancel_quote(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: Option<web::Json<CancelQuoteParams>>,
) -> Result<HttpResponse, InvoicingError> {
    let quote = stripe
        .quotes()
        .cancel(id.as_str(), params.as_deref())
        .await?;
    Ok(HttpResponse::Ok().json(quote))
}

/// `GET /quotes/{id}/pdf` — Downloads the rendered quote PDF as zero-copy `bytes::Bytes`
/// via `stripe.quotes().pdf()`.
#[get("/quotes/{id}/pdf")]
pub async fn download_quote_pdf(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, InvoicingError> {
    let pdf_bytes = stripe.quotes().pdf(id.as_str()).await?;
    Ok(HttpResponse::Ok()
        .content_type("application/pdf")
        .body(pdf_bytes))
}

/// `GET /quotes` — Lists quotes via `stripe.quotes().list()`.
#[get("/quotes")]
pub async fn list_quotes(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListQuotesParams>,
) -> Result<HttpResponse, InvoicingError> {
    let list = stripe.quotes().list(Some(&query)).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /billing-portal/sessions` — Creates a Customer Billing Portal session via
/// `stripe.billing_portal().create_session()`.
#[post("/billing-portal/sessions")]
pub async fn create_portal_session(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreatePortalSessionParams>,
) -> Result<HttpResponse, InvoicingError> {
    let session = stripe.billing_portal().create_session(&params).await?;
    Ok(HttpResponse::Created().json(session))
}

/// `POST /billing-portal/configurations` — Creates a Customer Billing Portal configuration via
/// `stripe.billing_portal().create_configuration()`.
#[post("/billing-portal/configurations")]
pub async fn create_portal_configuration(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreatePortalConfigurationParams>,
) -> Result<HttpResponse, InvoicingError> {
    let cfg = stripe
        .billing_portal()
        .create_configuration(&params)
        .await?;
    Ok(HttpResponse::Created().json(cfg))
}

/// `GET /billing-portal/configurations/{id}` — Retrieves a Billing Portal configuration via
/// `stripe.billing_portal().retrieve_configuration()`.
#[get("/billing-portal/configurations/{id}")]
pub async fn retrieve_portal_configuration(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, InvoicingError> {
    let cfg = stripe
        .billing_portal()
        .retrieve_configuration(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(cfg))
}

/// `POST /billing-portal/configurations/{id}` — Updates a Billing Portal configuration via
/// `stripe.billing_portal().update_configuration()`.
#[post("/billing-portal/configurations/{id}")]
pub async fn update_portal_configuration(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdatePortalConfigurationParams>,
) -> Result<HttpResponse, InvoicingError> {
    let cfg = stripe
        .billing_portal()
        .update_configuration(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(cfg))
}

/// `GET /billing-portal/configurations` — Lists Billing Portal configurations via
/// `stripe.billing_portal().list_configurations()`.
#[get("/billing-portal/configurations")]
pub async fn list_portal_configurations(
    stripe: web::Data<StripeClient>,
) -> Result<HttpResponse, InvoicingError> {
    let list = stripe.billing_portal().list_configurations(Some(10)).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// Mounts all Invoices, Credit Notes, Quotes & Billing Portal routes onto an Actix-Web [`web::ServiceConfig`].
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_invoice)
        .service(retrieve_invoice)
        .service(update_invoice)
        .service(delete_invoice)
        .service(finalize_invoice)
        .service(pay_invoice)
        .service(void_invoice)
        .service(mark_invoice_uncollectible)
        .service(send_invoice)
        .service(list_invoices)
        .service(retrieve_upcoming_invoice)
        .service(add_invoice_lines)
        .service(update_invoice_line)
        .service(remove_invoice_lines)
        .service(download_invoice_pdf)
        .service(create_credit_note)
        .service(retrieve_credit_note)
        .service(update_credit_note)
        .service(void_credit_note)
        .service(preview_credit_note)
        .service(list_credit_notes)
        .service(validate_credit_deduction)
        .service(create_quote)
        .service(retrieve_quote)
        .service(update_quote)
        .service(finalize_quote)
        .service(accept_quote)
        .service(cancel_quote)
        .service(download_quote_pdf)
        .service(list_quotes)
        .service(create_portal_session)
        .service(create_portal_configuration)
        .service(retrieve_portal_configuration)
        .service(update_portal_configuration)
        .service(list_portal_configurations);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Automatically resolves $STRIPE_SECRET_KEY from process env, .env.local, .env.dev, .env, or .env.example
    let stripe = web::Data::new(StripeClient::new("$STRIPE_SECRET_KEY"));

    println!("Starting Actix-Web Invoicing & Billing Portal example on http://127.0.0.1:8085");
    HttpServer::new(move || {
        App::new()
            .app_data(stripe.clone())
            .configure(configure_routes)
    })
    .bind(("127.0.0.1", 8085))?
    .run()
    .await
}
