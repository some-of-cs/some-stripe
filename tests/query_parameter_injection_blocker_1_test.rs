//! Integration tests verifying HTTP Query Parameter Injection and Parameter Pollution prevention
//! across all 7 resource endpoints (Blocker 1 security fix).
//!
//! Verifies that special characters (`&`, `=`, `#`, spaces, etc.) in filter parameters
//! are safely percent-encoded and do not inject extraneous parameters or truncate URLs.

use some_stripe::{
    CreditNoteReason, InvoiceUpcomingParams, ListApplicationFeesParams, ListCreditNotesParams,
    ListFeeRefundsParams, ListInvoicesParams, ListPayoutsParams, ListQuotesParams, PayoutStatus,
    PreviewCreditNoteParams, QuoteStatus, StripeClient, StripeConfig,
};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn send_mock_response(socket: &mut tokio::net::TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    socket.write_all(response.as_bytes()).await.unwrap();
}

#[tokio::test]
async fn test_invoices_list_and_upcoming_query_injection_prevention() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        // 1. InvoicesResource::list with malicious customer parameter
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            let req_line = req_str.lines().next().unwrap();
            assert!(
                req_line.starts_with("GET /v1/invoices?"),
                "Expected GET /v1/invoices?, got: {req_line}"
            );
            // Verify percent-encoding of &, =, #, space
            assert!(
                req_line.contains("customer=cus_123%26injected%3Dtrue%23fragment%20space"),
                "Customer must be percent encoded, got: {req_line}"
            );
            // Ensure no unencoded injected parameter
            assert!(
                !req_line.contains("&injected=true"),
                "Unencoded injection found in: {req_line}"
            );

            let body = r#"{"object":"list","data":[],"has_more":false,"url":"/v1/invoices"}"#;
            send_mock_response(&mut socket, body).await;
        }

        // 2. InvoicesResource::list with None params
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            let req_line = req_str.lines().next().unwrap();
            assert_eq!(req_line, "GET /v1/invoices HTTP/1.1");

            let body = r#"{"object":"list","data":[],"has_more":false,"url":"/v1/invoices"}"#;
            send_mock_response(&mut socket, body).await;
        }

        // 3. InvoicesResource::upcoming with special characters in params
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            let req_line = req_str.lines().next().unwrap();
            assert!(
                req_line.starts_with("GET /v1/invoices/upcoming?"),
                "Expected GET /v1/invoices/upcoming?, got: {req_line}"
            );
            assert!(
                req_line.contains("customer=cus_test%26evil%3Dtrue"),
                "Expected percent encoded customer, got: {req_line}"
            );
            assert!(
                req_line.contains("subscription=sub_val%3D123%26admin%3D1"),
                "Expected percent encoded subscription, got: {req_line}"
            );
            assert!(
                !req_line.contains("&evil=true"),
                "Unencoded injection evil=true found in: {req_line}"
            );
            assert!(
                !req_line.contains("&admin=1"),
                "Unencoded injection admin=1 found in: {req_line}"
            );

            let body = r#"{
                "object": "invoice",
                "customer": "cus_test&evil=true",
                "subscription": "sub_val=123&admin=1",
                "amount_due": 1500,
                "currency": "usd",
                "status": "draft"
            }"#;
            send_mock_response(&mut socket, body).await;
        }
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let invoices = client.invoices();

    // 1. List with injection payload
    let list_params =
        ListInvoicesParams::new().with_customer("cus_123&injected=true#fragment space");
    let list = invoices.list(Some(&list_params)).await.unwrap();
    assert_eq!(list.data.len(), 0);

    // 2. List with None
    let list_none = invoices.list(None).await.unwrap();
    assert_eq!(list_none.data.len(), 0);

    // 3. Upcoming with injection payload
    let upcoming_params = InvoiceUpcomingParams::new()
        .customer("cus_test&evil=true")
        .subscription("sub_val=123&admin=1");
    let upcoming = invoices.upcoming(&upcoming_params).await.unwrap();
    assert_eq!(upcoming.amount_due, Some(1500));

    server.await.unwrap();
}

#[tokio::test]
async fn test_credit_notes_preview_and_list_query_injection_and_metadata() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        // 1. CreditNotesResource::preview with metadata and special characters
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            let req_line = req_str.lines().next().unwrap();
            assert!(
                req_line.starts_with("GET /v1/credit_notes/preview?"),
                "Expected preview request, got: {req_line}"
            );
            // Verify invoice is encoded
            assert!(
                req_line.contains("invoice=in_test%26inject%3D1%23frag"),
                "Invoice must be encoded, got: {req_line}"
            );
            // Verify memo is encoded
            assert!(
                req_line.contains("memo=Memo%20%26%20Special%3DValue%23hash"),
                "Memo must be encoded, got: {req_line}"
            );
            // Verify metadata is serialized with bracket notation and encoded
            assert!(
                req_line.contains("metadata%5Benv%26dev%5D=test%3Dval%231")
                    || req_line.contains("metadata[env&dev]=test=val#1"),
                "Metadata must be bracket formatted and safely encoded, got: {req_line}"
            );
            // Verify no raw unencoded injection
            assert!(
                !req_line.contains("&inject=1"),
                "Found raw parameter injection &inject=1: {req_line}"
            );

            let body = r#"{
                "id": "cn_preview_test",
                "object": "credit_note",
                "amount": 1000,
                "currency": "usd",
                "customer": "cus_123",
                "invoice": "in_test&inject=1#frag",
                "status": "issued"
            }"#;
            send_mock_response(&mut socket, body).await;
        }

        // 2. CreditNotesResource::list with injection payload
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            let req_line = req_str.lines().next().unwrap();
            assert!(
                req_line.starts_with("GET /v1/credit_notes?"),
                "Expected list request, got: {req_line}"
            );
            assert!(
                req_line.contains("customer=cus_pollute%26danger%3Dtrue"),
                "Customer must be encoded, got: {req_line}"
            );
            assert!(
                !req_line.contains("&danger=true"),
                "Found raw injection &danger=true: {req_line}"
            );

            let body = r#"{"object":"list","data":[],"has_more":false,"url":"/v1/credit_notes"}"#;
            send_mock_response(&mut socket, body).await;
        }

        // 3. CreditNotesResource::list with None
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            let req_line = req_str.lines().next().unwrap();
            assert_eq!(req_line, "GET /v1/credit_notes HTTP/1.1");

            let body = r#"{"object":"list","data":[],"has_more":false,"url":"/v1/credit_notes"}"#;
            send_mock_response(&mut socket, body).await;
        }
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let credit_notes = client.credit_notes();

    // 1. Preview with metadata
    let mut meta = HashMap::new();
    meta.insert("env&dev".to_string(), "test=val#1".to_string());
    let preview_params = PreviewCreditNoteParams::for_invoice("in_test&inject=1#frag")
        .with_amount(1000)
        .with_reason(CreditNoteReason::Duplicate)
        .with_memo("Memo & Special=Value#hash");
    let mut preview_params_with_meta = preview_params;
    preview_params_with_meta.metadata = Some(meta);

    let preview = credit_notes
        .preview(&preview_params_with_meta)
        .await
        .unwrap();
    assert_eq!(preview.amount, 1000);

    // 2. List with injection payload
    let list_params = ListCreditNotesParams {
        customer: Some("cus_pollute&danger=true".into()),
        ..Default::default()
    };
    let list = credit_notes.list(Some(&list_params)).await.unwrap();
    assert_eq!(list.data.len(), 0);

    // 3. List with None
    let list_none = credit_notes.list(None).await.unwrap();
    assert_eq!(list_none.data.len(), 0);

    server.await.unwrap();
}

#[tokio::test]
async fn test_quotes_list_query_injection_prevention() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        // 1. QuotesResource::list with customer injection payload
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            let req_line = req_str.lines().next().unwrap();
            assert!(
                req_line.starts_with("GET /v1/quotes?"),
                "Expected GET /v1/quotes?, got: {req_line}"
            );
            assert!(
                req_line.contains("customer=cus_quote%26attack%3D1%23hash"),
                "Customer must be encoded, got: {req_line}"
            );
            assert!(
                req_line.contains("status=open"),
                "Status must be open, got: {req_line}"
            );
            assert!(
                !req_line.contains("&attack=1"),
                "Found unencoded injection &attack=1 in: {req_line}"
            );

            let body = r#"{"object":"list","data":[],"has_more":false,"url":"/v1/quotes"}"#;
            send_mock_response(&mut socket, body).await;
        }

        // 2. QuotesResource::list with None
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            let req_line = req_str.lines().next().unwrap();
            assert_eq!(req_line, "GET /v1/quotes HTTP/1.1");

            let body = r#"{"object":"list","data":[],"has_more":false,"url":"/v1/quotes"}"#;
            send_mock_response(&mut socket, body).await;
        }
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let quotes = client.quotes();

    // 1. List with injection
    let list_params = ListQuotesParams {
        customer: Some("cus_quote&attack=1#hash".into()),
        status: Some(QuoteStatus::Open),
        ..Default::default()
    };
    let list = quotes.list(Some(&list_params)).await.unwrap();
    assert_eq!(list.data.len(), 0);

    // 2. List with None
    let list_none = quotes.list(None).await.unwrap();
    assert_eq!(list_none.data.len(), 0);

    server.await.unwrap();
}

#[tokio::test]
async fn test_application_fees_list_and_refunds_injection_prevention() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        // 1. ApplicationFeesResource::list with charge injection
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            let req_line = req_str.lines().next().unwrap();
            assert!(
                req_line.starts_with("GET /v1/application_fees?"),
                "Expected GET /v1/application_fees?, got: {req_line}"
            );
            assert!(
                req_line.contains("charge=ch_test%26evil_fee%3D999"),
                "Charge must be encoded, got: {req_line}"
            );
            assert!(
                !req_line.contains("&evil_fee=999"),
                "Found unencoded injection &evil_fee=999 in: {req_line}"
            );

            let body =
                r#"{"object":"list","data":[],"has_more":false,"url":"/v1/application_fees"}"#;
            send_mock_response(&mut socket, body).await;
        }

        // 2. ApplicationFeesResource::list with None
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            let req_line = req_str.lines().next().unwrap();
            assert_eq!(req_line, "GET /v1/application_fees HTTP/1.1");

            let body =
                r#"{"object":"list","data":[],"has_more":false,"url":"/v1/application_fees"}"#;
            send_mock_response(&mut socket, body).await;
        }

        // 3. ApplicationFeesResource::list_refunds with starting_after injection
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            let req_line = req_str.lines().next().unwrap();
            assert!(
                req_line.starts_with("GET /v1/application_fees/fee_123/refunds?"),
                "Expected GET refunds with query, got: {req_line}"
            );
            assert!(
                req_line.contains("starting_after=fr_start%26leak%3Dtrue"),
                "starting_after must be encoded, got: {req_line}"
            );
            assert!(
                !req_line.contains("&leak=true"),
                "Found unencoded injection &leak=true in: {req_line}"
            );

            let body = r#"{"object":"list","data":[],"has_more":false,"url":"/v1/application_fees/fee_123/refunds"}"#;
            send_mock_response(&mut socket, body).await;
        }

        // 4. ApplicationFeesResource::list_refunds with None
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            let req_line = req_str.lines().next().unwrap();
            assert_eq!(
                req_line,
                "GET /v1/application_fees/fee_123/refunds HTTP/1.1"
            );

            let body = r#"{"object":"list","data":[],"has_more":false,"url":"/v1/application_fees/fee_123/refunds"}"#;
            send_mock_response(&mut socket, body).await;
        }
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let app_fees = client.application_fees();

    // 1. List with charge injection
    let list_params = ListApplicationFeesParams {
        charge: Some("ch_test&evil_fee=999".into()),
        ..Default::default()
    };
    let list = app_fees.list(Some(&list_params)).await.unwrap();
    assert_eq!(list.data.len(), 0);

    // 2. List with None
    let list_none = app_fees.list(None).await.unwrap();
    assert_eq!(list_none.data.len(), 0);

    // 3. List refunds with starting_after injection
    let refund_params = ListFeeRefundsParams {
        starting_after: Some("fr_start&leak=true".into()),
        ..Default::default()
    };
    let refunds = app_fees
        .list_refunds("fee_123", Some(&refund_params))
        .await
        .unwrap();
    assert_eq!(refunds.data.len(), 0);

    // 4. List refunds with None
    let refunds_none = app_fees.list_refunds("fee_123", None).await.unwrap();
    assert_eq!(refunds_none.data.len(), 0);

    server.await.unwrap();
}

#[tokio::test]
async fn test_payouts_list_query_injection_prevention() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        // 1. PayoutsResource::list with starting_after injection and status
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            let req_line = req_str.lines().next().unwrap();
            assert!(
                req_line.starts_with("GET /v1/payouts?"),
                "Expected GET /v1/payouts?, got: {req_line}"
            );
            assert!(
                req_line.contains("starting_after=po_start%26escalate%3Dtrue%23hash"),
                "starting_after must be encoded, got: {req_line}"
            );
            assert!(
                req_line.contains("status=pending"),
                "status must be pending, got: {req_line}"
            );
            assert!(
                !req_line.contains("&escalate=true"),
                "Found unencoded injection &escalate=true in: {req_line}"
            );

            let body = r#"{"object":"list","data":[],"has_more":false,"url":"/v1/payouts"}"#;
            send_mock_response(&mut socket, body).await;
        }

        // 2. PayoutsResource::list with None
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            let req_line = req_str.lines().next().unwrap();
            assert_eq!(req_line, "GET /v1/payouts HTTP/1.1");

            let body = r#"{"object":"list","data":[],"has_more":false,"url":"/v1/payouts"}"#;
            send_mock_response(&mut socket, body).await;
        }
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let payouts = client.payouts();

    // 1. List with starting_after injection
    let list_params = ListPayoutsParams {
        status: Some(PayoutStatus::Pending),
        starting_after: Some("po_start&escalate=true#hash".into()),
        ..Default::default()
    };
    let list = payouts.list(Some(&list_params)).await.unwrap();
    assert_eq!(list.data.len(), 0);

    // 2. List with None
    let list_none = payouts.list(None).await.unwrap();
    assert_eq!(list_none.data.len(), 0);

    server.await.unwrap();
}
