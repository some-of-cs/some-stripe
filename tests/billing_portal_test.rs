//! Integration tests for Stripe Customer Billing Portal lifecycle, security guards, and configurations (Unit 5).

use some_stripe::{
    CreatePortalConfigurationParams, CreatePortalSessionParams, EventData, EventObject,
    PortalBusinessProfile, PortalFeatures, PortalSession, StripeClient, StripeConfig,
    UpdatePortalConfigurationParams, check_portal_session_expiry, is_valid_uuid_v4,
    validate_portal_url,
};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_portal_url_validation_open_redirect_guards() {
    // Valid HTTPS URLs
    assert!(validate_portal_url("https://example.com/account").is_ok());
    assert!(validate_portal_url("https://billing.mycompany.org/return?status=done").is_ok());
    assert!(validate_portal_url("https://sub.domain.co.uk:443/app").is_ok());

    // Insecure HTTP rejected
    let http_err = validate_portal_url("http://insecure.example.com").unwrap_err();
    assert!(http_err.contains("strictly use HTTPS"));

    // Javascript URI rejected
    let js_err = validate_portal_url("javascript:alert(document.cookie)").unwrap_err();
    assert!(js_err.contains("strictly use HTTPS"));

    let js_query_err =
        validate_portal_url("https://example.com?callback=javascript:steal()").unwrap_err();
    assert!(js_query_err.contains("invalid or malicious"));

    // CRLF injection rejected
    let cr_err = validate_portal_url("https://example.com/return\rSet-Cookie:bad").unwrap_err();
    assert!(cr_err.contains("invalid or malicious"));

    let lf_err = validate_portal_url("https://example.com/return\nLocation:evil").unwrap_err();
    assert!(lf_err.contains("invalid or malicious"));

    let crlf_err =
        validate_portal_url("https://example.com/return\r\nContent-Length:0").unwrap_err();
    assert!(crlf_err.contains("invalid or malicious"));

    // Empty URL rejected
    assert!(validate_portal_url("").is_err());
}

#[test]
fn test_short_lived_session_expiry_300s_guard() {
    let created = 1_700_000_000;

    // Immediately after creation
    assert!(!check_portal_session_expiry(created, created));

    // 299 seconds later (still valid)
    assert!(!check_portal_session_expiry(created, created + 299));

    // Exactly 300 seconds later (expired)
    assert!(check_portal_session_expiry(created, created + 300));

    // Beyond 300 seconds (expired)
    assert!(check_portal_session_expiry(created, created + 301));
    assert!(check_portal_session_expiry(created, created + 10_000));

    // Verified on PortalSession entity method
    let session = PortalSession {
        id: "bps_test_expiry".into(),
        object: "billing_portal.session".into(),
        configuration: None,
        created,
        customer: "cus_test_123".into(),
        livemode: Some(false),
        on_behalf_of: None,
        return_url: Some("https://example.com/return".into()),
        url: "https://billing.stripe.com/p/session/test".into(),
    };
    assert!(!session.is_expired(created + 299));
    assert!(session.is_expired(created + 300));
}

#[tokio::test]
async fn test_portal_session_creation_with_idempotency_and_mock_server() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("POST /v1/billing_portal/sessions HTTP/1.1"));
        assert!(req_str.contains("customer=cus_portal_customer_1"));
        assert!(req_str.contains("configuration=bpc_conf_enterprise"));
        assert!(req_str.contains("return_url=https%3A%2F%2Fexample.com%2Faccount"));
        assert!(req_str.contains("on_behalf_of=acct_connected_123"));

        let idemp_header = req_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
            .map(|l| l.split(':').nth(1).unwrap().trim())
            .expect("Idempotency-Key header missing");
        assert!(is_valid_uuid_v4(idemp_header));

        let body = r#"{
            "id": "bps_mock_session_101",
            "object": "billing_portal.session",
            "configuration": "bpc_conf_enterprise",
            "created": 1700000000,
            "customer": "cus_portal_customer_1",
            "livemode": false,
            "on_behalf_of": "acct_connected_123",
            "return_url": "https://example.com/account",
            "url": "https://billing.stripe.com/session/bps_mock_session_101"
        }"#;

        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(resp.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);

    let params = CreatePortalSessionParams::for_customer("cus_portal_customer_1")
        .with_configuration("bpc_conf_enterprise")
        .with_return_url("https://example.com/account")
        .with_on_behalf_of("acct_connected_123");

    let session = client
        .billing_portal()
        .create_session(&params)
        .await
        .unwrap();

    assert_eq!(session.id, "bps_mock_session_101");
    assert_eq!(session.object, "billing_portal.session");
    assert_eq!(session.customer, "cus_portal_customer_1");
    assert_eq!(
        session.configuration.as_deref(),
        Some("bpc_conf_enterprise")
    );
    assert_eq!(
        session.url,
        "https://billing.stripe.com/session/bps_mock_session_101"
    );

    server.await.unwrap();
}

#[tokio::test]
async fn test_portal_session_creation_rejects_insecure_url_without_http_request() {
    let client = StripeClient::new("sk_test_123");

    let params = CreatePortalSessionParams::for_customer("cus_portal_customer_1")
        .with_return_url("http://insecure.com/redirect");

    let err = client
        .billing_portal()
        .create_session(&params)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("strictly use HTTPS"));
}

#[tokio::test]
async fn test_portal_configuration_crud_and_mock_server() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        // 1. Create Configuration
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            assert!(req_str.starts_with("POST /v1/billing_portal/configurations HTTP/1.1"));
            assert!(req_str.contains("default_return_url=https%3A%2F%2Fexample.com%2Fdashboard"));
            assert!(req_str.contains("business_profile%5Bheadline%5D=Acme+Portal"));
            assert!(req_str.contains(
                "business_profile%5Bprivacy_policy_url%5D=https%3A%2F%2Fexample.com%2Fprivacy"
            ));
            assert!(req_str.contains("features%5Bcustomer_update%5D%5Benabled%5D=true"));
            assert!(req_str.contains("features%5Binvoice_history%5D%5Benabled%5D=true"));
            assert!(req_str.contains("metadata%5Btier%5D=premium"));

            let idemp_header = req_str
                .lines()
                .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
                .map(|l| l.split(':').nth(1).unwrap().trim())
                .expect("Idempotency-Key header missing");
            assert!(is_valid_uuid_v4(idemp_header));

            let body = r#"{
                "id": "bpc_mock_config_202",
                "object": "billing_portal.configuration",
                "active": true,
                "business_profile": {
                    "headline": "Acme Portal",
                    "privacy_policy_url": "https://example.com/privacy",
                    "terms_of_service_url": "https://example.com/terms"
                },
                "features": {
                    "customer_update": { "enabled": true },
                    "invoice_history": { "enabled": true },
                    "payment_method_update": { "enabled": true },
                    "subscription_cancel": { "enabled": false }
                },
                "default_return_url": "https://example.com/dashboard",
                "is_default": false,
                "created": 1700000000,
                "metadata": {
                    "tier": "premium"
                }
            }"#;

            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(resp.as_bytes()).await.unwrap();
        }

        // 2. Retrieve Configuration
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            assert!(
                req_str.starts_with(
                    "GET /v1/billing_portal/configurations/bpc_mock_config_202 HTTP/1.1"
                )
            );

            let body = r#"{
                "id": "bpc_mock_config_202",
                "object": "billing_portal.configuration",
                "active": true,
                "default_return_url": "https://example.com/dashboard",
                "created": 1700000000
            }"#;

            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(resp.as_bytes()).await.unwrap();
        }

        // 3. Update Configuration
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            assert!(req_str.starts_with(
                "POST /v1/billing_portal/configurations/bpc_mock_config_202 HTTP/1.1"
            ));
            assert!(req_str.contains("active=false"));
            assert!(req_str.contains("features%5Bsubscription_cancel%5D%5Benabled%5D=true"));

            let body = r#"{
                "id": "bpc_mock_config_202",
                "object": "billing_portal.configuration",
                "active": false,
                "features": {
                    "subscription_cancel": { "enabled": true }
                },
                "created": 1700000000
            }"#;

            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(resp.as_bytes()).await.unwrap();
        }

        // 4. List Configurations
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            assert!(req_str.starts_with("GET /v1/billing_portal/configurations?limit=10 HTTP/1.1"));

            let body = r#"{
                "object": "list",
                "data": [
                    {
                        "id": "bpc_mock_config_202",
                        "object": "billing_portal.configuration",
                        "active": false,
                        "created": 1700000000
                    }
                ],
                "has_more": false,
                "url": "/v1/billing_portal/configurations"
            }"#;

            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(resp.as_bytes()).await.unwrap();
        }
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let portal = client.billing_portal();

    // 1. Create
    let mut meta = HashMap::new();
    meta.insert("tier".into(), "premium".into());

    let create_params = CreatePortalConfigurationParams::new()
        .with_default_return_url("https://example.com/dashboard")
        .with_business_profile(
            PortalBusinessProfile::new()
                .with_headline("Acme Portal")
                .with_privacy_policy_url("https://example.com/privacy")
                .with_terms_of_service_url("https://example.com/terms"),
        )
        .with_features(
            PortalFeatures::new()
                .with_customer_update(true)
                .with_invoice_history(true)
                .with_payment_method_update(true)
                .with_subscription_cancel(false),
        )
        .with_metadata(meta);

    let created_config = portal.create_configuration(&create_params).await.unwrap();
    assert_eq!(created_config.id, "bpc_mock_config_202");
    assert!(created_config.is_active());
    assert_eq!(
        created_config.default_return_url.as_deref(),
        Some("https://example.com/dashboard")
    );

    // 2. Retrieve
    let retrieved = portal
        .retrieve_configuration("bpc_mock_config_202")
        .await
        .unwrap();
    assert_eq!(retrieved.id, "bpc_mock_config_202");

    // 3. Update
    let update_params = UpdatePortalConfigurationParams::new()
        .with_active(false)
        .with_features(PortalFeatures::new().with_subscription_cancel(true));

    let updated = portal
        .update_configuration("bpc_mock_config_202", &update_params)
        .await
        .unwrap();
    assert_eq!(updated.id, "bpc_mock_config_202");
    assert!(!updated.is_active());

    // 4. List
    let list = portal.list_configurations(Some(10)).await.unwrap();
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id, "bpc_mock_config_202");
    assert!(!list.has_more);

    server.await.unwrap();
}

#[test]
fn test_event_object_portal_variants_deserialization() {
    // 1. billing_portal.session event
    let session_json = serde_json::json!({
        "id": "bps_webhook_session_99",
        "object": "billing_portal.session",
        "configuration": "bpc_custom_portal",
        "created": 1735689600,
        "customer": "cus_webhook_cust",
        "livemode": true,
        "return_url": "https://example.com/settings",
        "url": "https://billing.stripe.com/p/session/bps_webhook_session_99"
    });

    let event_data = EventData {
        object: session_json,
    };
    let event_obj = event_data
        .deserialize_object()
        .expect("Failed to deserialize billing_portal.session event");

    match event_obj {
        EventObject::PortalSession(session) => {
            assert_eq!(session.id, "bps_webhook_session_99");
            assert_eq!(session.object, "billing_portal.session");
            assert_eq!(session.customer, "cus_webhook_cust");
            assert_eq!(session.configuration.as_deref(), Some("bpc_custom_portal"));
            assert_eq!(session.livemode, Some(true));
        }
        other => panic!("Expected PortalSession variant, got {other:?}"),
    }

    // 2. billing_portal.configuration event
    let config_json = serde_json::json!({
        "id": "bpc_webhook_config_88",
        "object": "billing_portal.configuration",
        "active": true,
        "created": 1735689600,
        "default_return_url": "https://example.com/home"
    });

    let event_data = EventData {
        object: config_json,
    };
    let event_obj = event_data
        .deserialize_object()
        .expect("Failed to deserialize billing_portal.configuration event");

    match event_obj {
        EventObject::PortalConfiguration(config) => {
            assert_eq!(config.id, "bpc_webhook_config_88");
            assert_eq!(config.object, "billing_portal.configuration");
            assert!(config.is_active());
            assert_eq!(
                config.default_return_url.as_deref(),
                Some("https://example.com/home")
            );
        }
        other => panic!("Expected PortalConfiguration variant, got {other:?}"),
    }
}
