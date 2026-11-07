//! Integration and wire tests for Issuing Authorizations and Transactions resources (US-7.2.0 Unit 2).
//!
//! Verifies:
//! - Full HTTP wire contracts for `retrieve`, `update`, `approve`, `decline`, and `list` endpoints
//! - Header injection (`Idempotency-Key`, `Stripe-Account`)
//! - Form parameter serialization with bracket notation for metadata
//! - Bank Secrecy Act (BSA) & Anti-Money Laundering (AML) reporting thresholds
//! - PCI-DSS 4.0 zero PAN leakage verification
//! - Path traversal rejection on ID validation
//! - Rust 2024 zero-clone borrowing discipline

use some_stripe::{
    ApproveAuthorizationParams, DeclineAuthorizationParams, IssuingAuthorizationMethod,
    IssuingAuthorizationStatus, IssuingTransactionType, IssuingVelocityTracker,
    ListIssuingAuthorizationsParams, ListIssuingTransactionsParams, StripeClient, StripeConfig,
    UpdateIssuingAuthorizationParams, UpdateIssuingTransactionParams, is_bsa_ctr_reportable,
    is_bsa_sar_threshold_exceeded, validate_issuing_transaction_amount, validate_zero_pan_leakage,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn write_json_response(socket: &mut tokio::net::TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    socket.write_all(response.as_bytes()).await.unwrap();
}

#[tokio::test]
async fn test_issuing_authorizations_full_lifecycle() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let auth_resp = r#"{
            "id": "iauth_1001",
            "object": "issuing.authorization",
            "amount": 7500,
            "approved": true,
            "authorization_method": "chip",
            "card": "ic_card_111",
            "cardholder": "ich_holder_222",
            "created": 1728345600,
            "currency": "usd",
            "livemode": false,
            "merchant_amount": 7500,
            "merchant_currency": "usd",
            "merchant_data": {
                "name": "Supermarket Acme",
                "category": "grocery_stores",
                "category_code": "5411",
                "city": "Seattle",
                "country": "US",
                "postal_code": "98101"
            },
            "status": "closed",
            "transactions": ["ipi_3001"]
        }"#;

        let mut buf = vec![0; 8192];

        // 1. Retrieve Authorization
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/issuing/authorizations/iauth_1001 HTTP/1.1"));
        assert!(
            wire.to_lowercase()
                .contains("stripe-account: acct_platform_999")
        );
        write_json_response(&mut socket, auth_resp).await;

        // 2. Update Authorization
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/issuing/authorizations/iauth_1001 HTTP/1.1"));
        assert!(
            wire.contains("metadata%5Bdepartment%5D=procurement")
                || wire.contains("metadata[department]=procurement")
        );
        assert!(
            wire.to_lowercase()
                .contains("idempotency-key: custom_idem_auth_update")
        );
        write_json_response(&mut socket, auth_resp).await;

        // 3. Approve Authorization
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/issuing/authorizations/iauth_1001/approve HTTP/1.1"));
        assert!(wire.contains("amount=7500"));
        assert!(
            wire.to_lowercase()
                .contains("idempotency-key: custom_idem_approve")
        );
        write_json_response(&mut socket, auth_resp).await;

        // 4. Decline Authorization
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/issuing/authorizations/iauth_1001/decline HTTP/1.1"));
        assert!(
            wire.contains("metadata%5Breason%5D=velocity_limit")
                || wire.contains("metadata[reason]=velocity_limit")
        );
        assert!(
            wire.to_lowercase()
                .contains("idempotency-key: custom_idem_decline")
        );
        write_json_response(&mut socket, auth_resp).await;

        // 5. List Authorizations
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/issuing/authorizations?"));
        assert!(wire.contains("card=ic_card_111"));
        assert!(wire.contains("status=closed"));
        assert!(wire.contains("limit=10"));

        let list_resp = format!(
            r#"{{
                "object": "list",
                "data": [{auth_resp}],
                "has_more": false,
                "url": "/v1/issuing/authorizations"
            }}"#
        );
        write_json_response(&mut socket, &list_resp).await;
    });

    let config =
        StripeConfig::new("sk_test_auth_wire").with_api_base(format!("http://{}", address));
    let client = StripeClient::with_config(config);
    let resource = client
        .issuing_authorizations()
        .with_account("acct_platform_999");

    // 1. Retrieve
    let retrieved = resource.retrieve("iauth_1001").await.unwrap();
    assert_eq!(retrieved.id, "iauth_1001");
    assert_eq!(retrieved.amount, 7500);
    assert_eq!(retrieved.card.id(), "ic_card_111");
    assert_eq!(retrieved.status, IssuingAuthorizationStatus::Closed);
    assert_eq!(
        retrieved.authorization_method,
        Some(IssuingAuthorizationMethod::Chip)
    );

    // 2. Update
    let update_params = UpdateIssuingAuthorizationParams::new()
        .with_metadata_entry("department", "procurement")
        .with_idempotency_key("custom_idem_auth_update");
    let updated = resource.update("iauth_1001", &update_params).await.unwrap();
    assert_eq!(updated.id, "iauth_1001");

    // 3. Approve
    let approve_params = ApproveAuthorizationParams::new()
        .with_amount(7500)
        .with_idempotency_key("custom_idem_approve");
    let approved = resource
        .approve("iauth_1001", Some(&approve_params))
        .await
        .unwrap();
    assert!(approved.approved);

    // 4. Decline
    let decline_params = DeclineAuthorizationParams::new()
        .with_metadata_entry("reason", "velocity_limit")
        .with_idempotency_key("custom_idem_decline");
    let declined = resource
        .decline("iauth_1001", Some(&decline_params))
        .await
        .unwrap();
    assert_eq!(declined.id, "iauth_1001");

    // 5. List
    let list_params = ListIssuingAuthorizationsParams::new()
        .with_card("ic_card_111")
        .with_status(IssuingAuthorizationStatus::Closed)
        .with_limit(10);
    let list = resource.list(Some(&list_params)).await.unwrap();
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id, "iauth_1001");
    assert!(!list.has_more);

    server.await.unwrap();
}

#[tokio::test]
async fn test_issuing_transactions_full_lifecycle() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let txn_resp = r#"{
            "id": "ipi_2002",
            "object": "issuing.transaction",
            "amount": 3200,
            "amount_details": {
                "atm_fee": null,
                "cashback_amount": null
            },
            "authorization": "iauth_1001",
            "balance_transaction": "txn_bal_4004",
            "card": "ic_card_111",
            "cardholder": "ich_holder_222",
            "created": 1728345700,
            "currency": "usd",
            "livemode": false,
            "merchant_amount": 3200,
            "merchant_currency": "usd",
            "merchant_data": {
                "name": "Electronics Shop",
                "category": "electronics_stores",
                "category_code": "5732",
                "city": "Portland",
                "country": "US",
                "postal_code": "97201"
            },
            "type": "capture",
            "wallet": "apple_pay"
        }"#;

        let mut buf = vec![0; 8192];

        // 1. Retrieve Transaction
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/issuing/transactions/ipi_2002 HTTP/1.1"));
        assert!(
            wire.to_lowercase()
                .contains("stripe-account: acct_connect_888")
        );
        write_json_response(&mut socket, txn_resp).await;

        // 2. Update Transaction
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/issuing/transactions/ipi_2002 HTTP/1.1"));
        assert!(
            wire.contains("metadata%5Btax_code%5D=txcd_9999")
                || wire.contains("metadata[tax_code]=txcd_9999")
        );
        assert!(
            wire.to_lowercase()
                .contains("idempotency-key: custom_idem_txn_upd")
        );
        write_json_response(&mut socket, txn_resp).await;

        // 3. List Transactions
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/issuing/transactions?"));
        assert!(wire.contains("card=ic_card_111"));
        assert!(wire.contains("type=capture"));
        assert!(wire.contains("limit=5"));

        let list_resp = format!(
            r#"{{
                "object": "list",
                "data": [{txn_resp}],
                "has_more": false,
                "url": "/v1/issuing/transactions"
            }}"#
        );
        write_json_response(&mut socket, &list_resp).await;
    });

    let config =
        StripeConfig::new("sk_test_auth_wire").with_api_base(format!("http://{}", address));
    let client = StripeClient::with_config(config);
    let resource = client
        .issuing_transactions()
        .with_account("acct_connect_888");

    // 1. Retrieve
    let txn = resource.retrieve("ipi_2002").await.unwrap();
    assert_eq!(txn.id, "ipi_2002");
    assert_eq!(txn.amount, 3200);
    assert_eq!(txn.r#type, IssuingTransactionType::Capture);
    assert_eq!(txn.authorization.as_deref(), Some("iauth_1001"));
    assert_eq!(txn.card, "ic_card_111");

    // 2. Update
    let update_params = UpdateIssuingTransactionParams::new()
        .with_metadata_entry("tax_code", "txcd_9999")
        .with_idempotency_key("custom_idem_txn_upd");
    let updated = resource.update("ipi_2002", &update_params).await.unwrap();
    assert_eq!(updated.id, "ipi_2002");

    // 3. List
    let list_params = ListIssuingTransactionsParams::new()
        .with_card("ic_card_111")
        .with_type(IssuingTransactionType::Capture)
        .with_limit(5);
    let list = resource.list(Some(&list_params)).await.unwrap();
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id, "ipi_2002");

    server.await.unwrap();
}

#[tokio::test]
async fn test_path_traversal_validation() {
    let client = StripeClient::new("sk_test_123");
    let auth_res = client.issuing_authorizations();
    let txn_res = client.issuing_transactions();

    // Invalid characters and path traversal attempts
    assert!(auth_res.retrieve("../iauth_secret").await.is_err());
    assert!(auth_res.retrieve("iauth/123").await.is_err());
    assert!(auth_res.retrieve("iauth?evil=1").await.is_err());
    assert!(auth_res.retrieve("   ").await.is_err());

    assert!(txn_res.retrieve("../ipi_secret").await.is_err());
    assert!(txn_res.retrieve("ipi/123").await.is_err());
    assert!(txn_res.retrieve("ipi#tag").await.is_err());
    assert!(txn_res.retrieve("").await.is_err());
}

#[test]
fn test_bsa_aml_and_security_invariants() {
    // 1. Bank Secrecy Act CTR threshold: $10,000 (1,000,000 cents)
    assert!(is_bsa_ctr_reportable(1_000_000));
    assert!(is_bsa_ctr_reportable(2_500_000));
    assert!(!is_bsa_ctr_reportable(999_999));

    // 2. BSA SAR threshold: $5,000 (500,000 cents)
    assert!(is_bsa_sar_threshold_exceeded(500_000));
    assert!(is_bsa_sar_threshold_exceeded(800_000));
    assert!(!is_bsa_sar_threshold_exceeded(499_999));

    // 3. Zero PAN Leakage: PCI-DSS 4.0
    assert!(validate_zero_pan_leakage("Normal invoice note").is_ok());
    // Luhn valid test PAN: 4111 1111 1111 1111
    assert!(validate_zero_pan_leakage("Card number 4111111111111111 used").is_err());
    assert!(validate_zero_pan_leakage("Card number 4111-1111-1111-1111 used").is_err());

    // 4. Ledger Amount Consistency
    assert!(validate_issuing_transaction_amount(100, IssuingTransactionType::Capture).is_ok());
    assert!(validate_issuing_transaction_amount(100, IssuingTransactionType::Refund).is_ok());
    assert!(validate_issuing_transaction_amount(0, IssuingTransactionType::Capture).is_err());
    assert!(validate_issuing_transaction_amount(-50, IssuingTransactionType::Refund).is_err());

    // 5. Velocity Tracker
    let mut tracker = IssuingVelocityTracker::new(50_000); // $500.00
    assert!(tracker.evaluate_authorization(20_000).is_ok());
    tracker.record_approved(20_000).unwrap();
    assert_eq!(tracker.accumulated_amount, 20_000);

    assert!(tracker.evaluate_authorization(30_000).is_ok());
    tracker.record_approved(30_000).unwrap();
    assert_eq!(tracker.accumulated_amount, 50_000);

    // Over limit
    assert!(tracker.evaluate_authorization(100).is_err());
    assert!(tracker.record_approved(100).is_err());
    assert_eq!(tracker.accumulated_amount, 50_000);
}
