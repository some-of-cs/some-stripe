//! API resource namespaces for Stripe operations.

pub mod accounts;
pub mod balance;
pub mod checkout;
pub mod credit_notes;
pub mod disputes;
pub mod invoices;
pub mod payment_intents;
pub mod payment_links;
pub mod quotes;
pub mod refunds;
pub mod webhooks;

pub use accounts::AccountsResource;
pub use balance::{BalanceResource, BalanceTransactionsResource};
pub use checkout::{
    CheckoutResource, CheckoutSessionExt, RedirectError, SessionsResource, is_fulfillment_ready,
    validate_and_encode_id, validate_redirect_url,
};
pub use credit_notes::{CreditNotesResource, credit_note_id};
pub use disputes::DisputesResource;
pub use invoices::InvoicesResource;
pub use payment_intents::{PaymentIntentsResource, generate_uuid_v4, is_valid_uuid_v4};
pub use payment_links::PaymentLinksResource;
pub use quotes::{QuotesResource, quote_id};
pub use refunds::RefundsResource;
pub use webhooks::WebhooksResource;
