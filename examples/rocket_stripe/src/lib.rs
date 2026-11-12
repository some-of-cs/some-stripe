//! # Rocket.rs some-stripe Full Service Library
//!
//! Exposes the Rocket server builder, application configuration (`$VAR_NAME` secret resolver),
//! DTO models, and all 8 Stripe product namespace route modules so integration tests can mount
//! and test the full Rocket application via `rocket::local::asynchronous::Client`.

#[macro_use]
extern crate rocket;

pub mod config;
pub mod models;
pub mod routes;

use config::AppConfig;
use models::ApiResponse;
use rocket::serde::json::Json;
use rocket::{Build, Rocket};
use serde_json::json;

/// Service index endpoint returning the full directory of available routes.
#[get("/")]
pub fn index() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::new(json!({
        "service": "some-stripe Rocket.rs Example",
        "version": "0.8.0",
        "namespaces": {
            "checkout": [
                "POST /api/checkout/sessions - Create hosted checkout session",
                "GET /api/checkout/sessions/<id> - Retrieve checkout session",
                "POST /api/checkout/sessions/<id>/expire - Expire checkout session",
                "POST /api/checkout/payment-links - Create Omnibus-compliant payment link",
                "GET /api/checkout/payment-links/<id> - Retrieve payment link"
            ],
            "payments": [
                "POST /api/payments/intents - Create PaymentIntent",
                "GET /api/payments/intents/<id> - Retrieve PaymentIntent",
                "POST /api/payments/intents/<id>/confirm - Confirm PaymentIntent",
                "POST /api/payments/intents/<id>/capture - Capture authorized PaymentIntent",
                "POST /api/payments/intents/<id>/cancel - Cancel PaymentIntent",
                "POST /api/payments/refunds - Issue refund",
                "GET /api/payments/refunds/<id> - Retrieve refund",
                "GET /api/payments/disputes/<id> - Retrieve dispute details",
                "POST /api/payments/disputes/<id>/close - Close dispute",
                "GET /api/payments/balance - Retrieve Stripe ledger balance",
                "GET /api/payments/balance/transactions/<id> - Retrieve balance transaction"
            ],
            "billing": [
                "POST /api/billing/customers - Create customer profile",
                "GET /api/billing/customers/<id> - Retrieve customer",
                "DELETE /api/billing/customers/<id> - Delete customer",
                "POST /api/billing/subscriptions - Create subscription",
                "GET /api/billing/subscriptions/<id> - Retrieve subscription",
                "POST /api/billing/subscriptions/<id>/cancel - Cancel subscription",
                "POST /api/billing/invoices - Create draft invoice",
                "GET /api/billing/invoices/<id> - Retrieve invoice",
                "POST /api/billing/invoices/<id>/finalize - Finalize invoice",
                "POST /api/billing/invoices/<id>/pay - Pay invoice",
                "POST /api/billing/portal/sessions/<customer_id> - Create billing portal session"
            ],
            "connect": [
                "POST /api/connect/accounts - Create connected account",
                "GET /api/connect/accounts/<id> - Retrieve connected account",
                "POST /api/connect/account-links - Create onboarding Account Link",
                "POST /api/connect/transfers - Transfer platform funds to connected account",
                "POST /api/connect/transfers/<id>/reversals - Reverse transfer",
                "GET /api/connect/application-fees/<id> - Retrieve collected platform fee"
            ],
            "specialized": [
                "GET /api/tax/settings - Retrieve Stripe Tax settings",
                "POST /api/identity/verification-sessions - Create identity KYC session",
                "GET /api/identity/verification-sessions/<id> - Retrieve identity session",
                "POST /api/issuing/cards/<cardholder_id> - Issue virtual card",
                "POST /api/treasury/financial-accounts - Open commercial Treasury account",
                "POST /api/terminal/connection-tokens - Generate POS Terminal connection token"
            ],
            "v2_services": [
                "GET /api/v2/events/<id> - Retrieve V2 Core Event",
                "GET /api/v2/destinations/<id> - Retrieve V2 Event Destination",
                "GET /api/v2/accounts/<id> - Retrieve V2 merchant account",
                "POST /api/v2/events/hydrate - Hydrate V2 thin event notification",
                "POST /api/v2/billing/meter-events - Ingest high-throughput usage meter event"
            ],
            "webhooks": [
                "POST /webhooks/stripe - Ingest, verify HMAC-SHA256 signatures, and route events"
            ]
        }
    })))
}

#[catch(400)]
pub fn bad_request() -> Json<serde_json::Value> {
    Json(json!({ "success": false, "error": "Bad Request: Malformed JSON or invalid parameter" }))
}

#[catch(404)]
pub fn not_found() -> Json<serde_json::Value> {
    Json(json!({ "success": false, "error": "Resource Not Found" }))
}

#[catch(422)]
pub fn unprocessable_entity() -> Json<serde_json::Value> {
    Json(json!({ "success": false, "error": "Unprocessable Entity: Validation constraint failed" }))
}

#[catch(500)]
pub fn internal_error() -> Json<serde_json::Value> {
    Json(json!({ "success": false, "error": "Internal Server Error" }))
}

/// Builds a Rocket instance configured with the provided [`AppConfig`].
///
/// Used by both the production binary and the integration test harness (`tests/`).
pub fn build_rocket(config: AppConfig) -> Rocket<Build> {
    rocket::build()
        .manage(config)
        .mount("/", routes![index])
        .mount(
            "/api",
            routes![
                routes::checkout::create_checkout_session,
                routes::checkout::get_checkout_session,
                routes::checkout::expire_checkout_session,
                routes::checkout::create_payment_link,
                routes::checkout::get_payment_link,
                routes::payments::create_payment_intent,
                routes::payments::get_payment_intent,
                routes::payments::confirm_payment_intent,
                routes::payments::capture_payment_intent,
                routes::payments::cancel_payment_intent,
                routes::payments::create_refund,
                routes::payments::get_refund,
                routes::payments::get_dispute,
                routes::payments::close_dispute,
                routes::payments::get_balance,
                routes::payments::get_balance_transaction,
                routes::billing::create_customer,
                routes::billing::get_customer,
                routes::billing::delete_customer,
                routes::billing::create_subscription,
                routes::billing::get_subscription,
                routes::billing::cancel_subscription,
                routes::billing::create_invoice,
                routes::billing::get_invoice,
                routes::billing::finalize_invoice,
                routes::billing::pay_invoice,
                routes::billing::create_billing_portal_session,
                routes::connect::create_connected_account,
                routes::connect::get_connected_account,
                routes::connect::create_account_link,
                routes::connect::create_transfer,
                routes::connect::reverse_transfer,
                routes::connect::get_application_fee,
                routes::specialized::get_tax_settings,
                routes::specialized::create_identity_session,
                routes::specialized::get_identity_session,
                routes::specialized::create_issuing_card,
                routes::specialized::create_financial_account,
                routes::specialized::create_terminal_connection_token,
                routes::v2_services::get_v2_event,
                routes::v2_services::get_v2_destination,
                routes::v2_services::get_v2_account,
                routes::v2_services::hydrate_v2_event,
                routes::v2_services::record_meter_event,
            ],
        )
        .mount("/webhooks", routes![routes::webhooks::handle_stripe_webhook])
        .register(
            "/",
            catchers![bad_request, not_found, unprocessable_entity, internal_error],
        )
}

/// Builds a Rocket instance initialized from `.env*` and process environment variables
/// via `AppConfig::from_env()` (`StripeClient::new("$STRIPE_SECRET_KEY")`).
pub fn rocket() -> Rocket<Build> {
    build_rocket(AppConfig::from_env())
}
