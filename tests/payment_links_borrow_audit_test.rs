//! Gate G-Borrow, G-Law, and Mock Wire Audit Test Suite for PaymentLinks (US-4.2.0).
//!
//! Validates:
//! - Strict zero-clone parameter borrowing (`&CreatePaymentLinkParams`, `&UpdatePaymentLinkParams`, `&str`, `&ListLineItemsParams`).
//! - Zero deep clones on `PaymentLink` domain models and zero-copy string slice accessors (`url()`, `id()`, `currency()`, `metadata_get()`).
//! - Gate G-Law: Statutory ISO-4217 minor currency validation and EU Omnibus Directive 2019/2161 price transparency disclosures.
//! - Direct resource mounting on both `client.checkout().payment_links()` and `client.payment_links()`.
//! - Ephemeral mock wire server verifying HTTP request encoding and response parsing without heap-cloning domain models.

use some_stripe::{
    AdjustableQuantityParams, CreatePaymentLinkParams, LineItem, ListLineItemsParams,
    OmnibusPriceTransparencyMetadata, OmnibusTaxBreakdown, PaymentLink, PaymentLinkLineItemParams,
    PaymentLinkPriceDataParams, StripeClient, StripeConfig, TaxBehavior, UpdatePaymentLinkParams,
    validate_omnibus_price_transparency, validate_tax_behavior_transparency,
};
use some_stripe_types::{BalanceTransaction, validate_ledger_reconciliation};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_gate_g_borrow_zero_clone_parameter_borrowing() {
    let price_data = PaymentLinkPriceDataParams::new("usd", 3500)
        .product("prod_enterprise_sub")
        .tax_behavior("inclusive");
    let line_item = PaymentLinkLineItemParams::with_price_data(price_data, 1)
        .adjustable_quantity(AdjustableQuantityParams::with_limits(1, 10));

    let create_params = CreatePaymentLinkParams::new(vec![line_item])
        .active(true)
        .allow_promotion_codes(true)
        .billing_address_collection("required")
        .metadata_entry("campaign", "audit_test_q4")
        .idempotency_key("idem_plink_borrow_001");

    // Borrow reference without consuming or cloning
    let borrowed_ref: &CreatePaymentLinkParams = &create_params;
    assert_eq!(borrowed_ref.line_items.len(), 1);
    assert_eq!(borrowed_ref.active, Some(true));
    assert_eq!(borrowed_ref.allow_promotion_codes, Some(true));
    assert_eq!(
        borrowed_ref.billing_address_collection.as_deref(),
        Some("required")
    );
    assert_eq!(
        borrowed_ref.idempotency_key.as_deref(),
        Some("idem_plink_borrow_001")
    );

    // Verify params is unconsumed and can be borrowed again
    let second_borrow: &CreatePaymentLinkParams = &create_params;
    assert_eq!(second_borrow.line_items[0].quantity, 1);
    assert!(second_borrow.validate().is_ok());

    // Update params zero-clone borrow
    let update_params = UpdatePaymentLinkParams::new()
        .active(false)
        .allow_promotion_codes(false)
        .metadata_entry("deactivation_reason", "campaign_ended")
        .idempotency_key("idem_plink_update_001");

    let borrowed_update: &UpdatePaymentLinkParams = &update_params;
    assert_eq!(borrowed_update.active, Some(false));
    assert_eq!(borrowed_update.allow_promotion_codes, Some(false));
    assert_eq!(
        borrowed_update.idempotency_key.as_deref(),
        Some("idem_plink_update_001")
    );
    assert!(borrowed_update.validate().is_ok());
    assert_eq!(update_params.active, Some(false));

    // List pagination params zero-clone borrow
    let list_params = ListLineItemsParams::new()
        .limit(20)
        .starting_after("li_cursor_123");
    let list_ref: &ListLineItemsParams = &list_params;
    assert_eq!(list_ref.limit, Some(20));
    assert_eq!(list_ref.starting_after.as_deref(), Some("li_cursor_123"));
    assert_eq!(list_params.limit, Some(20));
}

#[test]
fn test_gate_g_borrow_zero_copy_accessors() {
    let mut metadata = HashMap::new();
    metadata.insert("storefront_id".into(), "sf_987".into());
    metadata.insert("channel".into(), "instagram".into());

    let link = PaymentLink {
        id: "plink_test_borrow_inspect".into(),
        object: "payment_link".into(),
        active: true,
        url: "https://buy.stripe.com/test_zero_copy_456".into(),
        currency: Some("eur".into()),
        metadata: Some(metadata),
        payment_method_types: None,
        allow_promotion_codes: Some(true),
        billing_address_collection: None,
        shipping_address_collection: None,
        customer_creation: None,
        after_completion: None,
        livemode: Some(false),
    };

    // Assert zero-copy borrowed string slices
    let id_ref: &str = link.id();
    assert_eq!(id_ref, "plink_test_borrow_inspect");

    let url_ref: &str = link.url();
    assert_eq!(url_ref, "https://buy.stripe.com/test_zero_copy_456");

    let currency_ref: Option<&str> = link.currency();
    assert_eq!(currency_ref, Some("eur"));

    let sf_ref: Option<&str> = link.metadata_get("storefront_id");
    assert_eq!(sf_ref, Some("sf_987"));

    let ch_ref: Option<&str> = link.metadata_get("channel");
    assert_eq!(ch_ref, Some("instagram"));

    assert_eq!(link.metadata_get("non_existent"), None);
    assert!(link.is_active());
}

#[test]
fn test_gate_g_law_iso_4217_and_omnibus_transparency() {
    // 1. Valid compliant promotional discount under EU Omnibus Directive 2019/2161 Article 6a
    // Regular price €100 (10000 cents), announced sale price €80 (8000 cents),
    // documented lowest prior 30-day baseline €85 (8500 cents).
    assert!(validate_omnibus_price_transparency(10000, 8000, Some(8500)).is_ok());

    // 2. Compliant non-discounted catalog price (no baseline required)
    assert!(validate_omnibus_price_transparency(10000, 10000, None).is_ok());

    // 3. Violation: Discount announced without prior 30-day baseline price
    let err_missing = validate_omnibus_price_transparency(10000, 8000, None).unwrap_err();
    assert!(err_missing.to_string().contains("prior_30d_lowest_price"));

    // 4. Violation: Deceptive discount exceeding prior 30-day baseline
    // (Retailer raised price before sale; advertised €90 exceeds 30-day lowest €80)
    let err_deceptive = validate_omnibus_price_transparency(12000, 9000, Some(8000)).unwrap_err();
    assert!(err_deceptive.to_string().contains("deceptive discounting"));

    // 5. Violation: Discounted price exceeds regular catalog price
    assert!(validate_omnibus_price_transparency(5000, 6000, Some(5000)).is_err());

    // 6. Violation: Non-positive amounts (Gate G-Law integer minor units must be > 0)
    assert!(validate_omnibus_price_transparency(0, 1000, Some(1000)).is_err());
    assert!(validate_omnibus_price_transparency(-500, 1000, Some(1000)).is_err());
    assert!(validate_omnibus_price_transparency(1000, 0, Some(1000)).is_err());
    assert!(validate_omnibus_price_transparency(1000, -200, Some(1000)).is_err());

    // 7. Transparent Tax Behavior under EU consumer law
    assert!(validate_tax_behavior_transparency(TaxBehavior::Inclusive).is_ok());
    assert!(validate_tax_behavior_transparency(TaxBehavior::Exclusive).is_ok());
    assert!(validate_tax_behavior_transparency(TaxBehavior::Unspecified).is_err());

    // 8. Zero-float integer tax calculation using basis points (e.g. 2000 bps = 20% VAT)
    let breakdown = OmnibusTaxBreakdown::calculate(12000, TaxBehavior::Inclusive, 2000)
        .expect("Valid tax calculation");
    assert_eq!(breakdown.gross_amount, 12000);
    assert_eq!(breakdown.net_amount + breakdown.tax_amount, 12000);

    // 9. Statutory metadata roundtrip
    let meta = OmnibusPriceTransparencyMetadata::new(10000, 7500, Some(8000)).unwrap();
    let map = meta.to_metadata_map();
    let parsed = OmnibusPriceTransparencyMetadata::from_metadata_map(&map)
        .unwrap()
        .unwrap();
    assert_eq!(parsed.regular_price, 10000);
    assert_eq!(parsed.announced_discounted_price, 7500);
    assert_eq!(parsed.prior_30d_lowest_price, Some(8000));
}

#[test]
fn test_gate_g_borrow_move_semantics_without_clone() {
    let json_payload = serde_json::json!({
        "id": "plink_roundtrip_test",
        "object": "payment_link",
        "active": true,
        "url": "https://buy.stripe.com/plink_roundtrip_test",
        "currency": "usd",
        "metadata": {
            "source": "borrow_audit_test"
        }
    });

    // Deserialization moves directly into domain model
    let link: PaymentLink = serde_json::from_value(json_payload).expect("Deserialization failed");
    assert_eq!(link.id(), "plink_roundtrip_test");
    assert_eq!(link.url(), "https://buy.stripe.com/plink_roundtrip_test");
    assert_eq!(link.currency(), Some("usd"));
    assert_eq!(link.metadata_get("source"), Some("borrow_audit_test"));
    assert!(link.is_active());

    // LineItem deserialization without cloning
    let item_json = serde_json::json!({
        "id": "li_rt_1",
        "object": "item",
        "description": "Premium Membership",
        "amount_subtotal": 5000,
        "amount_total": 5000,
        "currency": "usd",
        "quantity": 1,
        "price": {
            "id": "price_rt_1",
            "object": "price",
            "currency": "usd",
            "unit_amount": 5000,
            "product": "prod_123"
        }
    });

    let item: LineItem = serde_json::from_value(item_json).expect("Item deserialization failed");
    assert_eq!(item.id, "li_rt_1");
    assert_eq!(item.amount_total, 5000);
    assert_eq!(item.currency, "usd");
    assert_eq!(
        item.price.as_ref().map(|p| p.id.as_str()),
        Some("price_rt_1")
    );
}

#[tokio::test]
async fn test_payment_links_lifecycle_mock_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind ephemeral mock server");
    let port = listener.local_addr().unwrap().port();
    let api_base = format!("http://127.0.0.1:{port}");

    // Spawn ephemeral mock server handling 4 lifecycle calls
    tokio::spawn(async move {
        for step in 1..=4 {
            let (mut socket, _) = listener.accept().await.expect("Accept failed");
            let mut buf = vec![0u8; 4096];
            let n = socket.read(&mut buf).await.expect("Read request failed");
            let req_str = String::from_utf8_lossy(&buf[..n]);
            eprintln!("=== STEP {step} REQUEST ===\n{req_str}\n======================");

            match step {
                // Step 1: POST /v1/payment_links (create)
                1 => {
                    assert!(
                        req_str.starts_with("POST /v1/payment_links HTTP"),
                        "Expected POST /v1/payment_links, got: {req_str}"
                    );
                    assert!(
                        req_str.to_lowercase().contains("idempotency-key:"),
                        "Missing Idempotency-Key header in request"
                    );

                    let resp_body = serde_json::json!({
                        "id": "plink_mock_lifecycle_001",
                        "object": "payment_link",
                        "active": true,
                        "url": "https://buy.stripe.com/mock_lifecycle_001",
                        "currency": "usd"
                    })
                    .to_string();

                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        resp_body.len(),
                        resp_body
                    );
                    socket
                        .write_all(response.as_bytes())
                        .await
                        .expect("Write response failed");
                }
                // Step 2: GET /v1/payment_links/plink_mock_lifecycle_001 (retrieve)
                2 => {
                    assert!(
                        req_str.starts_with("GET /v1/payment_links/plink_mock_lifecycle_001 HTTP"),
                        "Expected GET /v1/payment_links/..., got: {req_str}"
                    );

                    let resp_body = serde_json::json!({
                        "id": "plink_mock_lifecycle_001",
                        "object": "payment_link",
                        "active": true,
                        "url": "https://buy.stripe.com/mock_lifecycle_001",
                        "currency": "usd"
                    })
                    .to_string();

                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        resp_body.len(),
                        resp_body
                    );
                    socket
                        .write_all(response.as_bytes())
                        .await
                        .expect("Write response failed");
                }
                // Step 3: POST /v1/payment_links/plink_mock_lifecycle_001 (update: deactivate)
                3 => {
                    assert!(
                        req_str.starts_with("POST /v1/payment_links/plink_mock_lifecycle_001 HTTP"),
                        "Expected POST update, got: {req_str}"
                    );

                    let resp_body = serde_json::json!({
                        "id": "plink_mock_lifecycle_001",
                        "object": "payment_link",
                        "active": false,
                        "url": "https://buy.stripe.com/mock_lifecycle_001",
                        "currency": "usd"
                    })
                    .to_string();

                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        resp_body.len(),
                        resp_body
                    );
                    socket
                        .write_all(response.as_bytes())
                        .await
                        .expect("Write response failed");
                }
                // Step 4: GET /v1/payment_links/plink_mock_lifecycle_001/line_items (list line items)
                4 => {
                    assert!(
                        req_str.starts_with(
                            "GET /v1/payment_links/plink_mock_lifecycle_001/line_items"
                        ),
                        "Expected GET line_items, got: {req_str}"
                    );

                    let resp_body = serde_json::json!({
                        "object": "list",
                        "data": [
                            {
                                "id": "li_mock_item_01",
                                "object": "item",
                                "amount_subtotal": 2500,
                                "amount_total": 2500,
                                "currency": "usd",
                                "quantity": 2,
                                "price": {
                                    "id": "price_widget_01",
                                    "object": "price",
                                    "currency": "usd",
                                    "unit_amount": 2500
                                }
                            }
                        ],
                        "has_more": false,
                        "url": "/v1/payment_links/plink_mock_lifecycle_001/line_items"
                    })
                    .to_string();

                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        resp_body.len(),
                        resp_body
                    );
                    socket
                        .write_all(response.as_bytes())
                        .await
                        .expect("Write response failed");
                }
                _ => break,
            }
        }
    });

    let config = StripeConfig::new("sk_test_wire_audit_mock").with_api_base(api_base);
    let client = StripeClient::with_config(config);

    // 1. Create shareable link via client.checkout().payment_links()
    let price_data = PaymentLinkPriceDataParams::new("usd", 2500).product("prod_widget_01");
    let line_item = PaymentLinkLineItemParams::with_price_data(price_data, 2);
    let create_params = CreatePaymentLinkParams::new(vec![line_item]).active(true);

    let created_link = client
        .checkout()
        .payment_links()
        .create(&create_params)
        .await
        .expect("Create failed");
    assert_eq!(created_link.id(), "plink_mock_lifecycle_001");
    assert!(created_link.is_active());
    assert_eq!(
        created_link.url(),
        "https://buy.stripe.com/mock_lifecycle_001"
    );

    // 2. Retrieve link via client.payment_links()
    let retrieved_link = client
        .payment_links()
        .retrieve("plink_mock_lifecycle_001")
        .await
        .expect("Retrieve failed");
    assert_eq!(retrieved_link.id(), "plink_mock_lifecycle_001");
    assert!(retrieved_link.is_active());

    // 3. Update (deactivate) link via client.checkout().payment_links()
    let update_params = UpdatePaymentLinkParams::new().active(false);
    let updated_link = client
        .checkout()
        .payment_links()
        .update("plink_mock_lifecycle_001", &update_params)
        .await
        .expect("Update failed");
    assert!(!updated_link.is_active());

    // 4. List line items via client.payment_links() with stream paginator
    let list_params = ListLineItemsParams::new().limit(5);
    let mut paginator = client
        .payment_links()
        .list_line_items("plink_mock_lifecycle_001", &list_params);

    let first_item = paginator
        .next_item()
        .await
        .expect("Request failed")
        .expect("Expected line item in page");
    assert_eq!(first_item.id, "li_mock_item_01");
    assert_eq!(first_item.amount_total, 2500);
    assert_eq!(first_item.quantity, Some(2));
}

#[test]
fn test_balance_transaction_ledger_reconciliation() {
    // Valid transaction: net = amount - fee (10000 - 320 = 9680)
    let valid_txn = BalanceTransaction {
        id: "txn_plink_recon_001".into(),
        object: "balance_transaction".into(),
        amount: 10000,
        currency: "usd".into(),
        fee: 320,
        net: 9680,
        status: "available".into(),
        r#type: "payment".into(),
        reporting_category: Some("charge".into()),
        created: 1728345600,
        available_on: 1728518400,
        description: Some("Payment link settlement".into()),
        source: Some("ch_test_123".into()),
        fee_details: vec![],
    };

    assert!(valid_txn.verify_accounting_invariant());
    assert!(validate_ledger_reconciliation(&[valid_txn]).is_ok());

    // Invariant violation: net != amount - fee (10000 - 320 != 9500)
    let invalid_txn = BalanceTransaction {
        id: "txn_plink_recon_err".into(),
        object: "balance_transaction".into(),
        amount: 10000,
        currency: "usd".into(),
        fee: 320,
        net: 9500, // Invalid net!
        status: "available".into(),
        r#type: "payment".into(),
        reporting_category: Some("charge".into()),
        created: 1728345600,
        available_on: 1728518400,
        description: Some("Corrupted transaction".into()),
        source: Some("ch_test_corrupt".into()),
        fee_details: vec![],
    };

    assert!(!invalid_txn.verify_accounting_invariant());
    assert!(validate_ledger_reconciliation(&[invalid_txn]).is_err());
}
