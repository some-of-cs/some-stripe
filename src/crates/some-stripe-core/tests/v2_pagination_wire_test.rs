//! Wire mock tests for URL-driven V2List pagination engine (`TC-8.1.0-03`).

use serde::{Deserialize, Serialize};
use some_stripe_core::client::{StripeClient, StripeConfig};
use some_stripe_core::error::StripeError;
use some_stripe_core::patterns::v2_pagination::V2Paginator;
use some_stripe_types::V2List;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MockV2Event {
    pub id: String,
    pub r#type: String,
    pub created: i64,
}

#[tokio::test]
async fn tc_8_1_0_03_multi_page_url_driven_pagination() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Page 1
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buf1 = vec![0; 4096];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
        assert!(req1.starts_with("GET /v2/core/events?limit=2 HTTP/1.1"));
        assert!(req1.contains("accept: application/json"));

        let body1 = r#"{
            "data": [
                {"id": "evt_v2_1", "type": "v2.billing.meter.event_received", "created": 1728345600},
                {"id": "evt_v2_2", "type": "v2.billing.meter.event_received", "created": 1728345601}
            ],
            "next_page_url": "/v2/core/events?page=2",
            "previous_page_url": null,
            "page_info": {"has_more": true, "total_count": 4}
        }"#;

        socket1
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body1}",
                    body1.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();

        // Page 2
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf2 = vec![0; 4096];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req2 = String::from_utf8(buf2[..n2].to_vec()).unwrap();
        assert!(req2.starts_with("GET /v2/core/events?page=2 HTTP/1.1"));
        assert!(req2.contains("accept: application/json"));

        let body2 = r#"{
            "data": [
                {"id": "evt_v2_3", "type": "v2.billing.meter.event_received", "created": 1728345602},
                {"id": "evt_v2_4", "type": "v2.billing.meter.event_received", "created": 1728345603}
            ],
            "next_page_url": null,
            "previous_page_url": "/v2/core/events?page=1",
            "page_info": {"has_more": false, "total_count": 4}
        }"#;

        socket2
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body2}",
                    body2.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config = StripeConfig::new("sk_test_wire_mock").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let mut stream: V2Paginator<'_, MockV2Event> =
        V2Paginator::new(&client, "/v2/core/events?limit=2").auto_paging_each();

    let mut events = Vec::new();
    while let Some(evt) = stream.next_item().await.unwrap() {
        events.push(evt);
    }

    assert_eq!(events.len(), 4);
    assert_eq!(events[0].id, "evt_v2_1");
    assert_eq!(events[1].id, "evt_v2_2");
    assert_eq!(events[2].id, "evt_v2_3");
    assert_eq!(events[3].id, "evt_v2_4");

    assert_eq!(stream.pages_fetched(), 2);
    assert!(stream.is_done());
    assert_eq!(stream.next_url(), None);

    server.await.unwrap();
}

#[tokio::test]
async fn test_absolute_url_rewrite_safety() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Page 1 returning absolute next_page_url
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buf1 = vec![0; 4096];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
        assert!(req1.starts_with("GET /v2/core/events HTTP/1.1"));

        let body1 = r#"{
            "data": [{"id": "evt_1", "type": "v2.event", "created": 100}],
            "next_page_url": "https://api.stripe.com/v2/core/events?page=page_two_token",
            "previous_page_url": null,
            "page_info": {"has_more": true}
        }"#;

        socket1
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body1}",
                    body1.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();

        // Page 2 dispatched to local mock listener (not external https://api.stripe.com)
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf2 = vec![0; 4096];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req2 = String::from_utf8(buf2[..n2].to_vec()).unwrap();
        assert!(req2.starts_with("GET /v2/core/events?page=page_two_token HTTP/1.1"));

        let body2 = r#"{
            "data": [{"id": "evt_2", "type": "v2.event", "created": 200}],
            "next_page_url": null,
            "previous_page_url": "/v2/core/events?page=1",
            "page_info": {"has_more": false}
        }"#;

        socket2
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body2}",
                    body2.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config = StripeConfig::new("sk_test_wire_mock").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let paginator: V2Paginator<'_, MockV2Event> = V2Paginator::new(&client, "/v2/core/events");
    let all = paginator.collect_all().await.unwrap();

    assert_eq!(all.len(), 2);
    assert_eq!(all[0].id, "evt_1");
    assert_eq!(all[1].id, "evt_2");

    server.await.unwrap();
}

#[tokio::test]
async fn test_connected_account_header_preservation() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Page 1
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buf1 = vec![0; 4096];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
        assert!(req1.contains("stripe-account: acct_connected_tenant_789"));

        let body1 = r#"{
            "data": [{"id": "evt_1", "type": "v2.event", "created": 100}],
            "next_page_url": "/v2/core/events?page=2",
            "previous_page_url": null,
            "page_info": {"has_more": true}
        }"#;

        socket1
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body1}",
                    body1.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();

        // Page 2
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf2 = vec![0; 4096];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req2 = String::from_utf8(buf2[..n2].to_vec()).unwrap();
        assert!(req2.contains("stripe-account: acct_connected_tenant_789"));

        let body2 = r#"{
            "data": [{"id": "evt_2", "type": "v2.event", "created": 200}],
            "next_page_url": null,
            "previous_page_url": null,
            "page_info": {"has_more": false}
        }"#;

        socket2
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body2}",
                    body2.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config = StripeConfig::new("sk_test_wire_mock").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let paginator: V2Paginator<'_, MockV2Event> =
        V2Paginator::new(&client, "/v2/core/events").with_account("acct_connected_tenant_789");

    let mut count = 0;
    paginator
        .for_each(|evt| {
            count += 1;
            assert!(!evt.id.is_empty());
            async { Ok(()) }
        })
        .await
        .unwrap();

    assert_eq!(count, 2);
    server.await.unwrap();
}

#[tokio::test]
async fn test_cyclical_loop_interception_on_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Page 1 returns link to page 2
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buf1 = vec![0; 4096];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let _ = String::from_utf8(buf1[..n1].to_vec()).unwrap();

        let body1 = r#"{
            "data": [{"id": "evt_1", "type": "v2.event", "created": 100}],
            "next_page_url": "/v2/core/events?page=loop",
            "previous_page_url": null,
            "page_info": {"has_more": true}
        }"#;

        socket1
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body1}",
                    body1.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();

        // Page 2 returns circular link back to /v2/core/events?page=loop
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf2 = vec![0; 4096];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let _ = String::from_utf8(buf2[..n2].to_vec()).unwrap();

        let body2 = r#"{
            "data": [{"id": "evt_2", "type": "v2.event", "created": 200}],
            "next_page_url": "/v2/core/events?page=loop",
            "previous_page_url": null,
            "page_info": {"has_more": true}
        }"#;

        socket2
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body2}",
                    body2.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config = StripeConfig::new("sk_test_wire_mock").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let mut paginator: V2Paginator<'_, MockV2Event> =
        V2Paginator::new(&client, "/v2/core/events?page=initial");

    // First item from page 1
    let item1 = paginator.next_item().await.unwrap();
    assert_eq!(item1.map(|i| i.id), Some("evt_1".into()));

    // Second item from page 2
    let item2 = paginator.next_item().await.unwrap();
    assert_eq!(item2.map(|i| i.id), Some("evt_2".into()));

    // Third attempt tries to re-fetch /v2/core/events?page=loop -> caught by loop detector!
    let err = paginator.next_item().await.unwrap_err();
    assert!(matches!(err, StripeError::PaginationLoopDetected { .. }));
    assert!(paginator.is_done());

    server.await.unwrap();
}

#[tokio::test]
async fn test_from_page_preseeded_stream() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Only page 2 is fetched over wire!
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(req.starts_with("GET /v2/core/events?page=2 HTTP/1.1"));

        let body = r#"{
            "data": [{"id": "evt_seeded_2", "type": "v2.event", "created": 200}],
            "next_page_url": null,
            "previous_page_url": "/v2/core/events?page=1",
            "page_info": {"has_more": false}
        }"#;

        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config = StripeConfig::new("sk_test_wire_mock").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let initial_page = V2List::new(
        vec![MockV2Event {
            id: "evt_seeded_1".into(),
            r#type: "v2.event".into(),
            created: 100,
        }],
        Some("/v2/core/events?page=2".into()),
        None,
        None,
    );

    let mut paginator = V2Paginator::from_page(&client, initial_page);
    assert_eq!(paginator.pages_fetched(), 1);

    let item1 = paginator.next_item().await.unwrap();
    assert_eq!(item1.map(|i| i.id), Some("evt_seeded_1".into()));

    let item2 = paginator.next_item().await.unwrap();
    assert_eq!(item2.map(|i| i.id), Some("evt_seeded_2".into()));

    let item3 = paginator.next_item().await.unwrap();
    assert!(item3.is_none());
    assert_eq!(paginator.pages_fetched(), 2);
    assert!(paginator.is_done());

    server.await.unwrap();
}
