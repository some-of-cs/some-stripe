//! # Actix-Web Example 6: Connect & Multi-Party Marketplaces (Module 6)
//!
//! Demonstrates how to integrate **Module 6 (Connect & Marketplaces)** of `some-stripe`
//! inside an `actix-web` application.
//!
//! ## Services Covered
//! - `stripe.accounts()`: `create`, `retrieve` (current platform account), `retrieve_by_id`,
//!   `update`, `delete`, `reject`, `list`, and `.with_account(acct_id)` multi-tenant header scoping
//! - `stripe.persons()`: `create`, `retrieve`, `update`, `delete`, and `list` (KYC / FinCEN CDD)
//! - `stripe.account_links()` & `stripe.account_sessions()`: Hosted onboarding redirect links
//!   and embedded Connect UI component sessions
//! - `stripe.transfers()`: `create`, `retrieve`, `update`, `reverse`, `list`, and `list_reversals`
//! - `stripe.application_fees()`: `retrieve`, `list`, `create_refund`, `retrieve_refund`, and
//!   `list_refunds`
//! - `stripe.payouts()`: `create`, `retrieve`, `update`, `cancel`, `reverse`, and `list`
//!
//! ## Run the Example
//! ```bash
//! STRIPE_SECRET_KEY=sk_test_123 cargo run --example actix_connect_marketplace
//! ```

use actix_web::{App, HttpResponse, HttpServer, ResponseError, delete, get, post, web};
use some_stripe::{
    CreateAccountLinkParams, CreateAccountParams, CreateAccountSessionParams,
    CreateFeeRefundParams, CreatePayoutParams, CreatePersonParams, CreateTransferParams,
    CreateTransferReversalParams, ListAccountsParams, ListApplicationFeesParams,
    ListFeeRefundsParams, ListPayoutsParams, ListPersonsParams, ListTransferReversalsParams,
    ListTransfersParams, RejectAccountParams, SecretKey, StripeClient, StripeError,
    UpdateAccountParams, UpdatePayoutParams, UpdatePersonParams, UpdateTransferParams,
};
use std::fmt;

/// Error wrapper converting `StripeError` into structured Actix-Web HTTP responses.
#[derive(Debug)]
pub struct ConnectError(pub StripeError);

impl fmt::Display for ConnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<StripeError> for ConnectError {
    fn from(err: StripeError) -> Self {
        Self(err)
    }
}

impl ResponseError for ConnectError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::BadGateway().json(serde_json::json!({
            "error": self.0.to_string(),
        }))
    }
}

/// `POST /connect/accounts` — Creates a connected account via `stripe.accounts().create()`.
#[post("/connect/accounts")]
pub async fn create_account(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateAccountParams>,
) -> Result<HttpResponse, ConnectError> {
    let account = stripe.accounts().create(&params).await?;
    Ok(HttpResponse::Created().json(account))
}

/// `GET /connect/accounts/me` — Retrieves the platform's own account via `stripe.accounts().retrieve()`.
#[get("/connect/accounts/me")]
pub async fn retrieve_current_account(
    stripe: web::Data<StripeClient>,
) -> Result<HttpResponse, ConnectError> {
    let account = stripe.accounts().retrieve().await?;
    Ok(HttpResponse::Ok().json(account))
}

/// `GET /connect/accounts/{id}` — Retrieves a connected account by ID via `stripe.accounts().retrieve_by_id()`.
#[get("/connect/accounts/{id}")]
pub async fn retrieve_account_by_id(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, ConnectError> {
    let account = stripe.accounts().retrieve_by_id(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(account))
}

/// `POST /connect/accounts/{id}` — Updates a connected account via `stripe.accounts().update()`.
#[post("/connect/accounts/{id}")]
pub async fn update_account(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateAccountParams>,
) -> Result<HttpResponse, ConnectError> {
    let account = stripe.accounts().update(id.as_str(), &params).await?;
    Ok(HttpResponse::Ok().json(account))
}

/// `DELETE /connect/accounts/{id}` — Deletes a connected account via `stripe.accounts().delete()`.
#[delete("/connect/accounts/{id}")]
pub async fn delete_account(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, ConnectError> {
    let deleted = stripe.accounts().delete(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(deleted))
}

/// `POST /connect/accounts/{id}/reject` — Rejects a custom/express account for fraud or terms
/// violation via `stripe.accounts().reject()`.
#[post("/connect/accounts/{id}/reject")]
pub async fn reject_account(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<RejectAccountParams>,
) -> Result<HttpResponse, ConnectError> {
    let account = stripe.accounts().reject(id.as_str(), &params).await?;
    Ok(HttpResponse::Ok().json(account))
}

/// `GET /connect/accounts` — Lists connected accounts via `stripe.accounts().list()`.
#[get("/connect/accounts")]
pub async fn list_accounts(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListAccountsParams>,
) -> Result<HttpResponse, ConnectError> {
    let list = stripe.accounts().list(&query).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /connect/account-links` — Creates a hosted Connect onboarding link via
/// `stripe.account_links().create()`.
#[post("/connect/account-links")]
pub async fn create_account_link(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateAccountLinkParams>,
) -> Result<HttpResponse, ConnectError> {
    let link = stripe.account_links().create(&params).await?;
    Ok(HttpResponse::Created().json(link))
}

/// `POST /connect/account-sessions` — Creates an embedded Connect component session via
/// `stripe.account_sessions().create()`.
#[post("/connect/account-sessions")]
pub async fn create_account_session(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateAccountSessionParams>,
) -> Result<HttpResponse, ConnectError> {
    let session = stripe.account_sessions().create(&params).await?;
    Ok(HttpResponse::Created().json(session))
}

/// `POST /connect/accounts/{account_id}/persons` — Creates a KYC Person for a connected account
/// via `stripe.persons().create()`.
#[post("/connect/accounts/{account_id}/persons")]
pub async fn create_person(
    stripe: web::Data<StripeClient>,
    account_id: web::Path<String>,
    params: web::Json<CreatePersonParams>,
) -> Result<HttpResponse, ConnectError> {
    let person = stripe
        .persons()
        .create(account_id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Created().json(person))
}

/// `GET /connect/accounts/{account_id}/persons/{person_id}` — Retrieves a KYC Person via
/// `stripe.persons().retrieve()`.
#[get("/connect/accounts/{account_id}/persons/{person_id}")]
pub async fn retrieve_person(
    stripe: web::Data<StripeClient>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ConnectError> {
    let (account_id, person_id) = path.into_inner();
    let person = stripe.persons().retrieve(&account_id, &person_id).await?;
    Ok(HttpResponse::Ok().json(person))
}

/// `POST /connect/accounts/{account_id}/persons/{person_id}` — Updates a KYC Person via
/// `stripe.persons().update()`.
#[post("/connect/accounts/{account_id}/persons/{person_id}")]
pub async fn update_person(
    stripe: web::Data<StripeClient>,
    path: web::Path<(String, String)>,
    params: web::Json<UpdatePersonParams>,
) -> Result<HttpResponse, ConnectError> {
    let (account_id, person_id) = path.into_inner();
    let person = stripe
        .persons()
        .update(&account_id, &person_id, &params)
        .await?;
    Ok(HttpResponse::Ok().json(person))
}

/// `DELETE /connect/accounts/{account_id}/persons/{person_id}` — Deletes a KYC Person via
/// `stripe.persons().delete()`.
#[delete("/connect/accounts/{account_id}/persons/{person_id}")]
pub async fn delete_person(
    stripe: web::Data<StripeClient>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ConnectError> {
    let (account_id, person_id) = path.into_inner();
    let deleted = stripe.persons().delete(&account_id, &person_id).await?;
    Ok(HttpResponse::Ok().json(deleted))
}

/// `GET /connect/accounts/{account_id}/persons` — Lists KYC Persons for a connected account
/// via `stripe.persons().list()`.
#[get("/connect/accounts/{account_id}/persons")]
pub async fn list_persons(
    stripe: web::Data<StripeClient>,
    account_id: web::Path<String>,
    query: web::Query<ListPersonsParams>,
) -> Result<HttpResponse, ConnectError> {
    let list = stripe.persons().list(account_id.as_str(), &query).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /connect/transfers` — Creates a marketplace transfer to a connected account via
/// `stripe.transfers().create()`.
#[post("/connect/transfers")]
pub async fn create_transfer(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateTransferParams>,
) -> Result<HttpResponse, ConnectError> {
    let transfer = stripe.transfers().create(&params).await?;
    Ok(HttpResponse::Created().json(transfer))
}

/// `GET /connect/transfers/{id}` — Retrieves a transfer via `stripe.transfers().retrieve()`.
#[get("/connect/transfers/{id}")]
pub async fn retrieve_transfer(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, ConnectError> {
    let transfer = stripe.transfers().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(transfer))
}

/// `POST /connect/transfers/{id}` — Updates a transfer via `stripe.transfers().update()`.
#[post("/connect/transfers/{id}")]
pub async fn update_transfer(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateTransferParams>,
) -> Result<HttpResponse, ConnectError> {
    let transfer = stripe.transfers().update(id.as_str(), &params).await?;
    Ok(HttpResponse::Ok().json(transfer))
}

/// `POST /connect/transfers/{id}/reverse` — Reverses a transfer via `stripe.transfers().reverse()`.
#[post("/connect/transfers/{id}/reverse")]
pub async fn reverse_transfer(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: Option<web::Json<CreateTransferReversalParams>>,
) -> Result<HttpResponse, ConnectError> {
    let reversal = stripe
        .transfers()
        .reverse(id.as_str(), params.as_deref())
        .await?;
    Ok(HttpResponse::Created().json(reversal))
}

/// `GET /connect/transfers` — Lists transfers via `stripe.transfers().list()`.
#[get("/connect/transfers")]
pub async fn list_transfers(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListTransfersParams>,
) -> Result<HttpResponse, ConnectError> {
    let list = stripe.transfers().list(Some(&query)).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `GET /connect/transfers/{id}/reversals` — Lists transfer reversals via
/// `stripe.transfers().list_reversals()`.
#[get("/connect/transfers/{id}/reversals")]
pub async fn list_transfer_reversals(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    query: web::Query<ListTransferReversalsParams>,
) -> Result<HttpResponse, ConnectError> {
    let list = stripe
        .transfers()
        .list_reversals(id.as_str(), Some(&query))
        .await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `GET /connect/application-fees/{id}` — Retrieves a platform application fee via
/// `stripe.application_fees().retrieve()`.
#[get("/connect/application-fees/{id}")]
pub async fn retrieve_application_fee(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, ConnectError> {
    let fee = stripe.application_fees().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(fee))
}

/// `GET /connect/application-fees` — Lists platform application fees via
/// `stripe.application_fees().list()`.
#[get("/connect/application-fees")]
pub async fn list_application_fees(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListApplicationFeesParams>,
) -> Result<HttpResponse, ConnectError> {
    let list = stripe.application_fees().list(Some(&query)).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /connect/application-fees/{id}/refunds` — Refunds an application fee via
/// `stripe.application_fees().create_refund()`.
#[post("/connect/application-fees/{id}/refunds")]
pub async fn create_application_fee_refund(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: Option<web::Json<CreateFeeRefundParams>>,
) -> Result<HttpResponse, ConnectError> {
    let refund = stripe
        .application_fees()
        .create_refund(id.as_str(), params.as_deref())
        .await?;
    Ok(HttpResponse::Created().json(refund))
}

/// `GET /connect/application-fees/{id}/refunds/{refund_id}` — Retrieves an application fee refund
/// via `stripe.application_fees().retrieve_refund()`.
#[get("/connect/application-fees/{id}/refunds/{refund_id}")]
pub async fn retrieve_application_fee_refund(
    stripe: web::Data<StripeClient>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ConnectError> {
    let (fee_id, refund_id) = path.into_inner();
    let refund = stripe
        .application_fees()
        .retrieve_refund(&fee_id, &refund_id)
        .await?;
    Ok(HttpResponse::Ok().json(refund))
}

/// `GET /connect/application-fees/{id}/refunds` — Lists application fee refunds via
/// `stripe.application_fees().list_refunds()`.
#[get("/connect/application-fees/{id}/refunds")]
pub async fn list_application_fee_refunds(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    query: web::Query<ListFeeRefundsParams>,
) -> Result<HttpResponse, ConnectError> {
    let list = stripe
        .application_fees()
        .list_refunds(id.as_str(), Some(&query))
        .await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /connect/payouts` — Creates a payout (optionally scoped to a connected account via
/// `.with_account()`) using `stripe.payouts().create()`.
#[post("/connect/payouts")]
pub async fn create_payout(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreatePayoutParams>,
) -> Result<HttpResponse, ConnectError> {
    let payout = stripe.payouts().create(&params).await?;
    Ok(HttpResponse::Created().json(payout))
}

/// `GET /connect/payouts/{id}` — Retrieves a payout via `stripe.payouts().retrieve()`.
#[get("/connect/payouts/{id}")]
pub async fn retrieve_payout(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, ConnectError> {
    let payout = stripe.payouts().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(payout))
}

/// `POST /connect/payouts/{id}` — Updates a payout via `stripe.payouts().update()`.
#[post("/connect/payouts/{id}")]
pub async fn update_payout(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdatePayoutParams>,
) -> Result<HttpResponse, ConnectError> {
    let payout = stripe.payouts().update(id.as_str(), &params).await?;
    Ok(HttpResponse::Ok().json(payout))
}

/// `POST /connect/payouts/{id}/cancel` — Cancels a pending payout via `stripe.payouts().cancel()`.
#[post("/connect/payouts/{id}/cancel")]
pub async fn cancel_payout(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, ConnectError> {
    let payout = stripe.payouts().cancel(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(payout))
}

/// `POST /connect/payouts/{id}/reverse` — Reverses a payout via `stripe.payouts().reverse()`.
#[post("/connect/payouts/{id}/reverse")]
pub async fn reverse_payout(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, ConnectError> {
    let payout = stripe.payouts().reverse(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(payout))
}

/// `GET /connect/payouts` — Lists payouts via `stripe.payouts().list()`.
#[get("/connect/payouts")]
pub async fn list_payouts(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListPayoutsParams>,
) -> Result<HttpResponse, ConnectError> {
    let list = stripe.payouts().list(Some(&query)).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// Mounts all Module 6 (Connect & Marketplaces) routes onto an Actix-Web [`web::ServiceConfig`].
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_account)
        .service(retrieve_current_account)
        .service(retrieve_account_by_id)
        .service(update_account)
        .service(delete_account)
        .service(reject_account)
        .service(list_accounts)
        .service(create_account_link)
        .service(create_account_session)
        .service(create_person)
        .service(retrieve_person)
        .service(update_person)
        .service(delete_person)
        .service(list_persons)
        .service(create_transfer)
        .service(retrieve_transfer)
        .service(update_transfer)
        .service(reverse_transfer)
        .service(list_transfers)
        .service(list_transfer_reversals)
        .service(retrieve_application_fee)
        .service(list_application_fees)
        .service(create_application_fee_refund)
        .service(retrieve_application_fee_refund)
        .service(list_application_fee_refunds)
        .service(create_payout)
        .service(retrieve_payout)
        .service(update_payout)
        .service(cancel_payout)
        .service(reverse_payout)
        .service(list_payouts);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let secret_key =
        std::env::var("STRIPE_SECRET_KEY").unwrap_or_else(|_| "sk_test_example_key".to_string());
    let stripe = web::Data::new(StripeClient::new(SecretKey::new(secret_key)));

    println!("Starting Actix-Web Connect Marketplace example on http://127.0.0.1:8086");
    HttpServer::new(move || {
        App::new()
            .app_data(stripe.clone())
            .configure(configure_routes)
    })
    .bind(("127.0.0.1", 8086))?
    .run()
    .await
}
