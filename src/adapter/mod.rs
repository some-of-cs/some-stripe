//! Optional `some-payment-core` adapter implementing `PaymentProvider` for Stripe.
//!
//! Enabled via the `some-payment-adapter` Cargo feature without impacting standalone
//! `some-stripe` consumers.

pub mod mappers;
pub mod provider;

#[cfg(test)]
mod conformance_tests;
