//! Stripe API v2 Core Domain Entities and DTO Schemas.
//!
//! Models next-generation Stripe API v2 core resources including [`V2Event`],
//! [`V2ThinEvent`], [`V2RelatedObject`], [`EventDestination`], [`V2Account`], and [`V2List`].
//!
//! # Architecture & Quality Invariants
//! - **Gate G-Borrow**: Zero deep clone on domain entities; borrowed parameter schemas.
//! - **Gate G-Complexity**: Cyclomatic complexity $CC \le 6$ for domain predicates.
//! - **Gate G-Docs**: Runnable doctests verified against the test harness.

use serde::{Deserialize, Serialize};

/// Reference metadata pointing to a domain resource in a [`V2ThinEvent`] or [`V2Event`].
///
/// # Example
/// ```
/// use some_stripe_types::V2RelatedObject;
///
/// let related = V2RelatedObject {
///     id: "meter_123".into(),
///     object_type: "billing.meter".into(),
///     url: Some("https://api.stripe.com/v2/billing/meters/meter_123".into()),
/// };
/// assert_eq!(related.id, "meter_123");
/// assert_eq!(related.object_type, "billing.meter");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2RelatedObject {
    /// Identifier of the related Stripe resource (e.g., `"meter_123"`).
    pub id: String,

    /// Object type name of the related resource (e.g., `"billing.meter"`).
    #[serde(rename = "type")]
    pub object_type: String,

    /// Canonical REST API URL for hydrating the full resource object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Lightweight envelope for Stripe API v2 thin events.
///
/// Unlike API v1 events that embed full snapshots under `data.object`,
/// v2 thin events contain lightweight metadata and a [`V2RelatedObject`] reference.
///
/// # Example
/// ```
/// use some_stripe_types::V2ThinEvent;
///
/// let json = r#"{
///     "id": "v2_evt_abc123",
///     "object": "v2.core.event",
///     "type": "v1.billing.meter.error_report_triggered",
///     "created": 1728345600,
///     "livemode": false,
///     "related_object": {
///         "id": "meter_001",
///         "type": "billing.meter",
///         "url": "https://api.stripe.com/v2/billing/meters/meter_001"
///     }
/// }"#;
///
/// let event: V2ThinEvent = serde_json::from_str(json).unwrap();
/// assert_eq!(event.id, "v2_evt_abc123");
/// assert_eq!(event.event_type, "v1.billing.meter.error_report_triggered");
/// assert_eq!(event.related_object_id(), Some("meter_001"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2ThinEvent {
    /// Unique identifier for the v2 event (`v2_evt_...`).
    pub id: String,

    /// Object type descriptor, typically `"v2.core.event"`.
    pub object: String,

    /// Event type identifier (e.g., `"v1.billing.meter.error_report_triggered"`).
    #[serde(rename = "type")]
    pub event_type: String,

    /// Time at which the event occurred (UNIX epoch seconds).
    pub created: i64,

    /// Whether this event was generated in live mode (`true`) or test mode (`false`).
    pub livemode: bool,

    /// Reference to the underlying Stripe resource associated with this event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_object: Option<V2RelatedObject>,

    /// Optional event context or operational reason provided by Stripe API v2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

impl V2ThinEvent {
    /// Returns the ID of the related object, if present.
    #[must_use]
    pub fn related_object_id(&self) -> Option<&str> {
        self.related_object.as_ref().map(|ro| ro.id.as_str())
    }

    /// Returns the type identifier of the related object, if present.
    #[must_use]
    pub fn related_object_type(&self) -> Option<&str> {
        self.related_object
            .as_ref()
            .map(|ro| ro.object_type.as_str())
    }

    /// Returns the hydration URL for the related object, if present.
    #[must_use]
    pub fn related_object_url(&self) -> Option<&str> {
        self.related_object
            .as_ref()
            .and_then(|ro| ro.url.as_deref())
    }
}

/// Canonical type alias for Stripe API v2 thin event notifications.
pub type EventNotification = V2ThinEvent;

/// Strongly typed Stripe API v2 Core Event with full live resource state.
///
/// # Example
/// ```
/// use some_stripe_types::V2Event;
///
/// let json = r#"{
///     "id": "v2_evt_full_001",
///     "object": "v2.core.event",
///     "type": "v1.billing.meter.error_report_triggered",
///     "created": 1728345600,
///     "livemode": false,
///     "related_object": {
///         "id": "meter_001",
///         "type": "billing.meter",
///         "url": "https://api.stripe.com/v2/billing/meters/meter_001"
///     }
/// }"#;
///
/// let event: V2Event = serde_json::from_str(json).unwrap();
/// assert_eq!(event.id(), "v2_evt_full_001");
/// assert_eq!(event.event_type(), "v1.billing.meter.error_report_triggered");
/// assert!(!event.is_livemode());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2Event {
    /// Unique identifier for the v2 event (`v2_evt_...`).
    pub id: String,

    /// Object type descriptor, typically `"v2.core.event"`.
    pub object: String,

    /// Event type identifier (e.g., `"v1.billing.meter.error_report_triggered"`).
    #[serde(rename = "type")]
    pub event_type: String,

    /// Time at which the event occurred (UNIX epoch seconds).
    pub created: i64,

    /// Whether this event was generated in live mode (`true`) or test mode (`false`).
    pub livemode: bool,

    /// Reference to the underlying Stripe resource associated with this event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_object: Option<V2RelatedObject>,

    /// Optional event context or operational reason provided by Stripe API v2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,

    /// Reason why this event was generated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<V2EventReason>,

    /// Optional payload data containing live state snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl V2Event {
    /// Returns the unique event identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the event type string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    /// Returns the creation timestamp in UNIX epoch seconds.
    #[must_use]
    pub fn created(&self) -> i64 {
        self.created
    }

    /// Returns true if this event is from live mode.
    #[must_use]
    pub fn is_livemode(&self) -> bool {
        self.livemode
    }

    /// Returns the related object identifier, if present.
    #[must_use]
    pub fn related_object_id(&self) -> Option<&str> {
        self.related_object.as_ref().map(|ro| ro.id.as_str())
    }

    /// Returns the related object type descriptor, if present.
    #[must_use]
    pub fn related_object_type(&self) -> Option<&str> {
        self.related_object
            .as_ref()
            .map(|ro| ro.object_type.as_str())
    }

    /// Returns the related object hydration URL, if present.
    #[must_use]
    pub fn related_object_url(&self) -> Option<&str> {
        self.related_object
            .as_ref()
            .and_then(|ro| ro.url.as_deref())
    }
}

/// Operational reason why a [`V2Event`] was triggered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2EventReason {
    /// Type of reason (e.g., `"request"`).
    #[serde(rename = "type")]
    pub reason_type: String,

    /// Request context when the event was triggered by an API request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<V2EventRequestReason>,
}

/// API request reason metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2EventRequestReason {
    /// Request ID (`req_...`).
    pub id: String,

    /// Idempotency key attached to the triggering request, if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

/// Parameters for querying the V2 Core Events list endpoint (`GET /v2/core/events`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListV2EventsParams {
    /// Limit on number of events to return (1..100).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Filter by exact event type string.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,

    /// Pagination token or URL cursor for next page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
}

/// Status of an [`EventDestination`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventDestinationStatus {
    /// Destination is actively receiving event notifications.
    Enabled,
    /// Destination is paused or disabled.
    Disabled,
}

/// Delivery mechanism type for an [`EventDestination`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventDestinationType {
    /// Amazon EventBridge partner event source.
    AmazonEventbridge,
    /// Azure Event Grid partner topic.
    AzureEventGrid,
    /// Standard HTTPS webhook endpoint.
    WebhookEndpoint,
    /// Unknown destination type fallback.
    #[serde(other)]
    Unknown,
}

/// AWS Amazon EventBridge delivery details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmazonEventBridgeDetails {
    /// 12-digit AWS account ID.
    pub aws_account_id: String,

    /// AWS region (e.g., `"us-east-1"`).
    pub aws_region: String,

    /// Partner event source ARN generated by Stripe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partner_event_source_arn: Option<String>,
}

/// Microsoft Azure Event Grid delivery details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AzureEventGridDetails {
    /// Azure tenant UUID.
    pub azure_tenant_id: String,

    /// Azure subscription UUID.
    pub azure_subscription_id: String,

    /// Azure resource group name.
    pub resource_group: String,

    /// Azure topic name.
    pub topic_name: String,
}

/// Webhook endpoint delivery details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookEndpointDetails {
    /// HTTPS URL destination.
    pub url: String,
}

/// Stripe API v2 Event Destination configuration entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventDestination {
    /// Unique destination identifier (`ed_...`).
    pub id: String,

    /// Object type name (`"v2.core.event_destination"`).
    pub object: String,

    /// Current operational status.
    pub status: EventDestinationStatus,

    /// Delivery transport mechanism.
    #[serde(rename = "type")]
    pub destination_type: EventDestinationType,

    /// List of event type patterns subscribed to (e.g. `["v2.*"]`).
    pub events: Vec<String>,

    /// Details for Amazon EventBridge destination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amazon_eventbridge: Option<AmazonEventBridgeDetails>,

    /// Details for Azure Event Grid destination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub azure_event_grid: Option<AzureEventGridDetails>,

    /// Details for webhook endpoint destination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook_endpoint: Option<WebhookEndpointDetails>,

    /// Optional human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether destination operates in livemode.
    pub livemode: bool,

    /// Creation timestamp (UNIX epoch seconds).
    pub created: i64,

    /// Last update timestamp (UNIX epoch seconds).
    pub updated: i64,
}

impl EventDestination {
    /// Returns true if the destination is currently enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.status == EventDestinationStatus::Enabled
    }
}

/// Parameters for creating an [`EventDestination`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateEventDestinationParams {
    /// Transport type string (`"amazon_eventbridge"`, `"azure_event_grid"`, or `"webhook_endpoint"`).
    #[serde(rename = "type")]
    pub destination_type: String,

    /// Event patterns to route to this destination.
    pub events: Vec<String>,

    /// Amazon EventBridge parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amazon_eventbridge: Option<AmazonEventBridgeDetails>,

    /// Azure Event Grid parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub azure_event_grid: Option<AzureEventGridDetails>,

    /// Webhook endpoint parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook_endpoint: Option<WebhookEndpointDetails>,

    /// Optional description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Parameters for updating an existing [`EventDestination`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateEventDestinationParams {
    /// Updated list of event patterns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<String>>,

    /// Updated description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Operational status of a [`V2Account`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum V2AccountStatus {
    /// Account is fully active.
    Active,
    /// Account has restricted functionality.
    Restricted,
    /// Account onboarding or verification is pending.
    Pending,
}

/// Feature configuration flags for a [`V2Account`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2AccountConfiguration {
    /// Enabled features for the account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
}

/// Stripe API v2 Core Account entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2Account {
    /// Unique account identifier (`acct_...` or `v2_acct_...`).
    pub id: String,

    /// Object type descriptor (`"v2.core.account"`).
    pub object: String,

    /// Livemode indicator.
    pub livemode: bool,

    /// Creation timestamp (UNIX epoch seconds).
    pub created: i64,

    /// Display business or individual name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Primary contact email.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_email: Option<String>,

    /// Country code (ISO 3166-1 alpha-2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Default currency code (ISO 4217 lowercase).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_currency: Option<String>,

    /// Current operational status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<V2AccountStatus>,

    /// Account configuration settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration: Option<V2AccountConfiguration>,
}

/// Short-lived access token for a [`V2Account`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2AccountToken {
    /// Token identifier.
    pub id: String,

    /// Object type name (`"v2.core.account_token"`).
    pub object: String,

    /// Target account ID bound to this token.
    pub account: String,

    /// Token creation timestamp.
    pub created: i64,

    /// Token expiration timestamp.
    pub expires_at: i64,
}

/// Parameters for creating a [`V2Account`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateV2AccountParams {
    /// Business display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Primary contact email.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_email: Option<String>,

    /// Two-letter country code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Default 3-letter currency code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_currency: Option<String>,
}

/// Parameters for updating a [`V2Account`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateV2AccountParams {
    /// Updated display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Updated contact email.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_email: Option<String>,
}

/// Parameters for querying the V2 Accounts list endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListV2AccountsParams {
    /// Page size limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Pagination cursor or token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
}

/// Parameters for creating a [`V2AccountToken`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateV2AccountTokenParams {
    /// Purpose string for the requested token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

/// Stripe API v2 URL-driven pagination envelope.
///
/// Contains page items in `data` and navigation links in `next_page_url` / `previous_page_url`.
///
/// # Example
/// ```
/// use some_stripe_types::V2List;
///
/// let list = V2List::new(vec!["item1".to_string(), "item2".to_string()]);
/// assert_eq!(list.len(), 2);
/// assert!(!list.has_more());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2List<T> {
    /// Page items.
    pub data: Vec<T>,

    /// Absolute URL to fetch the next page of results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page_url: Option<String>,

    /// Absolute URL to fetch the previous page of results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_page_url: Option<String>,
}

impl<T> V2List<T> {
    /// Creates a new `V2List` with the given data and no navigation links.
    #[must_use]
    pub fn new(data: Vec<T>) -> Self {
        Self {
            data,
            next_page_url: None,
            previous_page_url: None,
        }
    }

    /// Returns `true` if a next page URL is available for pagination.
    #[must_use]
    pub fn has_more(&self) -> bool {
        self.next_page_url.is_some()
    }

    /// Returns number of items on this page.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if this page contains no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}
