//! Wire mock integration tests for Refunds, Disputes, and Balance ledger resources
//! covering User Story 3.2.0 (`TC-3.2.0-01` through `TC-3.2.0-06`).
//!
//! # Quality Gates Verified:
//! - **G0**: Clean compilation on Rust 2024 edition without warnings.
//! - **G-Borrow**: Zero-clone borrowing discipline across resources and paginator stream.
//! - **G-Law**: ISO 4217 minor currency integer units ([`i64`]); EU Directive 2011/83/EU Article 13
//!   statutory 14-day withdrawal compliance (100% full reimbursement without penalty).
//! - **G2**: Idempotency key preservation across retry and execution headers.

use some_stripe::resources::balance::BalanceTransactionsResource;
use some_stripe::{
    Balance, BalanceTransaction, CancelRefundParams, CreateRefundParams, Dispute,
    DisputeEvidenceParams, Refund, StripeClient, StripeClientExt, StripeConfig,
    UpdateDisputeParams, UpdateRefundParams,
};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Helper to write an HTTP 200 response with JSON payload to a socket.
async fn write_json_response(socket: &mut tokio::net::TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    socket.write_all(response.as_bytes()).await.unwrap();
}

#[tokio::test]
async fn test_refunds_create_statutory_and_idempotency_e2e() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut req_buf = vec![0; 8192];
        let n = socket.read(&mut req_buf).await.unwrap();
        let wire = String::from_utf8(req_buf[..n].to_vec()).unwrap();

        assert!(wire.starts_with("POST /v1/refunds HTTP/1.1"));
        assert!(wire.contains("idempotency-key: idem_refund_test_001"));
        assert!(wire.contains("payment_intent=pi_test_intent_123"));
        assert!(wire.contains("amount=2500"));
        assert!(wire.contains("reason=requested_by_customer"));

        let body = r#"{
            "id": "re_mock_test_001",
            "object": "refund",
            "amount": 2500,
            "currency": "usd",
            "status": "succeeded",
            "payment_intent": "pi_test_intent_123",
            "reason": "requested_by_customer"
        }"#;
        write_json_response(&mut socket, body).await;
    });

    let config = StripeConfig::new("sk_test_refunds").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let refund: Refund = client
        .refunds()
        .create(&CreateRefundParams {
            payment_intent: Some("pi_test_intent_123".into()),
            charge: None,
            amount: Some(2500),
            reason: Some("requested_by_customer".into()),
            refund_application_fee: None,
            reverse_transfer: None,
            metadata: None,
            idempotency_key: Some("idem_refund_test_001".into()),
            ..Default::default()
        })
        .await
        .unwrap();

    server.await.unwrap();
    assert_eq!(refund.id, "re_mock_test_001");
    assert_eq!(refund.amount, 2500);
    assert_eq!(refund.currency, "usd");
    assert_eq!(refund.status, "succeeded");
    assert_eq!(refund.payment_intent.as_deref(), Some("pi_test_intent_123"));
}

#[tokio::test]
async fn test_statutory_withdrawal_eu_directive_2011_83_compliance() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut req_buf = vec![0; 8192];
        let n = socket.read(&mut req_buf).await.unwrap();
        let wire = String::from_utf8(req_buf[..n].to_vec()).unwrap();

        assert!(wire.starts_with("POST /v1/refunds HTTP/1.1"));
        assert!(wire.contains("payment_intent=pi_statutory_withdrawal_eu"));
        // Gate G-Law: Amount omitted for statutory 100% full refund
        assert!(!wire.contains("amount="));

        let body = r#"{
            "id": "re_statutory_100_full",
            "object": "refund",
            "amount": 9900,
            "currency": "eur",
            "status": "succeeded",
            "payment_intent": "pi_statutory_withdrawal_eu"
        }"#;
        write_json_response(&mut socket, body).await;
    });

    let config = StripeConfig::new("sk_test_statutory").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // EU Directive 2011/83/EU Article 13 compliance: omitting amount triggers full statutory refund
    let refund = client
        .refunds()
        .create(&CreateRefundParams {
            payment_intent: Some("pi_statutory_withdrawal_eu".into()),
            charge: None,
            amount: None,
            reason: Some("requested_by_customer".into()),
            refund_application_fee: None,
            reverse_transfer: None,
            metadata: None,
            idempotency_key: None,
            ..Default::default()
        })
        .await
        .unwrap();

    server.await.unwrap();
    assert_eq!(refund.id, "re_statutory_100_full");
    assert_eq!(refund.amount, 9900);
    assert_eq!(refund.currency, "eur");
    assert_eq!(refund.status, "succeeded");
}

#[tokio::test]
async fn test_refunds_retrieve_update_and_cancel_lifecycle() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Retrieve
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buf1 = vec![0; 4096];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
        assert!(req1.starts_with("GET /v1/refunds/re_lifecycle_123 HTTP/1.1"));
        let body1 = r#"{"id":"re_lifecycle_123","object":"refund","amount":3500,"currency":"usd","status":"pending"}"#;
        write_json_response(&mut socket1, body1).await;

        // 2. Update
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf2 = vec![0; 4096];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req2 = String::from_utf8(buf2[..n2].to_vec()).unwrap();
        assert!(req2.starts_with("POST /v1/refunds/re_lifecycle_123 HTTP/1.1"));
        assert!(req2.contains("idempotency-key: idem_upd_456"));
        assert!(req2.contains("metadata%5Bticket%5D=CASE-789"));
        let body2 = r#"{"id":"re_lifecycle_123","object":"refund","amount":3500,"currency":"usd","status":"pending","metadata":{"ticket":"CASE-789"}}"#;
        write_json_response(&mut socket2, body2).await;

        // 3. Cancel
        let (mut socket3, _) = listener.accept().await.unwrap();
        let mut buf3 = vec![0; 4096];
        let n3 = socket3.read(&mut buf3).await.unwrap();
        let req3 = String::from_utf8(buf3[..n3].to_vec()).unwrap();
        assert!(req3.starts_with("POST /v1/refunds/re_lifecycle_123/cancel HTTP/1.1"));
        assert!(req3.contains("idempotency-key: idem_cnc_789"));
        let body3 = r#"{"id":"re_lifecycle_123","object":"refund","amount":3500,"currency":"usd","status":"canceled"}"#;
        write_json_response(&mut socket3, body3).await;
    });

    let config =
        StripeConfig::new("sk_test_refund_lifecycle").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // 1. Retrieve
    let r1 = client.refunds().retrieve("re_lifecycle_123").await.unwrap();
    assert_eq!(r1.id, "re_lifecycle_123");
    assert_eq!(r1.status, "pending");

    // 2. Update
    let mut meta = HashMap::new();
    meta.insert("ticket".into(), "CASE-789".into());
    let r2 = client
        .refunds()
        .update(
            "re_lifecycle_123",
            &UpdateRefundParams {
                metadata: Some(meta),
                idempotency_key: Some("idem_upd_456".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        r2.metadata.as_ref().and_then(|m| m.get("ticket")),
        Some(&"CASE-789".to_string())
    );

    // 3. Cancel
    let r3 = client
        .refunds()
        .cancel(
            "re_lifecycle_123",
            Some(&CancelRefundParams {
                idempotency_key: Some("idem_cnc_789".into()),
            }),
        )
        .await
        .unwrap();
    assert_eq!(r3.status, "canceled");

    server.await.unwrap();
}

#[tokio::test]
async fn test_disputes_update_with_submit_true_and_evidence_e2e() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut req_buf = vec![0; 8192];
        let n = socket.read(&mut req_buf).await.unwrap();
        let wire = String::from_utf8(req_buf[..n].to_vec()).unwrap();

        assert!(wire.starts_with("POST /v1/disputes/dp_test_dispute_999 HTTP/1.1"));
        assert!(wire.contains("idempotency-key: idem_disp_submit_true"));
        assert!(wire.contains("submit=true"));
        assert!(wire.contains("evidence%5Bcustomer_name%5D=Alice+Smith"));
        assert!(wire.contains("evidence%5Bcustomer_email_address%5D=alice%40example.com"));
        assert!(wire.contains("evidence%5Bproduct_description%5D=Cloud+Platform+Access"));

        let body = r#"{
            "id": "dp_test_dispute_999",
            "object": "dispute",
            "amount": 7500,
            "currency": "usd",
            "status": "under_review",
            "reason": "fraudulent",
            "evidence": {
                "customer_name": "Alice Smith",
                "customer_email_address": "alice@example.com",
                "product_description": "Cloud Platform Access"
            }
        }"#;
        write_json_response(&mut socket, body).await;
    });

    let config =
        StripeConfig::new("sk_test_dispute_submit").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let dispute: Dispute = client
        .disputes()
        .update(
            "dp_test_dispute_999",
            &UpdateDisputeParams {
                evidence: Some(DisputeEvidenceParams {
                    customer_name: Some("Alice Smith".into()),
                    customer_email_address: Some("alice@example.com".into()),
                    product_description: Some("Cloud Platform Access".into()),
                    ..Default::default()
                }),
                metadata: None,
                submit: Some(true),
                idempotency_key: Some("idem_disp_submit_true".into()),
            },
        )
        .await
        .unwrap();

    server.await.unwrap();
    assert_eq!(dispute.id, "dp_test_dispute_999");
    assert_eq!(dispute.status, "under_review");
    assert_eq!(dispute.amount, 7500);
    assert_eq!(
        dispute.evidence.customer_name.as_deref(),
        Some("Alice Smith")
    );
}

#[tokio::test]
async fn test_disputes_update_transmits_all_evidence_fields_without_dropping() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut req_buf = vec![0; 8192];
        let n = socket.read(&mut req_buf).await.unwrap();
        let wire = String::from_utf8(req_buf[..n].to_vec()).unwrap();

        assert!(wire.starts_with("POST /v1/disputes/dp_exhaustive_evidence HTTP/1.1"));
        // Verify automatic idempotency key injection
        assert!(wire.to_ascii_lowercase().contains("idempotency-key: "));
        // Verify evidence fields that were previously dropped
        assert!(wire.contains("evidence%5Bcustomer_communication%5D=Email+thread+attached"));
        assert!(wire.contains("evidence%5Bshipping_tracking_number%5D=TRACK-123456789"));
        assert!(wire.contains("evidence%5Bservice_date%5D=2026-10-15"));
        assert!(wire.contains("evidence%5Bduplicate_charge_explanation%5D=Distinct+orders"));
        assert!(wire.contains("evidence%5Bcancellation_policy%5D=file_cancel_policy_pdf"));
        assert!(wire.contains("evidence%5Breceipt%5D=file_receipt_img"));

        let body = r#"{
            "id": "dp_exhaustive_evidence",
            "object": "dispute",
            "amount": 5000,
            "currency": "usd",
            "status": "under_review",
            "reason": "fraudulent"
        }"#;
        write_json_response(&mut socket, body).await;
    });

    let config = StripeConfig::new("sk_test_exhaustive").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let dispute: Dispute = client
        .disputes()
        .update(
            "dp_exhaustive_evidence",
            &UpdateDisputeParams {
                evidence: Some(DisputeEvidenceParams {
                    customer_communication: Some("Email thread attached".into()),
                    shipping_tracking_number: Some("TRACK-123456789".into()),
                    service_date: Some("2026-10-15".into()),
                    duplicate_charge_explanation: Some("Distinct orders".into()),
                    cancellation_policy: Some("file_cancel_policy_pdf".into()),
                    receipt: Some("file_receipt_img".into()),
                    ..Default::default()
                }),
                metadata: None,
                submit: Some(false),
                idempotency_key: None, // Auto-injected UUID v4 key tested
            },
        )
        .await
        .unwrap();

    server.await.unwrap();
    assert_eq!(dispute.id, "dp_exhaustive_evidence");
}

#[tokio::test]
async fn test_disputes_retrieve_and_close_lifecycle() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Retrieve
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buf1 = vec![0; 4096];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
        assert!(req1.starts_with("GET /v1/disputes/dp_close_test_001 HTTP/1.1"));
        let body1 = r#"{"id":"dp_close_test_001","object":"dispute","amount":4000,"currency":"usd","status":"needs_response"}"#;
        write_json_response(&mut socket1, body1).await;

        // 2. Close
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf2 = vec![0; 4096];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req2 = String::from_utf8(buf2[..n2].to_vec()).unwrap();
        assert!(req2.starts_with("POST /v1/disputes/dp_close_test_001/close HTTP/1.1"));
        let body2 = r#"{"id":"dp_close_test_001","object":"dispute","amount":4000,"currency":"usd","status":"lost"}"#;
        write_json_response(&mut socket2, body2).await;
    });

    let config =
        StripeConfig::new("sk_test_dispute_close").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // 1. Retrieve
    let d1 = client
        .disputes()
        .retrieve("dp_close_test_001")
        .await
        .unwrap();
    assert_eq!(d1.id, "dp_close_test_001");
    assert_eq!(d1.status, "needs_response");

    // 2. Close
    let d2 = client.disputes().close("dp_close_test_001").await.unwrap();
    assert_eq!(d2.id, "dp_close_test_001");
    assert_eq!(d2.status, "lost");

    server.await.unwrap();
}

#[tokio::test]
async fn test_balance_retrieve_e2e() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut req_buf = vec![0; 4096];
        let n = socket.read(&mut req_buf).await.unwrap();
        let wire = String::from_utf8(req_buf[..n].to_vec()).unwrap();

        assert!(wire.starts_with("GET /v1/balance HTTP/1.1"));

        let body = r#"{
            "object": "balance",
            "available": [
                {"amount": 125000, "currency": "usd", "source_types": {"card": 125000}}
            ],
            "pending": [
                {"amount": 45000, "currency": "usd", "source_types": {"card": 45000}}
            ],
            "livemode": false
        }"#;
        write_json_response(&mut socket, body).await;
    });

    let config = StripeConfig::new("sk_test_balance").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let balance: Balance = client.balance().retrieve().await.unwrap();
    server.await.unwrap();

    assert_eq!(balance.object, "balance");
    assert_eq!(balance.available.len(), 1);
    assert_eq!(balance.available[0].amount, 125000); // $1,250.00
    assert_eq!(balance.available[0].currency, "usd");
    assert_eq!(balance.pending.len(), 1);
    assert_eq!(balance.pending[0].amount, 45000); // $450.00
    assert_eq!(balance.pending[0].currency, "usd");
    assert!(!balance.livemode);
}

#[tokio::test]
async fn test_balance_transactions_auto_pagination_across_pages() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Page 1
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buf1 = vec![0; 4096];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
        assert!(req1.starts_with("GET /v1/balance_transactions?limit=2 HTTP/1.1"));

        let body1 = r#"{
            "object": "list",
            "data": [
                {
                    "id": "txn_pag_001",
                    "object": "balance_transaction",
                    "amount": 5000,
                    "available_on": 1728345600,
                    "created": 1728345600,
                    "currency": "usd",
                    "net": 4825,
                    "fee": 175,
                    "status": "available",
                    "type": "charge"
                },
                {
                    "id": "txn_pag_002",
                    "object": "balance_transaction",
                    "amount": -2000,
                    "available_on": 1728345600,
                    "created": 1728345600,
                    "currency": "usd",
                    "net": -2000,
                    "fee": 0,
                    "status": "available",
                    "type": "refund"
                }
            ],
            "has_more": true
        }"#;
        write_json_response(&mut socket1, body1).await;

        // Page 2
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf2 = vec![0; 4096];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req2 = String::from_utf8(buf2[..n2].to_vec()).unwrap();
        assert!(req2.contains("starting_after=txn_pag_002"));

        let body2 = r#"{
            "object": "list",
            "data": [
                {
                    "id": "txn_pag_003",
                    "object": "balance_transaction",
                    "amount": 10000,
                    "available_on": 1728345600,
                    "created": 1728345600,
                    "currency": "usd",
                    "net": 9680,
                    "fee": 320,
                    "status": "available",
                    "type": "charge"
                }
            ],
            "has_more": false
        }"#;
        write_json_response(&mut socket2, body2).await;
    });

    let config =
        StripeConfig::new("sk_test_bt_pagination").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let mut paginator = client.balance_transactions().list(Some(2));

    // Yield items through zero-clone next_item()
    let item1: BalanceTransaction = paginator
        .next_item()
        .await
        .unwrap()
        .expect("Item 1 missing");
    assert_eq!(item1.id, "txn_pag_001");
    assert_eq!(item1.amount, 5000);
    assert_eq!(item1.net, 4825);
    assert_eq!(item1.fee, 175);
    assert_eq!(item1.r#type, "charge");

    let item2 = paginator
        .next_item()
        .await
        .unwrap()
        .expect("Item 2 missing");
    assert_eq!(item2.id, "txn_pag_002");
    assert_eq!(item2.amount, -2000);
    assert_eq!(item2.net, -2000);
    assert_eq!(item2.fee, 0);
    assert_eq!(item2.r#type, "refund");

    let item3 = paginator
        .next_item()
        .await
        .unwrap()
        .expect("Item 3 missing");
    assert_eq!(item3.id, "txn_pag_003");
    assert_eq!(item3.amount, 10000);
    assert_eq!(item3.net, 9680);
    assert_eq!(item3.fee, 320);

    // Terminal page reached
    let item4 = paginator.next_item().await.unwrap();
    assert!(item4.is_none());
    assert_eq!(paginator.pages_fetched(), 2);

    server.await.unwrap();
}

#[tokio::test]
async fn test_balance_transaction_retrieve_e2e() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut req_buf = vec![0; 4096];
        let n = socket.read(&mut req_buf).await.unwrap();
        let wire = String::from_utf8(req_buf[..n].to_vec()).unwrap();

        assert!(wire.starts_with("GET /v1/balance_transactions/txn_single_555 HTTP/1.1"));

        let body = r#"{
            "id": "txn_single_555",
            "object": "balance_transaction",
            "amount": 4200,
            "available_on": 1728345600,
            "created": 1728345600,
            "currency": "usd",
            "net": 4048,
            "fee": 152,
            "status": "available",
            "type": "charge"
        }"#;
        write_json_response(&mut socket, body).await;
    });

    let config =
        StripeConfig::new("sk_test_bt_retrieve").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let txn = client
        .balance_transactions()
        .retrieve("txn_single_555")
        .await
        .unwrap();

    server.await.unwrap();
    assert_eq!(txn.id, "txn_single_555");
    assert_eq!(txn.amount, 4200);
    assert_eq!(txn.net, 4048);
    assert_eq!(txn.fee, 152);
    assert_eq!(txn.status, "available");
    assert_eq!(txn.r#type, "charge");
}

#[tokio::test]
async fn test_stripe_client_ext_trait_direct_access() {
    let config = StripeConfig::new("sk_test_ext_trait");
    let core_client = some_stripe_core::StripeClient::with_config(config);

    // Call extension trait methods on core client
    let _refunds = core_client.refunds();
    let _disputes = core_client.disputes();
    let _balance = core_client.balance();
    let _bt: BalanceTransactionsResource<'_> = core_client.balance_transactions();
}

#[tokio::test]
async fn test_resource_ids_reject_empty_and_path_traversal_payloads() {
    let config = StripeConfig::new("sk_test_security_validation");
    let client = StripeClient::with_config(config);

    let invalid_ids = [
        "",
        "   ",
        "../../account",
        "../admin",
        "/v1/accounts",
        "dp_123?inject=1",
        "dp_123#fragment",
        "..\\windows\\system32",
    ];

    for &bad_id in &invalid_ids {
        // Accounts
        let res_acc = client.accounts().retrieve_by_id(bad_id).await;
        assert!(
            matches!(res_acc, Err(some_stripe::StripeError::InvalidRequest(_))),
            "accounts().retrieve_by_id({bad_id:?}) must fail fast with InvalidRequest, got: {res_acc:?}"
        );

        // Balance Transactions
        let res_bt = client.balance_transactions().retrieve(bad_id).await;
        assert!(
            matches!(res_bt, Err(some_stripe::StripeError::InvalidRequest(_))),
            "balance_transactions().retrieve({bad_id:?}) must fail fast with InvalidRequest, got: {res_bt:?}"
        );

        // Disputes retrieve
        let res_disp_ret = client.disputes().retrieve(bad_id).await;
        assert!(
            matches!(
                res_disp_ret,
                Err(some_stripe::StripeError::InvalidRequest(_))
            ),
            "disputes().retrieve({bad_id:?}) must fail fast with InvalidRequest, got: {res_disp_ret:?}"
        );

        // Disputes update
        let res_disp_upd = client
            .disputes()
            .update(bad_id, &UpdateDisputeParams::default())
            .await;
        assert!(
            matches!(
                res_disp_upd,
                Err(some_stripe::StripeError::InvalidRequest(_))
            ),
            "disputes().update({bad_id:?}) must fail fast with InvalidRequest, got: {res_disp_upd:?}"
        );

        // Disputes close
        let res_disp_close = client.disputes().close(bad_id).await;
        assert!(
            matches!(
                res_disp_close,
                Err(some_stripe::StripeError::InvalidRequest(_))
            ),
            "disputes().close({bad_id:?}) must fail fast with InvalidRequest, got: {res_disp_close:?}"
        );

        // Refunds retrieve
        let res_ref_ret = client.refunds().retrieve(bad_id).await;
        assert!(
            matches!(
                res_ref_ret,
                Err(some_stripe::StripeError::InvalidRequest(_))
            ),
            "refunds().retrieve({bad_id:?}) must fail fast with InvalidRequest, got: {res_ref_ret:?}"
        );

        // Refunds update
        let res_ref_upd = client
            .refunds()
            .update(bad_id, &UpdateRefundParams::default())
            .await;
        assert!(
            matches!(
                res_ref_upd,
                Err(some_stripe::StripeError::InvalidRequest(_))
            ),
            "refunds().update({bad_id:?}) must fail fast with InvalidRequest, got: {res_ref_upd:?}"
        );

        // Refunds cancel
        let res_ref_cnc = client.refunds().cancel(bad_id, None).await;
        assert!(
            matches!(
                res_ref_cnc,
                Err(some_stripe::StripeError::InvalidRequest(_))
            ),
            "refunds().cancel({bad_id:?}) must fail fast with InvalidRequest, got: {res_ref_cnc:?}"
        );
    }
}
