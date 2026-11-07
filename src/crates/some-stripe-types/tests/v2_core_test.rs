//! Unit tests for Stripe API v2 Core domain models and serialization contracts.

use some_stripe_types::{
    AmazonEventBridgeDetails, AzureEventGridDetails, CreateEventDestinationParams,
    CreateV2AccountParams, CreateV2AccountTokenParams, EventDestination, EventDestinationStatus,
    EventDestinationType, EventObject, ListV2AccountsParams, ListV2EventsParams,
    UpdateEventDestinationParams, UpdateV2AccountParams, V2Account, V2AccountConfiguration,
    V2AccountStatus, V2AccountToken, V2Event, V2EventReason, V2EventRequestReason, V2List,
    V2RelatedObject, V2ThinEvent, WebhookEndpointDetails,
};

#[test]
fn test_v2_event_serialization_and_deserialization() {
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
        },
        "context": "ctx_operational_123",
        "reason": {
            "type": "request",
            "request": {
                "id": "req_xyz_999",
                "idempotency_key": "idem_audit_001"
            }
        },
        "data": {
            "error_count": 5,
            "status": "investigating"
        }
    }"#;

    let event: V2Event = serde_json::from_str(json).expect("Failed to deserialize V2Event");
    assert_eq!(event.id(), "v2_evt_test_001");
    assert_eq!(event.object, "v2.core.event");
    assert_eq!(event.event_type(), "v1.billing.meter.error_report_triggered");
    assert_eq!(event.created(), 1728345600);
    assert!(!event.is_livemode());
    assert_eq!(event.related_object_id(), Some("meter_abc"));
    assert_eq!(event.related_object_type(), Some("billing.meter"));
    assert_eq!(
        event.related_object_url(),
        Some("https://api.stripe.com/v2/billing/meters/meter_abc")
    );
    assert_eq!(event.context.as_deref(), Some("ctx_operational_123"));

    let reason = event.reason.as_ref().expect("Reason must be present");
    assert_eq!(reason.reason_type, "request");
    let req = reason.request.as_ref().expect("Request reason present");
    assert_eq!(req.id, "req_xyz_999");
    assert_eq!(req.idempotency_key.as_deref(), Some("idem_audit_001"));

    // Verify roundtrip
    let serialized = serde_json::to_string(&event).expect("Serialize V2Event");
    let roundtrip: V2Event = serde_json::from_str(&serialized).expect("Deserialize roundtrip");
    assert_eq!(roundtrip, event);
}

#[test]
fn test_v2_thin_event_properties() {
    let thin = V2ThinEvent {
        id: "v2_evt_thin_123".into(),
        object: "v2.core.event".into(),
        event_type: "v2.billing.meter.error_report_triggered".into(),
        created: 1728345700,
        livemode: true,
        related_object: Some(V2RelatedObject {
            id: "meter_456".into(),
            object_type: "billing.meter".into(),
            url: Some("https://api.stripe.com/v2/billing/meters/meter_456".into()),
        }),
        context: None,
    };

    assert_eq!(thin.related_object_id(), Some("meter_456"));
    assert_eq!(thin.related_object_type(), Some("billing.meter"));
    assert_eq!(
        thin.related_object_url(),
        Some("https://api.stripe.com/v2/billing/meters/meter_456")
    );
}

#[test]
fn test_event_object_discriminator_for_v2_event() {
    let json = serde_json::json!({
        "id": "v2_evt_discriminated",
        "object": "v2.core.event",
        "type": "v1.billing.meter.error_report_triggered",
        "created": 1728345800,
        "livemode": false
    });

    let obj: EventObject = serde_json::from_value(json).expect("Deserialize into EventObject");
    match obj {
        EventObject::V2Event(ev) => {
            assert_eq!(ev.id(), "v2_evt_discriminated");
            assert_eq!(ev.event_type(), "v1.billing.meter.error_report_triggered");
        }
        other => panic!("Expected EventObject::V2Event, got {other:?}"),
    }
}

#[test]
fn test_event_destination_serialization() {
    let destination = EventDestination {
        id: "ed_test_123".into(),
        object: "v2.core.event_destination".into(),
        status: EventDestinationStatus::Enabled,
        destination_type: EventDestinationType::AmazonEventbridge,
        events: vec!["v2.*".into()],
        amazon_eventbridge: Some(AmazonEventBridgeDetails {
            aws_account_id: "123456789012".into(),
            aws_region: "us-east-1".into(),
            partner_event_source_arn: Some("arn:aws:events:us-east-1:123456789012:source".into()),
        }),
        azure_event_grid: None,
        webhook_endpoint: None,
        description: Some("Production AWS EventBridge router".into()),
        livemode: true,
        created: 1728345600,
        updated: 1728345700,
    };

    assert!(destination.is_enabled());
    let serialized = serde_json::to_string(&destination).expect("Serialize EventDestination");
    let parsed: EventDestination = serde_json::from_str(&serialized).expect("Deserialize");
    assert_eq!(parsed.id, "ed_test_123");
    assert_eq!(
        parsed.destination_type,
        EventDestinationType::AmazonEventbridge
    );
    assert_eq!(
        parsed.amazon_eventbridge.unwrap().aws_account_id,
        "123456789012"
    );
}

#[test]
fn test_v2_account_and_token() {
    let acct = V2Account {
        id: "v2_acct_abc".into(),
        object: "v2.core.account".into(),
        livemode: false,
        created: 1728345600,
        display_name: Some("Acme SaaS".into()),
        contact_email: Some("billing@acme.com".into()),
        country: Some("US".into()),
        default_currency: Some("usd".into()),
        status: Some(V2AccountStatus::Active),
        configuration: Some(V2AccountConfiguration {
            features: Some(vec!["billing".into()]),
        }),
    };

    let serialized = serde_json::to_string(&acct).expect("Serialize V2Account");
    let parsed: V2Account = serde_json::from_str(&serialized).expect("Deserialize V2Account");
    assert_eq!(parsed.id, "v2_acct_abc");
    assert_eq!(parsed.status, Some(V2AccountStatus::Active));

    let tok = V2AccountToken {
        id: "tok_v2_999".into(),
        object: "v2.core.account_token".into(),
        account: "v2_acct_abc".into(),
        created: 1728345600,
        expires_at: 1728349200,
    };
    let tok_json = serde_json::to_string(&tok).expect("Serialize token");
    let parsed_tok: V2AccountToken = serde_json::from_str(&tok_json).expect("Deserialize token");
    assert_eq!(parsed_tok.account, "v2_acct_abc");
}

#[test]
fn test_v2_list_pagination() {
    let mut list = V2List::new(vec!["evt_1".to_string(), "evt_2".to_string()]);
    assert_eq!(list.len(), 2);
    assert!(!list.is_empty());
    assert!(!list.has_more());

    list.next_page_url =
        Some("https://api.stripe.com/v2/core/events?page=page_token_abc".to_string());
    assert!(list.has_more());
}
