//! # Actix-Web Example 4: Customers, Subscriptions, Items & Schedules (Module 5 Part A)
//!
//! Demonstrates how to integrate **Customers, Subscriptions, SubscriptionItems, and
//! SubscriptionSchedules** from `some-stripe` inside an `actix-web` application.
//!
//! ## Services Covered
//! - `stripe.customers()`: `create`, `retrieve`, `update` (3-state `Nullable<String>`), `delete`
//!   (GDPR Art. 17 erasure), `list`, `search`, `retrieve_cash_balance`, and `update_cash_balance`
//! - `stripe.subscriptions()`: `create`, `retrieve`, `update`, `cancel`, `resume`, `list`,
//!   `search`, `preview_proration`, `pause_collection`, and `resume_collection`
//! - `stripe.subscriptions().items()`: `create`, `retrieve`, `update`, `del`, `list`, and
//!   `create_usage_record`
//! - `stripe.subscription_schedules()`: `create`, `retrieve`, `update`, `release`, `cancel`,
//!   and `list_default`
//!
//! ## Run the Example
//! ```bash
//! STRIPE_SECRET_KEY=sk_test_123 cargo run --example actix_customers_and_subscriptions
//! ```

use actix_web::{App, HttpResponse, HttpServer, ResponseError, delete, get, post, web};
use some_stripe::{
    CancelSubscriptionParams, CancelSubscriptionScheduleParams, CreateCustomerParams,
    CreateSubscriptionItemParams, CreateSubscriptionParams, CreateSubscriptionScheduleParams,
    CreateUsageRecordParams, DeleteSubscriptionItemParams, ListCustomersParams,
    ListSubscriptionItemsParams, PauseCollectionParams, PreviewProrationParams,
    ReleaseSubscriptionScheduleParams, ResumeSubscriptionParams, SearchCustomersParams,
    SearchSubscriptionsParams, SecretKey, StripeClient, StripeError,
    UpdateCustomerCashBalanceParams, UpdateCustomerParams, UpdateSubscriptionItemParams,
    UpdateSubscriptionParams, UpdateSubscriptionScheduleParams,
};
use std::fmt;

/// Error wrapper converting `StripeError` into structured Actix-Web HTTP responses.
#[derive(Debug)]
pub struct BillingError(pub StripeError);

impl fmt::Display for BillingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<StripeError> for BillingError {
    fn from(err: StripeError) -> Self {
        Self(err)
    }
}

impl ResponseError for BillingError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::BadGateway().json(serde_json::json!({
            "error": self.0.to_string(),
        }))
    }
}

/// `POST /customers` — Creates a customer in the Stripe vault via `stripe.customers().create()`.
#[post("/customers")]
pub async fn create_customer(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateCustomerParams>,
) -> Result<HttpResponse, BillingError> {
    let customer = stripe.customers().create(&params).await?;
    Ok(HttpResponse::Created().json(customer))
}

/// `GET /customers/{id}` — Retrieves a customer via `stripe.customers().retrieve()`.
#[get("/customers/{id}")]
pub async fn retrieve_customer(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, BillingError> {
    let customer = stripe.customers().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(customer))
}

/// `POST /customers/{id}` — Updates a customer using 3-state `Nullable<String>` fields via
/// `stripe.customers().update()`.
#[post("/customers/{id}")]
pub async fn update_customer(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateCustomerParams>,
) -> Result<HttpResponse, BillingError> {
    let customer = stripe.customers().update(id.as_str(), &params).await?;
    Ok(HttpResponse::Ok().json(customer))
}

/// `DELETE /customers/{id}` — Permanently deletes a customer (GDPR Art. 17 right to erasure)
/// via `stripe.customers().delete()`.
#[delete("/customers/{id}")]
pub async fn delete_customer(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, BillingError> {
    let deleted = stripe.customers().delete(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(deleted))
}

/// `GET /customers` — Lists customers via `stripe.customers().list()`.
#[get("/customers")]
pub async fn list_customers(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListCustomersParams>,
) -> Result<HttpResponse, BillingError> {
    let list = stripe.customers().list(&query).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /customers/search` — Searches customers by query string via `stripe.customers().search()`.
#[post("/customers/search")]
pub async fn search_customers(
    stripe: web::Data<StripeClient>,
    params: web::Json<SearchCustomersParams>,
) -> Result<HttpResponse, BillingError> {
    let results = stripe.customers().search(&params).await?;
    Ok(HttpResponse::Ok().json(results))
}

/// `GET /customers/{id}/cash-balance` — Retrieves a customer's cash balance via
/// `stripe.customers().retrieve_cash_balance()`.
#[get("/customers/{id}/cash-balance")]
pub async fn retrieve_customer_cash_balance(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, BillingError> {
    let cash_balance = stripe.customers().retrieve_cash_balance(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(cash_balance))
}

/// `POST /customers/{id}/cash-balance` — Updates a customer's cash balance reconciliation settings
/// via `stripe.customers().update_cash_balance()`.
#[post("/customers/{id}/cash-balance")]
pub async fn update_customer_cash_balance(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateCustomerCashBalanceParams>,
) -> Result<HttpResponse, BillingError> {
    let cash_balance = stripe
        .customers()
        .update_cash_balance(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(cash_balance))
}

/// `POST /subscriptions` — Creates a recurring subscription via `stripe.subscriptions().create()`.
#[post("/subscriptions")]
pub async fn create_subscription(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateSubscriptionParams>,
) -> Result<HttpResponse, BillingError> {
    let sub = stripe.subscriptions().create(&params).await?;
    Ok(HttpResponse::Created().json(sub))
}

/// `GET /subscriptions/{id}` — Retrieves a subscription via `stripe.subscriptions().retrieve()`.
#[get("/subscriptions/{id}")]
pub async fn retrieve_subscription(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, BillingError> {
    let sub = stripe.subscriptions().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(sub))
}

/// `POST /subscriptions/{id}` — Updates a subscription via `stripe.subscriptions().update()`.
#[post("/subscriptions/{id}")]
pub async fn update_subscription(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateSubscriptionParams>,
) -> Result<HttpResponse, BillingError> {
    let sub = stripe.subscriptions().update(id.as_str(), &params).await?;
    Ok(HttpResponse::Ok().json(sub))
}

/// `POST /subscriptions/{id}/cancel` — Cancels a subscription via `stripe.subscriptions().cancel()`.
#[post("/subscriptions/{id}/cancel")]
pub async fn cancel_subscription(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<CancelSubscriptionParams>,
) -> Result<HttpResponse, BillingError> {
    let sub = stripe
        .subscriptions()
        .cancel(id.as_str(), Some(&params))
        .await?;
    Ok(HttpResponse::Ok().json(sub))
}

/// `POST /subscriptions/{id}/resume` — Resumes a paused subscription via `stripe.subscriptions().resume()`.
#[post("/subscriptions/{id}/resume")]
pub async fn resume_subscription(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<ResumeSubscriptionParams>,
) -> Result<HttpResponse, BillingError> {
    let sub = stripe
        .subscriptions()
        .resume(id.as_str(), Some(&params))
        .await?;
    Ok(HttpResponse::Ok().json(sub))
}

/// `POST /subscriptions/{id}/pause-collection` — Pauses invoice collection on a subscription via
/// `stripe.subscriptions().pause_collection()`.
#[post("/subscriptions/{id}/pause-collection")]
pub async fn pause_subscription_collection(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<PauseCollectionParams>,
) -> Result<HttpResponse, BillingError> {
    let sub = stripe
        .subscriptions()
        .pause_collection(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(sub))
}

/// `POST /subscriptions/{id}/resume-collection` — Resumes invoice collection on a subscription via
/// `stripe.subscriptions().resume_collection()`.
#[post("/subscriptions/{id}/resume-collection")]
pub async fn resume_subscription_collection(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, BillingError> {
    let sub = stripe
        .subscriptions()
        .resume_collection(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(sub))
}

/// `POST /subscriptions/{id}/preview-proration` — Previews proration adjustments via
/// `stripe.subscriptions().preview_proration()`.
#[post("/subscriptions/{id}/preview-proration")]
pub async fn preview_subscription_proration(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<PreviewProrationParams>,
) -> Result<HttpResponse, BillingError> {
    let preview = stripe
        .subscriptions()
        .preview_proration(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(preview))
}

/// `POST /subscriptions/search` — Searches subscriptions via `stripe.subscriptions().search_with_params()`.
#[post("/subscriptions/search")]
pub async fn search_subscriptions(
    stripe: web::Data<StripeClient>,
    params: web::Json<SearchSubscriptionsParams>,
) -> Result<HttpResponse, BillingError> {
    let list = stripe.subscriptions().search_with_params(&params).await?;
    Ok(HttpResponse::Ok().json(list))
}

/// `POST /subscription-items` — Adds an item to a subscription via `stripe.subscriptions().items().create()`.
#[post("/subscription-items")]
pub async fn create_subscription_item(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateSubscriptionItemParams>,
) -> Result<HttpResponse, BillingError> {
    let item = stripe.subscriptions().items().create(&params).await?;
    Ok(HttpResponse::Created().json(item))
}

/// `GET /subscription-items/{id}` — Retrieves a subscription item via `stripe.subscriptions().items().retrieve()`.
#[get("/subscription-items/{id}")]
pub async fn retrieve_subscription_item(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, BillingError> {
    let item = stripe.subscriptions().items().retrieve(id.as_str()).await?;
    Ok(HttpResponse::Ok().json(item))
}

/// `POST /subscription-items/{id}` — Updates a subscription item via `stripe.subscriptions().items().update()`.
#[post("/subscription-items/{id}")]
pub async fn update_subscription_item(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateSubscriptionItemParams>,
) -> Result<HttpResponse, BillingError> {
    let item = stripe
        .subscriptions()
        .items()
        .update(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(item))
}

/// `DELETE /subscription-items/{id}` — Deletes a subscription item via `stripe.subscriptions().items().del()`.
#[delete("/subscription-items/{id}")]
pub async fn delete_subscription_item(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: Option<web::Json<DeleteSubscriptionItemParams>>,
) -> Result<HttpResponse, BillingError> {
    let deleted = stripe
        .subscriptions()
        .items()
        .del(id.as_str(), params.as_deref())
        .await?;
    Ok(HttpResponse::Ok().json(deleted))
}

/// `GET /subscription-items` — Lists subscription items via `stripe.subscriptions().items().list()`.
#[get("/subscription-items")]
pub async fn list_subscription_items(
    stripe: web::Data<StripeClient>,
    query: web::Query<ListSubscriptionItemsParams>,
) -> Result<HttpResponse, BillingError> {
    let items = stripe.subscriptions().items().list(&query).await?;
    Ok(HttpResponse::Ok().json(items))
}

/// `POST /subscription-items/{id}/usage-records` — Reports metered usage on a v1 subscription item
/// via `stripe.subscriptions().items().create_usage_record()`.
#[post("/subscription-items/{id}/usage-records")]
pub async fn create_usage_record(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<CreateUsageRecordParams>,
) -> Result<HttpResponse, BillingError> {
    let record = stripe
        .subscriptions()
        .items()
        .create_usage_record(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Created().json(record))
}

/// `POST /subscription-schedules` — Creates a multi-phase subscription schedule via
/// `stripe.subscription_schedules().create()`.
#[post("/subscription-schedules")]
pub async fn create_subscription_schedule(
    stripe: web::Data<StripeClient>,
    params: web::Json<CreateSubscriptionScheduleParams>,
) -> Result<HttpResponse, BillingError> {
    let schedule = stripe.subscription_schedules().create(&params).await?;
    Ok(HttpResponse::Created().json(schedule))
}

/// `GET /subscription-schedules/{id}` — Retrieves a subscription schedule via
/// `stripe.subscription_schedules().retrieve()`.
#[get("/subscription-schedules/{id}")]
pub async fn retrieve_subscription_schedule(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
) -> Result<HttpResponse, BillingError> {
    let schedule = stripe
        .subscription_schedules()
        .retrieve(id.as_str())
        .await?;
    Ok(HttpResponse::Ok().json(schedule))
}

/// `POST /subscription-schedules/{id}` — Updates a subscription schedule via
/// `stripe.subscription_schedules().update()`.
#[post("/subscription-schedules/{id}")]
pub async fn update_subscription_schedule(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<UpdateSubscriptionScheduleParams>,
) -> Result<HttpResponse, BillingError> {
    let schedule = stripe
        .subscription_schedules()
        .update(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(schedule))
}

/// `POST /subscription-schedules/{id}/release` — Releases a subscription schedule via
/// `stripe.subscription_schedules().release()`.
#[post("/subscription-schedules/{id}/release")]
pub async fn release_subscription_schedule(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<ReleaseSubscriptionScheduleParams>,
) -> Result<HttpResponse, BillingError> {
    let schedule = stripe
        .subscription_schedules()
        .release(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(schedule))
}

/// `POST /subscription-schedules/{id}/cancel` — Cancels a subscription schedule via
/// `stripe.subscription_schedules().cancel()`.
#[post("/subscription-schedules/{id}/cancel")]
pub async fn cancel_subscription_schedule(
    stripe: web::Data<StripeClient>,
    id: web::Path<String>,
    params: web::Json<CancelSubscriptionScheduleParams>,
) -> Result<HttpResponse, BillingError> {
    let schedule = stripe
        .subscription_schedules()
        .cancel(id.as_str(), &params)
        .await?;
    Ok(HttpResponse::Ok().json(schedule))
}

/// Mounts all Customers, Subscriptions, Items & Schedules routes onto an Actix-Web [`web::ServiceConfig`].
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_customer)
        .service(retrieve_customer)
        .service(update_customer)
        .service(delete_customer)
        .service(list_customers)
        .service(search_customers)
        .service(retrieve_customer_cash_balance)
        .service(update_customer_cash_balance)
        .service(create_subscription)
        .service(retrieve_subscription)
        .service(update_subscription)
        .service(cancel_subscription)
        .service(resume_subscription)
        .service(pause_subscription_collection)
        .service(resume_subscription_collection)
        .service(preview_subscription_proration)
        .service(search_subscriptions)
        .service(create_subscription_item)
        .service(retrieve_subscription_item)
        .service(update_subscription_item)
        .service(delete_subscription_item)
        .service(list_subscription_items)
        .service(create_usage_record)
        .service(create_subscription_schedule)
        .service(retrieve_subscription_schedule)
        .service(update_subscription_schedule)
        .service(release_subscription_schedule)
        .service(cancel_subscription_schedule);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let secret_key =
        std::env::var("STRIPE_SECRET_KEY").unwrap_or_else(|_| "sk_test_example_key".to_string());
    let stripe = web::Data::new(StripeClient::new(SecretKey::new(secret_key)));

    println!("Starting Actix-Web Customers & Subscriptions example on http://127.0.0.1:8084");
    HttpServer::new(move || {
        App::new()
            .app_data(stripe.clone())
            .configure(configure_routes)
    })
    .bind(("127.0.0.1", 8084))?
    .run()
    .await
}
