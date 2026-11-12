//! Standardized request and response Data Transfer Objects (DTOs) for the Rocket example.

use rocket::http::Status;
use rocket::response::{Responder, Response};
use rocket::Request;
use serde::{Deserialize, Serialize};

/// Standard success envelope wrapping API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn new(data: T) -> Self {
        Self {
            success: true,
            data,
        }
    }
}

/// Standard error envelope returned on failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub success: bool,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl ApiError {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            success: false,
            error: error.into(),
            code: None,
        }
    }

    pub fn with_code(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            success: false,
            error: error.into(),
            code: Some(code.into()),
        }
    }
}

/// Custom JSON error responder mapping HTTP status and error body.
#[derive(Debug)]
pub struct ErrorResponse {
    pub status: Status,
    pub body: ApiError,
}

impl ErrorResponse {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: Status::BadRequest,
            body: ApiError::new(msg),
        }
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            status: Status::NotFound,
            body: ApiError::new(msg),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            status: Status::InternalServerError,
            body: ApiError::new(msg),
        }
    }

    pub fn payload_too_large(msg: impl Into<String>) -> Self {
        Self {
            status: Status::PayloadTooLarge,
            body: ApiError::new(msg),
        }
    }
}

impl<'r> Responder<'r, 'static> for ErrorResponse {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'static> {
        let json = serde_json::to_string(&self.body).unwrap_or_else(|_| "{}".to_string());
        Response::build_from(json.respond_to(req)?)
            .status(self.status)
            .header(rocket::http::ContentType::JSON)
            .ok()
    }
}

/// Request payload to initiate a hosted Checkout Session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCheckoutRequest {
    pub product_name: String,
    pub unit_amount: i64,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default = "default_quantity")]
    pub quantity: u64,
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
}

fn default_currency() -> String {
    "usd".to_string()
}

fn default_quantity() -> u64 {
    1
}

/// Request payload to create an Omnibus-compliant Payment Link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePaymentLinkRequest {
    pub line_items: Vec<LineItemParam>,
    pub regular_price: Option<i64>,
    pub announced_discounted_price: Option<i64>,
    pub prior_30d_lowest_price: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItemParam {
    pub price: Option<String>,
    pub quantity: Option<u64>,
}

/// Request payload to create a PaymentIntent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePaymentIntentRequest {
    pub amount: i64,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub description: Option<String>,
    pub capture_method: Option<String>,
    pub customer: Option<String>,
}

/// Request payload to confirm a PaymentIntent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmPaymentIntentRequest {
    pub payment_method: String,
    pub return_url: Option<String>,
}

/// Request payload to create a Customer profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCustomerRequest {
    pub email: String,
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Request payload to create a Subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub customer: String,
    pub price_id: String,
    pub quantity: Option<u64>,
}

/// Request payload to create an Invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvoiceRequest {
    pub customer: String,
    pub collection_method: Option<String>,
    pub description: Option<String>,
}

/// Request payload to create a Connected Account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConnectAccountRequest {
    pub email: Option<String>,
    pub country: Option<String>,
    pub account_type: Option<String>,
}

/// Request payload to create an Account Link for onboarding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccountLinkRequest {
    pub account: String,
    pub refresh_url: String,
    pub return_url: String,
    pub r#type: String,
}

/// Request payload to transfer funds to a Connected Account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTransferRequest {
    pub destination: String,
    pub amount: i64,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub description: Option<String>,
}

/// Request payload for V2 Usage Billing Meter Events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMeterEventRequest {
    pub event_name: String,
    pub stripe_customer_id: String,
    pub value: String,
}
