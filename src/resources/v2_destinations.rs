//! Stripe API v2 Event Destinations resource implementation (`/v2/core/event_destinations`).
//!
//! Provides CRUD operations and operational lifecycle actions (enable, disable, ping)
//! for Stripe API v2 Event Destinations including Amazon EventBridge, Azure Event Grid,
//! and HTTPS Webhook endpoints.
//!
//! # Architecture & Quality Invariants
//! - **Gate G-Borrow**: Zero-copy borrowing across parameter models and string slices.
//! - **Gate G-Complexity**: Cyclomatic complexity $CC \le 4$ across all endpoint methods.
//! - **Gate G-Docs**: Comprehensive rustdoc documentation and runnable doc-tests.
//! - **Gate G2**: Strict ID validation protecting against path traversal (`..`, `/`, `\`)
//!   and query injection attacks (`?`, `#`).
//!
//! # Example
//! ```no_run
//! use some_stripe::StripeClient;
//! use some_stripe_types::v2_destination::CreateEventDestinationParams;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = StripeClient::new("sk_test_123");
//! let params = CreateEventDestinationParams::amazon_eventbridge(
//!     "123456789012",
//!     "us-east-1",
//!     vec!["v2.billing.meter.error_report_triggered".to_string()],
//! );
//!
//! let destination = client.event_destinations().create(&params).await?;
//! println!("Created destination ID: {}", destination.id);
//! # Ok(())
//! # }
//! ```

use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::v2_destination::{
    CreateEventDestinationParams, EventDestination, UpdateEventDestinationParams,
};

#[derive(serde::Deserialize)]
struct ListEnvelopeHelper {
    data: Vec<EventDestination>,
}

fn parse_destination_list(val: serde_json::Value) -> Result<Vec<EventDestination>, StripeError> {
    if val.is_array() {
        return serde_json::from_value(val).map_err(StripeError::Json);
    }
    let envelope: ListEnvelopeHelper = serde_json::from_value(val)?;
    Ok(envelope.data)
}

/// Resource for managing Stripe API v2 Event Destinations (`stripe.event_destinations()`).
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

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this resource to operate on behalf of a specific connected account
    /// by attaching the `Stripe-Account` tenant header to all outbound requests.
    #[must_use]
    pub fn with_account(&self, account: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account),
        }
    }

    /// Helper for dispatching action POST requests (`/v2/core/event_destinations/{id}/{action}`).
    async fn send_action_post(
        &self,
        id: &str,
        action: &str,
    ) -> Result<EventDestination, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/core/event_destinations/{encoded}/{action}",
            self.client.config.api_base
        );
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

    /// Creates a new Stripe API v2 Event Destination (`POST /v2/core/event_destinations`).
    ///
    /// # Errors
    /// - Returns [`StripeError::InvalidRequest`] if parameters fail validation.
    /// - Returns [`StripeError::Api`] if the Stripe API returns a non-2xx status code.
    /// - Returns [`StripeError::Json`] if deserialization fails.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::v2_destination::CreateEventDestinationParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CreateEventDestinationParams::amazon_eventbridge(
    ///     "123456789012",
    ///     "us-east-1",
    ///     vec!["v2.billing.meter.error_report_triggered".to_string()],
    /// );
    ///
    /// let destination = client.event_destinations().create(&params).await?;
    /// assert_eq!(destination.object, "v2.core.event_destination");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        params: &CreateEventDestinationParams,
    ) -> Result<EventDestination, StripeError> {
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

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

    /// Retrieves an existing Event Destination by ID (`GET /v2/core/event_destinations/{id}`).
    ///
    /// # Errors
    /// - Returns [`StripeError::InvalidRequest`] if the ID is invalid or contains path traversal characters.
    /// - Returns [`StripeError::Api`] if the destination does not exist.
    /// - Returns [`StripeError::Json`] if response deserialization fails.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let destination = client.event_destinations().retrieve("edest_123").await?;
    /// println!("Status: {}", destination.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, id: &str) -> Result<EventDestination, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/core/event_destinations/{encoded}",
            self.client.config.api_base
        );

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let destination: EventDestination = serde_json::from_value(val)?;
        Ok(destination)
    }

    /// Updates an existing Event Destination (`POST /v2/core/event_destinations/{id}`).
    ///
    /// # Errors
    /// - Returns [`StripeError::InvalidRequest`] if the ID is invalid or update params fail validation.
    /// - Returns [`StripeError::Api`] if the destination cannot be updated.
    /// - Returns [`StripeError::Json`] if response deserialization fails.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::v2_destination::UpdateEventDestinationParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = UpdateEventDestinationParams::new()
    ///     .with_description("Updated destination description");
    ///
    /// let updated = client.event_destinations().update("edest_123", &params).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        id: &str,
        params: &UpdateEventDestinationParams,
    ) -> Result<EventDestination, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!(
            "{}/v2/core/event_destinations/{encoded}",
            self.client.config.api_base
        );

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

    /// Deletes an existing Event Destination (`DELETE /v2/core/event_destinations/{id}`).
    ///
    /// # Errors
    /// - Returns [`StripeError::InvalidRequest`] if the destination ID is invalid.
    /// - Returns [`StripeError::Api`] if the destination does not exist or cannot be deleted.
    /// - Returns [`StripeError::Json`] if response deserialization fails.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let deleted = client.event_destinations().del("edest_123").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn del(&self, id: &str) -> Result<EventDestination, StripeError> {
        let encoded = validate_and_encode_id(id)?;
        let url = format!(
            "{}/v2/core/event_destinations/{encoded}",
            self.client.config.api_base
        );

        let mut req = self.client.http.delete(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        let destination: EventDestination = serde_json::from_value(val)?;
        Ok(destination)
    }

    /// Alias for [`del`](Self::del) to delete an existing Event Destination.
    pub async fn delete(&self, id: &str) -> Result<EventDestination, StripeError> {
        self.del(id).await
    }

    /// Lists configured Event Destinations (`GET /v2/core/event_destinations`).
    ///
    /// # Errors
    /// - Returns [`StripeError::Api`] if the Stripe API returns a non-2xx status code.
    /// - Returns [`StripeError::Json`] if response deserialization fails.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let destinations = client.event_destinations().list().await?;
    /// println!("Found {} event destinations", destinations.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self) -> Result<Vec<EventDestination>, StripeError> {
        let url = format!("{}/v2/core/event_destinations", self.client.config.api_base);

        let mut req = self.client.http.get(&url);
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute(req).await?;
        parse_destination_list(val)
    }

    /// Enables an existing Event Destination (`POST /v2/core/event_destinations/{id}/enable`).
    ///
    /// # Errors
    /// - Returns [`StripeError::InvalidRequest`] if the ID is invalid.
    /// - Returns [`StripeError::Api`] if enabling fails.
    /// - Returns [`StripeError::Json`] if response deserialization fails.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let destination = client.event_destinations().enable("edest_123").await?;
    /// assert!(destination.is_enabled());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn enable(&self, id: &str) -> Result<EventDestination, StripeError> {
        self.send_action_post(id, "enable").await
    }

    /// Disables an existing Event Destination (`POST /v2/core/event_destinations/{id}/disable`).
    ///
    /// # Errors
    /// - Returns [`StripeError::InvalidRequest`] if the ID is invalid.
    /// - Returns [`StripeError::Api`] if disabling fails.
    /// - Returns [`StripeError::Json`] if response deserialization fails.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let destination = client.event_destinations().disable("edest_123").await?;
    /// assert!(destination.is_disabled());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn disable(&self, id: &str) -> Result<EventDestination, StripeError> {
        self.send_action_post(id, "disable").await
    }

    /// Sends a test ping to an existing Event Destination (`POST /v2/core/event_destinations/{id}/ping`).
    ///
    /// # Errors
    /// - Returns [`StripeError::InvalidRequest`] if the ID is invalid.
    /// - Returns [`StripeError::Api`] if pinging fails.
    /// - Returns [`StripeError::Json`] if response deserialization fails.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let destination = client.event_destinations().ping("edest_123").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn ping(&self, id: &str) -> Result<EventDestination, StripeError> {
        self.send_action_post(id, "ping").await
    }
}
