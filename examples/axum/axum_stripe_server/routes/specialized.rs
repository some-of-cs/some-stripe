//! Specialized Product Namespaces routes (Module 7).
//!
//! # Features
//! - **Stripe Tax**: Real-time sales tax calculation & post-payment transaction commit.
//! - **Identity**: VerificationSessions with biometric/ID checks and masked PII.
//! - **Issuing**: Virtual/physical cards, cardholders (with BSA/FinCEN individual legal name), and real-time authorization approvals.
//! - **Treasury**: Commercial banking financial accounts and ACH/wire transfers.
//! - **Terminal**: Smart Reader pairing and connection token issuance.

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
    CreateFinancialAccountParams, CreateOutboundTransferParams, CreateTaxCalculationParams,
    CreateTaxTransactionFromCalculationParams, CreateVerificationSessionParams,
    CustomerTaxAddress, CustomerTaxDetails, IssuingCardAddress, IssuingCardStatus,
    IssuingCardType, IssuingCardholderBillingParams, IssuingCardholderIndividualParams,
    IssuingCardholderType, TaxLineItemParam, VerificationSessionType,
};

use crate::state::{AppError, AppState};

/// Creates the specialized products router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/specialized/tax/calculations", post(create_tax_calculation))
        .route("/specialized/tax/transactions/commit", post(commit_tax_transaction))
        .route("/specialized/identity/verification-sessions", post(create_identity_session))
        .route("/specialized/identity/verification-sessions/{id}", get(get_identity_session))
        .route("/specialized/issuing/cardholders", post(create_cardholder))
        .route("/specialized/issuing/cards", post(create_card))
        .route("/specialized/treasury/financial-accounts", post(create_financial_account))
        .route("/specialized/treasury/transfers", post(create_treasury_transfer))
        .route("/specialized/terminal/connection-tokens", post(create_terminal_connection_token))
}

/// Request body for calculating sales tax.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateTaxCalcRequest {
    pub currency: String,
    pub postal_code: String,
    pub country: String,
    pub amount: i64,
    pub reference: String,
}

/// Calculate sales tax and VAT before checkout.
pub async fn create_tax_calculation(
    State(state): State<AppState>,
    Json(payload): Json<CreateTaxCalcRequest>,
) -> Result<impl IntoResponse, AppError> {
    let address = CustomerTaxAddress {
        city: None,
        country: payload.country,
        line1: None,
        line2: None,
        postal_code: Some(payload.postal_code),
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
        amount: payload.amount,
        reference: payload.reference,
        quantity: Some(1),
        tax_code: None,
        tax_behavior: Some("exclusive".to_string()),
        product: None,
        metadata: None,
    };

    let params = CreateTaxCalculationParams {
        currency: payload.currency.to_lowercase(),
        customer_details,
        line_items: vec![line_item],
        customer: None,
        tax_date: None,
        shipping_cost: None,
        expand: None,
        idempotency_key: None,
    };

    let calc = state.stripe.tax().calculations().create(&params).await?;
    Ok((StatusCode::CREATED, Json(calc)))
}

/// Request body for committing a tax transaction.
#[derive(Debug, Deserialize, Serialize)]
pub struct CommitTaxTxRequest {
    pub calculation_id: String,
    pub reference: String,
}

/// Commit a calculated tax transaction for statutory audit logs.
pub async fn commit_tax_transaction(
    State(state): State<AppState>,
    Json(payload): Json<CommitTaxTxRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateTaxTransactionFromCalculationParams {
        calculation: payload.calculation_id,
        reference: payload.reference,
        posted_at: None,
        metadata: None,
        idempotency_key: None,
    };

    let tx = state.stripe.tax().transactions().create_from_calculation(&params).await?;
    Ok((StatusCode::CREATED, Json(tx)))
}

/// Request body for creating an Identity verification session.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateIdentityRequest {
    pub return_url: Option<String>,
}

/// Create an Identity verification session for passport/ID biometric verification.
pub async fn create_identity_session(
    State(state): State<AppState>,
    Json(payload): Json<CreateIdentityRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateVerificationSessionParams {
        r#type: VerificationSessionType::Document,
        options: None,
        return_url: payload.return_url,
        client_reference_id: None,
        metadata: None,
        provided_details: None,
        idempotency_key: None,
    };

    let session = state.stripe.identity().verification_sessions().create(&params).await?;
    Ok((StatusCode::CREATED, Json(session)))
}

/// Retrieve an Identity verification session (sensitive PII masked).
pub async fn get_identity_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let session = state.stripe.identity().verification_sessions().retrieve(&id).await?;
    Ok((StatusCode::OK, Json(session)))
}

/// Request body for creating an Issuing Cardholder (requires `first_name` and `last_name` for active card issuance).
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateCardholderRequest {
    pub name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub line1: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
}

/// Register a corporate cardholder for Issuing with individual KYC details.
pub async fn create_cardholder(
    State(state): State<AppState>,
    Json(payload): Json<CreateCardholderRequest>,
) -> Result<impl IntoResponse, AppError> {
    let address = IssuingCardAddress {
        line1: payload.line1,
        line2: None,
        city: payload.city,
        state: None,
        postal_code: payload.postal_code,
        country: payload.country,
    };

    let billing = IssuingCardholderBillingParams { address };
    let individual = IssuingCardholderIndividualParams {
        first_name: payload.first_name,
        last_name: payload.last_name,
        dob: None,
        card_issuing: None,
        verification: None,
    };

    let params = CreateCardholderParams {
        name: payload.name,
        billing,
        r#type: IssuingCardholderType::Individual,
        email: Some(payload.email),
        phone_number: None,
        status: None,
        spending_controls: None,
        individual: Some(individual),
        company: None,
        metadata: None,
        idempotency_key: None,
        expand: None,
    };

    let cardholder = state.stripe.issuing_cardholders().create(&params).await?;
    Ok((StatusCode::CREATED, Json(cardholder)))
}

/// Request body for creating an Issuing Card.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateCardRequest {
    pub cardholder: String,
    pub currency: String,
}

/// Issue a virtual commercial payment card.
pub async fn create_card(
    State(state): State<AppState>,
    Json(payload): Json<CreateCardRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateCardParams {
        cardholder: payload.cardholder,
        currency: payload.currency.to_lowercase(),
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

    let card = state.stripe.issuing_cards().create(&params).await?;
    Ok((StatusCode::CREATED, Json(card)))
}

/// Create a commercial banking FinancialAccount.
pub async fn create_financial_account(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateFinancialAccountParams {
        supported_currencies: vec!["usd".to_string()],
        features: None,
        platform_restrictions: None,
        metadata: None,
        idempotency_key: None,
    };

    let fa = state.stripe.treasury_financial_accounts().create(&params).await?;
    Ok((StatusCode::CREATED, Json(fa)))
}

/// Request body for creating a Treasury Outbound Transfer.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateTreasuryTransferRequest {
    pub financial_account: String,
    pub amount: i64,
    pub currency: String,
    pub destination_payment_method: String,
}

/// Initiate a Treasury outbound ACH/wire transfer.
pub async fn create_treasury_transfer(
    State(state): State<AppState>,
    Json(payload): Json<CreateTreasuryTransferRequest>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateOutboundTransferParams {
        financial_account: payload.financial_account,
        amount: payload.amount,
        currency: payload.currency.to_lowercase(),
        destination_payment_method: Some(payload.destination_payment_method),
        destination_payment_method_options: None,
        description: None,
        statement_descriptor: None,
        metadata: None,
        idempotency_key: None,
    };

    let transfer = state.stripe.treasury_transfers().outbound_transfers().create(&params).await?;
    Ok((StatusCode::CREATED, Json(transfer)))
}

/// Mint a secret connection token for in-person Terminal Smart Readers.
pub async fn create_terminal_connection_token(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let params = CreateConnectionTokenParams {
        location: None,
        idempotency_key: None,
    };

    let token = state.stripe.terminal().connection_tokens().create(&params).await?;
    Ok((StatusCode::CREATED, Json(token)))
}
