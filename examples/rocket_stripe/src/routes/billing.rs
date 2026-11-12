//! Customers, Subscriptions, Invoices, and Billing Portal routes (Module 5).
//!
//! Provides endpoints to:
//! - Manage customer vaulting and metadata.
//! - Create and cancel recurring subscriptions.
//! - Manage draft invoices, finalization, manual collection, and credit notes.
//! - Generate self-service Stripe Customer Billing Portal sessions.

use rocket::serde::json::Json;
use rocket::State;
use some_stripe::{
    CreateCustomerParams, CreateInvoiceParams, CreatePortalSessionParams,
    CreateSubscriptionParams, Customer, DeletedCustomer, Invoice, PortalSession,
    Subscription, SubscriptionItemParams,
};

use crate::config::AppConfig;
use crate::models::{
    ApiResponse, CreateCustomerRequest, CreateInvoiceRequest, CreateSubscriptionRequest,
    ErrorResponse,
};

/// Creates a new customer profile in Stripe vault.
///
/// Route: `POST /api/billing/customers`
#[rocket::post("/billing/customers", data = "<req>")]
pub async fn create_customer(
    config: &State<AppConfig>,
    req: Json<CreateCustomerRequest>,
) -> Result<Json<ApiResponse<Customer>>, ErrorResponse> {
    let params = CreateCustomerParams {
        email: Some(req.email.clone()),
        name: req.name.clone(),
        description: req.description.clone(),
        address: None,
        balance: None,
        cash_balance: None,
        coupon: None,
        default_source: None,
        idempotency_key: None,
        invoice_prefix: None,
        invoice_settings: None,
        metadata: None,
        next_invoice_sequence: None,
        payment_method: None,
        phone: None,
        preferred_locales: None,
        promotion_code: None,
        shipping: None,
        source: None,
        tax: None,
        tax_exempt: None,
        tax_id_data: None,
        test_clock: None,
    };

    let customer = config
        .client
        .customers()
        .create(&params)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to create customer: {e}")))?;

    Ok(Json(ApiResponse::new(customer)))
}

/// Retrieves customer details by ID.
///
/// Route: `GET /api/billing/customers/<id>`
#[rocket::get("/billing/customers/<id>")]
pub async fn get_customer(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<Customer>>, ErrorResponse> {
    let customer = config
        .client
        .customers()
        .retrieve(id)
        .await
        .map_err(|e| ErrorResponse::not_found(format!("Customer '{id}' not found: {e}")))?;

    Ok(Json(ApiResponse::new(customer)))
}

/// Deletes a customer profile from Stripe.
///
/// Route: `DELETE /api/billing/customers/<id>`
#[rocket::delete("/billing/customers/<id>")]
pub async fn delete_customer(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<DeletedCustomer>>, ErrorResponse> {
    let deleted = config
        .client
        .customers()
        .delete(id)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to delete customer '{id}': {e}")))?;

    Ok(Json(ApiResponse::new(deleted)))
}

/// Creates a recurring subscription for a customer.
///
/// Route: `POST /api/billing/subscriptions`
#[rocket::post("/billing/subscriptions", data = "<req>")]
pub async fn create_subscription(
    config: &State<AppConfig>,
    req: Json<CreateSubscriptionRequest>,
) -> Result<Json<ApiResponse<Subscription>>, ErrorResponse> {
    let items = vec![SubscriptionItemParams {
        price: Some(req.price_id.clone()),
        quantity: req.quantity,
        metadata: None,
        tax_rates: None,
    }];

    let params = CreateSubscriptionParams {
        customer: req.customer.clone(),
        items,
        cancel_at_period_end: Some(false),
        default_payment_method: None,
        description: None,
        currency: None,
        trial_period_days: None,
        proration_behavior: None,
        idempotency_key: None,
        metadata: None,
        add_invoice_items: None,
        billing_cycle_anchor: None,
        coupon: None,
        days_until_due: None,
    };

    let sub = config
        .client
        .subscriptions()
        .create(&params)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to create subscription: {e}")))?;

    Ok(Json(ApiResponse::new(sub)))
}

/// Retrieves a subscription by ID.
///
/// Route: `GET /api/billing/subscriptions/<id>`
#[rocket::get("/billing/subscriptions/<id>")]
pub async fn get_subscription(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<Subscription>>, ErrorResponse> {
    let sub = config
        .client
        .subscriptions()
        .retrieve(id)
        .await
        .map_err(|e| ErrorResponse::not_found(format!("Subscription '{id}' not found: {e}")))?;

    Ok(Json(ApiResponse::new(sub)))
}

/// Cancels a subscription.
///
/// Route: `POST /api/billing/subscriptions/<id>/cancel`
#[rocket::post("/billing/subscriptions/<id>/cancel")]
pub async fn cancel_subscription(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<Subscription>>, ErrorResponse> {
    let sub = config
        .client
        .subscriptions()
        .cancel(id, None)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to cancel subscription '{id}': {e}")))?;

    Ok(Json(ApiResponse::new(sub)))
}

/// Creates a draft invoice for a customer.
///
/// Route: `POST /api/billing/invoices`
#[rocket::post("/billing/invoices", data = "<req>")]
pub async fn create_invoice(
    config: &State<AppConfig>,
    req: Json<CreateInvoiceRequest>,
) -> Result<Json<ApiResponse<Invoice>>, ErrorResponse> {
    let params = CreateInvoiceParams {
        customer: req.customer.clone(),
        auto_advance: Some(true),
        collection_method: req.collection_method.clone(),
        currency: None,
        description: req.description.clone(),
        due_date: None,
        idempotency_key: None,
        metadata: None,
    };

    let invoice = config
        .client
        .invoices()
        .create(&params)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to create invoice: {e}")))?;

    Ok(Json(ApiResponse::new(invoice)))
}

/// Retrieves an invoice by ID.
///
/// Route: `GET /api/billing/invoices/<id>`
#[rocket::get("/billing/invoices/<id>")]
pub async fn get_invoice(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<Invoice>>, ErrorResponse> {
    let invoice = config
        .client
        .invoices()
        .retrieve(id)
        .await
        .map_err(|e| ErrorResponse::not_found(format!("Invoice '{id}' not found: {e}")))?;

    Ok(Json(ApiResponse::new(invoice)))
}

/// Finalizes a draft invoice.
///
/// Route: `POST /api/billing/invoices/<id>/finalize`
#[rocket::post("/billing/invoices/<id>/finalize")]
pub async fn finalize_invoice(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<Invoice>>, ErrorResponse> {
    let invoice = config
        .client
        .invoices()
        .finalize_invoice(id, None)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to finalize invoice '{id}': {e}")))?;

    Ok(Json(ApiResponse::new(invoice)))
}

/// Pays an invoice out of band or with a default payment method.
///
/// Route: `POST /api/billing/invoices/<id>/pay`
#[rocket::post("/billing/invoices/<id>/pay")]
pub async fn pay_invoice(
    config: &State<AppConfig>,
    id: &str,
) -> Result<Json<ApiResponse<Invoice>>, ErrorResponse> {
    let invoice = config
        .client
        .invoices()
        .pay(id, None)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to pay invoice '{id}': {e}")))?;

    Ok(Json(ApiResponse::new(invoice)))
}

/// Generates a self-service Stripe Customer Billing Portal session.
///
/// Route: `POST /api/billing/portal/sessions/<customer_id>`
#[rocket::post("/billing/portal/sessions/<customer_id>")]
pub async fn create_billing_portal_session(
    config: &State<AppConfig>,
    customer_id: &str,
) -> Result<Json<ApiResponse<PortalSession>>, ErrorResponse> {
    let params = CreatePortalSessionParams {
        customer: customer_id.to_string(),
        configuration: None,
        on_behalf_of: None,
        return_url: Some(config.default_success_url.clone()),
        idempotency_key: None,
    };

    let session = config
        .client
        .billing_portal()
        .create_session(&params)
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to create portal session: {e}")))?;

    Ok(Json(ApiResponse::new(session)))
}
