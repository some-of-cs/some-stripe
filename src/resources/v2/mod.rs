//! Stripe API V2 resource implementations and namespace hierarchies.

pub mod meter_adjustments;

pub use meter_adjustments::MeterEventAdjustmentsResource;

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
