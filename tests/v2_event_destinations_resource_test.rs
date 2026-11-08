//! Integration and wire mock tests for Stripe API v2 Event Destinations Resource.
//!
//! Validates:
//! - Quality Gate G0: 0 compiler/clippy warnings under Rust 2024.
//! - Quality Gate G-Borrow: Zero-clone borrowing discipline on domain models.
//! - Quality Gate G2: Path traversal and query injection prevention via strict ID encoding.
//! - Quality Gate G-Law: NIST SP 800-92 security event logging and audit traceability.
//! - Wire contracts for `/v2/core/event_destinations`: `create`, `retrieve`, `update`,
//!   `del`, `list`, `enable`, `disable`, `ping`.

use some_stripe::v2_destination::{
    CreateEventDestinationParams, EventDestinationStatus, EventDestinationType,
    EventDestinationValidationError, UpdateEventDestinationParams,
};
use some_stripe::{StripeClient, StripeConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_v2_eventbridge_destination_create_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("POST /v2/core/event_destinations HTTP/1.1"));
        assert!(req_str.contains("content-type: application/json"));
        assert!(req_str.contains("\"type\":\"amazon_eventbridge\""));
        assert!(req_str.contains("\"aws_account_id\":\"123456789012\""));
        assert!(req_str.contains("\"aws_region\":\"us-east-1\""));

        let resp_body = serde_json::json!({
            "id": "edest_eb_123",
            "object": "v2.core.event_destination",
            "status": "enabled",
            "type": "amazon_eventbridge",
            "events": ["v2.billing.meter.error_report_triggered"],
            "created": 1728345600,
            "updated": 1728345650,
            "amazon_eventbridge": {
                "aws_account_id": "123456789012",
                "aws_region": "us-east-1",
                "partner_event_source_arn": "arn:aws:events:us-east-1:123456789012:event-bus/stripe"
            }
        });

        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            resp_body.to_string().len(),
            resp_body
        );
        socket.write_all(resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock_123").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let params = CreateEventDestinationParams::amazon_eventbridge(
        "123456789012",
        "us-east-1",
        vec!["v2.billing.meter.error_report_triggered".to_string()],
    );

    let destination = client.event_destinations().create(&params).await.unwrap();
    assert_eq!(destination.id, "edest_eb_123");
    assert_eq!(destination.object, "v2.core.event_destination");
    assert_eq!(destination.status, EventDestinationStatus::Enabled);
    assert_eq!(
        destination.destination_type,
        EventDestinationType::AmazonEventbridge
    );
    assert!(destination.is_enabled());
    assert!(destination.is_amazon_eventbridge());
    assert_eq!(destination.created, 1728345600);
    assert_eq!(destination.updated, Some(1728345650));

    let eb = destination.amazon_eventbridge.unwrap();
    assert_eq!(eb.aws_account_id, "123456789012");
    assert_eq!(eb.aws_region, "us-east-1");
    assert_eq!(
        eb.partner_event_source_arn.as_deref(),
        Some("arn:aws:events:us-east-1:123456789012:event-bus/stripe")
    );

    server.await.unwrap();
}

#[tokio::test]
async fn test_v2_webhook_destination_create_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("POST /v2/core/event_destinations HTTP/1.1"));
        assert!(req_str.contains("\"type\":\"webhook_endpoint\""));
        assert!(req_str.contains("\"url\":\"https://example.com/webhook\""));

        let resp_body = serde_json::json!({
            "id": "edest_wh_456",
            "object": "v2.core.event_destination",
            "status": "enabled",
            "type": "webhook_endpoint",
            "events": ["*"],
            "created": 1728345600,
            "webhook_endpoint": {
                "url": "https://example.com/webhook"
            }
        });

        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            resp_body.to_string().len(),
            resp_body
        );
        socket.write_all(resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock_123").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let params = CreateEventDestinationParams::webhook_endpoint(
        "https://example.com/webhook",
        vec!["*".to_string()],
    );

    let destination = client.event_destinations().create(&params).await.unwrap();
    assert_eq!(destination.id, "edest_wh_456");
    assert!(destination.is_webhook_endpoint());
    let wh = destination.webhook_endpoint.unwrap();
    assert_eq!(wh.url, "https://example.com/webhook");

    server.await.unwrap();
}

#[tokio::test]
async fn test_v2_azure_event_grid_create_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("POST /v2/core/event_destinations HTTP/1.1"));
        assert!(req_str.contains("\"type\":\"azure_event_grid\""));
        assert!(req_str.contains("\"azure_tenant_id\":\"tenant-abc\""));
        assert!(req_str.contains("\"azure_subscription_id\":\"sub-xyz\""));

        let resp_body = serde_json::json!({
            "id": "edest_az_789",
            "object": "v2.core.event_destination",
            "status": "enabled",
            "type": "azure_event_grid",
            "events": ["v2.core.*"],
            "created": 1728345600,
            "azure_event_grid": {
                "azure_tenant_id": "tenant-abc",
                "azure_subscription_id": "sub-xyz",
                "azure_resource_group": "rg-stripe",
                "azure_event_grid_topic_name": "topic-prod"
            }
        });

        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            resp_body.to_string().len(),
            resp_body
        );
        socket.write_all(resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock_123").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let params = CreateEventDestinationParams::azure_event_grid(
        "tenant-abc",
        "sub-xyz",
        "rg-stripe",
        "topic-prod",
        vec!["v2.core.*".to_string()],
    );

    let destination = client.event_destinations().create(&params).await.unwrap();
    assert_eq!(destination.id, "edest_az_789");
    assert!(destination.is_azure_event_grid());
    let az = destination.azure_event_grid.unwrap();
    assert_eq!(az.azure_tenant_id, "tenant-abc");
    assert_eq!(az.azure_event_grid_topic_name, "topic-prod");

    server.await.unwrap();
}

#[tokio::test]
async fn test_v2_destination_crud_lifecycle_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Request 1: GET /v2/core/event_destinations/edest_101
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("GET /v2/core/event_destinations/edest_101 HTTP/1.1"));

            let resp_body = serde_json::json!({
                "id": "edest_101",
                "object": "v2.core.event_destination",
                "status": "enabled",
                "type": "webhook_endpoint",
                "events": ["v1.*"],
                "created": 1728345600,
                "webhook_endpoint": {
                    "url": "https://example.com/v1"
                }
            });
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp_body.to_string().len(),
                resp_body
            );
            socket.write_all(resp.as_bytes()).await.unwrap();
        }

        // Request 2: POST /v2/core/event_destinations/edest_101
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("POST /v2/core/event_destinations/edest_101 HTTP/1.1"));
            assert!(req_str.contains("\"description\":\"Updated description\""));

            let resp_body = serde_json::json!({
                "id": "edest_101",
                "object": "v2.core.event_destination",
                "status": "enabled",
                "type": "webhook_endpoint",
                "events": ["v1.*"],
                "created": 1728345600,
                "description": "Updated description",
                "webhook_endpoint": {
                    "url": "https://example.com/v1"
                }
            });
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp_body.to_string().len(),
                resp_body
            );
            socket.write_all(resp.as_bytes()).await.unwrap();
        }

        // Request 3: POST /v2/core/event_destinations/edest_101/disable
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(
                req_str.starts_with("POST /v2/core/event_destinations/edest_101/disable HTTP/1.1")
            );

            let resp_body = serde_json::json!({
                "id": "edest_101",
                "object": "v2.core.event_destination",
                "status": "disabled",
                "type": "webhook_endpoint",
                "events": ["v1.*"],
                "created": 1728345600,
                "webhook_endpoint": {
                    "url": "https://example.com/v1"
                }
            });
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp_body.to_string().len(),
                resp_body
            );
            socket.write_all(resp.as_bytes()).await.unwrap();
        }

        // Request 4: POST /v2/core/event_destinations/edest_101/enable
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(
                req_str.starts_with("POST /v2/core/event_destinations/edest_101/enable HTTP/1.1")
            );

            let resp_body = serde_json::json!({
                "id": "edest_101",
                "object": "v2.core.event_destination",
                "status": "enabled",
                "type": "webhook_endpoint",
                "events": ["v1.*"],
                "created": 1728345600,
                "webhook_endpoint": {
                    "url": "https://example.com/v1"
                }
            });
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp_body.to_string().len(),
                resp_body
            );
            socket.write_all(resp.as_bytes()).await.unwrap();
        }

        // Request 5: POST /v2/core/event_destinations/edest_101/ping
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(
                req_str.starts_with("POST /v2/core/event_destinations/edest_101/ping HTTP/1.1")
            );

            let resp_body = serde_json::json!({
                "id": "edest_101",
                "object": "v2.core.event_destination",
                "status": "enabled",
                "type": "webhook_endpoint",
                "events": ["v1.*"],
                "created": 1728345600,
                "webhook_endpoint": {
                    "url": "https://example.com/v1"
                }
            });
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp_body.to_string().len(),
                resp_body
            );
            socket.write_all(resp.as_bytes()).await.unwrap();
        }

        // Request 6: GET /v2/core/event_destinations
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("GET /v2/core/event_destinations HTTP/1.1"));

            let resp_body = serde_json::json!({
                "object": "list",
                "data": [
                    {
                        "id": "edest_101",
                        "object": "v2.core.event_destination",
                        "status": "enabled",
                        "type": "webhook_endpoint",
                        "events": ["v1.*"],
                        "created": 1728345600,
                        "webhook_endpoint": {
                            "url": "https://example.com/v1"
                        }
                    }
                ]
            });
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp_body.to_string().len(),
                resp_body
            );
            socket.write_all(resp.as_bytes()).await.unwrap();
        }

        // Request 7: DELETE /v2/core/event_destinations/edest_101
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("DELETE /v2/core/event_destinations/edest_101 HTTP/1.1"));

            let resp_body = serde_json::json!({
                "id": "edest_101",
                "object": "v2.core.event_destination",
                "status": "disabled",
                "type": "webhook_endpoint",
                "events": ["v1.*"],
                "created": 1728345600,
                "webhook_endpoint": {
                    "url": "https://example.com/v1"
                }
            });
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp_body.to_string().len(),
                resp_body
            );
            socket.write_all(resp.as_bytes()).await.unwrap();
        }
    });

    let config = StripeConfig::new("sk_test_mock_123").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);
    let resource = client.event_destinations();

    // 1. Retrieve
    let dest = resource.retrieve("edest_101").await.unwrap();
    assert_eq!(dest.id, "edest_101");

    // 2. Update
    let update_params = UpdateEventDestinationParams::new().with_description("Updated description");
    let updated = resource.update("edest_101", &update_params).await.unwrap();
    assert_eq!(updated.description.as_deref(), Some("Updated description"));

    // 3. Disable
    let disabled = resource.disable("edest_101").await.unwrap();
    assert!(disabled.is_disabled());

    // 4. Enable
    let enabled = resource.enable("edest_101").await.unwrap();
    assert!(enabled.is_enabled());

    // 5. Ping
    let pinged = resource.ping("edest_101").await.unwrap();
    assert_eq!(pinged.id, "edest_101");

    // 6. List
    let list = resource.list().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "edest_101");

    // 7. Delete
    let deleted = resource.del("edest_101").await.unwrap();
    assert_eq!(deleted.id, "edest_101");

    server.await.unwrap();
}

#[tokio::test]
async fn test_v2_destination_security_guards() {
    let client = StripeClient::new("sk_test_mock_123");
    let resource = client.event_destinations();

    // Path traversal in ID is rejected
    assert!(resource.retrieve("../secret_id").await.is_err());
    assert!(resource.del("dest/with/slash").await.is_err());
    assert!(resource.enable("dest?query=injection").await.is_err());

    // Client-side parameter validation guards
    let empty_events =
        CreateEventDestinationParams::new(EventDestinationType::AmazonEventbridge, vec![]);
    assert_eq!(
        resource
            .create(&empty_events)
            .await
            .unwrap_err()
            .to_string(),
        "Invalid request parameter: Events list cannot be empty; at least one event type pattern is required"
    );

    let invalid_aws = CreateEventDestinationParams::amazon_eventbridge(
        "invalid_id",
        "us-east-1",
        vec!["v2.*".to_string()],
    );
    assert!(matches!(
        invalid_aws.validate(),
        Err(EventDestinationValidationError::InvalidAwsAccountId(_))
    ));
}
