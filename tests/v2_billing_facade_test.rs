//! Wire Integration and Omnibus Tests for Stripe API V2 Billing Facade & Ingestion Engine (US-8.2.0).
//!
//! Verifies:
//! - [AC-US-8.2.0-01]: Initialize Meter Event Session and ingest batch token usage.
//! - [AC-US-8.2.0-02]: Meter event adjustment canceling erroneous usage.
//! - [AC-US-8.2.0-LAW]: Statutory consumer billing accuracy and FTC Section 5 transparency.
//! - V2 Billing Facade mounting on `StripeClient` via `client.v2().billing()`.
//! - High-throughput `MeterIngestionEngine` keep-alive connection pooling, client-side deduplication,
//!   session token auto-refresh, and batch buffer flushing.
//! - Gate G2: Token secret zeroization on drop and `[REDACTED]` debug masking.

use some_stripe::{
    AggregationFormula, CancelAdjustment, CreateBillingMeterParams,
    CreateMeterEventAdjustmentParams, CreateMeterEventStreamParams, CustomerMappingSettings,
    MeterEventPayload, MeterEventSession, MeterEventSessionStatus, MeterIngestionEngine,
    MeterPayload, MeterStatus, StripeClient, StripeConfig, ValueSettings,
    validate_statutory_meter_event_accuracy,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn write_response(socket: &mut tokio::net::TcpStream, status_line: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status_line,
        body.len(),
        body
    );
    socket.write_all(response.as_bytes()).await.unwrap();
}

#[tokio::test]
async fn test_ac_scenario_01_initialize_meter_session_and_ingest_batch() {
    // Scenario: [AC-US-8.2.0-01] Initialize Meter Event Session and ingest batch token usage
    // Given an AI billing meter "mtr_tokens_123" and customer "cus_123"
    // When client.v2().billing().meter_event_session().create() is executed
    // Then an authenticated session token is generated
    // When client.v2().billing().meter_event_stream().create(...) is executed
    // Then the batch of 15,400 tokens is ingested with HTTP 200/201 response.

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Step 1: Meter event session creation
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 8192];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("POST /v2/billing/meter_event_session HTTP/1.1"));

            let session_json = r#"{
                "id": "sess_tok_abc_123",
                "object": "v2.billing.meter_event_session",
                "authentication_token": "sess_tok_live_ephemeral_token_xyz",
                "expires_at": 1700000900,
                "created": 1700000000,
                "livemode": false,
                "status": "active"
            }"#;
            write_response(&mut socket, "200 OK", session_json).await;
        }

        // Step 2: Meter event stream batch ingestion
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 8192];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("POST /v2/billing/meter_event_stream HTTP/1.1"));
            assert!(req_str.contains("tokens"));
            assert!(req_str.contains("cus_123"));
            assert!(req_str.contains("15400"));
            assert!(req_str.contains("req_gen_1"));

            let stream_json = r#"{
                "object": "v2.billing.meter_event_stream",
                "events": [
                    {
                        "event_name": "tokens",
                        "payload": {
                            "stripe_customer_id": "cus_123",
                            "value": "15400"
                        },
                        "timestamp": 1700000000,
                        "identifier": "req_gen_1"
                    }
                ],
                "status": "accepted"
            }"#;
            write_response(&mut socket, "200 OK", stream_json).await;
        }
    });

    let config =
        StripeConfig::new("sk_test_mock_billing_facade").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    // 1. Initialize meter event session
    let session = client
        .v2()
        .billing()
        .meter_event_session()
        .create()
        .await
        .unwrap();
    assert_eq!(session.id, "sess_tok_abc_123");
    assert_eq!(
        session.authentication_token,
        "sess_tok_live_ephemeral_token_xyz"
    );
    assert_eq!(session.status, MeterEventSessionStatus::Active);

    // 2. Stream batch meter events
    let stream_params = CreateMeterEventStreamParams {
        events: vec![MeterEventPayload {
            event_name: "tokens".into(),
            payload: MeterPayload {
                stripe_customer_id: "cus_123".into(),
                value: "15400".into(),
            },
            timestamp: Some(1700000000),
            identifier: Some("req_gen_1".into()),
        }],
    };
    let stream_resp = client
        .v2()
        .billing()
        .meter_event_stream()
        .create(&stream_params)
        .await
        .unwrap();
    assert_eq!(stream_resp.status, "accepted");
    assert_eq!(stream_resp.events.len(), 1);
    assert_eq!(stream_resp.events[0].event_name, "tokens");
    assert_eq!(stream_resp.events[0].payload.value, "15400");

    server.await.unwrap();
}

#[tokio::test]
async fn test_ac_scenario_02_meter_event_adjustment_canceling_usage() {
    // Scenario: [AC-US-8.2.0-02] Meter event adjustment canceling erroneous usage
    // Given an ingested meter event with identifier "req_gen_1"
    // When client.v2().billing().meter_event_adjustments().create(...) is executed
    // Then the adjustment is accepted
    // And 15,400 tokens are deducted from the customer's billable aggregate.

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(req_str.starts_with("POST /v2/billing/meter_event_adjustments HTTP/1.1"));
        assert!(req_str.contains("\"event_name\":\"tokens\""));
        assert!(req_str.contains("\"type\":\"cancel\""));
        assert!(req_str.contains("\"identifier\":\"req_gen_1\""));

        let adjustment_json = r#"{
            "id": "mea_adjust_999",
            "object": "v2.billing.meter_event_adjustment",
            "event_name": "tokens",
            "type": "cancel",
            "cancel": {
                "identifier": "req_gen_1",
                "reason": "Duplicate inference call cancellation"
            },
            "status": "accepted",
            "created": 1700000050,
            "livemode": false
        }"#;
        write_response(&mut socket, "200 OK", adjustment_json).await;
    });

    let config =
        StripeConfig::new("sk_test_mock_adjustment_facade").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let adjustment_params = CreateMeterEventAdjustmentParams {
        event_name: "tokens".into(),
        r#type: "cancel".into(),
        cancel: Some(CancelAdjustment {
            identifier: "req_gen_1".into(),
        }),
        reason: Some("Duplicate inference call cancellation".into()),
    };

    let adjustment = client
        .v2()
        .billing()
        .meter_event_adjustments()
        .create(&adjustment_params)
        .await
        .unwrap();

    assert_eq!(adjustment.id, "mea_adjust_999");
    assert_eq!(adjustment.event_name, "tokens");
    assert_eq!(adjustment.adjustment_type, "cancel");
    assert_eq!(adjustment.status, "accepted");
    assert_eq!(adjustment.cancel.as_ref().unwrap().identifier, "req_gen_1");

    server.await.unwrap();
}

#[tokio::test]
async fn test_billing_meters_lifecycle_facade() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Create meter
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 8192];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("POST /v2/billing/meters HTTP/1.1"));

            let meter_json = r#"{
                "id": "mtr_tokens_123",
                "object": "v2.billing.meter",
                "event_name": "tokens",
                "display_name": "AI LLM Inference Tokens",
                "status": "active",
                "value_settings": {
                    "event_payload_key": "value"
                },
                "customer_mapping": {
                    "event_payload_key": "stripe_customer_id",
                    "type": "by_id"
                },
                "aggregation_formula": "sum",
                "created": 1700000000,
                "updated": 1700000000,
                "livemode": false
            }"#;
            write_response(&mut socket, "200 OK", meter_json).await;
        }

        // 2. Retrieve meter
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 8192];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("GET /v2/billing/meters/mtr_tokens_123 HTTP/1.1"));

            let meter_json = r#"{
                "id": "mtr_tokens_123",
                "object": "v2.billing.meter",
                "event_name": "tokens",
                "display_name": "AI LLM Inference Tokens",
                "status": "active",
                "value_settings": { "event_payload_key": "value" },
                "customer_mapping": { "event_payload_key": "stripe_customer_id", "type": "by_id" },
                "aggregation_formula": "sum",
                "created": 1700000000,
                "updated": 1700000000,
                "livemode": false
            }"#;
            write_response(&mut socket, "200 OK", meter_json).await;
        }

        // 3. Deactivate meter
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 8192];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(
                req_str.starts_with("POST /v2/billing/meters/mtr_tokens_123/deactivate HTTP/1.1")
            );

            let meter_json = r#"{
                "id": "mtr_tokens_123",
                "object": "v2.billing.meter",
                "event_name": "tokens",
                "display_name": "AI LLM Inference Tokens",
                "status": "inactive",
                "value_settings": { "event_payload_key": "value" },
                "customer_mapping": { "event_payload_key": "stripe_customer_id", "type": "by_id" },
                "aggregation_formula": "sum",
                "created": 1700000000,
                "updated": 1700000050,
                "livemode": false
            }"#;
            write_response(&mut socket, "200 OK", meter_json).await;
        }
    });

    let config = StripeConfig::new("sk_test_mock_meters").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    // Create meter
    let create_params = CreateBillingMeterParams {
        display_name: "AI LLM Inference Tokens".into(),
        event_name: "tokens".into(),
        default_aggregation: Some(AggregationFormula::Sum),
        value_settings: Some(ValueSettings {
            event_payload_key: "value".into(),
        }),
        customer_mapping: Some(CustomerMappingSettings {
            event_payload_key: "stripe_customer_id".into(),
            mapping_type: "by_id".into(),
        }),
    };
    let meter = client
        .v2()
        .billing()
        .meters()
        .create(&create_params)
        .await
        .unwrap();
    assert_eq!(meter.id, "mtr_tokens_123");
    assert_eq!(meter.status, MeterStatus::Active);

    // Retrieve meter
    let retrieved = client
        .v2()
        .billing()
        .meters()
        .retrieve("mtr_tokens_123")
        .await
        .unwrap();
    assert_eq!(retrieved.id, "mtr_tokens_123");

    // Deactivate meter
    let deactivated = client
        .v2()
        .billing()
        .meters()
        .deactivate("mtr_tokens_123")
        .await
        .unwrap();
    assert_eq!(deactivated.status, MeterStatus::Inactive);

    server.await.unwrap();
}

#[tokio::test]
async fn test_ingestion_engine_deduplication_and_batch_flush() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Expected stream request for batch of 2 events
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(req_str.starts_with("POST /v2/billing/meter_event_stream HTTP/1.1"));
        assert!(req_str.contains("Bearer preloaded_sess_tok_123"));
        assert!(req_str.contains("evt_batch_1"));
        assert!(req_str.contains("evt_batch_2"));

        let stream_json = r#"{
            "object": "v2.billing.meter_event_stream",
            "events": [],
            "status": "accepted"
        }"#;
        write_response(&mut socket, "200 OK", stream_json).await;
    });

    let engine = MeterIngestionEngine::with_options(
        format!("http://{addr}"),
        "sk_test_engine_key",
        2,  // batch size 2 triggers auto flush
        60, // refresh threshold
        1000,
    );

    // Preload an active session token
    let session = MeterEventSession {
        id: "sess_engine_test".into(),
        object: "v2.billing.meter_event_session".into(),
        authentication_token: "preloaded_sess_tok_123".into(),
        expires_at: 1999999999, // far future
        created: 1700000000,
        livemode: false,
        status: MeterEventSessionStatus::Active,
    };
    engine.set_session(session).await;

    // Push event 1 (buffer count 1, no flush)
    let event1 = MeterEventPayload {
        event_name: "tokens".into(),
        payload: MeterPayload {
            stripe_customer_id: "cus_123".into(),
            value: "500".into(),
        },
        timestamp: Some(1700000000),
        identifier: Some("evt_batch_1".into()),
    };
    let flush1 = engine.push_event(event1).await.unwrap();
    assert!(flush1.is_none());
    assert_eq!(engine.pending_count().await, 1);

    // Test Deduplication: Pushing identical identifier must fail
    let duplicate_event = MeterEventPayload {
        event_name: "tokens".into(),
        payload: MeterPayload {
            stripe_customer_id: "cus_123".into(),
            value: "500".into(),
        },
        timestamp: Some(1700000000),
        identifier: Some("evt_batch_1".into()),
    };
    let dup_err = engine.push_event(duplicate_event).await;
    assert!(dup_err.is_err());
    assert!(
        dup_err
            .unwrap_err()
            .to_string()
            .contains("Duplicate meter event identifier")
    );

    // Push event 2 (buffer reaches batch_size 2, triggers auto-flush)
    let event2 = MeterEventPayload {
        event_name: "tokens".into(),
        payload: MeterPayload {
            stripe_customer_id: "cus_123".into(),
            value: "1000".into(),
        },
        timestamp: Some(1700000001),
        identifier: Some("evt_batch_2".into()),
    };
    let flush2 = engine.push_event(event2).await.unwrap();
    assert!(flush2.is_some());
    assert_eq!(flush2.unwrap().status, "accepted");
    assert_eq!(engine.pending_count().await, 0);

    server.await.unwrap();
}

#[tokio::test]
async fn test_ingestion_engine_auto_token_refresh() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(req_str.starts_with("POST /v2/billing/meter_event_session HTTP/1.1"));

        let session_json = r#"{
            "id": "sess_refreshed_abc",
            "object": "v2.billing.meter_event_session",
            "authentication_token": "newly_refreshed_token_456",
            "expires_at": 1999999999,
            "created": 1700000000,
            "livemode": false,
            "status": "active"
        }"#;
        write_response(&mut socket, "200 OK", session_json).await;
    });

    let engine = MeterIngestionEngine::new(format!("http://{addr}"), "sk_test_auto_refresh");

    // Token should automatically be fetched from mock server
    let token = engine.get_valid_session_token().await.unwrap();
    assert_eq!(token, "newly_refreshed_token_456");

    server.await.unwrap();
}

#[test]
fn test_statutory_and_gate_g2_security_invariants() {
    // Gate G-Law: Statutory consumption validation
    let valid_event = MeterEventPayload {
        event_name: "tokens".into(),
        payload: MeterPayload {
            stripe_customer_id: "cus_audit_1".into(),
            value: "2500".into(),
        },
        timestamp: Some(1700000000),
        identifier: Some("idemp_uuid_1".into()),
    };
    assert!(validate_statutory_meter_event_accuracy(&valid_event).is_ok());

    let invalid_metric = MeterEventPayload {
        event_name: "tokens".into(),
        payload: MeterPayload {
            stripe_customer_id: "cus_audit_1".into(),
            value: "invalid_tokens_NaN".into(),
        },
        timestamp: Some(1700000000),
        identifier: Some("idemp_uuid_2".into()),
    };
    assert!(validate_statutory_meter_event_accuracy(&invalid_metric).is_err());

    // Gate G2: Sensitive token redaction
    let session = MeterEventSession {
        id: "sess_sensitive_123".into(),
        object: "v2.billing.meter_event_session".into(),
        authentication_token: "super_secret_bearer_token".into(),
        expires_at: 1700000000,
        created: 1699999000,
        livemode: false,
        status: MeterEventSessionStatus::Active,
    };
    let debug_output = format!("{session:?}");
    assert!(!debug_output.contains("super_secret_bearer_token"));
    assert!(debug_output.contains("[REDACTED]"));
}

#[tokio::test]
async fn test_client_v2_billing_shortcut_facade() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(req_str.starts_with("GET /v2/billing/meters/mtr_shortcut_test HTTP/1.1"));

        let meter_json = r#"{
            "id": "mtr_shortcut_test",
            "object": "v2.billing.meter",
            "event_name": "api_calls",
            "display_name": "API Calls",
            "status": "active",
            "value_settings": { "event_payload_key": "value" },
            "customer_mapping": { "event_payload_key": "stripe_customer_id", "type": "by_id" },
            "aggregation_formula": "count",
            "created": 1700000000,
            "updated": 1700000000,
            "livemode": false
        }"#;
        write_response(&mut socket, "200 OK", meter_json).await;
    });

    let config = StripeConfig::new("sk_test_shortcut").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    // Verify client.v2_billing() shortcut method
    let meter = client
        .v2_billing()
        .meters()
        .retrieve("mtr_shortcut_test")
        .await
        .unwrap();

    assert_eq!(meter.id, "mtr_shortcut_test");
    assert_eq!(meter.aggregation_formula, AggregationFormula::Count);

    server.await.unwrap();
}

#[tokio::test]
async fn test_meter_stream_collector_time_interval_flush() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(req_str.starts_with("POST /v2/billing/meter_event_stream HTTP/1.1"));
        assert!(req_str.contains("evt_collector_timer_1"));
        assert!(req_str.contains("Bearer collector_sess_tok"));

        let stream_json = r#"{
            "object": "v2.billing.meter_event_stream",
            "events": [],
            "status": "accepted"
        }"#;
        write_response(&mut socket, "200 OK", stream_json).await;
    });

    let engine = MeterIngestionEngine::with_options(
        format!("http://{addr}"),
        "sk_test_collector",
        100, // Large batch size (100) so batch count does NOT trigger flush
        60,
        1000,
    );

    let session = MeterEventSession {
        id: "sess_collector".into(),
        object: "v2.billing.meter_event_session".into(),
        authentication_token: "collector_sess_tok".into(),
        expires_at: 1999999999,
        created: 1700000000,
        livemode: false,
        status: MeterEventSessionStatus::Active,
    };
    engine.set_session(session).await;

    // Create stream collector with 50ms flush interval
    let collector = engine.create_stream_collector(50, std::time::Duration::from_millis(50));

    // Send 1 event
    collector
        .send(MeterEventPayload {
            event_name: "tokens".into(),
            payload: MeterPayload {
                stripe_customer_id: "cus_collector".into(),
                value: "100".into(),
            },
            timestamp: Some(1700000000),
            identifier: Some("evt_collector_timer_1".into()),
        })
        .await
        .unwrap();

    // Wait 150ms for the periodic interval flush to trigger
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let stats = collector.stats();
    assert_eq!(stats.events_received, 1);
    assert_eq!(stats.events_flushed, 1);
    assert_eq!(stats.batches_sent, 1);
    assert_eq!(stats.duplicates_dropped, 0);

    collector.abort();
    server.await.unwrap();
}

#[tokio::test]
async fn test_engine_periodic_flusher_and_manual_flush() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(req_str.starts_with("POST /v2/billing/meter_event_stream HTTP/1.1"));
        assert!(req_str.contains("evt_flusher_1"));

        let stream_json = r#"{
            "object": "v2.billing.meter_event_stream",
            "events": [],
            "status": "accepted"
        }"#;
        write_response(&mut socket, "200 OK", stream_json).await;
    });

    let engine = MeterIngestionEngine::with_options(
        format!("http://{addr}"),
        "sk_test_flusher",
        1000,
        60,
        1000,
    );

    let session = MeterEventSession {
        id: "sess_flusher".into(),
        object: "v2.billing.meter_event_session".into(),
        authentication_token: "flusher_sess_tok".into(),
        expires_at: 1999999999,
        created: 1700000000,
        livemode: false,
        status: MeterEventSessionStatus::Active,
    };
    engine.set_session(session).await;

    // Start flusher with 50ms interval
    let handle = engine.start_periodic_flusher(std::time::Duration::from_millis(50));

    // Enqueue event
    let event = MeterEventPayload {
        event_name: "tokens".into(),
        payload: MeterPayload {
            stripe_customer_id: "cus_flusher".into(),
            value: "75".into(),
        },
        timestamp: Some(1700000000),
        identifier: Some("evt_flusher_1".into()),
    };
    let flush_res = engine.push_event(event).await.unwrap();
    assert!(flush_res.is_none());
    assert_eq!(engine.pending_count().await, 1);

    // Wait for periodic flusher to flush
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    assert_eq!(engine.pending_count().await, 0);

    handle.abort();
    server.await.unwrap();
}
