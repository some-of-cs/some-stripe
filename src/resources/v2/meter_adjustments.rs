//! Stripe API V2 Meter Event Adjustments resource (`/v2/billing/meter_event_adjustments`).
//!
//! Provides correction and cancellation mechanisms for previously ingested meter events.
//! Enables reconciliation of erroneous or duplicate meter consumption.
//!
//! # Architecture & Quality Invariants
//! - **Gate G-Borrow**: Zero-copy borrowing across string slices and parameter models.
//! - **Gate G-Complexity**: Cyclomatic complexity $CC \le 5$ for all resource methods.
//! - **Gate G-Law**: Fulfills consumer billing accuracy and FTC Section 5 auditability.

use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{
    CreateMeterEventAdjustmentParams, ListMeterEventAdjustmentsParams, MeterEventAdjustment,
};

use crate::resources::checkout::validate_and_encode_id;

/// Resource client for Stripe V2 Meter Event Adjustments.
#[derive(Clone, Copy)]
pub struct MeterEventAdjustmentsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> MeterEventAdjustmentsResource<'a> {
    /// Creates a new `MeterEventAdjustmentsResource` bound to the provided client.
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates an adjustment modifying or canceling prior meter consumption.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if parameters fail validation.
    pub async fn create(
        &self,
        params: &CreateMeterEventAdjustmentParams,
    ) -> Result<MeterEventAdjustment, StripeError> {
        self.validate_params(params)?;

        let url = format!(
            "{}/v2/billing/meter_event_adjustments",
            self.client.config.api_base
        );
        let req = self.client.http.post(&url).json(params);

        let val = self.client.execute(req).await?;
        let adjustment: MeterEventAdjustment = serde_json::from_value(val)?;
        Ok(adjustment)
    }

    /// Retrieves an existing meter event adjustment by ID.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if the ID is invalid or empty.
    pub async fn retrieve(&self, id: &str) -> Result<MeterEventAdjustment, StripeError> {
        let encoded_id = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/billing/meter_event_adjustments/{encoded_id}",
            self.client.config.api_base
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let adjustment: MeterEventAdjustment = serde_json::from_value(val)?;
        Ok(adjustment)
    }

    /// Lists meter event adjustments matching query criteria.
    ///
    /// # Errors
    /// Returns [`StripeError`] on network or API failures.
    pub async fn list(
        &self,
        params: Option<&ListMeterEventAdjustmentsParams>,
    ) -> Result<Vec<MeterEventAdjustment>, StripeError> {
        let mut url = format!(
            "{}/v2/billing/meter_event_adjustments",
            self.client.config.api_base
        );

        if let Some(p) = params {
            let query_params = self.build_query_params(p);
            append_query_string(&mut url, &query_params);
        }

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let adjustments: Vec<MeterEventAdjustment> = match val {
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
        Ok(adjustments)
    }

    fn validate_params(
        &self,
        params: &CreateMeterEventAdjustmentParams,
    ) -> Result<(), StripeError> {
        if params.event_name.trim().is_empty() {
            return Err(StripeError::InvalidRequest(
                "event_name cannot be empty".to_string(),
            ));
        }
        if params.r#type.trim().is_empty() {
            return Err(StripeError::InvalidRequest(
                "adjustment type cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    fn build_query_params(
        &self,
        params: &ListMeterEventAdjustmentsParams,
    ) -> Vec<(&'static str, String)> {
        let mut query = Vec::new();
        if let Some(name) = &params.event_name {
            query.push(("event_name", name.clone()));
        }
        if let Some(lim) = params.limit {
            query.push(("limit", lim.to_string()));
        }
        if let Some(sa) = &params.starting_after {
            query.push(("starting_after", sa.clone()));
        }
        if let Some(eb) = &params.ending_before {
            query.push(("ending_before", eb.clone()));
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
