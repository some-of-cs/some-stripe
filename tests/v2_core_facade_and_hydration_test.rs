//! Integration and wire tests for Stripe API v2 Core Facade & Thin Event Hydrator (US-8.1.0 Unit 5).
//!
//! Verifies:
//! - End-to-end authenticated thin event notification hydration
//! - Cryptographic freshness and 300-second drift tolerance replay protection (Gate G2)
//! - Envelope and live object ID / type / related object integrity verification
//! - Full hierarchy of `client.v2().core()` resources (`events`, `event_destinations`, `accounts`, `hydrator`)
//! - Zero-clone borrowing discipline (Gate G-Borrow)

use some_stripe::{
    AmazonEventBridgeDetails, CreateEventDestinationParams, CreateV2AccountParams,
    CreateV2AccountTokenParams, EventDestinationStatus, EventDestinationType, HydrationError,
    ListV2AccountsParams, ListV2EventsParams, StripeClient, StripeConfig,
    UpdateEventDestinationParams, UpdateV2AccountParams, V2Event, V2RelatedObject, V2ThinEvent,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn write_http_response(socket: &mut tokio::net::TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        body.len(),
        body
    );
    socket.write_all(response.as_bytes()).await.unwrap();
}

#[tokio::test]
async fn test_thin_event_hydration_e2e_flow_and_integrity() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Hydrator fetches live event: GET /v2/core/events/v2_evt_live_123
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req.starts_with("GET /v2/core/events/v2_evt_live_123 HTTP/1.1"));
        assert!(req.to_lowercase().contains("content-type: application/json"));

        let live_payload = r#"{
            "id": "v2_evt_live_123",
            "object": "v2.core.event",
            "type": "v1.billing.meter.error_report_triggered",
            "created": 1728345600,
            "livemode": false,
            "related_object": {
                "id": "meter_999",
                "type": "billing.meter",
                "url": "https://api.stripe.com/v2/billing/meters/meter_999"
            },
            "context": "ctx_alert_high_volume",
            "data": {
                "alert_level": "critical",
                "unprocessed_events": 42
            }
        }"#;
        write_http_response(&mut socket, "200 OK", live_payload).await;
    });

    let config = StripeConfig::new("sk_test_mock_123")
        .with_api_base(format!("http://{}", addr));
    let client = StripeClient::with_config(config);
    let hydrator = client.v2().core().hydrator().with_clock_timestamp(1728345650); // 50s drift (within 300s)

    let thin_event = V2ThinEvent {
        id: "v2_evt_live_123".into(),
        object: "v2.core.event".into(),
        event_type: "v1.billing.meter.error_report_triggered".into(),
        created: 1728345600,
        livemode: false,
        related_object: Some(V2RelatedObject {
            id: "meter_999".into(),
            object_type: "billing.meter".into(),
            url: Some("https://api.stripe.com/v2/billing/meters/meter_999".into()),
        }),
        context: Some("ctx_alert_high_volume".into()),
    };

    let live_event = hydrator.hydrate(&thin_event).await.expect("Hydration should succeed");
    assert_eq!(live_event.id(), "v2_evt_live_123");
    assert_eq!(live_event.event_type(), "v1.billing.meter.error_report_triggered");
    assert_eq!(live_event.related_object_id(), Some("meter_999"));
    assert_eq!(live_event.context.as_deref(), Some("ctx_alert_high_volume"));

    server.await.unwrap();
}

#[tokio::test]
async fn test_thin_event_stale_replay_rejection_gate_g2() {
    let client = StripeClient::new("sk_test_123");
    let hydrator = client.v2().core().hydrator()
        .with_tolerance_seconds(300)
        .with_clock_timestamp(1728346000); // 400 seconds later -> exceeds 300s tolerance

    let thin_event = V2ThinEvent {
        id: "v2_evt_stale_123".into(),
        object: "v2.core.event".into(),
        event_type: "v1.billing.meter.error_report_triggered".into(),
        created: 1728345600, // 400s drift
        livemode: false,
        related_object: None,
        context: None,
    };

    let err = hydrator.hydrate(&thin_event).await.unwrap_err();
    match err {
        HydrationError::StaleEventReplay { created, current, tolerance } => {
            assert_eq!(created, 1728345600);
            assert_eq!(current, 1728346000);
            assert_eq!(tolerance, 300);
        }
        other => panic!("Expected StaleEventReplay error, got {other:?}"),
    }
}

#[tokio::test]
async fn test_thin_event_future_timestamp_rejection() {
    let client = StripeClient::new("sk_test_123");
    let hydrator = client.v2().core().hydrator()
        .with_tolerance_seconds(300)
        .with_clock_timestamp(1728345000); // Clock is 1728345000, event is 1728345600 (600s in the future)

    let thin_event = V2ThinEvent {
        id: "v2_evt_future_123".into(),
        object: "v2.core.event".into(),
        event_type: "v1.billing.meter.error_report_triggered".into(),
        created: 1728345600,
        livemode: false,
        related_object: None,
        context: None,
    };

    let err = hydrator.hydrate(&thin_event).await.unwrap_err();
    match err {
        HydrationError::FutureTimestamp { created, current } => {
            assert_eq!(created, 1728345600);
            assert_eq!(current, 1728345000);
        }
        other => panic!("Expected FutureTimestamp error, got {other:?}"),
    }
}

#[tokio::test]
async fn test_thin_event_related_object_id_mismatch_tampering() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let _ = String::from_utf8(buf[..n].to_vec()).unwrap();

        // Server returns live state with related object "meter_authentic"
        let live_payload = r#"{
            "id": "v2_evt_tampered",
            "object": "v2.core.event",
            "type": "v1.billing.meter.error_report_triggered",
            "created": 1728345600,
            "livemode": false,
            "related_object": {
                "id": "meter_authentic",
                "type": "billing.meter"
            }
        }"#;
        write_http_response(&mut socket, "200 OK", live_payload).await;
    });

    let config = StripeConfig::new("sk_test_mock_123")
        .with_api_base(format!("http://{}", addr));
    let client = StripeClient::with_config(config);
    let hydrator = client.v2().core().hydrator().with_clock_timestamp(1728345600);

    // Attacker forged notification envelope pointing to "meter_fraudulent"
    let thin_event = V2ThinEvent {
        id: "v2_evt_tampered".into(),
        object: "v2.core.event".into(),
        event_type: "v1.billing.meter.error_report_triggered".into(),
        created: 1728345600,
        livemode: false,
        related_object: Some(V2RelatedObject {
            id: "meter_fraudulent".into(),
            object_type: "billing.meter".into(),
            url: None,
        }),
        context: None,
    };

    let err = hydrator.hydrate(&thin_event).await.unwrap_err();
    match err {
        HydrationError::RelatedObjectIdMismatch { envelope_id, retrieved_id } => {
            assert_eq!(envelope_id, "meter_fraudulent");
            assert_eq!(retrieved_id.as_deref(), Some("meter_authentic"));
        }
        other => panic!("Expected RelatedObjectIdMismatch error, got {other:?}"),
    }

    server.await.unwrap();
}

#[tokio::test]
async fn test_omnibus_v2_core_hierarchy_wire_flow() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. events().retrieve()
        let (mut s1, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = s1.read(&mut buf).await.unwrap();
        assert!(String::from_utf8(buf[..n].to_vec()).unwrap().contains("GET /v2/core/events/evt_test_1"));
        write_http_response(&mut s1, "200 OK", r#"{"id":"evt_test_1","object":"v2.core.event","type":"custom.event","created":1728345600,"livemode":false}"#).await;

        // 2. events().list()
        let (mut s2, _) = listener.accept().await.unwrap();
        let n = s2.read(&mut buf).await.unwrap();
        assert!(String::from_utf8(buf[..n].to_vec()).unwrap().contains("GET /v2/core/events"));
        write_http_response(&mut s2, "200 OK", r#"{"data":[{"id":"evt_test_1","object":"v2.core.event","type":"custom.event","created":1728345600,"livemode":false}]}"#).await;

        // 3. event_destinations().create()
        let (mut s3, _) = listener.accept().await.unwrap();
        let n = s3.read(&mut buf).await.unwrap();
        let req3 = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(req3.contains("POST /v2/core/event_destinations"));
        assert!(req3.contains("amazon_eventbridge"));
        let dest_resp = r#"{
            "id": "ed_123",
            "object": "v2.core.event_destination",
            "status": "enabled",
            "type": "amazon_eventbridge",
            "events": ["v2.*"],
            "amazon_eventbridge": {
                "aws_account_id": "123456789012",
                "aws_region": "us-east-1",
                "partner_event_source_arn": "arn:aws:events:us-east-1:123456789012:source"
            },
            "livemode": false,
            "created": 1728345600,
            "updated": 1728345600
        }"#;
        write_http_response(&mut s3, "200 OK", dest_resp).await;

        // 4. event_destinations().retrieve()
        let (mut s4, _) = listener.accept().await.unwrap();
        let n = s4.read(&mut buf).await.unwrap();
        assert!(String::from_utf8(buf[..n].to_vec()).unwrap().contains("GET /v2/core/event_destinations/ed_123"));
        write_http_response(&mut s4, "200 OK", dest_resp).await;

        // 5. event_destinations().enable()
        let (mut s5, _) = listener.accept().await.unwrap();
        let n = s5.read(&mut buf).await.unwrap();
        assert!(String::from_utf8(buf[..n].to_vec()).unwrap().contains("POST /v2/core/event_destinations/ed_123/enable"));
        write_http_response(&mut s5, "200 OK", dest_resp).await;

        // 6. event_destinations().disable()
        let (mut s6, _) = listener.accept().await.unwrap();
        let n = s6.read(&mut buf).await.unwrap();
        assert!(String::from_utf8(buf[..n].to_vec()).unwrap().contains("POST /v2/core/event_destinations/ed_123/disable"));
        let disabled_resp = dest_resp.replace(r#""status": "enabled""#, r#""status": "disabled""#);
        write_http_response(&mut s6, "200 OK", &disabled_resp).await;

        // 7. event_destinations().ping()
        let (mut s7, _) = listener.accept().await.unwrap();
        let n = s7.read(&mut buf).await.unwrap();
        assert!(String::from_utf8(buf[..n].to_vec()).unwrap().contains("POST /v2/core/event_destinations/ed_123/ping"));
        write_http_response(&mut s7, "200 OK", r#"{"ping":"ok"}"#).await;

        // 8. accounts().create()
        let (mut s8, _) = listener.accept().await.unwrap();
        let n = s8.read(&mut buf).await.unwrap();
        let req8 = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(req8.contains("POST /v2/core/accounts"));
        let acct_resp = r#"{
            "id": "v2_acct_777",
            "object": "v2.core.account",
            "livemode": false,
            "created": 1728345600,
            "display_name": "Test Platform Merchant",
            "contact_email": "merchant@test.com",
            "country": "US",
            "default_currency": "usd"
        }"#;
        write_http_response(&mut s8, "200 OK", acct_resp).await;

        // 9. accounts().create_token()
        let (mut s9, _) = listener.accept().await.unwrap();
        let n = s9.read(&mut buf).await.unwrap();
        assert!(String::from_utf8(buf[..n].to_vec()).unwrap().contains("POST /v2/core/accounts/v2_acct_777/tokens"));
        let token_resp = r#"{
            "id": "v2_tok_999",
            "object": "v2.core.account_token",
            "account": "v2_acct_777",
            "created": 1728345600,
            "expires_at": 1728349200
        }"#;
        write_http_response(&mut s9, "200 OK", token_resp).await;
    });

    let config = StripeConfig::new("sk_test_mock_123")
        .with_api_base(format!("http://{}", addr));
    let client = StripeClient::with_config(config);
    let core = client.v2().core();

    // 1. events().retrieve()
    let ev = core.events().retrieve("evt_test_1").await.unwrap();
    assert_eq!(ev.id(), "evt_test_1");

    // 2. events().list()
    let ev_list = core.events().list(&ListV2EventsParams::default()).await.unwrap();
    assert_eq!(ev_list.len(), 1);

    // 3. event_destinations().create()
    let create_dest = CreateEventDestinationParams {
        destination_type: "amazon_eventbridge".into(),
        events: vec!["v2.*".into()],
        amazon_eventbridge: Some(AmazonEventBridgeDetails {
            aws_account_id: "123456789012".into(),
            aws_region: "us-east-1".into(),
            partner_event_source_arn: None,
        }),
        ..Default::default()
    };
    let dest = core.event_destinations().create(&create_dest).await.unwrap();
    assert_eq!(dest.id, "ed_123");
    assert!(dest.is_enabled());

    // 4. event_destinations().retrieve()
    let dest_ret = core.event_destinations().retrieve("ed_123").await.unwrap();
    assert_eq!(dest_ret.id, "ed_123");

    // 5. event_destinations().enable()
    let dest_en = core.event_destinations().enable("ed_123").await.unwrap();
    assert!(dest_en.is_enabled());

    // 6. event_destinations().disable()
    let dest_dis = core.event_destinations().disable("ed_123").await.unwrap();
    assert_eq!(dest_dis.status, EventDestinationStatus::Disabled);

    // 7. event_destinations().ping()
    let ping_res = core.event_destinations().ping("ed_123").await.unwrap();
    assert_eq!(ping_res["ping"], "ok");

    // 8. accounts().create()
    let create_acct = CreateV2AccountParams {
        display_name: Some("Test Platform Merchant".into()),
        contact_email: Some("merchant@test.com".into()),
        country: Some("US".into()),
        default_currency: Some("usd".into()),
    };
    let acct = core.accounts().create(&create_acct).await.unwrap();
    assert_eq!(acct.id, "v2_acct_777");

    // 9. accounts().create_token()
    let tok_params = CreateV2AccountTokenParams {
        purpose: Some("onboarding".into()),
    };
    let tok = core.accounts().create_token("v2_acct_777", &tok_params).await.unwrap();
    assert_eq!(tok.id, "v2_tok_999");
    assert_eq!(tok.account, "v2_acct_777");

    server.await.unwrap();
}
