//! Stream Zero-Clone Borrowing & Move Semantics Audit Suite (Gate G-Borrow)
//!
//! Verifies that Paginator<T> operates purely with move semantics and does NOT
//! require items to implement Clone, ensuring zero heap duplication during streaming.

use serde::{Deserialize, Serialize};
use some_stripe_core::client::{StripeClient, StripeConfig};
use some_stripe_core::patterns::pagination::Paginator;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Non-Clone domain entity intentionally lacking #[derive(Clone)]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NonCloneResource {
    pub id: String,
    pub payload: String,
}

#[tokio::test]
async fn test_paginator_zero_clone_move_semantics() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let _ = socket.read(&mut buf).await.unwrap();

        let body = r#"{"object":"list","data":[{"id":"res_1","payload":"data_1"},{"id":"res_2","payload":"data_2"}],"has_more":false}"#;
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

    let config = StripeConfig::new("sk_test_mock_zero_clone_stream")
        .with_api_base(format!("http://{}", addr));
    let client = StripeClient::with_config(config);

    let mut paginator: Paginator<NonCloneResource> =
        Paginator::new(&client, "v1/resources", 10, |r| r.id.clone());

    let item1 = paginator
        .next_item()
        .await
        .unwrap()
        .expect("First item must exist");
    assert_eq!(item1.id, "res_1");
    assert_eq!(item1.payload, "data_1");

    let item2 = paginator
        .next_item()
        .await
        .unwrap()
        .expect("Second item must exist");
    assert_eq!(item2.id, "res_2");
    assert_eq!(item2.payload, "data_2");

    let item3 = paginator.next_item().await.unwrap();
    assert!(item3.is_none());

    server.await.unwrap();
}
