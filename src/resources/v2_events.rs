//! Stripe API v2 Core Events resource implementation.
//!
//! Provides endpoints to retrieve individual v2 core events (`GET /v2/core/events/{id}`)
//! for live authenticated resource hydration and to list events (`GET /v2/core/events`).
//!
//! # Architecture & Quality Invariants:
//! - **Gate G-Borrow**: Zero deep clones; borrows parameters by reference (`&str`, `&ListV2EventsParams`).
//! - **Gate G-Docs**: Runnable doc-tests and complete rustdoc documentation.
//! - **Gate G-Complexity**: Cyclomatic complexity CC <= 6 on all functions. Max nesting depth <= 2.
//! - **Gate G2 & G-Law**: Path traversal protection via `validate_and_encode_id`, zero sensitive credential leaks.
//! - **Gate G3**: Strict `application/json` payload headers for Stripe API v2 operations.
//!
//! # Quickstart
//!
//! ```no_run
//! use some_stripe::{ListV2EventsParams, StripeClient};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = StripeClient::new("sk_test_123");
//! let events_resource = client.v2_events();
//!
//! // 1. Retrieve and hydrate a v2 core event by ID:
//! let event = events_resource.retrieve("evt_v2_sample_123").await?;
//! println!("Hydrated event type: {}", event.event_type());
//!
//! // 2. List v2 events with filtering parameters:
//! let params = ListV2EventsParams::new().limit(10);
//! let event_list = events_resource.list_events(Some(&params)).await?;
//! for evt in &event_list.data {
//!     println!("Event: {} ({})", evt.id, evt.event_type());
//! }
//! # Ok(())
//! # }
//! ```

use some_stripe_core::patterns::encode_form_pairs;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::v2_events::{ListV2EventsParams, V2Event, V2EventList};

/// Resource for interacting with Stripe API v2 Core Events (`stripe.v2().core().events()` or `stripe.v2_events()`).
#[derive(Clone, Copy)]
pub struct V2EventsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> V2EventsResource<'a> {
    /// Creates a new `V2EventsResource` bound to the provided [`StripeClient`].
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
    /// by attaching the `Stripe-Account` tenant header to outbound requests.
    #[must_use]
    pub fn with_account(&self, account_id: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account_id),
        }
    }

    /// Retrieves an authenticated Stripe API v2 event by ID (`GET /v2/core/events/{id}`).
    ///
    /// Executes a GET request with `Content-Type: application/json` and `Accept: application/json`
    /// to hydrate the full authenticated live state of an event received via a thin notification.
    ///
    /// # Errors
    /// - Returns [`StripeError::InvalidRequest`] if `event_id` is empty, contains whitespace,
    ///   or includes invalid characters such as `'..'`, `'/'`, `'\\'`, `'?'`, or `'#'`.
    /// - Returns [`StripeError::Api`] if the event does not exist or the Stripe API returns an error.
    /// - Returns [`StripeError::Json`] if the response body cannot be deserialized into a [`V2Event`].
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let event = client.v2_events().retrieve("evt_v2_123").await?;
    /// assert_eq!(event.id, "evt_v2_123");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, event_id: &str) -> Result<V2Event, StripeError> {
        let encoded_id = validate_and_encode_id(event_id)?;
        let url = format!("{}/v2/core/events/{}", self.client.config.api_base, encoded_id);
        let mut req = self.client.http.get(&url);
        req = req.header(reqwest::header::CONTENT_TYPE, "application/json");
        req = req.header(reqwest::header::ACCEPT, "application/json");
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }
        let val = self.client.execute(req).await?;
        let event: V2Event = serde_json::from_value(val)?;
        Ok(event)
    }

    /// Lists Stripe API v2 events matching the specified query criteria (`GET /v2/core/events`).
    ///
    /// Returns the raw JSON response [`serde_json::Value`] representing the page of events.
    ///
    /// # Errors
    /// - Returns [`StripeError::Api`] if the API returns an error response.
    /// - Returns [`StripeError::Json`] if the response cannot be parsed as JSON.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::{ListV2EventsParams, StripeClient};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = ListV2EventsParams::new().limit(5);
    /// let val = client.v2_events().list(Some(&params)).await?;
    /// println!("V2 events raw JSON: {}", val);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        &self,
        params: Option<&ListV2EventsParams>,
    ) -> Result<serde_json::Value, StripeError> {
        let mut url = format!("{}/v2/core/events", self.client.config.api_base);
        if let Some(p) = params {
            let query_pairs = p.to_query_params();
            if !query_pairs.is_empty() {
                url.push('?');
                url.push_str(&encode_form_pairs(&query_pairs));
            }
        }
        let mut req = self.client.http.get(&url);
        req = req.header(reqwest::header::CONTENT_TYPE, "application/json");
        req = req.header(reqwest::header::ACCEPT, "application/json");
        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }
        self.client.execute(req).await
    }

    /// Lists Stripe API v2 events as a strongly typed [`V2EventList`] response (`GET /v2/core/events`).
    ///
    /// # Errors
    /// - Returns [`StripeError`] if the request fails or deserialization into [`V2EventList`] fails.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::{ListV2EventsParams, StripeClient};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = ListV2EventsParams::new().limit(10);
    /// let list = client.v2_events().list_events(Some(&params)).await?;
    /// assert!(list.data.len() <= 10);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_events(
        &self,
        params: Option<&ListV2EventsParams>,
    ) -> Result<V2EventList, StripeError> {
        let val = self.list(params).await?;
        let list: V2EventList = serde_json::from_value(val)?;
        Ok(list)
    }
}
