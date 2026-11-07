//! Integration and wire tests for Stripe Treasury Outbound Transfers & Outbound Payments (US-7.2.0 Unit 4).
//!
//! Verifies:
//! - [AC-US-7.2.0-02]: Outbound wire transfer from Treasury Financial Account with balance verification.
//! - Parameter serialization to Stripe API v1 form format.
//! - Idempotency key generation and header propagation.
//! - Multi-tenant isolation header (`Stripe-Account`).
//! - OutboundTransfer, OutboundPayment, and ReceivedCredit endpoints.
//! - Zero PAN leakage client-side guard.

use some_stripe::{
    CancelOutboundPaymentParams, CancelOutboundTransferParams, CreateOutboundPaymentParams,
    CreateOutboundTransferParams, ListOutboundPaymentsParams, ListOutboundTransfersParams,
    ListReceivedCreditsParams, OutboundPaymentStatus, OutboundTransferStatus, ReceivedCreditStatus,
    StripeClient, StripeConfig,
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
async fn test_ac_scenario_outbound_wire_transfer_with_balance_verification() {
    // AC-US-7.2.0-02:
    // Given a Treasury Financial Account "fa_123" with available balance $10,000.00 (1,000,000 cents)
    // When client.treasury().outbound_transfers().create(&CreateOutboundTransferParams {
    //      financial_account: "fa_123".into(),
    //      amount: 250000,
    //      currency: "usd".into(),
    //      destination_payment_method: Some("pm_bank_wire_123".into()),
    //      ..Default::default()
    // }) is executed
    // Then an OutboundTransfer "obt_123" is created with status "posted"
    // And the financial account balance is debited by $2,500.00 (to $7,500.00).

    let mut available_balance = 1_000_000; // $10,000.00

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(wire.starts_with("POST /v1/treasury/outbound_transfers HTTP/1.1"));
        assert!(wire.contains("amount=250000"));
        assert!(wire.contains("currency=usd"));
        assert!(wire.contains("financial_account=fa_123"));
        assert!(wire.contains("destination_payment_method=pm_bank_wire_123"));
        assert!(wire.to_lowercase().contains("idempotency-key: "));

        let resp = r#"{
            "id": "obt_123",
            "object": "treasury.outbound_transfer",
            "amount": 250000,
            "cancelable": false,
            "created": 1728345600,
            "currency": "usd",
            "destination_payment_method": "pm_bank_wire_123",
            "expected_arrival_date": 1728349200,
            "financial_account": "fa_123",
            "livemode": false,
            "status": "posted",
            "status_transitions": {
                "posted_at": 1728345600
            }
        }"#;
        write_json_response(&mut socket, resp).await;
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let create_params = CreateOutboundTransferParams::new("fa_123", 250000, "usd")
        .with_destination_payment_method("pm_bank_wire_123")
        .with_idempotency_key("idem_wire_wire_001");

    // Pre-flight balance verification
    assert!(create_params.verify_balance(available_balance).is_ok());

    let transfer = client
        .treasury_transfers()
        .outbound_transfers()
        .create(&create_params)
        .await
        .unwrap();

    server.await.unwrap();

    assert_eq!(transfer.id, "obt_123");
    assert_eq!(transfer.status, OutboundTransferStatus::Posted);
    assert_eq!(transfer.amount, 250000);
    assert_eq!(transfer.currency, "usd");
    assert_eq!(transfer.financial_account, "fa_123");

    // Debit financial account balance by transfer amount
    available_balance -= transfer.amount;
    assert_eq!(available_balance, 750000); // $7,500.00

    // Verify next transfer exceeding $7,500 is rejected locally
    let excessive_params = CreateOutboundTransferParams::new("fa_123", 800000, "usd");
    assert!(excessive_params.verify_balance(available_balance).is_err());
}

#[tokio::test]
async fn test_outbound_transfers_retrieve_cancel_list() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Retrieve OutboundTransfer
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/treasury/outbound_transfers/obt_test_999 HTTP/1.1"));

        let resp_retrieve = r#"{
            "id": "obt_test_999",
            "object": "treasury.outbound_transfer",
            "amount": 50000,
            "cancelable": true,
            "created": 1728345600,
            "currency": "usd",
            "expected_arrival_date": 1728350000,
            "financial_account": "fa_123",
            "livemode": false,
            "status": "processing",
            "status_transitions": {}
        }"#;
        write_json_response(&mut socket, resp_retrieve).await;

        // 2. Cancel OutboundTransfer
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(
            wire.starts_with("POST /v1/treasury/outbound_transfers/obt_test_999/cancel HTTP/1.1")
        );

        let resp_cancel = r#"{
            "id": "obt_test_999",
            "object": "treasury.outbound_transfer",
            "amount": 50000,
            "cancelable": false,
            "created": 1728345600,
            "currency": "usd",
            "expected_arrival_date": 1728350000,
            "financial_account": "fa_123",
            "livemode": false,
            "status": "canceled",
            "status_transitions": {
                "canceled_at": 1728346000
            }
        }"#;
        write_json_response(&mut socket, resp_cancel).await;

        // 3. List OutboundTransfers
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/treasury/outbound_transfers?"));
        assert!(wire.contains("financial_account=fa_123"));

        let resp_list = r#"{
            "object": "list",
            "data": [
                {
                    "id": "obt_test_999",
                    "object": "treasury.outbound_transfer",
                    "amount": 50000,
                    "cancelable": false,
                    "created": 1728345600,
                    "currency": "usd",
                    "expected_arrival_date": 1728350000,
                    "financial_account": "fa_123",
                    "livemode": false,
                    "status": "canceled",
                    "status_transitions": {}
                }
            ],
            "has_more": false,
            "url": "/v1/treasury/outbound_transfers"
        }"#;
        write_json_response(&mut socket, resp_list).await;
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);
    let resource = client.treasury_transfers().outbound_transfers();

    let transfer = resource.retrieve("obt_test_999").await.unwrap();
    assert_eq!(transfer.id, "obt_test_999");
    assert_eq!(transfer.status, OutboundTransferStatus::Processing);

    let canceled = resource
        .cancel(
            "obt_test_999",
            Some(&CancelOutboundTransferParams::default()),
        )
        .await
        .unwrap();
    assert_eq!(canceled.id, "obt_test_999");
    assert_eq!(canceled.status, OutboundTransferStatus::Canceled);

    let list_params = ListOutboundTransfersParams::new("fa_123").with_limit(10);
    let list = resource.list(&list_params).await.unwrap();
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id, "obt_test_999");

    server.await.unwrap();
}

#[tokio::test]
async fn test_outbound_payments_lifecycle() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Create OutboundPayment
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/treasury/outbound_payments HTTP/1.1"));
        assert!(wire.contains("amount=150000"));
        assert!(wire.contains("currency=usd"));
        assert!(wire.contains("financial_account=fa_test_p"));

        let resp_create = r#"{
            "id": "obp_test_101",
            "object": "treasury.outbound_payment",
            "amount": 150000,
            "cancelable": true,
            "created": 1728345600,
            "currency": "usd",
            "expected_arrival_date": 1728352000,
            "financial_account": "fa_test_p",
            "livemode": false,
            "status": "processing",
            "status_transitions": {}
        }"#;
        write_json_response(&mut socket, resp_create).await;

        // 2. Retrieve OutboundPayment
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/treasury/outbound_payments/obp_test_101 HTTP/1.1"));
        write_json_response(&mut socket, resp_create).await;

        // 3. Cancel OutboundPayment
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(
            wire.starts_with("POST /v1/treasury/outbound_payments/obp_test_101/cancel HTTP/1.1")
        );

        let resp_cancel = r#"{
            "id": "obp_test_101",
            "object": "treasury.outbound_payment",
            "amount": 150000,
            "cancelable": false,
            "created": 1728345600,
            "currency": "usd",
            "expected_arrival_date": 1728352000,
            "financial_account": "fa_test_p",
            "livemode": false,
            "status": "canceled",
            "status_transitions": {}
        }"#;
        write_json_response(&mut socket, resp_cancel).await;

        // 4. List OutboundPayments
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/treasury/outbound_payments?"));

        let resp_list = r#"{
            "object": "list",
            "data": [],
            "has_more": false,
            "url": "/v1/treasury/outbound_payments"
        }"#;
        write_json_response(&mut socket, resp_list).await;
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);
    let resource = client.treasury_transfers().outbound_payments();

    let params = CreateOutboundPaymentParams::new("fa_test_p", 150000, "usd")
        .with_destination_payment_method("pm_ach_target")
        .with_idempotency_key("idem_obp_create_101");

    let payment = resource.create(&params).await.unwrap();
    assert_eq!(payment.id, "obp_test_101");
    assert_eq!(payment.status, OutboundPaymentStatus::Processing);

    let retrieved = resource.retrieve("obp_test_101").await.unwrap();
    assert_eq!(retrieved.id, "obp_test_101");

    let canceled = resource
        .cancel(
            "obp_test_101",
            Some(&CancelOutboundPaymentParams::default()),
        )
        .await
        .unwrap();
    assert_eq!(canceled.status, OutboundPaymentStatus::Canceled);

    let list_params = ListOutboundPaymentsParams::new("fa_test_p");
    let list = resource.list(&list_params).await.unwrap();
    assert!(list.data.is_empty());

    server.await.unwrap();
}

#[tokio::test]
async fn test_received_credits_endpoints() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Retrieve ReceivedCredit
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/treasury/received_credits/rc_test_500 HTTP/1.1"));

        let resp_credit = r#"{
            "id": "rc_test_500",
            "object": "treasury.received_credit",
            "amount": 800000,
            "created": 1728345600,
            "currency": "usd",
            "financial_account": "fa_cred_fa",
            "livemode": false,
            "network": "us_domestic_wire",
            "status": "succeeded"
        }"#;
        write_json_response(&mut socket, resp_credit).await;

        // 2. List ReceivedCredits
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/treasury/received_credits?"));

        let resp_list = r#"{
            "object": "list",
            "data": [
                {
                    "id": "rc_test_500",
                    "object": "treasury.received_credit",
                    "amount": 800000,
                    "created": 1728345600,
                    "currency": "usd",
                    "financial_account": "fa_cred_fa",
                    "livemode": false,
                    "network": "us_domestic_wire",
                    "status": "succeeded"
                }
            ],
            "has_more": false,
            "url": "/v1/treasury/received_credits"
        }"#;
        write_json_response(&mut socket, resp_list).await;
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);
    let resource = client.treasury_transfers().received_credits();

    let credit = resource.retrieve("rc_test_500").await.unwrap();
    assert_eq!(credit.id, "rc_test_500");
    assert_eq!(credit.amount, 800000);
    assert_eq!(credit.status, ReceivedCreditStatus::Succeeded);

    let list_params = ListReceivedCreditsParams::new("fa_cred_fa");
    let list = resource.list(&list_params).await.unwrap();
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id, "rc_test_500");

    server.await.unwrap();
}

#[tokio::test]
async fn test_multi_tenant_with_account_header() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(
            wire.to_lowercase()
                .contains("stripe-account: acct_treasury_platform_1")
        );

        let resp = r#"{
            "id": "obt_scoped_1",
            "object": "treasury.outbound_transfer",
            "amount": 10000,
            "cancelable": false,
            "created": 1728345600,
            "currency": "usd",
            "expected_arrival_date": 1728350000,
            "financial_account": "fa_scoped",
            "livemode": false,
            "status": "posted",
            "status_transitions": {}
        }"#;
        write_json_response(&mut socket, resp).await;
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let scoped = client
        .treasury_transfers()
        .with_account("acct_treasury_platform_1");

    let params = CreateOutboundTransferParams::new("fa_scoped", 10000, "usd");
    let res = scoped.create_outbound_transfer(&params).await.unwrap();
    assert_eq!(res.id, "obt_scoped_1");

    server.await.unwrap();
}

#[tokio::test]
async fn test_client_validation_prevents_pan_leakage_on_wire() {
    let config = StripeConfig::new("sk_test_mock").with_api_base("http://127.0.0.1:9999");
    let client = StripeClient::with_config(config);

    // 16-digit valid Luhn card number
    let pan = "4532015112830366";
    let leaking_params = CreateOutboundTransferParams::new("fa_test", 5000, "usd")
        .with_description(format!("Payment to card {pan}"));

    // Call should fail client-side before any network I/O
    let result = client
        .treasury_transfers()
        .create_outbound_transfer(&leaking_params)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("PCI-DSS 4.0 violation"));
}
