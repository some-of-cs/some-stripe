//! # 04: Axum Connect Marketplace Service Example
//!
//! Multi-tenant Connect Marketplace service:
//! - Connected Account creation with Express / Custom onboarding.
//! - AccountLink generation for hosted onboarding flows.
//! - KYC Persons management.
//! - Split payment transfers to connected accounts.
//! - Marketplace application fees and merchant payouts.
//!
//! Run with:
//! ```bash
//! export STRIPE_SECRET_KEY="sk_test_..."
//! cargo run --example 04_axum_connect_marketplace
//! ```

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use some_stripe::{
    AccountLinkType, AccountType, CreateAccountLinkParams, CreateAccountParams, CreatePayoutParams,
    CreatePersonParams, CreateTransferParams, StripeClient,
};
use std::sync::Arc;

/// Shared state holding the lock-free [`StripeClient`].
#[derive(Clone)]
pub struct ConnectState {
    /// Thread-safe Stripe client.
    pub stripe: Arc<StripeClient>,
}

impl ConnectState {
    /// Creates a new [`ConnectState`] from an existing [`StripeClient`].
    pub fn with_client(client: StripeClient) -> Self {
        Self {
            stripe: Arc::new(client),
        }
    }
}

/// Builds the Axum [`Router`] for the Connect Marketplace microservice.
pub fn build_router(state: ConnectState) -> Router {
    Router::new()
        .route("/connect/account", post(create_merchant_account))
        .route("/connect/account/{id}/link", post(create_onboarding_link))
        .route("/connect/account/{id}/person", post(add_kyc_person))
        .route("/connect/transfer", post(transfer_split))
        .route("/connect/payout", post(trigger_payout))
        .with_state(state)
}

/// Entry point for the Connect Marketplace microservice.
#[allow(dead_code)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Automatically resolves $STRIPE_SECRET_KEY from process env, .env.local, .env.dev, .env, or .env.example
    let state = ConnectState::with_client(StripeClient::new("$STRIPE_SECRET_KEY"));
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3004").await?;
    tracing::info!("Connect Marketplace service listening on http://0.0.0.0:3004");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Request payload for creating a Connected Merchant Account.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateMerchantReq {
    pub email: String,
    pub country: String,
}

/// Creates a new Express Connected Account for a marketplace merchant.
pub async fn create_merchant_account(
    State(state): State<ConnectState>,
    Json(p): Json<CreateMerchantReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreateAccountParams {
        account_type: Some(AccountType::Express),
        country: Some(p.country),
        email: Some(p.email),
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

    let acct = state
        .stripe
        .accounts()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(acct)))
}

/// Request payload for creating an onboarding AccountLink.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateLinkReq {
    pub refresh_url: String,
    pub return_url: String,
}

/// Creates a hosted AccountLink for merchant KYC onboarding.
pub async fn create_onboarding_link(
    State(state): State<ConnectState>,
    Path(id): Path<String>,
    Json(p): Json<CreateLinkReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreateAccountLinkParams {
        account: id,
        refresh_url: Some(p.refresh_url),
        return_url: Some(p.return_url),
        type_: AccountLinkType::AccountOnboarding,
        collection_options: None,
        expand: None,
        idempotency_key: None,
    };

    let link = state
        .stripe
        .account_links()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(link)))
}

/// Request payload for adding a KYC representative Person.
#[derive(Debug, Deserialize, Serialize)]
pub struct AddPersonReq {
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
}

/// Adds a KYC representative Person to a Connected Account.
pub async fn add_kyc_person(
    State(state): State<ConnectState>,
    Path(id): Path<String>,
    Json(p): Json<AddPersonReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreatePersonParams {
        first_name: Some(p.first_name),
        last_name: Some(p.last_name),
        maiden_name: None,
        email: p.email,
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
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(person)))
}

/// Request payload for creating a split payment Transfer.
#[derive(Debug, Deserialize, Serialize)]
pub struct TransferReq {
    pub destination: String,
    pub amount: i64,
    pub currency: String,
}

/// Transfers split funds to a Connected Account.
pub async fn transfer_split(
    State(state): State<ConnectState>,
    Json(p): Json<TransferReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreateTransferParams {
        amount: p.amount,
        currency: p.currency.to_lowercase(),
        destination: p.destination,
        description: Some("Marketplace order payment split".to_string()),
        metadata: None,
        source_transaction: None,
        source_type: None,
        transfer_group: None,
        idempotency_key: None,
    };

    let transfer = state
        .stripe
        .transfers()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(transfer)))
}

/// Request payload for triggering a Payout.
#[derive(Debug, Deserialize, Serialize)]
pub struct PayoutReq {
    pub amount: i64,
    pub currency: String,
}

/// Triggers a Payout to the connected bank account or debit card.
pub async fn trigger_payout(
    State(state): State<ConnectState>,
    Json(p): Json<PayoutReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreatePayoutParams {
        amount: p.amount,
        currency: p.currency.to_lowercase(),
        description: Some("Merchant earnings payout".to_string()),
        destination: None,
        metadata: None,
        method: None,
        source_type: None,
        statement_descriptor: None,
        idempotency_key: None,
    };

    let payout = state
        .stripe
        .payouts()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(payout)))
}
