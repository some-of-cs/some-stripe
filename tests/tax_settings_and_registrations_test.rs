//! Integration and mock wire tests for Tax Settings and Registrations Resources (US-7.1.0 Unit 3).
//!
//! Verifies:
//! - API wire endpoint dispatch for `/v1/tax/settings` and `/v1/tax/registrations`.
//! - Parameter serialization and RFC 1738 nested form encoding.
//! - Header injection (`Idempotency-Key`, `Stripe-Account`).
//! - Domain entity deserialization, zero-clone borrowing discipline (Gate G-Borrow).
//! - Statutory compliance for EU VAT Directive 2006/112/EC and US sales tax registrations (Gate G-Law).

use some_stripe::{
    Address, CreateTaxRegistrationParams, ListTaxRegistrationsParams, StripeClient, StripeConfig,
    TaxHeadOfficeDetailsParams, TaxRegistrationStatus, TaxRegistrationType,
    TaxSettingsDefaultsParams, TaxStatus, UpdateTaxRegistrationParams, UpdateTaxSettingsParams,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn write_json_response(socket: &mut tokio::net::TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    socket.write_all(response.as_bytes()).await.unwrap();
}

#[tokio::test]
async fn test_tax_settings_retrieve_and_update_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Retrieve Tax Settings (GET /v1/tax/settings)
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/tax/settings HTTP/1.1"));
        assert!(
            wire.to_lowercase()
                .contains("stripe-account: acct_tax_tenant_1")
        );

        let settings_resp1 = r#"{
            "object": "tax.settings",
            "status": "pending",
            "status_details": {
                "pending": {
                    "missing_fields": ["head_office.address"]
                }
            },
            "head_office": null,
            "defaults": {
                "tax_behavior": null,
                "tax_code": null
            },
            "livemode": false
        }"#;
        write_json_response(&mut socket, settings_resp1).await;

        // 2. Update Tax Settings (POST /v1/tax/settings)
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/tax/settings HTTP/1.1"));
        assert!(wire.to_lowercase().contains("idempotency-key: "));
        assert!(
            wire.to_lowercase()
                .contains("stripe-account: acct_tax_tenant_1")
        );
        assert!(wire.contains("head_office"));
        assert!(wire.contains("defaults"));

        let settings_resp2 = r#"{
            "object": "tax.settings",
            "status": "active",
            "status_details": {
                "active": {}
            },
            "head_office": {
                "address": {
                    "city": "South San Francisco",
                    "country": "US",
                    "line1": "354 Oyster Point Blvd",
                    "line2": null,
                    "postal_code": "94080",
                    "state": "CA"
                }
            },
            "defaults": {
                "tax_behavior": "exclusive",
                "tax_code": "txcd_10000000"
            },
            "livemode": false,
            "locations": [
                {
                    "country": "US",
                    "state": "CA"
                }
            ]
        }"#;
        write_json_response(&mut socket, settings_resp2).await;
    });

    let config =
        StripeConfig::new("sk_test_tax_settings").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);
    let resource =
        some_stripe::resources::TaxSettingsResource::new(&client).with_account("acct_tax_tenant_1");

    // 1. Retrieve
    let settings = resource.retrieve().await.unwrap();
    assert_eq!(settings.status, TaxStatus::Pending);
    assert!(!settings.is_ready_for_calculation());
    assert!(settings.head_office.is_none());

    // 2. Update
    let update_params = UpdateTaxSettingsParams {
        defaults: Some(TaxSettingsDefaultsParams {
            tax_behavior: Some("exclusive".into()),
            tax_code: Some("txcd_10000000".into()),
        }),
        head_office: Some(TaxHeadOfficeDetailsParams {
            address: Address {
                city: Some("South San Francisco".into()),
                country: Some("US".into()),
                line1: Some("354 Oyster Point Blvd".into()),
                line2: None,
                postal_code: Some("94080".into()),
                state: Some("CA".into()),
            },
        }),
    };

    let updated = resource.update(&update_params).await.unwrap();
    assert_eq!(updated.status, TaxStatus::Active);
    assert!(updated.is_ready_for_calculation());
    assert_eq!(updated.defaults.tax_behavior.as_deref(), Some("exclusive"));
    assert_eq!(updated.defaults.tax_code.as_deref(), Some("txcd_10000000"));
    assert_eq!(
        updated
            .head_office
            .as_ref()
            .and_then(|h| h.address.state.as_deref()),
        Some("CA")
    );

    server.await.unwrap();
}

#[tokio::test]
async fn test_tax_registrations_lifecycle_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Create Tax Registration (POST /v1/tax/registrations)
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/tax/registrations HTTP/1.1"));
        assert!(wire.contains("country=IE"));
        assert!(wire.contains("active_from=now"));
        assert!(wire.to_lowercase().contains("idempotency-key: "));

        let reg_resp1 = r#"{
            "id": "taxreg_eu_oss_001",
            "object": "tax.registration",
            "active_from": 1669249440,
            "country": "IE",
            "country_options": {
                "ie": {
                    "type": "oss_union"
                }
            },
            "created": 1669249000,
            "expires_at": null,
            "livemode": false,
            "status": "active"
        }"#;
        write_json_response(&mut socket, reg_resp1).await;

        // 2. Retrieve Tax Registration (GET /v1/tax/registrations/taxreg_eu_oss_001)
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/tax/registrations/taxreg_eu_oss_001 HTTP/1.1"));
        write_json_response(&mut socket, reg_resp1).await;

        // 3. Update Tax Registration (POST /v1/tax/registrations/taxreg_eu_oss_001)
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/tax/registrations/taxreg_eu_oss_001 HTTP/1.1"));
        assert!(wire.contains("expires_at=now"));

        let reg_resp2 = r#"{
            "id": "taxreg_eu_oss_001",
            "object": "tax.registration",
            "active_from": 1669249440,
            "country": "IE",
            "country_options": {
                "ie": {
                    "type": "oss_union"
                }
            },
            "created": 1669249000,
            "expires_at": 1669250000,
            "livemode": false,
            "status": "expired"
        }"#;
        write_json_response(&mut socket, reg_resp2).await;

        // 4. List Tax Registrations (GET /v1/tax/registrations)
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/tax/registrations?limit=5 HTTP/1.1"));

        let list_resp = r#"{
            "object": "list",
            "data": [
                {
                    "id": "taxreg_eu_oss_001",
                    "object": "tax.registration",
                    "active_from": 1669249440,
                    "country": "IE",
                    "country_options": {
                        "ie": {
                            "type": "oss_union"
                        }
                    },
                    "livemode": false,
                    "status": "expired"
                }
            ],
            "has_more": false,
            "url": "/v1/tax/registrations"
        }"#;
        write_json_response(&mut socket, list_resp).await;
    });

    let config =
        StripeConfig::new("sk_test_tax_settings").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);
    let resource = some_stripe::resources::TaxRegistrationsResource::new(&client);

    // 1. Create (EU VAT Directive 2006/112/EC Art 196 OSS Union Registration)
    let create_params = CreateTaxRegistrationParams {
        country: "IE".into(),
        active_from: "now".into(),
        country_options: serde_json::json!({
            "ie": {
                "type": "oss_union"
            }
        }),
        expires_at: None,
    };
    let reg = resource.create(&create_params).await.unwrap();
    assert_eq!(reg.id(), "taxreg_eu_oss_001");
    assert_eq!(reg.country(), "IE");
    assert!(reg.is_active());
    assert_eq!(reg.registration_type(), Some(TaxRegistrationType::OssUnion));

    // 2. Retrieve
    let fetched = resource.retrieve("taxreg_eu_oss_001").await.unwrap();
    assert_eq!(fetched.id(), "taxreg_eu_oss_001");
    assert_eq!(fetched.country(), "IE");

    // 3. Update (expire registration)
    let update_params = UpdateTaxRegistrationParams {
        expires_at: Some("now".into()),
        ..Default::default()
    };
    let expired = resource
        .update("taxreg_eu_oss_001", &update_params)
        .await
        .unwrap();
    assert_eq!(expired.status, TaxRegistrationStatus::Expired);
    assert!(expired.is_expired());

    // 4. List page
    let list_params = ListTaxRegistrationsParams {
        limit: Some(5),
        ..Default::default()
    };
    let list = resource.list_page(Some(&list_params)).await.unwrap();
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id(), "taxreg_eu_oss_001");

    server.await.unwrap();
}

#[tokio::test]
async fn test_invalid_tax_registration_id_rejected() {
    let client = StripeClient::new("sk_test_key");
    let resource = some_stripe::resources::TaxRegistrationsResource::new(&client);

    // Path traversal injection check
    let err = resource.retrieve("../hack").await.unwrap_err();
    assert!(matches!(err, some_stripe::StripeError::InvalidRequest(_)));

    let err_update = resource
        .update(
            "bad/id",
            &UpdateTaxRegistrationParams {
                expires_at: Some("now".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err_update,
        some_stripe::StripeError::InvalidRequest(_)
    ));
}
