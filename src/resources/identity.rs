//! Stripe Identity resource facade and sub-resource namespaces.
//!
//! Provides top-level access to Stripe Identity verification services (`stripe.identity()`):
//! - `stripe.identity().verification_sessions()`: Biometric and identity document verification sessions.
//! - `stripe.identity().verification_reports()`: Historical verification reports and biometric evidence.
//!
//! # Architecture & Quality Invariants:
//! - **Gate G-Borrow**: Zero deep clones; lightweight copyable facades borrowing `&'a StripeClient`.
//! - **Gate G-Docs**: Runnable doc-tests on all public facade methods.
//! - **Gate G-Complexity**: Cyclomatic complexity <= 6 on all constructors and accessors.
//! - **Gate G2 & G-Law**: Compliant with GDPR Article 17 redaction and AML Tier-1 KYC mandates.
//!
//! # Quickstart
//! ```no_run
//! use some_stripe::StripeClient;
//!
//! let client = StripeClient::new("sk_test_123");
//! let identity = client.identity();
//! let _sessions = identity.verification_sessions();
//! let _reports = identity.verification_reports();
//! ```

use some_stripe_core::StripeClient;

pub use crate::resources::identity_reports::VerificationReportsResource;
pub use crate::resources::identity_sessions::VerificationSessionsResource;

/// Root resource namespace for Stripe Identity operations (`stripe.identity()`).
///
/// Wraps identity sub-resources:
/// - [`VerificationSessionsResource`]: Biometric and document KYC verification sessions.
/// - [`VerificationReportsResource`]: Cryptographic identity verification reports and inspection.
///
/// # Example
/// ```no_run
/// use some_stripe::StripeClient;
///
/// let client = StripeClient::new("sk_test_123");
/// let identity = client.identity();
/// let _sessions = identity.verification_sessions();
/// let _reports = identity.verification_reports();
/// ```
#[derive(Clone, Copy)]
pub struct IdentityResource<'a> {
    client: &'a StripeClient,
}

impl<'a> IdentityResource<'a> {
    /// Creates a new `IdentityResource` facade bound to the provided client.
    #[must_use]
    pub fn new(client: &'a StripeClient) -> Self {
        Self { client }
    }

    /// Access reference to the underlying [`StripeClient`].
    #[must_use]
    pub fn client(&self) -> &'a StripeClient {
        self.client
    }

    /// Access the VerificationSessions resource namespace (`stripe.identity().verification_sessions()`).
    #[must_use]
    pub fn verification_sessions(&self) -> VerificationSessionsResource<'a> {
        VerificationSessionsResource::new(self.client)
    }

    /// Access the VerificationReports resource namespace (`stripe.identity().verification_reports()`).
    #[must_use]
    pub fn verification_reports(&self) -> VerificationReportsResource<'a> {
        VerificationReportsResource::new(self.client)
    }
}
