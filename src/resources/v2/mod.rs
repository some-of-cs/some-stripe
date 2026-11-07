//! Stripe API v2 Next-Generation Services and Resource Facades (`stripe.v2()`).
//!
//! Provides access to API v2 core namespaces and services modeled after `stripe-node`.
//!
//! # Architecture & Quality Invariants
//! - **Gate G-Borrow**: Zero-copy borrowing across string slices and parameter schemas.
//! - **Gate G-Complexity**: Cyclomatic complexity $CC \le 3$ for all facade accessor methods.
//! - **Gate G-Docs**: Comprehensive rustdoc documentation and runnable doctests.
//!
//! # Quickstart
//! ```no_run
//! use some_stripe::StripeClient;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = StripeClient::new("sk_test_123");
//! let core = client.v2().core();
//! let _events = core.events();
//! let _destinations = core.event_destinations();
//! let _accounts = core.accounts();
//! let _hydrator = core.hydrator();
//! # Ok(())
//! # }
//! ```

pub mod accounts;
pub mod core;
pub mod event_destinations;
pub mod events;
pub mod hydration;

pub use accounts::V2AccountsResource;
pub use core::V2Core;
pub use event_destinations::EventDestinationsResource;
pub use events::V2EventsResource;
pub use hydration::{HydrationError, ThinEventHydrator};
pub use some_stripe_types::EventNotification;

use some_stripe_core::StripeClient;

/// Facade resource for Stripe API v2 operations (`stripe.v2`).
///
/// Top-level accessor for next-generation v2 resource namespaces:
/// - [`V2Core`] (`core()`): Core events, destinations, accounts, and thin event hydration.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
///
/// let client = StripeClient::new("sk_test_123");
/// let v2 = client.v2();
/// let _core = v2.core();
/// ```
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

    /// Access the API v2 Core namespace (`stripe.v2.core`).
    #[must_use]
    pub fn core(&self) -> V2Core<'a> {
        V2Core::new(self.client)
    }
}
