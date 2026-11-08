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
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Access the V2 Billing resource namespace (`stripe.v2.billing`).
    #[must_use]
    pub fn billing(&self) -> V2Billing<'a> {
        V2Billing::new(self.client)
    }
}

/// Alias for [`V2Resource`] (`stripe.v2`).
pub type V2Namespace<'a> = V2Resource<'a>;
