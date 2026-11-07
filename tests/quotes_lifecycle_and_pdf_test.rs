//! Integration tests for Stripe Quotes lifecycle, state transitions, and zero-copy binary PDF streaming (Unit 4).

use some_stripe::{
    CreateQuoteParams, ListQuotesParams, QuoteLineItemParams, QuoteStatus, QuoteStatusTransitions,
    StripeClient, StripeConfig, UpdateQuoteParams, is_valid_uuid_v4,
};
use std::collections::HashMap;
use std::str::FromStr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_quote_status_parsing_and_transitions() {
    let draft = QuoteStatus::from_str("draft").unwrap();
    assert_eq!(draft, QuoteStatus::Draft);
    assert!(draft.is_draft());
    assert_eq!(draft.as_str(), "draft");

    let open = QuoteStatus::from_str("open").unwrap();
    assert_eq!(open, QuoteStatus::Open);
    assert!(open.is_open());

    let accepted = QuoteStatus::from_str("accepted").unwrap();
    assert_eq!(accepted, QuoteStatus::Accepted);
    assert!(accepted.is_accepted());

    let canceled = QuoteStatus::from_str("canceled").unwrap();
    assert_eq!(canceled, QuoteStatus::Canceled);
    assert!(canceled.is_canceled());

    // Valid state transitions
    assert!(QuoteStatusTransitions::can_transition(
        QuoteStatus::Draft,
        QuoteStatus::Open
    ));
    assert!(QuoteStatusTransitions::can_transition(
        QuoteStatus::Draft,
        QuoteStatus::Canceled
    ));
    assert!(QuoteStatusTransitions::can_transition(
        QuoteStatus::Open,
        QuoteStatus::Accepted
    ));
    assert!(QuoteStatusTransitions::can_transition(
        QuoteStatus::Open,
        QuoteStatus::Canceled
    ));

    // Illegal transitions
    assert!(!QuoteStatusTransitions::can_transition(
        QuoteStatus::Accepted,
        QuoteStatus::Open
    ));
    assert!(!QuoteStatusTransitions::can_transition(
        QuoteStatus::Canceled,
        QuoteStatus::Draft
    ));
    assert!(!QuoteStatusTransitions::can_transition(
        QuoteStatus::Draft,
        QuoteStatus::Accepted
    ));
}

#[tokio::test]
async fn test_quote_crud_and_lifecycle_transitions() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        // 1. Create
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            assert!(req_str.starts_with("POST /v1/quotes HTTP/1.1"));
            assert!(req_str.contains("customer=cus_quote_customer_1"));
            assert!(req_str.contains("description=Enterprise+SaaS+License"));
            assert!(req_str.contains("expires_at=1735689600"));
            assert!(req_str.contains("metadata%5Bdeal_id%5D=deal_999"));
            assert!(req_str.contains("line_items%5B0%5D%5Bprice%5D=price_enterprise_yearly"));
            assert!(req_str.contains("line_items%5B0%5D%5Bquantity%5D=5"));

            let idemp_header = req_str
                .lines()
                .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
                .map(|l| l.split(':').nth(1).unwrap().trim())
                .expect("Idempotency-Key header missing");
            assert!(is_valid_uuid_v4(idemp_header));

            let body = r#"{
                "id": "qt_mock_1",
                "object": "quote",
                "amount_subtotal": 50000,
                "amount_total": 50000,
                "currency": "usd",
                "customer": "cus_quote_customer_1",
                "description": "Enterprise SaaS License",
                "expires_at": 1735689600,
                "status": "draft"
            }"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }

        // 2. Retrieve
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);
            assert!(req_str.starts_with("GET /v1/quotes/qt_mock_1 HTTP/1.1"));

            let body = r#"{
                "id": "qt_mock_1",
                "object": "quote",
                "amount_subtotal": 50000,
                "amount_total": 50000,
                "currency": "usd",
                "customer": "cus_quote_customer_1",
                "status": "draft",
                "expires_at": 1735689600
            }"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }

        // 3. Update
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);
            assert!(req_str.starts_with("POST /v1/quotes/qt_mock_1 HTTP/1.1"));
            assert!(req_str.contains("description=Updated+Description"));

            let body = r#"{
                "id": "qt_mock_1",
                "object": "quote",
                "amount_subtotal": 50000,
                "amount_total": 50000,
                "currency": "usd",
                "customer": "cus_quote_customer_1",
                "description": "Updated Description",
                "status": "draft",
                "expires_at": 1735689600
            }"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }

        // 4. Finalize (draft -> open)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);
            assert!(req_str.starts_with("POST /v1/quotes/qt_mock_1/finalize HTTP/1.1"));

            let body = r#"{
                "id": "qt_mock_1",
                "object": "quote",
                "amount_subtotal": 50000,
                "amount_total": 50000,
                "currency": "usd",
                "customer": "cus_quote_customer_1",
                "status": "open",
                "expires_at": 1735689600
            }"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }

        // 5. Accept (open -> accepted)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);
            assert!(req_str.starts_with("POST /v1/quotes/qt_mock_1/accept HTTP/1.1"));

            let body = r#"{
                "id": "qt_mock_1",
                "object": "quote",
                "amount_subtotal": 50000,
                "amount_total": 50000,
                "currency": "usd",
                "customer": "cus_quote_customer_1",
                "invoice": "in_created_by_quote_1",
                "status": "accepted",
                "expires_at": 1735689600
            }"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let quotes = client.quotes();

    // 1. Create
    let mut meta = HashMap::new();
    meta.insert("deal_id".into(), "deal_999".into());
    let line = QuoteLineItemParams::new()
        .with_price("price_enterprise_yearly")
        .with_quantity(5);

    let create_params = CreateQuoteParams::new()
        .with_customer("cus_quote_customer_1")
        .with_description("Enterprise SaaS License")
        .with_expires_at(1735689600)
        .with_metadata(meta)
        .with_line_items(vec![line]);

    let created = quotes.create(&create_params).await.unwrap();
    assert_eq!(created.id, "qt_mock_1");
    assert!(created.is_draft());
    assert_eq!(created.amount_total, 50000);

    // 2. Retrieve
    let retrieved = quotes.retrieve("qt_mock_1").await.unwrap();
    assert_eq!(retrieved.id, "qt_mock_1");
    assert!(retrieved.is_draft());

    // 3. Update
    let update_params = UpdateQuoteParams::new().with_description("Updated Description");
    let updated = quotes.update("qt_mock_1", &update_params).await.unwrap();
    assert_eq!(updated.description.as_deref(), Some("Updated Description"));

    // 4. Finalize
    let finalized = quotes.finalize_quote("qt_mock_1", None).await.unwrap();
    assert_eq!(finalized.status, QuoteStatus::Open);
    assert!(finalized.is_open());

    // 5. Accept
    let accepted = quotes.accept("qt_mock_1", None).await.unwrap();
    assert_eq!(accepted.status, QuoteStatus::Accepted);
    assert!(accepted.is_accepted());
    assert_eq!(accepted.invoice.as_deref(), Some("in_created_by_quote_1"));

    server.await.unwrap();
}

#[tokio::test]
async fn test_quote_cancel_transition() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);
        assert!(req_str.starts_with("POST /v1/quotes/qt_to_cancel/cancel HTTP/1.1"));

        let body = r#"{
            "id": "qt_to_cancel",
            "object": "quote",
            "amount_subtotal": 20000,
            "amount_total": 20000,
            "currency": "usd",
            "status": "canceled",
            "expires_at": 1735689600
        }"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let quotes = client.quotes();

    let canceled = quotes.cancel("qt_to_cancel", None).await.unwrap();
    assert_eq!(canceled.status, QuoteStatus::Canceled);
    assert!(canceled.is_canceled());

    server.await.unwrap();
}

#[tokio::test]
async fn test_zero_copy_quote_and_invoice_binary_pdf_streaming() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let mock_pdf_bytes = b"%PDF-1.4\n1 0 obj\n<< /Title (Test Invoice/Quote) >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF";

    let server = tokio::spawn(async move {
        // 1. Quote PDF
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);
            assert!(req_str.starts_with("GET /v1/quotes/qt_stream_pdf/pdf HTTP/1.1"));

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/pdf\r\nContent-Length: {}\r\n\r\n",
                mock_pdf_bytes.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.write_all(mock_pdf_bytes).await.unwrap();
        }

        // 2. Invoice PDF
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);
            assert!(req_str.starts_with("GET /v1/invoices/in_stream_pdf/pdf HTTP/1.1"));

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/pdf\r\nContent-Length: {}\r\n\r\n",
                mock_pdf_bytes.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.write_all(mock_pdf_bytes).await.unwrap();
        }
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);

    // Stream Quote PDF
    let quote_pdf: bytes::Bytes = client.quotes().pdf("qt_stream_pdf").await.unwrap();
    assert!(quote_pdf.starts_with(b"%PDF-1.4"));
    assert_eq!(&quote_pdf[..], mock_pdf_bytes);

    // Stream Invoice PDF
    let invoice_pdf: bytes::Bytes = client.invoices().pdf("in_stream_pdf").await.unwrap();
    assert!(invoice_pdf.starts_with(b"%PDF-1.4"));
    assert_eq!(&invoice_pdf[..], mock_pdf_bytes);

    server.await.unwrap();
}

#[tokio::test]
async fn test_list_quotes_and_paginator() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);
        assert!(req_str.starts_with("GET /v1/quotes?"));
        assert!(req_str.contains("customer=cus_list_test"));
        assert!(req_str.contains("status=open"));
        assert!(req_str.contains("limit=5"));

        let body = r#"{
            "object": "list",
            "data": [
                {
                    "id": "qt_list_1",
                    "object": "quote",
                    "amount_subtotal": 12000,
                    "amount_total": 12000,
                    "currency": "usd",
                    "customer": "cus_list_test",
                    "status": "open",
                    "expires_at": 1735689600
                }
            ],
            "has_more": false,
            "url": "/v1/quotes"
        }"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);

    let params = ListQuotesParams {
        customer: Some("cus_list_test".into()),
        status: Some(QuoteStatus::Open),
        limit: Some(5),
        ..Default::default()
    };

    let list = client.quotes().list(Some(&params)).await.unwrap();
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id, "qt_list_1");
    assert_eq!(list.data[0].status, QuoteStatus::Open);
    assert!(!list.has_more);

    server.await.unwrap();
}
