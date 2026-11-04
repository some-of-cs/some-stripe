//! Stripe Disputes resource implementations.

use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{Dispute, UpdateDisputeParams};

/// Resource for managing Stripe Disputes (`stripe.disputes`).
#[derive(Clone, Copy)]
pub struct DisputesResource<'a> {
    client: &'a StripeClient,
}

impl<'a> DisputesResource<'a> {
    pub(crate) fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Retrieves the details of an existing dispute (`GET /v1/disputes/{id}`).
    pub async fn retrieve(&self, dispute_id: &str) -> Result<Dispute, StripeError> {
        let url = format!("{}/v1/disputes/{}", self.client.config.api_base, dispute_id);
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let dispute: Dispute = serde_json::from_value(val)?;
        Ok(dispute)
    }

    /// Updates evidence or metadata for an existing dispute (`POST /v1/disputes/{id}`).
    ///
    /// When `params.submit` is `Some(true)`, evidence is formally submitted to the issuing bank.
    pub async fn update(
        &self,
        dispute_id: &str,
        params: &UpdateDisputeParams,
    ) -> Result<Dispute, StripeError> {
        let url = format!("{}/v1/disputes/{}", self.client.config.api_base, dispute_id);
        let mut form_data = Vec::new();

        let submit_str = params.submit.map(|s| s.to_string());
        if let Some(ref s) = submit_str {
            form_data.push(("submit", s.as_str()));
        }

        if let Some(ref ev) = params.evidence {
            if let Some(ref name) = ev.customer_name {
                form_data.push(("evidence[customer_name]", name.as_str()));
            }
            if let Some(ref email) = ev.customer_email_address {
                form_data.push(("evidence[customer_email_address]", email.as_str()));
            }
            if let Some(ref billing) = ev.billing_address {
                form_data.push(("evidence[billing_address]", billing.as_str()));
            }
            if let Some(ref shipping) = ev.shipping_address {
                form_data.push(("evidence[shipping_address]", shipping.as_str()));
            }
            if let Some(ref doc) = ev.shipping_documentation {
                form_data.push(("evidence[shipping_documentation]", doc.as_str()));
            }
            if let Some(ref desc) = ev.product_description {
                form_data.push(("evidence[product_description]", desc.as_str()));
            }
            if let Some(ref uncateg) = ev.uncategorized_text {
                form_data.push(("evidence[uncategorized_text]", uncateg.as_str()));
            }
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

        let mut req = self.client.http.post(&url).form(&form_data);
        if let Some(ref idempotency_key) = params.idempotency_key {
            req = req.header("Idempotency-Key", idempotency_key.as_str());
        }

        let val = self.client.execute(req).await?;
        let dispute: Dispute = serde_json::from_value(val)?;
        Ok(dispute)
    }

    /// Closes the dispute, conceding it and releasing funds (`POST /v1/disputes/{id}/close`).
    pub async fn close(&self, dispute_id: &str) -> Result<Dispute, StripeError> {
        let url = format!(
            "{}/v1/disputes/{}/close",
            self.client.config.api_base, dispute_id
        );
        let form_data: Vec<(&str, &str)> = Vec::new();
        let req = self.client.http.post(&url).form(&form_data);
        let val = self.client.execute(req).await?;
        let dispute: Dispute = serde_json::from_value(val)?;
        Ok(dispute)
    }
}
