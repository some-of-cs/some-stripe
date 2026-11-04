//! API resource namespaces for Stripe operations.

pub mod accounts;
pub mod balance;
pub mod checkout;
pub mod disputes;
pub mod payment_intents;
pub mod payment_links;
pub mod refunds;
pub mod webhooks;

pub use accounts::AccountsResource;
pub use balance::{BalanceResource, BalanceTransactionsResource};
pub use checkout::{CheckoutResource, SessionsResource};
pub use disputes::DisputesResource;
pub use payment_intents::{PaymentIntentsResource, generate_uuid_v4, is_valid_uuid_v4};
pub use payment_links::PaymentLinksResource;
pub use refunds::RefundsResource;
pub use webhooks::WebhooksResource;
