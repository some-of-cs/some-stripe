//! Integration tests for PaymentIntents CRUD operations and automatic idempotency key injection.

use some_stripe::{
    CancelPaymentIntentParams, CapturePaymentIntentParams, ConfirmPaymentIntentParams,
    CreatePaymentIntentParams, StripeClient, StripeConfig, is_valid_uuid_v4,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_payment_intent_full_crud_lifecycle_and_idempotency() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Step 1: Create PaymentIntent (auto UUID v4 idempotency key)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/payment_intents HTTP/1.1"));
            let header = req_str
                .lines()
                .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
                .expect("Idempotency-Key must be injected on create");
            let key = header.split(':').nth(1).unwrap().trim();
            assert!(
                is_valid_uuid_v4(key),
                "Injected create key must be RFC 4122 UUID v4: {key}"
            );
            assert!(req_str.contains("amount=10000"));
            assert!(req_str.contains("currency=usd"));
            assert!(req_str.contains("capture_method=manual"));

            let resp = r#"{"id":"pi_crud_100","object":"payment_intent","amount":10000,"currency":"usd","status":"requires_payment_method","capture_method":"manual"}"#;
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
        }

        // Step 2: Retrieve PaymentIntent (MUST NOT have Idempotency-Key header)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("GET /v1/payment_intents/pi_crud_100 HTTP/1.1"));
            assert!(
                !req_str.to_lowercase().contains("idempotency-key"),
                "GET retrieve request must NOT inject Idempotency-Key"
            );

            let resp = r#"{"id":"pi_crud_100","object":"payment_intent","amount":10000,"currency":"usd","status":"requires_payment_method","capture_method":"manual"}"#;
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
        }

        // Step 3: Confirm PaymentIntent (with custom user idempotency key)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/payment_intents/pi_crud_100/confirm HTTP/1.1"));
            assert!(
                req_str.contains("idempotency-key: custom_confirm_token_abc"),
                "User idempotency key must be preserved on confirm"
            );
            assert!(req_str.contains("payment_method=pm_card_visa"));
            assert!(req_str.contains("setup_future_usage=off_session"));

            let resp = r#"{"id":"pi_crud_100","object":"payment_intent","amount":10000,"currency":"usd","status":"requires_capture","capture_method":"manual"}"#;
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
        }

        // Step 4: Capture Partial Amount (auto UUID v4 idempotency key)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/payment_intents/pi_crud_100/capture HTTP/1.1"));
            let header = req_str
                .lines()
                .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
                .expect("Idempotency-Key must be injected on capture");
            let key = header.split(':').nth(1).unwrap().trim();
            assert!(
                is_valid_uuid_v4(key),
                "Injected capture key must be RFC 4122 UUID v4: {key}"
            );
            assert!(req_str.contains("amount_to_capture=7500"));

            let resp = r#"{"id":"pi_crud_100","object":"payment_intent","amount":10000,"amount_received":7500,"currency":"usd","status":"succeeded"}"#;
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
        }

        // Step 5: Cancel another uncaptured Intent (with user idempotency key & reason)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/payment_intents/pi_crud_200/cancel HTTP/1.1"));
            assert!(
                req_str.contains("idempotency-key: custom_cancel_token_xyz"),
                "User idempotency key must be preserved on cancel"
            );
            assert!(req_str.contains("cancellation_reason=requested_by_customer"));

            let resp = r#"{"id":"pi_crud_200","object":"payment_intent","amount":5000,"currency":"usd","status":"canceled","cancellation_reason":"requested_by_customer"}"#;
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
        }
    });

    let config = StripeConfig::new("sk_test_crud_key").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // 1. Create
    let create_params = CreatePaymentIntentParams {
        amount: 10000,
        currency: "usd".into(),
        capture_method: Some("manual".into()),
        idempotency_key: None,
        ..Default::default()
    };
    let created = client
        .payment_intents()
        .create(&create_params)
        .await
        .unwrap();
    assert_eq!(created.id, "pi_crud_100");
    assert_eq!(created.amount, 10000);
    assert_eq!(created.status, "requires_payment_method");

    // 2. Retrieve
    let retrieved = client
        .payment_intents()
        .retrieve("pi_crud_100")
        .await
        .unwrap();
    assert_eq!(retrieved.id, "pi_crud_100");
    assert_eq!(retrieved.status, "requires_payment_method");

    // 3. Confirm
    let confirm_params = ConfirmPaymentIntentParams {
        payment_method: Some("pm_card_visa".into()),
        setup_future_usage: Some("off_session".into()),
        idempotency_key: Some("custom_confirm_token_abc".into()),
        ..Default::default()
    };
    let confirmed = client
        .payment_intents()
        .confirm("pi_crud_100", Some(&confirm_params))
        .await
        .unwrap();
    assert_eq!(confirmed.status, "requires_capture");

    // 4. Capture
    let capture_params = CapturePaymentIntentParams {
        amount_to_capture: Some(7500),
        idempotency_key: None,
    };
    let captured = client
        .payment_intents()
        .capture("pi_crud_100", Some(&capture_params))
        .await
        .unwrap();
    assert_eq!(captured.status, "succeeded");
    assert_eq!(captured.amount_received, Some(7500));

    // 5. Cancel
    let cancel_params = CancelPaymentIntentParams {
        cancellation_reason: Some("requested_by_customer".into()),
        idempotency_key: Some("custom_cancel_token_xyz".into()),
    };
    let canceled = client
        .payment_intents()
        .cancel("pi_crud_200", Some(&cancel_params))
        .await
        .unwrap();
    assert_eq!(canceled.id, "pi_crud_200");
    assert_eq!(canceled.status, "canceled");
    assert_eq!(
        canceled.cancellation_reason.as_deref(),
        Some("requested_by_customer")
    );

    server.await.unwrap();
}
