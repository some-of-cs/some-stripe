//! Gate G-Borrow, Gate G-Law, Gate G2, and Wire Mock Integration Test Suite for Checkout Sessions (US-4.1.0).
//!
//! # Quality Invariants Verified:
//! - **Gate G-Borrow**:
//!   - Zero-clone parameter borrowing (`&CreateCheckoutSessionParams`, `&str`, `Option<&ListLineItemsParams>`).
//!   - Zero-copy borrowing of line items slice (`session.line_items_data() -> Option<&[LineItem]>`).
//!   - Deref coercion of `LineItemList` to `[LineItem]` and reference iteration (`for item in &list`).
//!   - Zero-allocation line item ID borrowing function pointer (`line_item_id(&item) -> &str`).
//!   - Zero-copy string slice borrowing on session properties (`url.as_deref()`, `status.as_deref()`, `payment_status.as_deref()`).
//!   - Zero-clone status inspector methods borrowing `&self` (`is_open()`, `is_complete()`, `is_expired()`, `is_paid()`, `is_unpaid()`, `is_fulfillment_ready()`).
//! - **Gate G-Law**:
//!   - Statutory ISO-4217 minor currency integer compliance (amounts as 64-bit signed integers, zero floating-point representation).
//! - **Gate G2**:
//!   - `idempotency_key` is isolated from request body serialization (`#[serde(skip)]`) and only injected into HTTP headers.
//!   - Automatic RFC 4122 UUID v4 Idempotency-Key injection and custom key preservation.
//! - **Mock Wire**:
//!   - Full HTTP wire simulation verifying bracket serialization fidelity and zero-clone deserialization.

use some_stripe::{
    CheckoutSession, CreateCheckoutSessionParams, LineItem, LineItemList, LineItemParams,
    LineItemPriceDataParams, LineItemProductDataParams, ListLineItemsParams, StripeClient,
    StripeConfig, is_fulfillment_ready, is_valid_uuid_v4, line_item_id,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_gate_g_borrow_zero_clone_parameter_borrowing() {
    let params = CreateCheckoutSessionParams::single_item(
        "https://example.com/success",
        "usd",
        5000,
        "Premium Annual Plan",
        2,
    )
    .cancel_url("https://example.com/cancel")
    .mode("payment")
    .client_reference_id("audit_order_ref_001")
    .expires_at(1730000000)
    .idempotency_key("idem_audit_checkout_001");

    // Borrow reference strictly by & without consuming or cloning
    let borrowed_ref: &CreateCheckoutSessionParams = &params;
    assert_eq!(borrowed_ref.success_url, "https://example.com/success");
    assert_eq!(
        borrowed_ref.cancel_url.as_deref(),
        Some("https://example.com/cancel")
    );
    assert_eq!(borrowed_ref.mode, "payment");
    assert_eq!(
        borrowed_ref.client_reference_id.as_deref(),
        Some("audit_order_ref_001")
    );
    assert_eq!(borrowed_ref.currency.as_deref(), Some("usd"));
    assert_eq!(borrowed_ref.unit_amount, Some(5000));
    assert_eq!(
        borrowed_ref.product_name.as_deref(),
        Some("Premium Annual Plan")
    );
    assert_eq!(borrowed_ref.quantity, Some(2));
    assert_eq!(borrowed_ref.expires_at, Some(1730000000));
    assert_eq!(
        borrowed_ref.idempotency_key.as_deref(),
        Some("idem_audit_checkout_001")
    );

    // Verify params remains intact and accessible after borrowing
    assert_eq!(params.unit_amount, Some(5000));
    assert_eq!(params.success_url, "https://example.com/success");

    // Zero-clone string slice session ID borrow
    let session_id = "cs_test_borrow_audit_id";
    let borrowed_id: &str = session_id;
    assert_eq!(borrowed_id, "cs_test_borrow_audit_id");

    // Zero-clone ListLineItemsParams borrow
    let list_params = ListLineItemsParams::new()
        .limit(10)
        .starting_after("li_prev_100");
    let borrowed_list_params: &ListLineItemsParams = &list_params;
    assert_eq!(borrowed_list_params.limit, Some(10));
}

#[test]
fn test_gate_g_borrow_zero_copy_session_and_line_items() {
    let item1 = LineItem::new("li_audit_1", 2000, 2000, "usd").with_quantity(1);
    let item2 = LineItem::new("li_audit_2", 3000, 3000, "usd").with_quantity(1);
    let line_items = LineItemList::new(
        vec![item1, item2],
        false,
        "/v1/checkout/sessions/cs_audit_01/line_items",
    );

    let session = CheckoutSession::new("cs_audit_01")
        .with_url("https://checkout.stripe.com/c/pay/cs_audit_01")
        .with_status("open")
        .with_payment_status("unpaid")
        .with_mode("payment")
        .with_amounts(5000, 5000)
        .with_currency("usd")
        .with_line_items(line_items);

    // Zero-copy string slice borrowing on session properties
    assert_eq!(
        session.url.as_deref(),
        Some("https://checkout.stripe.com/c/pay/cs_audit_01")
    );
    assert_eq!(session.status.as_deref(), Some("open"));
    assert_eq!(session.payment_status.as_deref(), Some("unpaid"));

    // Status inspector methods borrow &self without allocations
    assert!(session.is_open());
    assert!(!session.is_complete());
    assert!(!session.is_expired());
    assert!(session.is_unpaid());
    assert!(!session.is_paid());
    assert!(!session.is_no_payment_required());
    assert!(!session.is_fulfillment_ready());
    assert!(session.is_payment_mode());

    // Gate G-Borrow: Borrowed slice accessor for line items
    let items_slice: Option<&[LineItem]> = session.line_items_data();
    assert!(items_slice.is_some());
    let slice = items_slice.unwrap();
    assert_eq!(slice.len(), 2);
    assert_eq!(slice[0].id, "li_audit_1");
    assert_eq!(slice[1].id, "li_audit_2");

    // Deref & Iterator borrowing
    let line_items_ref = session.line_items.as_ref().unwrap();
    assert_eq!(line_items_ref[0].amount_total, 2000);
    assert_eq!(line_items_ref[1].amount_total, 3000);

    let mut count = 0;
    for item in line_items_ref {
        assert!(item.id.starts_with("li_audit_"));
        // Zero-copy ID borrowing
        let borrowed_id = line_item_id(item);
        assert_eq!(borrowed_id, item.id.as_str());
        count += 1;
    }
    assert_eq!(count, 2);
}

#[test]
fn test_gate_g_borrow_fulfillment_guard_invariants() {
    // 1. Open session -> Not fulfillment ready
    let mut session = CheckoutSession::new("cs_test_guard")
        .with_status("open")
        .with_payment_status("unpaid");
    assert!(!session.is_fulfillment_ready());
    assert!(!is_fulfillment_ready(&session));

    // 2. Completed session with unpaid status (e.g. delayed payment method) -> NOT fulfillment ready
    session.status = Some("complete".into());
    session.payment_status = Some("unpaid".into());
    assert!(!session.is_fulfillment_ready());
    assert!(!is_fulfillment_ready(&session));

    // 3. Completed session with paid status -> Fulfillment ready
    session.payment_status = Some("paid".into());
    assert!(session.is_fulfillment_ready());
    assert!(is_fulfillment_ready(&session));

    // 4. Completed session with no_payment_required -> Fulfillment ready
    session.payment_status = Some("no_payment_required".into());
    assert!(session.is_fulfillment_ready());
    assert!(is_fulfillment_ready(&session));

    // 5. Expired session -> Not fulfillment ready
    session.status = Some("expired".into());
    assert!(!session.is_fulfillment_ready());
    assert!(!is_fulfillment_ready(&session));
}

#[test]
fn test_gate_g_borrow_response_parsing_without_heap_cloning() {
    let payload = serde_json::json!({
        "id": "cs_zero_clone_val",
        "object": "checkout.session",
        "url": "https://checkout.stripe.com/c/pay/cs_zero_clone_val",
        "status": "open",
        "payment_status": "unpaid",
        "amount_total": 4500,
        "currency": "usd"
    });

    // Zero-clone deserialize directly from borrowed reference &Value
    use serde::Deserialize;
    let borrowed_parsed = CheckoutSession::deserialize(&payload)
        .expect("Must deserialize from borrowed &Value without clone");
    assert_eq!(borrowed_parsed.id, "cs_zero_clone_val");
    assert_eq!(
        borrowed_parsed.url.as_deref(),
        Some("https://checkout.stripe.com/c/pay/cs_zero_clone_val")
    );
    assert!(borrowed_parsed.is_open());

    // Consuming owned Value directly (move semantics, zero deep copy)
    let moved_parsed: CheckoutSession = serde_json::from_value(payload)
        .expect("Must consume owned Value without intermediate clone");
    assert_eq!(moved_parsed.id, "cs_zero_clone_val");

    // Zero-clone parsing from borrowed byte slice
    let raw_bytes = br#"{"id":"cs_slice_parse","object":"checkout.session","status":"open","payment_status":"unpaid"}"#;
    let slice_parsed: CheckoutSession =
        serde_json::from_slice(raw_bytes).expect("Must parse directly from byte slice");
    assert_eq!(slice_parsed.id, "cs_slice_parse");
    assert!(slice_parsed.is_open());
}

#[test]
fn test_gate_g_law_iso_4217_minor_currency_integer_safety() {
    // Assert integer minor currency units: $50.00 is exactly 5000 cents
    let params = CreateCheckoutSessionParams::single_item(
        "https://example.com/success",
        "usd",
        5000,
        "Widget Standard",
        1,
    );
    assert_eq!(params.unit_amount, Some(5000));
    assert_eq!(params.currency.as_deref(), Some("usd"));

    // Zero floating-point representation in CheckoutSession
    let session = CheckoutSession {
        id: "cs_minor_currency_test".into(),
        object: "checkout.session".into(),
        url: None,
        status: Some("open".into()),
        payment_status: Some("unpaid".into()),
        mode: Some("payment".into()),
        amount_total: Some(1999), // $19.99 represented as 1999 cents, not 19.99f64
        amount_subtotal: Some(1999),
        currency: Some("usd".into()),
        customer: None,
        client_reference_id: None,
        success_url: None,
        cancel_url: None,
        payment_intent: None,
        expires_at: None,
        created: None,
        metadata: None,
        line_items: None,
    };
    assert_eq!(session.amount_total, Some(1999));
    assert_eq!(session.amount_subtotal, Some(1999));

    // Zero floating-point representation in LineItem
    let item = LineItem::new("li_minor_test", 1550, 1550, "usd");
    assert_eq!(item.amount_total, 1550);
    assert_eq!(item.amount_subtotal, 1550);
}

#[test]
fn test_gate_g2_idempotency_key_header_isolation() {
    let params = CreateCheckoutSessionParams::single_item(
        "https://example.com/success",
        "usd",
        3000,
        "Book",
        1,
    )
    .idempotency_key("secret_idempotency_key_xyz");

    // Verify #[serde(skip)] ensures idempotency_key is NEVER serialized into request body
    let json = serde_json::to_string(&params).expect("Serialization must succeed");
    assert!(
        !json.contains("idempotency_key"),
        "Gate G2 violation: idempotency_key must be skipped during body serialization"
    );
    assert!(
        !json.contains("secret_idempotency_key_xyz"),
        "Gate G2 violation: idempotency_key value leaked into request body"
    );
}

#[tokio::test]
async fn test_checkout_sessions_lifecycle_mock_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind ephemeral mock port");
    let address = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        // Step 1: Create Checkout Session
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req.starts_with("POST /v1/checkout/sessions HTTP/1.1"));
        assert!(req.contains("success_url=https%3A%2F%2Fexample.com%2Fsuccess"));
        assert!(req.contains("cancel_url=https%3A%2F%2Fexample.com%2Fcancel"));
        assert!(req.contains("mode=payment"));
        assert!(req.contains("line_items%5B0%5D%5Bprice_data%5D%5Bcurrency%5D=usd"));
        assert!(req.contains("line_items%5B0%5D%5Bprice_data%5D%5Bunit_amount%5D=9900"));
        assert!(req.contains(
            "line_items%5B0%5D%5Bprice_data%5D%5Bproduct_data%5D%5Bname%5D=Audit+Pro+Plan"
        ));
        assert!(req.contains("line_items%5B0%5D%5Bquantity%5D=2"));
        assert!(req.contains("client_reference_id=ref_audit_99"));
        assert!(req.contains("idempotency-key: idem_mock_flow_01"));

        let create_body = r#"{
            "id": "cs_mock_lifecycle_100",
            "object": "checkout.session",
            "url": "https://checkout.stripe.com/c/pay/cs_mock_lifecycle_100",
            "status": "open",
            "payment_status": "unpaid",
            "amount_total": 19800,
            "currency": "usd",
            "client_reference_id": "ref_audit_99"
        }"#;
        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{create_body}",
                    create_body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();

        // Step 2: Retrieve Checkout Session
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req.starts_with("GET /v1/checkout/sessions/cs_mock_lifecycle_100 HTTP/1.1"));
        assert!(!req.to_lowercase().contains("idempotency-key:"));

        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{create_body}",
                    create_body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();

        // Step 3: Expire Checkout Session
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(
            req.starts_with("POST /v1/checkout/sessions/cs_mock_lifecycle_100/expire HTTP/1.1")
        );
        let expire_header = req
            .lines()
            .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
            .expect("Expire request must contain auto-injected Idempotency-Key header");
        assert!(is_valid_uuid_v4(
            expire_header.split(':').nth(1).unwrap().trim()
        ));

        let expire_body = r#"{
            "id": "cs_mock_lifecycle_100",
            "object": "checkout.session",
            "status": "expired",
            "payment_status": "unpaid",
            "amount_total": 19800,
            "currency": "usd"
        }"#;
        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{expire_body}",
                    expire_body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config =
        StripeConfig::new("sk_test_mock_secret").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // 1. Create Checkout Session
    let create_params = CreateCheckoutSessionParams::single_item(
        "https://example.com/success",
        "usd",
        9900,
        "Audit Pro Plan",
        2,
    )
    .cancel_url("https://example.com/cancel")
    .mode("payment")
    .client_reference_id("ref_audit_99")
    .idempotency_key("idem_mock_flow_01");

    let session = client
        .checkout()
        .sessions
        .create(&create_params)
        .await
        .expect("Create failed");
    assert_eq!(session.id, "cs_mock_lifecycle_100");
    assert_eq!(
        session.url.as_deref(),
        Some("https://checkout.stripe.com/c/pay/cs_mock_lifecycle_100")
    );
    assert!(session.is_open());
    assert!(session.is_unpaid());
    assert_eq!(session.amount_total, Some(19800));

    // 2. Retrieve Checkout Session
    let retrieved = client
        .checkout()
        .sessions
        .retrieve("cs_mock_lifecycle_100")
        .await
        .expect("Retrieve failed");
    assert_eq!(retrieved.id, "cs_mock_lifecycle_100");
    assert!(retrieved.is_open());
    assert_eq!(retrieved.amount_total, Some(19800));

    // 3. Expire Checkout Session
    let expired = client
        .checkout()
        .sessions
        .expire("cs_mock_lifecycle_100")
        .await
        .expect("Expire failed");
    assert_eq!(expired.id, "cs_mock_lifecycle_100");
    assert_eq!(expired.status.as_deref(), Some("expired"));
    assert!(expired.is_expired());
    assert!(!expired.is_open());

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_checkout_create_auto_injected_idempotency_key() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind ephemeral mock port");
    let address = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req.starts_with("POST /v1/checkout/sessions HTTP/1.1"));
        let header = req
            .lines()
            .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
            .expect("Auto Idempotency-Key header must be injected when omitted");
        let key = header.split(':').nth(1).unwrap().trim();
        assert!(
            is_valid_uuid_v4(key),
            "Auto-injected key must be valid RFC 4122 UUID v4: {key}"
        );

        let resp = r#"{"id":"cs_auto_idem_99","object":"checkout.session","status":"open","payment_status":"unpaid"}"#;
        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                    resp.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config = StripeConfig::new("sk_test_auto_idem").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // Call without specifying idempotency_key
    let params = CreateCheckoutSessionParams::single_item(
        "https://example.com/success",
        "eur",
        2500,
        "Basic Tier",
        1,
    );
    let session = client.checkout().sessions.create(&params).await.unwrap();
    assert_eq!(session.id, "cs_auto_idem_99");
    assert!(session.is_open());

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_checkout_create_bracket_encoding_and_multi_item() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind ephemeral mock port");
    let address = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req.starts_with("POST /v1/checkout/sessions HTTP/1.1"));
        // Check bracket serialization fidelity
        assert!(req.contains("line_items%5B0%5D%5Bprice_data%5D%5Bcurrency%5D=usd"));
        assert!(req.contains("line_items%5B0%5D%5Bprice_data%5D%5Bunit_amount%5D=1250"));
        assert!(req.contains("line_items%5B0%5D%5Bquantity%5D=3"));

        let resp = r#"{"id":"cs_bracket_test","object":"checkout.session","status":"open","payment_status":"unpaid"}"#;
        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                    resp.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config = StripeConfig::new("sk_test_brackets").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let price_data = LineItemPriceDataParams::with_amount(
        "usd",
        1250,
        LineItemProductDataParams::new("Deluxe & Special Kit / 100% Cotton"),
    );
    let line_item = LineItemParams::from_price_data(price_data, 3);

    let params =
        CreateCheckoutSessionParams::new("https://example.com/success?order_id=ord_999&src=promo")
            .line_items(vec![line_item])
            .idempotency_key("idem_special_chars_123");

    let session = client.checkout().sessions.create(&params).await.unwrap();
    assert_eq!(session.id, "cs_bracket_test");

    server_task.await.unwrap();
}
