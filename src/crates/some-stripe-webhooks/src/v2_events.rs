//! Stripe API v2 Thin Event Envelope and Identification Predicates.
//!
//! Models Stripe API v2 thin webhook event envelopes modeled after `stripe-node`.
//! V2 thin events deliver lightweight notifications containing event metadata and
//! references to related domain objects, requiring consumers to hydrate the full resource.

use serde::Deserialize;
pub use some_stripe_types::{V2RelatedObject, V2ThinEvent};

/// Lightweight probe struct to detect Stripe API v2 thin events without full DOM allocation.
///
/// Uses `serde::de::IgnoredAny` for payload subtrees (`related_object` and `data`) to eliminate
/// heap allocations on large incoming webhooks (Gate G-Borrow zero-clone discipline).
#[derive(Debug, Deserialize)]
struct ThinEventProbe<'a> {
    #[serde(borrow)]
    object: Option<&'a str>,
    related_object: Option<serde::de::IgnoredAny>,
    data: Option<serde::de::IgnoredAny>,
}

/// Inspects root JSON properties to determine whether a webhook payload represents a Stripe API v2 thin event.
///
/// Evaluates payload format using a lightweight zero-copy probe ([`ThinEventProbe`]) rather than allocating
/// a full DOM representation with `serde_json::Value`.
///
/// Returns `true` if:
/// - The root JSON is an object,
/// - The root `object` starts with `"v2."` (e.g. `"v2.core.event"`), OR contains a `"related_object"` key,
/// - And the root object does NOT contain a v1 `"data"` object.
///
/// Returns `false` for v1 event payloads, non-object JSON, or invalid JSON byte slices.
///
/// # Example
///
/// ```
/// use some_stripe_webhooks::v2_events::is_thin_event;
///
/// let v2_payload = br#"{"id":"v2_evt_1","object":"v2.core.event","type":"some.event","created":1728345600,"livemode":false}"#;
/// assert!(is_thin_event(v2_payload));
///
/// let v1_payload = br#"{"id":"evt_1","object":"event","type":"payment_intent.succeeded","data":{"object":{}}}"#;
/// assert!(!is_thin_event(v1_payload));
/// ```
pub fn is_thin_event(payload: &[u8]) -> bool {
    let Ok(probe) = serde_json::from_slice::<ThinEventProbe<'_>>(payload) else {
        return false;
    };

    let has_related_object = probe.related_object.is_some();
    let is_v2_object = probe.object.map(|s| s.starts_with("v2.")).unwrap_or(false);

    (has_related_object || is_v2_object) && probe.data.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v2_thin_event_deserialization_with_related_object() {
        let json = r#"{
            "id": "v2_evt_test_001",
            "object": "v2.core.event",
            "type": "v1.billing.meter.error_report_triggered",
            "created": 1728345600,
            "livemode": false,
            "related_object": {
                "id": "meter_abc",
                "type": "billing.meter",
                "url": "https://api.stripe.com/v2/billing/meters/meter_abc"
            }
        }"#;

        let event: V2ThinEvent =
            serde_json::from_str(json).expect("failed to deserialize v2 thin event");
        assert_eq!(event.id, "v2_evt_test_001");
        assert_eq!(event.object, "v2.core.event");
        assert_eq!(event.event_type, "v1.billing.meter.error_report_triggered");
        assert_eq!(event.created, 1728345600);
        assert!(!event.livemode);

        let related = event
            .related_object
            .as_ref()
            .expect("related_object must be present");
        assert_eq!(related.id, "meter_abc");
        assert_eq!(related.object_type, "billing.meter");
        assert_eq!(
            related.url.as_deref(),
            Some("https://api.stripe.com/v2/billing/meters/meter_abc")
        );

        assert_eq!(event.related_object_id(), Some("meter_abc"));
        assert_eq!(event.related_object_type(), Some("billing.meter"));
        assert_eq!(
            event.related_object_url(),
            Some("https://api.stripe.com/v2/billing/meters/meter_abc")
        );
    }

    #[test]
    fn test_v2_thin_event_without_related_object() {
        let json = r#"{
            "id": "v2_evt_test_002",
            "object": "v2.core.event",
            "type": "ping.event",
            "created": 1728345700,
            "livemode": true
        }"#;

        let event: V2ThinEvent = serde_json::from_str(json).expect("failed to deserialize");
        assert_eq!(event.id, "v2_evt_test_002");
        assert!(event.livemode);
        assert!(event.related_object.is_none());
        assert_eq!(event.related_object_id(), None);
        assert_eq!(event.related_object_type(), None);
        assert_eq!(event.related_object_url(), None);
    }

    #[test]
    fn test_is_thin_event_predicate() {
        // V2 event with related object
        let v2_with_ro = br#"{
            "id": "v2_evt_1",
            "object": "v2.core.event",
            "type": "meter.created",
            "created": 1728345600,
            "livemode": false,
            "related_object": {"id": "m_1", "type": "meter"}
        }"#;
        assert!(is_thin_event(v2_with_ro));

        // V2 event without related object but with v2. object type
        let v2_no_ro = br#"{
            "id": "v2_evt_2",
            "object": "v2.core.event",
            "type": "meter.created",
            "created": 1728345600,
            "livemode": false
        }"#;
        assert!(is_thin_event(v2_no_ro));

        // V1 event with data.object
        let v1_event = br#"{
            "id": "evt_1",
            "object": "event",
            "created": 1728345600,
            "type": "checkout.session.completed",
            "data": {
                "object": {
                    "id": "cs_123"
                }
            }
        }"#;
        assert!(!is_thin_event(v1_event));

        // Malformed JSON or non-object payloads
        assert!(!is_thin_event(b"not json"));
        assert!(!is_thin_event(b""));
        assert!(!is_thin_event(b"[]"));
    }
}
