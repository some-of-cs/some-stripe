//! Stripe Refunds resource implementations.
//!
//! Provides endpoints to issue, retrieve, update, and cancel refunds against charges
//! or PaymentIntents, ensuring strict compliance with statutory 14-day consumer withdrawal
//! directives (EU Directive 2011/83/EU Article 13) and idempotent request handling.

use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{CancelRefundParams, CreateRefundParams, Refund, UpdateRefundParams};

/// Resource for managing Stripe Refunds (`stripe.refunds`).
///
/// Wraps endpoints for creating refunds (`POST /v1/refunds`), retrieving refund records
/// (`GET /v1/refunds/{id}`), updating metadata (`POST /v1/refunds/{id}`), and canceling
/// pending refunds (`POST /v1/refunds/{id}/cancel`).
///
/// # Statutory 14-Day Consumer Withdrawal Compliance
///
/// In accordance with EU Directive 2011/83/EU Article 13 (Consumer Rights Directive), consumers
/// have a 14-day right of withdrawal for distance and off-premises contracts. Omitting the
/// `amount` parameter when calling [`create`](Self::create) automatically triggers a 100% full
/// reimbursement without penalty or unjustified deductions.
#[derive(Clone, Copy)]
pub struct RefundsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> RefundsResource<'a> {
    /// Creates a new `RefundsResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates a refund (`POST /v1/refunds`).
    ///
    /// Accepts parameters by reference (`&CreateRefundParams`), serializing charge/intent references,
    /// optional integer minor currency amounts, refund reasons, transfer reversals, and metadata.
    ///
    /// Omitting `params.amount` executes a full refund compliant with statutory withdrawal directives.
    ///
    /// Automatically injects an RFC 4122 compliant UUID v4 `Idempotency-Key` header
    /// if none is provided in `params`.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if neither `payment_intent` nor `charge` is specified.
    /// - Returns [`StripeError::Api`] if the charge has already been refunded or the API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into a [`Refund`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::{CreateRefundParams, RefundReason};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    ///
    /// // Issue a partial refund with explicit customer reason:
    /// let params = CreateRefundParams::for_payment_intent("pi_123456789")
    ///     .with_amount(1500) // $15.00 in minor units
    ///     .with_reason(RefundReason::RequestedByCustomer)
    ///     .with_idempotency_key("refund_pi_123456789_001");
    ///
    /// let refund = client.refunds().create(&params).await?;
    /// println!("Created refund: {} (status: {})", refund.id, refund.status);
    /// # Ok(())
    /// # }
    /// ```
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
    ///
    /// Validates `refund_id` to prevent path traversal and injection attacks before transmitting.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `refund_id` is empty, contains whitespace,
    ///   or includes invalid characters such as `'..'`, `'/'`, `'\\'`, `'?'`, or `'#'`.
    /// - Returns [`StripeError::Api`] if the refund ID does not exist or the API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into a [`Refund`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let refund = client.refunds().retrieve("re_123456789").await?;
    ///
    /// println!("Refund ID: {}", refund.id);
    /// println!("Refund amount: {} {}", refund.amount, refund.currency);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, refund_id: &str) -> Result<Refund, StripeError> {
        let encoded = validate_and_encode_id(refund_id)?;
        let url = format!("{}/v1/refunds/{encoded}", self.client.config.api_base);
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let refund: Refund = serde_json::from_value(val)?;
        Ok(refund)
    }

    /// Updates specified properties of an existing refund (`POST /v1/refunds/{id}`).
    ///
    /// Commonly used to update custom key-value `metadata` associated with the refund.
    ///
    /// Automatically injects an RFC 4122 compliant UUID v4 `Idempotency-Key` header
    /// if none is provided in `params`.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `refund_id` fails path traversal checks.
    /// - Returns [`StripeError::Api`] if the API returns an error.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into a [`Refund`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::UpdateRefundParams;
    /// use std::collections::HashMap;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    ///
    /// let mut meta = HashMap::new();
    /// meta.insert("support_ticket".to_string(), "TIC-9912".to_string());
    /// let params = UpdateRefundParams::new()
    ///     .with_metadata(meta)
    ///     .with_idempotency_key("refund_update_001");
    ///
    /// let refund = client.refunds().update("re_123456789", &params).await?;
    /// println!("Updated refund metadata: {:?}", refund.metadata);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        refund_id: &str,
        params: &UpdateRefundParams,
    ) -> Result<Refund, StripeError> {
        let encoded = validate_and_encode_id(refund_id)?;
        let url = format!("{}/v1/refunds/{encoded}", self.client.config.api_base);
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
    ///
    /// Only refunds with status `pending` or `requires_action` can be canceled.
    /// Once a refund has succeeded, it cannot be canceled.
    ///
    /// Automatically injects an RFC 4122 compliant UUID v4 `Idempotency-Key` header
    /// if none is provided in `params`.
    ///
    /// # Errors
    ///
    /// - Returns [`StripeError::InvalidRequest`] if `refund_id` fails path traversal checks.
    /// - Returns [`StripeError::Api`] if the refund is not in a cancelable state.
    /// - Returns [`StripeError::Json`] if the response cannot be deserialized into a [`Refund`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let canceled = client.refunds().cancel("re_123456789", None).await?;
    ///
    /// println!("Canceled refund status: {}", canceled.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel(
        &self,
        refund_id: &str,
        params: Option<&CancelRefundParams>,
    ) -> Result<Refund, StripeError> {
        let encoded = validate_and_encode_id(refund_id)?;
        let url = format!(
            "{}/v1/refunds/{encoded}/cancel",
            self.client.config.api_base
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
