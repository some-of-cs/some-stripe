//! Integration tests for PaymentLinksResource CRUD operations and Line Item pagination.
//!
//! Validates:
//! - Creating PaymentLink with multiple line items, price/quantity, and automatic/custom Idempotency-Key injection.
//! - Retrieving PaymentLink without injecting Idempotency-Key.
//! - Updating PaymentLink active status (deactivating and activating).
//! - Streaming line items associated with a payment link across multiple pages via `Paginator`.
//! - Gate G-Borrow zero-clone borrowing discipline.

use some_stripe::{
    CreatePaymentLinkParams, LineItem, ListLineItemsParams, PaymentLinkLineItemParams,
    PaymentLinkStatus, StripeClient, StripeConfig, UpdatePaymentLinkParams, is_valid_uuid_v4,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_payment_link_create_multi_line_items_and_idempotency() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Request 1: Create with auto UUID v4 idempotency key
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/payment_links HTTP/1.1"));
            let header = req_str
                .lines()
                .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
                .expect("Idempotency-Key must be injected on create");
            let key = header.split(':').nth(1).unwrap().trim();
            assert!(
                is_valid_uuid_v4(key),
                "Auto-injected key must be RFC 4122 UUID v4: {key}"
            );

            // Form data serialization supporting multiple line items:
            // Check that bracket notation line_items[0] and line_items[1] are encoded
            assert!(
                req_str.contains("line_items%5B0%5D%5Bprice%5D=price_standard_100")
                    || req_str.contains("line_items[0][price]=price_standard_100")
            );
            assert!(
                req_str.contains("line_items%5B0%5D%5Bquantity%5D=2")
                    || req_str.contains("line_items[0][quantity]=2")
            );
            assert!(
                req_str.contains("line_items%5B1%5D%5Bprice%5D=price_addon_50")
                    || req_str.contains("line_items[1][price]=price_addon_50")
            );
            assert!(
                req_str.contains("line_items%5B1%5D%5Bquantity%5D=1")
                    || req_str.contains("line_items[1][quantity]=1")
            );

            let resp = serde_json::json!({
                "id": "plink_test_create_001",
                "object": "payment_link",
                "active": true,
                "url": "https://buy.stripe.com/test_plink_create_001",
                "currency": "usd"
            })
            .to_string();

            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                        resp.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }

        // Request 2: Create with explicit user-provided idempotency key
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/payment_links HTTP/1.1"));
            assert!(
                req_str.contains("idempotency-key: custom_plink_idem_xyz"),
                "User-supplied Idempotency-Key must be preserved"
            );

            let resp = serde_json::json!({
                "id": "plink_test_create_002",
                "object": "payment_link",
                "active": true,
                "url": "https://buy.stripe.com/test_plink_create_002",
                "currency": "usd"
            })
            .to_string();

            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                        resp.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
    });

    let config = StripeConfig::new("sk_test_plink_crud").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // Call 1: Auto idempotency key with 2 line items
    let params1 = CreatePaymentLinkParams::new(vec![
        PaymentLinkLineItemParams::new("price_standard_100", 2),
        PaymentLinkLineItemParams::new("price_addon_50", 1),
    ])
    .active(true);

    let link1 = client.payment_links().create(&params1).await.unwrap();
    assert_eq!(link1.id, "plink_test_create_001");
    assert!(link1.is_active());
    assert_eq!(link1.status(), PaymentLinkStatus::Active);
    assert_eq!(link1.url, "https://buy.stripe.com/test_plink_create_001");

    // Call 2: Custom idempotency key via checkout().payment_links() accessor
    let params2 = CreatePaymentLinkParams::new(vec![PaymentLinkLineItemParams::new(
        "price_standard_100",
        1,
    )])
    .idempotency_key("custom_plink_idem_xyz");

    let link2 = client
        .checkout()
        .payment_links()
        .create(&params2)
        .await
        .unwrap();
    assert_eq!(link2.id, "plink_test_create_002");
    assert!(link2.is_active());

    server.await.unwrap();
}

#[tokio::test]
async fn test_payment_link_retrieve() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("GET /v1/payment_links/plink_retrieve_999 HTTP/1.1"));
        assert!(
            !req_str.to_lowercase().contains("idempotency-key"),
            "GET retrieve operation must NOT inject Idempotency-Key"
        );

        let resp = serde_json::json!({
            "id": "plink_retrieve_999",
            "object": "payment_link",
            "active": true,
            "url": "https://buy.stripe.com/test_plink_retrieve_999",
            "currency": "eur"
        })
        .to_string();

        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                    resp.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config =
        StripeConfig::new("sk_test_plink_retrieve").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let link = client
        .payment_links()
        .retrieve("plink_retrieve_999")
        .await
        .unwrap();
    assert_eq!(link.id, "plink_retrieve_999");
    assert!(link.is_active());
    assert_eq!(link.currency.as_deref(), Some("eur"));

    server.await.unwrap();
}

#[tokio::test]
async fn test_payment_link_update_active_status() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Step 1: Deactivate link (active: false)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/payment_links/plink_toggle_123 HTTP/1.1"));
            assert!(
                req_str.contains("active=false"),
                "Deactivate request must send active=false: {req_str}"
            );

            let resp = serde_json::json!({
                "id": "plink_toggle_123",
                "object": "payment_link",
                "active": false,
                "url": "https://buy.stripe.com/test_plink_toggle_123"
            })
            .to_string();

            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                        resp.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }

        // Step 2: Reactivate link (active: true)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/payment_links/plink_toggle_123 HTTP/1.1"));
            assert!(
                req_str.contains("active=true"),
                "Reactivate request must send active=true: {req_str}"
            );

            let resp = serde_json::json!({
                "id": "plink_toggle_123",
                "object": "payment_link",
                "active": true,
                "url": "https://buy.stripe.com/test_plink_toggle_123"
            })
            .to_string();

            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                        resp.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
    });

    let config =
        StripeConfig::new("sk_test_plink_update").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // 1. Deactivate
    let update_deactivate = UpdatePaymentLinkParams::new().active(false);
    let deactivated = client
        .payment_links()
        .update("plink_toggle_123", &update_deactivate)
        .await
        .unwrap();
    assert!(!deactivated.is_active());
    assert_eq!(deactivated.status(), PaymentLinkStatus::Inactive);

    // 2. Reactivate
    let update_reactivate = UpdatePaymentLinkParams::new().active(true);
    let reactivated = client
        .payment_links()
        .update("plink_toggle_123", &update_reactivate)
        .await
        .unwrap();
    assert!(reactivated.is_active());
    assert_eq!(reactivated.status(), PaymentLinkStatus::Active);

    server.await.unwrap();
}

#[tokio::test]
async fn test_payment_link_list_line_items_pagination() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Page 1
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buf1 = vec![0; 4096];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
        assert!(
            req1.starts_with("GET /v1/payment_links/plink_stream_001/line_items?limit=2 HTTP/1.1")
        );

        let page1 = serde_json::json!({
            "object": "list",
            "data": [
                {
                    "id": "li_item_001",
                    "object": "item",
                    "description": "Standard Widget",
                    "amount_subtotal": 2000,
                    "amount_total": 2000,
                    "currency": "usd",
                    "quantity": 1,
                    "price": {
                        "id": "price_widget_001",
                        "object": "price",
                        "active": true,
                        "currency": "usd",
                        "unit_amount": 2000
                    }
                },
                {
                    "id": "li_item_002",
                    "object": "item",
                    "description": "Widget Addon",
                    "amount_subtotal": 500,
                    "amount_total": 500,
                    "currency": "usd",
                    "quantity": 2,
                    "price": {
                        "id": "price_addon_002",
                        "object": "price",
                        "active": true,
                        "currency": "usd",
                        "unit_amount": 250
                    }
                }
            ],
            "has_more": true
        })
        .to_string();

        socket1
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page1}",
                    page1.len()
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
        assert!(req2.contains("starting_after=li_item_002"));

        let page2 = serde_json::json!({
            "object": "list",
            "data": [
                {
                    "id": "li_item_003",
                    "object": "item",
                    "description": "Priority Shipping",
                    "amount_subtotal": 800,
                    "amount_total": 800,
                    "currency": "usd",
                    "quantity": 1,
                    "price": {
                        "id": "price_shipping_003",
                        "object": "price",
                        "active": true,
                        "currency": "usd",
                        "unit_amount": 800
                    }
                }
            ],
            "has_more": false
        })
        .to_string();

        socket2
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page2}",
                    page2.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config =
        StripeConfig::new("sk_test_plink_paginator").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let params = ListLineItemsParams::new().limit(2);
    let mut paginator = client
        .payment_links()
        .list_line_items("plink_stream_001", &params);

    let mut items: Vec<LineItem> = Vec::new();
    while let Some(item) = paginator.next_item().await.unwrap() {
        items.push(item);
    }

    assert_eq!(items.len(), 3);
    assert_eq!(items[0].id, "li_item_001");
    assert_eq!(items[0].amount_total, 2000);
    assert_eq!(items[0].quantity, Some(1));
    assert_eq!(
        items[0].price.as_ref().map(|p| p.id.as_str()),
        Some("price_widget_001")
    );

    assert_eq!(items[1].id, "li_item_002");
    assert_eq!(items[1].amount_total, 500);
    assert_eq!(items[1].quantity, Some(2));

    assert_eq!(items[2].id, "li_item_003");
    assert_eq!(items[2].amount_total, 800);

    assert_eq!(paginator.pages_fetched(), 2);

    server.await.unwrap();
}

#[tokio::test]
async fn test_payment_link_list_line_items_with_limit_helper() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(
            req.starts_with("GET /v1/payment_links/plink_stream_002/line_items?limit=5 HTTP/1.1")
        );

        let resp = serde_json::json!({
            "object": "list",
            "data": [
                {
                    "id": "li_single_001",
                    "object": "item",
                    "description": "Solo Item",
                    "amount_subtotal": 1500,
                    "amount_total": 1500,
                    "currency": "usd"
                }
            ],
            "has_more": false
        })
        .to_string();

        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                    resp.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config =
        StripeConfig::new("sk_test_plink_limit_helper").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let mut paginator = client
        .checkout()
        .payment_links()
        .list_line_items_with_limit("plink_stream_002", 5);

    let item = paginator.next_item().await.unwrap().expect("1 item");
    assert_eq!(item.id, "li_single_001");
    assert_eq!(item.amount_total, 1500);

    let end = paginator.next_item().await.unwrap();
    assert!(end.is_none());
    assert_eq!(paginator.pages_fetched(), 1);

    server.await.unwrap();
}
