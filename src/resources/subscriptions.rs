//! Stripe Subscriptions resource implementations.
//!
//! Provides bindings for managing recurring Stripe Subscriptions (`stripe.subscriptions`),
//! enabling SaaS applications to automate multi-item billing, trials, plan migrations,
//! prorations, dunning retries, and pause/resume lifecycles.
//!
//! # Architecture & Quality Gates:
//! - **G0 (Rust 2024 Edition)**: Clean compilation with 0 clippy warnings and strict code formatting.
//! - **G-Borrow (Zero-Clone Discipline)**: Zero-copy accessors (`sub.id()`, `sub.customer()`)
//!   and streaming subscriptions via [`Paginator`].
//! - **G2 (Security & Idempotency)**: Automatic injection of RFC 4122 UUID v4 `Idempotency-Key` headers
//!   on mutable operations (`create`, `update`, `cancel`, `resume`) when omitted by the caller,
//!   and strict ID validation against CRLF/null-byte injection.
//! - **Gate G-Law (ISO 4217 & Minor Currency Units)**: Minor integer unit amounts for pricing
//!   and EU consumer withdrawal compliance safeguards.
//!
//! # Quickstart
//! ```no_run
//! use some_stripe::StripeClient;
//! use some_stripe_types::{CreateSubscriptionParams, SubscriptionItemParams, ProrationBehavior};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = StripeClient::new("sk_test_123");
//! let items = vec![SubscriptionItemParams::new("price_monthly_seat", 5)];
//! let params = CreateSubscriptionParams::new("cus_12345", items)
//!     .trial_period_days(14)
//!     .proration_behavior(ProrationBehavior::CreateProrations);
//!
//! let subscription = client.subscriptions().create(&params).await?;
//! println!("Subscription active: {}", subscription.id());
//! # Ok(())
//! # }
//! ```

use crate::resources::generate_uuid_v4;
use crate::resources::subscription_items::SubscriptionItemsResource;
use some_stripe_core::patterns::pagination::{ListResponse, Paginator};
use some_stripe_core::patterns::serializer::{encode_form_pairs, percent_encode, to_form_params};
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::subscription::{
    CancelSubscriptionParams, CreateSubscriptionParams, ListSubscriptionsParams,
    ResumeSubscriptionParams, SearchSubscriptionsParams, Subscription, UpdateSubscriptionParams,
    subscription_id,
};

/// Resource for managing Stripe Subscriptions (`stripe.subscriptions`).
///
/// Accessed via `client.subscriptions()`.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
///
/// let client = StripeClient::new("sk_test_123");
/// let subs = client.subscriptions();
/// let items = subs.items;
/// ```
#[derive(Clone, Copy)]
pub struct SubscriptionsResource<'a> {
    client: &'a StripeClient,
    /// Subscription items sub-resource (`stripe.subscriptions.items`).
    pub items: SubscriptionItemsResource<'a>,
}

impl<'a> SubscriptionsResource<'a> {
    /// Creates a new `SubscriptionsResource` bound to the provided client.
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            items: SubscriptionItemsResource::new(client),
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Access the Subscription Items sub-resource (`stripe.subscriptions.items`).
    pub fn items(&self) -> &SubscriptionItemsResource<'a> {
        &self.items
    }

    /// Creates a new subscription (`POST /v1/subscriptions`).
    ///
    /// Supports multi-item indexed form serialization (`items[0][price]=...`),
    /// trial configurations, proration rules, and automatic RFC 4122 UUID v4
    /// `Idempotency-Key` injection when omitted.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if parameter validation fails,
    /// or network/API errors returned by Stripe.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::{CreateSubscriptionParams, SubscriptionItemParams};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let items = vec![SubscriptionItemParams::new("price_pro_seat", 10)];
    /// let params = CreateSubscriptionParams::new("cus_12345", items)
    ///     .trial_period_days(30)
    ///     .idempotency_key("sub_create_idem_001");
    ///
    /// let sub = client.subscriptions().create(&params).await?;
    /// assert_eq!(sub.customer(), "cus_12345");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        params: &CreateSubscriptionParams,
    ) -> Result<Subscription, StripeError> {
        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!("{}/v1/subscriptions", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let generated_key;
        let idempotency_key = match params.idempotency_key.as_deref() {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = generate_uuid_v4();
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
        let sub: Subscription = serde_json::from_value(val)?;
        Ok(sub)
    }

    /// Retrieves an existing Subscription by ID (`GET /v1/subscriptions/:id`).
    ///
    /// Accepts the identifier as a string reference (`&str`), ensuring zero allocations.
    /// Validates against CRLF/null-byte injection before executing the request.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if the ID is invalid or not found.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let sub = client.subscriptions().retrieve("sub_12345").await?;
    /// println!("Status: {}", sub.status());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(&self, id: &str) -> Result<Subscription, StripeError> {
        let safe_id =
            validate_and_encode_id(id).map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!(
            "{}/v1/subscriptions/{}",
            self.client.config.api_base, safe_id
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let sub: Subscription = serde_json::from_value(val)?;
        Ok(sub)
    }

    /// Updates an existing Subscription (`POST /v1/subscriptions/:id`).
    ///
    /// Supports updating items, pausing/resuming collection, plan migration with prorations,
    /// and automatic injection of an RFC 4122 UUID v4 `Idempotency-Key`.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if validation fails or subscription cannot be updated.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::{UpdateSubscriptionParams, ProrationBehavior};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = UpdateSubscriptionParams::new()
    ///     .proration_behavior(ProrationBehavior::CreateProrations)
    ///     .cancel_at_period_end(true);
    ///
    /// let sub = client.subscriptions().update("sub_12345", &params).await?;
    /// assert!(sub.cancel_at_period_end);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(
        &self,
        id: &str,
        params: &UpdateSubscriptionParams,
    ) -> Result<Subscription, StripeError> {
        let safe_id =
            validate_and_encode_id(id).map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        params
            .validate()
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!(
            "{}/v1/subscriptions/{}",
            self.client.config.api_base, safe_id
        );
        let form_data = to_form_params(params)?;

        let generated_key;
        let idempotency_key = match params.idempotency_key.as_deref() {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = generate_uuid_v4();
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
        let sub: Subscription = serde_json::from_value(val)?;
        Ok(sub)
    }

    /// Cancels a Subscription immediately (`DELETE /v1/subscriptions/:id`).
    ///
    /// Supports final invoicing (`invoice_now: true`) and proration crediting (`prorate: true`).
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if the ID is invalid or cannot be canceled.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::CancelSubscriptionParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = CancelSubscriptionParams::new().prorate(true);
    /// let sub = client.subscriptions().cancel("sub_12345", Some(&params)).await?;
    /// assert!(sub.is_canceled());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel(
        &self,
        id: &str,
        params: Option<&CancelSubscriptionParams>,
    ) -> Result<Subscription, StripeError> {
        let safe_id =
            validate_and_encode_id(id).map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!(
            "{}/v1/subscriptions/{}",
            self.client.config.api_base, safe_id
        );
        let form_data = if let Some(p) = params {
            to_form_params(p)?
        } else {
            Vec::new()
        };

        let generated_key;
        let idempotency_key = match params.and_then(|p| p.idempotency_key.as_deref()) {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = generate_uuid_v4();
                &generated_key
            }
        };

        let req = self
            .client
            .http
            .delete(&url)
            .form(&form_data)
            .header("Idempotency-Key", idempotency_key);

        let val = self.client.execute(req).await?;
        let sub: Subscription = serde_json::from_value(val)?;
        Ok(sub)
    }

    /// Resumes a paused Subscription (`POST /v1/subscriptions/:id/resume`).
    ///
    /// Configures billing cycle anchor (`now` or `unchanged`) and proration behavior.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if the subscription is not paused or cannot be resumed.
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::ResumeSubscriptionParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = ResumeSubscriptionParams::new().billing_cycle_anchor("now");
    /// let sub = client.subscriptions().resume("sub_12345", Some(&params)).await?;
    /// assert_eq!(sub.status().as_str(), "active");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn resume(
        &self,
        id: &str,
        params: Option<&ResumeSubscriptionParams>,
    ) -> Result<Subscription, StripeError> {
        let safe_id =
            validate_and_encode_id(id).map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        let url = format!(
            "{}/v1/subscriptions/{}/resume",
            self.client.config.api_base, safe_id
        );
        let form_data = if let Some(p) = params {
            to_form_params(p)?
        } else {
            Vec::new()
        };

        let generated_key;
        let idempotency_key = match params.and_then(|p| p.idempotency_key.as_deref()) {
            Some(k) if !k.trim().is_empty() => k,
            _ => {
                generated_key = generate_uuid_v4();
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
        let sub: Subscription = serde_json::from_value(val)?;
        Ok(sub)
    }

    /// Returns a zero-clone, auto-paginating stream of Subscriptions (`GET /v1/subscriptions`).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    /// use some_stripe_types::ListSubscriptionsParams;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let params = ListSubscriptionsParams::new()
    ///     .customer("cus_12345")
    ///     .status("active");
    ///
    /// let mut paginator = client.subscriptions().list(&params);
    /// while let Some(sub) = paginator.next_item().await? {
    ///     println!("Sub ID: {}", sub.id());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list(&self, params: &ListSubscriptionsParams) -> Paginator<'a, Subscription> {
        let mut query_pairs = Vec::new();
        if let Some(ref customer) = params.customer {
            query_pairs.push(("customer".to_string(), customer.clone()));
        }
        if let Some(ref price) = params.price {
            query_pairs.push(("price".to_string(), price.clone()));
        }
        if let Some(ref status) = params.status {
            query_pairs.push(("status".to_string(), status.clone()));
        }
        if let Some(start) = params.current_period_start {
            query_pairs.push(("current_period_start".to_string(), start.to_string()));
        }
        if let Some(end) = params.current_period_end {
            query_pairs.push(("current_period_end".to_string(), end.to_string()));
        }
        if let Some(created) = params.created {
            query_pairs.push(("created".to_string(), created.to_string()));
        }

        let base_path = if query_pairs.is_empty() {
            "v1/subscriptions".to_string()
        } else {
            format!("v1/subscriptions?{}", encode_form_pairs(&query_pairs))
        };

        let limit = params.limit.unwrap_or(10);
        let mut paginator = Paginator::new(self.client, base_path, limit, subscription_id);

        if let Some(ref sa) = params.starting_after {
            paginator = paginator.starting_after(sa.clone());
        }
        if let Some(ref eb) = params.ending_before {
            paginator = paginator.ending_before(eb.clone());
        }

        paginator
    }

    /// Helper for listing subscriptions with default parameters.
    pub fn list_default(&self) -> Paginator<'a, Subscription> {
        self.list(&ListSubscriptionsParams::default())
    }

    /// Searches subscriptions using Stripe's search query syntax (`GET /v1/subscriptions/search`).
    ///
    /// # Example
    /// ```no_run
    /// use some_stripe::StripeClient;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = StripeClient::new("sk_test_123");
    /// let results = client.subscriptions().search("status:'active'").await?;
    /// println!("Found {} active subscriptions", results.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search(&self, query: &str) -> Result<Vec<Subscription>, StripeError> {
        let encoded_query = percent_encode(query);
        let url = format!(
            "{}/v1/subscriptions/search?query={}",
            self.client.config.api_base, encoded_query
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let envelope: ListResponse<Subscription> = serde_json::from_value(val)?;
        Ok(envelope.data)
    }

    /// Searches subscriptions with custom pagination and limit parameters (`GET /v1/subscriptions/search`).
    pub async fn search_with_params(
        &self,
        params: &SearchSubscriptionsParams,
    ) -> Result<ListResponse<Subscription>, StripeError> {
        let mut query_pairs = vec![("query".to_string(), params.query.clone())];
        if let Some(limit) = params.limit {
            query_pairs.push(("limit".to_string(), limit.to_string()));
        }
        if let Some(ref page) = params.page {
            query_pairs.push(("page".to_string(), page.clone()));
        }

        let url = format!(
            "{}/v1/subscriptions/search?{}",
            self.client.config.api_base,
            encode_form_pairs(&query_pairs)
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let envelope: ListResponse<Subscription> = serde_json::from_value(val)?;
        Ok(envelope)
    }
}
