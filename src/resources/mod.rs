//! API resource namespaces for Stripe operations.

pub mod account_links;
pub mod account_sessions;
pub mod accounts;
pub mod application_fees;
pub mod balance;
pub mod billing_portal;
pub mod checkout;
pub mod credit_notes;
pub mod customers;
pub mod disputes;
pub mod identity;
pub mod identity_reports;
pub mod identity_sessions;
pub mod invoices;
pub mod issuing_authorizations;
pub mod issuing_cards;
pub mod issuing_transactions;
pub mod payment_intents;
pub mod payment_links;
pub mod payouts;
pub mod persons;
pub mod quotes;
pub mod refunds;
pub mod subscription_items;
pub mod subscription_schedules;
pub mod subscriptions;
pub mod tax;
pub mod tax_calculations;
pub mod tax_registrations;
pub mod tax_settings;
pub mod tax_transactions;
pub mod terminal;
pub mod transfers;
pub mod treasury;
pub mod treasury_financial_accounts;
pub mod treasury_transfers;
pub mod v2_destinations;
pub mod webhooks;

pub use account_links::AccountLinksResource;
pub use account_sessions::AccountSessionsResource;
pub use accounts::AccountsResource;
pub use application_fees::ApplicationFeesResource;
pub use balance::{BalanceResource, BalanceTransactionsResource};
pub use billing_portal::BillingPortalResource;
pub use checkout::{
    CheckoutResource, CheckoutSessionExt, RedirectError, SessionsResource, is_fulfillment_ready,
    validate_and_encode_id, validate_redirect_url,
};
pub use credit_notes::{CreditNotesResource, credit_note_id};
pub use customers::CustomersResource;
pub use disputes::DisputesResource;
pub use identity::IdentityResource;
pub use identity_reports::VerificationReportsResource;
pub use identity_sessions::VerificationSessionsResource;
pub use invoices::InvoicesResource;
pub use issuing_authorizations::IssuingAuthorizationsResource;
pub use issuing_cards::{IssuingCardholdersResource, IssuingCardsResource, IssuingResource};
pub use issuing_transactions::IssuingTransactionsResource;
pub use payment_intents::{PaymentIntentsResource, generate_uuid_v4, is_valid_uuid_v4};
pub use payment_links::PaymentLinksResource;
pub use payouts::PayoutsResource;
pub use persons::PersonsResource;
pub use quotes::{QuotesResource, quote_id};
pub use refunds::RefundsResource;
pub use subscription_items::SubscriptionItemsResource;
pub use subscription_schedules::SubscriptionSchedulesResource;
pub use subscriptions::SubscriptionsResource;
pub use tax::TaxResource;
pub use tax_calculations::CalculationsResource;
pub use tax_registrations::{TaxRegistrationsResource, tax_registration_id};
pub use tax_settings::TaxSettingsResource;
pub use tax_transactions::TransactionsResource;
pub use terminal::{
    TerminalConnectionTokensResource, TerminalLocationsResource, TerminalReadersResource,
    TerminalResource,
};
pub use transfers::TransfersResource;
pub use treasury::TreasuryResource;
pub use treasury_financial_accounts::TreasuryFinancialAccountsResource;
pub use treasury_transfers::{
    OutboundPaymentsResource, OutboundTransfersResource, ReceivedCreditsResource,
    TreasuryTransfersResource,
};
pub use v2_destinations::EventDestinationsResource;
pub use webhooks::WebhooksResource;

/// Resolves an idempotency key for state-mutating requests.
///
/// Preserves zero-clone borrowing (`Cow::Borrowed`) when caller provides a non-empty,
/// trimmed key, and generates an RFC 4122 compliant UUID v4 (`Cow::Owned`) when omitted
/// or whitespace-only.
pub(crate) fn resolve_idempotency_key(explicit: Option<&str>) -> std::borrow::Cow<'_, str> {
    match explicit.map(str::trim).filter(|k| !k.is_empty()) {
        Some(k) => std::borrow::Cow::Borrowed(k),
        None => std::borrow::Cow::Owned(generate_uuid_v4()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn test_resolve_idempotency_key_borrowed_when_explicit() {
        let explicit = "user_provided_key_123";
        let resolved = resolve_idempotency_key(Some(explicit));
        assert!(matches!(resolved, Cow::Borrowed(_)));
        assert_eq!(resolved.as_ref(), "user_provided_key_123");
    }

    #[test]
    fn test_resolve_idempotency_key_trims_whitespace_and_borrows() {
        let explicit = "   trimmed_key_456   ";
        let resolved = resolve_idempotency_key(Some(explicit));
        assert!(matches!(resolved, Cow::Borrowed(_)));
        assert_eq!(resolved.as_ref(), "trimmed_key_456");
    }

    #[test]
    fn test_resolve_idempotency_key_owned_uuid_on_none() {
        let resolved = resolve_idempotency_key(None);
        assert!(matches!(resolved, Cow::Owned(_)));
        assert!(is_valid_uuid_v4(resolved.as_ref()));
    }

    #[test]
    fn test_resolve_idempotency_key_owned_uuid_on_empty_or_whitespace() {
        let empty = resolve_idempotency_key(Some(""));
        assert!(matches!(empty, Cow::Owned(_)));
        assert!(is_valid_uuid_v4(empty.as_ref()));

        let whitespace = resolve_idempotency_key(Some("   \t\n  "));
        assert!(matches!(whitespace, Cow::Owned(_)));
        assert!(is_valid_uuid_v4(whitespace.as_ref()));
    }
}
