//! Integration and wire tests for Stripe Tax Transactions resource and ledger domain invariants (Unit 2).

use some_stripe::{
    CreateTaxTransactionFromCalculationParams, CreateTaxTransactionReversalParams,
    ListTaxTransactionLineItemsParams, ReversalBalanceAdjustment, ReversalBalanceError,
    ReversalLineItemParams, ReversalMode, ReversalShippingCostParams, StripeClient, StripeConfig,
    TAX_CALCULATION_EXPIRATION_SECONDS, TaxTransactionType, calculate_reversal_balance_adjustment,
    is_tax_calculation_expired, is_tax_calculation_valid, is_valid_uuid_v4,
    tax_transaction_line_item_id,
};
use std::collections::HashMap;
use std::str::FromStr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_tax_transaction_types_and_reversal_modes() {
    let t_type = TaxTransactionType::from_str("transaction").unwrap();
    assert_eq!(t_type, TaxTransactionType::Transaction);
    assert_eq!(t_type.as_str(), "transaction");
    assert_eq!(t_type.to_string(), "transaction");

    let r_type = TaxTransactionType::from_str("reversal").unwrap();
    assert_eq!(r_type, TaxTransactionType::Reversal);
    assert_eq!(r_type.as_str(), "reversal");

    let invalid = TaxTransactionType::from_str("unknown_type");
    assert!(invalid.is_err());

    let mode_full = ReversalMode::Full;
    assert_eq!(mode_full.as_str(), "full");
    assert_eq!(mode_full.to_string(), "full");

    let mode_partial = ReversalMode::Partial;
    assert_eq!(mode_partial.as_str(), "partial");
    assert_eq!(mode_partial.to_string(), "partial");
}

#[test]
fn test_48_hour_calculation_expiration_lifecycle() {
    let calculation_time = 1_700_000_000;

    // Immediately after creation
    assert!(is_tax_calculation_valid(calculation_time, calculation_time));
    assert!(!is_tax_calculation_expired(
        calculation_time,
        calculation_time
    ));

    // 24 hours later
    let at_24h = calculation_time + 24 * 3600;
    assert!(is_tax_calculation_valid(calculation_time, at_24h));
    assert!(!is_tax_calculation_expired(calculation_time, at_24h));

    // Exactly at 48 hours (172,800 seconds)
    let at_48h = calculation_time + TAX_CALCULATION_EXPIRATION_SECONDS;
    assert!(is_tax_calculation_valid(calculation_time, at_48h));
    assert!(!is_tax_calculation_expired(calculation_time, at_48h));

    // 1 second past 48 hours (expired)
    let past_48h = at_48h + 1;
    assert!(!is_tax_calculation_valid(calculation_time, past_48h));
    assert!(is_tax_calculation_expired(calculation_time, past_48h));

    // Negative drift / future timestamp relative to calculation
    let antedated = calculation_time - 1;
    assert!(!is_tax_calculation_valid(calculation_time, antedated));
    assert!(is_tax_calculation_expired(calculation_time, antedated));
}

#[test]
fn test_reversal_balance_adjustment_invariants() {
    // 1. Exact full reversal calculation
    let full = calculate_reversal_balance_adjustment(25000, 2063, 25000, 2063).unwrap();
    assert_eq!(
        full,
        ReversalBalanceAdjustment {
            original_amount: 25000,
            original_tax: 2063,
            reversed_amount: 25000,
            reversed_tax: 2063,
            remaining_amount: 0,
            remaining_tax: 0,
        }
    );

    // 2. Exact partial reversal calculation
    let partial = calculate_reversal_balance_adjustment(25000, 2063, 10000, 825).unwrap();
    assert_eq!(
        partial,
        ReversalBalanceAdjustment {
            original_amount: 25000,
            original_tax: 2063,
            reversed_amount: 10000,
            reversed_tax: 825,
            remaining_amount: 15000,
            remaining_tax: 1238,
        }
    );

    // 3. Error: Reversed gross amount exceeds original
    let err_gross = calculate_reversal_balance_adjustment(1000, 80, 1001, 80).unwrap_err();
    assert!(matches!(
        err_gross,
        ReversalBalanceError::AmountExceedsOriginal {
            original: 1000,
            reversed: 1001
        }
    ));

    // 4. Error: Reversed tax exceeds original tax
    let err_tax = calculate_reversal_balance_adjustment(1000, 80, 500, 85).unwrap_err();
    assert!(matches!(
        err_tax,
        ReversalBalanceError::TaxExceedsOriginal {
            original: 80,
            reversed: 85
        }
    ));

    // 5. Error: Negative inputs
    assert_eq!(
        calculate_reversal_balance_adjustment(-100, 10, 50, 5).unwrap_err(),
        ReversalBalanceError::InvalidOriginalAmounts
    );
    assert_eq!(
        calculate_reversal_balance_adjustment(100, -10, 50, 5).unwrap_err(),
        ReversalBalanceError::InvalidOriginalAmounts
    );
    assert_eq!(
        calculate_reversal_balance_adjustment(100, 10, -50, 5).unwrap_err(),
        ReversalBalanceError::InvalidReversalMagnitude
    );
    assert_eq!(
        calculate_reversal_balance_adjustment(100, 10, 50, -5).unwrap_err(),
        ReversalBalanceError::InvalidReversalMagnitude
    );
}

#[tokio::test]
async fn test_create_tax_transaction_from_calculation_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("POST /v1/tax/transactions/create_from_calculation HTTP/1.1"));
        assert!(req_str.contains("calculation=taxcalc_wire_test_123"));
        assert!(req_str.contains("reference=order_tx_wire_999"));
        assert!(req_str.contains("posted_at=1735689600"));
        assert!(req_str.contains("metadata%5Benv%5D=production_test"));

        let idemp_header = req_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
            .map(|l| l.split(':').nth(1).unwrap().trim())
            .expect("Idempotency-Key header missing");
        assert_eq!(idemp_header, "idem_custom_tax_key_777");

        let body = r#"{
            "id": "tax_tx_wire_res_101",
            "object": "tax.transaction",
            "created": 1735689600,
            "currency": "usd",
            "customer": "cus_wire_tax_1",
            "customer_details": {
                "address": {
                    "city": "San Francisco",
                    "country": "US",
                    "line1": "123 Market St",
                    "postal_code": "94103",
                    "state": "CA"
                },
                "address_source": "shipping",
                "ip_address": "127.0.0.1",
                "taxability_override": "none"
            },
            "livemode": false,
            "metadata": {
                "env": "production_test"
            },
            "posted_at": 1735689600,
            "reference": "order_tx_wire_999",
            "reversal": null,
            "shipping_cost": {
                "amount": 750,
                "amount_tax": 62,
                "tax_behavior": "exclusive",
                "tax_code": "txcd_92010001"
            },
            "tax_date": 1735689600,
            "type": "transaction"
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);

    let mut metadata = HashMap::new();
    metadata.insert("env".into(), "production_test".into());

    let params = CreateTaxTransactionFromCalculationParams::new(
        "taxcalc_wire_test_123",
        "order_tx_wire_999",
    )
    .with_posted_at(1735689600)
    .with_metadata(metadata)
    .with_idempotency_key("idem_custom_tax_key_777");

    let tx = client
        .tax_transactions()
        .create_from_calculation(&params)
        .await
        .expect("create_from_calculation failed");

    assert_eq!(tx.id, "tax_tx_wire_res_101");
    assert_eq!(tx.reference, "order_tx_wire_999");
    assert_eq!(tx.currency, "usd");
    assert!(tx.is_standard_transaction());
    assert!(!tx.is_reversal());

    let shipping = tx.shipping_cost.unwrap();
    assert_eq!(shipping.amount, 750);
    assert_eq!(shipping.amount_tax, 62);

    server.await.unwrap();
}

#[tokio::test]
async fn test_create_tax_transaction_reversal_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("POST /v1/tax/transactions/create_reversal HTTP/1.1"));
        assert!(req_str.contains("original_transaction=tax_tx_wire_res_101"));
        assert!(req_str.contains("mode=partial"));
        assert!(req_str.contains("reference=refund_tx_wire_999"));
        assert!(req_str.contains("flat_amount=5000"));
        assert!(req_str.contains("line_items%5B0%5D%5Boriginal_line_item%5D=tax_li_1"));
        assert!(req_str.contains("line_items%5B0%5D%5Bamount%5D=5000"));
        assert!(req_str.contains("line_items%5B0%5D%5Bamount_tax%5D=413"));
        assert!(req_str.contains("shipping_cost%5Bamount%5D=250"));
        assert!(req_str.contains("shipping_cost%5Bamount_tax%5D=21"));

        // Auto-generated UUID v4 idempotency key when none provided
        let idemp_header = req_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
            .map(|l| l.split(':').nth(1).unwrap().trim())
            .expect("Idempotency-Key header missing");
        assert!(is_valid_uuid_v4(idemp_header));

        let body = r#"{
            "id": "tax_tx_rev_wire_202",
            "object": "tax.transaction",
            "created": 1735691000,
            "currency": "usd",
            "customer": null,
            "customer_details": null,
            "livemode": false,
            "metadata": null,
            "posted_at": 1735691000,
            "reference": "refund_tx_wire_999",
            "reversal": {
                "original_transaction": "tax_tx_wire_res_101",
                "mode": "partial"
            },
            "shipping_cost": {
                "amount": 250,
                "amount_tax": 21,
                "tax_behavior": "exclusive",
                "tax_code": "txcd_92010001"
            },
            "tax_date": 1735691000,
            "type": "reversal"
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);

    let line = ReversalLineItemParams {
        amount: 5000,
        amount_tax: 413,
        original_line_item: "tax_li_1".into(),
        reference: "sku_refund_1".into(),
        metadata: None,
    };

    let shipping = ReversalShippingCostParams {
        amount: 250,
        amount_tax: 21,
    };

    let params =
        CreateTaxTransactionReversalParams::partial("tax_tx_wire_res_101", "refund_tx_wire_999")
            .with_flat_amount(5000)
            .with_line_items(vec![line])
            .with_shipping_cost(shipping);

    let rev = client
        .tax_transactions()
        .create_reversal(&params)
        .await
        .expect("create_reversal failed");

    assert_eq!(rev.id, "tax_tx_rev_wire_202");
    assert!(rev.is_reversal());
    assert!(!rev.is_standard_transaction());
    let rev_details = rev.reversal.unwrap();
    assert_eq!(
        rev_details.original_transaction.as_deref(),
        Some("tax_tx_wire_res_101")
    );
    assert_eq!(rev_details.mode.as_deref(), Some("partial"));

    server.await.unwrap();
}

#[tokio::test]
async fn test_retrieve_tax_transaction_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("GET /v1/tax/transactions/tax_tx_fetch_303 HTTP/1.1"));

        let body = r#"{
            "id": "tax_tx_fetch_303",
            "object": "tax.transaction",
            "created": 1735689600,
            "currency": "usd",
            "livemode": false,
            "posted_at": 1735689600,
            "reference": "ref_fetch_303",
            "tax_date": 1735689600,
            "type": "transaction"
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);

    let tx = client
        .tax_transactions()
        .retrieve("tax_tx_fetch_303")
        .await
        .expect("retrieve failed");

    assert_eq!(tx.id, "tax_tx_fetch_303");
    assert_eq!(tx.reference, "ref_fetch_303");
    assert!(tx.is_standard_transaction());

    server.await.unwrap();
}

#[tokio::test]
async fn test_list_tax_transaction_line_items_pagination() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        // Page 1
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(
            req_str.starts_with(
                "GET /v1/tax/transactions/tax_tx_paged_404/line_items?limit=2 HTTP/1.1"
            )
        );

        let body1 = r#"{
            "object": "list",
            "data": [
                {
                    "id": "tax_li_p1_item1",
                    "object": "tax.transaction_line_item",
                    "amount": 2000,
                    "amount_tax": 165,
                    "livemode": false,
                    "reference": "sku_p1_1",
                    "type": "transaction"
                },
                {
                    "id": "tax_li_p1_item2",
                    "object": "tax.transaction_line_item",
                    "amount": 3000,
                    "amount_tax": 248,
                    "livemode": false,
                    "reference": "sku_p1_2",
                    "type": "transaction"
                }
            ],
            "has_more": true
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body1.len(),
            body1
        );
        socket.write_all(response.as_bytes()).await.unwrap();

        // Page 2
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf2 = vec![0; 4096];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req_str2 = String::from_utf8_lossy(&buf2[..n2]);

        assert!(req_str2.contains("starting_after=tax_li_p1_item2"));

        let body2 = r#"{
            "object": "list",
            "data": [
                {
                    "id": "tax_li_p2_item3",
                    "object": "tax.transaction_line_item",
                    "amount": 5000,
                    "amount_tax": 413,
                    "livemode": false,
                    "reference": "sku_p2_3",
                    "type": "transaction"
                }
            ],
            "has_more": false
        }"#;

        let response2 = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body2.len(),
            body2
        );
        socket2.write_all(response2.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);

    let params = ListTaxTransactionLineItemsParams::new().with_limit(2);
    let mut paginator = client
        .tax_transactions()
        .list_line_items("tax_tx_paged_404", &params);

    let item1 = paginator
        .next_item()
        .await
        .unwrap()
        .expect("item 1 missing");
    assert_eq!(item1.id, "tax_li_p1_item1");
    assert_eq!(tax_transaction_line_item_id(&item1), "tax_li_p1_item1");

    let item2 = paginator
        .next_item()
        .await
        .unwrap()
        .expect("item 2 missing");
    assert_eq!(item2.id, "tax_li_p1_item2");

    let item3 = paginator
        .next_item()
        .await
        .unwrap()
        .expect("item 3 missing");
    assert_eq!(item3.id, "tax_li_p2_item3");

    let end = paginator.next_item().await.unwrap();
    assert!(end.is_none());

    server.await.unwrap();
}

#[tokio::test]
async fn test_parameter_validation_guards() {
    let client = StripeClient::new("sk_test_mock");
    let resource = client.tax_transactions();

    // Empty calculation
    let p_calc = CreateTaxTransactionFromCalculationParams::new("", "ref_123");
    let err = resource.create_from_calculation(&p_calc).await.unwrap_err();
    assert!(err.to_string().contains("Calculation ID"));

    // Empty reference
    let p_ref = CreateTaxTransactionFromCalculationParams::new("taxcalc_1", "   ");
    let err = resource.create_from_calculation(&p_ref).await.unwrap_err();
    assert!(err.to_string().contains("Transaction reference"));

    // Empty original transaction for reversal
    let r_tx = CreateTaxTransactionReversalParams::full("", "rev_ref");
    let err = resource.create_reversal(&r_tx).await.unwrap_err();
    assert!(err.to_string().contains("Original transaction ID"));

    // Empty reversal reference
    let r_ref = CreateTaxTransactionReversalParams::full("tax_tx_1", "  ");
    let err = resource.create_reversal(&r_ref).await.unwrap_err();
    assert!(err.to_string().contains("Reversal reference"));

    // Invalid path traversal ID on retrieve
    let err = resource.retrieve("tax_tx/../hack").await.unwrap_err();
    assert!(err.to_string().to_lowercase().contains("path traversal"));
}
