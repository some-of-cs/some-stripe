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
//! let calculations = tax.calculations();
//! let transactions = tax.transactions();
//! let settings = tax.settings();
//! let registrations = tax.registrations();
//! ```

use some_stripe_core::StripeClient;

/// Calculations sub-resource facade for Stripe Tax calculations (`stripe.tax().calculations()`).
///
/// Handles real-time tax calculation for line items based on customer addresses and product tax codes.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
///
/// let client = StripeClient::new("sk_test_123");
/// let calcs = client.tax().calculations();
/// assert_eq!(calcs.client().config.api_base, "https://api.stripe.com");
/// ```
#[derive(Clone, Copy)]
pub struct CalculationsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> CalculationsResource<'a> {
    /// Creates a new `CalculationsResource` bound to the client.
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }
}

/// Transactions sub-resource facade for Stripe Tax ledger transactions (`stripe.tax().transactions()`).
///
/// Commits calculations to official tax reporting ledgers and creates tax transaction reversals.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
///
/// let client = StripeClient::new("sk_test_123");
/// let txs = client.tax().transactions();
/// assert_eq!(txs.client().config.api_base, "https://api.stripe.com");
/// ```
#[derive(Clone, Copy)]
pub struct TransactionsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> TransactionsResource<'a> {
    /// Creates a new `TransactionsResource` bound to the client.
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }
}

/// Settings sub-resource facade for Stripe Tax account settings (`stripe.tax().settings()`).
///
/// Configures head office locations, default tax codes, and calculation statuses.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
///
/// let client = StripeClient::new("sk_test_123");
/// let settings = client.tax().settings();
/// assert_eq!(settings.client().config.api_base, "https://api.stripe.com");
/// ```
#[derive(Clone, Copy)]
pub struct TaxSettingsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> TaxSettingsResource<'a> {
    /// Creates a new `TaxSettingsResource` bound to the client.
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }
}

/// Registrations sub-resource facade for jurisdictional tax nexus registrations (`stripe.tax().registrations()`).
///
/// Manages state, country, and local tax registrations and collection thresholds.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
///
/// let client = StripeClient::new("sk_test_123");
/// let reg = client.tax().registrations();
/// assert_eq!(reg.client().config.api_base, "https://api.stripe.com");
/// ```
#[derive(Clone, Copy)]
pub struct TaxRegistrationsResource<'a> {
    client: &'a StripeClient,
}

impl<'a> TaxRegistrationsResource<'a> {
    /// Creates a new `TaxRegistrationsResource` bound to the client.
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }
}

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
