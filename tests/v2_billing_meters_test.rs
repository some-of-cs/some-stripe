//! Wire integration and domain model tests for Stripe API V2 Billing Meters (US-8.2.0 Unit 4).
//!
//! Covers:
//! - Domain entity serialization/deserialization for `V2BillingMeter`, `MeterStatus`, `AggregationFormula`,
//!   `ValueSettings`, and `CustomerMappingSettings`.
//! - Gate G2 automatic UUID v4 `Idempotency-Key` generation and explicit key forwarding.
//! - Security path traversal prevention via `validate_and_encode_id`.
//! - Wire protocol validation against mock HTTP server for `create`, `retrieve`, `update`, `deactivate`, `reactivate`, and `list`.
//! - Gate G-Borrow zero deep-clone borrowing discipline.

use some_stripe::resources::BillingMetersResource;
use some_stripe::{
    AggregationFormula, CreateBillingMeterParams, CustomerMappingSettings, ListBillingMetersParams,
    MeterStatus, StripeClient, StripeConfig, StripeError, UpdateBillingMeterParams, V2BillingMeter,
    ValueSettings, is_valid_uuid_v4,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_meter_status_enum_invariants() {
    let active = MeterStatus::Active;
    assert_eq!(active.as_str(), "active");
    assert_eq!(active.to_string(), "active");
    assert_eq!(serde_json::to_string(&active).unwrap(), "\"active\"");

    let inactive = MeterStatus::Inactive;
    assert_eq!(inactive.as_str(), "inactive");
    assert_eq!(inactive.to_string(), "inactive");
    assert_eq!(serde_json::to_string(&inactive).unwrap(), "\"inactive\"");

    let des_active: MeterStatus = serde_json::from_str("\"active\"").unwrap();
    assert_eq!(des_active, MeterStatus::Active);

    let des_inactive: MeterStatus = serde_json::from_str("\"inactive\"").unwrap();
    assert_eq!(des_inactive, MeterStatus::Inactive);
}

#[test]
fn test_aggregation_formula_enum_invariants() {
    let formulas = [
        (AggregationFormula::Sum, "sum"),
        (AggregationFormula::Count, "count"),
        (AggregationFormula::Latest, "latest"),
    ];

    for (formula, expected_str) in formulas {
        assert_eq!(formula.as_str(), expected_str);
        assert_eq!(formula.to_string(), expected_str);
        assert_eq!(
            serde_json::to_string(&formula).unwrap(),
            format!("\"{expected_str}\"")
        );
        let deserialized: AggregationFormula =
            serde_json::from_str(&format!("\"{expected_str}\"")).unwrap();
        assert_eq!(deserialized, formula);
    }
}

#[test]
fn test_value_settings_and_customer_mapping() {
    let val_settings = ValueSettings::new("tokens");
    assert_eq!(val_settings.event_payload_key, "tokens");
    let val_json = serde_json::to_value(&val_settings).unwrap();
    assert_eq!(val_json["event_payload_key"], "tokens");

    let cust_mapping = CustomerMappingSettings::by_id("stripe_customer_id");
    assert_eq!(cust_mapping.event_payload_key, "stripe_customer_id");
    assert_eq!(cust_mapping.mapping_type, "by_id");

    let mapping_json = serde_json::to_value(&cust_mapping).unwrap();
    assert_eq!(mapping_json["event_payload_key"], "stripe_customer_id");
    assert_eq!(mapping_json["type"], "by_id");

    let mapping_roundtrip: CustomerMappingSettings = serde_json::from_value(mapping_json).unwrap();
    assert_eq!(mapping_roundtrip, cust_mapping);
}

#[test]
fn test_create_and_update_params_serialization() {
    let create_params = CreateBillingMeterParams::new(
        "AI LLM Tokens",
        "llm_tokens",
        AggregationFormula::Sum,
        ValueSettings::new("token_count"),
        CustomerMappingSettings::by_id("customer_id"),
    )
    .idempotency_key("req_idem_abc");

    assert_eq!(
        create_params.idempotency_key.as_deref(),
        Some("req_idem_abc")
    );
    let create_json = serde_json::to_value(&create_params).unwrap();
    assert_eq!(create_json["display_name"], "AI LLM Tokens");
    assert_eq!(create_json["event_name"], "llm_tokens");
    assert_eq!(create_json["aggregation_formula"], "sum");
    assert_eq!(
        create_json["value_settings"]["event_payload_key"],
        "token_count"
    );
    assert_eq!(create_json["customer_mapping"]["type"], "by_id");
    assert!(create_json.get("idempotency_key").is_none());

    let update_params = UpdateBillingMeterParams::new()
        .display_name("Updated LLM Meter")
        .idempotency_key("req_idem_update");

    let update_json = serde_json::to_value(&update_params).unwrap();
    assert_eq!(update_json["display_name"], "Updated LLM Meter");
    assert!(update_json.get("idempotency_key").is_none());
}

#[test]
fn test_list_params_builder() {
    let params = ListBillingMetersParams::new()
        .limit(25)
        .status(MeterStatus::Active)
        .starting_after("mtr_cursor_start")
        .ending_before("mtr_cursor_end");

    assert_eq!(params.limit, Some(25));
    assert_eq!(params.status, Some(MeterStatus::Active));
    assert_eq!(params.starting_after.as_deref(), Some("mtr_cursor_start"));
    assert_eq!(params.ending_before.as_deref(), Some("mtr_cursor_end"));
}

#[test]
fn test_v2_billing_meter_serde_roundtrip() {
    let raw_json = r#"{
        "id": "mtr_123456789",
        "object": "billing.meter",
        "display_name": "API Request Count",
        "event_name": "api_requests",
        "status": "active",
        "value_settings": {
            "event_payload_key": "count"
        },
        "customer_mapping": {
            "event_payload_key": "customer_id",
            "type": "by_id"
        },
        "aggregation_formula": "count",
        "created": 1720000000,
        "updated": 1720001000,
        "livemode": false
    }"#;

    let meter: V2BillingMeter = serde_json::from_str(raw_json).unwrap();
    assert_eq!(meter.id, "mtr_123456789");
    assert_eq!(meter.object, "billing.meter");
    assert_eq!(meter.display_name, "API Request Count");
    assert_eq!(meter.event_name, "api_requests");
    assert_eq!(meter.status, MeterStatus::Active);
    assert_eq!(meter.aggregation_formula, AggregationFormula::Count);
    assert_eq!(meter.value_settings.event_payload_key, "count");
    assert_eq!(meter.customer_mapping.mapping_type, "by_id");
    assert_eq!(meter.customer_mapping.event_payload_key, "customer_id");
    assert_eq!(meter.created, 1720000000);
    assert_eq!(meter.updated, 1720001000);
    assert!(!meter.livemode);

    let serialized = serde_json::to_string(&meter).unwrap();
    let roundtrip: V2BillingMeter = serde_json::from_str(&serialized).unwrap();
    assert_eq!(roundtrip, meter);
}

#[tokio::test]
async fn test_create_billing_meter_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("POST /v2/billing/meters HTTP/1.1"));
        assert!(req_str.contains("content-type: application/json"));

        // Verify Gate G2 automatic UUID v4 Idempotency-Key
        let idemp_line = req_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
            .expect("Idempotency-Key header missing");
        let idemp_val = idemp_line.split(':').nth(1).unwrap().trim();
        assert!(
            is_valid_uuid_v4(idemp_val),
            "Expected valid UUID v4 idempotency key, got '{idemp_val}'"
        );

        let body = r#"{
            "id": "mtr_created_101",
            "object": "billing.meter",
            "display_name": "Inference Compute",
            "event_name": "compute_seconds",
            "status": "active",
            "value_settings": {
                "event_payload_key": "seconds"
            },
            "customer_mapping": {
                "event_payload_key": "stripe_customer_id",
                "type": "by_id"
            },
            "aggregation_formula": "sum",
            "created": 1728000000,
            "updated": 1728000000,
            "livemode": false
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_meters").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let resource = BillingMetersResource::new(&client);

    let params = CreateBillingMeterParams::new(
        "Inference Compute",
        "compute_seconds",
        AggregationFormula::Sum,
        ValueSettings::new("seconds"),
        CustomerMappingSettings::by_id("stripe_customer_id"),
    );

    let meter = resource.create(&params).await.unwrap();
    assert_eq!(meter.id, "mtr_created_101");
    assert_eq!(meter.display_name, "Inference Compute");
    assert_eq!(meter.status, MeterStatus::Active);
    assert_eq!(meter.aggregation_formula, AggregationFormula::Sum);

    server.await.unwrap();
}

#[tokio::test]
async fn test_create_billing_meter_with_explicit_idempotency_key() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("POST /v2/billing/meters HTTP/1.1"));
        assert!(req_str.contains("idempotency-key: user_meter_key_999"));

        let body = r#"{
            "id": "mtr_explicit_key_202",
            "object": "billing.meter",
            "display_name": "Explicit Key Meter",
            "event_name": "tokens",
            "status": "active",
            "value_settings": { "event_payload_key": "val" },
            "customer_mapping": { "event_payload_key": "cus_id", "type": "by_id" },
            "aggregation_formula": "sum",
            "created": 1728000000,
            "updated": 1728000000,
            "livemode": false
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_meters").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let resource = BillingMetersResource::new(&client);

    let params = CreateBillingMeterParams::new(
        "Explicit Key Meter",
        "tokens",
        AggregationFormula::Sum,
        ValueSettings::new("val"),
        CustomerMappingSettings::by_id("cus_id"),
    )
    .idempotency_key("user_meter_key_999");

    let meter = resource.create(&params).await.unwrap();
    assert_eq!(meter.id, "mtr_explicit_key_202");

    server.await.unwrap();
}

#[tokio::test]
async fn test_retrieve_billing_meter_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("GET /v2/billing/meters/mtr_retrieve_303 HTTP/1.1"));

        let body = r#"{
            "id": "mtr_retrieve_303",
            "object": "billing.meter",
            "display_name": "Storage Gigabytes",
            "event_name": "storage_gb",
            "status": "active",
            "value_settings": { "event_payload_key": "gigabytes" },
            "customer_mapping": { "event_payload_key": "stripe_customer_id", "type": "by_id" },
            "aggregation_formula": "latest",
            "created": 1728000100,
            "updated": 1728000200,
            "livemode": false
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_meters").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let resource = BillingMetersResource::new(&client);

    let meter = resource.retrieve("mtr_retrieve_303").await.unwrap();
    assert_eq!(meter.id, "mtr_retrieve_303");
    assert_eq!(meter.display_name, "Storage Gigabytes");
    assert_eq!(meter.aggregation_formula, AggregationFormula::Latest);

    server.await.unwrap();
}

#[tokio::test]
async fn test_update_billing_meter_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("POST /v2/billing/meters/mtr_update_404 HTTP/1.1"));
        assert!(req_str.contains("content-type: application/json"));
        assert!(req_str.contains("\"display_name\":\"Renamed Meter\""));

        let body = r#"{
            "id": "mtr_update_404",
            "object": "billing.meter",
            "display_name": "Renamed Meter",
            "event_name": "storage_gb",
            "status": "active",
            "value_settings": { "event_payload_key": "gigabytes" },
            "customer_mapping": { "event_payload_key": "stripe_customer_id", "type": "by_id" },
            "aggregation_formula": "latest",
            "created": 1728000100,
            "updated": 1728000500,
            "livemode": false
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_meters").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let resource = BillingMetersResource::new(&client);

    let params = UpdateBillingMeterParams::new().display_name("Renamed Meter");
    let meter = resource.update("mtr_update_404", &params).await.unwrap();
    assert_eq!(meter.id, "mtr_update_404");
    assert_eq!(meter.display_name, "Renamed Meter");

    server.await.unwrap();
}

#[tokio::test]
async fn test_deactivate_and_reactivate_billing_meter_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        // First call: Deactivate
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("POST /v2/billing/meters/mtr_toggle_505/deactivate HTTP/1.1"));
        let deact_body = r#"{
            "id": "mtr_toggle_505",
            "object": "billing.meter",
            "display_name": "Toggle Meter",
            "event_name": "tokens",
            "status": "inactive",
            "value_settings": { "event_payload_key": "val" },
            "customer_mapping": { "event_payload_key": "cus_id", "type": "by_id" },
            "aggregation_formula": "sum",
            "created": 1728000000,
            "updated": 1728000600,
            "livemode": false
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            deact_body.len(),
            deact_body
        );
        socket.write_all(response.as_bytes()).await.unwrap();

        // Second call: Reactivate
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf2 = vec![0; 4096];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req_str2 = String::from_utf8_lossy(&buf2[..n2]);

        assert!(req_str2.starts_with("POST /v2/billing/meters/mtr_toggle_505/reactivate HTTP/1.1"));
        let react_body = r#"{
            "id": "mtr_toggle_505",
            "object": "billing.meter",
            "display_name": "Toggle Meter",
            "event_name": "tokens",
            "status": "active",
            "value_settings": { "event_payload_key": "val" },
            "customer_mapping": { "event_payload_key": "cus_id", "type": "by_id" },
            "aggregation_formula": "sum",
            "created": 1728000000,
            "updated": 1728000700,
            "livemode": false
        }"#;

        let response2 = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            react_body.len(),
            react_body
        );
        socket2.write_all(response2.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_meters").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let resource = BillingMetersResource::new(&client);

    let deactivated = resource.deactivate("mtr_toggle_505").await.unwrap();
    assert_eq!(deactivated.status, MeterStatus::Inactive);

    let reactivated = resource.reactivate("mtr_toggle_505").await.unwrap();
    assert_eq!(reactivated.status, MeterStatus::Active);

    server.await.unwrap();
}

#[tokio::test]
async fn test_list_billing_meters_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("GET /v2/billing/meters?"));
        assert!(req_str.contains("limit=2"));
        assert!(req_str.contains("status=active"));
        assert!(req_str.contains("starting_after=mtr_000"));

        let body = r#"{
            "data": [
                {
                    "id": "mtr_list_001",
                    "object": "billing.meter",
                    "display_name": "Meter 1",
                    "event_name": "tokens",
                    "status": "active",
                    "value_settings": { "event_payload_key": "val" },
                    "customer_mapping": { "event_payload_key": "cus_id", "type": "by_id" },
                    "aggregation_formula": "sum",
                    "created": 1728000000,
                    "updated": 1728000000,
                    "livemode": false
                },
                {
                    "id": "mtr_list_002",
                    "object": "billing.meter",
                    "display_name": "Meter 2",
                    "event_name": "api_calls",
                    "status": "active",
                    "value_settings": { "event_payload_key": "calls" },
                    "customer_mapping": { "event_payload_key": "cus_id", "type": "by_id" },
                    "aggregation_formula": "count",
                    "created": 1728000100,
                    "updated": 1728000100,
                    "livemode": false
                }
            ],
            "next_page_url": "/v2/billing/meters?starting_after=mtr_list_002"
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_meters").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let resource = BillingMetersResource::new(&client);

    let params = ListBillingMetersParams::new()
        .limit(2)
        .status(MeterStatus::Active)
        .starting_after("mtr_000");

    let meters = resource.list(&params).await.unwrap();
    assert_eq!(meters.len(), 2);
    assert_eq!(meters[0].id, "mtr_list_001");
    assert_eq!(meters[1].id, "mtr_list_002");

    server.await.unwrap();
}

#[tokio::test]
async fn test_path_traversal_validation() {
    let client = StripeClient::new("sk_test_dummy");
    let resource = BillingMetersResource::new(&client);

    let bad_ids = [
        "",
        "   ",
        "../bad_id",
        "bad/id",
        "bad\\id",
        "bad?query=1",
        "bad#hash",
        "bad\nid",
    ];

    for bad in bad_ids {
        assert!(
            matches!(
                resource.retrieve(bad).await,
                Err(StripeError::InvalidRequest(_))
            ),
            "Expected InvalidRequest for retrieve('{bad}')"
        );

        let update_params = UpdateBillingMeterParams::new();
        assert!(
            matches!(
                resource.update(bad, &update_params).await,
                Err(StripeError::InvalidRequest(_))
            ),
            "Expected InvalidRequest for update('{bad}')"
        );

        assert!(
            matches!(
                resource.deactivate(bad).await,
                Err(StripeError::InvalidRequest(_))
            ),
            "Expected InvalidRequest for deactivate('{bad}')"
        );

        assert!(
            matches!(
                resource.reactivate(bad).await,
                Err(StripeError::InvalidRequest(_))
            ),
            "Expected InvalidRequest for reactivate('{bad}')"
        );
    }
}

#[test]
fn test_client_billing_meters_accessor() {
    let client = StripeClient::new("sk_test_mock");
    let resource = client.billing_meters();
    assert_eq!(
        resource.client().config.secret_key.expose_secret(),
        "sk_test_mock"
    );
}
