//! Stripe API v2 Core Resource Facade (`stripe.v2().core()` and `stripe.v2_core()`).
//!
//! Exposes access to v2 Core Events, Event Destinations, Core Accounts, and the Thin Event Hydrator.
//!
//! # Architecture & Quality Invariants
//! - **Gate G-Borrow**: Zero-copy borrowing of inner client and references.
//! - **Gate G-Complexity**: Cyclomatic complexity $CC \le 3$ across all facade accessors.
//! - **Gate G-Docs**: Comprehensive rustdoc documentation and runnable doctests.

use some_stripe_core::StripeClient;
use some_stripe_types::v2_events::V2Event;
use some_stripe_webhooks::v2_events::V2ThinEvent;

use super::v2_accounts::V2AccountsResource;
use super::v2_destinations::EventDestinationsResource;
use super::v2_events::V2EventsResource;
use super::v2_hydration::{HydrationError, ThinEventHydrator};

pub use super::v2::V2Resource;

/// Facade resource for Stripe API v2 Core operations (`stripe.v2().core()` or `stripe.v2_core()`).
///
/// Mounts resources for:
/// - [`V2EventsResource`] (`events()`): authenticated event retrieval and listing.
/// - [`EventDestinationsResource`] (`event_destinations()`): AWS EventBridge, Azure Event Grid, and Webhook management.
/// - [`V2AccountsResource`] (`accounts()`): next-generation merchant account provisioning.
/// - [`ThinEventHydrator`] (`hydrator()`): cryptographic freshness and thin event live state hydration.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let client = StripeClient::new("sk_test_123");
/// let core = client.v2().core();
///
/// let _events = core.events();
/// let _destinations = core.event_destinations();
/// let _accounts = core.accounts();
/// let _hydrator = core.hydrator();
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy)]
pub struct V2Core<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> V2Core<'a> {
    /// Creates a new `V2Core` facade bound to the underlying [`StripeClient`].
    #[must_use]
    pub const fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub const fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this facade and all child resources to a specific connected account.
    #[must_use]
    pub const fn with_account(&self, account: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account),
        }
    }

    /// Access the V2 Events resource (`stripe.v2().core().events()` or `stripe.v2_core().events()`).
    #[must_use]
    pub fn events(&self) -> V2EventsResource<'a> {
        let res = V2EventsResource::new(self.client);
        if let Some(acct) = self.account {
            res.with_account(acct)
        } else {
            res
        }
    }

    /// Access the V2 Event Destinations resource (`stripe.v2().core().event_destinations()` or `stripe.v2_core().event_destinations()`).
    #[must_use]
    pub fn event_destinations(&self) -> EventDestinationsResource<'a> {
        let res = EventDestinationsResource::new(self.client);
        if let Some(acct) = self.account {
            res.with_account(acct)
        } else {
            res
        }
    }

    /// Access the V2 Core Accounts resource (`stripe.v2().core().accounts()` or `stripe.v2_core().accounts()`).
    #[must_use]
    pub fn accounts(&self) -> V2AccountsResource<'a> {
        let res = V2AccountsResource::new(self.client);
        if let Some(acct) = self.account {
            res.with_account(acct)
        } else {
            res
        }
    }

    /// Access the Thin Event Hydrator (`stripe.v2().core().hydrator()` or `stripe.v2_core().hydrator()`).
    #[must_use]
    pub const fn hydrator(&self) -> ThinEventHydrator<'a> {
        ThinEventHydrator::new(self.client)
    }

    /// Hydrates a thin event notification into a fully populated live event.
    ///
    /// # Errors
    /// Returns [`HydrationError`] if the event timestamp fails freshness checks or API retrieval fails.
    pub async fn hydrate_thin_event(
        &self,
        notification: &V2ThinEvent,
    ) -> Result<V2Event, HydrationError> {
        self.hydrator().hydrate_thin_event(notification).await
    }
}
