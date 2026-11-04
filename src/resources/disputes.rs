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
            if let Some(ref val) = ev.customer_communication {
                form_data.push(("evidence[customer_communication]", val.as_str()));
            }
            if let Some(ref val) = ev.customer_email_address {
                form_data.push(("evidence[customer_email_address]", val.as_str()));
            }
            if let Some(ref val) = ev.customer_name {
                form_data.push(("evidence[customer_name]", val.as_str()));
            }
            if let Some(ref val) = ev.customer_purchase_ip {
                form_data.push(("evidence[customer_purchase_ip]", val.as_str()));
            }
            if let Some(ref val) = ev.billing_address {
                form_data.push(("evidence[billing_address]", val.as_str()));
            }
            if let Some(ref val) = ev.shipping_address {
                form_data.push(("evidence[shipping_address]", val.as_str()));
            }
            if let Some(ref val) = ev.shipping_date {
                form_data.push(("evidence[shipping_date]", val.as_str()));
            }
            if let Some(ref val) = ev.shipping_documentation {
                form_data.push(("evidence[shipping_documentation]", val.as_str()));
            }
            if let Some(ref val) = ev.shipping_tracking_number {
                form_data.push(("evidence[shipping_tracking_number]", val.as_str()));
            }
            if let Some(ref val) = ev.service_date {
                form_data.push(("evidence[service_date]", val.as_str()));
            }
            if let Some(ref val) = ev.service_documentation {
                form_data.push(("evidence[service_documentation]", val.as_str()));
            }
            if let Some(ref val) = ev.duplicate_charge_documentation {
                form_data.push(("evidence[duplicate_charge_documentation]", val.as_str()));
            }
            if let Some(ref val) = ev.duplicate_charge_explanation {
                form_data.push(("evidence[duplicate_charge_explanation]", val.as_str()));
            }
            if let Some(ref val) = ev.duplicate_charge_id {
                form_data.push(("evidence[duplicate_charge_id]", val.as_str()));
            }
            if let Some(ref val) = ev.refund_refusal_explanation {
                form_data.push(("evidence[refund_refusal_explanation]", val.as_str()));
            }
            if let Some(ref val) = ev.cancellation_policy {
                form_data.push(("evidence[cancellation_policy]", val.as_str()));
            }
            if let Some(ref val) = ev.cancellation_policy_disclosure {
                form_data.push(("evidence[cancellation_policy_disclosure]", val.as_str()));
            }
            if let Some(ref val) = ev.product_description {
                form_data.push(("evidence[product_description]", val.as_str()));
            }
            if let Some(ref val) = ev.receipt {
                form_data.push(("evidence[receipt]", val.as_str()));
            }
            if let Some(ref val) = ev.uncategorized_documentation {
                form_data.push(("evidence[uncategorized_documentation]", val.as_str()));
            }
            if let Some(ref val) = ev.uncategorized_text {
                form_data.push(("evidence[uncategorized_text]", val.as_str()));
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
        let generated_key = crate::resources::generate_uuid_v4();
        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &generated_key);
        let val = self.client.execute(req).await?;
        let dispute: Dispute = serde_json::from_value(val)?;
        Ok(dispute)
    }
}
