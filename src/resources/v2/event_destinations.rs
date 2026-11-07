//! Stripe API v2 Core Event Destinations resource implementation (`/v2/core/event_destinations`).
//!
//! Provides management of enterprise event destinations (Amazon EventBridge, Azure Event Grid, Webhooks).
//!
//! # Architecture & Quality Invariants
//! - **Gate G-Borrow**: Parameters borrowed by reference; zero deep clones.
//! - **Gate G-Complexity**: Cyclomatic complexity $CC \le 6$ for each method.
//! - **Gate G-Docs**: Comprehensive rustdoc documentation and runnable doctests.

use some_stripe_core::patterns::crud::{resource_sub_url, try_resource_url_with_id};
use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::{
    CreateEventDestinationParams, EventDestination, UpdateEventDestinationParams, V2List,
};

/// Resource for managing Stripe API v2 Event Destinations (`stripe.v2().core().event_destinations()`).
#[derive(Clone, Copy)]
pub struct EventDestinationsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> EventDestinationsResource<'a> {
    /// Creates a new `EventDestinationsResource` bound to the provided [`StripeClient`].
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Scopes this resource to operate on behalf of a specific connected account.
    #[must_use]
    pub fn with_account(&self, account: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account),
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates an event destination (`POST /v2/core/event_destinations`).
    ///
    /// # Errors
    /// Returns [`StripeError`] if the request fails or API returns an error status.
    pub async fn create(
        &self,
        params: &CreateEventDestinationParams,
    ) -> Result<EventDestination, StripeError> {
        let url = format!("{}/v2/core/event_destinations", self.client.config.api_base);

        let mut req = self
            .client
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .json(params);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let destination: EventDestination = serde_json::from_value(val)?;
        Ok(destination)
    }

    /// Retrieves an event destination by ID (`GET /v2/core/event_destinations/{id}`).
    pub async fn retrieve(&self, id: &str) -> Result<EventDestination, StripeError> {
        let url =
            try_resource_url_with_id(&self.client.config.api_base, "/v2/core/event_destinations", id)?;

        let mut req = self
            .client
            .http
            .get(&url)
            .header("Content-Type", "application/json");

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let destination: EventDestination = serde_json::from_value(val)?;
        Ok(destination)
    }

    /// Updates an event destination (`POST /v2/core/event_destinations/{id}`).
    pub async fn update(
        &self,
        id: &str,
        params: &UpdateEventDestinationParams,
    ) -> Result<EventDestination, StripeError> {
        let url =
            try_resource_url_with_id(&self.client.config.api_base, "/v2/core/event_destinations", id)?;

        let mut req = self
            .client
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .json(params);

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let destination: EventDestination = serde_json::from_value(val)?;
        Ok(destination)
    }

    /// Deletes an event destination (`DELETE /v2/core/event_destinations/{id}`).
    pub async fn del(&self, id: &str) -> Result<EventDestination, StripeError> {
        let url =
            try_resource_url_with_id(&self.client.config.api_base, "/v2/core/event_destinations", id)?;

        let mut req = self
            .client
            .http
            .delete(&url)
            .header("Content-Type", "application/json");

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let destination: EventDestination = serde_json::from_value(val)?;
        Ok(destination)
    }

    /// Lists event destinations (`GET /v2/core/event_destinations`).
    pub async fn list(&self) -> Result<V2List<EventDestination>, StripeError> {
        let url = format!("{}/v2/core/event_destinations", self.client.config.api_base);

        let mut req = self
            .client
            .http
            .get(&url)
            .header("Content-Type", "application/json");

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let list: V2List<EventDestination> = serde_json::from_value(val)?;
        Ok(list)
    }

    /// Enables an event destination (`POST /v2/core/event_destinations/{id}/enable`).
    pub async fn enable(&self, id: &str) -> Result<EventDestination, StripeError> {
        let url = resource_sub_url(
            &self.client.config.api_base,
            "/v2/core/event_destinations",
            id,
            "enable",
        )?;

        let mut req = self
            .client
            .http
            .post(&url)
            .header("Content-Type", "application/json");

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let destination: EventDestination = serde_json::from_value(val)?;
        Ok(destination)
    }

    /// Disables an event destination (`POST /v2/core/event_destinations/{id}/disable`).
    pub async fn disable(&self, id: &str) -> Result<EventDestination, StripeError> {
        let url = resource_sub_url(
            &self.client.config.api_base,
            "/v2/core/event_destinations",
            id,
            "disable",
        )?;

        let mut req = self
            .client
            .http
            .post(&url)
            .header("Content-Type", "application/json");

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let destination: EventDestination = serde_json::from_value(val)?;
        Ok(destination)
    }

    /// Sends a ping notification test event to the destination (`POST /v2/core/event_destinations/{id}/ping`).
    pub async fn ping(&self, id: &str) -> Result<serde_json::Value, StripeError> {
        let url = resource_sub_url(
            &self.client.config.api_base,
            "/v2/core/event_destinations",
            id,
            "ping",
        )?;

        let mut req = self
            .client
            .http
            .post(&url)
            .header("Content-Type", "application/json");

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        self.client.execute(req).await
    }
}
