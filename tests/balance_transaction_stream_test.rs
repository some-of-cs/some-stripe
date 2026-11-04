//! Integration test verifying `BalanceTransaction` stream pagination with `Paginator`.
//!
//! Validates:
//! - Seamless integration of `BalanceTransaction` with `Paginator` using `balance_transaction_id`.
//! - Zero-clone stream traversal yielding individual `BalanceTransaction` entries across multiple pages.
//! - Invariant enforcement on every streamed balance transaction and full ledger reconciliation.

use some_stripe::{BalanceTransaction, balance_transaction_id, validate_ledger_reconciliation};
use some_stripe_core::client::{StripeClient, StripeConfig};
use some_stripe_core::patterns::pagination::Paginator;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_balance_transaction_stream_traversal_3_pages() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // --- Page 1 ---
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buf1 = vec![0; 4096];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
        assert!(req1.starts_with("GET /v1/balance_transactions?limit=2 HTTP/1.1"));

        let page1_json = serde_json::json!({
            "object": "list",
            "data": [
                {
                    "id": "txn_stream_001",
                    "object": "balance_transaction",
                    "amount": 5000,
                    "available_on": 1728345600,
                    "created": 1728345600,
                    "currency": "usd",
                    "description": "Payment for order #1",
                    "fee": 175,
                    "fee_details": [
                        {
                            "amount": 175,
                            "currency": "usd",
                            "description": "Stripe processing fee",
                            "type": "stripe_fee",
                            "application": null
                        }
                    ],
                    "net": 4825,
                    "reporting_category": "charge",
                    "source": "ch_stream_001",
                    "status": "available",
                    "type": "charge"
                },
                {
                    "id": "txn_stream_002",
                    "object": "balance_transaction",
                    "amount": 10000,
                    "available_on": 1728345660,
                    "created": 1728345660,
                    "currency": "usd",
                    "description": "Payment for order #2",
                    "fee": 320,
                    "fee_details": [
                        {
                            "amount": 320,
                            "currency": "usd",
                            "description": "Stripe processing fee",
                            "type": "stripe_fee",
                            "application": null
                        }
                    ],
                    "net": 9680,
                    "reporting_category": "charge",
                    "source": "ch_stream_002",
                    "status": "available",
                    "type": "charge"
                }
            ],
            "has_more": true
        });
        let body1 = serde_json::to_string(&page1_json).unwrap();
        socket1
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body1}",
                    body1.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();

        // --- Page 2 ---
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf2 = vec![0; 4096];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req2 = String::from_utf8(buf2[..n2].to_vec()).unwrap();
        assert!(req2.contains("starting_after=txn_stream_002"));

        let page2_json = serde_json::json!({
            "object": "list",
            "data": [
                {
                    "id": "txn_stream_003",
                    "object": "balance_transaction",
                    "amount": -2500,
                    "available_on": 1728345720,
                    "created": 1728345720,
                    "currency": "usd",
                    "description": "Refund for item return",
                    "fee": 0,
                    "fee_details": [],
                    "net": -2500,
                    "reporting_category": "refund",
                    "source": "re_stream_003",
                    "status": "available",
                    "type": "refund"
                },
                {
                    "id": "txn_stream_004",
                    "object": "balance_transaction",
                    "amount": 7500,
                    "available_on": 1728345780,
                    "created": 1728345780,
                    "currency": "usd",
                    "description": "Payment for order #4",
                    "fee": 248,
                    "fee_details": [
                        {
                            "amount": 248,
                            "currency": "usd",
                            "type": "stripe_fee"
                        }
                    ],
                    "net": 7252,
                    "reporting_category": "charge",
                    "source": "ch_stream_004",
                    "status": "available",
                    "type": "charge"
                }
            ],
            "has_more": true
        });
        let body2 = serde_json::to_string(&page2_json).unwrap();
        socket2
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body2}",
                    body2.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();

        // --- Page 3 (Final Page) ---
        let (mut socket3, _) = listener.accept().await.unwrap();
        let mut buf3 = vec![0; 4096];
        let n3 = socket3.read(&mut buf3).await.unwrap();
        let req3 = String::from_utf8(buf3[..n3].to_vec()).unwrap();
        assert!(req3.contains("starting_after=txn_stream_004"));

        let page3_json = serde_json::json!({
            "object": "list",
            "data": [
                {
                    "id": "txn_stream_005",
                    "object": "balance_transaction",
                    "amount": -1500,
                    "available_on": 1728345840,
                    "created": 1728345840,
                    "currency": "usd",
                    "description": "Dispute administrative inquiry fee",
                    "fee": 0,
                    "fee_details": [],
                    "net": -1500,
                    "reporting_category": "dispute",
                    "source": "dp_stream_005",
                    "status": "available",
                    "type": "adjustment"
                }
            ],
            "has_more": false
        });
        let body3 = serde_json::to_string(&page3_json).unwrap();
        socket3
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body3}",
                    body3.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config =
        StripeConfig::new("sk_test_ledger_stream").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // Initialize Paginator passing balance_transaction_id as id_extractor
    let mut paginator: Paginator<BalanceTransaction> = Paginator::new(
        &client,
        "v1/balance_transactions",
        2,
        balance_transaction_id,
    );

    let mut collected: Vec<BalanceTransaction> = Vec::new();
    while let Some(txn) = paginator.next_item().await.unwrap() {
        // Assert individual transaction accounting invariant
        assert!(
            txn.verify_accounting_invariant(),
            "Transaction {} violated invariant",
            txn.id
        );
        collected.push(txn);
    }

    // Verify stream traversal counts and IDs
    assert_eq!(collected.len(), 5);
    assert_eq!(collected[0].id, "txn_stream_001");
    assert_eq!(collected[1].id, "txn_stream_002");
    assert_eq!(collected[2].id, "txn_stream_003");
    assert_eq!(collected[3].id, "txn_stream_004");
    assert_eq!(collected[4].id, "txn_stream_005");

    assert_eq!(paginator.pages_fetched(), 3);

    // Verify full ledger reconciliation over all collected transactions
    let reconciliation = validate_ledger_reconciliation(&collected);
    assert!(
        reconciliation.is_ok(),
        "Ledger failed to reconcile: {:?}",
        reconciliation.err()
    );

    // Compute expected totals:
    // Amount: 5000 + 10000 - 2500 + 7500 - 1500 = 18500
    // Fee:    175  + 320   + 0     + 248  + 0     = 743
    // Net:    4825 + 9680  - 2500 + 7252 - 1500  = 17757
    // Invariant check: 18500 - 743 = 17757.
    let total_amount: i64 = collected.iter().map(|t| t.amount).sum();
    let total_fee: i64 = collected.iter().map(|t| t.fee).sum();
    let total_net: i64 = collected.iter().map(|t| t.net).sum();
    assert_eq!(total_amount, 18500);
    assert_eq!(total_fee, 743);
    assert_eq!(total_net, 17757);
    assert_eq!(total_net, total_amount - total_fee);

    server.await.unwrap();
}
