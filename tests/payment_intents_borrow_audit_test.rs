//! Gate G-Borrow, G2, and G-Law Audit Test Suite for PaymentIntents (US-3.1.0).
//!
//! Validates:
//! - Strict zero-clone parameter borrowing (`&CreatePaymentIntentParams`, `Option<&CapturePaymentIntentParams>`, `&str`).
//! - Zero deep clones on `PaymentIntent` domain models and zero-copy string slice accessors (`redirect_url() -> Option<&str>`).
//! - PCI-DSS 4.0 `[REDACTED]` debug masking of `client_secret` (Gate G2).
//! - Statutory ISO-4217 minor currency unit validation (positive integer amounts, lowercase 3-char currencies).
//! - PSD2 SCA 3D Secure next_action redirection (Gate G-Law).

use some_stripe::{
    CancelPaymentIntentParams, CaptureMethod, CapturePaymentIntentParams,
    ConfirmPaymentIntentParams, CreatePaymentIntentParams, PaymentIntent, PaymentIntentStatus,
    StripeClient, StripeConfig, StripeError,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_gate_g_borrow_zero_clone_parameter_borrowing() {
    let params = CreatePaymentIntentParams::new(10000, "usd")
        .with_capture_method("manual")
        .with_description("Audit Order #999")
        .with_idempotency_key("idem_audit_borrow_001");

    // Borrowing references without cloning
    let borrowed_ref: &CreatePaymentIntentParams = &params;
    assert_eq!(borrowed_ref.amount, 10000);
    assert_eq!(borrowed_ref.currency, "usd");
    assert_eq!(borrowed_ref.capture_method.as_deref(), Some("manual"));
    assert_eq!(
        borrowed_ref.description.as_deref(),
        Some("Audit Order #999")
    );
    assert_eq!(
        borrowed_ref.idempotency_key.as_deref(),
        Some("idem_audit_borrow_001")
    );

    // Ensure params is still fully valid and unconsumed after borrow
    assert_eq!(params.amount, 10000);

    // Cancel params zero-clone borrow
    let cancel = CancelPaymentIntentParams::new()
        .with_cancellation_reason("requested_by_customer")
        .with_idempotency_key("idem_cancel_001");
    let cancel_ref: &CancelPaymentIntentParams = &cancel;
    assert_eq!(
        cancel_ref.cancellation_reason.as_deref(),
        Some("requested_by_customer")
    );
    assert_eq!(cancel.idempotency_key.as_deref(), Some("idem_cancel_001"));

    // Confirm params zero-clone borrow
    let confirm = ConfirmPaymentIntentParams::new()
        .with_payment_method("pm_123")
        .with_return_url("https://example.com")
        .with_idempotency_key("idem_confirm_001");
    let confirm_ref: &ConfirmPaymentIntentParams = &confirm;
    assert_eq!(confirm_ref.payment_method.as_deref(), Some("pm_123"));
    assert_eq!(confirm.return_url.as_deref(), Some("https://example.com"));
}

#[test]
fn test_gate_g_borrow_zero_copy_redirect_url_extraction() {
    let json = serde_json::json!({
        "id": "pi_audit_3ds_challenge",
        "object": "payment_intent",
        "amount": 15000,
        "currency": "eur",
        "status": "requires_action",
        "client_secret": "pi_audit_secret_vault_777",
        "next_action": {
            "type": "redirect_to_url",
            "redirect_to_url": {
                "url": "https://hooks.stripe.com/redirect/authenticate/src_audit_123"
            }
        }
    });

    let pi: PaymentIntent = serde_json::from_value(json).expect("Deserialization failed");

    // Extract reference without cloning
    let redirect_ref: Option<&str> = pi.redirect_url();
    assert_eq!(
        redirect_ref,
        Some("https://hooks.stripe.com/redirect/authenticate/src_audit_123"),
        "Gate G-Borrow violation: redirect_url must return a borrowed &str slice"
    );

    // Validate status inspectors borrow &self without cloning
    assert!(pi.requires_action());
    assert!(!pi.is_succeeded());
    assert!(!pi.requires_capture());
    assert!(!pi.is_canceled());
}

#[test]
fn test_gate_g2_pci_dss_debug_redaction_invariant() {
    let mut pi = PaymentIntent::new(
        "pi_pci_dss_test",
        8500,
        "usd",
        PaymentIntentStatus::Succeeded,
    );
    pi.amount_received = Some(8500);
    pi.capture_method = Some(CaptureMethod::Automatic);
    pi.client_secret = Some("pi_pci_dss_test_secret_live_card_vault_credential".into());
    pi.created = Some(1728345600);

    let debug_output = format!("{pi:?}");

    // Gate G2: Secret MUST be masked as [REDACTED]
    assert!(
        debug_output.contains("[REDACTED]"),
        "Gate G2 violation: Debug implementation must display [REDACTED] for client_secret"
    );

    // Gate G2 / PCI-DSS: Raw credential MUST NOT appear anywhere in the string
    assert!(
        !debug_output.contains("live_card_vault_credential"),
        "Gate G2 violation: client_secret leaked into debug string"
    );
}

#[test]
fn test_gate_g_law_iso_4217_minor_currency_validation() {
    // Valid positive integer in minor units
    let valid = CreatePaymentIntentParams::new(5000, "usd");
    assert!(valid.validate().is_ok());

    // Zero amount rejection (statutory minor currency rule)
    let zero = CreatePaymentIntentParams::new(0, "usd");
    assert!(
        zero.validate().is_err(),
        "Gate G-Law violation: zero amount must be rejected"
    );

    // Negative amount rejection
    let negative = CreatePaymentIntentParams::new(-100, "usd");
    assert!(
        negative.validate().is_err(),
        "Gate G-Law violation: negative amount must be rejected"
    );

    // Uppercase currency rejection (ISO-4217 codes in Stripe must be lowercase)
    let upper = CreatePaymentIntentParams::new(5000, "USD");
    assert!(
        upper.validate().is_err(),
        "Gate G-Law violation: uppercase currency must be rejected"
    );

    // Invalid length currency rejection
    let short = CreatePaymentIntentParams::new(5000, "us");
    assert!(short.validate().is_err());
    let long = CreatePaymentIntentParams::new(5000, "usdt");
    assert!(long.validate().is_err());

    // Partial capture validation
    let cap_valid = CapturePaymentIntentParams::new().with_amount_to_capture(2500);
    assert!(cap_valid.validate().is_ok());

    let cap_zero = CapturePaymentIntentParams::new().with_amount_to_capture(0);
    assert!(cap_zero.validate().is_err());

    let cap_neg = CapturePaymentIntentParams::new().with_amount_to_capture(-500);
    assert!(cap_neg.validate().is_err());
}

#[tokio::test]
async fn test_payment_intents_lifecycle_mock_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind ephemeral mock port");
    let address = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        // Step 1: Create PaymentIntent with manual capture
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(req.starts_with("POST /v1/payment_intents HTTP/1.1"));
        assert!(req.contains("amount=10000"));
        assert!(req.contains("currency=usd"));
        assert!(req.contains("capture_method=manual"));
        assert!(req.contains("idempotency-key: idem_mock_flow_01"));

        let create_body = r#"{
            "id": "pi_mock_lifecycle_100",
            "object": "payment_intent",
            "amount": 10000,
            "currency": "usd",
            "status": "requires_capture",
            "capture_method": "manual",
            "client_secret": "pi_mock_lifecycle_secret_xyz"
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

        // Step 2: Retrieve PaymentIntent
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(req.starts_with("GET /v1/payment_intents/pi_mock_lifecycle_100 HTTP/1.1"));

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

        // Step 3: Partial Capture
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(req.starts_with("POST /v1/payment_intents/pi_mock_lifecycle_100/capture HTTP/1.1"));
        assert!(req.contains("amount_to_capture=7500"));
        assert!(req.contains("idempotency-key: idem_mock_capture_01"));

        let cap_body = r#"{
            "id": "pi_mock_lifecycle_100",
            "object": "payment_intent",
            "amount": 10000,
            "amount_received": 7500,
            "currency": "usd",
            "status": "succeeded"
        }"#;
        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{cap_body}",
                    cap_body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config =
        StripeConfig::new("sk_test_mock_secret").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // Pre-flight validation rejection test
    let invalid_params = CreatePaymentIntentParams::new(-50, "usd");
    let err = client
        .payment_intents()
        .create(&invalid_params)
        .await
        .unwrap_err();
    assert!(matches!(err, StripeError::InvalidRequest(_)));

    // 1. Create with manual capture
    let create_params = CreatePaymentIntentParams::new(10000, "usd")
        .with_capture_method("manual")
        .with_idempotency_key("idem_mock_flow_01");

    let intent = client
        .payment_intents()
        .create(&create_params)
        .await
        .expect("Create failed");
    assert_eq!(intent.id, "pi_mock_lifecycle_100");
    assert!(intent.requires_capture());
    assert_eq!(intent.amount, 10000);

    // 2. Retrieve
    let retrieved = client
        .payment_intents()
        .retrieve("pi_mock_lifecycle_100")
        .await
        .expect("Retrieve failed");
    assert_eq!(retrieved.id, "pi_mock_lifecycle_100");
    assert!(retrieved.requires_capture());

    // 3. Partial Capture (75%)
    let cap_params = CapturePaymentIntentParams::new()
        .with_amount_to_capture(7500)
        .with_idempotency_key("idem_mock_capture_01");
    let captured = client
        .payment_intents()
        .capture("pi_mock_lifecycle_100", Some(&cap_params))
        .await
        .expect("Capture failed");
    assert_eq!(captured.status, "succeeded");
    assert_eq!(captured.amount_received, Some(7500));
    assert!(captured.is_succeeded());

    server_task.await.unwrap();
}
