//! Specialized Product Namespaces routes (Module 7).
//!
//! Exposes endpoints for:
//! - Stripe Tax (`stripe.tax().settings()`)
//! - Stripe Identity verification sessions (`stripe.identity().verification_sessions()`)
//! - Stripe Issuing card management (`stripe.issuing().cards()`)
//! - Stripe Treasury financial accounts (`stripe.treasury().financial_accounts()`)
//! - Stripe Terminal POS connection tokens and readers (`stripe.terminal().connection_tokens()`)

use rocket::serde::json::Json;
use rocket::State;
use some_stripe::{
    ConnectionToken, CreateCardParams, CreateConnectionTokenParams, CreateFinancialAccountParams,
    CreateVerificationSessionParams, FinancialAccount, IssuingCard, TaxSettings,
    VerificationSession, VerificationSessionType,
};

use crate::config::AppConfig;
use crate::models::{ApiResponse, ErrorResponse};

/// Retrieves the current Stripe Tax settings and nexus configurations.
///
/// Route: `GET /api/tax/settings`
#[rocket::get("/tax/settings")]
pub async fn get_tax_settings(
    config: &State<AppConfig>,
) -> Result<Json<ApiResponse<TaxSettings>>, ErrorResponse> {
    let settings = config
        .client
        .tax()
        .settings()
        .retrieve()
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to retrieve tax settings: {e}")))?;

    Ok(Json(ApiResponse::new(settings)))
}

/// Creates a new biometric and document Identity Verification Session.
///
/// Route: `POST /api/identity/verification-sessions`
#[rocket::post("/identity/verification-sessions")]
pub async fn create_identity_session(
    config: &State<AppConfig>,
) -> Result<Json<ApiResponse<VerificationSession>>, ErrorResponse> {
    let params = CreateVerificationSessionParams {
        r#type: VerificationSessionType::Document,
        options: None,
        return_url: Some(config.default_success_url.clone()),
        metadata: None,
        provided_details: None,
        related_customer: None,
        client_reference_id: None,
        idempotency_key: None,
    };

    let session = config
        .client
        .identity()
        .verification_sessions()
        .create(&params)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to create identity session: {e}")))?;

    Ok(Json(ApiResponse::new(session)))
}

/// Retrieves an Identity Verification Session by ID.
///
/// Route: `GET /api/identity/verification-sessions/<id>`
#[rocket::get("/identity/verification-sessions/<id>")]
pub async fn get_identity_session(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<VerificationSession>>, ErrorResponse> {
    let session = config
        .client
        .identity()
        .verification_sessions()
        .retrieve(id)
        .await
        .map_err(|e| ErrorResponse::not_found(format!("Identity session '{id}' not found: {e}")))?;

    Ok(Json(ApiResponse::new(session)))
}

/// Issues a new virtual payment card via Stripe Issuing.
///
/// Route: `POST /api/issuing/cards/<cardholder_id>`
#[rocket::post("/issuing/cards/<cardholder_id>")]
pub async fn create_issuing_card(
    config: &State<AppConfig>,
    cardholder_id: &str,
) -> Result<Json<ApiResponse<IssuingCard>>, ErrorResponse> {
    let params = CreateCardParams {
        cardholder: cardholder_id.to_string(),
        currency: "usd".to_string(),
        r#type: "virtual".to_string(),
        status: Some("active".to_string()),
        spending_controls: None,
        metadata: None,
        shipping: None,
        idempotency_key: None,
    };

    let card = config
        .client
        .issuing()
        .cards()
        .create(&params)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to issue card: {e}")))?;

    Ok(Json(ApiResponse::new(card)))
}

/// Opens a commercial FDIC-insured Treasury Financial Account.
///
/// Route: `POST /api/treasury/financial-accounts`
#[rocket::post("/treasury/financial-accounts")]
pub async fn create_financial_account(
    config: &State<AppConfig>,
) -> Result<Json<ApiResponse<FinancialAccount>>, ErrorResponse> {
    let params = CreateFinancialAccountParams {
        supported_currencies: vec!["usd".to_string()],
        features: None,
        metadata: None,
        idempotency_key: None,
    };

    let account = config
        .client
        .treasury()
        .financial_accounts()
        .create(&params)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to create financial account: {e}")))?;

    Ok(Json(ApiResponse::new(account)))
}

/// Generates a connection token for Stripe Terminal point-of-sale reader SDKs.
///
/// Route: `POST /api/terminal/connection-tokens`
#[rocket::post("/terminal/connection-tokens")]
pub async fn create_terminal_connection_token(
    config: &State<AppConfig>,
) -> Result<Json<ApiResponse<ConnectionToken>>, ErrorResponse> {
    let params = CreateConnectionTokenParams {
        location: None,
        idempotency_key: None,
    };

    let token = config
        .client
        .terminal()
        .connection_tokens()
        .create(&params)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to generate connection token: {e}")))?;

    Ok(Json(ApiResponse::new(token)))
}
