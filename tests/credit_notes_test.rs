//! Integration tests for Stripe Credit Notes lifecycle and balance deductions (Unit 3).

use some_stripe::{
    CreateCreditNoteParams, CreditNote, CreditNoteLineItemParams, CreditNoteReason,
    CreditNoteStatus, CreditNoteType, ListCreditNotesParams, PreviewCreditNoteParams, StripeClient,
    StripeConfig, UpdateCreditNoteParams, is_valid_uuid_v4,
};
use std::collections::HashMap;
use std::str::FromStr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_credit_note_types_reasons_and_status() {
    let reason = CreditNoteReason::from_str("fraudulent").unwrap();
    assert_eq!(reason, CreditNoteReason::Fraudulent);
    assert_eq!(reason.as_str(), "fraudulent");
    assert_eq!(reason.to_string(), "fraudulent");

    let status = CreditNoteStatus::from_str("issued").unwrap();
    assert_eq!(status, CreditNoteStatus::Issued);
    assert!(status.is_issued());
    assert!(!status.is_void());

    let status_void = CreditNoteStatus::from_str("void").unwrap();
    assert_eq!(status_void, CreditNoteStatus::Void);
    assert!(status_void.is_void());
    assert!(!status_void.is_issued());

    let note_type = CreditNoteType::PrePayment;
    assert_eq!(note_type.as_str(), "pre_payment");
}

#[test]
fn test_credit_note_balance_deduction_invariants() {
    let client = StripeClient::new("sk_test_mock");
    let resource = client.credit_notes();

    // Valid: credit note <= paid amount
    assert!(resource.validate_balance_deduction(5000, 10000).is_ok());
    assert!(resource.validate_balance_deduction(10000, 10000).is_ok());

    // Invalid: credit note <= 0
    let err_zero = resource.validate_balance_deduction(0, 10000).unwrap_err();
    assert!(err_zero.to_string().contains("strictly positive"));

    let err_neg = resource
        .validate_balance_deduction(-500, 10000)
        .unwrap_err();
    assert!(err_neg.to_string().contains("strictly positive"));

    // Invalid: credit note exceeds paid amount (preventing historical invoice ledger tampering)
    let err_exceed = resource
        .validate_balance_deduction(15000, 10000)
        .unwrap_err();
    assert!(
        err_exceed
            .to_string()
            .contains("cannot exceed total invoice paid amount")
    );

    // Test struct method
    let cn = CreditNote {
        id: "cn_test_invariant".into(),
        object: "credit_note".into(),
        amount: 2500,
        credit_amount: 2500,
        currency: "usd".into(),
        customer: "cus_inv_1".into(),
        customer_balance_transaction: None,
        discount_amount: None,
        invoice: "in_inv_1".into(),
        lines: vec![],
        livemode: Some(false),
        memo: None,
        metadata: None,
        number: Some("CN-001".into()),
        out_of_band_amount: None,
        pdf: None,
        reason: Some(CreditNoteReason::OrderChange),
        refund: None,
        status: CreditNoteStatus::Issued,
        subtotal: Some(2500),
        total: Some(2500),
        r#type: Some(CreditNoteType::PostPayment),
        voided_at: None,
        created: Some(1730900000),
    };

    assert!(cn.validate_balance_deduction(5000).is_ok());
    assert!(cn.validate_balance_deduction(2000).is_err());
}

#[tokio::test]
async fn test_create_credit_note_wire_and_idempotency() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("POST /v1/credit_notes HTTP/1.1"));
        assert!(req_str.contains("invoice=in_test_invoice_999"));
        assert!(req_str.contains("amount=4500"));
        assert!(req_str.contains("reason=order_change"));
        assert!(req_str.contains("memo=Defective+merchandise"));
        assert!(req_str.contains("metadata%5Bstore_id%5D=store_777"));
        assert!(req_str.contains("lines%5B0%5D%5Btype%5D=invoice_line_item"));
        assert!(req_str.contains("lines%5B0%5D%5Binvoice_line_item%5D=il_line_item_1"));
        assert!(req_str.contains("lines%5B0%5D%5Bamount%5D=4500"));

        // Verify UUID v4 idempotency key header
        let idemp_header = req_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
            .map(|l| l.split(':').nth(1).unwrap().trim())
            .expect("Idempotency-Key header missing");
        assert!(is_valid_uuid_v4(idemp_header));

        let body = r#"{
            "id": "cn_test_wire_101",
            "object": "credit_note",
            "amount": 4500,
            "credit_amount": 4500,
            "currency": "usd",
            "customer": "cus_wire_customer_1",
            "invoice": "in_test_invoice_999",
            "status": "issued",
            "memo": "Defective merchandise",
            "reason": "order_change",
            "lines": [
                {
                    "id": "cnli_line_1",
                    "object": "credit_note_line_item",
                    "amount": 4500,
                    "invoice_line_item": "il_line_item_1",
                    "type": "invoice_line_item"
                }
            ]
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

    let mut metadata = HashMap::new();
    metadata.insert("store_id".into(), "store_777".into());

    let line = CreditNoteLineItemParams {
        r#type: "invoice_line_item".into(),
        invoice_line_item: Some("il_line_item_1".into()),
        amount: Some(4500),
        ..Default::default()
    };

    let params = CreateCreditNoteParams::for_invoice("in_test_invoice_999")
        .with_amount(4500)
        .with_reason(CreditNoteReason::OrderChange)
        .with_memo("Defective merchandise")
        .with_metadata(metadata)
        .with_lines(vec![line]);

    let credit_note = client.credit_notes().create(&params).await.unwrap();
    assert_eq!(credit_note.id, "cn_test_wire_101");
    assert_eq!(credit_note.amount, 4500);
    assert_eq!(credit_note.invoice, "in_test_invoice_999");
    assert!(credit_note.is_issued());
    assert_eq!(credit_note.lines.len(), 1);
    assert_eq!(credit_note.lines[0].id, "cnli_line_1");

    server.await.unwrap();
}

#[tokio::test]
async fn test_retrieve_update_and_void_credit_note() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        // 1. Retrieve
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);
            assert!(req_str.starts_with("GET /v1/credit_notes/cn_test_retrieve HTTP/1.1"));

            let body = r#"{
                "id": "cn_test_retrieve",
                "object": "credit_note",
                "amount": 3000,
                "credit_amount": 3000,
                "currency": "usd",
                "customer": "cus_123",
                "invoice": "in_456",
                "status": "issued"
            }"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }

        // 2. Update
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);
            assert!(req_str.starts_with("POST /v1/credit_notes/cn_test_retrieve HTTP/1.1"));
            assert!(req_str.contains("memo=Updated+memo"));
            assert!(req_str.contains("metadata%5Bupdated%5D=true"));

            let body = r#"{
                "id": "cn_test_retrieve",
                "object": "credit_note",
                "amount": 3000,
                "credit_amount": 3000,
                "currency": "usd",
                "customer": "cus_123",
                "invoice": "in_456",
                "status": "issued",
                "memo": "Updated memo"
            }"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }

        // 3. Void
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);
            assert!(req_str.starts_with("POST /v1/credit_notes/cn_test_retrieve/void HTTP/1.1"));

            let body = r#"{
                "id": "cn_test_retrieve",
                "object": "credit_note",
                "amount": 3000,
                "credit_amount": 3000,
                "currency": "usd",
                "customer": "cus_123",
                "invoice": "in_456",
                "status": "void",
                "voided_at": 1730905000
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
    let resource = client.credit_notes();

    // 1. Retrieve
    let cn = resource.retrieve("cn_test_retrieve").await.unwrap();
    assert_eq!(cn.id, "cn_test_retrieve");
    assert!(cn.is_issued());

    // 2. Update
    let mut meta = HashMap::new();
    meta.insert("updated".into(), "true".into());
    let update_params = UpdateCreditNoteParams::new()
        .with_memo("Updated memo")
        .with_metadata(meta);
    let updated = resource
        .update("cn_test_retrieve", &update_params)
        .await
        .unwrap();
    assert_eq!(updated.memo.as_deref(), Some("Updated memo"));

    // 3. Void
    let voided = resource
        .void_credit_note("cn_test_retrieve", None)
        .await
        .unwrap();
    assert_eq!(voided.status, CreditNoteStatus::Void);
    assert!(voided.is_void());
    assert_eq!(voided.voided_at, Some(1730905000));

    server.await.unwrap();
}

#[tokio::test]
async fn test_preview_and_list_credit_notes() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        // 1. Preview
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);
            assert!(req_str.starts_with("GET /v1/credit_notes/preview?"));
            assert!(req_str.contains("invoice=in_preview_101"));
            assert!(req_str.contains("amount=2000"));

            let body = r#"{
                "id": "cn_preview_mock",
                "object": "credit_note",
                "amount": 2000,
                "credit_amount": 2000,
                "currency": "usd",
                "customer": "cus_preview_cust",
                "invoice": "in_preview_101",
                "status": "issued"
            }"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }

        // 2. List
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);
            assert!(req_str.starts_with("GET /v1/credit_notes?"));
            assert!(req_str.contains("invoice=in_preview_101"));
            assert!(req_str.contains("limit=5"));

            let body = r#"{
                "object": "list",
                "data": [
                    {
                        "id": "cn_list_1",
                        "object": "credit_note",
                        "amount": 2000,
                        "currency": "usd",
                        "customer": "cus_preview_cust",
                        "invoice": "in_preview_101",
                        "status": "issued"
                    }
                ],
                "has_more": false,
                "url": "/v1/credit_notes"
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
    let resource = client.credit_notes();

    // 1. Preview
    let preview_params = PreviewCreditNoteParams::for_invoice("in_preview_101")
        .with_amount(2000)
        .with_reason(CreditNoteReason::Duplicate);
    let preview = resource.preview(&preview_params).await.unwrap();
    assert_eq!(preview.amount, 2000);
    assert_eq!(preview.invoice, "in_preview_101");

    // 2. List
    let list_params = ListCreditNotesParams {
        invoice: Some("in_preview_101".into()),
        limit: Some(5),
        ..Default::default()
    };
    let list = resource.list(Some(&list_params)).await.unwrap();
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id, "cn_list_1");
    assert!(!list.has_more);

    server.await.unwrap();
}
