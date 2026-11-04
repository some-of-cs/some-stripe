//! API resource namespaces for Stripe operations.

pub mod accounts;
pub mod checkout;
pub mod payment_intents;
pub mod webhooks;

pub use accounts::AccountsResource;
pub use checkout::{CheckoutResource, SessionsResource};
pub use payment_intents::{PaymentIntentsResource, generate_uuid_v4, is_valid_uuid_v4};
pub use webhooks::WebhooksResource;
