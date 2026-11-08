//! Stripe API V2 Billing Meters management resource (`/v2/billing/meters`).
//!
//! Provides lifecycle configuration for billing meters, including aggregation formulas,
//! customer mapping, activation, deactivation, and status inspection.
//!
//! # Architecture & Quality Invariants
//! - **Gate G-Borrow**: Zero-copy borrowing across string slices and parameter models.
//! - **Gate G-Complexity**: Cyclomatic complexity $CC \le 5$ for all resource methods.
//! - **Gate G-Docs**: Comprehensive rustdoc documentation and examples.

use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{
    CreateBillingMeterParams, ListBillingMetersParams, UpdateBillingMeterParams, V2BillingMeter,
};

use crate::resources::checkout::validate_and_encode_id;

/// Resource client for Stripe V2 Billing Meters.
#[derive(Clone, Copy)]
pub struct BillingMetersResource<'a> {
    client: &'a StripeClient,
}

impl<'a> BillingMetersResource<'a> {
    /// Creates a new `BillingMetersResource` bound to the provided client.
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates a new billing meter configuration.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if parameters fail validation.
    pub async fn create(
        &self,
        params: &CreateBillingMeterParams,
    ) -> Result<V2BillingMeter, StripeError> {
        self.validate_create_params(params)?;

        let url = format!("{}/v2/billing/meters", self.client.config.api_base);
        let req = self.client.http.post(&url).json(params);

        let val = self.client.execute(req).await?;
        let meter: V2BillingMeter = serde_json::from_value(val)?;
        Ok(meter)
    }

    /// Retrieves an existing billing meter by its ID (`mtr_...`).
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if the ID is invalid or empty.
    pub async fn retrieve(&self, id: &str) -> Result<V2BillingMeter, StripeError> {
        let encoded_id = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/billing/meters/{encoded_id}",
            self.client.config.api_base
        );
        let req = self.client.http.get(&url);

        let val = self.client.execute(req).await?;
        let meter: V2BillingMeter = serde_json::from_value(val)?;
        Ok(meter)
    }

    /// Updates an existing billing meter's configuration.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if the ID is invalid.
    pub async fn update(
        &self,
        id: &str,
        params: &UpdateBillingMeterParams,
    ) -> Result<V2BillingMeter, StripeError> {
        let encoded_id = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/billing/meters/{encoded_id}",
            self.client.config.api_base
        );
        let req = self.client.http.post(&url).json(params);

        let val = self.client.execute(req).await?;
        let meter: V2BillingMeter = serde_json::from_value(val)?;
        Ok(meter)
    }

    /// Deactivates a billing meter, stopping future usage ingestion.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if the ID is invalid.
    pub async fn deactivate(&self, id: &str) -> Result<V2BillingMeter, StripeError> {
        let encoded_id = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/billing/meters/{encoded_id}/deactivate",
            self.client.config.api_base
        );
        let req = self.client.http.post(&url).json(&serde_json::json!({}));

        let val = self.client.execute(req).await?;
        let meter: V2BillingMeter = serde_json::from_value(val)?;
        Ok(meter)
    }

    /// Reactivates an inactive billing meter.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if the ID is invalid.
    pub async fn reactivate(&self, id: &str) -> Result<V2BillingMeter, StripeError> {
        let encoded_id = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/billing/meters/{encoded_id}/reactivate",
            self.client.config.api_base
        );
        let req = self.client.http.post(&url).json(&serde_json::json!({}));

        let val = self.client.execute(req).await?;
        let meter: V2BillingMeter = serde_json::from_value(val)?;
        Ok(meter)
    }

    /// Lists billing meters configured on this Stripe account.
    ///
    /// # Errors
    /// Returns [`StripeError`] on network or API failures.
    pub async fn list(
        &self,
        params: Option<&ListBillingMetersParams>,
    ) -> Result<Vec<V2BillingMeter>, StripeError> {
        let mut url = format!("{}/v2/billing/meters", self.client.config.api_base);

        if let Some(p) = params {
            let query_params = self.build_query_params(p);
            append_query_string(&mut url, &query_params);
        }

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let meters: Vec<V2BillingMeter> = match val {
            serde_json::Value::Array(items) => {
                serde_json::from_value(serde_json::Value::Array(items))?
            }
            serde_json::Value::Object(map) => {
                if let Some(data) = map.get("data") {
                    serde_json::from_value(data.clone())?
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        };
        Ok(meters)
    }

    fn validate_create_params(&self, params: &CreateBillingMeterParams) -> Result<(), StripeError> {
        if params.display_name.trim().is_empty() {
            return Err(StripeError::InvalidRequest(
                "display_name cannot be empty".to_string(),
            ));
        }
        if params.event_name.trim().is_empty() {
            return Err(StripeError::InvalidRequest(
                "event_name cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    fn build_query_params(&self, params: &ListBillingMetersParams) -> Vec<(&'static str, String)> {
        let mut query = Vec::new();
        if let Some(lim) = params.limit {
            query.push(("limit", lim.to_string()));
        }
        if let Some(st) = params.status {
            let status_str = match st {
                some_stripe_types::MeterStatus::Active => "active",
                some_stripe_types::MeterStatus::Inactive => "inactive",
            };
            query.push(("status", status_str.to_string()));
        }
        query
    }
}

fn append_query_string(url: &mut String, query_params: &[(&'static str, String)]) {
    if query_params.is_empty() {
        return;
    }
    let encoded_pairs: Vec<String> = query_params
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    url.push('?');
    url.push_str(&encoded_pairs.join("&"));
}
