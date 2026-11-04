//! Wire Mock Integration Test Suite for PaymentIntents (`US-3.1.0`).
//!
//! Verifies:
//! - `AC-US-3.1.0-01`: Manual capture flow ($100.00 authorization, partial capture of $75.00).
//! - `AC-US-3.1.0-02`: Safe cancellation of uncaptured authorization (`requested_by_customer`).
//! - `AC-US-3.1.0-03`: Idempotent retry with identical `Idempotency-Key`.
//! - `AC-US-3.1.0-LAW`: 3D Secure PSD2 Strong Customer Authentication (Article 98 RTS) next_action challenge parsing.

use some_stripe::{
    CancelPaymentIntentParams, CaptureMethod, CapturePaymentIntentParams,
    CreatePaymentIntentParams, NextActionType, PaymentIntent, PaymentIntentStatus, StripeClient,
    StripeConfig,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

#[tokio::test]
async fn test_ac_us_3_1_0_01_manual_capture_partial() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind listener");
    let address = listener.local_addr().expect("Failed to get local address");

    // Spawn mock Stripe server
    let server = tokio::spawn(async move {
        // Step 1: Create manual PaymentIntent ($100.00 / 10000 cents)
        let (mut socket1, _) = listener.accept().await.expect("Accept create failed");
        let (headers1, body1) = read_http_request(&mut socket1).await;
        assert!(headers1.starts_with("POST /v1/payment_intents HTTP/1.1"));
        assert!(body1.contains("amount=10000"));
        assert!(body1.contains("currency=usd"));
        assert!(body1.contains("capture_method=manual"));

        let create_resp = r#"{
            "id": "pi_mock_manual_100",
            "object": "payment_intent",
            "amount": 10000,
            "amount_received": 0,
            "currency": "usd",
            "status": "requires_capture",
            "capture_method": "manual",
            "client_secret": "pi_mock_manual_100_secret_LiveKeyPCI40Token",
            "created": 1728345600,
            "next_action": null,
            "cancellation_reason": null
        }"#;
        write_http_json_response(&mut socket1, create_resp).await;

        // Step 2: Partial capture ($75.00 / 7500 cents)
        let (mut socket2, _) = listener.accept().await.expect("Accept capture failed");
        let (headers2, body2) = read_http_request(&mut socket2).await;
        assert!(
            headers2.starts_with("POST /v1/payment_intents/pi_mock_manual_100/capture HTTP/1.1")
        );
        assert!(body2.contains("amount_to_capture=7500"));

        let capture_resp = r#"{
            "id": "pi_mock_manual_100",
            "object": "payment_intent",
            "amount": 10000,
            "amount_received": 7500,
            "currency": "usd",
            "status": "succeeded",
            "capture_method": "manual",
            "client_secret": "pi_mock_manual_100_secret_LiveKeyPCI40Token",
            "created": 1728345600,
            "next_action": null,
            "cancellation_reason": null
        }"#;
        write_http_json_response(&mut socket2, capture_resp).await;
    });

    let config = StripeConfig::new("sk_test_mock_123").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // 1. Create authorization
    let intent = client
        .payment_intents()
        .create(
            &CreatePaymentIntentParams::new(10000, "usd")
                .unwrap()
                .capture_method("manual")
                .description("Order #1001 manual auth"),
        )
        .await
        .expect("Failed to create manual payment intent");

    assert_eq!(intent.id, "pi_mock_manual_100");
    assert_eq!(intent.amount, 10000);
    assert_eq!(intent.status, "requires_capture");
    assert_eq!(intent.amount_received, Some(0));

    // Gate G2 & PCI-DSS 4.0: client_secret must be masked
    let debug_repr = format!("{intent:?}");
    assert!(debug_repr.contains("[REDACTED]"));
    assert!(!debug_repr.contains("LiveKeyPCI40Token"));

    // 2. Capture partial 7500 cents
    let captured = client
        .payment_intents()
        .capture(
            &intent.id,
            Some(&CapturePaymentIntentParams {
                amount_to_capture: Some(7500),
                idempotency_key: None,
            }),
        )
        .await
        .expect("Failed to capture partial payment intent");

    assert_eq!(captured.id, "pi_mock_manual_100");
    assert_eq!(captured.amount, 10000);
    assert_eq!(captured.status, "succeeded");
    assert_eq!(captured.amount_received, Some(7500));

    server.await.expect("Mock server task panicked");
}

#[tokio::test]
async fn test_ac_us_3_1_0_02_safe_cancellation_requested_by_customer() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind listener");
    let address = listener.local_addr().expect("Failed to get local address");

    // Spawn mock Stripe server
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("Accept cancel failed");
        let (headers, body) = read_http_request(&mut socket).await;
        assert!(headers.starts_with("POST /v1/payment_intents/pi_mock_cancel_200/cancel HTTP/1.1"));
        assert!(body.contains("cancellation_reason=requested_by_customer"));

        let cancel_resp = r#"{
            "id": "pi_mock_cancel_200",
            "object": "payment_intent",
            "amount": 5000,
            "amount_received": 0,
            "currency": "usd",
            "status": "canceled",
            "capture_method": "manual",
            "cancellation_reason": "requested_by_customer",
            "client_secret": "pi_mock_cancel_200_secret_ClientSecretToken",
            "created": 1728345600,
            "next_action": null
        }"#;
        write_http_json_response(&mut socket, cancel_resp).await;
    });

    let config = StripeConfig::new("sk_test_mock_123").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let canceled = client
        .payment_intents()
        .cancel(
            "pi_mock_cancel_200",
            Some(&CancelPaymentIntentParams {
                cancellation_reason: Some("requested_by_customer".into()),
                idempotency_key: None,
            }),
        )
        .await
        .expect("Failed to cancel payment intent");

    assert_eq!(canceled.id, "pi_mock_cancel_200");
    assert_eq!(canceled.status, "canceled");
    assert_eq!(
        canceled.cancellation_reason.as_deref(),
        Some("requested_by_customer")
    );

    // Gate G2 & PCI-DSS 4.0: client_secret must be masked
    let debug_repr = format!("{canceled:?}");
    assert!(debug_repr.contains("[REDACTED]"));
    assert!(!debug_repr.contains("ClientSecretToken"));

    server.await.expect("Mock server task panicked");
}

#[tokio::test]
async fn test_ac_us_3_1_0_03_idempotent_retry_identical_response() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind listener");
    let address = listener.local_addr().expect("Failed to get local address");

    let expected_idem_key = "idem_key_payment_intents_retry_99999";

    // Spawn mock Stripe server
    let server = tokio::spawn(async move {
        let response_body = r#"{
            "id": "pi_mock_idem_300",
            "object": "payment_intent",
            "amount": 4200,
            "amount_received": 4200,
            "currency": "eur",
            "status": "succeeded",
            "capture_method": "automatic",
            "client_secret": "pi_mock_idem_300_secret_TokenRetry",
            "created": 1728345600,
            "next_action": null,
            "cancellation_reason": null
        }"#;

        // Request 1
        let (mut socket1, _) = listener.accept().await.expect("Accept req1 failed");
        let (headers1, _) = read_http_request(&mut socket1).await;
        assert!(
            headers1
                .to_ascii_lowercase()
                .contains(&format!("idempotency-key: {expected_idem_key}"))
        );
        write_http_json_response(&mut socket1, response_body).await;

        // Request 2 (Retry with identical Idempotency-Key)
        let (mut socket2, _) = listener.accept().await.expect("Accept req2 failed");
        let (headers2, _) = read_http_request(&mut socket2).await;
        assert!(
            headers2
                .to_ascii_lowercase()
                .contains(&format!("idempotency-key: {expected_idem_key}"))
        );
        write_http_json_response(&mut socket2, response_body).await;
    });

    let config = StripeConfig::new("sk_test_mock_123").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let params = CreatePaymentIntentParams::new(4200, "eur")
        .unwrap()
        .capture_method("automatic")
        .description("Idempotency test payload")
        .idempotency_key(expected_idem_key);

    // First attempt
    let pi1 = client
        .payment_intents()
        .create(&params)
        .await
        .expect("First create attempt failed");

    // Second attempt (identical Idempotency-Key)
    let pi2 = client
        .payment_intents()
        .create(&params)
        .await
        .expect("Second create attempt (retry) failed");

    assert_eq!(pi1.id, pi2.id);
    assert_eq!(pi1.amount, pi2.amount);
    assert_eq!(pi1.currency, pi2.currency);
    assert_eq!(pi1.status, pi2.status);
    assert_eq!(pi1.client_secret, pi2.client_secret);
    assert_eq!(pi1.id, "pi_mock_idem_300");

    server.await.expect("Mock server task panicked");
}

#[tokio::test]
async fn test_ac_us_3_1_0_law_3ds_challenge_redirect_and_sdk_action() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind listener");
    let address = listener.local_addr().expect("Failed to get local address");

    // Spawn mock Stripe server
    let server = tokio::spawn(async move {
        // 1. 3DS Redirect challenge response
        let (mut socket1, _) = listener.accept().await.expect("Accept 3DS redirect failed");
        let (headers1, _) = read_http_request(&mut socket1).await;
        assert!(headers1.starts_with("GET /v1/payment_intents/pi_mock_3ds_redirect HTTP/1.1"));

        let redirect_resp = r#"{
            "id": "pi_mock_3ds_redirect",
            "object": "payment_intent",
            "amount": 8500,
            "currency": "eur",
            "status": "requires_action",
            "capture_method": "automatic",
            "client_secret": "pi_mock_3ds_redirect_secret_SecuredToken",
            "created": 1728345600,
            "next_action": {
                "type": "redirect_to_url",
                "redirect_to_url": {
                    "url": "https://hooks.stripe.com/3d_secure_2/hosted_challenge_xyz",
                    "return_url": "https://merchant.example.com/checkout/complete"
                }
            }
        }"#;
        write_http_json_response(&mut socket1, redirect_resp).await;

        // 2. Stripe SDK native challenge response
        let (mut socket2, _) = listener.accept().await.expect("Accept 3DS SDK failed");
        let (headers2, _) = read_http_request(&mut socket2).await;
        assert!(headers2.starts_with("GET /v1/payment_intents/pi_mock_3ds_sdk HTTP/1.1"));

        let sdk_resp = r#"{
            "id": "pi_mock_3ds_sdk",
            "object": "payment_intent",
            "amount": 9500,
            "currency": "gbp",
            "status": "requires_action",
            "capture_method": "automatic",
            "client_secret": "pi_mock_3ds_sdk_secret_SdkToken",
            "created": 1728345600,
            "next_action": {
                "type": "use_stripe_sdk",
                "use_stripe_sdk": {
                    "type": "stripe_3ds2_fingerprint",
                    "directory_server_name": "mastercard",
                    "three_d_secure_2_source": "src_challenge_789"
                }
            }
        }"#;
        write_http_json_response(&mut socket2, sdk_resp).await;
    });

    let config = StripeConfig::new("sk_test_mock_123").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // -----------------------------------------------------------------
    // Part A: Gate G-Law / PSD2 Article 98 3DS Challenge Redirect Flow
    // -----------------------------------------------------------------
    let pi_redirect = client
        .payment_intents()
        .retrieve("pi_mock_3ds_redirect")
        .await
        .expect("Failed to retrieve 3DS redirect intent");

    assert_eq!(pi_redirect.status, "requires_action");
    let next_action = pi_redirect
        .next_action()
        .expect("next_action must be successfully parsed");
    assert_eq!(next_action.r#type, NextActionType::RedirectToUrl);
    assert!(next_action.is_redirect());
    assert!(!next_action.is_stripe_sdk());

    assert_eq!(
        next_action.redirect_url(),
        Some("https://hooks.stripe.com/3d_secure_2/hosted_challenge_xyz")
    );
    assert_eq!(
        next_action.return_url(),
        Some("https://merchant.example.com/checkout/complete")
    );

    // PSD2 Article 98 Statutory Verification: redirect URL must be TLS/HTTPS
    let redirect_details = next_action.redirect_to_url.as_ref().unwrap();
    assert!(redirect_details.is_valid_url());
    assert!(redirect_details.url.starts_with("https://"));

    // -----------------------------------------------------------------
    // Part B: Native Stripe SDK Challenge Flow (`use_stripe_sdk`)
    // -----------------------------------------------------------------
    let pi_sdk = client
        .payment_intents()
        .retrieve("pi_mock_3ds_sdk")
        .await
        .expect("Failed to retrieve 3DS SDK intent");

    assert_eq!(pi_sdk.status, "requires_action");
    let sdk_action = pi_sdk
        .next_action()
        .expect("next_action must be successfully parsed for SDK flow");
    assert_eq!(sdk_action.r#type, NextActionType::UseStripeSdk);
    assert!(sdk_action.is_stripe_sdk());
    assert!(!sdk_action.is_redirect());

    let sdk_payload = sdk_action
        .use_stripe_sdk
        .as_ref()
        .expect("use_stripe_sdk payload must be present");
    assert_eq!(sdk_payload["directory_server_name"], "mastercard");
    assert_eq!(sdk_payload["three_d_secure_2_source"], "src_challenge_789");

    server.await.expect("Mock server task panicked");
}

#[test]
fn test_gate_g_borrow_zero_clone_discipline_on_payment_intent() {
    let pi = PaymentIntent {
        id: "pi_borrow_test_1".into(),
        object: "payment_intent".into(),
        amount: 15000,
        amount_received: Some(15000),
        currency: "usd".into(),
        status: PaymentIntentStatus::Succeeded,
        capture_method: Some(CaptureMethod::Automatic),
        client_secret: Some("pi_borrow_test_secret_SecretKey123".into()),
        next_action: None,
        created: Some(1728345600),
        cancellation_reason: None,
        description: None,
        payment_method_options: None,
    };

    // Helper borrowing the entity by reference (&PaymentIntent)
    fn inspect_intent(intent: &PaymentIntent) -> (&str, i64, PaymentIntentStatus) {
        (intent.id.as_str(), intent.amount, intent.status)
    }

    let (id, amount, status) = inspect_intent(&pi);
    assert_eq!(id, "pi_borrow_test_1");
    assert_eq!(amount, 15000);
    assert_eq!(status, PaymentIntentStatus::Succeeded);
    assert_eq!(status, "succeeded");

    // Gate G2 redaction verification
    let debug = format!("{pi:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("SecretKey123"));
}
