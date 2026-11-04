use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripePaymentIntentRequest {
    pub amount_cents: i64,
    pub currency: String,
    pub customer_id: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripePaymentIntentResponse {
    pub id: String,
    pub client_secret: String,
    pub status: String,
}
