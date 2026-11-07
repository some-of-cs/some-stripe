//! Comprehensive integration and wire contract tests for Identity Verification Reports
//! and specialized product namespaces (`stripe.tax()` and `stripe.identity()`).

use some_stripe::{
    DocumentDate, EventData, EventObject, IdentityResource, ListVerificationReportsParams,
    StripeClient, StripeConfig, StripeError, TaxResource, VerificationReport,
    VerificationReportStatus, VerificationReportType, verification_report_id,
};
use std::str::FromStr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_verification_report_type_and_status_enums() {
    // Type enum
    let doc_type = VerificationReportType::Document;
    assert_eq!(doc_type.as_str(), "document");
    assert_eq!(doc_type.to_string(), "document");
    assert_eq!(doc_type.as_ref(), "document");
    assert_eq!(
        VerificationReportType::from_str("document").unwrap(),
        VerificationReportType::Document
    );

    let id_num_type = VerificationReportType::IdNumber;
    assert_eq!(id_num_type.as_str(), "id_number");
    assert_eq!(id_num_type.to_string(), "id_number");
    assert_eq!(
        VerificationReportType::from_str("id_number").unwrap(),
        VerificationReportType::IdNumber
    );

    assert!(VerificationReportType::from_str("invalid_type").is_err());

    // Status enum
    let verified = VerificationReportStatus::Verified;
    assert_eq!(verified.as_str(), "verified");
    assert_eq!(verified.to_string(), "verified");
    assert_eq!(verified.as_ref(), "verified");
    assert_eq!(
        VerificationReportStatus::from_str("verified").unwrap(),
        VerificationReportStatus::Verified
    );

    let unverified = VerificationReportStatus::Unverified;
    assert_eq!(unverified.as_str(), "unverified");
    assert_eq!(unverified.to_string(), "unverified");
    assert_eq!(
        VerificationReportStatus::from_str("unverified").unwrap(),
        VerificationReportStatus::Unverified
    );

    assert!(VerificationReportStatus::from_str("unknown_status").is_err());
}

#[test]
fn test_verification_report_deserialization_and_helpers() {
    let json = serde_json::json!({
        "id": "vr_test_doc_999",
        "object": "identity.verification_report",
        "created": 1735689600,
        "livemode": false,
        "type": "document",
        "status": "verified",
        "verification_session": "vs_session_abc",
        "client_reference_id": "cust_ref_42",
        "document": {
            "status": "verified",
            "expiration_date": {
                "day": 12,
                "month": 8,
                "year": 2032
            },
            "files": ["file_front_1", "file_back_2"],
            "first_name": "Alice",
            "last_name": "Smith",
            "dob": {
                "day": 5,
                "month": 4,
                "year": 1991
            },
            "issued_date": {
                "day": 1,
                "month": 1,
                "year": 2022
            },
            "issuing_country": "US",
            "document_type": "passport",
            "number": "A12345678"
        },
        "selfie": {
            "status": "verified",
            "selfie": "file_selfie_1",
            "document": "file_crop_1"
        }
    });

    let report: VerificationReport = serde_json::from_value(json).unwrap();
    assert_eq!(report.id(), "vr_test_doc_999");
    assert_eq!(verification_report_id(&report), "vr_test_doc_999");
    assert!(report.is_verified());
    assert_eq!(report.report_type, VerificationReportType::Document);
    assert_eq!(
        report.verification_session.as_deref(),
        Some("vs_session_abc")
    );
    assert_eq!(report.client_reference_id.as_deref(), Some("cust_ref_42"));

    let doc = report.document.unwrap();
    assert_eq!(doc.status, VerificationReportStatus::Verified);
    assert_eq!(doc.first_name.as_deref(), Some("Alice"));
    assert_eq!(doc.last_name.as_deref(), Some("Smith"));
    assert_eq!(doc.document_type.as_deref(), Some("passport"));
    assert_eq!(doc.issuing_country.as_deref(), Some("US"));
    assert_eq!(doc.number.as_deref(), Some("A12345678"));
    assert_eq!(doc.expiration_date, Some(DocumentDate::new(2032, 8, 12)));
    assert_eq!(doc.dob, Some(DocumentDate::new(1991, 4, 5)));

    let selfie = report.selfie.unwrap();
    assert_eq!(selfie.status, VerificationReportStatus::Verified);
    assert_eq!(selfie.selfie.as_deref(), Some("file_selfie_1"));
    assert_eq!(selfie.document.as_deref(), Some("file_crop_1"));
}

#[test]
fn test_id_number_report_deserialization() {
    let json = serde_json::json!({
        "id": "vr_test_idnum_111",
        "object": "identity.verification_report",
        "created": 1735689600,
        "livemode": false,
        "type": "id_number",
        "status": "verified",
        "id_number": {
            "status": "verified",
            "dob": {
                "day": 15,
                "month": 7,
                "year": 1985
            },
            "first_name": "Bob",
            "last_name": "Johnson",
            "id_number": "*********6789",
            "id_number_type": "us_ssn"
        }
    });

    let report: VerificationReport = serde_json::from_value(json).unwrap();
    assert_eq!(report.id(), "vr_test_idnum_111");
    assert_eq!(report.report_type, VerificationReportType::IdNumber);
    assert!(report.is_verified());

    let id_num = report.id_number.unwrap();
    assert_eq!(id_num.status, VerificationReportStatus::Verified);
    assert_eq!(id_num.first_name.as_deref(), Some("Bob"));
    assert_eq!(id_num.last_name.as_deref(), Some("Johnson"));
    assert_eq!(id_num.id_number.as_deref(), Some("*********6789"));
    assert_eq!(id_num.id_number_type.as_deref(), Some("us_ssn"));
    assert_eq!(id_num.dob, Some(DocumentDate::new(1985, 7, 15)));
}

#[test]
fn test_list_verification_reports_params_builder() {
    let params = ListVerificationReportsParams::new()
        .verification_session("vs_query_test")
        .client_reference_id("ref_client_01")
        .status(VerificationReportStatus::Verified)
        .report_type(VerificationReportType::Document)
        .created(1728345600)
        .starting_after("vr_prev_cursor")
        .ending_before("vr_next_cursor")
        .limit(25);

    assert_eq!(
        params.verification_session.as_deref(),
        Some("vs_query_test")
    );
    assert_eq!(params.client_reference_id.as_deref(), Some("ref_client_01"));
    assert_eq!(params.status, Some(VerificationReportStatus::Verified));
    assert_eq!(params.report_type, Some(VerificationReportType::Document));
    assert_eq!(params.created, Some(1728345600));
    assert_eq!(params.starting_after.as_deref(), Some("vr_prev_cursor"));
    assert_eq!(params.ending_before.as_deref(), Some("vr_next_cursor"));
    assert_eq!(params.limit, Some(25));
}

#[test]
fn test_facade_namespaces_mounting() {
    let client = StripeClient::new("sk_test_mock_secret");

    // Test client.tax() facade and sub-resources
    let tax: TaxResource<'_> = client.tax();
    assert_eq!(tax.client().config.api_base, "https://api.stripe.com");

    let calcs = tax.calculations();
    assert_eq!(calcs.client().config.api_base, "https://api.stripe.com");

    let txs = tax.transactions();
    assert_eq!(txs.client().config.api_base, "https://api.stripe.com");

    let settings = tax.settings();
    assert_eq!(settings.client().config.api_base, "https://api.stripe.com");

    let registrations = tax.registrations();
    assert_eq!(
        registrations.client().config.api_base,
        "https://api.stripe.com"
    );

    // Test client.identity() facade and sub-resources
    let identity: IdentityResource<'_> = client.identity();
    assert_eq!(identity.client().config.api_base, "https://api.stripe.com");

    let sessions = identity.verification_sessions();
    assert_eq!(sessions.client().config.api_base, "https://api.stripe.com");

    let reports = identity.verification_reports();
    assert_eq!(reports.client().config.api_base, "https://api.stripe.com");
}

#[test]
fn test_event_object_verification_report_deserialization() {
    let json = serde_json::json!({
        "id": "vr_event_123",
        "object": "identity.verification_report",
        "created": 1735689600,
        "livemode": false,
        "type": "document",
        "status": "verified"
    });

    let data = EventData { object: json };
    let obj = data
        .deserialize_object()
        .expect("Failed to deserialize identity.verification_report event object");

    match obj {
        EventObject::VerificationReport(report) => {
            assert_eq!(report.id(), "vr_event_123");
            assert_eq!(report.report_type, VerificationReportType::Document);
            assert!(report.is_verified());
        }
        other => panic!("Expected EventObject::VerificationReport, got {other:?}"),
    }
}

#[tokio::test]
async fn test_retrieve_verification_report_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(
            req_str.starts_with("GET /v1/identity/verification_reports/vr_test_mock_123 HTTP/1.1")
        );

        let body = serde_json::json!({
            "id": "vr_test_mock_123",
            "object": "identity.verification_report",
            "created": 1735689600,
            "livemode": false,
            "type": "document",
            "status": "verified",
            "verification_session": "vs_wire_123"
        })
        .to_string();

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{}", port));
    let client = StripeClient::with_config(config);

    let report = client
        .identity()
        .verification_reports()
        .retrieve("vr_test_mock_123")
        .await
        .unwrap();

    assert_eq!(report.id, "vr_test_mock_123");
    assert_eq!(report.status, VerificationReportStatus::Verified);
    assert_eq!(report.verification_session.as_deref(), Some("vs_wire_123"));

    server.await.unwrap();
}

#[tokio::test]
async fn test_retrieve_verification_report_path_traversal_protection() {
    let client = StripeClient::new("sk_test_mock");
    let resource = client.identity().verification_reports();

    // Invalid path characters or traversal
    let err_traversal = resource.retrieve("../vr_secret").await.unwrap_err();
    assert!(matches!(err_traversal, StripeError::InvalidRequest(_)));

    let err_empty = resource.retrieve("").await.unwrap_err();
    assert!(matches!(err_empty, StripeError::InvalidRequest(_)));

    let err_whitespace = resource.retrieve("   ").await.unwrap_err();
    assert!(matches!(err_whitespace, StripeError::InvalidRequest(_)));
}

#[tokio::test]
async fn test_list_verification_reports_wire_paginator() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]);

        assert!(req_str.starts_with("GET /v1/identity/verification_reports"));
        assert!(req_str.contains("verification_session=vs_test_page"));
        assert!(req_str.contains("limit=2"));

        let body = serde_json::json!({
            "object": "list",
            "has_more": false,
            "data": [
                {
                    "id": "vr_stream_01",
                    "object": "identity.verification_report",
                    "created": 1735689600,
                    "livemode": false,
                    "type": "document",
                    "status": "verified"
                },
                {
                    "id": "vr_stream_02",
                    "object": "identity.verification_report",
                    "created": 1735689601,
                    "livemode": false,
                    "type": "id_number",
                    "status": "unverified"
                }
            ]
        })
        .to_string();

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config =
        StripeConfig::new("sk_test_mock").with_api_base(format!("http://127.0.0.1:{}", port));
    let client = StripeClient::with_config(config);

    let params = ListVerificationReportsParams::new()
        .verification_session("vs_test_page")
        .limit(2);

    let mut paginator = client.identity().verification_reports().list(&params);

    let first = paginator.next_item().await.unwrap().expect("First item");
    assert_eq!(first.id, "vr_stream_01");
    assert_eq!(first.report_type, VerificationReportType::Document);
    assert!(first.is_verified());

    let second = paginator.next_item().await.unwrap().expect("Second item");
    assert_eq!(second.id, "vr_stream_02");
    assert_eq!(second.report_type, VerificationReportType::IdNumber);
    assert!(!second.is_verified());

    let third = paginator.next_item().await.unwrap();
    assert!(third.is_none());

    server.await.unwrap();
}
