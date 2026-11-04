//! Stripe Refunds resource implementations.

use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{CancelRefundParams, CreateRefundParams, Refund, UpdateRefundParams};

/// Resource for managing Stripe Refunds (`stripe.refunds`).
#[derive(Clone, Copy)]
pub struct RefundsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> RefundsResource<'a> {
    pub(crate) fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Creates a refund (`POST /v1/refunds`).
    ///
    /// Supports idempotency via `params.idempotency_key` and statutory 14-day
    /// full withdrawal (EU Directive 2011/83/EU Article 13) when `amount` is omitted.
    pub async fn create(&self, params: &CreateRefundParams) -> Result<Refund, StripeError> {
        let url = format!("{}/v1/refunds", self.client.config.api_base);
        let mut form_data = Vec::new();

        if let Some(ref pi) = params.payment_intent {
            form_data.push(("payment_intent", pi.as_str()));
        }
        if let Some(ref ch) = params.charge {
            form_data.push(("charge", ch.as_str()));
        }
        let amount_str = params.amount.map(|a| a.to_string());
        if let Some(ref a) = amount_str {
            form_data.push(("amount", a.as_str()));
        }
        if let Some(ref r) = params.reason {
            form_data.push(("reason", r.as_str()));
        }
        let app_fee_str = params.refund_application_fee.map(|f| f.to_string());
        if let Some(ref f) = app_fee_str {
            form_data.push(("refund_application_fee", f.as_str()));
        }
        let rev_transfer_str = params.reverse_transfer.map(|r| r.to_string());
        if let Some(ref r) = rev_transfer_str {
            form_data.push(("reverse_transfer", r.as_str()));
        }

        let mut metadata_entries = Vec::new();
        if let Some(ref meta) = params.metadata {
            for (k, v) in meta {
                metadata_entries.push((format!("metadata[{k}]"), v.as_str()));
            }
        }
        for (k, v) in &metadata_entries {
            form_data.push((k.as_str(), *v));
        }

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
        let refund: Refund = serde_json::from_value(val)?;
        Ok(refund)
    }

    /// Retrieves the details of an existing refund (`GET /v1/refunds/{id}`).
    pub async fn retrieve(&self, refund_id: &str) -> Result<Refund, StripeError> {
        let url = format!("{}/v1/refunds/{}", self.client.config.api_base, refund_id);
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let refund: Refund = serde_json::from_value(val)?;
        Ok(refund)
    }

    /// Updates specified properties of an existing refund (`POST /v1/refunds/{id}`).
    pub async fn update(
        &self,
        refund_id: &str,
        params: &UpdateRefundParams,
    ) -> Result<Refund, StripeError> {
        let url = format!("{}/v1/refunds/{}", self.client.config.api_base, refund_id);
        let mut form_data = Vec::new();

        let mut metadata_entries = Vec::new();
        if let Some(ref meta) = params.metadata {
            for (k, v) in meta {
                metadata_entries.push((format!("metadata[{k}]"), v.as_str()));
            }
        }
        for (k, v) in &metadata_entries {
            form_data.push((k.as_str(), *v));
        }

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
        let refund: Refund = serde_json::from_value(val)?;
        Ok(refund)
    }

    /// Cancels a pending refund (`POST /v1/refunds/{id}/cancel`).
    pub async fn cancel(
        &self,
        refund_id: &str,
        params: Option<&CancelRefundParams>,
    ) -> Result<Refund, StripeError> {
        let url = format!(
            "{}/v1/refunds/{}/cancel",
            self.client.config.api_base, refund_id
        );
        let form_data: Vec<(&str, &str)> = Vec::new();
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
        let refund: Refund = serde_json::from_value(val)?;
        Ok(refund)
    }
}
