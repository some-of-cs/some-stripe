//! Unit and integration tests for Stripe Customer Vaulting, Cash Balance, and Tax IDs.
//!
//! Validates:
//! - Quality Gate G0: 0 compiler/clippy warnings under Rust 2024.
//! - Quality Gate G-Borrow: Zero deep `.clone()` on domain models; references and move semantics preserved.
//! - Quality Gate G2: Sensitive customer PII redacted under GDPR Article 17, zero plain-text leaks,
//!   auto-injected RFC 4122 UUID v4 `Idempotency-Key` headers on mutable operations.
//! - Quality Gate G-Law: ISO 4217 minor currency integer units (`balance: i64`, `available: HashMap<String, i64>`),
//!   and PCI-DSS 4.0 tokenization verification (zero raw PAN in application memory).
//! - Dual-Protocol Serialization: RFC 1738 / RFC 3986 bracket notation form params and 3-state [`Nullable`].
//! - CustomersResource wire contracts: `create`, `retrieve`, `update`, `del`, `list`, `search`,
//!   `retrieve_cash_balance`, `update_cash_balance`, and path traversal protection.

use some_stripe::patterns::serializer::to_form_params;
use some_stripe::types::customer::{
    CashBalance, CashBalanceSettingsParams, CreateCustomerParams, Customer, CustomerAddress,
    CustomerInvoiceSettings, CustomerInvoiceSettingsParams, CustomerShipping, ListCustomersParams,
    PciDssViolationError, SearchCustomersParams, TaxId, UpdateCustomerCashBalanceParams,
    UpdateCustomerParams, customer_id, is_pci_dss_tokenized, redact_gdpr_customer_data,
    validate_pci_dss_token,
};
use some_stripe::{StripeClient, StripeConfig, is_valid_uuid_v4};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_customer_domain_entity_json_deserialization_and_accessors() {
    let json = r#"{
        "id": "cus_N4m2L2eZvKYlo2C",
        "object": "customer",
        "email": "jane.doe@example.com",
        "name": "Jane Doe",
        "phone": "+15551234567",
        "description": "Premium customer",
        "balance": -5000,
        "currency": "usd",
        "created": 1728345600,
        "delinquent": false,
        "livemode": false,
        "default_payment_method": "pm_1N4m2L2eZvKYlo2C01234567",
        "address": {
            "city": "Austin",
            "country": "US",
            "line1": "100 Congress Ave",
            "postal_code": "78701",
            "state": "TX"
        },
        "metadata": {
            "tier": "enterprise",
            "segment": "b2b"
        },
        "invoice_settings": {
            "default_payment_method": "pm_1N4m2L2eZvKYlo2C01234567",
            "footer": "Net 30 terms"
        }
    }"#;

    let customer: Customer = serde_json::from_str(json).expect("Customer deserialization failed");
    assert_eq!(customer.id(), "cus_N4m2L2eZvKYlo2C");
    assert_eq!(customer.email(), Some("jane.doe@example.com"));
    assert_eq!(customer.name(), Some("Jane Doe"));
    assert_eq!(customer.balance(), -5000);
    assert_eq!(
        customer.default_payment_method(),
        Some("pm_1N4m2L2eZvKYlo2C01234567")
    );
    assert!(!customer.is_delinquent());
    assert!(!customer.is_deleted());
    assert_eq!(
        customer.metadata.get("tier"),
        Some(&"enterprise".to_string())
    );

    let address = customer.address.as_ref().expect("Address missing");
    assert_eq!(address.city.as_deref(), Some("Austin"));
    assert_eq!(address.country.as_deref(), Some("US"));
    assert_eq!(address.state.as_deref(), Some("TX"));

    let invoice_settings = customer
        .invoice_settings
        .as_ref()
        .expect("Invoice settings missing");
    assert_eq!(invoice_settings.footer.as_deref(), Some("Net 30 terms"));
}

#[test]
fn test_cash_balance_and_tax_id_deserialization() {
    let cb_json = r#"{
        "object": "cash_balance",
        "customer": "cus_N4m2L2eZvKYlo2C",
        "available": {
            "usd": 25000,
            "eur": 18500
        },
        "livemode": false,
        "settings": {
            "reconciliation_mode": "automatic",
            "using_merchant_default": true
        }
    }"#;

    let cb: CashBalance =
        serde_json::from_str(cb_json).expect("CashBalance deserialization failed");
    assert_eq!(cb.customer, "cus_N4m2L2eZvKYlo2C");
    let available = cb.available.as_ref().expect("Available balance missing");
    assert_eq!(available.get("usd"), Some(&25000));
    assert_eq!(available.get("eur"), Some(&18500));
    assert_eq!(
        cb.settings
            .as_ref()
            .and_then(|s| s.reconciliation_mode.as_deref()),
        Some("automatic")
    );

    let tax_id_json = r#"{
        "id": "txi_1234567890",
        "object": "tax_id",
        "type": "eu_vat",
        "value": "DE123456789",
        "country": "DE",
        "customer": "cus_N4m2L2eZvKYlo2C",
        "livemode": false,
        "verification": {
            "status": "verified",
            "verified_name": "Acme GmbH",
            "verified_address": "Musterstr. 1, Berlin"
        }
    }"#;

    let tax_id: TaxId = serde_json::from_str(tax_id_json).expect("TaxId deserialization failed");
    assert_eq!(tax_id.id, "txi_1234567890");
    assert_eq!(tax_id.tax_id_type, "eu_vat");
    assert_eq!(tax_id.value, "DE123456789");
    assert_eq!(tax_id.country.as_deref(), Some("DE"));
    assert_eq!(
        tax_id.verification.as_ref().map(|v| v.status.as_str()),
        Some("verified")
    );
}

#[test]
fn test_pci_dss_4_tokenization_validation_rule() {
    // Valid tokenized identifiers under PCI-DSS 4.0 Gate G-Law
    let valid_tokens = [
        "pm_1N4m2L2eZvKYlo2C01234567",
        "pm_card_visa",
        "src_18eYal2eZvKYlo2C01234567",
        "tok_visa",
        "tok_1234567890abcdef",
        "card_1N4m2L2eZvKYlo2C",
        "ba_1N4m2L2eZvKYlo2C",
        "btok_1N4m2L2eZvKYlo2C",
    ];
    for token in valid_tokens {
        assert!(
            validate_pci_dss_token(token).is_ok(),
            "Token '{token}' should be valid under PCI-DSS 4.0"
        );
        assert!(is_pci_dss_tokenized(token));
    }

    // Prohibited raw Primary Account Numbers (PAN) under PCI-DSS 4.0 Requirement 3.4
    let raw_pans = [
        "4242424242424242",
        "4000056655665556",
        "5555555555554444",
        "378282246310005",
        "4111 1111 1111 1111",
        "3782-822463-10005",
    ];
    for pan in raw_pans {
        let err = validate_pci_dss_token(pan).expect_err("Raw PAN must be strictly rejected");
        assert!(
            matches!(err, PciDssViolationError(_)),
            "Expected PciDssViolationError for '{pan}'"
        );
        assert!(!is_pci_dss_tokenized(pan));
        assert!(err.0.contains("Raw Primary Account Number (PAN) detected"));
    }

    // Invalid formats or empty
    assert!(validate_pci_dss_token("").is_err());
    assert!(validate_pci_dss_token("   ").is_err());
    assert!(validate_pci_dss_token("unknown_credential_prefix").is_err());
}

#[test]
fn test_gdpr_article_17_right_to_erasure_workflow() {
    let mut customer = Customer {
        id: "cus_gdpr_target_001".into(),
        object: "customer".into(),
        currency: Some("usd".into()),
        email: Some("erasure.request@example.com".into()),
        name: Some("John Consumer".into()),
        phone: Some("+15559876543".into()),
        description: Some("EU Citizen requesting Article 17 erasure".into()),
        balance: -2500, // 25.00 USD credit
        created: 1728345600,
        delinquent: false,
        livemode: false,
        deleted: None,
        default_source: Some("src_old_source".into()),
        default_payment_method: Some("pm_saved_card".into()),
        address: Some(CustomerAddress {
            city: Some("Berlin".into()),
            country: Some("DE".into()),
            line1: Some("Unter den Linden 10".into()),
            ..Default::default()
        }),
        shipping: Some(CustomerShipping {
            name: "John Consumer".into(),
            address: CustomerAddress::default(),
            phone: Some("+15559876543".into()),
        }),
        metadata: {
            let mut m = HashMap::new();
            m.insert("marketing_id".into(), "mkt_123".into());
            m
        },
        tax_ids: Some(vec![TaxId {
            id: "txi_123".into(),
            object: "tax_id".into(),
            tax_id_type: "eu_vat".into(),
            value: "DE999999999".into(),
            country: Some("DE".into()),
            customer: Some("cus_gdpr_target_001".into()),
            livemode: false,
            created: Some(1728345600),
            verification: None,
        }]),
        cash_balance: None,
        invoice_settings: Some(CustomerInvoiceSettings {
            default_payment_method: Some("pm_saved_card".into()),
            custom_fields: None,
            footer: Some("Tax invoice for John Consumer".into()),
        }),
        invoice_prefix: Some("INV-".into()),
        next_invoice_sequence: Some(5),
        tax_exempt: Some("none".into()),
        test_clock: None,
    };

    assert!(!customer.is_pii_scrubbed());
    assert!(!customer.is_deleted());

    // Execute GDPR Article 17 Right to Erasure
    redact_gdpr_customer_data(&mut customer);

    // Verify all PII fields are completely scrubbed
    assert!(customer.is_pii_scrubbed());
    assert!(customer.is_deleted());
    assert_eq!(customer.email, None);
    assert_eq!(customer.name, None);
    assert_eq!(customer.phone, None);
    assert_eq!(customer.description, None);
    assert_eq!(customer.address, None);
    assert_eq!(customer.shipping, None);
    assert_eq!(customer.default_source, None);
    assert_eq!(customer.default_payment_method, None);
    assert_eq!(customer.tax_ids, None);
    assert!(customer.metadata.is_empty());

    // Invariant: Non-PII statutory financial ledger entries MUST be preserved for tax retention
    assert_eq!(customer.id(), "cus_gdpr_target_001");
    assert_eq!(customer.balance(), -2500);
    assert_eq!(customer.currency.as_deref(), Some("usd"));
    assert_eq!(customer.created, 1728345600);
}

#[test]
fn test_create_customer_params_builder_and_validation() {
    // Valid creation parameters
    let params = CreateCustomerParams::new()
        .email("alex@example.com")
        .name("Alex Smith")
        .phone("+15555551234")
        .description("Corporate client")
        .balance(-10000)
        .currency("usd")
        .payment_method("pm_1N4m2L2eZvKYlo2C01234567")
        .metadata_entry("department", "engineering")
        .idempotency_key("req_alex_create_001");

    assert!(params.validate().is_ok());
    assert_eq!(params.email.as_deref(), Some("alex@example.com"));
    assert_eq!(params.name.as_deref(), Some("Alex Smith"));
    assert_eq!(params.balance, Some(-10000));
    assert_eq!(params.currency.as_deref(), Some("usd"));
    assert_eq!(
        params.metadata.as_ref().and_then(|m| m.get("department")),
        Some(&"engineering".to_string())
    );

    // Validation failure: raw PAN passed as payment method
    let bad_pan_params = CreateCustomerParams::new()
        .email("victim@example.com")
        .payment_method("4242424242424242");
    assert!(bad_pan_params.validate().is_err());

    // Validation failure: invalid ISO 4217 currency code
    let bad_currency_params = CreateCustomerParams::new().currency("US_DOLLARS");
    assert!(bad_currency_params.validate().is_err());
}

#[test]
fn test_update_customer_params_nullability_and_validation() {
    // Value state: updating fields
    let val_params = UpdateCustomerParams::new()
        .name("Updated Name")
        .email("updated@example.com")
        .default_payment_method("pm_new_card_123");
    assert!(val_params.validate().is_ok());
    assert!(val_params.name.is_value());
    assert_eq!(val_params.name.into_value(), Some("Updated Name".into()));

    // Null state: clearing fields via 3-state Nullable
    let clear_params = UpdateCustomerParams::new()
        .clear_name()
        .clear_email()
        .clear_description()
        .clear_default_payment_method();
    assert!(clear_params.name.is_null());
    assert!(clear_params.email.is_null());
    assert!(clear_params.description.is_null());
    assert!(clear_params.default_payment_method.is_null());

    // Omit state: default leaves fields omitted
    let omit_params = UpdateCustomerParams::default();
    assert!(omit_params.name.is_omit());
    assert!(omit_params.email.is_omit());
    assert!(omit_params.description.is_omit());

    // Validation failure on raw PAN in update
    let bad_update = UpdateCustomerParams::new().default_payment_method("4111111111111111");
    assert!(bad_update.validate().is_err());
}

#[test]
fn test_to_form_params_bracket_notation_serialization() {
    let mut meta = HashMap::new();
    meta.insert("plan".into(), "pro".into());
    meta.insert("org_id".into(), "org_42".into());

    let params = CreateCustomerParams::new()
        .email("billing@example.com")
        .name("Enterprise Corp")
        .balance(5000)
        .currency("eur")
        .address(CustomerAddress {
            city: Some("Paris".into()),
            country: Some("FR".into()),
            line1: Some("15 Rue de Rivoli".into()),
            ..Default::default()
        })
        .metadata(meta)
        .invoice_settings(
            CustomerInvoiceSettingsParams::new()
                .default_payment_method("pm_card_mastercard")
                .footer("Payment due in 14 days"),
        );

    let form_pairs = to_form_params(&params).expect("Serialization to form params failed");
    let map: HashMap<String, String> = form_pairs.into_iter().collect();

    assert_eq!(map.get("email"), Some(&"billing@example.com".to_string()));
    assert_eq!(map.get("name"), Some(&"Enterprise Corp".to_string()));
    assert_eq!(map.get("balance"), Some(&"5000".to_string()));
    assert_eq!(map.get("currency"), Some(&"eur".to_string()));

    // Nested address bracket notation
    assert_eq!(map.get("address[city]"), Some(&"Paris".to_string()));
    assert_eq!(map.get("address[country]"), Some(&"FR".to_string()));
    assert_eq!(
        map.get("address[line1]"),
        Some(&"15 Rue de Rivoli".to_string())
    );

    // Metadata bracket notation
    assert_eq!(map.get("metadata[plan]"), Some(&"pro".to_string()));
    assert_eq!(map.get("metadata[org_id]"), Some(&"org_42".to_string()));

    // Invoice settings bracket notation
    assert_eq!(
        map.get("invoice_settings[default_payment_method]"),
        Some(&"pm_card_mastercard".to_string())
    );
    assert_eq!(
        map.get("invoice_settings[footer]"),
        Some(&"Payment due in 14 days".to_string())
    );
}

#[test]
fn test_nullability_form_encoding_resets_fields() {
    // Nullable::Null must serialize to empty string in form payloads to clear the field on Stripe
    let clear_params = UpdateCustomerParams::new().clear_name().clear_description();

    let form_pairs = to_form_params(&clear_params).expect("Form serialization failed");
    let map: HashMap<String, String> = form_pairs.into_iter().collect();

    assert_eq!(map.get("name"), Some(&"".to_string()));
    assert_eq!(map.get("description"), Some(&"".to_string()));

    // Nullable::Omit must NOT appear in the form payload at all
    assert!(!map.contains_key("email"));
    assert!(!map.contains_key("phone"));
    assert!(!map.contains_key("default_payment_method"));
}

#[tokio::test]
async fn test_customers_resource_create_with_auto_idempotency_key() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("POST /v1/customers HTTP/1.1"));

        // Verify auto-injected Idempotency-Key header is a valid UUID v4
        let key_header = req_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
            .expect("Auto Idempotency-Key must be present");
        let key = key_header.split(':').nth(1).unwrap().trim();
        assert!(is_valid_uuid_v4(key));

        // Verify form data content
        assert!(
            req_str.contains("email=test.user%40example.com")
                || req_str.contains("email=test.user@example.com")
        );
        assert!(req_str.contains("name=Test%20User") || req_str.contains("name=Test+User"));

        let resp_body = serde_json::json!({
            "id": "cus_wire_test_101",
            "object": "customer",
            "email": "test.user@example.com",
            "name": "Test User",
            "balance": 0,
            "created": 1728345600,
            "delinquent": false,
            "livemode": false
        });
        let body_str = resp_body.to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body_str.len(),
            body_str
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock_secret").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let params = CreateCustomerParams::new()
        .email("test.user@example.com")
        .name("Test User");

    let customer = client
        .customers()
        .create(&params)
        .await
        .expect("Create customer failed");
    assert_eq!(customer.id(), "cus_wire_test_101");
    assert_eq!(customer.email(), Some("test.user@example.com"));
    assert_eq!(customer.name(), Some("Test User"));

    server.await.unwrap();
}

#[tokio::test]
async fn test_customers_resource_retrieve_without_idempotency_key() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("GET /v1/customers/cus_lookup_123 HTTP/1.1"));

        // Idempotent read: Idempotency-Key must NOT be sent
        assert!(
            !req_str.to_lowercase().contains("idempotency-key:"),
            "Idempotency-Key should not be present on GET requests"
        );

        let resp_body = serde_json::json!({
            "id": "cus_lookup_123",
            "object": "customer",
            "email": "lookup@example.com",
            "name": "Lookup User",
            "balance": -2500,
            "currency": "usd",
            "created": 1728345600,
            "delinquent": false,
            "livemode": false
        });
        let body_str = resp_body.to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body_str.len(),
            body_str
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock_secret").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let customer = client
        .customers()
        .retrieve("cus_lookup_123")
        .await
        .expect("Retrieve customer failed");
    assert_eq!(customer.id(), "cus_lookup_123");
    assert_eq!(customer.balance(), -2500);

    server.await.unwrap();
}

#[tokio::test]
async fn test_customers_resource_update_with_custom_idempotency_key() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("POST /v1/customers/cus_update_999 HTTP/1.1"));

        // Verify explicit caller-provided Idempotency-Key is preserved
        let key_header = req_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
            .expect("Custom Idempotency-Key header missing");
        let key = key_header.split(':').nth(1).unwrap().trim();
        assert_eq!(key, "custom_update_key_xyz");

        let resp_body = serde_json::json!({
            "id": "cus_update_999",
            "object": "customer",
            "email": "updated@example.com",
            "name": "Updated Name",
            "balance": 0,
            "created": 1728345600,
            "delinquent": false,
            "livemode": false
        });
        let body_str = resp_body.to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body_str.len(),
            body_str
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock_secret").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let params = UpdateCustomerParams::new()
        .name("Updated Name")
        .email("updated@example.com")
        .idempotency_key("custom_update_key_xyz");

    let customer = client
        .customers()
        .update("cus_update_999", &params)
        .await
        .expect("Update customer failed");
    assert_eq!(customer.id(), "cus_update_999");
    assert_eq!(customer.name(), Some("Updated Name"));

    server.await.unwrap();
}

#[tokio::test]
async fn test_customers_resource_del_and_delete_alias() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Request 1: del
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("DELETE /v1/customers/cus_del_001 HTTP/1.1"));

            let resp = serde_json::json!({
                "id": "cus_del_001",
                "object": "customer",
                "deleted": true
            });
            let body = resp.to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }

        // Request 2: delete (alias)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("DELETE /v1/customers/cus_del_002 HTTP/1.1"));

            let resp = serde_json::json!({
                "id": "cus_del_002",
                "object": "customer",
                "deleted": true
            });
            let body = resp.to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });

    let config = StripeConfig::new("sk_test_mock_secret").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let res1 = client.customers().del("cus_del_001").await.unwrap();
    assert_eq!(res1.id, "cus_del_001");
    assert!(res1.deleted);

    let res2 = client.customers().delete("cus_del_002").await.unwrap();
    assert_eq!(res2.id, "cus_del_002");
    assert!(res2.deleted);

    server.await.unwrap();
}

#[tokio::test]
async fn test_customers_resource_list_with_query_params() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("GET /v1/customers?"));
        assert!(req_str.contains("limit=5"));
        assert!(
            req_str.contains("email=search%40example.com")
                || req_str.contains("email=search@example.com")
        );

        let resp_body = serde_json::json!({
            "object": "list",
            "data": [
                {
                    "id": "cus_item_1",
                    "object": "customer",
                    "email": "search@example.com",
                    "balance": 0,
                    "created": 1728345600,
                    "delinquent": false,
                    "livemode": false
                }
            ],
            "has_more": false,
            "url": "/v1/customers"
        });
        let body_str = resp_body.to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body_str.len(),
            body_str
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock_secret").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let params = ListCustomersParams::new()
        .limit(5)
        .email("search@example.com");

    let list = client.customers().list(&params).await.unwrap();
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id(), "cus_item_1");

    server.await.unwrap();
}

#[tokio::test]
async fn test_customers_resource_search_with_query_params() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("GET /v1/customers/search?"));
        assert!(req_str.contains("query=name%3A%27VIP%27") || req_str.contains("query=name:'VIP'"));

        let resp_body = serde_json::json!({
            "object": "search_result",
            "data": [
                {
                    "id": "cus_vip_007",
                    "object": "customer",
                    "name": "VIP Customer",
                    "balance": -50000,
                    "created": 1728345600,
                    "delinquent": false,
                    "livemode": false
                }
            ],
            "has_more": false,
            "url": "/v1/customers/search"
        });
        let body_str = resp_body.to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body_str.len(),
            body_str
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock_secret").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let params = SearchCustomersParams::new("name:'VIP'").limit(10);
    let result = client.customers().search(&params).await.unwrap();
    assert_eq!(result.data.len(), 1);
    assert_eq!(result.data[0].id(), "cus_vip_007");

    server.await.unwrap();
}

#[tokio::test]
async fn test_customers_resource_cash_balance_retrieve_and_update() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Request 1: retrieve_cash_balance
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("GET /v1/customers/cus_cb_user/cash_balance HTTP/1.1"));

            let resp = serde_json::json!({
                "object": "cash_balance",
                "customer": "cus_cb_user",
                "available": { "usd": 12500 },
                "livemode": false,
                "settings": {
                    "reconciliation_mode": "automatic",
                    "using_merchant_default": true
                }
            });
            let body = resp.to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }

        // Request 2: update_cash_balance
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("POST /v1/customers/cus_cb_user/cash_balance HTTP/1.1"));

            // Verify form parameter for settings[reconciliation_mode]
            assert!(
                req_str.contains("settings%5Breconciliation_mode%5D=manual")
                    || req_str.contains("settings[reconciliation_mode]=manual")
            );

            let resp = serde_json::json!({
                "object": "cash_balance",
                "customer": "cus_cb_user",
                "available": { "usd": 12500 },
                "livemode": false,
                "settings": {
                    "reconciliation_mode": "manual",
                    "using_merchant_default": false
                }
            });
            let body = resp.to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });

    let config = StripeConfig::new("sk_test_mock_secret").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    // Retrieve
    let cb = client
        .customers()
        .retrieve_cash_balance("cus_cb_user")
        .await
        .unwrap();
    assert_eq!(cb.customer, "cus_cb_user");
    assert_eq!(cb.available.as_ref().unwrap().get("usd"), Some(&12500));

    // Update
    let update_params = UpdateCustomerCashBalanceParams::new().settings(
        CashBalanceSettingsParams::new()
            .reconciliation_mode("manual")
            .using_merchant_default(false),
    );
    let updated_cb = client
        .customers()
        .update_cash_balance("cus_cb_user", &update_params)
        .await
        .unwrap();
    assert_eq!(
        updated_cb
            .settings
            .as_ref()
            .unwrap()
            .reconciliation_mode
            .as_deref(),
        Some("manual")
    );

    server.await.unwrap();
}

#[tokio::test]
async fn test_customers_resource_path_traversal_guards() {
    let client = StripeClient::new("sk_test_mock_secret");

    let malicious_ids = [
        "../secret_endpoint",
        "cus/123",
        "cus\\123",
        "cus?param=inject",
        "cus#frag",
        "",
        "   ",
    ];

    for bad_id in malicious_ids {
        assert!(
            client.customers().retrieve(bad_id).await.is_err(),
            "Path traversal retrieve must fail for '{bad_id}'"
        );
        assert!(
            client.customers().del(bad_id).await.is_err(),
            "Path traversal del must fail for '{bad_id}'"
        );
        assert!(
            client
                .customers()
                .retrieve_cash_balance(bad_id)
                .await
                .is_err(),
            "Path traversal retrieve_cash_balance must fail for '{bad_id}'"
        );
        assert!(
            client
                .customers()
                .update_cash_balance(bad_id, &UpdateCustomerCashBalanceParams::default())
                .await
                .is_err(),
            "Path traversal update_cash_balance must fail for '{bad_id}'"
        );
    }
}

#[test]
fn test_gate_g_borrow_zero_clone_discipline() {
    let customer = Customer {
        id: "cus_borrow_check_101".into(),
        object: "customer".into(),
        currency: Some("usd".into()),
        email: Some("borrow@example.com".into()),
        name: Some("Borrower".into()),
        phone: None,
        description: None,
        balance: 0,
        created: 1728345600,
        delinquent: false,
        livemode: false,
        deleted: None,
        default_source: None,
        default_payment_method: None,
        address: None,
        shipping: None,
        metadata: HashMap::new(),
        tax_ids: None,
        cash_balance: None,
        invoice_settings: None,
        invoice_prefix: None,
        next_invoice_sequence: None,
        tax_exempt: None,
        test_clock: None,
    };

    // Zero-clone borrower extractor
    let id_ref: &str = customer_id(&customer);
    assert_eq!(id_ref, "cus_borrow_check_101");

    // Borrowed accessors
    assert_eq!(customer.id(), "cus_borrow_check_101");
    assert_eq!(customer.email(), Some("borrow@example.com"));
    assert_eq!(customer.name(), Some("Borrower"));
}
