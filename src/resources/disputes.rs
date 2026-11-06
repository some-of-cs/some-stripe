//! Stripe Disputes resource implementations.
//!
//! Provides endpoints to retrieve dispute details, submit defense evidence to card issuers,
//! and close or concede chargebacks, enforcing strict input sanitization and evidence locking invariants.

use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{Dispute, UpdateDisputeParams};

/// Resource for managing Stripe Disputes (`stripe.disputes`).
///
/// Wraps endpoints for retrieving disputes (`GET /v1/disputes/{id}`), submitting or updating evidence
/// (`POST /v1/disputes/{id}`), and closing/conceding disputes (`POST /v1/disputes/{id}/close`).
///
/// # Evidence Submission Invariant
///
/// When updating a dispute with [`update`](Self::update) and setting `submit: true`, the evidence is
/// formally transmitted to the card issuing bank. Once submitted or when the dispute transitions to
/// [`DisputeStatus::UnderReview`](some_stripe_types::DisputeStatus::UnderReview),
/// [`DisputeStatus::Won`](some_stripe_types::DisputeStatus::Won),
/// [`DisputeStatus::Lost`](some_stripe_types::DisputeStatus::Lost), or
/// [`DisputeStatus::Closed`](some_stripe_types::DisputeStatus::Closed), evidence can no longer be edited.
#[derive(Clone, Copy)]
pub struct DisputesResource<'a> {
    client: &'a StripeClient,
}

impl<'a> DisputesResource<'a> {
    /// Creates a new `DisputesResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Retrieves the details of an existing dispute (`GET /v1/disputes/{id}`).
    ///
    /// Validates `dispute_id` to prevent path traversal and injection attacks before transmitting.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `dispute_id` is empty, contains whitespace,
    ///   or includes invalid characters such as `'..'`, `'/'`, `'\\'`, `'?'`, or `'#'`.
    /// - Returns [`StripeError::Api`] if the dispute does not exist or the Stripe API returns an error.
    /// - Returns [`StripeError::Json`] if the response body cannot be deserialized into a [`Dispute`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let dispute = client.disputes().retrieve("dp_123456789").await?;
    ///
    /// println!("Dispute ID: {}", dispute.id);
    /// println!("Status: {:?}", dispute.status);
    /// println!("Amount: {} {}", dispute.amount, dispute.currency);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, dispute_id: &str) -> Result<Dispute, StripeError> {
        let encoded = validate_and_encode_id(dispute_id)?;
        let url = format!("{}/v1/disputes/{encoded}", self.client.config.api_base);
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let dispute: Dispute = serde_json::from_value(val)?;
        Ok(dispute)
    }

    /// Updates evidence or metadata for an existing dispute (`POST /v1/disputes/{id}`).
    ///
    /// Serializes dispute evidence using standard Stripe bracket notation (`evidence[customer_name]`, etc.)
    /// and metadata key-value pairs (`metadata[key]`).
    ///
    /// When `params.submit` is `Some(true)`, evidence is formally submitted to the issuing bank
    /// and evidence editing is locked.
    ///
    /// Automatically injects an RFC 4122 compliant UUID v4 `Idempotency-Key` header
    /// if none is provided in `params`.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `dispute_id` fails path traversal checks.
    /// - Returns [`StripeError::Api`] if the evidence is already locked or the API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into a [`Dispute`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::{DisputeEvidence, UpdateDisputeParams};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    ///
    /// let evidence = DisputeEvidence {
    ///     customer_email_address: Some("cardholder@example.com".to_string()),
    ///     customer_name: Some("Jane Doe".to_string()),
    ///     product_description: Some("Annual cloud storage license".to_string()),
    ///     shipping_tracking_number: Some("TRACK123456".to_string()),
    ///     ..Default::default()
    /// };
    ///
    /// let params = UpdateDisputeParams::new()
    ///     .with_evidence(evidence)
    ///     .with_submit(false)
    ///     .with_idempotency_key("dp_update_idem_001");
    ///
    /// let dispute = client.disputes().update("dp_123456789", &params).await?;
    /// println!("Updated dispute evidence details: {:?}", dispute.evidence_details);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        dispute_id: &str,
        params: &UpdateDisputeParams,
    ) -> Result<Dispute, StripeError> {
        let encoded = validate_and_encode_id(dispute_id)?;
        let url = format!("{}/v1/disputes/{encoded}", self.client.config.api_base);
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
    ///
    /// Conceding a dispute immediately acknowledges the cardholder's claim, transitions
    /// the dispute to `lost` status, and releases the contested amount to the cardholder.
    /// This action cannot be reversed.
    ///
    /// Automatically injects an RFC 4122 compliant UUID v4 `Idempotency-Key` header
    /// to guarantee safe retries.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `dispute_id` fails path traversal checks.
    /// - Returns [`StripeError::Api`] if the dispute is already closed or does not exist.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into a [`Dispute`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let closed = client.disputes().close("dp_123456789").await?;
    ///
    /// println!("Closed dispute status: {:?}", closed.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn close(&self, dispute_id: &str) -> Result<Dispute, StripeError> {
        let encoded = validate_and_encode_id(dispute_id)?;
        let url = format!(
            "{}/v1/disputes/{encoded}/close",
            self.client.config.api_base
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
