//! Integration and wire mock tests for Stripe API V2 Billing Meter Event Streams (US-8.2.0 Unit 2).

use some_stripe::{
    BatchIngestionResponse, CreateMeterEventParams, CreateMeterEventStreamParams,
    DEFAULT_TIMESTAMP_FUTURE_DRIFT_SECS, DEFAULT_TIMESTAMP_MAX_PAST_SECS, MeterBatchError,
    MeterBatchSerializer, MeterEventBatch, MeterEventPayload, MeterPayload, StripeClient,
    StripeConfig, StripeError, V2MeterEvent, verify_timestamp,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_meter_event_payload_and_batch_models() {
    let payload = MeterPayload::new("cus_test_123", "4500");
    assert_eq!(payload.stripe_customer_id(), "cus_test_123");
    assert_eq!(payload.value(), "4500");

    let event = MeterEventPayload::from_payload("ai_tokens", payload)
        .with_timestamp(1_728_000_000)
        .with_identifier("idemp_tok_001");

    assert_eq!(event.event_name(), "ai_tokens");
    assert_eq!(event.payload().stripe_customer_id(), "cus_test_123");
    assert_eq!(event.payload().value(), "4500");
    assert_eq!(event.timestamp(), Some(1_728_000_000));
    assert_eq!(event.identifier(), Some("idemp_tok_001"));

    // Batch validation
    let empty_res = MeterEventBatch::new(vec![]);
    assert_eq!(empty_res, Err(MeterBatchError::EmptyBatch));

    let batch = MeterEventBatch::new(vec![event.clone()]).expect("valid batch");
    assert_eq!(batch.len(), 1);
    assert!(!batch.is_empty());
    assert_eq!(batch.as_slice().len(), 1);
    assert_eq!(batch.iter().count(), 1);
    assert_eq!(batch[0].event_name(), "ai_tokens");

    let into_events = batch.into_vec();
    assert_eq!(into_events.len(), 1);
}

#[test]
fn test_v2_meter_event_model_serialization() {
    let json = serde_json::json!({
        "object": "v2.billing.meter_event",
        "created": 1728345600,
        "livemode": false,
        "identifier": "idmp_wire_12345",
        "event_name": "ai_inference_compute",
        "timestamp": 1728345590,
        "payload": {
            "stripe_customer_id": "cus_prod_888",
            "value": "128"
        }
    });

    let event: V2MeterEvent =
        serde_json::from_value(json).expect("should deserialize v2 meter event");
    assert_eq!(event.object(), "v2.billing.meter_event");
    assert_eq!(event.created(), 1728345600);
    assert!(!event.livemode());
    assert_eq!(event.identifier(), Some("idmp_wire_12345"));
    assert_eq!(event.event_name(), "ai_inference_compute");
    assert_eq!(event.timestamp(), Some(1728345590));
    assert_eq!(event.customer_id(), "cus_prod_888");
    assert_eq!(event.value(), "128");

    // Builder test
    let constructed = V2MeterEvent::new("ai_inference_compute", "cus_prod_888", "128")
        .with_created(1728345600)
        .with_identifier("idmp_wire_12345")
        .with_timestamp(1728345590)
        .with_livemode(false);
    assert_eq!(event, constructed);
}

#[test]
fn test_create_meter_event_params_and_validation() {
    let now = 1_700_000_000;
    let params = CreateMeterEventParams::new("gpu_minutes", "cus_compute_01", "60")
        .with_identifier("evt_gpu_001")
        .with_timestamp(now);

    assert!(params.validate_with_timestamp(now).is_ok());

    let empty_name = CreateMeterEventParams::new("   ", "cus_compute_01", "60");
    assert!(empty_name.validate_with_timestamp(now).is_err());

    let empty_cus = CreateMeterEventParams::new("gpu_minutes", "", "60");
    assert!(empty_cus.validate_with_timestamp(now).is_err());
}

#[test]
fn test_timestamp_bounds_verification() {
    let now = 1_700_000_000;

    // Current time
    assert!(verify_timestamp(now, now).is_ok());

    // Within allowable future drift
    assert!(verify_timestamp(now + DEFAULT_TIMESTAMP_FUTURE_DRIFT_SECS, now).is_ok());

    // Beyond allowable future drift
    let future_err =
        verify_timestamp(now + DEFAULT_TIMESTAMP_FUTURE_DRIFT_SECS + 1, now).unwrap_err();
    assert!(matches!(
        future_err,
        MeterBatchError::FutureTimestamp { .. }
    ));

    // Within allowable past age
    assert!(verify_timestamp(now - DEFAULT_TIMESTAMP_MAX_PAST_SECS, now).is_ok());

    // Beyond allowable past age
    let past_err = verify_timestamp(now - DEFAULT_TIMESTAMP_MAX_PAST_SECS - 1, now).unwrap_err();
    assert!(matches!(past_err, MeterBatchError::TimestampTooOld { .. }));

    // Batch timestamp verification
    let batch = MeterEventBatch::new(vec![
        MeterEventPayload::new("tokens", "cus_1", "10").with_timestamp(now),
        MeterEventPayload::new("tokens", "cus_2", "20").with_timestamp(now + 100),
    ])
    .expect("valid batch");
    assert!(batch.verify_timestamps(now).is_ok());

    let invalid_batch = MeterEventBatch::new(vec![
        MeterEventPayload::new("tokens", "cus_1", "10").with_timestamp(now + 500),
    ])
    .expect("valid batch");
    assert!(invalid_batch.verify_timestamps(now).is_err());
}

#[test]
fn test_meter_batch_serializer_json_and_ndjson() {
    let events = vec![
        MeterEventPayload::new("tokens", "cus_1", "100")
            .with_identifier("req_1")
            .with_timestamp(1_700_000_000),
        MeterEventPayload::new("tokens", "cus_2", "200").with_identifier("req_2"),
    ];

    let json = MeterBatchSerializer::serialize_batch_json(&events).expect("valid json");
    assert!(json.starts_with("{\"events\":["));
    assert!(json.ends_with("]}"));
    assert!(json.contains("\"event_name\":\"tokens\""));
    assert!(json.contains("\"stripe_customer_id\":\"cus_1\""));
    assert!(json.contains("\"stripe_customer_id\":\"cus_2\""));

    let ndjson = MeterBatchSerializer::serialize_ndjson(&events).expect("valid ndjson");
    let lines: Vec<&str> = ndjson.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("\"cus_1\""));
    assert!(lines[1].contains("\"cus_2\""));
}

#[test]
fn test_high_throughput_serializer_benchmark() {
    let mut events = Vec::with_capacity(1000);
    for i in 0..1000 {
        events.push(
            MeterEventPayload::new("compute_cycles", format!("cus_{i}"), format!("{}", i * 100))
                .with_identifier(format!("idemp_bench_{i}"))
                .with_timestamp(1_700_000_000 + i),
        );
    }

    let start = std::time::Instant::now();
    let json = MeterBatchSerializer::serialize_batch_json(&events).expect("serialize 1000 events");
    let elapsed = start.elapsed();

    assert_eq!(
        json.matches("\"event_name\":\"compute_cycles\"").count(),
        1000
    );
    assert!(
        elapsed.as_millis() < 50,
        "High-throughput batch serialization exceeded 50ms: {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_wire_create_meter_event_single_endpoint() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("POST /v2/billing/meter_events HTTP/1.1"));
        assert!(req_str.contains("content-type: application/json"));
        assert!(req_str.contains("accept: application/json"));
        assert!(req_str.contains("idempotency-key: idemp_wire_single_99"));
        assert!(req_str.contains("\"event_name\":\"api_queries\""));
        assert!(req_str.contains("\"stripe_customer_id\":\"cus_wire_123\""));
        assert!(req_str.contains("\"value\":\"15\""));

        let response_payload = serde_json::json!({
            "object": "v2.billing.meter_event",
            "created": 1728345600,
            "livemode": false,
            "identifier": "idemp_wire_single_99",
            "event_name": "api_queries",
            "timestamp": 1728345590,
            "payload": {
                "stripe_customer_id": "cus_wire_123",
                "value": "15"
            }
        })
        .to_string();

        let http_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            response_payload.len(),
            response_payload
        );
        socket.write_all(http_response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_wire_mock").with_api_base(format!("http://127.0.0.1:{}", port));
    let client = StripeClient::with_config(config);

    let params = CreateMeterEventParams::new("api_queries", "cus_wire_123", "15")
        .with_identifier("idemp_wire_single_99")
        .with_timestamp(1728345590);

    let event = client
        .meter_events()
        .create(&params)
        .await
        .expect("create meter event must succeed");

    assert_eq!(event.object(), "v2.billing.meter_event");
    assert_eq!(event.event_name(), "api_queries");
    assert_eq!(event.customer_id(), "cus_wire_123");
    assert_eq!(event.value(), "15");
    assert_eq!(event.identifier(), Some("idemp_wire_single_99"));

    server.await.unwrap();
}

#[tokio::test]
async fn test_wire_create_meter_event_stream_batch_endpoint() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("POST /v2/billing/meter_event_stream HTTP/1.1"));
        assert!(req_str.contains("content-type: application/json"));
        assert!(req_str.contains("accept: application/json"));
        assert!(req_str.contains("\"events\":["));
        assert!(req_str.contains("\"event_name\":\"token_usage\""));
        assert!(req_str.contains("\"stripe_customer_id\":\"cus_batch_1\""));
        assert!(req_str.contains("\"stripe_customer_id\":\"cus_batch_2\""));

        let response_payload = serde_json::json!({
            "events_ingested": 2,
            "status": "success",
            "warnings": null
        })
        .to_string();

        let http_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            response_payload.len(),
            response_payload
        );
        socket.write_all(http_response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_wire_mock").with_api_base(format!("http://127.0.0.1:{}", port));
    let client = StripeClient::with_config(config);

    let events = vec![
        MeterEventPayload::new("token_usage", "cus_batch_1", "120"),
        MeterEventPayload::new("token_usage", "cus_batch_2", "340"),
    ];

    let res = client
        .meter_event_stream()
        .create_batch(&events, None)
        .await
        .expect("batch ingestion must succeed");

    assert_eq!(res.events_ingested(), 2);
    assert_eq!(res.status(), "success");
    assert!(res.is_success());

    server.await.unwrap();
}

#[tokio::test]
async fn test_wire_create_with_session_bearer_auth() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("POST /v2/billing/meter_event_stream HTTP/1.1"));
        assert!(req_str.contains("authorization: Bearer sess_tok_meter_ephemeral_xyz"));

        let response_payload = serde_json::json!({
            "events_ingested": 1,
            "status": "success"
        })
        .to_string();

        let http_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            response_payload.len(),
            response_payload
        );
        socket.write_all(http_response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_wire_mock").with_api_base(format!("http://127.0.0.1:{}", port));
    let client = StripeClient::with_config(config);

    let params = CreateMeterEventStreamParams::single(MeterEventPayload::new(
        "ai_tokens",
        "cus_session_99",
        "1000",
    ));

    let res = client
        .meter_event_stream()
        .create_with_session(&params, "sess_tok_meter_ephemeral_xyz")
        .await
        .expect("session ingestion must succeed");

    assert_eq!(res.events_ingested(), 1);
    assert!(res.is_success());

    server.await.unwrap();
}

#[tokio::test]
async fn test_wire_connected_account_tenant_header() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.contains("stripe-account: acct_connected_tenant_01"));

        let response_payload = serde_json::json!({
            "events_ingested": 1,
            "status": "success"
        })
        .to_string();

        let http_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            response_payload.len(),
            response_payload
        );
        socket.write_all(http_response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_wire_mock").with_api_base(format!("http://127.0.0.1:{}", port));
    let client = StripeClient::with_config(config);

    let params = CreateMeterEventStreamParams::single(MeterEventPayload::new(
        "storage_gb",
        "cus_tenant_01",
        "50",
    ));

    let res = client
        .meter_event_stream()
        .with_account("acct_connected_tenant_01")
        .create(&params)
        .await
        .expect("tenant ingestion must succeed");

    assert_eq!(res.events_ingested(), 1);

    server.await.unwrap();
}

#[tokio::test]
async fn test_wire_validation_errors_empty_and_oversized_batches() {
    let client = StripeClient::new("sk_test_wire_mock");

    // Empty batch rejection
    let empty_events: Vec<MeterEventPayload> = vec![];
    let err = client
        .meter_event_stream()
        .create_batch(&empty_events, None)
        .await
        .unwrap_err();
    assert!(matches!(err, StripeError::InvalidRequest(_)));

    // Oversized batch rejection
    let oversized: Vec<_> = (0..1001)
        .map(|i| MeterEventPayload::new("tokens", format!("cus_{i}"), "1"))
        .collect();
    let err = client
        .meter_event_stream()
        .create_batch(&oversized, None)
        .await
        .unwrap_err();
    assert!(matches!(err, StripeError::InvalidRequest(_)));
}
