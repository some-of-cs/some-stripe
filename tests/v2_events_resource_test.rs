//! Integration and wire contract tests for Stripe API v2 Core Events & Thin Event Notifications (US-8.1.0 Unit 1).

use some_stripe::{
    ListV2EventsParams, StripeClient, StripeConfig, StripeError, V2Event, V2EventContext,
    V2EventList, V2EventReason, V2EventReasonRequest, V2EventsResource, V2RelatedObject,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_v2_event_thin_notification_model() {
    let json = serde_json::json!({
        "id": "evt_v2_thin_meter_001",
        "object": "v2.core.event",
        "type": "v1.billing.meter.error_report_triggered",
        "created": 1728345600,
        "livemode": false,
        "related_object": {
            "id": "mtr_001",
            "type": "billing.meter",
            "url": "https://api.stripe.com/v2/billing/meters/mtr_001"
        }
    });

    let event: V2Event = serde_json::from_value(json).expect("should deserialize v2 event");
    assert_eq!(event.id, "evt_v2_thin_meter_001");
    assert_eq!(event.object, "v2.core.event");
    assert_eq!(event.event_type(), "v1.billing.meter.error_report_triggered");
    assert_eq!(event.r#type, "v1.billing.meter.error_report_triggered");
    assert_eq!(event.created, 1728345600);
    assert!(!event.livemode);
    assert!(event.is_thin());
    assert!(event.data.is_none());

    assert_eq!(event.related_object_id(), Some("mtr_001"));
    assert_eq!(event.related_object_type(), Some("billing.meter"));
    assert_eq!(
        event.related_object_url(),
        Some("https://api.stripe.com/v2/billing/meters/mtr_001")
    );
}

#[test]
fn test_v2_event_hydrated_model_with_context_and_reason() {
    let json = serde_json::json!({
        "id": "evt_v2_hydrated_002",
        "object": "v2.core.event",
        "type": "v2.core.event_destination.created",
        "created": 1728345700,
        "livemode": true,
        "data": {
            "destination_id": "ed_123",
            "status": "enabled"
        },
        "related_object": {
            "id": "ed_123",
            "type": "event_destination",
            "url": "https://api.stripe.com/v2/core/event_destinations/ed_123"
        },
        "context": {
            "account": "acct_platform_001",
            "request": "req_audit_999",
            "user_agent": "some-stripe-sdk/0.1.0"
        },
        "reason": {
            "type": "request",
            "request": {
                "id": "req_audit_999",
                "idempotency_key": "idemp_evt_dest_create"
            }
        }
    });

    let event: V2Event = serde_json::from_value(json).expect("should deserialize hydrated event");
    assert_eq!(event.id, "evt_v2_hydrated_002");
    assert!(event.livemode);
    assert!(!event.is_thin()); // has embedded data

    let data = event.data.as_ref().expect("data must be present");
    assert_eq!(data["destination_id"], "ed_123");
    assert_eq!(data["status"], "enabled");

    let ctx = event.context.as_ref().expect("context must be present");
    assert_eq!(ctx.account.as_deref(), Some("acct_platform_001"));
    assert_eq!(ctx.request.as_deref(), Some("req_audit_999"));
    assert_eq!(ctx.user_agent.as_deref(), Some("some-stripe-sdk/0.1.0"));

    let reason = event.reason.as_ref().expect("reason must be present");
    assert_eq!(reason.reason_type.as_deref(), Some("request"));
    let req_reason = reason.request.as_ref().expect("request details in reason");
    assert_eq!(req_reason.id.as_deref(), Some("req_audit_999"));
    assert_eq!(
        req_reason.idempotency_key.as_deref(),
        Some("idemp_evt_dest_create")
    );
}

#[test]
fn test_v2_event_builders_and_setters() {
    let related = V2RelatedObject::new("mtr_999", "billing.meter")
        .with_url("https://api.stripe.com/v2/billing/meters/mtr_999");
    assert_eq!(related.id, "mtr_999");
    assert_eq!(related.object_type(), "billing.meter");
    assert_eq!(
        related.url.as_deref(),
        Some("https://api.stripe.com/v2/billing/meters/mtr_999")
    );

    let ctx = V2EventContext::new()
        .with_account("acct_999")
        .with_request("req_test_abc")
        .with_user_agent("test-agent/1.0");
    assert_eq!(ctx.account.as_deref(), Some("acct_999"));
    assert_eq!(ctx.request.as_deref(), Some("req_test_abc"));
    assert_eq!(ctx.user_agent.as_deref(), Some("test-agent/1.0"));

    let reason = V2EventReason::new("request").with_request(V2EventReasonRequest::new(
        Some("req_test_abc".into()),
        Some("idemp_123".into()),
    ));
    assert_eq!(reason.reason_type.as_deref(), Some("request"));
    assert_eq!(
        reason.request.as_ref().unwrap().id.as_deref(),
        Some("req_test_abc")
    );

    let event = V2Event::new("evt_v2_builder", "test.event", 1728345800, false)
        .with_related_object(related)
        .with_context(ctx)
        .with_reason(reason)
        .with_data(serde_json::json!({"custom_field": "custom_val"}));

    assert_eq!(event.id, "evt_v2_builder");
    assert_eq!(event.event_type(), "test.event");
    assert_eq!(event.related_object_id(), Some("mtr_999"));
    assert!(!event.is_thin());
}

#[test]
fn test_list_v2_events_params_builder_and_query_generation() {
    let params = ListV2EventsParams::new()
        .limit(20)
        .starting_after("evt_v2_start")
        .ending_before("evt_v2_end")
        .object_id("obj_related_123")
        .created(1728345600)
        .page("token_page_1")
        .types(vec![
            "v1.billing.meter.error_report_triggered".into(),
            "v2.core.event_destination.created".into(),
        ]);

    let pairs = params.to_query_params();
    assert!(pairs.contains(&("limit".to_string(), "20".to_string())));
    assert!(pairs.contains(&("starting_after".to_string(), "evt_v2_start".to_string())));
    assert!(pairs.contains(&("ending_before".to_string(), "evt_v2_end".to_string())));
    assert!(pairs.contains(&("object_id".to_string(), "obj_related_123".to_string())));
    assert!(pairs.contains(&("created".to_string(), "1728345600".to_string())));
    assert!(pairs.contains(&("page".to_string(), "token_page_1".to_string())));
    assert!(pairs.contains(&(
        "types[]".to_string(),
        "v1.billing.meter.error_report_triggered".to_string()
    )));
    assert!(pairs.contains(&(
        "types[]".to_string(),
        "v2.core.event_destination.created".to_string()
    )));
}

#[test]
fn test_v2_events_resource_accessor_and_client_mounting() {
    let client = StripeClient::new("sk_test_mock_v2_key");

    let events: V2EventsResource<'_> = client.v2_events();
    assert_eq!(events.client().config.api_base, "https://api.stripe.com");

    let scoped = events.with_account("acct_partner_777");
    assert_eq!(scoped.client().config.api_base, "https://api.stripe.com");
}

#[tokio::test]
async fn test_retrieve_v2_event_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        // Verify HTTP path and strict JSON headers (Gate G3)
        assert!(req_str.starts_with("GET /v2/core/events/evt_v2_wire_123 HTTP/1.1"));
        assert!(req_str.contains("content-type: application/json"));
        assert!(req_str.contains("accept: application/json"));

        let response_payload = serde_json::json!({
            "id": "evt_v2_wire_123",
            "object": "v2.core.event",
            "type": "v1.billing.meter.error_report_triggered",
            "created": 1728345600,
            "livemode": false,
            "related_object": {
                "id": "meter_wire_abc",
                "type": "billing.meter",
                "url": "https://api.stripe.com/v2/billing/meters/meter_wire_abc"
            }
        })
        .to_string();

        let http_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            response_payload.len(),
            response_payload
        );
        socket.write_all(http_response.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_wire_secret")
        .with_api_base(format!("http://127.0.0.1:{}", port));
    let client = StripeClient::with_config(config);

    let event = client
        .v2_events()
        .retrieve("evt_v2_wire_123")
        .await
        .expect("wire retrieval must succeed");

    assert_eq!(event.id, "evt_v2_wire_123");
    assert_eq!(event.event_type(), "v1.billing.meter.error_report_triggered");
    assert_eq!(event.related_object_id(), Some("meter_wire_abc"));
    assert_eq!(event.related_object_type(), Some("billing.meter"));
    assert!(event.is_thin());

    server.await.unwrap();
}

#[tokio::test]
async fn test_retrieve_v2_event_with_connect_account() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("GET /v2/core/events/evt_v2_tenant_456 HTTP/1.1"));
        assert!(req_str.contains("stripe-account: acct_connected_tenant_1"));

        let response_payload = serde_json::json!({
            "id": "evt_v2_tenant_456",
            "object": "v2.core.event",
            "type": "account.updated",
            "created": 1728345900,
            "livemode": false
        })
        .to_string();

        let http_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            response_payload.len(),
            response_payload
        );
        socket.write_all(http_response.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_wire_secret")
        .with_api_base(format!("http://127.0.0.1:{}", port));
    let client = StripeClient::with_config(config);

    let event = client
        .v2_events()
        .with_account("acct_connected_tenant_1")
        .retrieve("evt_v2_tenant_456")
        .await
        .expect("tenant retrieval must succeed");

    assert_eq!(event.id, "evt_v2_tenant_456");
    server.await.unwrap();
}

#[tokio::test]
async fn test_list_v2_events_wire_and_typed() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("GET /v2/core/events?"));
        assert!(req_str.contains("limit=10"));
        assert!(req_str.contains("types%5B%5D=v1.billing.meter.error_report_triggered"));

        let response_payload = serde_json::json!({
            "data": [
                {
                    "id": "evt_v2_list_1",
                    "object": "v2.core.event",
                    "type": "v1.billing.meter.error_report_triggered",
                    "created": 1728345600,
                    "livemode": false
                }
            ],
            "next_page_url": "https://api.stripe.com/v2/core/events?page=page_2_cursor",
            "previous_page_url": null
        })
        .to_string();

        let http_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            response_payload.len(),
            response_payload
        );
        socket.write_all(http_response.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_wire_secret")
        .with_api_base(format!("http://127.0.0.1:{}", port));
    let client = StripeClient::with_config(config);

    let params = ListV2EventsParams::new()
        .limit(10)
        .add_type("v1.billing.meter.error_report_triggered");

    let event_list: V2EventList = client
        .v2_events()
        .list_events(Some(&params))
        .await
        .expect("list_events must succeed");

    assert_eq!(event_list.data.len(), 1);
    assert_eq!(event_list.data[0].id, "evt_v2_list_1");
    assert!(event_list.has_more());
    assert_eq!(
        event_list.next_page_url.as_deref(),
        Some("https://api.stripe.com/v2/core/events?page=page_2_cursor")
    );

    server.await.unwrap();
}

#[tokio::test]
async fn test_retrieve_v2_event_security_path_traversal() {
    let client = StripeClient::new("sk_test_mock");
    let resource = client.v2_events();

    // Traversal attempts
    let err = resource.retrieve("../evt_secret").await.unwrap_err();
    assert!(matches!(err, StripeError::InvalidRequest(_)));

    let err = resource.retrieve("evt/secret").await.unwrap_err();
    assert!(matches!(err, StripeError::InvalidRequest(_)));

    let err = resource.retrieve("evt\\secret").await.unwrap_err();
    assert!(matches!(err, StripeError::InvalidRequest(_)));

    let err = resource.retrieve("evt?param=1").await.unwrap_err();
    assert!(matches!(err, StripeError::InvalidRequest(_)));

    let err = resource.retrieve("evt#anchor").await.unwrap_err();
    assert!(matches!(err, StripeError::InvalidRequest(_)));

    // Empty and whitespace
    let err = resource.retrieve("").await.unwrap_err();
    assert!(matches!(err, StripeError::InvalidRequest(_)));

    let err = resource.retrieve("   ").await.unwrap_err();
    assert!(matches!(err, StripeError::InvalidRequest(_)));
}

#[tokio::test]
async fn test_ac_us_8_1_0_01_thin_event_notification_and_live_hydration_flow() {
    // Scenario [AC-US-8.1.0-01]:
    // Inbound thin event delivered -> consumer reads notification -> invokes retrieve(id) -> full authenticated live state returned.
    let thin_event_payload = serde_json::json!({
        "id": "evt_v2_thin_meter_999",
        "object": "v2.core.event",
        "type": "v1.billing.meter.error_report_triggered",
        "created": 1728345600,
        "livemode": false,
        "related_object": {
            "id": "meter_prod_999",
            "type": "billing.meter",
            "url": "https://api.stripe.com/v2/billing/meters/meter_prod_999"
        }
    });

    let inbound: V2Event = serde_json::from_value(thin_event_payload).unwrap();
    assert!(inbound.is_thin());
    assert_eq!(inbound.related_object_id(), Some("meter_prod_999"));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let expected_event_id = inbound.id.clone();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(
            req_str.starts_with(&format!(
                "GET /v2/core/events/{} HTTP/1.1",
                expected_event_id
            ))
        );

        // Server responds with live hydrated state
        let live_payload = serde_json::json!({
            "id": "evt_v2_thin_meter_999",
            "object": "v2.core.event",
            "type": "v1.billing.meter.error_report_triggered",
            "created": 1728345600,
            "livemode": false,
            "data": {
                "meter_id": "meter_prod_999",
                "error_report": "threshold_breached",
                "drop_count": 42
            },
            "related_object": {
                "id": "meter_prod_999",
                "type": "billing.meter",
                "url": "https://api.stripe.com/v2/billing/meters/meter_prod_999"
            }
        })
        .to_string();

        let http_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            live_payload.len(),
            live_payload
        );
        socket.write_all(http_response.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{}", port));
    let client = StripeClient::with_config(config);

    // Live hydration call
    let hydrated = client
        .v2_events()
        .retrieve(&inbound.id)
        .await
        .expect("live hydration must succeed");

    assert_eq!(hydrated.id, inbound.id);
    assert!(!hydrated.is_thin()); // now hydrated with data
    let data = hydrated.data.unwrap();
    assert_eq!(data["error_report"], "threshold_breached");
    assert_eq!(data["drop_count"], 42);

    server.await.unwrap();
}
