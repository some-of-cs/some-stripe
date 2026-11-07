//! Stripe Application Fees resource implementations.

use some_stripe_core::patterns::{encode_form_pairs, to_form_params};
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    ApplicationFee, ApplicationFeeList, CreateFeeRefundParams, FeeRefund, FeeRefundList,
    ListApplicationFeesParams, ListFeeRefundsParams,
};

/// Resource for managing Stripe Application Fees (`stripe.application_fees`).
#[derive(Clone, Copy)]
pub struct ApplicationFeesResource<'a> {
    client: &'a StripeClient,
}

impl<'a> ApplicationFeesResource<'a> {
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    pub async fn retrieve(&self, id: &str) -> Result<ApplicationFee, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/application_fees/{encoded}",
            self.client.config.api_base
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let fee: ApplicationFee = serde_json::from_value(val)?;
        Ok(fee)
    }

    pub async fn list(
        &self,
        params: Option<&ListApplicationFeesParams>,
    ) -> Result<ApplicationFeeList, StripeError> {
        let url = match params {
            Some(p) => {
                let query_data = to_form_params(p)?;
                if query_data.is_empty() {
                    format!("{}/v1/application_fees", self.client.config.api_base)
                } else {
                    let query_str = encode_form_pairs(&query_data);
                    format!(
                        "{}/v1/application_fees?{query_str}",
                        self.client.config.api_base
                    )
                }
            }
            None => format!("{}/v1/application_fees", self.client.config.api_base),
        };

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let list: ApplicationFeeList = serde_json::from_value(val)?;
        Ok(list)
    }

    pub async fn create_refund(
        &self,
        fee_id: &str,
        params: Option<&CreateFeeRefundParams>,
    ) -> Result<FeeRefund, StripeError> {
        let encoded = validate_and_encode_id(fee_id)?;
        let url = format!(
            "{}/v1/application_fees/{encoded}/refunds",
            self.client.config.api_base
        );
        let form_data = match params {
            Some(p) => to_form_params(p)?,
            None => to_form_params(&CreateFeeRefundParams::default())?,
        };

        let generated_key;
        let idempotency_key = match params.and_then(|p| p.idempotency_key.as_deref()) {
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
        let refund: FeeRefund = serde_json::from_value(val)?;
        Ok(refund)
    }

    pub async fn retrieve_refund(
        &self,
        fee_id: &str,
        refund_id: &str,
    ) -> Result<FeeRefund, StripeError> {
        let encoded_fee = validate_and_encode_id(fee_id)?;
        let encoded_refund = validate_and_encode_id(refund_id)?;
        let url = format!(
            "{}/v1/application_fees/{encoded_fee}/refunds/{encoded_refund}",
            self.client.config.api_base
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let refund: FeeRefund = serde_json::from_value(val)?;
        Ok(refund)
    }

    pub async fn list_refunds(
        &self,
        fee_id: &str,
        params: Option<&ListFeeRefundsParams>,
    ) -> Result<FeeRefundList, StripeError> {
        let encoded_fee = validate_and_encode_id(fee_id)?;
        let url = match params {
            Some(p) => {
                let query_data = to_form_params(p)?;
                if query_data.is_empty() {
                    format!(
                        "{}/v1/application_fees/{encoded_fee}/refunds",
                        self.client.config.api_base
                    )
                } else {
                    let query_str = encode_form_pairs(&query_data);
                    format!(
                        "{}/v1/application_fees/{encoded_fee}/refunds?{query_str}",
                        self.client.config.api_base
                    )
                }
            }
            None => format!(
                "{}/v1/application_fees/{encoded_fee}/refunds",
                self.client.config.api_base
            ),
        };

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let list: FeeRefundList = serde_json::from_value(val)?;
        Ok(list)
    }
}
