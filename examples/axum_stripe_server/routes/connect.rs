//! Connect & Marketplaces routes (Module 6).
//!
//! # Features
//! - Connected Accounts (Standard, Express, Custom) with `Stripe-Account` multi-tenancy.
//! - KYC Persons identity onboarding.
//! - Account Links (hosted Connect onboarding) & Account Sessions (embedded components).
//! - Multi-party split Transfers, Application Fees, and merchant Payouts.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use some_stripe::{
    AccountLinkType, AccountSessionComponents, AccountType, CreateAccountLinkParams,
    CreateAccountParams, CreateAccountSessionParams, CreatePayoutParams, CreatePersonParams,
    CreateTransferParams,
};

use crate::state::{AppError, AppState};

/// Creates the Connect marketplace router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/connect/accounts", post(create_account))
        .route("/connect/accounts/{id}", get(get_account))
        .route("/connect/accounts/{id}/links", post(create_account_link))
        .route("/connect/accounts/{id}/sessions", post(create_account_session))
        .route("/connect/accounts/{id}/persons", post(create_person))
        .route("/connect/transfers", post(create_transfer))
        .route("/connect/application-fees", get(list_application_fees))
        .route("/connect/payouts", post(create_payout))
}

/// Request body for creating a Connected Account.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateAccountRequest {
    pub account_type: Option<String>,
    pub country: Option<String>,
    pub email: Option<String>,
}

/// Create a new connected account.
pub async fn create_account(
    State(state): State<AppState>,
    Json(payload): Json<CreateAccountRequest>,
) -> Result<impl IntoResponse, AppError> {
    let act_type = match payload.account_type.as_deref() {
        Some("express") => Some(AccountType::Express),
        Some("standard") => Some(AccountType::Standard),
        _ => Some(AccountType::Custom),
    };

    let params = CreateAccountParams {
        account_type: act_type,
        country: payload.country.or_else(|| Some("US".to_string())),
        email: payload.email,
        business_type: None,
        default_currency: None,
        capabilities: None,
        business_profile: None,
        company: None,
        individual: None,
        settings: None,
        tos_acceptance: None,
        metadata: None,
        idempotency_key: None,
    };

    let account = state.stripe.accounts().create(&params).await?;
    Ok((StatusCode::CREATED, Json(account)))
}

/// Retrieve connected account details.
pub async fn get_account(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let account = state.stripe.accounts().with_account(&id).retrieve().await?;
    Ok((StatusCode::OK, Json(account)))
}

/// Request body for generating an Account Link.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateAccountLinkRequest {
    pub refresh_url: String,
    pub return_url: String,
}

/// Create an Account Link for hosted onboarding.
pub async fn create_account_link(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<CreateAccountLinkRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateAccountLinkParams {
        account: id,
        refresh_url: Some(payload.refresh_url),
        return_url: Some(payload.return_url),
        type_: AccountLinkType::AccountOnboarding,
        collection_options: None,
        expand: None,
        idempotency_key: None,
    };

    let link = state.stripe.account_links().create(&params).await?;
    Ok((StatusCode::CREATED, Json(link)))
}

/// Generate an Account Session for embedded Connect components.
pub async fn create_account_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateAccountSessionParams {
        account: id,
        components: AccountSessionComponents::default(),
        expand: None,
        idempotency_key: None,
    };

    let session = state.stripe.account_sessions().create(&params).await?;
    Ok((StatusCode::CREATED, Json(session)))
}

/// Request body for adding a KYC representative Person.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreatePersonRequest {
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
}

/// Add a KYC representative to a connected account.
pub async fn create_person(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<CreatePersonRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreatePersonParams {
        first_name: Some(payload.first_name),
        last_name: Some(payload.last_name),
        maiden_name: None,
        email: payload.email,
        phone: None,
        dob: None,
        address: None,
        registered_address: None,
        relationship: None,
        verification: None,
        documents: None,
        id_number: None,
        id_number_secondary: None,
        ssn_last_4: None,
        nationality: None,
        political_exposure: None,
        metadata: None,
        idempotency_key: None,
    };

    let person = state
        .stripe
        .persons()
        .with_account(&id)
        .create(&id, &params)
        .await?;
    Ok((StatusCode::CREATED, Json(person)))
}

/// Request body for creating a Transfer.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateTransferRequest {
    pub amount: i64,
    pub currency: String,
    pub destination: String,
    pub description: Option<String>,
}

/// Transfer funds to a connected account.
pub async fn create_transfer(
    State(state): State<AppState>,
    Json(payload): Json<CreateTransferRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateTransferParams {
        amount: payload.amount,
        currency: payload.currency.to_lowercase(),
        destination: payload.destination,
        description: payload.description,
        metadata: None,
        source_transaction: None,
        source_type: None,
        transfer_group: None,
        idempotency_key: None,
    };

    let transfer = state.stripe.transfers().create(&params).await?;
    Ok((StatusCode::CREATED, Json(transfer)))
}

/// List application fees collected by the platform.
pub async fn list_application_fees(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let fees = state.stripe.application_fees().list(None).await?;
    Ok((StatusCode::OK, Json(fees)))
}

/// Request body for creating a Payout.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreatePayoutRequest {
    pub amount: i64,
    pub currency: String,
    pub description: Option<String>,
}

/// Create a payout to a bank account or debit card.
pub async fn create_payout(
    State(state): State<AppState>,
    Json(payload): Json<CreatePayoutRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreatePayoutParams {
        amount: payload.amount,
        currency: payload.currency.to_lowercase(),
        description: payload.description,
        destination: None,
        metadata: None,
        method: None,
        source_type: None,
        statement_descriptor: None,
        idempotency_key: None,
    };

    let payout = state.stripe.payouts().create(&params).await?;
    Ok((StatusCode::CREATED, Json(payout)))
}
