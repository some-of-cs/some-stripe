//! Integration and Security Test Suite for Checkout Sessions (`US-4.1.0` Unit 4).
//!
//! Validates:
//! - `is_fulfillment_ready`: Status and payment_status evaluation matrix (`complete` + `paid` / `no_payment_required`).
//! - Delayed payment method lifecycle (SEPA Direct Debit, Boleto, Konbini) where fulfillment
//!   is deferred until asynchronous payment settlement.
//! - Open redirect protection: `validate_redirect_url` rejecting `javascript:`, data URIs,
//!   insecure schemes, embedded userinfo, and malicious domains while accepting configured domains and localhost.
//! - Gate G-Borrow zero-clone borrowing discipline.
//! - Wire mock integration simulating asynchronous payment clearing lifecycle.

use some_stripe::{
    CheckoutSession, Event, EventObject, RedirectError, StripeClient, StripeConfig,
    is_fulfillment_ready, validate_redirect_url,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_is_fulfillment_ready_status_matrix() {
    let test_cases = vec![
        // (status, payment_status, expected_ready)
        (Some("complete"), Some("paid"), true),
        (Some("complete"), Some("no_payment_required"), true),
        (Some("complete"), Some("unpaid"), false),
        (Some("complete"), None, false),
        (Some("open"), Some("paid"), false),
        (Some("open"), Some("unpaid"), false),
        (Some("open"), Some("no_payment_required"), false),
        (Some("open"), None, false),
        (Some("expired"), Some("paid"), false),
        (Some("expired"), Some("unpaid"), false),
        (Some("expired"), Some("no_payment_required"), false),
        (Some("expired"), None, false),
        (None, Some("paid"), false),
        (None, Some("unpaid"), false),
        (None, Some("no_payment_required"), false),
        (None, None, false),
    ];

    for (status, payment_status, expected) in test_cases {
        let mut session = CheckoutSession::new("cs_test_matrix_123");
        session.url = Some("https://checkout.stripe.com/c/pay/cs_test_matrix_123".into());
        session.status = status.map(String::from);
        session.payment_status = payment_status.map(String::from);
        session.payment_intent = Some("pi_matrix_123".into());
        session.amount_total = Some(4999);
        session.currency = Some("usd".into());
        session.client_reference_id = Some("ref_test_001".into());
        session.customer = Some("cus_test_001".into());
        session.expires_at = Some(1728345600);
        session.created = Some(1728342000);

        // Gate G-Borrow: borrowing &session without cloning
        assert_eq!(
            is_fulfillment_ready(&session),
            expected,
            "Failed standalone is_fulfillment_ready for status={status:?}, payment_status={payment_status:?}"
        );
        assert_eq!(
            session.is_fulfillment_ready(),
            expected,
            "Failed CheckoutSession::is_fulfillment_ready for status={status:?}, payment_status={payment_status:?}"
        );
    }
}

#[test]
fn test_delayed_payment_method_deferred_fulfillment_lifecycle() {
    // 1. Initial webhook: checkout.session.completed with delayed payment method (SEPA Direct Debit / Boleto).
    // Status is 'complete', but payment_status is 'unpaid' because funds take days to settle.
    let initial_completed_event_json = r#"{
        "id": "evt_sepa_completed_001",
        "object": "event",
        "api_version": "2024-06-20",
        "created": 1728342000,
        "type": "checkout.session.completed",
        "data": {
            "object": {
                "id": "cs_sepa_delayed_123",
                "object": "checkout.session",
                "status": "complete",
                "payment_status": "unpaid",
                "payment_intent": "pi_sepa_delayed_123",
                "amount_total": 7500,
                "currency": "eur",
                "client_reference_id": "order_sepa_777",
                "customer": "cus_sepa_cust_42"
            }
        }
    }"#;

    let event: Event = serde_json::from_str(initial_completed_event_json)
        .expect("Failed to deserialize initial completed event");
    let event_obj = event
        .deserialize_object()
        .expect("Failed to parse EventData into EventObject");

    match event_obj {
        EventObject::CheckoutSession(session) => {
            assert_eq!(session.id, "cs_sepa_delayed_123");
            assert_eq!(session.status.as_deref(), Some("complete"));
            assert_eq!(session.payment_status.as_deref(), Some("unpaid"));

            // CRITICAL STRIPE INVARIANT: Fulfillment MUST be deferred!
            assert!(
                !is_fulfillment_ready(&session),
                "Fulfillment must be DEFERRED when payment_status is unpaid"
            );
            assert!(
                !session.is_fulfillment_ready(),
                "CheckoutSession method must also report fulfillment deferred"
            );
        }
        other => panic!("Expected CheckoutSession, got: {other:?}"),
    }

    // 2. Async clearing webhook: checkout.session.async_payment_succeeded.
    // Received 2-3 business days later when bank transfer clears.
    let cleared_event_json = r#"{
        "id": "evt_sepa_cleared_002",
        "object": "event",
        "api_version": "2024-06-20",
        "created": 1728514800,
        "type": "checkout.session.async_payment_succeeded",
        "data": {
            "object": {
                "id": "cs_sepa_delayed_123",
                "object": "checkout.session",
                "status": "complete",
                "payment_status": "paid",
                "payment_intent": "pi_sepa_delayed_123",
                "amount_total": 7500,
                "currency": "eur",
                "client_reference_id": "order_sepa_777",
                "customer": "cus_sepa_cust_42"
            }
        }
    }"#;

    let cleared_event: Event = serde_json::from_str(cleared_event_json)
        .expect("Failed to deserialize async payment succeeded event");
    let cleared_obj = cleared_event
        .deserialize_object()
        .expect("Failed to parse EventData into EventObject");

    match cleared_obj {
        EventObject::CheckoutSession(session) => {
            assert_eq!(session.id, "cs_sepa_delayed_123");
            assert_eq!(session.status.as_deref(), Some("complete"));
            assert_eq!(session.payment_status.as_deref(), Some("paid"));

            // Fulfillment is now UNBLOCKED and ready to dispatch
            assert!(
                is_fulfillment_ready(&session),
                "Fulfillment must be READY when payment_status == paid"
            );
            assert!(
                session.is_fulfillment_ready(),
                "CheckoutSession method must also report fulfillment ready"
            );
        }
        other => panic!("Expected CheckoutSession, got: {other:?}"),
    }

    // 3. Alternative failure path: checkout.session.async_payment_failed.
    let failed_event_json = r#"{
        "id": "evt_sepa_failed_003",
        "object": "event",
        "api_version": "2024-06-20",
        "created": 1728515000,
        "type": "checkout.session.async_payment_failed",
        "data": {
            "object": {
                "id": "cs_sepa_delayed_123",
                "object": "checkout.session",
                "status": "complete",
                "payment_status": "unpaid",
                "payment_intent": "pi_sepa_delayed_123",
                "amount_total": 7500,
                "currency": "eur"
            }
        }
    }"#;

    let failed_event: Event = serde_json::from_str(failed_event_json)
        .expect("Failed to deserialize async payment failed event");
    let failed_obj = failed_event
        .deserialize_object()
        .expect("Failed to parse EventData into EventObject");

    match failed_obj {
        EventObject::CheckoutSession(session) => {
            assert!(
                !is_fulfillment_ready(&session),
                "Fulfillment must NOT occur if async payment failed"
            );
        }
        other => panic!("Expected CheckoutSession, got: {other:?}"),
    }
}

#[test]
fn test_no_payment_required_fulfillment_lifecycle() {
    // 100% off coupon or free trial session
    let free_event_json = r#"{
        "id": "evt_free_trial_001",
        "object": "event",
        "api_version": "2024-06-20",
        "created": 1728342000,
        "type": "checkout.session.completed",
        "data": {
            "object": {
                "id": "cs_free_trial_123",
                "object": "checkout.session",
                "status": "complete",
                "payment_status": "no_payment_required",
                "amount_total": 0,
                "currency": "usd",
                "client_reference_id": "free_order_123"
            }
        }
    }"#;

    let event: Event = serde_json::from_str(free_event_json).expect("Failed to deserialize event");
    let event_obj = event
        .deserialize_object()
        .expect("Failed to parse EventObject");

    match event_obj {
        EventObject::CheckoutSession(session) => {
            assert_eq!(session.id, "cs_free_trial_123");
            assert_eq!(session.status.as_deref(), Some("complete"));
            assert_eq!(
                session.payment_status.as_deref(),
                Some("no_payment_required")
            );

            // Fulfillment MUST be unblocked because no payment is required!
            assert!(
                is_fulfillment_ready(&session),
                "Fulfillment must be ready for complete sessions with no_payment_required"
            );
            assert!(session.is_fulfillment_ready());
        }
        other => panic!("Expected CheckoutSession, got: {other:?}"),
    }
}

#[test]
fn test_validate_redirect_url_rejections() {
    let allowed = ["example.com", "mysite.org", "checkout.corp.internal"];

    // 1. Reject JavaScript schemes (XSS vectors)
    assert!(matches!(
        validate_redirect_url("javascript:alert(1)", &allowed),
        Err(RedirectError::ProhibitedScheme { scheme, .. }) if scheme == "javascript"
    ));
    assert!(matches!(
        validate_redirect_url("javascript:void(0)", &allowed),
        Err(RedirectError::ProhibitedScheme { .. })
    ));
    assert!(matches!(
        validate_redirect_url("JAVASCRIPT:alert('xss')", &allowed),
        Err(RedirectError::ProhibitedScheme { .. })
    ));

    // 2. Reject Data URIs
    assert!(matches!(
        validate_redirect_url(
            "data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==",
            &allowed
        ),
        Err(RedirectError::ProhibitedScheme { scheme, .. }) if scheme == "data"
    ));
    assert!(matches!(
        validate_redirect_url("data:text/plain,hello", &allowed),
        Err(RedirectError::ProhibitedScheme { .. })
    ));

    // 3. Reject non-HTTP/HTTPS schemes
    assert!(matches!(
        validate_redirect_url("vbscript:msgbox(1)", &allowed),
        Err(RedirectError::ProhibitedScheme { .. })
    ));
    assert!(matches!(
        validate_redirect_url("file:///etc/passwd", &allowed),
        Err(RedirectError::ProhibitedScheme { .. })
    ));
    assert!(matches!(
        validate_redirect_url("ftp://example.com/file", &allowed),
        Err(RedirectError::ProhibitedScheme { .. })
    ));
    assert!(matches!(
        validate_redirect_url("ssh://root@example.com", &allowed),
        Err(RedirectError::ProhibitedScheme { .. })
    ));

    // 4. Reject insecure HTTP for external domains
    assert!(matches!(
        validate_redirect_url("http://example.com/success", &allowed),
        Err(RedirectError::ProhibitedScheme { scheme, .. }) if scheme == "http"
    ));
    assert!(matches!(
        validate_redirect_url("http://mysite.org/cancel", &allowed),
        Err(RedirectError::ProhibitedScheme { .. })
    ));

    // 5. Reject malicious external domains (open redirect attacks)
    assert!(matches!(
        validate_redirect_url("https://evil.com/steal-creds", &allowed),
        Err(RedirectError::DisallowedDomain { ref host, .. }) if host == "evil.com"
    ));
    assert!(matches!(
        validate_redirect_url("https://phishing.example.com.evil.com/login", &allowed),
        Err(RedirectError::DisallowedDomain { .. })
    ));
    assert!(matches!(
        validate_redirect_url("https://example.com.attacker.com/return", &allowed),
        Err(RedirectError::DisallowedDomain { .. })
    ));
    assert!(matches!(
        validate_redirect_url("https://attacker-example.com/return", &allowed),
        Err(RedirectError::DisallowedDomain { .. })
    ));
    assert!(matches!(
        validate_redirect_url("https://notmysite.org/return", &allowed),
        Err(RedirectError::DisallowedDomain { .. })
    ));

    // 6. Reject embedded credentials / userinfo
    assert!(matches!(
        validate_redirect_url("https://user:password@example.com/success", &allowed),
        Err(RedirectError::EmbeddedCredentials { .. })
    ));
    assert!(matches!(
        validate_redirect_url("https://admin@example.com/success", &allowed),
        Err(RedirectError::EmbeddedCredentials { .. })
    ));

    // 7. Reject relative or malformed URLs
    assert!(matches!(
        validate_redirect_url("/orders/success", &allowed),
        Err(RedirectError::InvalidUrl { .. })
    ));
    assert!(matches!(
        validate_redirect_url("orders/success", &allowed),
        Err(RedirectError::InvalidUrl { .. })
    ));
    assert!(matches!(
        validate_redirect_url("", &allowed),
        Err(RedirectError::InvalidUrl { .. })
    ));
    assert!(matches!(
        validate_redirect_url("https://", &allowed),
        Err(RedirectError::MissingHost { .. })
    ));
    assert!(matches!(
        validate_redirect_url("://invalid", &allowed),
        Err(RedirectError::InvalidUrl { .. })
    ));
}

#[test]
fn test_validate_redirect_url_acceptances() {
    let allowed = ["example.com", "mysite.org", "*.store.io"];

    // 1. Accept valid configured domains via HTTPS
    assert!(validate_redirect_url("https://example.com/checkout/success", &allowed).is_ok());
    assert!(validate_redirect_url("https://example.com/cancel", &allowed).is_ok());
    assert!(validate_redirect_url("https://mysite.org/billing/return", &allowed).is_ok());

    // 2. Accept subdomains of configured domains
    assert!(validate_redirect_url("https://shop.example.com/order/123", &allowed).is_ok());
    assert!(validate_redirect_url("https://app.mysite.org/callback", &allowed).is_ok());
    assert!(validate_redirect_url("https://eu.store.io/checkout/done", &allowed).is_ok());

    // 3. Accept URLs with complex query parameters and fragments
    assert!(
        validate_redirect_url(
            "https://example.com/success?session_id={CHECKOUT_SESSION_ID}&tracking=abc#step2",
            &allowed
        )
        .is_ok()
    );

    // 4. Accept localhost / loopback addresses for development and testing (HTTP and HTTPS)
    assert!(validate_redirect_url("http://localhost:3000/success", &allowed).is_ok());
    assert!(validate_redirect_url("http://localhost:8080/cancel", &allowed).is_ok());
    assert!(validate_redirect_url("https://localhost:8443/orders/success", &allowed).is_ok());
    assert!(validate_redirect_url("http://127.0.0.1:3000/success", &allowed).is_ok());
    assert!(validate_redirect_url("http://127.0.0.1:8080/cancel", &allowed).is_ok());
    assert!(validate_redirect_url("http://[::1]:3000/success", &allowed).is_ok());

    // 5. Permissive mode: when allowed_domains is empty, any valid HTTPS domain is permitted
    let empty_allowed: [&str; 0] = [];
    assert!(validate_redirect_url("https://example.com/success", &empty_allowed).is_ok());
    assert!(validate_redirect_url("https://external-partner.com/done", &empty_allowed).is_ok());
    assert!(validate_redirect_url("http://localhost:3000/success", &empty_allowed).is_ok());
    // Still enforces scheme rules even in permissive mode
    assert!(validate_redirect_url("http://external-partner.com/done", &empty_allowed).is_err());
    assert!(validate_redirect_url("javascript:alert(1)", &empty_allowed).is_err());
}

#[tokio::test]
async fn test_checkout_wire_mock_async_fulfillment_lifecycle() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        // Request 1: POST /v1/checkout/sessions (Create Session)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let (headers, body) = read_http_request(&mut socket).await;
            assert!(headers.starts_with("POST /v1/checkout/sessions HTTP/1.1"));
            assert!(headers.to_ascii_lowercase().contains("idempotency-key:"));
            assert!(body.contains("mode=payment"));
            assert!(body.contains("currency%5D=eur") || body.contains("currency=eur"));

            let resp_body = r#"{
                "id": "cs_wire_delayed_42",
                "object": "checkout.session",
                "status": "complete",
                "payment_status": "unpaid",
                "amount_total": 9900,
                "currency": "eur",
                "client_reference_id": "wire_order_42",
                "url": "https://checkout.stripe.com/c/pay/cs_wire_delayed_42"
            }"#;
            write_http_json_response(&mut socket, resp_body).await;
        }

        // Request 2: GET /v1/checkout/sessions/cs_wire_delayed_42 (Retrieve Session before clearing)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let (headers, _) = read_http_request(&mut socket).await;
            assert!(headers.starts_with("GET /v1/checkout/sessions/cs_wire_delayed_42 HTTP/1.1"));

            let resp_body = r#"{
                "id": "cs_wire_delayed_42",
                "object": "checkout.session",
                "status": "complete",
                "payment_status": "unpaid",
                "amount_total": 9900,
                "currency": "eur",
                "client_reference_id": "wire_order_42"
            }"#;
            write_http_json_response(&mut socket, resp_body).await;
        }

        // Request 3: GET /v1/checkout/sessions/cs_wire_delayed_42 (Retrieve Session after settlement)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let (headers, _) = read_http_request(&mut socket).await;
            assert!(headers.starts_with("GET /v1/checkout/sessions/cs_wire_delayed_42 HTTP/1.1"));

            let resp_body = r#"{
                "id": "cs_wire_delayed_42",
                "object": "checkout.session",
                "status": "complete",
                "payment_status": "paid",
                "amount_total": 9900,
                "currency": "eur",
                "client_reference_id": "wire_order_42"
            }"#;
            write_http_json_response(&mut socket, resp_body).await;
        }
    });

    let config =
        StripeConfig::new("sk_test_mock_async").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);

    let allowed_domains = ["example.com", "myshop.eu"];
    let success_url = "https://myshop.eu/checkout/complete";
    let cancel_url = "https://myshop.eu/checkout/cancel";

    // 1. AppSec check: validate redirect URLs prior to dispatch
    assert!(validate_redirect_url(success_url, &allowed_domains).is_ok());
    assert!(validate_redirect_url(cancel_url, &allowed_domains).is_ok());

    // 2. Create session with delayed payment method
    let create_params = some_stripe::CreateCheckoutSessionParams::single_item(
        success_url,
        "eur",
        9900,
        "Annual Subscription",
        1,
    )
    .with_cancel_url(cancel_url)
    .with_mode("payment")
    .with_client_reference_id("wire_order_42")
    .with_idempotency_key("idem_wire_async_42");

    let session = client
        .checkout()
        .sessions()
        .create(&create_params)
        .await
        .expect("Failed to create checkout session");

    assert_eq!(session.id, "cs_wire_delayed_42");
    assert_eq!(session.status.as_deref(), Some("complete"));
    assert_eq!(session.payment_status.as_deref(), Some("unpaid"));

    // Invariant: Immediate fulfillment must NOT be executed!
    assert!(!is_fulfillment_ready(&session));

    // 3. Retrieve session intermediate state (still unpaid)
    let pending_session = client
        .checkout()
        .sessions()
        .retrieve("cs_wire_delayed_42")
        .await
        .expect("Failed to retrieve pending session");

    assert_eq!(pending_session.status.as_deref(), Some("complete"));
    assert_eq!(pending_session.payment_status.as_deref(), Some("unpaid"));
    assert!(!is_fulfillment_ready(&pending_session));

    // 4. Retrieve session after clearing (simulating webhook notification trigger)
    let settled_session = client
        .checkout()
        .sessions()
        .retrieve("cs_wire_delayed_42")
        .await
        .expect("Failed to retrieve settled session");

    assert_eq!(settled_session.status.as_deref(), Some("complete"));
    assert_eq!(settled_session.payment_status.as_deref(), Some("paid"));
    // Fulfillment is now ready!
    assert!(is_fulfillment_ready(&settled_session));

    server.await.unwrap();
}

#[tokio::test]
async fn test_checkout_wire_mock_no_payment_required_fulfillment() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let (headers, _) = read_http_request(&mut socket).await;
        assert!(headers.starts_with("GET /v1/checkout/sessions/cs_wire_free_99 HTTP/1.1"));

        let resp_body = r#"{
            "id": "cs_wire_free_99",
            "object": "checkout.session",
            "status": "complete",
            "payment_status": "no_payment_required",
            "amount_total": 0,
            "currency": "usd",
            "client_reference_id": "wire_free_order_99"
        }"#;
        write_http_json_response(&mut socket, resp_body).await;
    });

    let config =
        StripeConfig::new("sk_test_mock_free").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);

    let session = client
        .checkout()
        .sessions()
        .retrieve("cs_wire_free_99")
        .await
        .expect("Failed to retrieve session");

    assert_eq!(session.id, "cs_wire_free_99");
    assert_eq!(session.status.as_deref(), Some("complete"));
    assert_eq!(
        session.payment_status.as_deref(),
        Some("no_payment_required")
    );
    // Fulfillment ready immediately!
    assert!(session.is_fulfillment_ready());
    assert!(is_fulfillment_ready(&session));

    server.await.unwrap();
}

/// Helper function to read a complete HTTP request (headers + Content-Length body) from a socket.
async fn read_http_request(socket: &mut tokio::net::TcpStream) -> (String, String) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        let n = socket
            .read(&mut chunk)
            .await
            .expect("Failed to read socket");
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
        .expect("Failed to write response");
    socket.flush().await.expect("Failed to flush socket");
}
