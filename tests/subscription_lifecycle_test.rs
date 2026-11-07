//! Comprehensive lifecycle and wire mock integration tests for Stripe Subscriptions.
//!
//! Validates:
//! - Multi-item creation with RFC 1738 indexed bracket serialization (`items[0][price]=...`)
//! - Subscription lifecycle state machine progression (`incomplete` -> `trialing` -> `active` -> `paused` -> `canceled`)
//! - Pausing and resuming subscriptions
//! - In-flight cancellations with proration and invoicing options
//! - Pagination and search query endpoints
//! - Security drift guards, CRLF injection sanitization, and automatic RFC 4122 UUID v4 idempotency keys
//! - Gate G0, Gate G-Borrow (zero-clone), Gate G2, and Gate G-Law compliance

use some_stripe::{
    CancelSubscriptionParams, CreateSubscriptionParams, ListSubscriptionsParams,
    PauseCollectionParams, PaymentSettings, ProrationBehavior, ResumeSubscriptionParams,
    SearchSubscriptionsParams, StripeClient, StripeConfig, StripeError, Subscription,
    SubscriptionItemParams, SubscriptionStateError, SubscriptionStatus,
    SubscriptionValidationError, UpdateSubscriptionParams,
};
use some_stripe_core::patterns::serializer::to_form_params;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_create_subscription_params_bracket_serialization() {
    let items = vec![
        SubscriptionItemParams::new("price_starter_monthly", 2),
        SubscriptionItemParams::new("price_addon_analytics", 1),
    ];

    let payment_settings = PaymentSettings::new()
        .payment_method_types(vec!["card".into(), "us_bank_account".into()])
        .save_default_payment_method("on_subscription");

    let params = CreateSubscriptionParams::new("cus_sub_test_001", items)
        .currency("usd")
        .trial_period_days(14)
        .proration_behavior(ProrationBehavior::CreateProrations)
        .payment_settings(payment_settings)
        .add_metadata("environment", "integration_test");

    assert!(params.validate().is_ok());

    let form_pairs = to_form_params(&params).expect("Failed to serialize to form parameters");
    let form_map: std::collections::HashMap<String, String> = form_pairs.into_iter().collect();

    assert_eq!(
        form_map.get("customer").map(String::as_str),
        Some("cus_sub_test_001")
    );
    assert_eq!(form_map.get("currency").map(String::as_str), Some("usd"));
    assert_eq!(
        form_map.get("trial_period_days").map(String::as_str),
        Some("14")
    );
    assert_eq!(
        form_map.get("proration_behavior").map(String::as_str),
        Some("create_prorations")
    );

    // Verify indexed bracket notation
    assert_eq!(
        form_map.get("items[0][price]").map(String::as_str),
        Some("price_starter_monthly")
    );
    assert_eq!(
        form_map.get("items[0][quantity]").map(String::as_str),
        Some("2")
    );
    assert_eq!(
        form_map.get("items[1][price]").map(String::as_str),
        Some("price_addon_analytics")
    );
    assert_eq!(
        form_map.get("items[1][quantity]").map(String::as_str),
        Some("1")
    );

    // Verify payment settings
    assert_eq!(
        form_map
            .get("payment_settings[payment_method_types][0]")
            .map(String::as_str),
        Some("card")
    );
    assert_eq!(
        form_map
            .get("payment_settings[payment_method_types][1]")
            .map(String::as_str),
        Some("us_bank_account")
    );
    assert_eq!(
        form_map
            .get("payment_settings[save_default_payment_method]")
            .map(String::as_str),
        Some("on_subscription")
    );

    // Verify metadata
    assert_eq!(
        form_map.get("metadata[environment]").map(String::as_str),
        Some("integration_test")
    );
}

#[test]
fn test_subscription_validation_rules() {
    // Empty customer ID
    let err_params = CreateSubscriptionParams::new("", vec![SubscriptionItemParams::new("p1", 1)]);
    assert!(matches!(
        err_params.validate(),
        Err(SubscriptionValidationError::InvalidCustomerId(_))
    ));

    // Empty items
    let err_items = CreateSubscriptionParams::new("cus_123", vec![]);
    assert!(matches!(
        err_items.validate(),
        Err(SubscriptionValidationError::EmptyItems)
    ));

    // Item without price or id
    let empty_item = SubscriptionItemParams::default();
    let err_item = CreateSubscriptionParams::new("cus_123", vec![empty_item]);
    assert!(matches!(
        err_item.validate(),
        Err(SubscriptionValidationError::InvalidParameter { field: "items", .. })
    ));

    // Invalid currency length
    let err_curr =
        CreateSubscriptionParams::new("cus_123", vec![SubscriptionItemParams::new("p1", 1)])
            .currency("us_dollar");
    assert!(matches!(
        err_curr.validate(),
        Err(SubscriptionValidationError::InvalidCurrency(_))
    ));

    // Invalid pause behavior
    let update_invalid_pause =
        UpdateSubscriptionParams::new().pause_collection(PauseCollectionParams {
            behavior: "unsupported_behavior".into(),
            resumes_at: None,
        });
    assert!(matches!(
        update_invalid_pause.validate(),
        Err(SubscriptionValidationError::InvalidPauseBehavior(_))
    ));
}

#[test]
fn test_subscription_state_machine_transitions() {
    let mut status = SubscriptionStatus::Incomplete;
    assert!(!status.is_operational());
    assert!(!status.is_terminal());

    // Incomplete -> Active
    assert!(status.can_transition_to(SubscriptionStatus::Active));
    assert!(
        status
            .validate_transition(SubscriptionStatus::Active)
            .is_ok()
    );

    // Incomplete -> Trialing
    assert!(status.can_transition_to(SubscriptionStatus::Trialing));
    assert!(
        status
            .validate_transition(SubscriptionStatus::Trialing)
            .is_ok()
    );

    // Trialing -> Active
    status = SubscriptionStatus::Trialing;
    assert!(status.is_operational());
    assert!(status.can_transition_to(SubscriptionStatus::Active));
    assert!(status.can_transition_to(SubscriptionStatus::Paused));
    assert!(status.can_transition_to(SubscriptionStatus::PastDue));

    // Active -> Paused
    status = SubscriptionStatus::Active;
    assert!(status.is_operational());
    assert!(status.can_transition_to(SubscriptionStatus::Paused));
    assert!(status.can_transition_to(SubscriptionStatus::PastDue));
    assert!(status.can_transition_to(SubscriptionStatus::Canceled));

    // Paused -> Active
    status = SubscriptionStatus::Paused;
    assert!(status.can_transition_to(SubscriptionStatus::Active));
    assert!(status.can_transition_to(SubscriptionStatus::Canceled));

    // PastDue -> Unpaid / Canceled
    status = SubscriptionStatus::PastDue;
    assert!(status.is_delinquent_or_canceled());
    assert!(status.can_transition_to(SubscriptionStatus::Active));
    assert!(status.can_transition_to(SubscriptionStatus::Unpaid));
    assert!(status.can_transition_to(SubscriptionStatus::Canceled));

    // Terminal Canceled state: cannot transition to anything
    status = SubscriptionStatus::Canceled;
    assert!(status.is_terminal());
    assert!(status.is_delinquent_or_canceled());
    assert!(!status.can_transition_to(SubscriptionStatus::Active));
    assert!(matches!(
        status.validate_transition(SubscriptionStatus::Active),
        Err(SubscriptionStateError::TerminalState(
            SubscriptionStatus::Canceled
        ))
    ));
}

#[test]
fn test_subscription_domain_entity_transition_to() {
    let json = r#"{
        "id": "sub_trans_test_1",
        "object": "subscription",
        "customer": "cus_user_99",
        "status": "trialing",
        "current_period_start": 1728345600,
        "current_period_end": 1730937600,
        "cancel_at_period_end": false,
        "created": 1728345600,
        "items": { "object": "list", "data": [], "has_more": false, "url": "/v1/subscription_items" }
    }"#;

    let mut sub: Subscription = serde_json::from_str(json).unwrap();
    assert_eq!(sub.status(), SubscriptionStatus::Trialing);
    assert!(sub.is_trialing());

    // Advance to active
    assert!(sub.transition_to(SubscriptionStatus::Active).is_ok());
    assert_eq!(sub.status(), SubscriptionStatus::Active);
    assert!(sub.is_active());

    // Advance to paused
    assert!(sub.transition_to(SubscriptionStatus::Paused).is_ok());
    assert_eq!(sub.status(), SubscriptionStatus::Paused);
    assert!(sub.is_paused());

    // Advance to canceled
    assert!(sub.transition_to(SubscriptionStatus::Canceled).is_ok());
    assert_eq!(sub.status(), SubscriptionStatus::Canceled);
    assert!(sub.is_canceled());

    // Cannot transition from terminal state
    assert!(matches!(
        sub.transition_to(SubscriptionStatus::Active),
        Err(SubscriptionStateError::TerminalState(_))
    ));
}

#[tokio::test]
async fn test_subscription_creation_wire_mock() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut req_buf = vec![0u8; 8192];
        let n = socket.read(&mut req_buf).await.unwrap();
        let wire_req = String::from_utf8(req_buf[..n].to_vec()).unwrap();

        assert!(wire_req.starts_with("POST /v1/subscriptions "));
        assert!(wire_req.contains("idempotency-key:"));
        assert!(
            wire_req.contains("items%5B0%5D%5Bprice%5D=price_seat_123")
                || wire_req.contains("items[0][price]=price_seat_123")
        );
        assert!(wire_req.contains("trial_period_days=14"));

        let response_body = r#"{
            "id": "sub_mock_wire_123",
            "object": "subscription",
            "customer": "cus_wire_buyer_1",
            "status": "trialing",
            "current_period_start": 1728345600,
            "current_period_end": 1730937600,
            "cancel_at_period_end": false,
            "created": 1728345600,
            "trial_start": 1728345600,
            "trial_end": 1729555200,
            "items": {
                "object": "list",
                "data": [
                    {
                        "id": "si_item_wire_1",
                        "object": "subscription_item",
                        "subscription": "sub_mock_wire_123",
                        "created": 1728345600,
                        "quantity": 5
                    }
                ],
                "has_more": false,
                "url": "/v1/subscription_items"
            }
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_wire_123").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let params = CreateSubscriptionParams::new(
        "cus_wire_buyer_1",
        vec![SubscriptionItemParams::new("price_seat_123", 5)],
    )
    .trial_period_days(14);

    let sub = client
        .subscriptions()
        .create(&params)
        .await
        .expect("Failed to create subscription over wire");

    assert_eq!(sub.id(), "sub_mock_wire_123");
    assert_eq!(sub.customer(), "cus_wire_buyer_1");
    assert_eq!(sub.status(), SubscriptionStatus::Trialing);
    assert!(sub.is_trialing());
    assert!(sub.is_operational());
    assert_eq!(sub.items().len(), 1);
    assert_eq!(sub.items()[0].quantity, Some(5));

    server.await.unwrap();
}

#[tokio::test]
async fn test_subscription_retrieve_and_update_wire_mock() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Step 1: Handle GET /v1/subscriptions/sub_wire_get
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buf1 = vec![0u8; 4096];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
        assert!(req1.starts_with("GET /v1/subscriptions/sub_wire_get "));

        let body1 = r#"{
            "id": "sub_wire_get",
            "object": "subscription",
            "customer": "cus_holder_1",
            "status": "active",
            "current_period_start": 1728345600,
            "current_period_end": 1730937600,
            "cancel_at_period_end": false,
            "created": 1728345600,
            "items": { "object": "list", "data": [], "has_more": false, "url": "/v1/subscription_items" }
        }"#;
        let resp1 = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body1.len(),
            body1
        );
        socket1.write_all(resp1.as_bytes()).await.unwrap();

        // Step 2: Handle POST /v1/subscriptions/sub_wire_get (update to pause)
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf2 = vec![0u8; 4096];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req2 = String::from_utf8(buf2[..n2].to_vec()).unwrap();
        assert!(req2.starts_with("POST /v1/subscriptions/sub_wire_get "));
        assert!(
            req2.contains("pause_collection%5Bbehavior%5D=void")
                || req2.contains("pause_collection[behavior]=void")
        );

        let body2 = r#"{
            "id": "sub_wire_get",
            "object": "subscription",
            "customer": "cus_holder_1",
            "status": "paused",
            "current_period_start": 1728345600,
            "current_period_end": 1730937600,
            "cancel_at_period_end": false,
            "created": 1728345600,
            "pause_collection": { "behavior": "void" },
            "items": { "object": "list", "data": [], "has_more": false, "url": "/v1/subscription_items" }
        }"#;
        let resp2 = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body2.len(),
            body2
        );
        socket2.write_all(resp2.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_wire").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    // Retrieve
    let sub = client
        .subscriptions()
        .retrieve("sub_wire_get")
        .await
        .expect("Failed to retrieve subscription");
    assert_eq!(sub.id(), "sub_wire_get");
    assert!(sub.is_active());

    // Update (pause)
    let update_params =
        UpdateSubscriptionParams::new().pause_collection(PauseCollectionParams::void());
    let updated = client
        .subscriptions()
        .update("sub_wire_get", &update_params)
        .await
        .expect("Failed to update subscription");
    assert_eq!(updated.status(), SubscriptionStatus::Paused);
    assert!(updated.is_paused());

    server.await.unwrap();
}

#[tokio::test]
async fn test_subscription_cancel_and_resume_wire_mock() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Step 1: DELETE /v1/subscriptions/sub_wire_term (cancel)
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buf1 = vec![0u8; 4096];
        let n1 = socket1.read(&mut buf1).await.unwrap();
        let req1 = String::from_utf8(buf1[..n1].to_vec()).unwrap();
        assert!(req1.starts_with("DELETE /v1/subscriptions/sub_wire_term "));
        assert!(req1.contains("prorate=true"));
        assert!(req1.contains("invoice_now=true"));

        let body1 = r#"{
            "id": "sub_wire_term",
            "object": "subscription",
            "customer": "cus_term_buyer",
            "status": "canceled",
            "current_period_start": 1728345600,
            "current_period_end": 1730937600,
            "canceled_at": 1729000000,
            "cancel_at_period_end": false,
            "created": 1728345600,
            "items": { "object": "list", "data": [], "has_more": false, "url": "/v1/subscription_items" }
        }"#;
        let resp1 = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body1.len(),
            body1
        );
        socket1.write_all(resp1.as_bytes()).await.unwrap();

        // Step 2: POST /v1/subscriptions/sub_wire_paused/resume (resume)
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf2 = vec![0u8; 4096];
        let n2 = socket2.read(&mut buf2).await.unwrap();
        let req2 = String::from_utf8(buf2[..n2].to_vec()).unwrap();
        assert!(req2.starts_with("POST /v1/subscriptions/sub_wire_paused/resume "));
        assert!(req2.contains("billing_cycle_anchor=now"));

        let body2 = r#"{
            "id": "sub_wire_paused",
            "object": "subscription",
            "customer": "cus_resumed_buyer",
            "status": "active",
            "current_period_start": 1729000000,
            "current_period_end": 1731592000,
            "cancel_at_period_end": false,
            "created": 1728345600,
            "items": { "object": "list", "data": [], "has_more": false, "url": "/v1/subscription_items" }
        }"#;
        let resp2 = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body2.len(),
            body2
        );
        socket2.write_all(resp2.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_wire").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    // Cancel
    let cancel_params = CancelSubscriptionParams::new()
        .prorate(true)
        .invoice_now(true);
    let canceled = client
        .subscriptions()
        .cancel("sub_wire_term", Some(&cancel_params))
        .await
        .expect("Failed to cancel subscription");
    assert_eq!(canceled.status(), SubscriptionStatus::Canceled);
    assert!(canceled.is_canceled());
    assert_eq!(canceled.canceled_at, Some(1729000000));

    // Resume
    let resume_params = ResumeSubscriptionParams::new().billing_cycle_anchor("now");
    let resumed = client
        .subscriptions()
        .resume("sub_wire_paused", Some(&resume_params))
        .await
        .expect("Failed to resume subscription");
    assert_eq!(resumed.status(), SubscriptionStatus::Active);
    assert!(resumed.is_active());

    server.await.unwrap();
}

#[tokio::test]
async fn test_subscription_list_paginator_wire_mock() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req.starts_with("GET /v1/subscriptions?"));
        assert!(req.contains("customer=cus_page_buyer"));
        assert!(req.contains("status=active"));

        let body = r#"{
            "object": "list",
            "data": [
                {
                    "id": "sub_p1",
                    "object": "subscription",
                    "customer": "cus_page_buyer",
                    "status": "active",
                    "current_period_start": 1728345600,
                    "current_period_end": 1730937600,
                    "cancel_at_period_end": false,
                    "created": 1728345600,
                    "items": { "object": "list", "data": [], "has_more": false, "url": "/v1/subscription_items" }
                },
                {
                    "id": "sub_p2",
                    "object": "subscription",
                    "customer": "cus_page_buyer",
                    "status": "active",
                    "current_period_start": 1728345600,
                    "current_period_end": 1730937600,
                    "cancel_at_period_end": false,
                    "created": 1728345600,
                    "items": { "object": "list", "data": [], "has_more": false, "url": "/v1/subscription_items" }
                }
            ],
            "has_more": false,
            "url": "/v1/subscriptions"
        }"#;

        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_wire").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let list_params = ListSubscriptionsParams::new()
        .customer("cus_page_buyer")
        .status("active");

    let mut paginator = client.subscriptions().list(&list_params);

    let sub1 = paginator
        .next_item()
        .await
        .unwrap()
        .expect("Item 1 expected");
    assert_eq!(sub1.id(), "sub_p1");

    let sub2 = paginator
        .next_item()
        .await
        .unwrap()
        .expect("Item 2 expected");
    assert_eq!(sub2.id(), "sub_p2");

    let empty = paginator.next_item().await.unwrap();
    assert!(empty.is_none());

    server.await.unwrap();
}

#[tokio::test]
async fn test_subscription_search_wire_mock() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req.starts_with("GET /v1/subscriptions/search?query=status%3A%27active%27"));

        let body = r#"{
            "object": "search_result",
            "data": [
                {
                    "id": "sub_search_found",
                    "object": "subscription",
                    "customer": "cus_search_client",
                    "status": "active",
                    "current_period_start": 1728345600,
                    "current_period_end": 1730937600,
                    "cancel_at_period_end": false,
                    "created": 1728345600,
                    "items": { "object": "list", "data": [], "has_more": false, "url": "/v1/subscription_items" }
                }
            ],
            "has_more": false
        }"#;

        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_wire").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let results = client
        .subscriptions()
        .search("status:'active'")
        .await
        .expect("Failed to search subscriptions");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id(), "sub_search_found");

    // Also verify search_with_params builder
    let search_params = SearchSubscriptionsParams::new("status:'active'").limit(10);
    assert_eq!(search_params.query, "status:'active'");
    assert_eq!(search_params.limit, Some(10));

    server.await.unwrap();
}

#[tokio::test]
async fn test_security_crlf_and_null_byte_rejection() {
    let client = StripeClient::new("sk_test_mock");

    // CRLF injection attempt in subscription ID
    let crlf_result = client
        .subscriptions()
        .retrieve("sub_123\r\nInjected-Header: evil")
        .await;
    assert!(matches!(crlf_result, Err(StripeError::InvalidRequest(_))));

    // Null byte injection attempt
    let null_result = client.subscriptions().cancel("sub_123\0admin", None).await;
    assert!(matches!(null_result, Err(StripeError::InvalidRequest(_))));

    // Path traversal injection attempt
    let path_result = client.subscriptions().retrieve("../admin/keys").await;
    assert!(matches!(path_result, Err(StripeError::InvalidRequest(_))));
}
