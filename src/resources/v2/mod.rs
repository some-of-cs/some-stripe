//! Stripe API V2 resource implementations and namespace hierarchies.

pub mod billing_meters;
pub mod meter_adjustments;
pub mod meter_events;
pub mod meter_session;
pub mod meter_stream;

pub use billing_meters::BillingMetersResource;
pub use meter_adjustments::MeterEventAdjustmentsResource;
pub use meter_events::MeterEventsResource;
pub use meter_session::MeterEventSessionResource;
pub use meter_stream::MeterEventStreamResource;

use some_stripe_core::StripeClient;

/// Stripe API V2 Billing namespace resource (`stripe.v2.billing`).
#[derive(Clone, Copy)]
pub struct V2BillingResource<'a> {
    client: &'a StripeClient,
}

impl<'a> V2BillingResource<'a> {
    /// Creates a new `V2BillingResource` bound to the provided [`StripeClient`].
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Access the Meter Event Adjustments resource (`stripe.v2.billing.meter_event_adjustments`).
    #[must_use]
    pub fn meter_event_adjustments(&self) -> MeterEventAdjustmentsResource<'a> {
        MeterEventAdjustmentsResource::new(self.client)
    }

    /// Access the Meter Event Session resource (`stripe.v2.billing.meter_event_session`).
    #[must_use]
    pub fn meter_event_session(&self) -> MeterEventSessionResource<'a> {
        MeterEventSessionResource::new(self.client)
    }

    /// Access the Meter Event Stream resource (`stripe.v2.billing.meter_event_stream`).
    #[must_use]
    pub fn meter_event_stream(&self) -> MeterEventStreamResource<'a> {
        MeterEventStreamResource::new(self.client)
    }

    /// Access the Meter Events resource (`stripe.v2.billing.meter_events`).
    #[must_use]
    pub fn meter_events(&self) -> MeterEventsResource<'a> {
        MeterEventsResource::new(self.client)
    }

    /// Access the Billing Meters resource (`stripe.v2.billing.meters`).
    #[must_use]
    pub fn meters(&self) -> BillingMetersResource<'a> {
        BillingMetersResource::new(self.client)
    }
}

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
    pub fn billing(&self) -> V2BillingResource<'a> {
        V2BillingResource::new(self.client)
    }
}
