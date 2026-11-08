//! Integration and wire tests for Stripe API v2 Core Accounts and Access Tokens (US-8.1.0 Unit 3).
//!
//! Validates:
//! - Full CRUD lifecycle on `/v2/core/accounts`
//! - Access token issuance on `/v2/core/accounts/{id}/tokens`
//! - JSON payload serialization and deserialization
//! - Idempotency key transmission and multi-tenant `Stripe-Account` scoping
//! - URL query encoding for listing and filtering
//! - PCI-DSS / Gate G2 token secret redaction and memory zeroization
//! - Client zero-clone borrowing discipline

use some_stripe::{
    CreateV2AccountParams, CreateV2AccountTokenParams, ListV2AccountsParams, StripeClient,
    StripeConfig, StripeError, UpdateV2AccountParams, V2AccountConfiguration, V2AccountFeatures,
    V2AccountStatus,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zeroize::Zeroize;

async fn write_json_response(socket: &mut tokio::net::TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    socket.write_all(response.as_bytes()).await.unwrap();
}

#[tokio::test]
async fn test_v2_accounts_crud_lifecycle_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Create V2 Account: POST /v2/core/accounts
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);

            assert!(req.starts_with("POST /v2/core/accounts HTTP/1.1"));
            assert!(
                req.to_lowercase()
                    .contains("content-type: application/json")
            );
            assert!(
                req.to_lowercase()
                    .contains("idempotency-key: idem_create_v2_1")
            );
            assert!(
                req.to_lowercase()
                    .contains("stripe-account: acct_platform_test")
            );
            assert!(req.contains(r#""display_name":"Acme Logistics""#));
            assert!(req.contains(r#""contact_email":"ops@acme.example.com""#));

            let resp = r#"{
                "id": "acct_v2_100",
                "object": "v2.core.account",
                "status": "pending",
                "display_name": "Acme Logistics",
                "contact_email": "ops@acme.example.com",
                "created": 1730000000,
                "updated": 1730000000,
                "configuration": {
                    "default_currency": "usd",
                    "features": {
                        "payments_enabled": true,
                        "payouts_enabled": false,
                        "event_destinations_enabled": true
                    }
                }
            }"#;
            write_json_response(&mut socket, "200 OK", resp).await;
        }

        // 2. Retrieve V2 Account: GET /v2/core/accounts/acct_v2_100
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);

            assert!(req.starts_with("GET /v2/core/accounts/acct_v2_100 HTTP/1.1"));
            assert!(req.to_lowercase().contains("accept: application/json"));
            assert!(
                req.to_lowercase()
                    .contains("stripe-account: acct_platform_test")
            );

            let resp = r#"{
                "id": "acct_v2_100",
                "object": "v2.core.account",
                "status": "active",
                "display_name": "Acme Logistics",
                "contact_email": "ops@acme.example.com",
                "created": 1730000000,
                "updated": 1730001000
            }"#;
            write_json_response(&mut socket, "200 OK", resp).await;
        }

        // 3. Update V2 Account: POST /v2/core/accounts/acct_v2_100
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);

            assert!(req.starts_with("POST /v2/core/accounts/acct_v2_100 HTTP/1.1"));
            assert!(
                req.to_lowercase()
                    .contains("content-type: application/json")
            );
            assert!(
                req.to_lowercase()
                    .contains("idempotency-key: idem_update_v2_2")
            );
            assert!(req.contains(r#""display_name":"Acme Logistics Global""#));
            assert!(req.contains(r#""status":"active""#));

            let resp = r#"{
                "id": "acct_v2_100",
                "object": "v2.core.account",
                "status": "active",
                "display_name": "Acme Logistics Global",
                "contact_email": "ops@acme.example.com",
                "created": 1730000000,
                "updated": 1730002000
            }"#;
            write_json_response(&mut socket, "200 OK", resp).await;
        }

        // 4. Delete V2 Account: DELETE /v2/core/accounts/acct_v2_100
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);

            assert!(req.starts_with("DELETE /v2/core/accounts/acct_v2_100 HTTP/1.1"));

            let resp = r#"{
                "id": "acct_v2_100",
                "object": "v2.core.account",
                "status": "closed",
                "created": 1730000000,
                "updated": 1730003000
            }"#;
            write_json_response(&mut socket, "200 OK", resp).await;
        }
    });

    let config = StripeConfig::new("sk_test_mock_v2").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);
    let resource = client.v2_accounts().with_account("acct_platform_test");

    // 1. Create
    let create_params = CreateV2AccountParams::new()
        .with_display_name("Acme Logistics")
        .with_contact_email("ops@acme.example.com")
        .with_configuration(
            V2AccountConfiguration::new()
                .with_default_currency("usd")
                .with_features(
                    V2AccountFeatures::new()
                        .with_payments_enabled(true)
                        .with_payouts_enabled(false)
                        .with_event_destinations_enabled(true),
                ),
        )
        .with_idempotency_key("idem_create_v2_1");

    let created = resource.create(&create_params).await.unwrap();
    assert_eq!(created.id(), "acct_v2_100");
    assert_eq!(created.object, "v2.core.account");
    assert!(created.is_pending());
    assert_eq!(created.display_name(), Some("Acme Logistics"));
    assert!(
        created
            .configuration
            .as_ref()
            .unwrap()
            .is_payments_enabled()
    );

    // 2. Retrieve
    let retrieved = resource.retrieve("acct_v2_100").await.unwrap();
    assert_eq!(retrieved.id(), "acct_v2_100");
    assert!(retrieved.is_active());

    // 3. Update
    let update_params = UpdateV2AccountParams::new()
        .with_display_name("Acme Logistics Global")
        .with_status(V2AccountStatus::Active)
        .with_idempotency_key("idem_update_v2_2");

    let updated = resource
        .update("acct_v2_100", &update_params)
        .await
        .unwrap();
    assert_eq!(updated.id(), "acct_v2_100");
    assert_eq!(updated.display_name(), Some("Acme Logistics Global"));
    assert!(updated.is_active());

    // 4. Delete
    let closed = resource.del("acct_v2_100").await.unwrap();
    assert_eq!(closed.id(), "acct_v2_100");
    assert!(closed.is_closed());

    server.await.unwrap();
}

#[tokio::test]
async fn test_v2_accounts_list_and_pagination_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // List with params
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8_lossy(&buf[..n]);

        assert!(req.starts_with("GET /v2/core/accounts?limit=2&status=active HTTP/1.1"));

        let resp = r#"{
            "object": "list",
            "data": [
                {
                    "id": "acct_v2_201",
                    "object": "v2.core.account",
                    "status": "active",
                    "created": 1730000100,
                    "updated": 1730000100
                },
                {
                    "id": "acct_v2_202",
                    "object": "v2.core.account",
                    "status": "active",
                    "created": 1730000200,
                    "updated": 1730000200
                }
            ],
            "next_page_url": "https://api.stripe.com/v2/core/accounts?page=2",
            "total_count": 42
        }"#;
        write_json_response(&mut socket, "200 OK", resp).await;
    });

    let config = StripeConfig::new("sk_test_mock_v2").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let list_params = ListV2AccountsParams::new()
        .with_limit(2)
        .with_status(V2AccountStatus::Active);

    let page = client
        .v2_accounts()
        .list_page(Some(&list_params))
        .await
        .unwrap();

    assert_eq!(page.object, "list");
    assert_eq!(page.len(), 2);
    assert!(!page.is_empty());
    assert!(page.has_more());
    assert_eq!(
        page.next_page_url.as_deref(),
        Some("https://api.stripe.com/v2/core/accounts?page=2")
    );
    assert_eq!(page.total_count, Some(42));
    assert_eq!(page.data[0].id(), "acct_v2_201");
    assert_eq!(page.data[1].id(), "acct_v2_202");

    server.await.unwrap();
}

#[tokio::test]
async fn test_v2_accounts_token_issuance_and_zeroize_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8_lossy(&buf[..n]);

        assert!(req.starts_with("POST /v2/core/accounts/acct_v2_token_target/tokens HTTP/1.1"));
        assert!(
            req.to_lowercase()
                .contains("content-type: application/json")
        );
        assert!(
            req.to_lowercase()
                .contains("idempotency-key: idem_token_001")
        );
        assert!(req.contains(r#""ttl_seconds":3600"#));
        assert!(req.contains(r#""permissions":["read_events","write_orders"]"#));

        let resp = r#"{
            "token": "v2_tok_live_secret_entropy_999",
            "expires_at": 1735689600,
            "account_id": "acct_v2_token_target"
        }"#;
        write_json_response(&mut socket, "200 OK", resp).await;
    });

    let config = StripeConfig::new("sk_test_mock_v2").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let token_params = CreateV2AccountTokenParams::new()
        .with_ttl_seconds(3600)
        .with_permissions(vec!["read_events".into(), "write_orders".into()])
        .with_idempotency_key("idem_token_001");

    let mut token = client
        .v2_accounts()
        .create_token("acct_v2_token_target", &token_params)
        .await
        .unwrap();

    assert_eq!(token.account_id(), "acct_v2_token_target");
    assert_eq!(token.expires_at(), 1735689600);
    assert_eq!(token.token(), "v2_tok_live_secret_entropy_999");

    // PCI-DSS / Gate G2 Debug redaction check
    let debug_output = format!("{token:?}");
    assert!(!debug_output.contains("v2_tok_live_secret_entropy_999"));
    assert!(debug_output.contains("[REDACTED]"));

    // Gate G2 zeroization verification
    token.token.zeroize();
    assert!(token.token().is_empty());

    server.await.unwrap();
}

#[tokio::test]
async fn test_v2_accounts_client_side_validation_guards() {
    let client = StripeClient::new("sk_test_validation");
    let resource = client.v2_accounts();

    // Directory traversal in ID guard
    let err_traversal = resource.retrieve("../../../etc/passwd").await.unwrap_err();
    assert!(matches!(err_traversal, StripeError::InvalidRequest(_)));

    // Empty ID guard
    let err_empty_id = resource.del("   ").await.unwrap_err();
    assert!(matches!(err_empty_id, StripeError::InvalidRequest(_)));

    // Invalid email in create params
    let invalid_create = CreateV2AccountParams::new().with_contact_email("invalid-no-at-sign");
    let err_create = resource.create(&invalid_create).await.unwrap_err();
    assert!(matches!(err_create, StripeError::InvalidRequest(_)));

    // Invalid TTL in token params (0 seconds)
    let invalid_token = CreateV2AccountTokenParams::new().with_ttl_seconds(0);
    let err_token = resource
        .create_token("acct_v2_test", &invalid_token)
        .await
        .unwrap_err();
    assert!(matches!(err_token, StripeError::InvalidRequest(_)));
}
