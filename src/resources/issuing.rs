//! Stripe Issuing resource facade (`stripe.issuing`).
//!
//! Provides access to commercial card issuance, cardholders, real-time authorizations,
//! and ledger transactions modeled after `stripe-node`.
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
//! let issuing = client.issuing();
//! let _cards = issuing.cards();
//! let _cardholders = issuing.cardholders();
//! let _authorizations = issuing.authorizations();
//! let _transactions = issuing.transactions();
//! # Ok(())
//! # }
//! ```

use some_stripe_core::StripeClient;

/// Facade resource for Stripe Issuing operations (`stripe.issuing`).
///
/// Provides access to cards, cardholders, authorizations, and transactions.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
///
/// let client = StripeClient::new("sk_test_123");
/// let issuing = client.issuing();
/// let _cards = issuing.cards();
/// ```
#[derive(Clone, Copy)]
pub struct IssuingResource<'a> {
    /// Issuing Cards resource (`stripe.issuing.cards`).
    pub cards: IssuingCardsResource<'a>,
    /// Issuing Cardholders resource (`stripe.issuing.cardholders`).
    pub cardholders: IssuingCardholdersResource<'a>,
    /// Issuing Authorizations resource (`stripe.issuing.authorizations`).
    pub authorizations: IssuingAuthorizationsResource<'a>,
    /// Issuing Transactions resource (`stripe.issuing.transactions`).
    pub transactions: IssuingTransactionsResource<'a>,
    client: &'a StripeClient,
}

impl<'a> IssuingResource<'a> {
    /// Creates a new `IssuingResource` bound to the provided [`StripeClient`].
    pub fn new(client: &'a StripeClient) -> Self {
        Self {
            cards: IssuingCardsResource::new(client),
            cardholders: IssuingCardholdersResource::new(client),
            authorizations: IssuingAuthorizationsResource::new(client),
            transactions: IssuingTransactionsResource::new(client),
            client,
        }
    }

    /// Access reference to the underlying [`StripeClient`].
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Access the Issuing Cards sub-resource (`stripe.issuing.cards`).
    pub fn cards(&self) -> &IssuingCardsResource<'a> {
        &self.cards
    }

    /// Access the Issuing Cardholders sub-resource (`stripe.issuing.cardholders`).
    pub fn cardholders(&self) -> &IssuingCardholdersResource<'a> {
        &self.cardholders
    }

    /// Access the Issuing Authorizations sub-resource (`stripe.issuing.authorizations`).
    pub fn authorizations(&self) -> &IssuingAuthorizationsResource<'a> {
        &self.authorizations
    }

    /// Access the Issuing Transactions sub-resource (`stripe.issuing.transactions`).
    pub fn transactions(&self) -> &IssuingTransactionsResource<'a> {
        &self.transactions
    }
}

/// Resource for managing Stripe Issuing Cards (`stripe.issuing.cards`).
#[derive(Clone, Copy)]
pub struct IssuingCardsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> IssuingCardsResource<'a> {
    /// Creates a new `IssuingCardsResource` bound to the provided [`StripeClient`].
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

/// Resource for managing Stripe Issuing Cardholders (`stripe.issuing.cardholders`).
#[derive(Clone, Copy)]
pub struct IssuingCardholdersResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> IssuingCardholdersResource<'a> {
    /// Creates a new `IssuingCardholdersResource` bound to the provided [`StripeClient`].
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

/// Resource for managing Stripe Issuing Authorizations (`stripe.issuing.authorizations`).
#[derive(Clone, Copy)]
pub struct IssuingAuthorizationsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> IssuingAuthorizationsResource<'a> {
    /// Creates a new `IssuingAuthorizationsResource` bound to the provided [`StripeClient`].
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

/// Resource for managing Stripe Issuing Transactions (`stripe.issuing.transactions`).
#[derive(Clone, Copy)]
pub struct IssuingTransactionsResource<'a> {
    client: &'a StripeClient,
    account: Option<&'a str>,
}

impl<'a> IssuingTransactionsResource<'a> {
    /// Creates a new `IssuingTransactionsResource` bound to the provided [`StripeClient`].
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
