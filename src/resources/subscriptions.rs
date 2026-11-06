//! Stripe Subscriptions resource implementations.
//!
//! Provides bindings for managing Stripe Subscriptions (`stripe.subscriptions`),
//! including multi-item subscriptions and subscription item sub-resource access
//! (`client.subscriptions().items`).

use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_core::{StripeClient, StripeError, validate_and_encode_id};
use some_stripe_types::{
    CreateSubscriptionParams, Subscription, SubscriptionList, UpdateSubscriptionParams,
};

use crate::resources::subscription_items::SubscriptionItemsResource;

/// Resource for managing Stripe Subscriptions (`stripe.subscriptions`).
///
/// Accessed via `client.subscriptions()`. Provides direct access to subscription items
/// via `client.subscriptions().items`.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
/// use some_stripe_types::{CreateSubscriptionParams, SubscriptionItemParam};
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let client = StripeClient::new("sk_test_123");
///
/// // Create a multi-item subscription
/// let params = CreateSubscriptionParams::new("cus_123")
///     .add_item(SubscriptionItemParam::with_price_and_quantity("price_seat_123", 5))
///     .add_item(SubscriptionItemParam::with_price_and_quantity("price_addon_456", 1));
/// let sub = client.subscriptions().create(&params).await?;
/// println!("Created subscription: {}", sub.id());
///
/// // Access subscription items directly
/// let items = client.subscriptions().items;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy)]
pub struct SubscriptionsResource<'a> {
    client: &'a StripeClient,
    /// Subscription items sub-resource (`stripe.subscriptions.items`).
    pub items: SubscriptionItemsResource<'a>,
}

impl<'a> SubscriptionsResource<'a> {
    /// Creates a new `SubscriptionsResource` bound to the client.
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

    /// Creates a Subscription (`POST /v1/subscriptions`).
    ///
    /// Supports multi-item zero-indexed bracket form serialization (`items[0][price]=...`).
    /// Automatically injects an RFC 4122 UUID v4 `Idempotency-Key` header if omitted.
    pub async fn create(
        &self,
        params: &CreateSubscriptionParams,
    ) -> Result<Subscription, StripeError> {
        let url = format!("{}/v1/subscriptions", self.client.config.api_base);
        let form_data = to_form_params(params)?;

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
        let sub: Subscription = serde_json::from_value(val)?;
        Ok(sub)
    }

    /// Retrieves an existing Subscription by ID (`GET /v1/subscriptions/{id}`).
    pub async fn retrieve(&self, subscription_id: &str) -> Result<Subscription, StripeError> {
        let encoded_id = validate_and_encode_id(subscription_id)?;
        let url = format!(
            "{}/v1/subscriptions/{}",
            self.client.config.api_base, encoded_id
        );
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let sub: Subscription = serde_json::from_value(val)?;
        Ok(sub)
    }

    /// Updates an existing Subscription (`POST /v1/subscriptions/{id}`).
    ///
    /// Supports multi-item additions, modifications, and deletions.
    /// Automatically injects an RFC 4122 UUID v4 `Idempotency-Key` header if omitted.
    pub async fn update(
        &self,
        subscription_id: &str,
        params: &UpdateSubscriptionParams,
    ) -> Result<Subscription, StripeError> {
        let encoded_id = validate_and_encode_id(subscription_id)?;
        let url = format!(
            "{}/v1/subscriptions/{}",
            self.client.config.api_base, encoded_id
        );
        let form_data = to_form_params(params)?;

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
        let sub: Subscription = serde_json::from_value(val)?;
        Ok(sub)
    }

    /// Cancels an existing Subscription (`DELETE /v1/subscriptions/{id}`).
    pub async fn cancel(&self, subscription_id: &str) -> Result<Subscription, StripeError> {
        let encoded_id = validate_and_encode_id(subscription_id)?;
        let url = format!(
            "{}/v1/subscriptions/{}",
            self.client.config.api_base, encoded_id
        );
        let req = self.client.http.delete(&url);
        let val = self.client.execute(req).await?;
        let sub: Subscription = serde_json::from_value(val)?;
        Ok(sub)
    }

    /// Lists Subscriptions (`GET /v1/subscriptions`).
    pub async fn list(&self, limit: Option<u32>) -> Result<SubscriptionList, StripeError> {
        let mut url = format!("{}/v1/subscriptions", self.client.config.api_base);
        if let Some(l) = limit {
            url.push_str(&format!("?limit={l}"));
        }
        let req = self.client.http.get(&url);
        let val = self.client.execute(req).await?;
        let list: SubscriptionList = serde_json::from_value(val)?;
        Ok(list)
    }
}
