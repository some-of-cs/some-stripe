//! # 05: Axum Specialized Products Service Example
//!
//! Stripe Tax, Identity, Issuing, Treasury, and Terminal microservice:
//! - Pre-checkout Tax calculation and transaction commit.
//! - Identity VerificationSessions with biometric passport verification.
//! - Issuing virtual corporate cards & cardholders (with BSA/FinCEN individual legal name).
//! - Treasury commercial financial accounts.
//! - Terminal Smart Reader connection tokens.
//!
//! Run with:
//! ```bash
//! export STRIPE_SECRET_KEY="sk_test_..."
//! cargo run --example 05_axum_specialized_products
//! ```

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use some_stripe::{
    CreateCardParams, CreateCardholderParams, CreateConnectionTokenParams,
    CreateFinancialAccountParams, CreateTaxCalculationParams,
    CreateTaxTransactionFromCalculationParams, CreateVerificationSessionParams,
    CustomerTaxAddress, CustomerTaxDetails, IssuingCardAddress, IssuingCardStatus,
    IssuingCardType, IssuingCardholderBillingParams, IssuingCardholderIndividualParams,
    IssuingCardholderType, StripeClient, TaxLineItemParam, VerificationSessionType,
};
use std::sync::Arc;

/// Shared state holding the lock-free [`StripeClient`].
#[derive(Clone)]
pub struct SpecializedState {
    /// Thread-safe Stripe client.
    pub stripe: Arc<StripeClient>,
}

impl SpecializedState {
    /// Creates a new [`SpecializedState`] from an existing [`StripeClient`].
    pub fn with_client(client: StripeClient) -> Self {
        Self {
            stripe: Arc::new(client),
        }
    }
}

/// Builds the Axum [`Router`] for the Specialized Products microservice.
pub fn build_router(state: SpecializedState) -> Router {
    Router::new()
        .route("/tax/calculate", post(calculate_tax))
        .route("/tax/commit", post(commit_tax))
        .route("/identity/verify", post(create_identity_session))
        .route("/identity/session/{id}", get(get_identity_session))
        .route("/issuing/cardholder", post(create_cardholder))
        .route("/issuing/card", post(create_card))
        .route("/treasury/account", post(create_treasury_account))
        .route("/terminal/token", post(create_terminal_token))
        .with_state(state)
}

/// Entry point for the Specialized Products microservice.
#[allow(dead_code)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Automatically resolves $STRIPE_SECRET_KEY from process env, .env.local, .env.dev, .env, or .env.example
    let state = SpecializedState::with_client(StripeClient::new("$STRIPE_SECRET_KEY"));
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3005").await?;
    tracing::info!("Specialized products service listening on http://0.0.0.0:3005");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Request payload for calculating sales tax.
#[derive(Debug, Deserialize, Serialize)]
pub struct TaxCalcReq {
    pub postal_code: String,
    pub country: String,
    pub amount: i64,
}

/// Calculates real-time sales tax or VAT prior to checkout.
pub async fn calculate_tax(
    State(state): State<SpecializedState>,
    Json(p): Json<TaxCalcReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let address = CustomerTaxAddress {
        city: None,
        country: p.country,
        line1: None,
        line2: None,
        postal_code: Some(p.postal_code),
        state: None,
    };

    let customer_details = CustomerTaxDetails {
        address: Some(address),
        address_source: Some("billing".to_string()),
        ip_address: None,
        tax_ids: None,
        taxability_override: None,
    };

    let line_item = TaxLineItemParam {
        amount: p.amount,
        reference: "line_1".to_string(),
        quantity: Some(1),
        tax_code: None,
        tax_behavior: Some("exclusive".to_string()),
        product: None,
        metadata: None,
    };

    let params = CreateTaxCalculationParams {
        currency: "usd".to_string(),
        customer_details,
        line_items: vec![line_item],
        customer: None,
        tax_date: None,
        shipping_cost: None,
        expand: None,
        idempotency_key: None,
    };

    let calc = state
        .stripe
        .tax()
        .calculations()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(calc)))
}

/// Request payload for committing a Tax Transaction.
#[derive(Debug, Deserialize, Serialize)]
pub struct CommitTaxReq {
    pub calculation_id: String,
}

/// Commits a calculated tax amount into an immutable Tax Transaction.
pub async fn commit_tax(
    State(state): State<SpecializedState>,
    Json(p): Json<CommitTaxReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreateTaxTransactionFromCalculationParams {
        calculation: p.calculation_id,
        reference: "order_tax_tx_1".to_string(),
        posted_at: None,
        metadata: None,
        idempotency_key: None,
    };

    let tx = state
        .stripe
        .tax()
        .transactions()
        .create_from_calculation(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(tx)))
}

/// Creates a biometric Identity VerificationSession.
pub async fn create_identity_session(
    State(state): State<SpecializedState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreateVerificationSessionParams {
        r#type: VerificationSessionType::Document,
        options: None,
        return_url: Some("https://example.com/kyc/complete".to_string()),
        client_reference_id: None,
        metadata: None,
        provided_details: None,
        idempotency_key: None,
    };

    let sess = state
        .stripe
        .identity()
        .verification_sessions()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(sess)))
}

/// Retrieves an Identity VerificationSession by ID.
pub async fn get_identity_session(
    State(state): State<SpecializedState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let sess = state
        .stripe
        .identity()
        .verification_sessions()
        .retrieve(&id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok((StatusCode::OK, Json(sess)))
}

/// Request payload for registering an Issuing Cardholder (requires `first_name` and `last_name` for active card issuance).
#[derive(Debug, Deserialize, Serialize)]
pub struct CardholderReq {
    pub name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub line1: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
}

/// Registers an individual corporate Cardholder for Issuing with individual KYC details.
pub async fn create_cardholder(
    State(state): State<SpecializedState>,
    Json(p): Json<CardholderReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let address = IssuingCardAddress {
        line1: p.line1,
        line2: None,
        city: p.city,
        state: None,
        postal_code: p.postal_code,
        country: p.country,
    };

    let billing = IssuingCardholderBillingParams { address };
    let individual = IssuingCardholderIndividualParams {
        first_name: p.first_name,
        last_name: p.last_name,
        dob: None,
        card_issuing: None,
        verification: None,
    };

    let params = CreateCardholderParams {
        name: p.name,
        billing,
        r#type: IssuingCardholderType::Individual,
        email: Some(p.email),
        phone_number: None,
        status: None,
        spending_controls: None,
        individual: Some(individual),
        company: None,
        metadata: None,
        idempotency_key: None,
        expand: None,
    };

    let holder = state
        .stripe
        .issuing_cardholders()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(holder)))
}

/// Request payload for creating an Issuing Card.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateCardReq {
    pub cardholder: String,
}

/// Issues an active virtual corporate payment card.
pub async fn create_card(
    State(state): State<SpecializedState>,
    Json(p): Json<CreateCardReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreateCardParams {
        cardholder: p.cardholder,
        currency: "usd".to_string(),
        r#type: IssuingCardType::Virtual,
        status: Some(IssuingCardStatus::Active),
        spending_controls: None,
        replacement_for: None,
        replacement_reason: None,
        shipping: None,
        personalization_design: None,
        financial_account: None,
        metadata: None,
        idempotency_key: None,
        expand: None,
    };

    let card = state
        .stripe
        .issuing_cards()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(card)))
}

/// Creates a commercial banking FinancialAccount in Stripe Treasury.
pub async fn create_treasury_account(
    State(state): State<SpecializedState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreateFinancialAccountParams {
        supported_currencies: vec!["usd".to_string()],
        features: None,
        platform_restrictions: None,
        metadata: None,
        idempotency_key: None,
    };

    let fa = state
        .stripe
        .treasury_financial_accounts()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(fa)))
}

/// Mints a secret ConnectionToken for Terminal Smart Readers.
pub async fn create_terminal_token(
    State(state): State<SpecializedState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let params = CreateConnectionTokenParams {
        location: None,
        idempotency_key: None,
    };

    let tok = state
        .stripe
        .terminal()
        .connection_tokens()
        .create(&params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(tok)))
}
