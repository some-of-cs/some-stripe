//! Stripe Payouts resource implementations.

use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CreatePayoutParams, ListPayoutsParams, Payout, PayoutList, UpdatePayoutParams,
};

/// Resource for managing Stripe Payouts (`stripe.payouts`).
#[derive(Clone, Copy)]
pub struct PayoutsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> PayoutsResource<'a> {
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this resource to operate on behalf of a specific connected account
    /// by attaching the `Stripe-Account` tenant header to all outbound requests.
    pub fn with_account(&self, account: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account),
        }
    }

    pub async fn create(&self, params: &CreatePayoutParams) -> Result<Payout, StripeError> {
        let url = format!("{}/v1/payouts", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let generated_key = crate::resources::generate_uuid_v4();
        let idempotency_key = generated_key.as_str();

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let payout: Payout = serde_json::from_value(val)?;
        Ok(payout)
    }

    pub async fn retrieve(&self, id: &str) -> Result<Payout, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!("{}/v1/payouts/{encoded}", self.client.config.api_base);
        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }
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

        let generated_key = crate::resources::generate_uuid_v4();
        let idempotency_key = generated_key.as_str();

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

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

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", generated_key.as_str());

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

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

        let mut req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", generated_key.as_str());

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let payout: Payout = serde_json::from_value(val)?;
        Ok(payout)
    }

    pub async fn list(
        &self,
        params: Option<&ListPayoutsParams>,
    ) -> Result<PayoutList, StripeError> {
        let mut url = format!("{}/v1/payouts", self.client.config.api_base);
        let mut query_params = Vec::new();
        let limit_str = params.and_then(|p| p.limit).map(|l| l.to_string());

        if let Some(p) = params {
            if let Some(ref st) = p.status {
                query_params.push(("status", st.as_str()));
            }
            if let Some(ref l) = limit_str {
                query_params.push(("limit", l.as_str()));
            }
            if let Some(ref a) = p.starting_after {
                query_params.push(("starting_after", a.as_str()));
            }
            if let Some(ref b) = p.ending_before {
                query_params.push(("ending_before", b.as_str()));
            }
        }

        if !query_params.is_empty() {
            let encoded: Vec<String> = query_params
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            url.push('?');
            url.push_str(&encoded.join("&"));
        }

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }
        let val = self.client.execute(req).await?;
        let list: PayoutList = serde_json::from_value(val)?;
        Ok(list)
    }
}
