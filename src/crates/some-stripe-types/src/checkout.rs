//! # Stripe Checkout Session & Line Item Domain Entities
//!
//! Strongly typed domain models, line item lists, and parameter DTOs for Stripe Checkout,
//! adhering to Stripe API conventions and ISO 4217 integer minor currency units.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Checkout session status indicating the customer has not yet completed payment.
pub const CHECKOUT_SESSION_STATUS_OPEN: &str = "open";
/// Checkout session status indicating payment was completed.
pub const CHECKOUT_SESSION_STATUS_COMPLETE: &str = "complete";
/// Checkout session status indicating the session has expired.
pub const CHECKOUT_SESSION_STATUS_EXPIRED: &str = "expired";

/// Payment status indicating funds have been collected.
pub const PAYMENT_STATUS_PAID: &str = "paid";
/// Payment status indicating payment is still pending or not yet submitted.
pub const PAYMENT_STATUS_UNPAID: &str = "unpaid";
/// Payment status for free or trial sessions requiring no initial payment.
pub const PAYMENT_STATUS_NO_PAYMENT_REQUIRED: &str = "no_payment_required";

/// Session mode for one-off payments.
pub const CHECKOUT_SESSION_MODE_PAYMENT: &str = "payment";
/// Session mode for saving payment details for future use without immediate charge.
pub const CHECKOUT_SESSION_MODE_SETUP: &str = "setup";
/// Session mode for recurring subscription checkouts.
pub const CHECKOUT_SESSION_MODE_SUBSCRIPTION: &str = "subscription";

/// Minimum expiration window required by Stripe (30 minutes = 1,800 seconds).
pub const MIN_EXPIRATION_SECONDS: i64 = 1_800;
/// Maximum expiration window permitted by Stripe (24 hours = 86,400 seconds).
pub const MAX_EXPIRATION_SECONDS: i64 = 86_400;
/// Allowable clock drift tolerance guard in seconds (Gate G2: 60 seconds).
pub const EXPIRATION_TIMESTAMP_DRIFT_GUARD_SECONDS: i64 = 60;

/// Errors returned during Checkout Session expiration validation and lifecycle transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckoutSessionLifecycleError {
    /// Expiration timestamp is too soon (less than 30 minutes from creation).
    ExpirationTooSoon {
        expires_at: i64,
        created_at: i64,
        min_allowed: i64,
    },
    /// Expiration timestamp is too late (more than 24 hours from creation).
    ExpirationTooLate {
        expires_at: i64,
        created_at: i64,
        max_allowed: i64,
    },
    /// Expiration timestamp is in the past relative to creation time (clock drift guard).
    ExpirationInPast { expires_at: i64, created_at: i64 },
    /// Illegal lifecycle status transition.
    InvalidTransition {
        from: String,
        to: String,
        reason: &'static str,
    },
}

impl std::fmt::Display for CheckoutSessionLifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExpirationTooSoon {
                expires_at,
                created_at,
                min_allowed,
            } => {
                write!(
                    f,
                    "CheckoutSession expires_at ({expires_at}) is too soon; must be at least 30 minutes after creation ({created_at}), minimum allowed is {min_allowed}"
                )
            }
            Self::ExpirationTooLate {
                expires_at,
                created_at,
                max_allowed,
            } => {
                write!(
                    f,
                    "CheckoutSession expires_at ({expires_at}) is too late; must be at most 24 hours after creation ({created_at}), maximum allowed is {max_allowed}"
                )
            }
            Self::ExpirationInPast {
                expires_at,
                created_at,
            } => {
                write!(
                    f,
                    "CheckoutSession expires_at ({expires_at}) cannot be in the past relative to creation ({created_at})"
                )
            }
            Self::InvalidTransition { from, to, reason } => {
                write!(
                    f,
                    "Invalid CheckoutSession lifecycle transition from '{from}' to '{to}': {reason}"
                )
            }
        }
    }
}

impl std::error::Error for CheckoutSessionLifecycleError {}

/// Validates an expiration timestamp against Stripe requirements (30 mins to 24 hours).
pub fn validate_expires_at(
    expires_at: i64,
    created_at: i64,
) -> Result<(), CheckoutSessionLifecycleError> {
    if expires_at < created_at - EXPIRATION_TIMESTAMP_DRIFT_GUARD_SECONDS {
        return Err(CheckoutSessionLifecycleError::ExpirationInPast {
            expires_at,
            created_at,
        });
    }
    let min_allowed = created_at + MIN_EXPIRATION_SECONDS;
    if expires_at < min_allowed {
        return Err(CheckoutSessionLifecycleError::ExpirationTooSoon {
            expires_at,
            created_at,
            min_allowed,
        });
    }
    let max_allowed = created_at + MAX_EXPIRATION_SECONDS;
    if expires_at > max_allowed {
        return Err(CheckoutSessionLifecycleError::ExpirationTooLate {
            expires_at,
            created_at,
            max_allowed,
        });
    }
    Ok(())
}

/// Returns the default object type string for CheckoutSession (`"checkout.session"`).
#[must_use]
pub fn default_checkout_session_object() -> String {
    "checkout.session".into()
}

/// Returns the default object type string for LineItem (`"item"`).
#[must_use]
pub fn default_line_item_object() -> String {
    "item".into()
}

/// Returns the default object type string for LineItemList (`"list"`).
#[must_use]
pub fn default_list_object() -> String {
    "list".into()
}

/// Returns the default checkout session mode (`"payment"`).
#[must_use]
pub fn default_payment_mode() -> String {
    CHECKOUT_SESSION_MODE_PAYMENT.into()
}

/// Returns the default checkout line item quantity (`1`).
#[must_use]
pub const fn default_quantity() -> u32 {
    1
}

/// Represents a Stripe Checkout Session object (`checkout.session`).
///
/// Checkout Sessions represent your customer's session as they pay for
/// one-time purchases or subscriptions through Stripe Checkout.
///
/// # Example
/// ```
/// use some_stripe_types::CheckoutSession;
///
/// let json = r#"{
///     "id": "cs_test_a1b2c3",
///     "object": "checkout.session",
///     "status": "open",
///     "payment_status": "unpaid",
///     "mode": "payment",
///     "amount_total": 4500,
///     "currency": "usd"
/// }"#;
/// let session: CheckoutSession = serde_json::from_str(json).unwrap();
/// assert_eq!(session.id, "cs_test_a1b2c3");
/// assert!(session.is_open());
/// assert!(session.is_unpaid());
/// assert_eq!(session.amount_total, Some(4500));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutSession {
    /// Unique identifier for the checkout session (`cs_...`).
    pub id: String,

    /// Object type, always `checkout.session`.
    #[serde(default = "default_checkout_session_object")]
    pub object: String,

    /// The URL to the hosted checkout page, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// The status of the checkout session: `open`, `complete`, or `expired`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// The payment status: `paid`, `unpaid`, or `no_payment_required`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_status: Option<String>,

    /// The mode of the checkout session: `payment`, `setup`, or `subscription`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,

    /// Total amount of items in minor units (e.g., cents) after discounts and taxes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_total: Option<i64>,

    /// Total of all items before discounts or taxes in minor units (e.g., cents).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_subtotal: Option<i64>,

    /// Three-letter ISO currency code in lowercase (e.g., `usd`, `eur`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    /// The ID of the customer for this session (`cus_...`), if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,

    /// A unique string to reference the Checkout Session on your service.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_reference_id: Option<String>,

    /// The URL the customer will be directed to upon successful payment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_url: Option<String>,

    /// The URL the customer will be directed to if they decide to cancel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancel_url: Option<String>,

    /// The ID of the PaymentIntent created (`pi_...`), if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_intent: Option<String>,

    /// The timestamp at which the checkout session expires (UNIX epoch seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,

    /// Time at which the object was created (UNIX epoch seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,

    /// Set of key-value pairs that you can attach to an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// The line items associated with this checkout session, if expanded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_items: Option<LineItemList>,
}

impl CheckoutSession {
    /// Creates a new minimal `CheckoutSession`.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            object: default_checkout_session_object(),
            url: None,
            status: None,
            payment_status: None,
            mode: None,
            amount_total: None,
            amount_subtotal: None,
            currency: None,
            customer: None,
            client_reference_id: None,
            success_url: None,
            cancel_url: None,
            payment_intent: None,
            expires_at: None,
            created: None,
            metadata: None,
            line_items: None,
        }
    }

    /// Builder method to attach URL.
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Builder method to attach status.
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Builder method to attach payment status.
    pub fn with_payment_status(mut self, payment_status: impl Into<String>) -> Self {
        self.payment_status = Some(payment_status.into());
        self
    }

    /// Builder method to attach mode.
    pub fn with_mode(mut self, mode: impl Into<String>) -> Self {
        self.mode = Some(mode.into());
        self
    }

    /// Builder method to attach amounts.
    pub fn with_amounts(mut self, subtotal: i64, total: i64) -> Self {
        self.amount_subtotal = Some(subtotal);
        self.amount_total = Some(total);
        self
    }

    /// Builder method to attach currency.
    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = Some(currency.into());
        self
    }

    /// Builder method to attach line items.
    pub fn with_line_items(mut self, line_items: LineItemList) -> Self {
        self.line_items = Some(line_items);
        self
    }

    /// Returns true if the checkout session is in `open` status.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.status.as_deref() == Some(CHECKOUT_SESSION_STATUS_OPEN)
    }

    /// Returns true if the checkout session is in `complete` status.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.status.as_deref() == Some(CHECKOUT_SESSION_STATUS_COMPLETE)
    }

    /// Returns true if the checkout session is in `expired` status.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.status.as_deref() == Some(CHECKOUT_SESSION_STATUS_EXPIRED)
    }

    /// Returns true if payment has been made (`payment_status == "paid"`).
    #[must_use]
    pub fn is_paid(&self) -> bool {
        self.payment_status.as_deref() == Some(PAYMENT_STATUS_PAID)
    }

    /// Returns true if payment has not been made (`payment_status == "unpaid"`).
    #[must_use]
    pub fn is_unpaid(&self) -> bool {
        self.payment_status.as_deref() == Some(PAYMENT_STATUS_UNPAID)
    }

    /// Returns true if no payment is required (`payment_status == "no_payment_required"`).
    #[must_use]
    pub fn is_no_payment_required(&self) -> bool {
        self.payment_status.as_deref() == Some(PAYMENT_STATUS_NO_PAYMENT_REQUIRED)
    }

    /// Returns true if the session mode is `payment`.
    #[must_use]
    pub fn is_payment_mode(&self) -> bool {
        self.mode.as_deref() == Some(CHECKOUT_SESSION_MODE_PAYMENT)
    }

    /// Returns true if the session mode is `setup`.
    #[must_use]
    pub fn is_setup_mode(&self) -> bool {
        self.mode.as_deref() == Some(CHECKOUT_SESSION_MODE_SETUP)
    }

    /// Returns true if the session mode is `subscription`.
    #[must_use]
    pub fn is_subscription_mode(&self) -> bool {
        self.mode.as_deref() == Some(CHECKOUT_SESSION_MODE_SUBSCRIPTION)
    }

    /// Returns a borrowed slice of the session's line items, if expanded (Gate G-Borrow).
    #[must_use]
    pub fn line_items_data(&self) -> Option<&[LineItem]> {
        self.line_items.as_ref().map(|l| l.data.as_slice())
    }

    /// Validates and applies a status transition in accordance with the Stripe Checkout lifecycle state machine.
    pub fn transition_to(&mut self, target: &str) -> Result<(), CheckoutSessionLifecycleError> {
        let current = self
            .status
            .as_deref()
            .unwrap_or(CHECKOUT_SESSION_STATUS_OPEN);
        if current == target {
            return Ok(());
        }
        match (current, target) {
            (CHECKOUT_SESSION_STATUS_OPEN, CHECKOUT_SESSION_STATUS_COMPLETE) => {
                self.status = Some(CHECKOUT_SESSION_STATUS_COMPLETE.into());
                self.payment_status = Some(PAYMENT_STATUS_PAID.into());
                Ok(())
            }
            (CHECKOUT_SESSION_STATUS_OPEN, CHECKOUT_SESSION_STATUS_EXPIRED) => {
                self.status = Some(CHECKOUT_SESSION_STATUS_EXPIRED.into());
                Ok(())
            }
            (CHECKOUT_SESSION_STATUS_COMPLETE, CHECKOUT_SESSION_STATUS_EXPIRED) => {
                Err(CheckoutSessionLifecycleError::InvalidTransition {
                    from: current.into(),
                    to: target.into(),
                    reason: "Cannot expire a completed checkout session",
                })
            }
            (CHECKOUT_SESSION_STATUS_EXPIRED, CHECKOUT_SESSION_STATUS_COMPLETE) => {
                Err(CheckoutSessionLifecycleError::InvalidTransition {
                    from: current.into(),
                    to: target.into(),
                    reason: "Cannot complete an expired checkout session",
                })
            }
            (CHECKOUT_SESSION_STATUS_COMPLETE, CHECKOUT_SESSION_STATUS_OPEN) => {
                Err(CheckoutSessionLifecycleError::InvalidTransition {
                    from: current.into(),
                    to: target.into(),
                    reason: "Cannot reopen a completed checkout session",
                })
            }
            (CHECKOUT_SESSION_STATUS_EXPIRED, CHECKOUT_SESSION_STATUS_OPEN) => {
                Err(CheckoutSessionLifecycleError::InvalidTransition {
                    from: current.into(),
                    to: target.into(),
                    reason: "Cannot reopen an expired checkout session",
                })
            }
            _ => Err(CheckoutSessionLifecycleError::InvalidTransition {
                from: current.into(),
                to: target.into(),
                reason: "Unsupported state transition",
            }),
        }
    }

    /// Transitions the checkout session to `complete` upon successful payment.
    pub fn mark_completed(&mut self) -> Result<(), CheckoutSessionLifecycleError> {
        self.transition_to(CHECKOUT_SESSION_STATUS_COMPLETE)
    }

    /// Transitions the checkout session to `expired` upon manual expiration or timeout.
    pub fn mark_expired(&mut self) -> Result<(), CheckoutSessionLifecycleError> {
        self.transition_to(CHECKOUT_SESSION_STATUS_EXPIRED)
    }

    /// Returns true if the session is abandoned (status is open and current timestamp exceeds `expires_at`).
    #[must_use]
    pub fn is_abandoned(&self, current_timestamp: i64) -> bool {
        if !self.is_open() {
            return false;
        }
        match self.expires_at {
            Some(exp) => current_timestamp >= exp,
            None => false,
        }
    }

    /// Returns a recovery URL for this checkout session to resume checkout or handle abandonment.
    #[must_use]
    pub fn recovery_url(&self) -> String {
        if let Some(ref url) = self.url {
            url.clone()
        } else {
            format!("https://checkout.stripe.com/c/recover/{}", self.id)
        }
    }
}

/// Represents a Stripe LineItem object (`item`).
///
/// A LineItem represents an item being purchased in a Checkout Session.
///
/// # Example
/// ```
/// use some_stripe_types::LineItem;
///
/// let item = LineItem::new("li_123", 2500, 2500, "usd")
///     .with_description("Premium Plan")
///     .with_quantity(1);
/// assert_eq!(item.id, "li_123");
/// assert_eq!(item.amount_total, 2500);
/// assert_eq!(item.quantity, Some(1));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineItem {
    /// Unique identifier for the line item (`li_...`).
    pub id: String,

    /// Object type, always `item`.
    #[serde(default = "default_line_item_object")]
    pub object: String,

    /// An arbitrary string attached to the object, often useful for displaying to users.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Total amount of the line item in minor units (e.g., cents) after discounts and taxes.
    pub amount_total: i64,

    /// Total of the line item before discounts or taxes in minor units (e.g., cents).
    pub amount_subtotal: i64,

    /// The amount of tax calculated for this line item in minor units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_tax: Option<i64>,

    /// The amount of discount applied to this line item in minor units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_discount: Option<i64>,

    /// Three-letter ISO currency code in lowercase (e.g., `usd`, `eur`).
    pub currency: String,

    /// The quantity of the item purchased.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<u32>,
}

impl LineItem {
    /// Creates a new `LineItem` with mandatory amount and currency fields.
    pub fn new(
        id: impl Into<String>,
        amount_subtotal: i64,
        amount_total: i64,
        currency: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            object: default_line_item_object(),
            description: None,
            amount_total,
            amount_subtotal,
            amount_tax: None,
            amount_discount: None,
            currency: currency.into(),
            quantity: None,
        }
    }

    /// Sets the optional description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the optional quantity.
    pub fn with_quantity(mut self, quantity: u32) -> Self {
        self.quantity = Some(quantity);
        self
    }

    /// Sets the optional tax amount in minor units.
    pub fn with_tax(mut self, amount_tax: i64) -> Self {
        self.amount_tax = Some(amount_tax);
        self
    }

    /// Sets the optional discount amount in minor units.
    pub fn with_discount(mut self, amount_discount: i64) -> Self {
        self.amount_discount = Some(amount_discount);
        self
    }
}

/// Represents a paginated list of Stripe LineItem objects (`list`).
///
/// Implements [`std::ops::Deref`] and [`IntoIterator`] for zero-copy borrowing (Gate G-Borrow).
///
/// # Example
/// ```
/// use some_stripe_types::{LineItem, LineItemList};
///
/// let item = LineItem::new("li_1", 1000, 1000, "usd");
/// let list = LineItemList::new(vec![item], false, "/v1/checkout/sessions/cs_123/line_items");
/// assert_eq!(list.len(), 1);
/// assert!(!list.has_more);
/// assert_eq!(list[0].id, "li_1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineItemList {
    /// Object type, always `list`.
    #[serde(default = "default_list_object")]
    pub object: String,

    /// A list of line items (`LineItem`).
    #[serde(default)]
    pub data: Vec<LineItem>,

    /// True if this list has another page of items that can be fetched.
    #[serde(default)]
    pub has_more: bool,

    /// The URL where this list can be accessed.
    pub url: String,
}

impl LineItemList {
    /// Creates a new `LineItemList`.
    pub fn new(data: Vec<LineItem>, has_more: bool, url: impl Into<String>) -> Self {
        Self {
            object: default_list_object(),
            data,
            has_more,
            url: url.into(),
        }
    }

    /// Creates an empty `LineItemList` with a target URL.
    pub fn empty(url: impl Into<String>) -> Self {
        Self {
            object: default_list_object(),
            data: Vec::new(),
            has_more: false,
            url: url.into(),
        }
    }

    /// Returns the number of line items in this list.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the list contains no line items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns a borrowed slice of the line items (Gate G-Borrow).
    #[must_use]
    pub fn as_slice(&self) -> &[LineItem] {
        &self.data
    }
}

impl std::ops::Deref for LineItemList {
    type Target = [LineItem];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl AsRef<[LineItem]> for LineItemList {
    fn as_ref(&self) -> &[LineItem] {
        &self.data
    }
}

impl<'a> IntoIterator for &'a LineItemList {
    type Item = &'a LineItem;
    type IntoIter = std::slice::Iter<'a, LineItem>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

/// Extracts the line item ID from a [`LineItem`].
///
/// Conforms to the `fn(&T) -> &str` function pointer signature required by `Paginator::new`.
pub fn line_item_id(item: &LineItem) -> &str {
    &item.id
}

pub use crate::params::{
    CreateCheckoutSessionParams, CreateLineItemParams, LineItemParams, LineItemPriceDataParams,
    LineItemProductDataParams, ListLineItemsParams,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkout_session_defaults() {
        let session = CheckoutSession::new("cs_test_minimal");
        assert_eq!(session.id, "cs_test_minimal");
        assert_eq!(session.object, "checkout.session");
        assert!(session.url.is_none());
        assert!(session.status.is_none());
        assert!(session.payment_status.is_none());
        assert!(session.mode.is_none());
        assert!(session.amount_total.is_none());
        assert!(session.amount_subtotal.is_none());
        assert!(session.currency.is_none());
        assert!(session.customer.is_none());
        assert!(session.client_reference_id.is_none());
        assert!(session.success_url.is_none());
        assert!(session.cancel_url.is_none());
        assert!(session.payment_intent.is_none());
        assert!(session.expires_at.is_none());
        assert!(session.created.is_none());
        assert!(session.metadata.is_none());
        assert!(session.line_items.is_none());
        assert!(!session.is_open());
        assert!(!session.is_complete());
        assert!(!session.is_expired());
        assert!(!session.is_paid());
        assert!(!session.is_unpaid());
        assert!(!session.is_no_payment_required());
    }

    #[test]
    fn test_checkout_session_lifecycle_helpers() {
        let mut session = CheckoutSession::new("cs_lifecycle")
            .with_status(CHECKOUT_SESSION_STATUS_OPEN)
            .with_payment_status(PAYMENT_STATUS_UNPAID)
            .with_mode(CHECKOUT_SESSION_MODE_PAYMENT);

        assert!(session.is_open());
        assert!(!session.is_complete());
        assert!(!session.is_expired());
        assert!(session.is_unpaid());
        assert!(!session.is_paid());
        assert!(!session.is_no_payment_required());
        assert!(session.is_payment_mode());
        assert!(!session.is_setup_mode());
        assert!(!session.is_subscription_mode());

        session.status = Some(CHECKOUT_SESSION_STATUS_COMPLETE.into());
        session.payment_status = Some(PAYMENT_STATUS_PAID.into());
        assert!(!session.is_open());
        assert!(session.is_complete());
        assert!(session.is_paid());
        assert!(!session.is_unpaid());

        session.status = Some(CHECKOUT_SESSION_STATUS_EXPIRED.into());
        assert!(session.is_expired());

        session.payment_status = Some(PAYMENT_STATUS_NO_PAYMENT_REQUIRED.into());
        assert!(session.is_no_payment_required());

        session.mode = Some(CHECKOUT_SESSION_MODE_SETUP.into());
        assert!(session.is_setup_mode());

        session.mode = Some(CHECKOUT_SESSION_MODE_SUBSCRIPTION.into());
        assert!(session.is_subscription_mode());
    }

    #[test]
    fn test_line_item_builders_and_defaults() {
        let item = LineItem::new("li_test_builder", 1500, 1650, "usd")
            .with_description("E-book download")
            .with_quantity(2)
            .with_tax(150)
            .with_discount(0);

        assert_eq!(item.id, "li_test_builder");
        assert_eq!(item.object, "item");
        assert_eq!(item.description.as_deref(), Some("E-book download"));
        assert_eq!(item.amount_subtotal, 1500);
        assert_eq!(item.amount_total, 1650);
        assert_eq!(item.amount_tax, Some(150));
        assert_eq!(item.amount_discount, Some(0));
        assert_eq!(item.currency, "usd");
        assert_eq!(item.quantity, Some(2));
    }

    #[test]
    fn test_line_item_list_deref_and_borrow() {
        let item1 = LineItem::new("li_001", 1000, 1000, "usd").with_quantity(1);
        let item2 = LineItem::new("li_002", 2000, 2000, "usd").with_quantity(2);

        let list = LineItemList::new(
            vec![item1, item2],
            false,
            "/v1/checkout/sessions/cs_test/line_items",
        );

        assert_eq!(list.len(), 2);
        assert!(!list.is_empty());
        assert_eq!(list.object, "list");
        assert!(!list.has_more);
        assert_eq!(list.url, "/v1/checkout/sessions/cs_test/line_items");

        // Test Deref to slice (Gate G-Borrow)
        assert_eq!(list[0].id, "li_001");
        assert_eq!(list[1].id, "li_002");

        // Test IntoIterator for &LineItemList without clone
        let mut count = 0;
        for item in &list {
            assert!(item.id.starts_with("li_"));
            count += 1;
        }
        assert_eq!(count, 2);

        // Test empty constructor
        let empty_list = LineItemList::empty("/v1/checkout/sessions/cs_empty/line_items");
        assert!(empty_list.is_empty());
        assert_eq!(empty_list.len(), 0);
    }

    #[test]
    fn test_create_params_builders() {
        let params = CreateCheckoutSessionParams::single_item(
            "https://example.com/success",
            "usd",
            5000,
            "Pro Membership",
            1,
        )
        .cancel_url("https://example.com/cancel")
        .mode("payment")
        .client_reference_id("user_42")
        .expires_at(1728400000)
        .idempotency_key("idem_checkout_123");

        assert_eq!(params.success_url, "https://example.com/success");
        assert_eq!(
            params.cancel_url.as_deref(),
            Some("https://example.com/cancel")
        );
        assert_eq!(params.mode, "payment");
        assert_eq!(params.client_reference_id.as_deref(), Some("user_42"));
        assert_eq!(params.currency.as_deref(), Some("usd"));
        assert_eq!(params.unit_amount, Some(5000));
        assert_eq!(params.product_name.as_deref(), Some("Pro Membership"));
        assert_eq!(params.quantity, Some(1));
        assert_eq!(params.expires_at, Some(1728400000));
        assert_eq!(params.idempotency_key.as_deref(), Some("idem_checkout_123"));
    }

    #[test]
    fn test_validate_expires_at_bounds_and_drift() {
        let created = 1_700_000_000;

        // Valid: exactly 30 minutes after creation
        assert!(validate_expires_at(created + 1_800, created).is_ok());

        // Valid: 2 hours after creation
        assert!(validate_expires_at(created + 7_200, created).is_ok());

        // Valid: exactly 24 hours after creation
        assert!(validate_expires_at(created + 86_400, created).is_ok());

        // Invalid: 29 minutes after creation (too soon)
        let too_soon = validate_expires_at(created + 1_799, created).unwrap_err();
        assert!(matches!(
            too_soon,
            CheckoutSessionLifecycleError::ExpirationTooSoon { .. }
        ));

        // Invalid: 24 hours + 1 second (too late)
        let too_late = validate_expires_at(created + 86_401, created).unwrap_err();
        assert!(matches!(
            too_late,
            CheckoutSessionLifecycleError::ExpirationTooLate { .. }
        ));

        // Invalid: in past beyond drift guard
        let in_past = validate_expires_at(created - 100, created).unwrap_err();
        assert!(matches!(
            in_past,
            CheckoutSessionLifecycleError::ExpirationInPast { .. }
        ));
    }

    #[test]
    fn test_session_state_machine_legal_and_illegal_transitions() {
        let mut session =
            CheckoutSession::new("cs_test_state").with_status(CHECKOUT_SESSION_STATUS_OPEN);

        // Open -> Complete is legal
        assert!(session.mark_completed().is_ok());
        assert!(session.is_complete());
        assert!(session.is_paid());

        // Complete -> Expired is illegal
        let err = session.mark_expired().unwrap_err();
        assert!(matches!(
            err,
            CheckoutSessionLifecycleError::InvalidTransition { .. }
        ));
        assert!(
            err.to_string()
                .contains("Cannot expire a completed checkout session")
        );

        // Complete -> Open is illegal
        let reopen_err = session
            .transition_to(CHECKOUT_SESSION_STATUS_OPEN)
            .unwrap_err();
        assert!(
            reopen_err
                .to_string()
                .contains("Cannot reopen a completed checkout session")
        );

        // Fresh session: Open -> Expired is legal
        let mut session_expire =
            CheckoutSession::new("cs_test_expire").with_status(CHECKOUT_SESSION_STATUS_OPEN);
        assert!(session_expire.mark_expired().is_ok());
        assert!(session_expire.is_expired());

        // Expired -> Complete is illegal
        let err2 = session_expire.mark_completed().unwrap_err();
        assert!(
            err2.to_string()
                .contains("Cannot complete an expired checkout session")
        );
    }

    #[test]
    fn test_abandonment_tracking_and_recovery_url() {
        let created = 1_700_000_000;
        let expires_at = created + 1_800;

        let mut session =
            CheckoutSession::new("cs_test_abandon").with_status(CHECKOUT_SESSION_STATUS_OPEN);
        session.expires_at = Some(expires_at);

        // Not abandoned before expiration
        assert!(!session.is_abandoned(created + 900));

        // Abandoned once current time reaches expires_at
        assert!(session.is_abandoned(expires_at));
        assert!(session.is_abandoned(expires_at + 10));

        // Not abandoned if completed
        session.status = Some(CHECKOUT_SESSION_STATUS_COMPLETE.into());
        assert!(!session.is_abandoned(expires_at + 10));

        // Recovery URL generation
        let session_no_url = CheckoutSession::new("cs_rec_123");
        assert_eq!(
            session_no_url.recovery_url(),
            "https://checkout.stripe.com/c/recover/cs_rec_123"
        );

        let session_with_url = CheckoutSession::new("cs_rec_456")
            .with_url("https://checkout.stripe.com/c/pay/cs_rec_456");
        assert_eq!(
            session_with_url.recovery_url(),
            "https://checkout.stripe.com/c/pay/cs_rec_456"
        );
    }
}
