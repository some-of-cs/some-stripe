//! Stripe Connect and Marketplace routes (Module 6).
//!
//! Exposes endpoints for:
//! - Creating connected seller/creator accounts (Standard, Express, Custom).
//! - Generating hosted Account Link onboarding flows.
//! - Creating multi-party transfers and processing reversals.
//! - Inspecting application fees and payouts.

use rocket::serde::json::Json;
use rocket::State;
use some_stripe::{
    Account, AccountLink, AccountLinkType, AccountType, ApplicationFee, CreateAccountLinkParams,
    CreateAccountParams, CreateTransferParams, CreateTransferReversalParams, Transfer,
    TransferReversal,
};

use crate::config::AppConfig;
use crate::models::{
    ApiResponse, CreateAccountLinkRequest, CreateConnectAccountRequest, CreateTransferRequest,
    ErrorResponse,
};

/// Creates a new connected account for a marketplace seller or platform user.
///
/// Route: `POST /api/connect/accounts`
#[rocket::post("/connect/accounts", data = "<req>")]
pub async fn create_connected_account(
    config: &State<AppConfig>,
    req: Json<CreateConnectAccountRequest>,
) -> Result<Json<ApiResponse<Account>>, ErrorResponse> {
    let account_type = match req.account_type.as_deref() {
        Some("custom") => Some(AccountType::Custom),
        Some("standard") => Some(AccountType::Standard),
        _ => Some(AccountType::Express),
    };

    let params = CreateAccountParams {
        account_type,
        country: req.country.clone().or_else(|| Some("US".to_string())),
        email: req.email.clone(),
        business_type: None,
        capabilities: None,
        company: None,
        individual: None,
        metadata: None,
        tos_acceptance: None,
        idempotency_key: None,
    };

    let account = config
        .client
        .accounts()
        .create(&params)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to create connected account: {e}")))?;

    Ok(Json(ApiResponse::new(account)))
}

/// Retrieves a connected account's details, requirements, and payout status.
///
/// Route: `GET /api/connect/accounts/<id>`
#[rocket::get("/connect/accounts/<id>")]
pub async fn get_connected_account(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<Account>>, ErrorResponse> {
    let account = config
        .client
        .accounts()
        .retrieve_by_id(id)
        .await
        .map_err(|e| ErrorResponse::not_found(format!("Connected account '{id}' not found: {e}")))?;

    Ok(Json(ApiResponse::new(account)))
}

/// Creates a Stripe-hosted Account Link onboarding or update URL.
///
/// Route: `POST /api/connect/account-links`
#[rocket::post("/connect/account-links", data = "<req>")]
pub async fn create_account_link(
    config: &State<AppConfig>,
    req: Json<CreateAccountLinkRequest>,
) -> Result<Json<ApiResponse<AccountLink>>, ErrorResponse> {
    let type_ = match req.r#type.as_str() {
        "account_update" => AccountLinkType::AccountUpdate,
        _ => AccountLinkType::AccountOnboarding,
    };

    let params = CreateAccountLinkParams {
        account: req.account.clone(),
        refresh_url: Some(req.refresh_url.clone()),
        return_url: Some(req.return_url.clone()),
        type_,
        collect: None,
    };

    let link = config
        .client
        .account_links()
        .create(&params)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to create account link: {e}")))?;

    Ok(Json(ApiResponse::new(link)))
}

/// Dispatches a funds transfer to a connected account.
///
/// Route: `POST /api/connect/transfers`
#[rocket::post("/connect/transfers", data = "<req>")]
pub async fn create_transfer(
    config: &State<AppConfig>,
    req: Json<CreateTransferRequest>,
) -> Result<Json<ApiResponse<Transfer>>, ErrorResponse> {
    let mut params = CreateTransferParams::new(req.amount, &req.currency, &req.destination);
    if let Some(ref desc) = req.description {
        params = params.with_description(desc);
    }

    params
        .validate()
        .map_err(|e| ErrorResponse::bad_request(format!("Transfer parameter validation failed: {e}")))?;

    let transfer = config
        .client
        .transfers()
        .create(&params)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to create transfer: {e}")))?;

    Ok(Json(ApiResponse::new(transfer)))
}

/// Reverses a transfer previously dispatched to a connected account.
///
/// Route: `POST /api/connect/transfers/<id>/reversals`
#[rocket::post("/connect/transfers/<id>/reversals")]
pub async fn reverse_transfer(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<TransferReversal>>, ErrorResponse> {
    let params = CreateTransferReversalParams::new().with_refund_application_fee(true);

    let reversal = config
        .client
        .transfers()
        .reverse(id, Some(&params))
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to reverse transfer '{id}': {e}")))?;

    Ok(Json(ApiResponse::new(reversal)))
}

/// Retrieves an application fee record collected by the platform.
///
/// Route: `GET /api/connect/application-fees/<id>`
#[rocket::get("/connect/application-fees/<id>")]
pub async fn get_application_fee(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<ApplicationFee>>, ErrorResponse> {
    let fee = config
        .client
        .application_fees()
        .retrieve(id)
        .await
        .map_err(|e| ErrorResponse::not_found(format!("Application fee '{id}' not found: {e}")))?;

    Ok(Json(ApiResponse::new(fee)))
}
