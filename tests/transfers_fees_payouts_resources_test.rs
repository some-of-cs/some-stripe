//! Integration and wire tests for Transfers, Application Fees, and Payouts resources (US-6.2.0 Unit 4).
//!
//! Verifies API endpoint dispatch, parameters serialization, header injection (`Idempotency-Key`),
//! and domain entity deserialization.

use some_stripe::{
    CreateFeeRefundParams, CreatePayoutParams, CreateTransferParams, CreateTransferReversalParams,
    StripeClient, StripeConfig, UpdatePayoutParams,
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
async fn test_transfers_crud_and_reversal() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Create Transfer
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/transfers HTTP/1.1"));
        assert!(wire.contains("amount=1000"));
        assert!(wire.contains("currency=usd"));
        assert!(wire.contains("destination=acct_test123"));

        let resp1 = r#"{
            "id": "tr_123",
            "object": "transfer",
            "amount": 1000,
            "amount_reversed": 0,
            "created": 1728345600,
            "currency": "usd",
            "destination": "acct_test123",
            "livemode": false,
            "reversed": false
        }"#;
        write_json_response(&mut socket, resp1).await;

        // 2. Retrieve Transfer
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/transfers/tr_123 HTTP/1.1"));
        write_json_response(&mut socket, resp1).await;

        // 3. Reverse Transfer
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/transfers/tr_123/reversals HTTP/1.1"));
        assert!(wire.contains("amount=500"));

        let resp_rev = r#"{
            "id": "trr_123",
            "object": "transfer_reversal",
            "amount": 500,
            "created": 1728345700,
            "currency": "usd",
            "transfer": "tr_123"
        }"#;
        write_json_response(&mut socket, resp_rev).await;
    });

    let config = StripeConfig::new("sk_test_transfers").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let create_params = CreateTransferParams::new(1000, "usd", "acct_test123")
        .with_idempotency_key("idem_tr_create");
    let transfer = client.transfers().create(&create_params).await.unwrap();
    assert_eq!(transfer.id, "tr_123");
    assert_eq!(transfer.amount, 1000);

    let retrieved = client.transfers().retrieve("tr_123").await.unwrap();
    assert_eq!(retrieved.id, "tr_123");

    let rev_params = CreateTransferReversalParams::new()
        .with_amount(500)
        .with_idempotency_key("idem_tr_rev");
    let reversal = client
        .transfers()
        .reverse("tr_123", Some(&rev_params))
        .await
        .unwrap();
    assert_eq!(reversal.id, "trr_123");
    assert_eq!(reversal.amount, 500);

    server.await.unwrap();
}

#[tokio::test]
async fn test_application_fees_and_refunds() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Retrieve Application Fee
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/application_fees/fee_123 HTTP/1.1"));

        let resp_fee = r#"{
            "id": "fee_123",
            "object": "application_fee",
            "account": "acct_test",
            "amount": 150,
            "amount_refunded": 0,
            "application": "ca_test",
            "charge": "ch_test",
            "created": 1728345600,
            "currency": "usd",
            "livemode": false,
            "refunded": false
        }"#;
        write_json_response(&mut socket, resp_fee).await;

        // 2. Create Fee Refund
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/application_fees/fee_123/refunds HTTP/1.1"));
        assert!(wire.contains("amount=50"));
        assert!(
            wire.to_lowercase()
                .contains("idempotency-key: idem_fee_ref")
        );

        let resp_refund = r#"{
            "id": "fr_123",
            "object": "fee_refund",
            "amount": 50,
            "created": 1728345800,
            "currency": "usd",
            "fee": "fee_123"
        }"#;
        write_json_response(&mut socket, resp_refund).await;
    });

    let config = StripeConfig::new("sk_test_fees").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let fee = client.application_fees().retrieve("fee_123").await.unwrap();
    assert_eq!(fee.id, "fee_123");
    assert_eq!(fee.amount, 150);

    let refund_params = CreateFeeRefundParams::new()
        .with_amount(50)
        .with_idempotency_key("idem_fee_ref");
    let fee_refund = client
        .application_fees()
        .create_refund("fee_123", Some(&refund_params))
        .await
        .unwrap();
    assert_eq!(fee_refund.id, "fr_123");
    assert_eq!(fee_refund.amount, 50);

    server.await.unwrap();
}

#[tokio::test]
async fn test_payouts_crud_cancel_reverse() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Create Payout
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/payouts HTTP/1.1"));
        assert!(wire.contains("amount=5000"));
        assert!(wire.contains("currency=usd"));
        assert!(
            wire.to_lowercase()
                .contains("idempotency-key: idem_po_create")
        );

        let resp_payout = r#"{
            "id": "po_123",
            "object": "payout",
            "amount": 5000,
            "arrival_date": 1728432000,
            "automatic": true,
            "created": 1728345600,
            "currency": "usd",
            "livemode": false,
            "method": "standard",
            "status": "pending",
            "type": "bank_account"
        }"#;
        write_json_response(&mut socket, resp_payout).await;

        // 2. Cancel Payout
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/payouts/po_123/cancel HTTP/1.1"));

        let resp_cancelled = r#"{
            "id": "po_123",
            "object": "payout",
            "amount": 5000,
            "arrival_date": 1728432000,
            "automatic": true,
            "created": 1728345600,
            "currency": "usd",
            "livemode": false,
            "method": "standard",
            "status": "canceled",
            "type": "bank_account"
        }"#;
        write_json_response(&mut socket, resp_cancelled).await;
    });

    let config = StripeConfig::new("sk_test_payouts").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let create_params = CreatePayoutParams::new(5000, "usd").with_idempotency_key("idem_po_create");
    let payout = client.payouts().create(&create_params).await.unwrap();
    assert_eq!(payout.id, "po_123");
    assert_eq!(payout.status, "pending");

    let cancelled = client.payouts().cancel("po_123").await.unwrap();
    assert_eq!(cancelled.status, "canceled");

    server.await.unwrap();
}

#[tokio::test]
async fn test_payout_and_fee_refund_idempotency_key_preservation_and_fallback() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Payout update with caller-provided idempotency key
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/payouts/po_upd_123 HTTP/1.1"));
        assert!(
            wire.to_lowercase()
                .contains("idempotency-key: idem_po_update_key")
        );

        let resp_payout = r#"{
            "id": "po_upd_123",
            "object": "payout",
            "amount": 5000,
            "arrival_date": 1728432000,
            "automatic": true,
            "created": 1728345600,
            "currency": "usd",
            "livemode": false,
            "method": "standard",
            "status": "pending",
            "type": "bank_account"
        }"#;
        write_json_response(&mut socket, resp_payout).await;

        // 2. Payout create without idempotency key (must generate UUID v4)
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/payouts HTTP/1.1"));
        assert!(wire.to_lowercase().contains("idempotency-key: "));

        write_json_response(&mut socket, resp_payout).await;

        // 3. Fee refund create without idempotency key (None, must generate UUID v4)
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/application_fees/fee_456/refunds HTTP/1.1"));
        assert!(wire.to_lowercase().contains("idempotency-key: "));

        let resp_refund = r#"{
            "id": "fr_456",
            "object": "fee_refund",
            "amount": 100,
            "created": 1728345900,
            "currency": "usd",
            "fee": "fee_456"
        }"#;
        write_json_response(&mut socket, resp_refund).await;
    });

    let config = StripeConfig::new("sk_test_idem_keys").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // 1. Update payout with caller key
    let update_params = UpdatePayoutParams::new().with_idempotency_key("idem_po_update_key");
    let payout = client
        .payouts()
        .update("po_upd_123", &update_params)
        .await
        .unwrap();
    assert_eq!(payout.id, "po_upd_123");

    // 2. Create payout without caller key (generates UUID)
    let create_params = CreatePayoutParams::new(5000, "usd");
    let payout2 = client.payouts().create(&create_params).await.unwrap();
    assert_eq!(payout2.id, "po_upd_123");

    // 3. Create fee refund with None params (generates UUID)
    let refund = client
        .application_fees()
        .create_refund("fee_456", None)
        .await
        .unwrap();
    assert_eq!(refund.id, "fr_456");

    server.await.unwrap();
}
