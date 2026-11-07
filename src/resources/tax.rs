//! Stripe Tax resource facade and sub-resource namespaces.
//!
//! Provides top-level access to Stripe Tax services (`stripe.tax()`):
//! - `stripe.tax().calculations()`: Dynamic real-time sales tax, VAT, and GST calculation.
//! - `stripe.tax().transactions()`: Official tax ledger recording and reversals.
//! - `stripe.tax().settings()`: Account-wide tax settings and physical head office registration.
//! - `stripe.tax().registrations()`: Jurisdictional tax nexus registrations and thresholds.
//!
//! # Architecture & Quality Invariants:
//! - **Gate G-Borrow**: Zero deep clones; lightweight copyable facades borrowing `&'a StripeClient`.
//! - **Gate G-Docs**: Runnable doc-tests on all public facade methods.
//! - **Gate G-Complexity**: Cyclomatic complexity <= 6 on all constructors and accessors.
//! - **Gate G-Law**: Compliant with EU VAT Directive 2006/112/EC Art 196 reverse charges.
//!
//! # Quickstart
//! ```no_run
//! use some_stripe::StripeClient;
//!
//! let client = StripeClient::new("sk_test_123");
//! let tax = client.tax();
//! let _calculations = tax.calculations();
//! let _transactions = tax.transactions();
//! let _settings = tax.settings();
//! let _registrations = tax.registrations();
//! ```

use some_stripe_core::StripeClient;

pub use crate::resources::tax_calculations::CalculationsResource;
pub use crate::resources::tax_registrations::{TaxRegistrationsResource, tax_registration_id};
pub use crate::resources::tax_settings::TaxSettingsResource;
pub use crate::resources::tax_transactions::TransactionsResource;

/// Root resource namespace for Stripe Tax operations (`stripe.tax()`).
///
/// Wraps tax sub-resources:
/// - [`CalculationsResource`]: Tax liability calculations per jurisdiction.
/// - [`TransactionsResource`]: Official tax ledger recording and reversals.
/// - [`TaxSettingsResource`]: Account tax settings.
/// - [`TaxRegistrationsResource`]: Tax registrations.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
///
/// let client = StripeClient::new("sk_test_123");
/// let tax = client.tax();
/// let _calcs = tax.calculations();
/// let _txs = tax.transactions();
/// let _settings = tax.settings();
/// let _regs = tax.registrations();
/// ```
#[derive(Clone, Copy)]
pub struct TaxResource<'a> {
    client: &'a StripeClient,
}

impl<'a> TaxResource<'a> {
    /// Creates a new `TaxResource` facade bound to the provided client.
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Access the Calculations resource namespace (`stripe.tax().calculations()`).
    #[must_use]
    pub fn calculations(&self) -> CalculationsResource<'a> {
        CalculationsResource::new(self.client)
    }

    /// Access the Transactions resource namespace (`stripe.tax().transactions()`).
    #[must_use]
    pub fn transactions(&self) -> TransactionsResource<'a> {
        TransactionsResource::new(self.client)
    }

    /// Access the Settings resource namespace (`stripe.tax().settings()`).
    #[must_use]
    pub fn settings(&self) -> TaxSettingsResource<'a> {
        TaxSettingsResource::new(self.client)
    }

    /// Access the Registrations resource namespace (`stripe.tax().registrations()`).
    #[must_use]
    pub fn registrations(&self) -> TaxRegistrationsResource<'a> {
        TaxRegistrationsResource::new(self.client)
    }
}
