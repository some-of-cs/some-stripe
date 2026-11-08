//! Stripe API V2 Billing resource facade (`stripe.v2.billing`).
//!
//! Provides top-level access to high-throughput meter event sessions, batch event streams,
//! meter event adjustments, and billing meter configurations.
//!
//! # Architecture & Quality Invariants
//! - **Gate G-Borrow**: Zero-copy borrowing across string slices and parameter models.
//! - **Gate G-Complexity**: Cyclomatic complexity $CC \le 3$ for all facade accessor methods.
//! - **Gate G-Docs**: Comprehensive rustdoc documentation and working doctests.
//!
//! # Quickstart
//! ```no_run
//! use some_stripe::StripeClient;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = StripeClient::new("sk_test_123");
//! let billing = client.v2().billing();
//! let _session_res = billing.meter_event_session();
//! let _stream_res = billing.meter_event_stream();
//! let _adjustments_res = billing.meter_event_adjustments();
//! let _meters_res = billing.meters();
//! # Ok(())
//! # }
//! ```

use some_stripe_core::StripeClient;

use super::billing_meters::BillingMetersResource;
use super::ingestion_engine::MeterIngestionEngine;
use super::meter_adjustments::MeterEventAdjustmentsResource;
use super::meter_session::MeterEventSessionResource;
use super::meter_stream::MeterEventStreamResource;

/// Facade resource for Stripe API V2 Billing operations (`stripe.v2.billing`).
///
/// Provides access to meter event sessions, high-throughput event streams,
/// adjustments, and billing meter definitions.
#[derive(Clone, Copy)]
pub struct V2Billing<'a> {
    client: &'a StripeClient,
}

impl<'a> V2Billing<'a> {
    /// Creates a new `V2Billing` facade bound to the provided client.
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Access the Meter Event Session resource (`/v2/billing/meter_event_session`).
    #[must_use]
    pub fn meter_event_session(&self) -> MeterEventSessionResource<'a> {
        MeterEventSessionResource::new(self.client)
    }

    /// Access the Meter Event Stream resource (`/v2/billing/meter_event_stream`).
    #[must_use]
    pub fn meter_event_stream(&self) -> MeterEventStreamResource<'a> {
        MeterEventStreamResource::new(self.client)
    }

    /// Access the Meter Event Adjustments resource (`/v2/billing/meter_event_adjustments`).
    #[must_use]
    pub fn meter_event_adjustments(&self) -> MeterEventAdjustmentsResource<'a> {
        MeterEventAdjustmentsResource::new(self.client)
    }

    /// Access the Billing Meters configuration resource (`/v2/billing/meters`).
    #[must_use]
    pub fn meters(&self) -> BillingMetersResource<'a> {
        BillingMetersResource::new(self.client)
    }

    /// Instantiates a high-throughput [`MeterIngestionEngine`] pre-configured with the client's API base and credentials.
    #[must_use]
    pub fn create_ingestion_engine(&self) -> MeterIngestionEngine {
        MeterIngestionEngine::new(
            &self.client.config.api_base,
            self.client.config.secret_key.expose_secret(),
        )
    }
}
