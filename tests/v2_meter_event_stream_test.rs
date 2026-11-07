//! Wire integration, throughput benchmark, and domain invariant tests for Stripe API V2
//! Meter Event Stream ingestion resource (`POST /v2/billing/meter_event_stream`).

use some_stripe::patterns::meter_serializer::MeterBatchSerializer;
use some_stripe::{
    BatchIngestionResponse, CreateMeterEventStreamParams, MeterBatchError, MeterEventBatch,
    MeterEventPayload, MeterPayload, StripeClient, StripeConfig,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Helper to read a complete HTTP request (handling chunked/fragmented TCP reads).
async fn read_full_http_request(socket: &mut tokio::net::TcpStream) -> String {
    let mut buf = Vec::with_capacity(256 * 1024);
    let mut temp = [0u8; 16384];
    let mut content_length = None;
    let mut body_start = None;

    loop {
        let n = socket.read(&mut temp).await.unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&temp[..n]);

        if body_start.is_none() {
            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                body_start = Some(pos + 4);
                let header_str = String::from_utf8_lossy(&buf[..pos]);
                for line in header_str.lines() {
                    let lower = line.to_ascii_lowercase();
                    if lower.starts_with("content-length:") {
                        let val = line["content-length:".len()..].trim();
                        content_length = val.parse::<usize>().ok();
                    }
                }
            }
        }

        if let (Some(start), Some(len)) = (body_start, content_length) {
            if buf.len() >= start + len {
                break;
            }
        }
    }

    String::from_utf8_lossy(&buf).to_string()
}

#[tokio::test]
async fn test_ingest_1000_events_batch_throughput_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let req_str = read_full_http_request(&mut socket).await;

        assert!(req_str.starts_with("POST /v2/billing/meter_event_stream HTTP/1.1"));
        assert!(req_str.contains("content-type: application/json"));
        assert!(req_str.contains("authorization: Basic"));
        assert!(req_str.contains("\"events\":["));
        assert!(req_str.contains("\"event_name\":\"alpaca_ai_tokens\""));
        assert!(req_str.contains("\"stripe_customer_id\":\"cus_batch_999\""));

        let count = req_str
            .matches("\"event_name\":\"alpaca_ai_tokens\"")
            .count();
        assert_eq!(count, 1000, "Must contain exactly 1,000 ingested events");

        let body = r#"{"events_ingested":1000,"status":"success","warnings":[]}"#;
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig {
        secret_key: "sk_test_mock_batch_stream".into(),
        api_base: format!("http://127.0.0.1:{port}"),
        ..Default::default()
    };
    let client = StripeClient::with_config(config);

    // Build 1,000 events
    let events: Vec<MeterEventPayload> = (0..1000)
        .map(|i| {
            MeterEventPayload::new(
                "alpaca_ai_tokens",
                format!("cus_batch_{i}"),
                format!("{}", (i + 1) * 15),
            )
            .with_timestamp(1700000000 + i)
            .with_identifier(format!("req_token_gen_{i}"))
        })
        .collect();

    let params = CreateMeterEventStreamParams::new(events);

    let start = std::time::Instant::now();
    let resp: BatchIngestionResponse = client
        .meter_event_stream()
        .create(&params)
        .await
        .expect("batch ingestion succeeds");
    let elapsed = start.elapsed();

    assert_eq!(resp.events_ingested(), 1000);
    assert_eq!(resp.status(), "success");
    assert!(resp.is_success());
    assert!(resp.warnings().is_none() || resp.warnings().unwrap().is_empty());

    // Sub-second overall turnaround including TCP loopback
    assert!(
        elapsed.as_millis() < 1000,
        "Ingestion took {:?}, expected < 1000ms",
        elapsed
    );

    server.await.unwrap();
}

#[tokio::test]
async fn test_ingest_with_session_bearer_token_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let req_str = read_full_http_request(&mut socket).await;

        assert!(req_str.starts_with("POST /v2/billing/meter_event_stream HTTP/1.1"));
        assert!(req_str.contains("authorization: Bearer sess_tok_alpha_omega_999"));
        assert!(req_str.contains("\"event_name\":\"cloud_compute_seconds\""));
        assert!(req_str.contains("\"stripe_customer_id\":\"cus_sess_123\""));

        let body = r#"{"events_ingested":2,"status":"success","warnings":null}"#;
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig {
        secret_key: "sk_test_not_used_for_bearer".into(),
        api_base: format!("http://127.0.0.1:{port}"),
        ..Default::default()
    };
    let client = StripeClient::with_config(config);

    let events = vec![
        MeterEventPayload::new("cloud_compute_seconds", "cus_sess_123", "3600")
            .with_identifier("id_sess_1"),
        MeterEventPayload::new("cloud_compute_seconds", "cus_sess_123", "1800")
            .with_identifier("id_sess_2"),
    ];
    let params = CreateMeterEventStreamParams::new(events);

    let resp = client
        .meter_event_stream()
        .create_with_session(&params, "sess_tok_alpha_omega_999")
        .await
        .expect("session bearer stream succeeds");

    assert_eq!(resp.events_ingested(), 2);
    assert_eq!(resp.status(), "success");

    server.await.unwrap();
}

#[tokio::test]
async fn test_zero_clone_slice_batch_ingestion() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let req_str = read_full_http_request(&mut socket).await;

        assert!(req_str.starts_with("POST /v2/billing/meter_event_stream HTTP/1.1"));
        assert!(req_str.contains("\"stripe_customer_id\":\"cus_slice_borrow\""));

        let body = r#"{"events_ingested":1,"status":"success"}"#;
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig {
        secret_key: "sk_test_slice".into(),
        api_base: format!("http://127.0.0.1:{port}"),
        ..Default::default()
    };
    let client = StripeClient::with_config(config);

    let events = [MeterEventPayload::new(
        "inference_tokens",
        "cus_slice_borrow",
        "42",
    )];

    // Strict slice borrowing: events is not consumed or cloned
    let resp = client
        .meter_event_stream()
        .create_batch(&events, None)
        .await
        .expect("slice ingestion succeeds");

    assert_eq!(resp.events_ingested(), 1);
    // Verify `events` slice is still accessible and intact (not consumed or moved)
    assert_eq!(events[0].payload.value, "42");

    server.await.unwrap();
}

#[tokio::test]
async fn test_idempotency_identifiers_deduplication_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        // Request 1: Fresh event
        let (mut socket1, _) = listener.accept().await.unwrap();
        let req1 = read_full_http_request(&mut socket1).await;
        assert!(req1.contains("\"identifier\":\"req_idemp_alpha_42\""));

        let body1 = r#"{"events_ingested":1,"status":"success","warnings":[]}"#;
        socket1
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body1}",
                    body1.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();

        // Request 2: Resubmission with same identifier (simulating retry)
        let (mut socket2, _) = listener.accept().await.unwrap();
        let req2 = read_full_http_request(&mut socket2).await;
        assert!(req2.contains("\"identifier\":\"req_idemp_alpha_42\""));

        let body2 = r#"{"events_ingested":0,"status":"success","warnings":["Event deduplicated"]}"#;
        socket2
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body2}",
                    body2.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config = StripeConfig {
        secret_key: "sk_test_idemp".into(),
        api_base: format!("http://127.0.0.1:{port}"),
        ..Default::default()
    };
    let client = StripeClient::with_config(config);

    let event = MeterEventPayload::new("tokens", "cus_retry_1", "500")
        .with_identifier("req_idemp_alpha_42")
        .with_timestamp(1700000000);

    let params = CreateMeterEventStreamParams::single(event);

    // First attempt: ingested
    let resp1 = client.meter_event_stream().create(&params).await.unwrap();
    assert_eq!(resp1.events_ingested(), 1);

    // Second attempt with duplicate identifier: deduplicated
    let resp2 = client.meter_event_stream().create(&params).await.unwrap();
    assert_eq!(resp2.events_ingested(), 0);
    assert!(resp2.warnings().unwrap()[0].contains("deduplicated"));

    server.await.unwrap();
}

#[tokio::test]
async fn test_batch_size_limits_validation() {
    let client = StripeClient::new("sk_test_limits");

    // Empty batch rejection
    let empty_params = CreateMeterEventStreamParams::new(vec![]);
    let err_empty = client.meter_event_stream().create(&empty_params).await;
    assert!(err_empty.is_err());

    // Batch exceeding 1,000 items rejection
    let over_sized: Vec<_> = (0..1001)
        .map(|i| MeterEventPayload::new("tokens", format!("cus_{i}"), "1"))
        .collect();
    let over_params = CreateMeterEventStreamParams::new(over_sized);
    let err_over = client.meter_event_stream().create(&over_params).await;
    assert!(err_over.is_err());
}

#[test]
fn test_meter_batch_error_and_batch_container() {
    assert_eq!(
        MeterEventBatch::new(vec![]),
        Err(MeterBatchError::EmptyBatch)
    );

    let events: Vec<_> = (0..1001)
        .map(|i| MeterEventPayload::new("tokens", format!("cus_{i}"), "5"))
        .collect();
    assert_eq!(
        MeterEventBatch::new(events),
        Err(MeterBatchError::ExceedsMaxBatchSize {
            size: 1001,
            max: 1000,
        })
    );

    let valid_events = vec![MeterEventPayload::new("tokens", "cus_test", "10")];
    let batch = MeterEventBatch::new(valid_events.clone()).unwrap();
    assert_eq!(batch.len(), 1);
    assert!(!batch.is_empty());
    assert_eq!(batch.as_slice(), valid_events.as_slice());
    assert_eq!(batch.into_vec(), valid_events);
}

#[test]
fn test_meter_batch_serializer_performance() {
    let events: Vec<MeterEventPayload> = (0..1000)
        .map(|i| {
            MeterEventPayload::from_payload(
                "token_metric",
                MeterPayload::new(format!("cus_{i}"), format!("{}", i * 5)),
            )
            .with_identifier(format!("idemp_bench_{i}"))
            .with_timestamp(1700000000 + i)
        })
        .collect();

    let start = std::time::Instant::now();
    let json = MeterBatchSerializer::serialize_batch_json(&events).unwrap();
    let duration = start.elapsed();

    // Verify correct structure
    assert!(json.starts_with("{\"events\":["));
    assert!(json.ends_with("]}"));
    assert_eq!(json.matches("\"token_metric\"").count(), 1000);

    // Serialization for 1,000 events must execute under 20ms (>50,000 events/sec)
    assert!(
        duration.as_millis() < 20,
        "Serialization took {:?}, expected < 20ms",
        duration
    );

    // NDJSON test
    let ndjson = MeterBatchSerializer::serialize_ndjson(&events[..10]).unwrap();
    assert_eq!(ndjson.lines().count(), 10);
}
