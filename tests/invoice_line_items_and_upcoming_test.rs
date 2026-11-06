//! Integration test suite for Unit 2: Invoice Line Items, Dynamic Adjustments & Upcoming Previews.
//!
//! Validates:
//! 1. Upcoming invoice null-ID wire safety invariant (`GET /v1/invoices/upcoming` returns `"id": null`).
//! 2. Multi-item indexed bracket serialization for dynamic additions (`POST /v1/invoices/{id}/add_lines`).
//! 3. Line updates (`POST /v1/invoices/{id}/lines/{line_id}`) and removals (`POST /v1/invoices/{id}/remove_lines`).
//! 4. Zero-clone cursor-based pagination stream for invoice line items.

use some_stripe::{
    AddInvoiceLinesParams, InvoiceLineItemParams, InvoiceUpcomingParams,
    ListInvoiceLineItemsParams, StripeClient, StripeConfig, UpdateInvoiceLineParams,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_upcoming_preview_wire_null_id() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0; 8192];
        let size = socket.read(&mut request).await.unwrap();
        let wire = String::from_utf8(request[..size].to_vec()).unwrap();

        assert!(wire.starts_with("GET /v1/invoices/upcoming?"));
        assert!(wire.contains("customer=cus_upcoming_test"));
        assert!(wire.contains("subscription=sub_upcoming_test"));

        let body = r#"{
            "id": null,
            "object": "invoice",
            "customer": "cus_upcoming_test",
            "amount_due": 3500,
            "currency": "usd",
            "status": "draft",
            "lines": {
                "object": "list",
                "data": [
                    {
                        "id": "il_preview_item_1",
                        "object": "line_item",
                        "amount": 3500,
                        "currency": "usd",
                        "description": "Pro Seat Preview",
                        "quantity": 1,
                        "proration": false
                    }
                ],
                "has_more": false,
                "url": "/v1/invoices/upcoming/lines"
            }
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let params = InvoiceUpcomingParams::new()
        .customer("cus_upcoming_test")
        .subscription("sub_upcoming_test");

    let invoice = client.invoices().upcoming(&params).await.unwrap();

    assert!(invoice.is_upcoming());
    assert!(invoice.id().is_none());
    assert_eq!(invoice.amount_due, Some(3500));
    assert_eq!(invoice.currency.as_deref(), Some("usd"));

    let lines = invoice.lines_slice().unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].id(), "il_preview_item_1");
    assert_eq!(lines[0].amount(), 3500);
    assert_eq!(lines[0].description(), Some("Pro Seat Preview"));

    server.await.unwrap();
}

#[tokio::test]
async fn test_add_invoice_lines_params_indexed_bracket_serialization() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0; 8192];
        let size = socket.read(&mut request).await.unwrap();
        let wire = String::from_utf8(request[..size].to_vec()).unwrap();

        assert!(wire.starts_with("POST /v1/invoices/in_test_add/add_lines HTTP/1.1"));
        assert!(wire.contains("lines%5B0%5D%5Bprice%5D=price_pro_monthly"));
        assert!(wire.contains("lines%5B0%5D%5Bquantity%5D=2"));
        assert!(wire.contains("lines%5B1%5D%5Bamount%5D=1000"));
        assert!(wire.contains("lines%5B1%5D%5Bcurrency%5D=usd"));
        assert!(wire.contains("idempotency-key: idem_add_lines_unit_test"));

        let body = r#"{
            "id": "in_test_add",
            "object": "invoice",
            "amount_due": 6000,
            "currency": "usd",
            "status": "draft"
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let params = AddInvoiceLinesParams::new(vec![
        InvoiceLineItemParams::from_price("price_pro_monthly", 2),
        InvoiceLineItemParams::from_amount(1000, "usd", "Setup fee"),
    ])
    .idempotency_key("idem_add_lines_unit_test");

    let invoice = client
        .invoices()
        .add_lines("in_test_add", &params)
        .await
        .unwrap();
    assert_eq!(invoice.id.as_deref(), Some("in_test_add"));
    assert_eq!(invoice.amount_due, Some(6000));

    server.await.unwrap();
}

#[tokio::test]
async fn test_update_and_remove_invoice_lines() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Update line item
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0; 8192];
        let size = socket.read(&mut request).await.unwrap();
        let wire = String::from_utf8(request[..size].to_vec()).unwrap();

        assert!(wire.starts_with("POST /v1/invoices/in_mod_test/lines/il_mod_1 HTTP/1.1"));
        assert!(wire.contains("quantity=5"));

        let body_upd = r#"{
            "id": "il_mod_1",
            "object": "line_item",
            "amount": 5000,
            "currency": "usd",
            "quantity": 5
        }"#;
        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body_upd.len(),
                    body_upd
                )
                .as_bytes(),
            )
            .await
            .unwrap();

        // 2. Remove line items
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0; 8192];
        let size = socket.read(&mut request).await.unwrap();
        let wire = String::from_utf8(request[..size].to_vec()).unwrap();

        assert!(wire.starts_with("POST /v1/invoices/in_mod_test/remove_lines HTTP/1.1"));
        assert!(wire.contains("lines%5B0%5D%5Bid%5D=il_mod_1"));

        let body_rem = r#"{
            "id": "in_mod_test",
            "object": "invoice",
            "amount_due": 0,
            "currency": "usd",
            "status": "draft"
        }"#;
        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body_rem.len(),
                    body_rem
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // Update line
    let upd_params = UpdateInvoiceLineParams::new().quantity(5);
    let updated = client
        .invoices()
        .update_lines("in_mod_test", "il_mod_1", &upd_params)
        .await
        .unwrap();
    assert_eq!(updated.id(), "il_mod_1");
    assert_eq!(updated.quantity(), Some(5));

    // Remove line
    let removed = client
        .invoices()
        .remove_lines("in_mod_test", &["il_mod_1"])
        .await
        .unwrap();
    assert_eq!(removed.id.as_deref(), Some("in_mod_test"));

    server.await.unwrap();
}

#[tokio::test]
async fn test_list_invoice_line_items_pagination() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Page 1
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0; 8192];
        let size = socket.read(&mut request).await.unwrap();
        let wire = String::from_utf8(request[..size].to_vec()).unwrap();

        assert!(wire.starts_with("GET /v1/invoices/in_stream_test/lines?"));
        assert!(wire.contains("limit=2"));

        let page1 = r#"{
            "object": "list",
            "data": [
                {"id": "il_stream_1", "object": "line_item", "amount": 1000, "currency": "usd"},
                {"id": "il_stream_2", "object": "line_item", "amount": 2000, "currency": "usd"}
            ],
            "has_more": true,
            "url": "/v1/invoices/in_stream_test/lines"
        }"#;

        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    page1.len(),
                    page1
                )
                .as_bytes(),
            )
            .await
            .unwrap();

        // Page 2
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0; 8192];
        let size = socket.read(&mut request).await.unwrap();
        let wire = String::from_utf8(request[..size].to_vec()).unwrap();

        assert!(wire.contains("starting_after=il_stream_2"));

        let page2 = r#"{
            "object": "list",
            "data": [
                {"id": "il_stream_3", "object": "line_item", "amount": 3000, "currency": "usd"}
            ],
            "has_more": false,
            "url": "/v1/invoices/in_stream_test/lines"
        }"#;

        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    page2.len(),
                    page2
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let params = ListInvoiceLineItemsParams::new().limit(2);
    let mut paginator = client.invoices().list_line_items("in_stream_test", &params);

    let mut item_ids = Vec::new();
    while let Some(item) = paginator.next_item().await.unwrap() {
        item_ids.push(item.id().to_string());
    }

    assert_eq!(item_ids, vec!["il_stream_1", "il_stream_2", "il_stream_3"]);

    server.await.unwrap();
}
