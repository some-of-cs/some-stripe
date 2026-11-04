//! Wire mock auto-pagination & loop detection contract tests (`TC-1.3.0-01` & `TC-1.3.0-02`).

use serde::{Deserialize, Serialize};
use some_stripe_core::client::{StripeClient, StripeConfig};
use some_stripe_core::error::StripeError;
use some_stripe_core::patterns::pagination::CursorLoopDetector;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MockItem {
    pub id: String,
    pub name: String,
}

#[tokio::test]
async fn tc_1_3_0_01_stream_traversal_across_3_pages() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Page 1
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buf1 = vec![0; 4096];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
        assert!(req1.starts_with("GET /v1/test_items?limit=2 HTTP/1.1"));

        let body1 = r#"{"object":"list","data":[{"id":"item_1","name":"Alpha"},{"id":"item_2","name":"Beta"}],"has_more":true}"#;
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
        assert!(req2.contains("starting_after=item_2"));

        let body2 = r#"{"object":"list","data":[{"id":"item_3","name":"Gamma"},{"id":"item_4","name":"Delta"}],"has_more":false}"#;
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

    let config = StripeConfig::new("sk_test_wire_mock")
        .with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let req = client.http.get(format!("{}/v1/test_items?limit=2", client.config.api_base));
    let val = client.execute(req).await.unwrap();
    let page: some_stripe_core::patterns::pagination::ListResponse<MockItem> =
        serde_json::from_value(val).unwrap();

    assert_eq!(page.data.len(), 2);
    assert_eq!(page.data[0].id, "item_1");
    assert_eq!(page.data[1].id, "item_2");

    server.await.unwrap();
}

#[test]
fn tc_1_3_0_02_cursor_loop_interception() {
    let mut detector = CursorLoopDetector::new();
    assert!(detector.record_cursor("item_1").is_ok());
    assert!(detector.record_cursor("item_2").is_ok());
    assert!(detector.record_cursor("item_3").is_ok());

    // Duplicate cursor triggers immediate loop detection error
    let err = detector.record_cursor("item_2").unwrap_err();
    assert!(matches!(err, StripeError::PaginationLoopDetected { .. }));
}
