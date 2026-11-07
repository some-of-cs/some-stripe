//! Stripe Payouts resource implementations.

use some_stripe_core::patterns::{encode_form_pairs, to_form_params};
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CreatePayoutParams, ListPayoutsParams, Payout, PayoutList, UpdatePayoutParams,
};

/// Resource for managing Stripe Payouts (`stripe.payouts`).
#[derive(Clone, Copy)]
pub struct PayoutsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> PayoutsResource<'a> {
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    pub async fn create(&self, params: &CreatePayoutParams) -> Result<Payout, StripeError> {
        let url = format!("{}/v1/payouts", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let generated_key;
        let idempotency_key = match params.idempotency_key.as_deref() {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = crate::resources::generate_uuid_v4();
                &generated_key
            }
        };

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        let val = self.client.execute(req).await?;
        let payout: Payout = serde_json::from_value(val)?;
        Ok(payout)
    }

    pub async fn retrieve(&self, id: &str) -> Result<Payout, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!("{}/v1/payouts/{encoded}", self.client.config.api_base);
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let payout: Payout = serde_json::from_value(val)?;
        Ok(payout)
    }

    pub async fn update(
        &self,
        id: &str,
        params: &UpdatePayoutParams,
    ) -> Result<Payout, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!("{}/v1/payouts/{encoded}", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let generated_key;
        let idempotency_key = match params.idempotency_key.as_deref() {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = crate::resources::generate_uuid_v4();
                &generated_key
            }
        };

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        let val = self.client.execute(req).await?;
        let payout: Payout = serde_json::from_value(val)?;
        Ok(payout)
    }

    pub async fn cancel(&self, id: &str) -> Result<Payout, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/payouts/{encoded}/cancel",
            self.client.config.api_base
        );
        let form_data: Vec<(&str, &str)> = Vec::new();
        let generated_key = crate::resources::generate_uuid_v4();

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", generated_key.as_str());

        let val = self.client.execute(req).await?;
        let payout: Payout = serde_json::from_value(val)?;
        Ok(payout)
    }

    pub async fn reverse(&self, id: &str) -> Result<Payout, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/payouts/{encoded}/reverse",
            self.client.config.api_base
        );
        let form_data: Vec<(&str, &str)> = Vec::new();
        let generated_key = crate::resources::generate_uuid_v4();

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", generated_key.as_str());

        let val = self.client.execute(req).await?;
        let payout: Payout = serde_json::from_value(val)?;
        Ok(payout)
    }

    pub async fn list(
        &self,
        params: Option<&ListPayoutsParams>,
    ) -> Result<PayoutList, StripeError> {
        let url = match params {
            Some(p) => {
                let query_data = to_form_params(p)?;
                if query_data.is_empty() {
                    format!("{}/v1/payouts", self.client.config.api_base)
                } else {
                    let query_str = encode_form_pairs(&query_data);
                    format!("{}/v1/payouts?{query_str}", self.client.config.api_base)
                }
            }
            None => format!("{}/v1/payouts", self.client.config.api_base),
        };

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let list: PayoutList = serde_json::from_value(val)?;
        Ok(list)
    }
}
