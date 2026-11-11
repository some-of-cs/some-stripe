//! # Actix-Web Example 7: Tax, Identity, Issuing, Treasury & Terminal (Module 7)
//!
//! Demonstrates how to integrate **Module 7 (Specialized Product Namespaces)** of `some-stripe`
//! inside an `actix-web` application.
//!
//! ## Services Covered
//! - `stripe.tax()`:
//!   - `calculations()`: `create`, `retrieve`, and `list_line_items_page`
//!   - `transactions()`: `create_from_calculation`, `create_reversal`, `retrieve`, and
//!     `list_line_items_default`
//!   - `registrations()`: `create`, `retrieve`, `update`, and `list_page`
//!   - `settings()`: `retrieve` and `update`
//! - `stripe.identity()`:
//!   - `verification_sessions()`: `create`, `retrieve`, `update`, `cancel`, and `redact`
//!   - `verification_reports()`: `retrieve` and `list_default`
//! - `stripe.issuing()`:
//!   - `cardholders()`: `create`, `retrieve`, `update`, and `list`
//!   - `cards()`: `create`, `retrieve`, `update`, and `list`
//!   - `authorizations()`: `retrieve`, `update`, `approve`, `decline`, and `list`
//!   - `transactions()`: `retrieve`, `update`, and `list`
//! - `stripe.treasury()`:
//!   - `financial_accounts()`: `create`, `retrieve`, `update`, `list`, and `close`
//!   - `outbound_transfers()`: `create`, `retrieve`, `cancel`, and `list`
//!   - `outbound_payments()`: `create`, `retrieve`, `cancel`, and `list`
//!   - `received_credits()`: `retrieve` and `list`
//! - `stripe.terminal()`:
//!   - `locations()`: `create`, `retrieve`, and `list`
//!   - `readers()`: `create`, `retrieve`, `list`, and `cancel_action`
//!   - `connection_tokens()`: `create`
//!
//! ## Run the Example
//! ```bash
//! STRIPE_SECRET_KEY=sk_test_123 cargo run --example actix_specialized_products
//! ```

use actix_web::{App, HttpResponse, HttpServer, ResponseError, get, post, web};
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
    ListTerminalLocationsParams, ListTerminalReadersParams, SecretKey, StripeClient, StripeError,
    UpdateCardParams, UpdateCardholderParams, UpdateFinancialAccountParams,
    UpdateIssuingAuthorizationParams, UpdateIssuingTransactionParams, UpdateTaxRegistrationParams,
    UpdateTaxSettingsParams, UpdateVerificationSessionParams,
};
use std::fmt;

/// Error wrapper converting `StripeError` into structured Actix-Web HTTP responses.
#[derive(Debug)]
pub struct SpecializedError(pub StripeError);

impl fmt::Display for SpecializedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<StripeError> for SpecializedError {
    fn from(err: StripeError) -> Self {
        Self(err)
    }
}

impl ResponseError for SpecializedError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::BadGateway().json(serde_json::json!({
            "error": self.0.to_string(),
        }))
    }
}

// ============================================================================
// 1. Stripe Tax (`stripe.tax()`)
// ============================================================================

/// `POST /tax/calculations` — Computes real-time sales tax / VAT / GST via
/// `stripe.tax().calculations().create()`.
#[post("/tax/calculations")]
pub async fn create_tax_calculation(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateTaxCalculationParams>,
) -> Result<HttpResponse, SpecializedError> {
    let calc = stripe.tax().calculations().create(&params).await?;
    Ok(HttpResponse::Created().json(calc))
}

/// `GET /tax/calculations/{id}` — Retrieves a tax calculation via
/// `stripe.tax().calculations().retrieve()`.
#[get("/tax/calculations/{id}")]
pub async fn retrieve_tax_calculation(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let calc = stripe.tax().calculations().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(calc))
}

/// `GET /tax/calculations/{id}/line-items` — Lists tax calculation line items via
/// `stripe.tax().calculations().list_line_items_page()`.
#[get("/tax/calculations/{id}/line-items")]
pub async fn list_tax_calculation_line_items(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    query: web::Query<ListTaxCalculationLineItemsParams>,
) -> Result<HttpResponse, SpecializedError> {
    let page = stripe
        .tax()
        .calculations()
        .list_line_items_page(id.as_str(), Some(&query))
        .await?;
    Ok(HttpResponse::Ok().json(page))
}

/// `POST /tax/transactions/from-calculation` — Commits a tax calculation to the tax ledger via
/// `stripe.tax().transactions().create_from_calculation()`.
#[post("/tax/transactions/from-calculation")]
pub async fn create_tax_transaction_from_calculation(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateTaxTransactionFromCalculationParams>,
) -> Result<HttpResponse, SpecializedError> {
    let tx = stripe
        .tax()
        .transactions()
        .create_from_calculation(&params)
        .await?;
    Ok(HttpResponse::Created().json(tx))
}

/// `POST /tax/transactions/reversal` — Creates a tax transaction reversal via
/// `stripe.tax().transactions().create_reversal()`.
#[post("/tax/transactions/reversal")]
pub async fn create_tax_transaction_reversal(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateTaxTransactionReversalParams>,
) -> Result<HttpResponse, SpecializedError> {
    let tx = stripe.tax().transactions().create_reversal(&params).await?;
    Ok(HttpResponse::Created().json(tx))
}

/// `GET /tax/transactions/{id}` — Retrieves a tax transaction via
/// `stripe.tax().transactions().retrieve()`.
#[get("/tax/transactions/{id}")]
pub async fn retrieve_tax_transaction(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let tx = stripe.tax().transactions().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(tx))
}

/// `POST /tax/registrations` — Creates a regional tax nexus registration via
/// `stripe.tax().registrations().create()`.
#[post("/tax/registrations")]
pub async fn create_tax_registration(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateTaxRegistrationParams>,
) -> Result<HttpResponse, SpecializedError> {
    let reg = stripe.tax().registrations().create(&params).await?;
    Ok(HttpResponse::Created().json(reg))
}

/// `GET /tax/registrations/{id}` — Retrieves a tax registration via
/// `stripe.tax().registrations().retrieve()`.
#[get("/tax/registrations/{id}")]
pub async fn retrieve_tax_registration(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let reg = stripe.tax().registrations().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(reg))
}

/// `POST /tax/registrations/{id}` — Updates a tax registration via
/// `stripe.tax().registrations().update()`.
#[post("/tax/registrations/{id}")]
pub async fn update_tax_registration(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateTaxRegistrationParams>,
) -> Result<HttpResponse, SpecializedError> {
    let reg = stripe
        .tax()
        .registrations()
        .update(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(reg))
}

/// `GET /tax/registrations` — Lists tax registrations via `stripe.tax().registrations().list_page()`.
#[get("/tax/registrations")]
pub async fn list_tax_registrations(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListTaxRegistrationsParams>,
) -> Result<HttpResponse, SpecializedError> {
    let page = stripe.tax().registrations().list_page(Some(&query)).await?;
    Ok(HttpResponse::Ok().json(page))
}

/// `GET /tax/settings` — Retrieves account-wide tax settings via `stripe.tax().settings().retrieve()`.
#[get("/tax/settings")]
pub async fn retrieve_tax_settings(
    stripe: web::Data<StripeClient>,
) -> Result<HttpResponse, SpecializedError> {
    let settings = stripe.tax().settings().retrieve().await?;
    Ok(HttpResponse::Ok().json(settings))
}

/// `POST /tax/settings` — Updates account-wide tax settings via `stripe.tax().settings().update()`.
#[post("/tax/settings")]
pub async fn update_tax_settings(
    stripe: web::Data<StripeClient>,
    params: web::Json<UpdateTaxSettingsParams>,
) -> Result<HttpResponse, SpecializedError> {
    let settings = stripe.tax().settings().update(&params).await?;
    Ok(HttpResponse::Ok().json(settings))
}

// ============================================================================
// 2. Stripe Identity (`stripe.identity()`)
// ============================================================================

/// `POST /identity/sessions` — Creates an Identity VerificationSession via
/// `stripe.identity().verification_sessions().create()`.
#[post("/identity/sessions")]
pub async fn create_verification_session(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateVerificationSessionParams>,
) -> Result<HttpResponse, SpecializedError> {
    let session = stripe
        .identity()
        .verification_sessions()
        .create(&params)
        .await?;
    Ok(HttpResponse::Created().json(session))
}

/// `GET /identity/sessions/{id}` — Retrieves an Identity VerificationSession via
/// `stripe.identity().verification_sessions().retrieve()`.
#[get("/identity/sessions/{id}")]
pub async fn retrieve_verification_session(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let session = stripe
        .identity()
        .verification_sessions()
        .retrieve(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(session))
}

/// `POST /identity/sessions/{id}` — Updates an Identity VerificationSession via
/// `stripe.identity().verification_sessions().update()`.
#[post("/identity/sessions/{id}")]
pub async fn update_verification_session(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateVerificationSessionParams>,
) -> Result<HttpResponse, SpecializedError> {
    let session = stripe
        .identity()
        .verification_sessions()
        .update(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(session))
}

/// `POST /identity/sessions/{id}/cancel` — Cancels an Identity VerificationSession via
/// `stripe.identity().verification_sessions().cancel()`.
#[post("/identity/sessions/{id}/cancel")]
pub async fn cancel_verification_session(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let session = stripe
        .identity()
        .verification_sessions()
        .cancel(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(session))
}

/// `POST /identity/sessions/{id}/redact` — Scrubs biometric & PII data (GDPR Art. 17) via
/// `stripe.identity().verification_sessions().redact()`.
#[post("/identity/sessions/{id}/redact")]
pub async fn redact_verification_session(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let session = stripe
        .identity()
        .verification_sessions()
        .redact(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(session))
}

/// `GET /identity/reports/{id}` — Retrieves an Identity VerificationReport via
/// `stripe.identity().verification_reports().retrieve()`.
#[get("/identity/reports/{id}")]
pub async fn retrieve_verification_report(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let report = stripe
        .identity()
        .verification_reports()
        .retrieve(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(report))
}

// ============================================================================
// 3. Stripe Issuing (`stripe.issuing()`)
// ============================================================================

/// `POST /issuing/cardholders` — Creates an Issuing cardholder via
/// `stripe.issuing().cardholders().create()`.
#[post("/issuing/cardholders")]
pub async fn create_issuing_cardholder(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateCardholderParams>,
) -> Result<HttpResponse, SpecializedError> {
    let ch = stripe.issuing().cardholders().create(&params).await?;
    Ok(HttpResponse::Created().json(ch))
}

/// `GET /issuing/cardholders/{id}` — Retrieves an Issuing cardholder via
/// `stripe.issuing().cardholders().retrieve()`.
#[get("/issuing/cardholders/{id}")]
pub async fn retrieve_issuing_cardholder(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let ch = stripe
        .issuing()
        .cardholders()
        .retrieve(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(ch))
}

/// `POST /issuing/cardholders/{id}` — Updates an Issuing cardholder via
/// `stripe.issuing().cardholders().update()`.
#[post("/issuing/cardholders/{id}")]
pub async fn update_issuing_cardholder(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateCardholderParams>,
) -> Result<HttpResponse, SpecializedError> {
    let ch = stripe
        .issuing()
        .cardholders()
        .update(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(ch))
}

/// `GET /issuing/cardholders` — Lists Issuing cardholders via `stripe.issuing().cardholders().list()`.
#[get("/issuing/cardholders")]
pub async fn list_issuing_cardholders(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListIssuingCardholdersParams>,
) -> Result<HttpResponse, SpecializedError> {
    let list = stripe.issuing().cardholders().list(Some(&query)).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /issuing/cards` — Provisions a virtual or physical Issuing card via
/// `stripe.issuing().cards().create()`.
#[post("/issuing/cards")]
pub async fn create_issuing_card(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateCardParams>,
) -> Result<HttpResponse, SpecializedError> {
    let card = stripe.issuing().cards().create(&params).await?;
    Ok(HttpResponse::Created().json(card))
}

/// `GET /issuing/cards/{id}` — Retrieves an Issuing card via `stripe.issuing().cards().retrieve()`.
#[get("/issuing/cards/{id}")]
pub async fn retrieve_issuing_card(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let card = stripe.issuing().cards().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(card))
}

/// `POST /issuing/cards/{id}` — Updates an Issuing card via `stripe.issuing().cards().update()`.
#[post("/issuing/cards/{id}")]
pub async fn update_issuing_card(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateCardParams>,
) -> Result<HttpResponse, SpecializedError> {
    let card = stripe
        .issuing()
        .cards()
        .update(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(card))
}

/// `GET /issuing/cards` — Lists Issuing cards via `stripe.issuing().cards().list()`.
#[get("/issuing/cards")]
pub async fn list_issuing_cards(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListIssuingCardsParams>,
) -> Result<HttpResponse, SpecializedError> {
    let list = stripe.issuing().cards().list(Some(&query)).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `GET /issuing/authorizations/{id}` — Retrieves an Issuing authorization via
/// `stripe.issuing().authorizations().retrieve()`.
#[get("/issuing/authorizations/{id}")]
pub async fn retrieve_issuing_authorization(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let auth = stripe
        .issuing()
        .authorizations()
        .retrieve(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(auth))
}

/// `POST /issuing/authorizations/{id}` — Updates an Issuing authorization via
/// `stripe.issuing().authorizations().update()`.
#[post("/issuing/authorizations/{id}")]
pub async fn update_issuing_authorization(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateIssuingAuthorizationParams>,
) -> Result<HttpResponse, SpecializedError> {
    let auth = stripe
        .issuing()
        .authorizations()
        .update(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(auth))
}

/// `POST /issuing/authorizations/{id}/approve` — Approves a real-time card authorization via
/// `stripe.issuing().authorizations().approve()`.
#[post("/issuing/authorizations/{id}/approve")]
pub async fn approve_issuing_authorization(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: Option<web::Json<ApproveAuthorizationParams>>,
) -> Result<HttpResponse, SpecializedError> {
    let auth = stripe
        .issuing()
        .authorizations()
        .approve(id.as_str(), params.as_deref())
        .await?;
    Ok(HttpResponse::Ok().json(auth))
}

/// `POST /issuing/authorizations/{id}/decline` — Declines a real-time card authorization via
/// `stripe.issuing().authorizations().decline()`.
#[post("/issuing/authorizations/{id}/decline")]
pub async fn decline_issuing_authorization(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: Option<web::Json<DeclineAuthorizationParams>>,
) -> Result<HttpResponse, SpecializedError> {
    let auth = stripe
        .issuing()
        .authorizations()
        .decline(id.as_str(), params.as_deref())
        .await?;
    Ok(HttpResponse::Ok().json(auth))
}

/// `GET /issuing/authorizations` — Lists Issuing authorizations via
/// `stripe.issuing().authorizations().list()`.
#[get("/issuing/authorizations")]
pub async fn list_issuing_authorizations(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListIssuingAuthorizationsParams>,
) -> Result<HttpResponse, SpecializedError> {
    let list = stripe
        .issuing()
        .authorizations()
        .list(Some(&query))
        .await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `GET /issuing/transactions/{id}` — Retrieves an Issuing transaction via
/// `stripe.issuing().transactions().retrieve()`.
#[get("/issuing/transactions/{id}")]
pub async fn retrieve_issuing_transaction(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let tx = stripe
        .issuing()
        .transactions()
        .retrieve(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(tx))
}

/// `POST /issuing/transactions/{id}` — Updates an Issuing transaction via
/// `stripe.issuing().transactions().update()`.
#[post("/issuing/transactions/{id}")]
pub async fn update_issuing_transaction(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateIssuingTransactionParams>,
) -> Result<HttpResponse, SpecializedError> {
    let tx = stripe
        .issuing()
        .transactions()
        .update(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(tx))
}

/// `GET /issuing/transactions` — Lists Issuing transactions via
/// `stripe.issuing().transactions().list()`.
#[get("/issuing/transactions")]
pub async fn list_issuing_transactions(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListIssuingTransactionsParams>,
) -> Result<HttpResponse, SpecializedError> {
    let list = stripe.issuing().transactions().list(Some(&query)).await?;
    Ok(HttpResponse::Ok().json(list))
}

// ============================================================================
// 4. Stripe Treasury (`stripe.treasury()`)
// ============================================================================

/// `POST /treasury/financial-accounts` — Creates a Treasury FinancialAccount via
/// `stripe.treasury().financial_accounts().create()`.
#[post("/treasury/financial-accounts")]
pub async fn create_financial_account(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateFinancialAccountParams>,
) -> Result<HttpResponse, SpecializedError> {
    let fa = stripe
        .treasury()
        .financial_accounts()
        .create(&params)
        .await?;
    Ok(HttpResponse::Created().json(fa))
}

/// `GET /treasury/financial-accounts/{id}` — Retrieves a Treasury FinancialAccount via
/// `stripe.treasury().financial_accounts().retrieve()`.
#[get("/treasury/financial-accounts/{id}")]
pub async fn retrieve_financial_account(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let fa = stripe
        .treasury()
        .financial_accounts()
        .retrieve(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(fa))
}

/// `POST /treasury/financial-accounts/{id}` — Updates a Treasury FinancialAccount via
/// `stripe.treasury().financial_accounts().update()`.
#[post("/treasury/financial-accounts/{id}")]
pub async fn update_financial_account(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateFinancialAccountParams>,
) -> Result<HttpResponse, SpecializedError> {
    let fa = stripe
        .treasury()
        .financial_accounts()
        .update(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(fa))
}

/// `GET /treasury/financial-accounts` — Lists Treasury FinancialAccounts via
/// `stripe.treasury().financial_accounts().list()`.
#[get("/treasury/financial-accounts")]
pub async fn list_financial_accounts(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListFinancialAccountsParams>,
) -> Result<HttpResponse, SpecializedError> {
    let list = stripe
        .treasury()
        .financial_accounts()
        .list(Some(&query))
        .await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /treasury/financial-accounts/{id}/close` — Closes a Treasury FinancialAccount via
/// `stripe.treasury().financial_accounts().close()`.
#[post("/treasury/financial-accounts/{id}/close")]
pub async fn close_financial_account(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: Option<web::Json<CloseFinancialAccountParams>>,
) -> Result<HttpResponse, SpecializedError> {
    let fa = stripe
        .treasury()
        .financial_accounts()
        .close(id.as_str(), params.as_deref())
        .await?;
    Ok(HttpResponse::Ok().json(fa))
}

/// `POST /treasury/outbound-transfers` — Creates a Treasury OutboundTransfer via
/// `stripe.treasury().outbound_transfers().create()`.
#[post("/treasury/outbound-transfers")]
pub async fn create_outbound_transfer(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateOutboundTransferParams>,
) -> Result<HttpResponse, SpecializedError> {
    let ot = stripe
        .treasury()
        .outbound_transfers()
        .create(&params)
        .await?;
    Ok(HttpResponse::Created().json(ot))
}

/// `GET /treasury/outbound-transfers/{id}` — Retrieves a Treasury OutboundTransfer via
/// `stripe.treasury().outbound_transfers().retrieve()`.
#[get("/treasury/outbound-transfers/{id}")]
pub async fn retrieve_outbound_transfer(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let ot = stripe
        .treasury()
        .outbound_transfers()
        .retrieve(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(ot))
}

/// `POST /treasury/outbound-transfers/{id}/cancel` — Cancels a Treasury OutboundTransfer via
/// `stripe.treasury().outbound_transfers().cancel()`.
#[post("/treasury/outbound-transfers/{id}/cancel")]
pub async fn cancel_outbound_transfer(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: Option<web::Json<CancelOutboundTransferParams>>,
) -> Result<HttpResponse, SpecializedError> {
    let ot = stripe
        .treasury()
        .outbound_transfers()
        .cancel(id.as_str(), params.as_deref())
        .await?;
    Ok(HttpResponse::Ok().json(ot))
}

/// `GET /treasury/outbound-transfers` — Lists Treasury OutboundTransfers via
/// `stripe.treasury().outbound_transfers().list()`.
#[get("/treasury/outbound-transfers")]
pub async fn list_outbound_transfers(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListOutboundTransfersParams>,
) -> Result<HttpResponse, SpecializedError> {
    let list = stripe.treasury().outbound_transfers().list(&query).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /treasury/outbound-payments` — Creates a Treasury OutboundPayment via
/// `stripe.treasury().outbound_payments().create()`.
#[post("/treasury/outbound-payments")]
pub async fn create_outbound_payment(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateOutboundPaymentParams>,
) -> Result<HttpResponse, SpecializedError> {
    let op = stripe
        .treasury()
        .outbound_payments()
        .create(&params)
        .await?;
    Ok(HttpResponse::Created().json(op))
}

/// `GET /treasury/outbound-payments/{id}` — Retrieves a Treasury OutboundPayment via
/// `stripe.treasury().outbound_payments().retrieve()`.
#[get("/treasury/outbound-payments/{id}")]
pub async fn retrieve_outbound_payment(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let op = stripe
        .treasury()
        .outbound_payments()
        .retrieve(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(op))
}

/// `POST /treasury/outbound-payments/{id}/cancel` — Cancels a Treasury OutboundPayment via
/// `stripe.treasury().outbound_payments().cancel()`.
#[post("/treasury/outbound-payments/{id}/cancel")]
pub async fn cancel_outbound_payment(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: Option<web::Json<CancelOutboundPaymentParams>>,
) -> Result<HttpResponse, SpecializedError> {
    let op = stripe
        .treasury()
        .outbound_payments()
        .cancel(id.as_str(), params.as_deref())
        .await?;
    Ok(HttpResponse::Ok().json(op))
}

/// `GET /treasury/outbound-payments` — Lists Treasury OutboundPayments via
/// `stripe.treasury().outbound_payments().list()`.
#[get("/treasury/outbound-payments")]
pub async fn list_outbound_payments(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListOutboundPaymentsParams>,
) -> Result<HttpResponse, SpecializedError> {
    let list = stripe.treasury().outbound_payments().list(&query).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `GET /treasury/received-credits/{id}` — Retrieves a Treasury ReceivedCredit via
/// `stripe.treasury().received_credits().retrieve()`.
#[get("/treasury/received-credits/{id}")]
pub async fn retrieve_received_credit(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let rc = stripe
        .treasury()
        .received_credits()
        .retrieve(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(rc))
}

/// `GET /treasury/received-credits` — Lists Treasury ReceivedCredits via
/// `stripe.treasury().received_credits().list()`.
#[get("/treasury/received-credits")]
pub async fn list_received_credits(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListReceivedCreditsParams>,
) -> Result<HttpResponse, SpecializedError> {
    let list = stripe.treasury().received_credits().list(&query).await?;
    Ok(HttpResponse::Ok().json(list))
}

// ============================================================================
// 5. Stripe Terminal (`stripe.terminal()`)
// ============================================================================

/// `POST /terminal/locations` — Creates a physical store Terminal Location via
/// `stripe.terminal().locations().create()`.
#[post("/terminal/locations")]
pub async fn create_terminal_location(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateLocationParams>,
) -> Result<HttpResponse, SpecializedError> {
    let loc = stripe.terminal().locations().create(&params).await?;
    Ok(HttpResponse::Created().json(loc))
}

/// `GET /terminal/locations/{id}` — Retrieves a Terminal Location via
/// `stripe.terminal().locations().retrieve()`.
#[get("/terminal/locations/{id}")]
pub async fn retrieve_terminal_location(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let loc = stripe.terminal().locations().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(loc))
}

/// `GET /terminal/locations` — Lists Terminal Locations via `stripe.terminal().locations().list()`.
#[get("/terminal/locations")]
pub async fn list_terminal_locations(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListTerminalLocationsParams>,
) -> Result<HttpResponse, SpecializedError> {
    let list = stripe.terminal().locations().list(&query).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /terminal/readers` — Registers a POS Terminal Reader via
/// `stripe.terminal().readers().create()`.
#[post("/terminal/readers")]
pub async fn create_terminal_reader(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateReaderParams>,
) -> Result<HttpResponse, SpecializedError> {
    let reader = stripe.terminal().readers().create(&params).await?;
    Ok(HttpResponse::Created().json(reader))
}

/// `GET /terminal/readers/{id}` — Retrieves a POS Terminal Reader via
/// `stripe.terminal().readers().retrieve()`.
#[get("/terminal/readers/{id}")]
pub async fn retrieve_terminal_reader(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let reader = stripe.terminal().readers().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(reader))
}

/// `GET /terminal/readers` — Lists POS Terminal Readers via `stripe.terminal().readers().list()`.
#[get("/terminal/readers")]
pub async fn list_terminal_readers(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListTerminalReadersParams>,
) -> Result<HttpResponse, SpecializedError> {
    let list = stripe.terminal().readers().list(&query).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /terminal/readers/{id}/cancel-action` — Cancels an in-flight POS reader action via
/// `stripe.terminal().readers().cancel_action()`.
#[post("/terminal/readers/{id}/cancel-action")]
pub async fn cancel_terminal_reader_action(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, SpecializedError> {
    let reader = stripe
        .terminal()
        .readers()
        .cancel_action(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(reader))
}

/// `POST /terminal/connection-tokens` — Mints an ephemeral POS SDK ConnectionToken via
/// `stripe.terminal().connection_tokens().create()`.
#[post("/terminal/connection-tokens")]
pub async fn create_terminal_connection_token(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateConnectionTokenParams>,
) -> Result<HttpResponse, SpecializedError> {
    let token = stripe
        .terminal()
        .connection_tokens()
        .create(&params)
        .await?;
    Ok(HttpResponse::Created().json(token))
}

/// Mounts all Module 7 (Tax, Identity, Issuing, Treasury & Terminal) routes onto an Actix-Web [`web::ServiceConfig`].
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_tax_calculation)
        .service(retrieve_tax_calculation)
        .service(list_tax_calculation_line_items)
        .service(create_tax_transaction_from_calculation)
        .service(create_tax_transaction_reversal)
        .service(retrieve_tax_transaction)
        .service(create_tax_registration)
        .service(retrieve_tax_registration)
        .service(update_tax_registration)
        .service(list_tax_registrations)
        .service(retrieve_tax_settings)
        .service(update_tax_settings)
        .service(create_verification_session)
        .service(retrieve_verification_session)
        .service(update_verification_session)
        .service(cancel_verification_session)
        .service(redact_verification_session)
        .service(retrieve_verification_report)
        .service(create_issuing_cardholder)
        .service(retrieve_issuing_cardholder)
        .service(update_issuing_cardholder)
        .service(list_issuing_cardholders)
        .service(create_issuing_card)
        .service(retrieve_issuing_card)
        .service(update_issuing_card)
        .service(list_issuing_cards)
        .service(retrieve_issuing_authorization)
        .service(update_issuing_authorization)
        .service(approve_issuing_authorization)
        .service(decline_issuing_authorization)
        .service(list_issuing_authorizations)
        .service(retrieve_issuing_transaction)
        .service(update_issuing_transaction)
        .service(list_issuing_transactions)
        .service(create_financial_account)
        .service(retrieve_financial_account)
        .service(update_financial_account)
        .service(list_financial_accounts)
        .service(close_financial_account)
        .service(create_outbound_transfer)
        .service(retrieve_outbound_transfer)
        .service(cancel_outbound_transfer)
        .service(list_outbound_transfers)
        .service(create_outbound_payment)
        .service(retrieve_outbound_payment)
        .service(cancel_outbound_payment)
        .service(list_outbound_payments)
        .service(retrieve_received_credit)
        .service(list_received_credits)
        .service(create_terminal_location)
        .service(retrieve_terminal_location)
        .service(list_terminal_locations)
        .service(create_terminal_reader)
        .service(retrieve_terminal_reader)
        .service(list_terminal_readers)
        .service(cancel_terminal_reader_action)
        .service(create_terminal_connection_token);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let secret_key =
        std::env::var("STRIPE_SECRET_KEY").unwrap_or_else(|_| "sk_test_example_key".to_string());
    let stripe = web::Data::new(StripeClient::new(SecretKey::new(secret_key)));

    println!("Starting Actix-Web Specialized Products example on http://127.0.0.1:8087");
    HttpServer::new(move || {
        App::new()
            .app_data(stripe.clone())
            .configure(configure_routes)
    })
    .bind(("127.0.0.1", 8087))?
    .run()
    .await
}
