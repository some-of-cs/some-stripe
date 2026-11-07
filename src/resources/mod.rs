//! API resource namespaces for Stripe operations.

pub mod account_links;
pub mod account_sessions;
pub mod accounts;
pub mod balance;
pub mod billing_portal;
pub mod checkout;
pub mod credit_notes;
pub mod customers;
pub mod disputes;
pub mod invoices;
pub mod payment_intents;
pub mod payment_links;
pub mod persons;
pub mod quotes;
pub mod refunds;
pub mod subscription_items;
pub mod subscription_schedules;
pub mod subscriptions;
pub mod webhooks;

pub use account_links::AccountLinksResource;
pub use account_sessions::AccountSessionsResource;
pub use accounts::AccountsResource;
pub use balance::{BalanceResource, BalanceTransactionsResource};
pub use billing_portal::BillingPortalResource;
pub use checkout::{
    CheckoutResource, CheckoutSessionExt, RedirectError, SessionsResource, is_fulfillment_ready,
    validate_and_encode_id, validate_redirect_url,
};
pub use credit_notes::{CreditNotesResource, credit_note_id};
pub use customers::CustomersResource;
pub use disputes::DisputesResource;
pub use invoices::InvoicesResource;
pub use payment_intents::{PaymentIntentsResource, generate_uuid_v4, is_valid_uuid_v4};
pub use payment_links::PaymentLinksResource;
pub use persons::PersonsResource;
pub use quotes::{QuotesResource, quote_id};
pub use refunds::RefundsResource;
pub use subscription_items::SubscriptionItemsResource;
pub use subscription_schedules::SubscriptionSchedulesResource;
pub use subscriptions::SubscriptionsResource;
pub use webhooks::WebhooksResource;
