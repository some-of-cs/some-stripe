//! Stripe SubscriptionItems resource implementations.
//!
//! Provides bindings for managing Stripe Subscription Items (`stripe.subscriptions.items`),
//! enabling multi-item subscriptions, per-item quantity adjustments, proration configurations,
//! and metered usage record reporting.
//!
//! # Architecture & Quality Gates:
//! - **G0 (Rust 2024 Edition)**: Clean compilation with 0 clippy warnings and strict code formatting.
//! - **G-Borrow (Zero-Clone Discipline)**: Zero-copy string slice accessors (`item.id()`, `item.subscription()`)
//!   and streaming item pagination via [`Paginator`].
//! - **G2 (Security & Idempotency)**: Automatic injection of RFC 4122 UUID v4 `Idempotency-Key` headers
//!   on mutable operations (`create`, `update`, `del`, `create_usage_record`) when omitted by caller.
//! - **Security Invariant**: Strict path traversal prevention via [`validate_and_encode_id`].
//! - **Gate G-Law (ISO 4217 Minor Currency Units)**: Integer minor currency units precision.
//!
//! # Quickstart
//! ```no_run
//! use some_stripe::StripeClient;
//! use some_stripe_types::{CreateSubscriptionItemParams, CreateUsageRecordParams, UsageRecordAction};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = StripeClient::new("sk_test_123");
//!
//! // 1. Add an item to an existing subscription
//! let params = CreateSubscriptionItemParams::new("sub_123", "price_addon_456")
//!     .quantity(2);
//! let item = client.subscriptions().items.create(&params).await?;
//! println!("Created subscription item: {}", item.id());
//!
//! // 2. Record metered usage
//! let usage_params = CreateUsageRecordParams::new(50)
//!     .action(UsageRecordAction::Increment);
//! let usage = client.subscriptions().items.create_usage_record(item.id(), &usage_params).await?;
//! println!("Recorded usage: {}", usage.quantity());
//! # Ok(())
//! # }
//! ```

use some_stripe_core::patterns::pagination::Paginator;
use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CreateSubscriptionItemParams, CreateUsageRecordParams, DeleteSubscriptionItemParams,
    DeletedSubscriptionItem, ListSubscriptionItemsParams, SubscriptionItem, SubscriptionItemList,
    UpdateSubscriptionItemParams, UsageRecord, subscription_item_id,
};

/// Resource for managing Stripe Subscription Items (`stripe.subscriptions.items`).
///
/// Accessed via `client.subscriptions().items`.
#[derive(Clone, Copy)]
pub struct SubscriptionItemsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> SubscriptionItemsResource<'a> {
    /// Creates a new `SubscriptionItemsResource` bound to the provided client.
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Creates a Subscription Item (`POST /v1/subscription_items`).
    ///
    /// Automatically injects an RFC 4122 UUID v4 `Idempotency-Key` header if omitted.
    pub async fn create(
        &self,
        params: &CreateSubscriptionItemParams,
    ) -> Result<SubscriptionItem, StripeError> {
        let url = format!("{}/v1/subscription_items", self.client.config.api_base);
        let form_data = to_form_params(params)?;

        let idempotency_key = super::resolve_idempotency_key(params.idempotency_key.as_deref());

        let req = self
            .client
            .http
            .post(&url)
            .form(&form_data)
            .header("Idempotency-Key", &*idempotency_key);

        let val = self.client.execute(req).await?;
        let item: SubscriptionItem = serde_json::from_value(val)?;
        Ok(item)
    }

    /// Retrieves an existing Subscription Item by ID (`GET /v1/subscription_items/{id}`).
    ///
    /// Accepts the identifier as a string reference (`&str`), ensuring zero allocations.
    /// Strictly protects against path traversal via [`validate_and_encode_id`].
    pub async fn retrieve(&self, item_id: &str) -> Result<SubscriptionItem, StripeError> {
        let encoded_id = validate_and_encode_id(item_id)?;
        let url = format!(
            "{}/v1/subscription_items/{}",
            self.client.config.api_base, encoded_id
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let item: SubscriptionItem = serde_json::from_value(val)?;
        Ok(item)
    }

    /// Updates an existing Subscription Item (`POST /v1/subscription_items/{id}`).
    ///
    /// Automatically injects an RFC 4122 UUID v4 `Idempotency-Key` header if omitted.
    pub async fn update(
        &self,
        item_id: &str,
        params: &UpdateSubscriptionItemParams,
    ) -> Result<SubscriptionItem, StripeError> {
        let encoded_id = validate_and_encode_id(item_id)?;
        let url = format!(
            "{}/v1/subscription_items/{}",
            self.client.config.api_base, encoded_id
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
        let item: SubscriptionItem = serde_json::from_value(val)?;
        Ok(item)
    }

    /// Deletes a Subscription Item (`DELETE /v1/subscription_items/{id}`).
    ///
    /// Supports optional deletion parameters (`clear_usage`, `proration_behavior`).
    pub async fn del(
        &self,
        item_id: &str,
        params: Option<&DeleteSubscriptionItemParams>,
    ) -> Result<DeletedSubscriptionItem, StripeError> {
        let encoded_id = validate_and_encode_id(item_id)?;
        let url = format!(
            "{}/v1/subscription_items/{}",
            self.client.config.api_base, encoded_id
        );

        let mut req = self.client.http.delete(&url);

        let idempotency_key =
            super::resolve_idempotency_key(params.and_then(|p| p.idempotency_key.as_deref()));
        req = req.header("Idempotency-Key", &*idempotency_key);

        if let Some(p) = params {
            let form_data = to_form_params(p)?;
            if !form_data.is_empty() {
                req = req.form(&form_data);
            }
        }

        let val = self.client.execute(req).await?;
        let deleted: DeletedSubscriptionItem = serde_json::from_value(val)?;
        Ok(deleted)
    }

    /// Lists subscription items for a given subscription (`GET /v1/subscription_items`).
    pub async fn list(
        &self,
        params: &ListSubscriptionItemsParams,
    ) -> Result<SubscriptionItemList, StripeError> {
        let encoded_sub = validate_and_encode_id(&params.subscription)?;
        let mut url = format!(
            "{}/v1/subscription_items?subscription={encoded_sub}",
            self.client.config.api_base
        );

        if let Some(limit) = params.limit {
            url.push_str(&format!("&limit={limit}"));
        }
        if let Some(ref sa) = params.starting_after {
            let enc = some_stripe_core::patterns::serializer::percent_encode(sa);
            url.push_str(&format!("&starting_after={enc}"));
        }
        if let Some(ref eb) = params.ending_before {
            let enc = some_stripe_core::patterns::serializer::percent_encode(eb);
            url.push_str(&format!("&ending_before={enc}"));
        }

        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let list: SubscriptionItemList = serde_json::from_value(val)?;
        Ok(list)
    }

    /// Returns an asynchronous streaming [`Paginator`] for auto-paginating through subscription items.
    pub fn list_stream(
        &self,
        params: &ListSubscriptionItemsParams,
    ) -> Paginator<'a, SubscriptionItem> {
        let (path, initial_err) = match validate_and_encode_id(&params.subscription) {
            Ok(enc) => (format!("v1/subscription_items?subscription={enc}"), None),
            Err(e) => (String::new(), Some(e)),
        };

        let mut paginator = Paginator::new(
            self.client,
            path,
            params.limit.unwrap_or(10),
            subscription_item_id,
        );

        if let Some(err) = initial_err {
            paginator = paginator.with_initial_error(err);
        }
        if let Some(ref sa) = params.starting_after {
            paginator = paginator.starting_after(sa);
        }
        if let Some(ref eb) = params.ending_before {
            paginator = paginator.ending_before(eb);
        }

        paginator
    }

    /// Creates a Usage Record for a metered subscription item (`POST /v1/subscription_items/{subscription_item}/usage_records`).
    ///
    /// Automatically injects an RFC 4122 UUID v4 `Idempotency-Key` header if omitted.
    pub async fn create_usage_record(
        &self,
        subscription_item_id: &str,
        params: &CreateUsageRecordParams,
    ) -> Result<UsageRecord, StripeError> {
        let encoded_id = validate_and_encode_id(subscription_item_id)?;
        let url = format!(
            "{}/v1/subscription_items/{}/usage_records",
            self.client.config.api_base, encoded_id
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
        let record: UsageRecord = serde_json::from_value(val)?;
        Ok(record)
    }
}
