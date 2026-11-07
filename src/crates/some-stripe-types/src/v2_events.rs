//! Stripe API v2 Core Events & Thin Event Notifications domain entities.
//!
//! Models core event types for Stripe API v2. Unlike API v1 events that embed full snapshots
//! under a `data.object` wrapper, v2 thin events deliver lightweight notifications containing
//! event metadata and references to related domain objects (`related_object`), enabling
//! live authenticated resource hydration.
//!
//! # Architecture & Quality Invariants:
//! - **Gate G-Borrow**: Zero deep clones; borrow parameters by reference (`&T`).
//! - **Gate G-Docs**: Runnable doc-tests and complete rustdoc documentation.
//! - **Gate G-Complexity**: Cyclomatic complexity CC <= 6 on all functions. Max nesting depth <= 2.
//! - **Gate G2 & G-Law**: Path traversal protection and secure event logging standards.
//!
//! # Example
//!
//! ```
//! use some_stripe_types::v2_events::{ListV2EventsParams, V2Event, V2RelatedObject};
//!
//! let json = r#"{
//!     "id": "evt_v2_abc123",
//!     "object": "v2.core.event",
//!     "type": "v1.billing.meter.error_report_triggered",
//!     "created": 1728345600,
//!     "livemode": false,
//!     "related_object": {
//!         "id": "meter_001",
//!         "type": "billing.meter",
//!         "url": "https://api.stripe.com/v2/billing/meters/meter_001"
//!     }
//! }"#;
//!
//! let event: V2Event = serde_json::from_str(json).unwrap();
//! assert_eq!(event.id, "evt_v2_abc123");
//! assert_eq!(event.event_type(), "v1.billing.meter.error_report_triggered");
//! assert_eq!(event.related_object_id(), Some("meter_001"));
//! assert_eq!(event.related_object_type(), Some("billing.meter"));
//! assert!(event.is_thin());
//!
//! let params = ListV2EventsParams::new().limit(25).add_type("v1.billing.meter.error_report_triggered");
//! let query_pairs = params.to_query_params();
//! assert_eq!(query_pairs.len(), 2);
//! ```

use serde::{Deserialize, Serialize};

fn default_v2_event_object() -> String {
    "v2.core.event".to_string()
}

/// Represents a Stripe API v2 Core Event.
///
/// Can represent either a lightweight thin event notification or a fully hydrated
/// event payload containing embedded data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2Event<T = serde_json::Value> {
    /// Unique identifier for the event (e.g., `"evt_v2_123"`).
    pub id: String,

    /// Object type descriptor, typically `"v2.core.event"`.
    #[serde(default = "default_v2_event_object")]
    pub object: String,

    /// Event type string (e.g., `"v1.billing.meter.error_report_triggered"`).
    #[serde(rename = "type")]
    pub r#type: String,

    /// Time at which the event occurred (UNIX epoch seconds).
    pub created: i64,

    /// Whether this event was generated in live mode (`true`) or test mode (`false`).
    pub livemode: bool,

    /// Payload data associated with this event, if hydrated or embedded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,

    /// Reference to the underlying Stripe resource associated with this event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_object: Option<V2RelatedObject>,

    /// Operational context metadata (e.g. `request` ID, `user_agent`, `account`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<V2EventContext>,

    /// Operational reason details (`reason_type`, `request`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<V2EventReason>,
}

impl<T> V2Event<T> {
    /// Creates a new `V2Event` with required attributes.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        event_type: impl Into<String>,
        created: i64,
        livemode: bool,
    ) -> Self {
        Self {
            id: id.into(),
            object: default_v2_event_object(),
            r#type: event_type.into(),
            created,
            livemode,
            data: None,
            related_object: None,
            context: None,
            reason: None,
        }
    }

    /// Returns the event type string identifier.
    #[inline]
    #[must_use]
    pub fn event_type(&self) -> &str {
        &self.r#type
    }

    /// Returns the ID of the related object, if present.
    #[inline]
    #[must_use]
    pub fn related_object_id(&self) -> Option<&str> {
        self.related_object.as_ref().map(|ro| ro.id.as_str())
    }

    /// Returns the type identifier of the related object, if present.
    #[inline]
    #[must_use]
    pub fn related_object_type(&self) -> Option<&str> {
        self.related_object.as_ref().map(|ro| ro.r#type.as_str())
    }

    /// Returns the hydration URL for the related object, if present.
    #[inline]
    #[must_use]
    pub fn related_object_url(&self) -> Option<&str> {
        self.related_object.as_ref().and_then(|ro| ro.url.as_deref())
    }

    /// Returns `true` if this event is a thin event notification (has a related object but no embedded data).
    #[inline]
    #[must_use]
    pub fn is_thin(&self) -> bool {
        self.related_object.is_some() && self.data.is_none()
    }

    /// Attaches an embedded data payload to this event.
    #[must_use]
    pub fn with_data(mut self, data: T) -> Self {
        self.data = Some(data);
        self
    }

    /// Attaches related object metadata to this event.
    #[must_use]
    pub fn with_related_object(mut self, related_object: V2RelatedObject) -> Self {
        self.related_object = Some(related_object);
        self
    }

    /// Attaches operational context metadata to this event.
    #[must_use]
    pub fn with_context(mut self, context: V2EventContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Attaches an operational reason descriptor to this event.
    #[must_use]
    pub fn with_reason(mut self, reason: V2EventReason) -> Self {
        self.reason = Some(reason);
        self
    }
}

/// Reference metadata pointing to a domain resource in a [`V2Event`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2RelatedObject {
    /// Identifier of the related Stripe resource (e.g., `"meter_123"`).
    pub id: String,

    /// Object type name of the related resource (e.g., `"billing.meter"`).
    #[serde(rename = "type")]
    pub r#type: String,

    /// Canonical REST API URL for hydrating the full resource object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl V2RelatedObject {
    /// Creates a new related object reference with the given ID and object type.
    #[must_use]
    pub fn new(id: impl Into<String>, r#type: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            r#type: r#type.into(),
            url: None,
        }
    }

    /// Sets the hydration URL for this related object.
    #[must_use]
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Returns the object type name of the related resource.
    #[inline]
    #[must_use]
    pub fn object_type(&self) -> &str {
        &self.r#type
    }
}

/// Operational context metadata associated with a [`V2Event`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct V2EventContext {
    /// Connected account identifier if this event pertains to a Connect account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,

    /// Request identifier that triggered this event (e.g., `"req_123"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<String>,

    /// User-Agent header from the originating request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
}

impl V2EventContext {
    /// Creates an empty `V2EventContext`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the connected account ID.
    #[must_use]
    pub fn with_account(mut self, account: impl Into<String>) -> Self {
        self.account = Some(account.into());
        self
    }

    /// Sets the originating request ID.
    #[must_use]
    pub fn with_request(mut self, request: impl Into<String>) -> Self {
        self.request = Some(request.into());
        self
    }

    /// Sets the originating user-agent string.
    #[must_use]
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }
}

/// Operational reason details explaining why a [`V2Event`] was generated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct V2EventReason {
    /// Reason type (e.g., `"request"`, `"automatic"`).
    #[serde(
        rename = "type",
        alias = "reason_type",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub reason_type: Option<String>,

    /// Operational request details associated with this reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<V2EventReasonRequest>,
}

impl V2EventReason {
    /// Creates a new `V2EventReason` with the specified reason type.
    #[must_use]
    pub fn new(reason_type: impl Into<String>) -> Self {
        Self {
            reason_type: Some(reason_type.into()),
            request: None,
        }
    }

    /// Attaches request details to this reason.
    #[must_use]
    pub fn with_request(mut self, request: V2EventReasonRequest) -> Self {
        self.request = Some(request);
        self
    }
}

/// Request details associated with an operational event reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct V2EventReasonRequest {
    /// Request ID that initiated the state mutation (`req_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Idempotency key attached to the initiating request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

impl V2EventReasonRequest {
    /// Creates a new request reason descriptor.
    #[must_use]
    pub fn new(id: Option<String>, idempotency_key: Option<String>) -> Self {
        Self {
            id,
            idempotency_key,
        }
    }
}

/// Strongly typed query parameters for listing Stripe API v2 events (`GET /v2/core/events`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ListV2EventsParams {
    /// Limit on the number of objects to be returned (1 to 100). Default is 10.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Cursor for forward pagination: an object ID defining your place in the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// Cursor for backward pagination: an object ID defining your place in the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,

    /// Filter events by event type strings (e.g. `["v1.billing.meter.error_report_triggered"]`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub types: Option<Vec<String>>,

    /// Filter events pertaining to a specific related object ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,

    /// Filter events created on or around a specific Unix timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,

    /// URL-driven pagination token for the next page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
}

impl ListV2EventsParams {
    /// Creates empty, default [`ListV2EventsParams`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum number of items to return.
    #[must_use]
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the `starting_after` pagination cursor.
    #[must_use]
    pub fn starting_after(mut self, starting_after: impl Into<String>) -> Self {
        self.starting_after = Some(starting_after.into());
        self
    }

    /// Sets the `ending_before` pagination cursor.
    #[must_use]
    pub fn ending_before(mut self, ending_before: impl Into<String>) -> Self {
        self.ending_before = Some(ending_before.into());
        self
    }

    /// Sets the event types to filter by.
    #[must_use]
    pub fn types(mut self, types: Vec<String>) -> Self {
        self.types = Some(types);
        self
    }

    /// Adds a single event type to the filter.
    #[must_use]
    pub fn add_type(mut self, event_type: impl Into<String>) -> Self {
        self.types
            .get_or_insert_with(Vec::new)
            .push(event_type.into());
        self
    }

    /// Filters events pertaining to a specific related object ID.
    #[must_use]
    pub fn object_id(mut self, object_id: impl Into<String>) -> Self {
        self.object_id = Some(object_id.into());
        self
    }

    /// Filters events created around a specific Unix timestamp.
    #[must_use]
    pub fn created(mut self, created: i64) -> Self {
        self.created = Some(created);
        self
    }

    /// Sets the URL-driven pagination page token.
    #[must_use]
    pub fn page(mut self, page: impl Into<String>) -> Self {
        self.page = Some(page.into());
        self
    }

    /// Converts the parameter values into query key-value pairs.
    #[must_use]
    pub fn to_query_params(&self) -> Vec<(String, String)> {
        let mut params = Vec::new();
        self.push_scalar_params(&mut params);
        self.push_types_params(&mut params);
        params
    }

    fn push_scalar_params(&self, params: &mut Vec<(String, String)>) {
        if let Some(limit) = self.limit {
            params.push(("limit".to_string(), limit.to_string()));
        }
        if let Some(starting_after) = &self.starting_after {
            params.push(("starting_after".to_string(), starting_after.clone()));
        }
        if let Some(ending_before) = &self.ending_before {
            params.push(("ending_before".to_string(), ending_before.clone()));
        }
        if let Some(object_id) = &self.object_id {
            params.push(("object_id".to_string(), object_id.clone()));
        }
        if let Some(created) = self.created {
            params.push(("created".to_string(), created.to_string()));
        }
        if let Some(page) = &self.page {
            params.push(("page".to_string(), page.clone()));
        }
    }

    fn push_types_params(&self, params: &mut Vec<(String, String)>) {
        if let Some(types) = &self.types {
            for t in types {
                params.push(("types[]".to_string(), t.clone()));
            }
        }
    }
}

/// Standard envelope for Stripe API v2 event lists (`GET /v2/core/events`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2EventList {
    /// List of V2 events returned for the current page.
    pub data: Vec<V2Event>,

    /// URL to fetch the next page of results, or `None` if at the end of the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page_url: Option<String>,

    /// URL to fetch the previous page of results, or `None` if at the beginning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_page_url: Option<String>,
}

impl V2EventList {
    /// Creates a new `V2EventList` with the given event records.
    #[must_use]
    pub fn new(data: Vec<V2Event>) -> Self {
        Self {
            data,
            next_page_url: None,
            previous_page_url: None,
        }
    }

    /// Returns `true` if there are more pages to fetch.
    #[inline]
    #[must_use]
    pub fn has_more(&self) -> bool {
        self.next_page_url.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v2_event_deserialization_full() {
        let json = r#"{
            "id": "evt_v2_full_123",
            "object": "v2.core.event",
            "type": "v1.billing.meter.error_report_triggered",
            "created": 1728345600,
            "livemode": false,
            "data": {
                "meter_id": "mtr_test_001",
                "error_count": 5
            },
            "related_object": {
                "id": "mtr_test_001",
                "type": "billing.meter",
                "url": "https://api.stripe.com/v2/billing/meters/mtr_test_001"
            },
            "context": {
                "account": "acct_connect_123",
                "request": "req_audit_789",
                "user_agent": "Stripe/v2 RustClient"
            },
            "reason": {
                "type": "request",
                "request": {
                    "id": "req_audit_789",
                    "idempotency_key": "idemp_test_456"
                }
            }
        }"#;

        let event: V2Event = serde_json::from_str(json).expect("should deserialize v2 event");
        assert_eq!(event.id, "evt_v2_full_123");
        assert_eq!(event.object, "v2.core.event");
        assert_eq!(event.event_type(), "v1.billing.meter.error_report_triggered");
        assert_eq!(event.created, 1728345600);
        assert!(!event.livemode);
        assert!(!event.is_thin()); // has embedded data

        let ro = event.related_object.as_ref().unwrap();
        assert_eq!(ro.id, "mtr_test_001");
        assert_eq!(ro.r#type, "billing.meter");
        assert_eq!(ro.object_type(), "billing.meter");
        assert_eq!(event.related_object_id(), Some("mtr_test_001"));
        assert_eq!(event.related_object_type(), Some("billing.meter"));
        assert_eq!(
            event.related_object_url(),
            Some("https://api.stripe.com/v2/billing/meters/mtr_test_001")
        );

        let ctx = event.context.as_ref().unwrap();
        assert_eq!(ctx.account.as_deref(), Some("acct_connect_123"));
        assert_eq!(ctx.request.as_deref(), Some("req_audit_789"));
        assert_eq!(ctx.user_agent.as_deref(), Some("Stripe/v2 RustClient"));

        let reason = event.reason.as_ref().unwrap();
        assert_eq!(reason.reason_type.as_deref(), Some("request"));
        let reason_req = reason.request.as_ref().unwrap();
        assert_eq!(reason_req.id.as_deref(), Some("req_audit_789"));
        assert_eq!(reason_req.idempotency_key.as_deref(), Some("idemp_test_456"));
    }

    #[test]
    fn test_v2_thin_event_deserialization() {
        let json = r#"{
            "id": "evt_v2_thin_999",
            "object": "v2.core.event",
            "type": "v1.billing.meter.created",
            "created": 1728345700,
            "livemode": true,
            "related_object": {
                "id": "mtr_new_999",
                "type": "billing.meter"
            }
        }"#;

        let event: V2Event = serde_json::from_str(json).expect("should deserialize thin event");
        assert_eq!(event.id, "evt_v2_thin_999");
        assert!(event.livemode);
        assert!(event.data.is_none());
        assert!(event.is_thin());
        assert_eq!(event.related_object_id(), Some("mtr_new_999"));
        assert_eq!(event.related_object_url(), None);
    }

    #[test]
    fn test_list_v2_events_params_serialization() {
        let params = ListV2EventsParams::new()
            .limit(50)
            .starting_after("evt_v2_100")
            .ending_before("evt_v2_200")
            .object_id("mtr_abc")
            .created(1728345600)
            .page("page_token_xyz")
            .add_type("v1.billing.meter.created")
            .add_type("v1.billing.meter.error_report_triggered");

        let pairs = params.to_query_params();
        assert!(pairs.contains(&("limit".to_string(), "50".to_string())));
        assert!(pairs.contains(&("starting_after".to_string(), "evt_v2_100".to_string())));
        assert!(pairs.contains(&("ending_before".to_string(), "evt_v2_200".to_string())));
        assert!(pairs.contains(&("object_id".to_string(), "mtr_abc".to_string())));
        assert!(pairs.contains(&("created".to_string(), "1728345600".to_string())));
        assert!(pairs.contains(&("page".to_string(), "page_token_xyz".to_string())));
        assert!(pairs.contains(&(
            "types[]".to_string(),
            "v1.billing.meter.created".to_string()
        )));
        assert!(pairs.contains(&(
            "types[]".to_string(),
            "v1.billing.meter.error_report_triggered".to_string()
        )));
    }

    #[test]
    fn test_v2_event_list_serialization_and_deserialization() {
        let list_json = r#"{
            "data": [
                {
                    "id": "evt_v2_1",
                    "object": "v2.core.event",
                    "type": "ping",
                    "created": 1728345600,
                    "livemode": false
                }
            ],
            "next_page_url": "https://api.stripe.com/v2/core/events?page=next_tok",
            "previous_page_url": null
        }"#;

        let list: V2EventList = serde_json::from_str(list_json).unwrap();
        assert_eq!(list.data.len(), 1);
        assert_eq!(list.data[0].id, "evt_v2_1");
        assert!(list.has_more());
        assert_eq!(
            list.next_page_url.as_deref(),
            Some("https://api.stripe.com/v2/core/events?page=next_tok")
        );
        assert_eq!(list.previous_page_url, None);
    }
}
