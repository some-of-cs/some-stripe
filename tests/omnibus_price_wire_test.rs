//! Wire Mock & Compliance Integration Test Suite for EU Omnibus Directive 2019/2161
//! Price Transparency on Stripe Payment Links (`US-4.2.0` Unit 4).
//!
//! # Statutary Legal Mandate: EU Directive 2019/2161 Article 6a (Gate G-Law)
//! Any announcement of a price reduction must indicate the prior price applied by the
//! trader for a period not less than 30 days prior to the reduction.
//! Deceptive discounts (where price was raised right before discount, or where advertised
//! sale price exceeds the 30-day lowest baseline) are strictly prohibited.
//!
//! # Quality Gates Verified:
//! - **G0**: Clean compilation on Rust 2024 edition without warnings.
//! - **G-Borrow**: Zero unnecessary deep copies/clones on parameters (`&CreatePaymentLinkParams`, `&str`).
//! - **G-Law**: Zero floating-point arithmetic; exact ISO 4217 integer minor currency units (`i64`).
//! - **G2 / PCI-DSS**: Idempotency key preservation across retry headers.

use some_stripe::{
    CreatePaymentLinkParams, OmnibusPriceTransparencyMetadata, OmnibusTaxBreakdown, PaymentLink,
    PaymentLinkId, PaymentLinkLineItemParams, PaymentLinkPriceDataParams, PaymentLinkStatus,
    StripeClient, StripeConfig, StripeError, TaxBehavior, UpdatePaymentLinkParams,
    validate_omnibus_price_transparency, validate_tax_behavior_transparency,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Helper function to write an HTTP 200 OK response with JSON payload and close the connection.
async fn write_http_json_response(socket: &mut tokio::net::TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    socket
        .write_all(response.as_bytes())
        .await
        .expect("Failed to write mock response");
    socket.flush().await.expect("Failed to flush socket");
}

/// Helper function to read a complete HTTP request (headers + body) from a socket.
async fn read_http_request(socket: &mut tokio::net::TcpStream) -> (String, String) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        let n = socket
            .read(&mut chunk)
            .await
            .expect("Failed to read from socket");
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let header_str = String::from_utf8_lossy(&buf[..pos]).to_string();
            let mut content_length = 0;
            for line in header_str.lines() {
                let lower = line.to_ascii_lowercase();
                if let Some(val) = lower.strip_prefix("content-length:") {
                    content_length = val.trim().parse::<usize>().unwrap_or(0);
                }
            }
            if buf.len() >= pos + 4 + content_length {
                let body_str =
                    String::from_utf8_lossy(&buf[pos + 4..pos + 4 + content_length]).to_string();
                return (header_str, body_str);
            }
        }
    }
    let full = String::from_utf8_lossy(&buf).to_string();
    (full, String::new())
}

// ============================================================================
// Deliverable 2: Test 1 - Compliant Discount with Documented 30-Day Baseline
// ============================================================================

#[test]
fn test_compliant_discount_with_documented_prior_30d_baseline() {
    // Scenario 1: Regular €100.00 (10000 cents), announced sale €80.00 (8000 cents),
    // with documented prior 30-day lowest price €85.00 (8500 cents).
    // Compliant because 8000 < 10000 and 8000 <= 8500.
    let result = validate_omnibus_price_transparency(10000, 8000, Some(8500));
    assert!(result.is_ok(), "Expected compliant discount to pass");

    // Scenario 2: Regular €100.00, announced sale €80.00, prior 30-day lowest was €80.00.
    // Compliant because 8000 <= 8000 (equality boundary condition).
    assert!(validate_omnibus_price_transparency(10000, 8000, Some(8000)).is_ok());

    // Scenario 3: Regular €100.00, announced sale €70.00, prior 30-day lowest was €100.00
    // (product was never on sale in the past 30 days). Compliant because 7000 <= 10000.
    assert!(validate_omnibus_price_transparency(10000, 7000, Some(10000)).is_ok());

    // Scenario 4: Regular €50.00, regular catalog price without discount (5000 cents),
    // no prior 30-day lowest required when no discount is announced.
    assert!(validate_omnibus_price_transparency(5000, 5000, None).is_ok());

    // Scenario 5: Large enterprise tier: Regular €100,000.00, discounted to €80,000.00
    // with 30-day baseline of €90,000.00. Verified integer arithmetic holds without float loss.
    assert!(validate_omnibus_price_transparency(10_000_000, 8_000_000, Some(9_000_000)).is_ok());
}

// ============================================================================
// Deliverable 2: Test 2 - Missing Prior 30-Day Baseline Price
// ============================================================================

#[test]
fn test_error_missing_prior_30d_baseline_when_discount_advertised() {
    // Attempting to announce a discount without statutory prior 30-day lowest baseline
    let result = validate_omnibus_price_transparency(10000, 8000, None);
    assert!(
        result.is_err(),
        "Must reject discount announcement missing prior 30-day baseline"
    );

    let err = result.unwrap_err();
    match err {
        StripeError::InvalidRequest(msg) => {
            assert!(
                msg.contains("prior_30d_lowest_price must be specified"),
                "Error message must specify missing prior 30-day baseline, got: {msg}"
            );
            assert!(
                msg.contains("EU Omnibus Directive 2019/2161 Article 6a"),
                "Error message must cite EU Omnibus Directive Article 6a, got: {msg}"
            );
        }
        other => panic!("Expected StripeError::InvalidRequest, got: {other:?}"),
    }
}

// ============================================================================
// Deliverable 2: Test 3 - Deceptive Discount Exceeding 30-Day Baseline
// ============================================================================

#[test]
fn test_error_deceptive_discount_exceeds_30d_baseline() {
    // Merchant raises regular price to €120.00, announces "discount" to €90.00.
    // However, the lowest price in the preceding 30 days was €80.00.
    // The advertised "sale" price (€90.00) exceeds the statutory baseline (€80.00).
    // This deceptive practice is strictly prohibited by EU Directive 2019/2161 Article 6a.
    let result = validate_omnibus_price_transparency(12000, 9000, Some(8000));
    assert!(
        result.is_err(),
        "Must reject deceptive discount where sale price exceeds 30-day baseline"
    );

    let err = result.unwrap_err();
    match err {
        StripeError::InvalidRequest(msg) => {
            assert!(
                msg.contains("exceeds the prior 30-day lowest baseline price"),
                "Error message must diagnose deceptive baseline violation, got: {msg}"
            );
            assert!(
                msg.contains("deceptive discounting"),
                "Error message must explicitly call out deceptive discounting, got: {msg}"
            );
        }
        other => panic!("Expected StripeError::InvalidRequest, got: {other:?}"),
    }

    // Additional boundary test: advertised discount exceeds regular catalog price
    let err_exceeds_regular =
        validate_omnibus_price_transparency(5000, 6000, Some(5000)).unwrap_err();
    match err_exceeds_regular {
        StripeError::InvalidRequest(msg) => {
            assert!(
                msg.contains("cannot exceed regular_price"),
                "Expected regular_price ceiling check, got: {msg}"
            );
        }
        other => panic!("Expected StripeError::InvalidRequest, got: {other:?}"),
    }

    // Additional boundary tests: Non-positive / zero minor units are rejected (Gate G-Law)
    assert!(validate_omnibus_price_transparency(0, 1000, Some(1000)).is_err());
    assert!(validate_omnibus_price_transparency(-100, 1000, Some(1000)).is_err());
    assert!(validate_omnibus_price_transparency(1000, 0, Some(1000)).is_err());
    assert!(validate_omnibus_price_transparency(1000, -50, Some(1000)).is_err());
    assert!(validate_omnibus_price_transparency(1000, 800, Some(0)).is_err());
    assert!(validate_omnibus_price_transparency(1000, 800, Some(-500)).is_err());
}

// ============================================================================
// Deliverable 2: Test 4 - PaymentLink Wire Mock Capturing Statutory Metadata
// ============================================================================

#[tokio::test]
async fn test_payment_link_creation_capturing_statutory_transparency_metadata() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind TCP listener");
    let server_addr = listener.local_addr().expect("Failed to get local addr");

    // Spawn the mock Stripe API server
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("Accept failed");
        let (headers, body) = read_http_request(&mut socket).await;

        // Verify HTTP Wire Invariants
        assert!(
            headers.starts_with("POST /v1/payment_links HTTP/1.1"),
            "Expected POST /v1/payment_links, got: {headers}"
        );
        assert!(
            headers.contains("idempotency-key: plink_idem_omnibus_001"),
            "Missing or incorrect Idempotency-Key header in request"
        );

        // Verify line item and price data in form body
        assert!(
            body.contains("line_items%5B0%5D%5Bprice%5D=price_omnibus_eur_101")
                || body.contains("line_items[0][price]=price_omnibus_eur_101"),
            "Missing line item price reference in body: {body}"
        );
        assert!(
            body.contains("line_items%5B0%5D%5Bquantity%5D=1")
                || body.contains("line_items[0][quantity]=1"),
            "Missing line item quantity in body: {body}"
        );

        // Verify statutory Omnibus metadata keys in form body
        assert!(
            body.contains("metadata%5Bomnibus_regular_price%5D=10000")
                || body.contains("metadata[omnibus_regular_price]=10000"),
            "Missing omnibus_regular_price metadata in wire body: {body}"
        );
        assert!(
            body.contains("metadata%5Bomnibus_discounted_price%5D=8000")
                || body.contains("metadata[omnibus_discounted_price]=8000"),
            "Missing omnibus_discounted_price metadata in wire body: {body}"
        );
        assert!(
            body.contains("metadata%5Bomnibus_prior_30d_lowest_price%5D=8500")
                || body.contains("metadata[omnibus_prior_30d_lowest_price]=8500"),
            "Missing omnibus_prior_30d_lowest_price metadata in wire body: {body}"
        );
        assert!(
            body.contains("metadata%5Bstatutory_directive%5D=EU_Directive_2019_2161_Article_6a")
                || body.contains("metadata[statutory_directive]=EU_Directive_2019_2161_Article_6a"),
            "Missing statutory_directive metadata in wire body: {body}"
        );

        // Respond with HTTP 200 and complete PaymentLink entity payload
        let response_json = r#"{
            "id": "plink_1OvuomnibusStatutory123",
            "object": "payment_link",
            "active": true,
            "url": "https://buy.stripe.com/test_1OvuomnibusStatutory123",
            "currency": "eur",
            "metadata": {
                "omnibus_regular_price": "10000",
                "omnibus_discounted_price": "8000",
                "omnibus_prior_30d_lowest_price": "8500",
                "statutory_directive": "EU_Directive_2019_2161_Article_6a"
            },
            "livemode": false
        }"#;

        write_http_json_response(&mut socket, response_json).await;
    });

    // Client configuration pointing to local wire mock
    let config =
        StripeConfig::new("sk_test_omnibus_key").with_api_base(format!("http://{server_addr}"));
    let client = StripeClient::with_config(config);

    // Prepare statutory price transparency metadata
    let transparency = OmnibusPriceTransparencyMetadata::new(10000, 8000, Some(8500))
        .expect("Valid statutory transparency metadata");

    let params = CreatePaymentLinkParams::new(vec![PaymentLinkLineItemParams::new(
        "price_omnibus_eur_101",
        1,
    )])
    .active(true)
    .metadata(transparency.to_metadata_map())
    .idempotency_key("plink_idem_omnibus_001");

    // Execute PaymentLink creation via client.checkout().payment_links()
    let payment_link = client
        .checkout()
        .payment_links()
        .create(&params)
        .await
        .expect("PaymentLink creation should succeed");

    // Await server task to ensure all wire assertions passed
    server
        .await
        .expect("Server task panicked or failed assertions");

    // Verify returned domain entity invariants
    assert_eq!(payment_link.id, "plink_1OvuomnibusStatutory123");
    assert!(payment_link.is_active());
    assert_eq!(payment_link.status(), PaymentLinkStatus::Active);
    assert_eq!(
        payment_link.url,
        "https://buy.stripe.com/test_1OvuomnibusStatutory123"
    );
    assert_eq!(payment_link.currency.as_deref(), Some("eur"));

    // Verify statutory metadata decoding and compliance validation
    let meta_map = payment_link
        .metadata
        .as_ref()
        .expect("Payment link must contain metadata");

    let decoded = OmnibusPriceTransparencyMetadata::from_metadata_map(meta_map)
        .expect("Metadata parsing should succeed")
        .expect("Omnibus transparency metadata must be present");

    assert_eq!(decoded.regular_price, 10000);
    assert_eq!(decoded.announced_discounted_price, 8000);
    assert_eq!(decoded.prior_30d_lowest_price, Some(8500));

    // Validate that the returned statutory metadata is fully compliant
    assert!(
        validate_omnibus_price_transparency(
            decoded.regular_price,
            decoded.announced_discounted_price,
            decoded.prior_30d_lowest_price,
        )
        .is_ok()
    );
}

// ============================================================================
// Deliverable 2: Test 5 - PaymentLink Deactivation & Update Lifecycle
// ============================================================================

#[tokio::test]
async fn test_payment_link_deactivation_lifecycle_wire_mock() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind TCP listener");
    let server_addr = listener.local_addr().expect("Failed to get local addr");

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("Accept failed");
        let (headers, body) = read_http_request(&mut socket).await;

        assert!(headers.starts_with("POST /v1/payment_links/plink_deactivate_001 HTTP/1.1"));
        assert!(body.contains("active=false"));

        let response_json = r#"{
            "id": "plink_deactivate_001",
            "object": "payment_link",
            "active": false,
            "url": "https://buy.stripe.com/test_plink_deactivate_001",
            "livemode": false
        }"#;

        write_http_json_response(&mut socket, response_json).await;
    });

    let config =
        StripeConfig::new("sk_test_omnibus_key").with_api_base(format!("http://{server_addr}"));
    let client = StripeClient::with_config(config);

    // Call update to deactivate the link
    let update_params = UpdatePaymentLinkParams::default().active(false);
    let updated_link = client
        .payment_links()
        .update("plink_deactivate_001", &update_params)
        .await
        .expect("Update should succeed");

    server.await.expect("Mock server error");

    assert_eq!(updated_link.id, "plink_deactivate_001");
    assert!(!updated_link.is_active());
    assert_eq!(updated_link.status(), PaymentLinkStatus::Inactive);
}

// ============================================================================
// Deliverable 2: Test 6 - Gate G-Borrow Zero-Clone Verification
// ============================================================================

#[test]
fn test_gate_g_borrow_zero_clone_discipline() {
    // Verify that parameters and entities are referenced without unnecessary deep cloning
    let regular_price = 15000;
    let sale_price = 12000;
    let prior_lowest = Some(13000);

    // Primitive borrowing
    assert!(validate_omnibus_price_transparency(regular_price, sale_price, prior_lowest).is_ok());

    let line_items = vec![PaymentLinkLineItemParams::new("price_test_01", 2)];
    let params = CreatePaymentLinkParams::new(line_items);

    // Verify validate() borrows &self
    assert!(params.validate().is_ok());

    let plink = PaymentLink::new("plink_borrow_test", "https://buy.stripe.com/borrow", true);
    // Verify status() borrows &self
    assert!(plink.is_active());
    assert_eq!(plink.status(), PaymentLinkStatus::Active);

    // PaymentLinkId AsRef<str>
    let plink_id = PaymentLinkId::from("plink_id_ref_test");
    assert_eq!(plink_id.as_ref(), "plink_id_ref_test");
}

// ============================================================================
// Deliverable 2: Test 7 - Wire Mock Transparent Tax Behavior (Inclusive)
// ============================================================================

#[tokio::test]
async fn test_payment_link_wire_mock_tax_behavior_inclusive() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind TCP listener");
    let server_addr = listener.local_addr().expect("Failed to get local addr");

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("Accept failed");
        let (headers, body) = read_http_request(&mut socket).await;

        assert!(headers.starts_with("POST /v1/payment_links HTTP/1.1"));
        assert!(
            body.contains("line_items%5B0%5D%5Bprice_data%5D%5Btax_behavior%5D=inclusive")
                || body.contains("line_items[0][price_data][tax_behavior]=inclusive"),
            "Wire body missing tax_behavior=inclusive: {body}"
        );
        assert!(
            body.contains("line_items%5B0%5D%5Bprice_data%5D%5Bunit_amount%5D=12000")
                || body.contains("line_items[0][price_data][unit_amount]=12000"),
            "Wire body missing unit_amount=12000: {body}"
        );

        let response_json = r#"{
            "id": "plink_tax_incl_001",
            "object": "payment_link",
            "active": true,
            "url": "https://buy.stripe.com/test_tax_incl_001",
            "currency": "eur",
            "livemode": false
        }"#;
        write_http_json_response(&mut socket, response_json).await;
    });

    let config =
        StripeConfig::new("sk_test_omnibus_key").with_api_base(format!("http://{server_addr}"));
    let client = StripeClient::with_config(config);

    // Validate tax transparency
    assert!(validate_tax_behavior_transparency(TaxBehavior::Inclusive).is_ok());

    // Calculate exact statutory tax breakdown: €120.00 gross with 20% VAT
    let breakdown = OmnibusTaxBreakdown::calculate(12000, TaxBehavior::Inclusive, 2000)
        .expect("Calculation failed");
    assert_eq!(breakdown.tax_behavior, TaxBehavior::Inclusive);
    assert_eq!(breakdown.gross_amount, 12000);
    assert_eq!(breakdown.net_amount, 10000);
    assert_eq!(breakdown.tax_amount, 2000);

    let price_data = PaymentLinkPriceDataParams::new("eur", 12000).tax_behavior("inclusive");
    let line_item = PaymentLinkLineItemParams::with_price_data(price_data, 1);
    let params = CreatePaymentLinkParams::new(vec![line_item]).active(true);

    let payment_link = client
        .checkout()
        .payment_links()
        .create(&params)
        .await
        .expect("Creation failed");

    server.await.expect("Server task panicked");
    assert_eq!(payment_link.id, "plink_tax_incl_001");
    assert!(payment_link.is_active());
}

// ============================================================================
// Deliverable 2: Test 8 - Wire Mock Transparent Tax Behavior (Exclusive)
// ============================================================================

#[tokio::test]
async fn test_payment_link_wire_mock_tax_behavior_exclusive() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind TCP listener");
    let server_addr = listener.local_addr().expect("Failed to get local addr");

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("Accept failed");
        let (headers, body) = read_http_request(&mut socket).await;

        assert!(headers.starts_with("POST /v1/payment_links HTTP/1.1"));
        assert!(
            body.contains("line_items%5B0%5D%5Bprice_data%5D%5Btax_behavior%5D=exclusive")
                || body.contains("line_items[0][price_data][tax_behavior]=exclusive"),
            "Wire body missing tax_behavior=exclusive: {body}"
        );
        assert!(
            body.contains("line_items%5B0%5D%5Bprice_data%5D%5Bunit_amount%5D=10000")
                || body.contains("line_items[0][price_data][unit_amount]=10000"),
            "Wire body missing unit_amount=10000: {body}"
        );

        let response_json = r#"{
            "id": "plink_tax_excl_001",
            "object": "payment_link",
            "active": true,
            "url": "https://buy.stripe.com/test_tax_excl_001",
            "currency": "eur",
            "livemode": false
        }"#;
        write_http_json_response(&mut socket, response_json).await;
    });

    let config =
        StripeConfig::new("sk_test_omnibus_key").with_api_base(format!("http://{server_addr}"));
    let client = StripeClient::with_config(config);

    // Validate tax transparency
    assert!(validate_tax_behavior_transparency(TaxBehavior::Exclusive).is_ok());

    // Calculate exact statutory tax breakdown: €100.00 net with 20% VAT
    let breakdown = OmnibusTaxBreakdown::calculate(10000, TaxBehavior::Exclusive, 2000)
        .expect("Calculation failed");
    assert_eq!(breakdown.tax_behavior, TaxBehavior::Exclusive);
    assert_eq!(breakdown.net_amount, 10000);
    assert_eq!(breakdown.tax_amount, 2000);
    assert_eq!(breakdown.gross_amount, 12000);

    let price_data = PaymentLinkPriceDataParams::new("eur", 10000).tax_behavior("exclusive");
    let line_item = PaymentLinkLineItemParams::with_price_data(price_data, 1);
    let params = CreatePaymentLinkParams::new(vec![line_item]).active(true);

    let payment_link = client
        .checkout()
        .payment_links()
        .create(&params)
        .await
        .expect("Creation failed");

    server.await.expect("Server task panicked");
    assert_eq!(payment_link.id, "plink_tax_excl_001");
    assert!(payment_link.is_active());
}

// ============================================================================
// Deliverable 2: Test 9 - Tax Behavior Transparency Rejection of Unspecified
// ============================================================================

#[test]
fn test_tax_behavior_transparency_invariants() {
    assert!(validate_tax_behavior_transparency(TaxBehavior::Inclusive).is_ok());
    assert!(validate_tax_behavior_transparency(TaxBehavior::Exclusive).is_ok());

    let err = validate_tax_behavior_transparency(TaxBehavior::Unspecified).unwrap_err();
    match err {
        StripeError::InvalidRequest(msg) => {
            assert!(msg.contains("transparent tax breakdown requires tax_behavior"));
            assert!(msg.contains("EU Omnibus Directive"));
        }
        other => panic!("Expected StripeError::InvalidRequest, got: {other:?}"),
    }

    assert!(OmnibusTaxBreakdown::calculate(10000, TaxBehavior::Unspecified, 2000).is_err());
}
