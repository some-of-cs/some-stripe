//! Stripe API v2 Core Events resource implementation (`GET /v2/core/events`).
//!
//! Provides retrieval of full live authenticated event state from Stripe API v2,
//! used in thin event hydration workflows.
//!
//! # Architecture & Quality Invariants
//! - **Gate G-Borrow**: Zero-copy borrowing of string slices and parameter schemas.
//! - **Gate G-Complexity**: Cyclomatic complexity $CC \le 6$ for all methods.
//! - **Gate G-Docs**: Comprehensive rustdoc documentation and runnable doctests.

use some_stripe_core::patterns::crud::try_resource_url_with_id;
use some_stripe_core::{RequestOptions, StripeClient, StripeError};
use some_stripe_types::{ListV2EventsParams, V2Event, V2List};

/// Resource for accessing Stripe API v2 Core Events (`stripe.v2().core().events()`).
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

    /// Scopes this resource to operate on behalf of a specific connected account (`Stripe-Account` header).
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

    /// Retrieves an authenticated Stripe API v2 event by ID (`GET /v2/core/events/{id}`).
    ///
    /// # Errors
    /// Returns [`StripeError`] if the ID contains invalid control characters or path traversals,
    /// transport failure occurs, or the API returns a non-2xx status code.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let event = client.v2().core().events().retrieve("v2_evt_abc123").await?;
    /// println!("Event type: {}", event.event_type());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, id: &str) -> Result<V2Event, StripeError> {
        self.retrieve_with_options(id, None).await
    }

    /// Retrieves an authenticated Stripe API v2 event with explicit [`RequestOptions`].
    pub async fn retrieve_with_options(
        &self,
        id: &str,
        options: Option<&RequestOptions>,
    ) -> Result<V2Event, StripeError> {
        let url = try_resource_url_with_id(&self.client.config.api_base, "/v2/core/events", id)?;

        let mut req = self
            .client
            .http
            .get(&url)
            .header("Content-Type", "application/json");

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute_with_options(req, options).await?;
        let event: V2Event = serde_json::from_value(val)?;
        Ok(event)
    }

    /// Lists Stripe API v2 events (`GET /v2/core/events`).
    ///
    /// # Errors
    /// Returns [`StripeError`] if network transport fails or non-2xx status is returned.
    pub async fn list(&self, params: &ListV2EventsParams) -> Result<V2List<V2Event>, StripeError> {
        self.list_with_options(params, None).await
    }

    /// Lists Stripe API v2 events with explicit [`RequestOptions`].
    pub async fn list_with_options(
        &self,
        params: &ListV2EventsParams,
        options: Option<&RequestOptions>,
    ) -> Result<V2List<V2Event>, StripeError> {
        let mut url = format!("{}/v2/core/events", self.client.config.api_base);

        let mut query_params: Vec<(&str, std::borrow::Cow<'_, str>)> = Vec::new();
        if let Some(limit) = params.limit {
            query_params.push(("limit", std::borrow::Cow::Owned(limit.to_string())));
        }
        if let Some(ref et) = params.event_type {
            query_params.push(("type", std::borrow::Cow::Borrowed(et.as_str())));
        }
        if let Some(ref page) = params.page {
            query_params.push(("page", std::borrow::Cow::Borrowed(page.as_str())));
        }

        if !query_params.is_empty() {
            let qs = query_params
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&");
            url = format!("{url}?{qs}");
        }

        let mut req = self
            .client
            .http
            .get(&url)
            .header("Content-Type", "application/json");

        if let Some(acct) = self.account {
            req = req.header("Stripe-Account", acct);
        }

        let val = self.client.execute_with_options(req, options).await?;
        let list: V2List<V2Event> = serde_json::from_value(val)?;
        Ok(list)
    }
}
