//! Stripe API V2 services and resource namespaces.

pub mod billing;
pub mod ingestion_engine;
pub mod meter_adjustments;
pub mod meter_session;
pub mod meter_stream;
pub mod meters;

pub use billing::V2Billing;
pub use ingestion_engine::{
    CollectorStats, IngestionEngineOptions, MeterIngestionEngine, MeterStreamCollector,
};
pub use meter_adjustments::MeterEventAdjustmentsResource;
pub use meter_session::MeterEventSessionResource;
pub use meter_stream::MeterEventStreamResource;
pub use meters::BillingMetersResource;

use some_stripe_core::StripeClient;

/// Top-level namespace for Stripe API V2 services (`client.v2()`).
#[derive(Clone, Copy)]
pub struct V2Namespace<'a> {
    client: &'a StripeClient,
}

impl<'a> V2Namespace<'a> {
    /// Creates a new `V2Namespace` bound to the provided client.
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Access the V2 Billing subsystem (`client.v2().billing()`).
    #[must_use]
    pub fn billing(&self) -> V2Billing<'a> {
        V2Billing::new(self.client)
    }
}
