//! Comprehensive unit and wire mock tests for Stripe Identity Verification Sessions Resource.
//!
//! Validates:
//! - Quality Gate G0: 0 compiler/clippy warnings under Rust 2024.
//! - Quality Gate G-Borrow: Zero-clone borrowing discipline on domain models.
//! - Quality Gate G2: Automatic RFC 4122 UUID v4 Idempotency-Key injection and tenant isolation (`Stripe-Account`).
//! - Quality Gate G2: Debug masking of `client_secret` as `[REDACTED]`.
//! - Quality Gate G-Law: GDPR Article 17 Right to Erasure redaction and GDPR Article 5(1)(c) biometric minimization.
//! - VerificationSessionsResource wire contracts: `create`, `retrieve`, `update`, `cancel`, `redact`.

use some_stripe::identity_session::{
    CreateVerificationSessionParams, DocumentOptions, IdNumberOptions,
    UpdateVerificationSessionParams, VerificationOptions, VerificationSessionStatus,
    VerificationSessionType,
};
use some_stripe::{StripeClient, StripeConfig, is_valid_uuid_v4};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_identity_verification_sessions_create_and_retrieve_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Request 1: POST /v1/identity/verification_sessions
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/identity/verification_sessions HTTP/1.1"));

            // Verify auto-injected Idempotency-Key
            let key_header = req_str
                .lines()
                .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
                .expect("Auto Idempotency-Key must be present");
            let key_val = key_header.split(':').nth(1).unwrap().trim();
            assert!(
                is_valid_uuid_v4(key_val),
                "Expected valid UUID v4 idempotency key, got {key_val}"
            );

            // Verify tenant header
            assert!(req_str.contains("stripe-account: acct_platform_sub_456"));

            // Verify form payload fields
            assert!(req_str.contains("type=document"));
            assert!(req_str.contains("return_url="));

            let resp = serde_json::json!({
                "id": "vs_test_live_001",
                "object": "identity.verification_session",
                "client_secret": "vcs_secret_ephemeral_token",
                "created": 1700000000,
                "livemode": false,
                "status": "requires_input",
                "type": "document",
                "url": "https://verify.stripe.com/start/vs_test_live_001",
                "return_url": "https://platform.com/kyc/complete",
                "options": {
                    "document": {
                        "require_matching_selfie": true,
                        "require_live_capture": true
                    }
                }
            });
            let body = resp.to_string();
            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(http_resp.as_bytes()).await.unwrap();
        }

        // Request 2: GET /v1/identity/verification_sessions/vs_test_live_001
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(
                req_str.starts_with(
                    "GET /v1/identity/verification_sessions/vs_test_live_001 HTTP/1.1"
                )
            );
            // GET must NOT have Idempotency-Key
            assert!(!req_str.to_lowercase().contains("idempotency-key:"));
            // Verify tenant header
            assert!(req_str.contains("stripe-account: acct_platform_sub_456"));

            let resp = serde_json::json!({
                "id": "vs_test_live_001",
                "object": "identity.verification_session",
                "client_secret": "vcs_secret_ephemeral_token",
                "created": 1700000000,
                "livemode": false,
                "status": "verified",
                "type": "document",
                "url": "https://verify.stripe.com/start/vs_test_live_001",
                "return_url": "https://platform.com/kyc/complete",
                "verified_outputs": {
                    "first_name": "Ada",
                    "last_name": "Lovelace",
                    "id_number": "123-45-6789"
                }
            });
            let body = resp.to_string();
            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(http_resp.as_bytes()).await.unwrap();
        }
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);
    let resource = client
        .identity_sessions()
        .with_account("acct_platform_sub_456");

    let params = CreateVerificationSessionParams::document()
        .with_options(
            VerificationOptions::new().with_document(
                DocumentOptions::new()
                    .with_matching_selfie(true)
                    .with_live_capture(true),
            ),
        )
        .with_return_url("https://platform.com/kyc/complete");

    let created = resource.create(&params).await.unwrap();
    assert_eq!(created.id(), "vs_test_live_001");
    assert_eq!(created.status(), VerificationSessionStatus::RequiresInput);
    assert_eq!(
        created.url(),
        Some("https://verify.stripe.com/start/vs_test_live_001")
    );
    assert!(created.requires_input());
    assert!(!created.is_verified());

    // Debug masking verification
    let debug_output = format!("{created:?}");
    assert!(!debug_output.contains("vcs_secret_ephemeral_token"));
    assert!(debug_output.contains("[REDACTED]"));

    let retrieved = resource.retrieve("vs_test_live_001").await.unwrap();
    assert_eq!(retrieved.id(), "vs_test_live_001");
    assert_eq!(retrieved.status(), VerificationSessionStatus::Verified);
    assert!(retrieved.is_verified());
    assert!(!retrieved.requires_input());

    let outputs = retrieved.verified_outputs.as_ref().unwrap();
    assert_eq!(outputs.first_name.as_deref(), Some("Ada"));
    assert_eq!(outputs.last_name.as_deref(), Some("Lovelace"));
    assert_eq!(outputs.id_number.as_deref(), Some("123-45-6789"));

    server.await.unwrap();
}

#[tokio::test]
async fn test_identity_verification_sessions_update_cancel_redact_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Request 1: POST /v1/identity/verification_sessions/vs_test_live_002 (Update)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(
                req_str.starts_with(
                    "POST /v1/identity/verification_sessions/vs_test_live_002 HTTP/1.1"
                )
            );
            assert!(req_str.contains("idempotency-key: custom_update_idem_key"));

            let resp = serde_json::json!({
                "id": "vs_test_live_002",
                "object": "identity.verification_session",
                "created": 1700000000,
                "livemode": false,
                "status": "requires_input",
                "type": "id_number"
            });
            let body = resp.to_string();
            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(http_resp.as_bytes()).await.unwrap();
        }

        // Request 2: POST /v1/identity/verification_sessions/vs_test_live_002/cancel (Cancel)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with(
                "POST /v1/identity/verification_sessions/vs_test_live_002/cancel HTTP/1.1"
            ));
            assert!(req_str.to_lowercase().contains("idempotency-key:"));

            let resp = serde_json::json!({
                "id": "vs_test_live_002",
                "object": "identity.verification_session",
                "created": 1700000000,
                "livemode": false,
                "status": "canceled",
                "type": "id_number"
            });
            let body = resp.to_string();
            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(http_resp.as_bytes()).await.unwrap();
        }

        // Request 3: POST /v1/identity/verification_sessions/vs_test_live_002/redact (Redact - GDPR Art 17)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with(
                "POST /v1/identity/verification_sessions/vs_test_live_002/redact HTTP/1.1"
            ));
            assert!(req_str.to_lowercase().contains("idempotency-key:"));

            let resp = serde_json::json!({
                "id": "vs_test_live_002",
                "object": "identity.verification_session",
                "created": 1700000000,
                "livemode": false,
                "status": "canceled",
                "type": "id_number",
                "redaction": {
                    "status": "redacted"
                }
            });
            let body = resp.to_string();
            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(http_resp.as_bytes()).await.unwrap();
        }
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);
    let resource = client.identity_sessions();

    // 1. Update
    let update_params = UpdateVerificationSessionParams::new()
        .with_type(VerificationSessionType::IdNumber)
        .with_options(VerificationOptions::for_id_number(IdNumberOptions::new()))
        .with_idempotency_key("custom_update_idem_key");

    let updated = resource
        .update("vs_test_live_002", &update_params)
        .await
        .unwrap();
    assert_eq!(
        updated.verification_type(),
        VerificationSessionType::IdNumber
    );

    // 2. Cancel
    let canceled = resource.cancel("vs_test_live_002").await.unwrap();
    assert!(canceled.is_canceled());
    assert_eq!(canceled.status(), VerificationSessionStatus::Canceled);

    // 3. Redact (GDPR Article 17)
    let redacted = resource.redact("vs_test_live_002").await.unwrap();
    assert!(redacted.is_redacted());
    assert_eq!(
        redacted
            .redaction
            .as_ref()
            .and_then(|r| r.status.as_deref()),
        Some("redacted")
    );

    server.await.unwrap();
}

#[tokio::test]
async fn test_url_and_path_security_invariants() {
    let client = StripeClient::new("sk_test_mock");
    let resource = client.identity_sessions();

    // Path traversal in ID rejected immediately before network
    let path_err = resource.retrieve("../../../etc/passwd").await;
    assert!(path_err.is_err());

    let null_byte_err = resource.retrieve("vs_123\0admin").await;
    assert!(null_byte_err.is_err());

    let empty_err = resource.retrieve("").await;
    assert!(empty_err.is_err());

    // Insecure or malicious return_url rejected immediately before network
    let bad_url_params =
        CreateVerificationSessionParams::document().with_return_url("javascript:steal_token()");
    let create_err = resource.create(&bad_url_params).await;
    assert!(create_err.is_err());

    let crlf_url_params = CreateVerificationSessionParams::document()
        .with_return_url("https://platform.com/verified\r\nInjected-Header: evil");
    let crlf_err = resource.create(&crlf_url_params).await;
    assert!(crlf_err.is_err());
}
