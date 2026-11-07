//! Stripe Treasury resource facade (`stripe.treasury`).
//!
//! Provides access to commercial financial accounts, FDIC-insured cash balances,
//! outbound wire transfers, outbound ACH payments, and received credits modeled after `stripe-node`.
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
//! let treasury = client.treasury();
//! let _accounts = treasury.financial_accounts();
//! let _transfers = treasury.outbound_transfers();
//! let _payments = treasury.outbound_payments();
//! # Ok(())
//! # }
//! ```

use some_stripe_core::StripeClient;

use super::treasury_financial_accounts::TreasuryFinancialAccountsResource;
use super::treasury_transfers::{
    OutboundPaymentsResource, OutboundTransfersResource, ReceivedCreditsResource,
    TreasuryTransfersResource,
};

/// Facade resource for Stripe Treasury operations (`stripe.treasury`).
///
/// Provides access to financial accounts, outbound transfers, and outbound payments.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
///
/// let client = StripeClient::new("sk_test_123");
/// let treasury = client.treasury();
/// let _accounts = treasury.financial_accounts();
/// ```
#[derive(Clone, Copy)]
pub struct TreasuryResource<'a> {
    client: &'a StripeClient,
}

impl<'a> TreasuryResource<'a> {
    /// Creates a new `TreasuryResource` bound to the provided [`StripeClient`].
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Access the Treasury Financial Accounts sub-resource (`stripe.treasury.financial_accounts`).
    #[must_use]
    pub fn financial_accounts(&self) -> TreasuryFinancialAccountsResource<'a> {
        TreasuryFinancialAccountsResource::new(self.client)
    }

    /// Access the Treasury Transfers sub-resource (`stripe.treasury.transfers`).
    #[must_use]
    pub fn transfers(&self) -> TreasuryTransfersResource<'a> {
        TreasuryTransfersResource::new(self.client)
    }

    /// Access the Treasury Outbound Transfers sub-resource (`stripe.treasury.outbound_transfers`).
    #[must_use]
    pub fn outbound_transfers(&self) -> OutboundTransfersResource<'a> {
        TreasuryTransfersResource::new(self.client).outbound_transfers()
    }

    /// Access the Treasury Outbound Payments sub-resource (`stripe.treasury.outbound_payments`).
    #[must_use]
    pub fn outbound_payments(&self) -> OutboundPaymentsResource<'a> {
        TreasuryTransfersResource::new(self.client).outbound_payments()
    }

    /// Access the Treasury Received Credits sub-resource (`stripe.treasury.received_credits`).
    #[must_use]
    pub fn received_credits(&self) -> ReceivedCreditsResource<'a> {
        TreasuryTransfersResource::new(self.client).received_credits()
    }
}
