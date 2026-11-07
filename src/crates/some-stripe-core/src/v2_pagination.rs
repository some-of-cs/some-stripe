//! URL-driven pagination stream engine for Stripe API v2.
//!
//! Re-exports [`V2Paginator`] and [`resolve_v2_url`] from [`crate::patterns::v2_pagination`]
//! along with [`V2List`] and [`V2PageInfo`] from `some_stripe_types`.
//!
//! # Examples
//! ```rust
//! use some_stripe_core::v2_pagination::{V2List, V2PageInfo, V2Paginator};
//! use some_stripe_core::StripeClient;
//!
//! #[derive(serde::Deserialize)]
//! struct MockItem {
//!     id: String,
//! }
//!
//! let client = StripeClient::new("sk_test_123");
//! let list = V2List::new(
//!     vec![MockItem { id: "evt_1".into() }],
//!     Some("/v2/core/events?page=2".into()),
//!     None,
//!     Some(V2PageInfo::new(Some(1), Some(false))),
//! );
//!
//! let paginator = V2Paginator::from_page(&client, list);
//! assert_eq!(paginator.pages_fetched(), 1);
//! ```

pub use crate::patterns::v2_pagination::{V2Paginator, resolve_v2_url};
pub use some_stripe_types::{V2List, V2PageInfo};
