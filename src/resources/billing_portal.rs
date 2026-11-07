//! Stripe Customer Billing Portal resource implementation.
//!
//! Provides endpoints to create self-service billing portal sessions,
//! create, retrieve, update, and list portal configurations,
//! enforcing open redirect prevention and RFC 4122 UUID v4 idempotency.

use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CreatePortalConfigurationParams, CreatePortalSessionParams, PortalBusinessProfile,
    PortalConfiguration, PortalConfigurationList, PortalSession, UpdatePortalConfigurationParams,
    validate_portal_url,
};

/// Resource for managing Stripe Customer Billing Portal (`stripe.billing_portal()`).
#[derive(Clone, Copy)]
pub struct BillingPortalResource<'a> {
    client: &'a StripeClient,
}

impl<'a> BillingPortalResource<'a> {
    /// Creates a new `BillingPortalResource` bound to the provided [`StripeClient`].
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates a new customer billing portal session (`POST /v1/billing_portal/sessions`).
    ///
    /// Validates `return_url` to prevent open redirect vulnerabilities (Gate G2).
    /// Generates RFC 4122 UUID v4 idempotency key when not explicitly provided.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if `return_url` is malformed, uses an insecure protocol,
    /// or if the customer ID contains invalid characters.
    pub async fn create_session(
        &self,
        params: &CreatePortalSessionParams,
    ) -> Result<PortalSession, StripeError> {
        if let Some(ref return_url) = params.return_url {
            validate_portal_url(return_url)
                .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;
        }

        validate_and_encode_id(&params.customer)?;

        let url = format!("{}/v1/billing_portal/sessions", self.client.config.api_base);
        let mut form_data = Vec::new();

        form_data.push(("customer", params.customer.as_str()));

        if let Some(ref c) = params.configuration {
            form_data.push(("configuration", c.as_str()));
        }
        if let Some(ref r) = params.return_url {
            form_data.push(("return_url", r.as_str()));
        }
        if let Some(ref o) = params.on_behalf_of {
            form_data.push(("on_behalf_of", o.as_str()));
        }

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let session: PortalSession = serde_json::from_value(val)?;
        Ok(session)
    }

    /// Creates a new billing portal configuration (`POST /v1/billing_portal/configurations`).
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if any provided URLs fail security validation.
    pub async fn create_configuration(
        &self,
        params: &CreatePortalConfigurationParams,
    ) -> Result<PortalConfiguration, StripeError> {
        validate_portal_configuration_urls(
            params.default_return_url.as_deref(),
            params.business_profile.as_ref(),
        )?;

        let url = format!(
            "{}/v1/billing_portal/configurations",
            self.client.config.api_base
        );
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let config: PortalConfiguration = serde_json::from_value(val)?;
        Ok(config)
    }

    /// Retrieves an existing portal configuration by ID (`GET /v1/billing_portal/configurations/{id}`).
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if `id` contains path traversal or invalid characters.
    pub async fn retrieve_configuration(
        &self,
        id: &str,
    ) -> Result<PortalConfiguration, StripeError> {
        let safe_id = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v1/billing_portal/configurations/{}",
            self.client.config.api_base, safe_id
        );

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let config: PortalConfiguration = serde_json::from_value(val)?;
        Ok(config)
    }

    /// Updates an existing portal configuration (`POST /v1/billing_portal/configurations/{id}`).
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if `id` or any provided URLs are invalid.
    pub async fn update_configuration(
        &self,
        id: &str,
        params: &UpdatePortalConfigurationParams,
    ) -> Result<PortalConfiguration, StripeError> {
        let safe_id = validate_and_encode_id(id)?;
        validate_portal_configuration_urls(
            params.default_return_url.as_deref(),
            params.business_profile.as_ref(),
        )?;

        let url = format!(
            "{}/v1/billing_portal/configurations/{}",
            self.client.config.api_base, safe_id
        );
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let config: PortalConfiguration = serde_json::from_value(val)?;
        Ok(config)
    }

    /// Lists billing portal configurations (`GET /v1/billing_portal/configurations`).
    ///
    /// # Errors
    /// Returns [`StripeError`] on network or API failure.
    pub async fn list_configurations(
        &self,
        limit: Option<u32>,
    ) -> Result<PortalConfigurationList, StripeError> {
        let mut url = format!(
            "{}/v1/billing_portal/configurations",
            self.client.config.api_base
        );
        if let Some(l) = limit {
            use std::fmt::Write;
            let _ = write!(url, "?limit={l}");
        }
        let req = self.client.http.get(&url);

        let val = self.client.execute(req).await?;
        let list: PortalConfigurationList = serde_json::from_value(val)?;
        Ok(list)
    }
}

fn validate_portal_configuration_urls(
    default_return_url: Option<&str>,
    business_profile: Option<&PortalBusinessProfile>,
) -> Result<(), StripeError> {
    if let Some(return_url) = default_return_url {
        validate_portal_url(return_url).map_err(|e| StripeError::InvalidRequest(e.to_string()))?;
    }
    if let Some(bp) = business_profile {
        if let Some(ref privacy_url) = bp.privacy_policy_url {
            validate_portal_url(privacy_url)
                .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;
        }
        if let Some(ref terms_url) = bp.terms_of_service_url {
            validate_portal_url(terms_url)
                .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;
        }
    }
    Ok(())
}
