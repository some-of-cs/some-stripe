//! Stripe Treasury resource facade (`stripe.treasury`).
//!
//! Provides access to commercial financial accounts, FDIC-insured cash balances,
//! outbound wire transfers, and outbound ACH payments modeled after `stripe-node`.
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
    /// Treasury Financial Accounts resource (`stripe.treasury.financial_accounts`).
    pub financial_accounts: TreasuryFinancialAccountsResource<'a>,
    /// Treasury Outbound Transfers resource (`stripe.treasury.outbound_transfers`).
    pub outbound_transfers: TreasuryOutboundTransfersResource<'a>,
    /// Treasury Outbound Payments resource (`stripe.treasury.outbound_payments`).
    pub outbound_payments: TreasuryOutboundPaymentsResource<'a>,
    client: &'a StripeClient,
}

impl<'a> TreasuryResource<'a> {
    /// Creates a new `TreasuryResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            financial_accounts: TreasuryFinancialAccountsResource::new(client),
            outbound_transfers: TreasuryOutboundTransfersResource::new(client),
            outbound_payments: TreasuryOutboundPaymentsResource::new(client),
            client,
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Access the Treasury Financial Accounts sub-resource (`stripe.treasury.financial_accounts`).
    pub fn financial_accounts(&self) -> &TreasuryFinancialAccountsResource<'a> {
        &self.financial_accounts
    }

    /// Access the Treasury Outbound Transfers sub-resource (`stripe.treasury.outbound_transfers`).
    pub fn outbound_transfers(&self) -> &TreasuryOutboundTransfersResource<'a> {
        &self.outbound_transfers
    }

    /// Access the Treasury Outbound Payments sub-resource (`stripe.treasury.outbound_payments`).
    pub fn outbound_payments(&self) -> &TreasuryOutboundPaymentsResource<'a> {
        &self.outbound_payments
    }
}

/// Resource for managing Stripe Treasury Financial Accounts (`stripe.treasury.financial_accounts`).
#[derive(Clone, Copy)]
pub struct TreasuryFinancialAccountsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> TreasuryFinancialAccountsResource<'a> {
    /// Creates a new `TreasuryFinancialAccountsResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this resource to operate on behalf of a specific connected account.
    pub fn with_account(&self, account_id: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account_id),
        }
    }

    /// Return the connected account ID if scoped.
    pub fn account(&self) -> Option<&'a str> {
        self.account
    }
}

/// Resource for managing Stripe Treasury Outbound Transfers (`stripe.treasury.outbound_transfers`).
#[derive(Clone, Copy)]
pub struct TreasuryOutboundTransfersResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> TreasuryOutboundTransfersResource<'a> {
    /// Creates a new `TreasuryOutboundTransfersResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this resource to operate on behalf of a specific connected account.
    pub fn with_account(&self, account_id: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account_id),
        }
    }

    /// Return the connected account ID if scoped.
    pub fn account(&self) -> Option<&'a str> {
        self.account
    }
}

/// Resource for managing Stripe Treasury Outbound Payments (`stripe.treasury.outbound_payments`).
#[derive(Clone, Copy)]
pub struct TreasuryOutboundPaymentsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> TreasuryOutboundPaymentsResource<'a> {
    /// Creates a new `TreasuryOutboundPaymentsResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            account: None,
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Scopes this resource to operate on behalf of a specific connected account.
    pub fn with_account(&self, account_id: &'a str) -> Self {
        Self {
            client: self.client,
            account: Some(account_id),
        }
    }

    /// Return the connected account ID if scoped.
    pub fn account(&self) -> Option<&'a str> {
        self.account
    }
}
