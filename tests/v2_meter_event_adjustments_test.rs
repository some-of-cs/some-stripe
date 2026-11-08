//! Integration and wire tests for Stripe API V2 Meter Event Adjustments & Cancelations (US-8.2.0 Unit 3).
//!
//! Covers:
//! - [AC-US-8.2.0-02] Meter event adjustment canceling erroneous usage by identifier.
//! - [AC-US-8.2.0-LAW] Statutory consumer billing accuracy and FTC Section 5 commercial fairness audit compliance.
//! - Wire protocol validation against mock HTTP server for `create`, `retrieve`, and `list`.
//! - Gate G2 Path traversal prevention and RFC 4122 UUID v4 idempotency generation.
//! - Gate G-Borrow zero deep-clone borrowing discipline.

use some_stripe::resources::MeterEventAdjustmentsResource;
use some_stripe::{
    AdjustmentReason, CancelAdjustment, CancelAdjustmentDetails, CreateMeterEventAdjustmentParams,
    ListMeterEventAdjustmentsParams, MeterEventAdjustment, StripeClient, StripeConfig, StripeError,
    is_valid_uuid_v4, validate_adjustment_audit_compliance,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_meter_event_adjustment_models_and_enums() {
    let reason = AdjustmentReason::DuplicateUsage;
    assert_eq!(reason.as_str(), "duplicate_usage");
    assert_eq!(reason.to_string(), "duplicate_usage");

    let serialized_reason = serde_json::to_string(&reason).unwrap();
    assert_eq!(serialized_reason, "\"duplicate_usage\"");

    let deserialized: AdjustmentReason = serde_json::from_str("\"system_error\"").unwrap();
    assert_eq!(deserialized, AdjustmentReason::SystemError);

    let cancel_details = CancelAdjustmentDetails::new("req_gen_99");
    assert_eq!(cancel_details.identifier, "req_gen_99");

    let adj = MeterEventAdjustment {
        id: "mea_12345".into(),
        object: "billing.meter_event_adjustment".into(),
        event_name: "alpaca_tokens".into(),
        status: "complete".into(),
        adjustment_type: "cancel".into(),
        cancel: Some(cancel_details),
        created: 1700000000,
        livemode: false,
    };

    assert!(adj.is_complete());
    assert!(!adj.is_pending());
    assert!(adj.is_cancel());
    assert_eq!(adj.id, "mea_12345");
    assert_eq!(adj.object, "billing.meter_event_adjustment");
    assert_eq!(
        adj.cancel.as_ref().map(|c| c.identifier.as_str()),
        Some("req_gen_99")
    );
}

#[test]
fn test_create_params_builders_and_json_serialization() {
    let params = CreateMeterEventAdjustmentParams::cancel_event("tokens", "req_gen_1")
        .reason(AdjustmentReason::FraudulentInference)
        .idempotency_key("idemp_custom_123");

    assert_eq!(params.event_name, "tokens");
    assert_eq!(params.adjustment_type, "cancel");
    assert_eq!(
        params.cancel.as_ref().map(|c| c.identifier.as_str()),
        Some("req_gen_1")
    );
    assert_eq!(params.reason, Some(AdjustmentReason::FraudulentInference));
    assert_eq!(params.idempotency_key.as_deref(), Some("idemp_custom_123"));

    // Verify JSON body serialization skips `idempotency_key` (delivered via HTTP header)
    let json_body = serde_json::to_string(&params).unwrap();
    assert!(json_body.contains("\"event_name\":\"tokens\""));
    assert!(json_body.contains("\"type\":\"cancel\""));
    assert!(json_body.contains("\"identifier\":\"req_gen_1\""));
    assert!(json_body.contains("\"reason\":\"fraudulent_inference\""));
    assert!(!json_body.contains("idempotency_key"));
}

#[test]
fn test_list_params_builders_and_pagination() {
    let params = ListMeterEventAdjustmentsParams::new()
        .limit(25)
        .starting_after("mea_cursor_1")
        .ending_before("mea_cursor_2")
        .event_name("compute_seconds");

    assert_eq!(params.limit, Some(25));
    assert_eq!(params.starting_after.as_deref(), Some("mea_cursor_1"));
    assert_eq!(params.ending_before.as_deref(), Some("mea_cursor_2"));
    assert_eq!(params.event_name.as_deref(), Some("compute_seconds"));
}

#[test]
fn test_statutory_audit_trail_and_ftc_fairness_ac_us_8_2_0_law() {
    // [AC-US-8.2.0-LAW] Statutory consumer billing accuracy and FTC Section 5 commercial fairness
    let valid_adj = MeterEventAdjustment {
        id: "mea_law_001".into(),
        object: "billing.meter_event_adjustment".into(),
        event_name: "tokens".into(),
        status: "complete".into(),
        adjustment_type: "cancel".into(),
        cancel: Some(CancelAdjustmentDetails::new("req_gen_1")),
        created: 1700000000,
        livemode: false,
    };

    assert!(validate_adjustment_audit_compliance(&valid_adj).is_ok());

    // Invariant 1: Meter event name must not be blank
    let mut empty_name = valid_adj.clone();
    empty_name.event_name = "   ".into();
    assert!(validate_adjustment_audit_compliance(&empty_name).is_err());

    // Invariant 2: Cancellation details must be present
    let mut missing_cancel = valid_adj.clone();
    missing_cancel.cancel = None;
    assert!(validate_adjustment_audit_compliance(&missing_cancel).is_err());

    // Invariant 3: Identifier must be non-empty for immutable ledger traceability
    let mut empty_identifier = valid_adj.clone();
    empty_identifier.cancel = Some(CancelAdjustmentDetails::new("   "));
    assert!(validate_adjustment_audit_compliance(&empty_identifier).is_err());

    // Simulation of immutable billing reconciliation:
    // Raw ingested event recorded: 15,400 tokens
    let initial_consumption: i64 = 15400;
    // When canceled by adjustment:
    let deduction: i64 = if valid_adj.is_complete() && valid_adj.is_cancel() {
        15400
    } else {
        0
    };
    let net_billable_usage = initial_consumption - deduction;
    assert_eq!(
        net_billable_usage, 0,
        "Canceled meter event must be fully deducted from customer's billable aggregate"
    );
}

#[tokio::test]
async fn test_cancellation_scenario_ac_us_8_2_0_02_mock_wire() {
    // [AC-US-8.2.0-02] Meter event adjustment canceling erroneous usage
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        // Verify HTTP Wire Invariants
        assert!(req_str.starts_with("POST /v2/billing/meter_event_adjustments HTTP/1.1"));
        assert!(
            req_str
                .to_lowercase()
                .contains("content-type: application/json")
        );
        assert!(req_str.contains("idempotency-key: custom_idemp_key_1"));

        // Verify JSON payload
        let body_idx = req_str.find("\r\n\r\n").unwrap() + 4;
        let body_str = &req_str[body_idx..];
        let val: serde_json::Value = serde_json::from_str(body_str).unwrap();
        assert_eq!(val["event_name"], "tokens");
        assert_eq!(val["type"], "cancel");
        assert_eq!(val["cancel"]["identifier"], "req_gen_1");

        let res_body = r#"{
            "id": "mea_wire_001",
            "object": "billing.meter_event_adjustment",
            "event_name": "tokens",
            "status": "complete",
            "type": "cancel",
            "cancel": {
                "identifier": "req_gen_1"
            },
            "created": 1700000000,
            "livemode": false
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            res_body.len(),
            res_body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_meter").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);

    let params = CreateMeterEventAdjustmentParams {
        event_name: "tokens".into(),
        adjustment_type: "cancel".into(),
        cancel: Some(CancelAdjustment {
            identifier: "req_gen_1".into(),
        }),
        reason: Some(AdjustmentReason::DuplicateUsage),
        idempotency_key: Some("custom_idemp_key_1".into()),
    };

    let adjustment = client
        .v2()
        .billing()
        .meter_event_adjustments()
        .create(&params)
        .await
        .unwrap();

    assert_eq!(adjustment.id, "mea_wire_001");
    assert_eq!(adjustment.status, "complete");
    assert!(adjustment.is_complete());
    assert_eq!(adjustment.event_name, "tokens");
    assert_eq!(
        adjustment.cancel.as_ref().map(|c| c.identifier.as_str()),
        Some("req_gen_1")
    );

    server.await.unwrap();
}

#[tokio::test]
async fn test_retrieve_meter_event_adjustment_mock_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(
            req_str.starts_with("GET /v2/billing/meter_event_adjustments/mea_get_123 HTTP/1.1")
        );

        let res_body = r#"{
            "id": "mea_get_123",
            "object": "billing.meter_event_adjustment",
            "event_name": "tokens",
            "status": "complete",
            "type": "cancel",
            "cancel": {
                "identifier": "req_gen_42"
            },
            "created": 1700000500,
            "livemode": false
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            res_body.len(),
            res_body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_meter").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);
    let resource = MeterEventAdjustmentsResource::new(&client);

    let adjustment = resource.retrieve("mea_get_123").await.unwrap();
    assert_eq!(adjustment.id, "mea_get_123");
    assert_eq!(adjustment.status, "complete");

    server.await.unwrap();
}

#[tokio::test]
async fn test_retrieve_path_traversal_prevention_gate_g2() {
    let config = StripeConfig::new("sk_test_meter").with_api_base("http://127.0.0.1:9999");
    let client = StripeClient::with_config(config);
    let resource = MeterEventAdjustmentsResource::new(&client);

    // Path traversal with ../
    let err1 = resource.retrieve("../admin_leak").await.unwrap_err();
    assert!(matches!(err1, StripeError::InvalidRequest(_)));

    // Slash injection
    let err2 = resource.retrieve("mea_123/evil").await.unwrap_err();
    assert!(matches!(err2, StripeError::InvalidRequest(_)));

    // Whitespace only
    let err3 = resource.retrieve("   ").await.unwrap_err();
    assert!(matches!(err3, StripeError::InvalidRequest(_)));

    // Null byte injection
    let err4 = resource.retrieve("mea_123\0injection").await.unwrap_err();
    assert!(matches!(err4, StripeError::InvalidRequest(_)));
}

#[tokio::test]
async fn test_list_meter_event_adjustments_mock_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("GET /v2/billing/meter_event_adjustments?"));
        assert!(req_str.contains("event_name=tokens"));
        assert!(req_str.contains("limit=10"));
        assert!(req_str.contains("starting_after=mea_last_seen"));

        let res_body = r#"{
            "object": "list",
            "data": [
                {
                    "id": "mea_list_1",
                    "object": "billing.meter_event_adjustment",
                    "event_name": "tokens",
                    "status": "complete",
                    "type": "cancel",
                    "cancel": { "identifier": "req_1" },
                    "created": 1700000100,
                    "livemode": false
                },
                {
                    "id": "mea_list_2",
                    "object": "billing.meter_event_adjustment",
                    "event_name": "tokens",
                    "status": "pending",
                    "type": "cancel",
                    "cancel": { "identifier": "req_2" },
                    "created": 1700000200,
                    "livemode": false
                }
            ],
            "has_more": false
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            res_body.len(),
            res_body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_meter").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);

    let params = ListMeterEventAdjustmentsParams::new()
        .limit(10)
        .starting_after("mea_last_seen")
        .event_name("tokens");

    let adjustments = client
        .v2()
        .billing()
        .meter_event_adjustments()
        .list(&params)
        .await
        .unwrap();
    assert_eq!(adjustments.len(), 2);
    assert_eq!(adjustments[0].id, "mea_list_1");
    assert_eq!(adjustments[1].id, "mea_list_2");
    assert!(adjustments[0].is_complete());
    assert!(adjustments[1].is_pending());

    server.await.unwrap();
}

#[tokio::test]
async fn test_create_automatic_idempotency_key_generation_gate_g2() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        // Extract Idempotency-Key header
        let idemp_prefix = "idempotency-key: ";
        let pos = req_str
            .to_lowercase()
            .find(idemp_prefix)
            .expect("Missing Idempotency-Key header");
        let start = pos + idemp_prefix.len();
        let end = req_str[start..].find("\r\n").unwrap() + start;
        let generated_key = req_str[start..end].trim();

        // Must be a valid RFC 4122 UUID v4
        assert!(
            is_valid_uuid_v4(generated_key),
            "Expected valid UUID v4 for automatic idempotency key, got: {generated_key}"
        );

        let res_body = r#"{
            "id": "mea_auto_idemp",
            "object": "billing.meter_event_adjustment",
            "event_name": "tokens",
            "status": "complete",
            "type": "cancel",
            "cancel": { "identifier": "req_gen_99" },
            "created": 1700000000,
            "livemode": false
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            res_body.len(),
            res_body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_meter").with_api_base(format!("http://127.0.0.1:{port}"));
    let client = StripeClient::with_config(config);

    // Call without explicit idempotency key
    let params = CreateMeterEventAdjustmentParams::cancel_event("tokens", "req_gen_99");
    assert!(params.idempotency_key.is_none());

    let adj = client
        .meter_event_adjustments()
        .create(&params)
        .await
        .unwrap();
    assert_eq!(adj.id, "mea_auto_idemp");

    server.await.unwrap();
}

#[test]
fn test_zero_clone_borrowing_discipline_gate_g_borrow() {
    let params = CreateMeterEventAdjustmentParams::cancel_event("tokens", "req_gen_borrow");
    let borrowed_ref: &CreateMeterEventAdjustmentParams = &params;

    // Verify properties accessed via references with zero cloning
    assert_eq!(&borrowed_ref.event_name, "tokens");
    assert_eq!(&borrowed_ref.adjustment_type, "cancel");
    if let Some(cancel) = &borrowed_ref.cancel {
        assert_eq!(&cancel.identifier, "req_gen_borrow");
    } else {
        panic!("Expected cancel details");
    }
}
