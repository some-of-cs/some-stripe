//! Wire mock integration tests for Checkout Sessions and Line Items resources
//! covering User Story 4.1.0 (`TC-4.1.0-01` through `TC-4.1.0-03`).
//!
//! # Quality Gates Verified:
//! - **G0**: Clean compilation on Rust 2024 edition without warnings or fmt diffs.
//! - **G-Borrow**: Zero-clone borrowing discipline across session creation, retrieval, and pagination.
//! - **G-Law**: ISO 4217 minor currency integer units ([`i64`]).
//! - **G2**: Idempotency key preservation and automatic RFC 4122 UUID v4 injection.

use some_stripe::resources::is_valid_uuid_v4;
use some_stripe::{
    CheckoutSession, CreateCheckoutSessionParams, LineItem, LineItemParams,
    LineItemPriceDataParams, LineItemProductDataParams, ListLineItemsParams, StripeClient,
    StripeClientExt, StripeConfig, StripeError, validate_and_encode_id,
};
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
async fn test_checkout_session_create_single_line_item_and_idempotency_e2e() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut req_buf = vec![0; 8192];
        let n = socket.read(&mut req_buf).await.unwrap();
        let wire = String::from_utf8(req_buf[..n].to_vec()).unwrap();

        assert!(wire.starts_with("POST /v1/checkout/sessions HTTP/1.1"));

        // Extract and verify automatic UUID v4 Idempotency-Key
        let idem_line = wire
            .lines()
            .find(|line| line.to_lowercase().starts_with("idempotency-key:"))
            .expect("Idempotency-Key header must be injected");
        let idem_key = idem_line.split(':').nth(1).unwrap().trim();
        assert!(
            is_valid_uuid_v4(idem_key),
            "Auto-generated key must be a valid RFC 4122 UUID v4: {idem_key}"
        );

        // Check form encoded parameters
        assert!(wire.contains("success_url=https%3A%2F%2Fexample.com%2Fsuccess"));
        assert!(wire.contains("mode=payment"));
        assert!(wire.contains("line_items%5B0%5D%5Bprice_data%5D%5Bcurrency%5D=usd"));
        assert!(wire.contains("line_items%5B0%5D%5Bprice_data%5D%5Bunit_amount%5D=5000"));
        assert!(
            wire.contains("line_items%5B0%5D%5Bprice_data%5D%5Bproduct_data%5D%5Bname%5D=Pro+Plan")
        );
        assert!(wire.contains("line_items%5B0%5D%5Bquantity%5D=1"));

        let body = r#"{
            "id": "cs_test_single_001",
            "object": "checkout.session",
            "url": "https://checkout.stripe.com/c/pay/cs_test_single_001",
            "status": "open",
            "payment_status": "unpaid",
            "mode": "payment",
            "amount_total": 5000,
            "amount_subtotal": 5000,
            "currency": "usd"
        }"#;
        write_json_response(&mut socket, body).await;
    });

    let config = StripeConfig::new("sk_test_checkout").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let params = CreateCheckoutSessionParams::single_item(
        "https://example.com/success",
        "usd",
        5000,
        "Pro Plan",
        1,
    );

    let session: CheckoutSession = client.checkout().sessions().create(&params).await.unwrap();
    server.await.unwrap();

    assert_eq!(session.id, "cs_test_single_001");
    assert_eq!(
        session.url.as_deref(),
        Some("https://checkout.stripe.com/c/pay/cs_test_single_001")
    );
    assert!(session.is_open());
    assert!(session.is_unpaid());
    assert!(!session.is_expired());
    assert!(!session.is_paid());
    assert_eq!(session.amount_total, Some(5000));
    assert_eq!(session.currency.as_deref(), Some("usd"));
}

#[tokio::test]
async fn test_checkout_session_create_multiple_line_items_and_custom_idempotency_e2e() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut req_buf = vec![0; 8192];
        let n = socket.read(&mut req_buf).await.unwrap();
        let wire = String::from_utf8(req_buf[..n].to_vec()).unwrap();

        assert!(wire.starts_with("POST /v1/checkout/sessions HTTP/1.1"));
        assert!(wire.contains("idempotency-key: idem_custom_checkout_999"));

        // Check multi-item bracket parameters
        assert!(wire.contains("line_items%5B0%5D%5Bprice_data%5D%5Bcurrency%5D=usd"));
        assert!(wire.contains("line_items%5B0%5D%5Bprice_data%5D%5Bunit_amount%5D=3000"));
        assert!(
            wire.contains("line_items%5B0%5D%5Bprice_data%5D%5Bproduct_data%5D%5Bname%5D=Item+A")
        );
        assert!(wire.contains("line_items%5B0%5D%5Bquantity%5D=2"));

        assert!(wire.contains("line_items%5B1%5D%5Bprice%5D=price_pre_existing_123"));
        assert!(wire.contains("line_items%5B1%5D%5Bquantity%5D=1"));

        let body = r#"{
            "id": "cs_test_multi_002",
            "object": "checkout.session",
            "url": "https://checkout.stripe.com/c/pay/cs_test_multi_002",
            "status": "open",
            "payment_status": "unpaid",
            "mode": "payment",
            "amount_total": 7500,
            "currency": "usd"
        }"#;
        write_json_response(&mut socket, body).await;
    });

    let config = StripeConfig::new("sk_test_checkout").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let params = CreateCheckoutSessionParams::new("https://example.com/success")
        .idempotency_key("idem_custom_checkout_999")
        .add_line_item(LineItemParams::from_price_data(
            LineItemPriceDataParams::with_amount(
                "usd",
                3000,
                LineItemProductDataParams::new("Item A"),
            ),
            2,
        ))
        .add_line_item(LineItemParams::from_price("price_pre_existing_123", 1));

    let session = client.checkout().sessions().create(&params).await.unwrap();
    server.await.unwrap();

    assert_eq!(session.id, "cs_test_multi_002");
    assert_eq!(session.amount_total, Some(7500));
}

#[tokio::test]
async fn test_checkout_session_retrieve_by_id_e2e() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req.starts_with("GET /v1/checkout/sessions/cs_test_ret_888 HTTP/1.1"));

        let body = r#"{
            "id": "cs_test_ret_888",
            "object": "checkout.session",
            "status": "complete",
            "payment_status": "paid",
            "mode": "payment",
            "amount_total": 10000,
            "currency": "usd"
        }"#;
        write_json_response(&mut socket, body).await;
    });

    let config = StripeConfig::new("sk_test_retrieve").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let session = client
        .checkout()
        .sessions()
        .retrieve("cs_test_ret_888")
        .await
        .unwrap();
    server.await.unwrap();

    assert_eq!(session.id, "cs_test_ret_888");
    assert!(session.is_complete());
    assert!(session.is_paid());
    assert_eq!(session.amount_total, Some(10000));
}

#[tokio::test]
async fn test_checkout_session_expire_transitions_to_expired_e2e() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req.starts_with("POST /v1/checkout/sessions/cs_test_expire_333/expire HTTP/1.1"));

        // Verify Idempotency-Key header is present on expire
        let idem_line = req
            .lines()
            .find(|line| line.to_lowercase().starts_with("idempotency-key:"))
            .expect("Idempotency-Key header must be injected on expire");
        let idem_key = idem_line.split(':').nth(1).unwrap().trim();
        assert!(is_valid_uuid_v4(idem_key));

        let body = r#"{
            "id": "cs_test_expire_333",
            "object": "checkout.session",
            "status": "expired",
            "payment_status": "unpaid",
            "mode": "payment"
        }"#;
        write_json_response(&mut socket, body).await;
    });

    let config = StripeConfig::new("sk_test_expire").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let expired_session = client
        .checkout()
        .sessions()
        .expire("cs_test_expire_333")
        .await
        .unwrap();
    server.await.unwrap();

    assert_eq!(expired_session.id, "cs_test_expire_333");
    assert!(expired_session.is_expired());
    assert!(!expired_session.is_open());
    assert_eq!(expired_session.status.as_deref(), Some("expired"));
}

#[tokio::test]
async fn test_checkout_session_line_items_pagination_e2e() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Page 1
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buf1 = vec![0; 4096];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
        assert!(req1.starts_with(
            "GET /v1/checkout/sessions/cs_test_pagination/line_items?limit=2 HTTP/1.1"
        ));

        let body1 = r#"{
            "object": "list",
            "data": [
                {
                    "id": "li_pag_001",
                    "object": "item",
                    "amount_subtotal": 1500,
                    "amount_total": 1500,
                    "currency": "usd",
                    "description": "Item 1",
                    "quantity": 1
                },
                {
                    "id": "li_pag_002",
                    "object": "item",
                    "amount_subtotal": 2500,
                    "amount_total": 2500,
                    "currency": "usd",
                    "description": "Item 2",
                    "quantity": 2
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
        assert!(req2.contains("starting_after=li_pag_002"));

        let body2 = r#"{
            "object": "list",
            "data": [
                {
                    "id": "li_pag_003",
                    "object": "item",
                    "amount_subtotal": 5000,
                    "amount_total": 5000,
                    "currency": "usd",
                    "description": "Item 3",
                    "quantity": 1
                }
            ],
            "has_more": false
        }"#;
        write_json_response(&mut socket2, body2).await;
    });

    let config = StripeConfig::new("sk_test_pagination").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let params = ListLineItemsParams::new().limit(2);
    let mut paginator = client
        .checkout()
        .sessions()
        .list_line_items("cs_test_pagination", &params);

    // Page 1: item 1
    let item1: LineItem = paginator
        .next_item()
        .await
        .unwrap()
        .expect("Item 1 must be present");
    assert_eq!(item1.id, "li_pag_001");
    assert_eq!(item1.amount_total, 1500);
    assert_eq!(item1.description.as_deref(), Some("Item 1"));
    assert_eq!(item1.quantity, Some(1));

    // Page 1: item 2
    let item2: LineItem = paginator
        .next_item()
        .await
        .unwrap()
        .expect("Item 2 must be present");
    assert_eq!(item2.id, "li_pag_002");
    assert_eq!(item2.amount_total, 2500);

    // Page 2: item 3
    let item3: LineItem = paginator
        .next_item()
        .await
        .unwrap()
        .expect("Item 3 must be present");
    assert_eq!(item3.id, "li_pag_003");
    assert_eq!(item3.amount_total, 5000);

    // Stream termination
    let item4 = paginator.next_item().await.unwrap();
    assert!(
        item4.is_none(),
        "Paginator must terminate when has_more is false"
    );

    server.await.unwrap();
    assert_eq!(paginator.pages_fetched(), 2);
}

#[tokio::test]
async fn test_checkout_session_facade_and_extension_trait() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // First request via core client extension trait
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buf1 = vec![0; 4096];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
        assert!(req1.starts_with("GET /v1/checkout/sessions/cs_ext_trait_1 HTTP/1.1"));
        let body1 = r#"{"id":"cs_ext_trait_1","object":"checkout.session","status":"open","mode":"payment"}"#;
        write_json_response(&mut socket1, body1).await;

        // Second request via facade sessions field
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf2 = vec![0; 4096];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req2 = String::from_utf8(buf2[..n2].to_vec()).unwrap();
        assert!(req2.starts_with("GET /v1/checkout/sessions/cs_facade_2 HTTP/1.1"));
        let body2 =
            r#"{"id":"cs_facade_2","object":"checkout.session","status":"open","mode":"payment"}"#;
        write_json_response(&mut socket2, body2).await;
    });

    let config = StripeConfig::new("sk_test_ext").with_api_base(format!("http://{address}"));

    // 1. Core client via StripeClientExt trait
    let core_client = some_stripe_core::StripeClient::with_config(config.clone());
    let s1 = core_client
        .checkout()
        .sessions()
        .retrieve("cs_ext_trait_1")
        .await
        .unwrap();
    assert_eq!(s1.id, "cs_ext_trait_1");

    // 2. High-level client via sessions direct field
    let client = StripeClient::with_config(config);
    let s2 = client
        .checkout()
        .sessions
        .retrieve("cs_facade_2")
        .await
        .unwrap();
    assert_eq!(s2.id, "cs_facade_2");

    server.await.unwrap();
}

#[tokio::test]
async fn test_checkout_session_create_unified_serialization_and_custom_idempotency_validation() {
    let config = StripeConfig::new("sk_test_mock").with_api_base("http://127.0.0.1:9");
    let client = StripeClient::with_config(config);

    // 1. Invalid whitespace/empty custom idempotency keys fail fast
    let empty_idem_params =
        CreateCheckoutSessionParams::new("https://example.com/success").idempotency_key("");
    let err = client
        .checkout()
        .sessions()
        .create(&empty_idem_params)
        .await
        .unwrap_err();
    assert!(
        matches!(err, StripeError::InvalidRequest(_)),
        "Empty idempotency key must return InvalidRequest: {err:?}"
    );

    let whitespace_idem_params =
        CreateCheckoutSessionParams::new("https://example.com/success").idempotency_key("   ");
    let err2 = client
        .checkout()
        .sessions()
        .create(&whitespace_idem_params)
        .await
        .unwrap_err();
    assert!(
        matches!(err2, StripeError::InvalidRequest(_)),
        "Whitespace idempotency key must return InvalidRequest: {err2:?}"
    );

    // 2. Unified shorthand fields + metadata + customer serialization e2e
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut req_buf = vec![0; 8192];
        let n = socket.read(&mut req_buf).await.unwrap();
        let wire = String::from_utf8(req_buf[..n].to_vec()).unwrap();

        assert!(wire.starts_with("POST /v1/checkout/sessions HTTP/1.1"));
        assert!(wire.contains("idempotency-key: idem_valid_key_456"));

        // Synthesized single item parameters
        assert!(wire.contains("line_items%5B0%5D%5Bprice_data%5D%5Bcurrency%5D=eur"));
        assert!(wire.contains("line_items%5B0%5D%5Bprice_data%5D%5Bunit_amount%5D=4500"));
        assert!(wire.contains(
            "line_items%5B0%5D%5Bprice_data%5D%5Bproduct_data%5D%5Bname%5D=Custom+T-Shirt"
        ));
        assert!(wire.contains("line_items%5B0%5D%5Bquantity%5D=2"));

        // Preserved top-level customer and metadata fields
        assert!(wire.contains("customer=cus_alice_123"));
        assert!(wire.contains("customer_email=alice%40example.com"));
        assert!(wire.contains("metadata%5Bsource%5D=checkout_patch"));
        assert!(wire.contains("cancel_url=https%3A%2F%2Fexample.com%2Fcancel"));

        // Must NOT emit un-nested flat fields as top-level parameters
        assert!(!wire.contains("&currency="));
        assert!(!wire.contains("&unit_amount="));
        assert!(!wire.contains("&product_name="));
        assert!(!wire.contains("&idempotency_key="));

        let body = r#"{
            "id": "cs_test_unified_001",
            "object": "checkout.session",
            "status": "open",
            "payment_status": "unpaid",
            "mode": "payment",
            "amount_total": 9000,
            "currency": "eur",
            "customer": "cus_alice_123"
        }"#;
        write_json_response(&mut socket, body).await;
    });

    let config = StripeConfig::new("sk_test_checkout").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let params = CreateCheckoutSessionParams::new("https://example.com/success")
        .cancel_url("https://example.com/cancel")
        .currency("eur")
        .unit_amount(4500)
        .product_name("Custom T-Shirt")
        .quantity(2)
        .customer("cus_alice_123")
        .customer_email("alice@example.com")
        .add_metadata("source", "checkout_patch")
        .idempotency_key("idem_valid_key_456");

    let session = client.checkout().sessions().create(&params).await.unwrap();
    server.await.unwrap();

    assert_eq!(session.id, "cs_test_unified_001");
    assert_eq!(session.customer.as_deref(), Some("cus_alice_123"));
    assert_eq!(session.amount_total, Some(9000));
}

#[tokio::test]
async fn test_checkout_session_id_validation_and_url_encoding_security() {
    // 1. Direct validation function tests
    assert!(validate_and_encode_id("").is_err());
    assert!(validate_and_encode_id("   ").is_err());
    assert!(validate_and_encode_id("cs_test/../traversal").is_err());
    assert!(validate_and_encode_id("cs_test\\evil").is_err());
    assert!(validate_and_encode_id("cs_test?param=1").is_err());
    assert!(validate_and_encode_id("cs_test#hash").is_err());
    assert!(validate_and_encode_id("cs_test\0null").is_err());
    assert!(validate_and_encode_id("cs_test\r\ninjection").is_err());
    assert_eq!(
        validate_and_encode_id("cs_test_valid_123").unwrap(),
        "cs_test_valid_123"
    );

    let config = StripeConfig::new("sk_test_mock").with_api_base("http://127.0.0.1:9");
    let client = StripeClient::with_config(config);

    // 2. retrieve fails fast on invalid IDs without network request
    assert!(matches!(
        client.checkout().sessions().retrieve("").await.unwrap_err(),
        StripeError::InvalidRequest(_)
    ));
    assert!(matches!(
        client
            .checkout()
            .sessions()
            .retrieve("cs_test/../evil")
            .await
            .unwrap_err(),
        StripeError::InvalidRequest(_)
    ));

    // 3. expire fails fast on invalid IDs
    assert!(matches!(
        client.checkout().sessions().expire("  ").await.unwrap_err(),
        StripeError::InvalidRequest(_)
    ));
    assert!(matches!(
        client
            .checkout()
            .sessions()
            .expire("cs_test?evil=1")
            .await
            .unwrap_err(),
        StripeError::InvalidRequest(_)
    ));

    // 4. list_line_items fails fast on invalid session IDs
    let mut paginator_empty = client
        .checkout()
        .sessions()
        .list_line_items("", &ListLineItemsParams::default());
    let err_pag = paginator_empty.next_item().await.unwrap_err();
    assert!(matches!(err_pag, StripeError::InvalidRequest(_)));

    let mut paginator_traversal = client
        .checkout()
        .sessions()
        .list_line_items("cs/evil/traversal", &ListLineItemsParams::default());
    let err_pag2 = paginator_traversal.next_item().await.unwrap_err();
    assert!(matches!(err_pag2, StripeError::InvalidRequest(_)));
}

#[tokio::test]
async fn test_checkout_session_list_line_items_cursor_propagation() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Request 1: starting_after cursor propagation
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buf1 = vec![0; 4096];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
        assert!(req1.contains("GET /v1/checkout/sessions/cs_test_curs/line_items?limit=5&starting_after=li_cursor_start_1"));

        let body1 = r#"{
            "object": "list",
            "data": [
                {
                    "id": "li_cursor_start_2",
                    "object": "item",
                    "amount_subtotal": 1000,
                    "amount_total": 1000,
                    "currency": "usd"
                }
            ],
            "has_more": false
        }"#;
        write_json_response(&mut socket1, body1).await;

        // Request 2: ending_before cursor propagation
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf2 = vec![0; 4096];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req2 = String::from_utf8(buf2[..n2].to_vec()).unwrap();
        assert!(req2.contains("GET /v1/checkout/sessions/cs_test_curs/line_items?limit=3&ending_before=li_cursor_end_9"));

        let body2 = r#"{
            "object": "list",
            "data": [
                {
                    "id": "li_cursor_end_8",
                    "object": "item",
                    "amount_subtotal": 2000,
                    "amount_total": 2000,
                    "currency": "usd"
                }
            ],
            "has_more": false
        }"#;
        write_json_response(&mut socket2, body2).await;
    });

    let config = StripeConfig::new("sk_test_curs").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // Test 1: starting_after
    let params_sa = ListLineItemsParams::new()
        .limit(5)
        .starting_after("li_cursor_start_1");
    let mut paginator_sa = client
        .checkout()
        .sessions()
        .list_line_items("cs_test_curs", &params_sa);
    let item_sa = paginator_sa
        .next_item()
        .await
        .unwrap()
        .expect("Item expected");
    assert_eq!(item_sa.id, "li_cursor_start_2");

    // Test 2: ending_before
    let params_eb = ListLineItemsParams::new()
        .limit(3)
        .ending_before("li_cursor_end_9");
    let mut paginator_eb = client
        .checkout()
        .sessions()
        .list_line_items("cs_test_curs", &params_eb);
    let item_eb = paginator_eb
        .next_item()
        .await
        .unwrap()
        .expect("Item expected");
    assert_eq!(item_eb.id, "li_cursor_end_8");

    server.await.unwrap();
}
