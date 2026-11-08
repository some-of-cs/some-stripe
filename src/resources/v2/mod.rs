//! Stripe API V2 resource implementations and namespace hierarchies.

pub mod billing;
pub mod billing_meters;
pub mod ingestion_engine;
pub mod meter_adjustments;
pub mod meter_events;
pub mod meter_session;
pub mod meter_stream;

pub use billing::V2Billing;
pub use billing_meters::BillingMetersResource;
pub use ingestion_engine::{
    CollectorStats, IngestionEngineOptions, MeterIngestionEngine, MeterStreamCollector,
};
pub use meter_adjustments::MeterEventAdjustmentsResource;
pub use meter_events::MeterEventsResource;
pub use meter_session::MeterEventSessionResource;
pub use meter_stream::MeterEventStreamResource;

use some_stripe_core::StripeClient;

use super::v2_accounts::V2AccountsResource;
use super::v2_core::V2Core;
use super::v2_destinations::EventDestinationsResource;
use super::v2_events::V2EventsResource;
use super::v2_hydration::ThinEventHydrator;

/// Stripe API V2 Billing namespace resource (`stripe.v2.billing`).
pub type V2BillingResource<'a> = V2Billing<'a>;

/// Root Stripe API V2 namespace resource (`stripe.v2`).
#[derive(Clone, Copy)]
pub struct V2Resource<'a> {
    client: &'a StripeClient,
}

impl<'a> V2Resource<'a> {
    /// Creates a new `V2Resource` bound to the provided [`StripeClient`].
    #[must_use]
    pub const fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub const fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Access the API v2 Core namespace (`stripe.v2().core()`).
    #[must_use]
    pub const fn core(&self) -> V2Core<'a> {
        V2Core::new(self.client)
    }

    /// Access the V2 Billing resource namespace (`stripe.v2.billing`).
    #[must_use]
    pub fn billing(&self) -> V2Billing<'a> {
        V2Billing::new(self.client)
    }

    /// Access the V2 Events resource (`stripe.v2().events()`).
    #[must_use]
    pub fn events(&self) -> V2EventsResource<'a> {
        self.core().events()
    }

    /// Access the V2 Event Destinations resource (`stripe.v2().event_destinations()`).
    #[must_use]
    pub fn event_destinations(&self) -> EventDestinationsResource<'a> {
        self.core().event_destinations()
    }

    /// Access the V2 Core Accounts resource (`stripe.v2().accounts()`).
    #[must_use]
    pub fn accounts(&self) -> V2AccountsResource<'a> {
        self.core().accounts()
    }

    /// Access the Thin Event Hydrator (`stripe.v2().hydrator()`).
    #[must_use]
    pub const fn hydrator(&self) -> ThinEventHydrator<'a> {
        self.core().hydrator()
    }
}

/// Alias for [`V2Resource`] (`stripe.v2`).
pub type V2Namespace<'a> = V2Resource<'a>;
