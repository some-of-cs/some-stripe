//! Integration and wire contract tests for Stripe API v2 Meter Event Sessions.

use some_stripe::{
    CreateMeterEventSessionParams, MeterEventSession, MeterEventSessionStatus, SessionTokenDetails,
    StripeClient, StripeConfig, StripeError,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use zeroize::Zeroize;

#[test]
fn test_session_token_details_zeroize_and_redaction() {
    let mut token = SessionTokenDetails::new("sens_live_bearer_token_xyz999");
    assert_eq!(token.expose_token(), "sens_live_bearer_token_xyz999");
    assert_eq!(format!("{token:?}"), "[REDACTED]");
    assert_eq!(format!("{token}"), "[REDACTED]");
    assert_eq!(token.len(), 29);
    assert!(!token.is_empty());

    // Explicit zeroize verification
    token.zeroize();
    assert_eq!(token.expose_token(), "");
    assert!(token.is_empty());
}

#[test]
fn test_session_status_variants_and_display() {
    assert_eq!(MeterEventSessionStatus::Active.as_str(), "active");
    assert_eq!(MeterEventSessionStatus::Active.to_string(), "active");
    assert!(MeterEventSessionStatus::Active.is_active());
    assert!(!MeterEventSessionStatus::Active.is_expired());
    assert!(!MeterEventSessionStatus::Active.is_revoked());

    assert_eq!(MeterEventSessionStatus::Expired.as_str(), "expired");
    assert_eq!(MeterEventSessionStatus::Expired.to_string(), "expired");
    assert!(!MeterEventSessionStatus::Expired.is_active());
    assert!(MeterEventSessionStatus::Expired.is_expired());
    assert!(!MeterEventSessionStatus::Expired.is_revoked());

    assert_eq!(MeterEventSessionStatus::Revoked.as_str(), "revoked");
    assert_eq!(MeterEventSessionStatus::Revoked.to_string(), "revoked");
    assert!(!MeterEventSessionStatus::Revoked.is_active());
    assert!(!MeterEventSessionStatus::Revoked.is_expired());
    assert!(MeterEventSessionStatus::Revoked.is_revoked());
}

#[test]
fn test_meter_event_session_lifespan_helpers() {
    let session = MeterEventSession {
        id: "bms_abc".into(),
        object: "billing.meter_event_session".into(),
        created: 1_700_000_000,
        expires_at: 1_700_000_900,
        authentication_token: SessionTokenDetails::new("secret"),
        status: MeterEventSessionStatus::Active,
        livemode: Some(false),
    };

    assert!(session.is_valid_at(1_700_000_500));
    assert!(!session.is_valid_at(1_700_000_900));
    assert!(!session.is_valid_at(1_700_001_000));
    assert_eq!(session.remaining_seconds(1_700_000_500), 400);
    assert_eq!(session.remaining_seconds(1_700_000_950), 0);
}

#[test]
fn test_create_params_serialization() {
    let params = CreateMeterEventSessionParams::new().with_idempotency_key("req_key_123");
    assert_eq!(params.idempotency_key(), Some("req_key_123"));

    // Ensure idempotency key is skipped from JSON body
    let json = serde_json::to_string(&params).expect("serialize params");
    assert_eq!(json, "{}");
}

#[tokio::test]
async fn test_wire_create_meter_event_session_success() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("POST /v2/billing/meter_event_session HTTP/1.1"));
        assert!(req_str.contains("content-type: application/json"));
        assert!(req_str.contains("idempotency-key: custom_idem_key_001"));

        let body = r#"{
            "id": "bms_live_12345",
            "object": "billing.meter_event_session",
            "created": 1728345600,
            "expires_at": 1728346500,
            "authentication_token": "stream_tok_live_abc123xyz",
            "status": "active",
            "livemode": false
        }"#;

        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let params = CreateMeterEventSessionParams::new().with_idempotency_key("custom_idem_key_001");
    let session = client.meter_event_sessions().create(&params).await.unwrap();

    server.await.unwrap();

    assert_eq!(session.id, "bms_live_12345");
    assert_eq!(session.object, "billing.meter_event_session");
    assert_eq!(session.created, 1728345600);
    assert_eq!(session.expires_at, 1728346500);
    assert_eq!(
        session.authentication_token.expose_token(),
        "stream_tok_live_abc123xyz"
    );
    assert_eq!(session.status, MeterEventSessionStatus::Active);
    assert_eq!(session.livemode, Some(false));
}

#[tokio::test]
async fn test_wire_retrieve_meter_event_session_success() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("GET /v2/billing/meter_event_session/bms_test_999 HTTP/1.1"));

        let body = r#"{
            "id": "bms_test_999",
            "object": "billing.meter_event_session",
            "created": 1728345600,
            "expires_at": 1728346500,
            "authentication_token": "tok_retrieved_secret",
            "status": "active"
        }"#;

        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let session = client
        .meter_event_sessions()
        .retrieve("bms_test_999")
        .await
        .unwrap();

    server.await.unwrap();

    assert_eq!(session.id, "bms_test_999");
    assert_eq!(
        session.authentication_token.expose_token(),
        "tok_retrieved_secret"
    );
    assert!(session.status.is_active());
}

#[tokio::test]
async fn test_wire_expire_meter_event_session_success() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(
            req_str
                .starts_with("POST /v2/billing/meter_event_session/bms_test_exp/expire HTTP/1.1")
        );
        assert!(req_str.contains("content-type: application/json"));
        assert!(req_str.contains("idempotency-key:"));

        let body = r#"{
            "id": "bms_test_exp",
            "object": "billing.meter_event_session",
            "created": 1728345600,
            "expires_at": 1728346500,
            "authentication_token": "tok_expired_secret",
            "status": "expired"
        }"#;

        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let session = client
        .meter_event_sessions()
        .expire("bms_test_exp")
        .await
        .unwrap();

    server.await.unwrap();

    assert_eq!(session.id, "bms_test_exp");
    assert_eq!(session.status, MeterEventSessionStatus::Expired);
}

#[tokio::test]
async fn test_path_traversal_validation() {
    let config = StripeConfig::new("sk_test_mock");
    let client = StripeClient::with_config(config);

    let err = client
        .meter_event_sessions()
        .retrieve("bms/../../traversal")
        .await
        .unwrap_err();

    match err {
        StripeError::InvalidRequest(msg) => {
            assert!(msg.contains("traversal") || msg.contains("invalid") || msg.contains('/'));
        }
        other => panic!("expected InvalidRequest, got {other:?}"),
    }

    let err_expire = client
        .meter_event_sessions()
        .expire("   ")
        .await
        .unwrap_err();

    match err_expire {
        StripeError::InvalidRequest(msg) => {
            assert!(msg.contains("empty") || msg.contains("whitespace"));
        }
        other => panic!("expected InvalidRequest, got {other:?}"),
    }
}
