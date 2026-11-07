//! Integration and wire mock tests for SubscriptionSchedulesResource (US-5.1.0 Unit 4).
//!
//! Validates:
//! - Multi-phase schedule creation, trial transitions, and automatic/custom Idempotency-Key injection.
//! - Bracket notation form serialization for nested phases (`phases[0][items][0][price]=...`).
//! - Schedule retrieval without injecting idempotency keys and path traversal protection.
//! - Schedule phase updates and transition parameter mutations.
//! - Schedule release endpoint (`/v1/subscription_schedules/{id}/release`) with `preserve_cancel_date`.
//! - Schedule cancellation endpoint (`/v1/subscription_schedules/{id}/cancel`) with `invoice_now` and `prorate`.
//! - Zero-clone cursor-driven pagination across multiple schedule pages.
//! - Gate G-Borrow zero-clone borrowing discipline and Gate G-Law statutory validations.

use some_stripe::{
    CancelSubscriptionScheduleParams, CreateSubscriptionScheduleParams,
    CreateSubscriptionSchedulePhaseParams, ListSubscriptionSchedulesParams, ProrationBehavior,
    ReleaseSubscriptionScheduleParams, StripeClient, StripeConfig, SubscriptionSchedule,
    SubscriptionScheduleEndBehavior, SubscriptionSchedulePhaseItemParams,
    SubscriptionScheduleStatus, SubscriptionScheduleValidationError,
    UpdateSubscriptionScheduleParams, is_valid_uuid_v4,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_subscription_schedule_create_multi_phase_and_idempotency() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Request 1: Create multi-phase schedule with auto-injected UUID v4 idempotency key
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/subscription_schedules HTTP/1.1"));
            let header = req_str
                .lines()
                .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
                .expect("Idempotency-Key must be injected on create");
            let key = header.split(':').nth(1).unwrap().trim();
            assert!(
                is_valid_uuid_v4(key),
                "Auto-injected key must be RFC 4122 UUID v4: {key}"
            );

            // Verify form body bracket notation encoding
            assert!(
                req_str.contains("customer=cus_vault_test_001"),
                "Request must contain customer ID"
            );
            assert!(
                req_str.contains("phases%5B0%5D%5Bitems%5D%5B0%5D%5Bprice%5D=price_trial_free")
                    || req_str.contains("phases[0][items][0][price]=price_trial_free"),
                "Phase 0 item price must be bracket-encoded"
            );
            assert!(
                req_str.contains("phases%5B0%5D%5Biterations%5D=1")
                    || req_str.contains("phases[0][iterations]=1"),
                "Phase 0 iterations must be encoded"
            );
            assert!(
                req_str
                    .contains("phases%5B1%5D%5Bitems%5D%5B0%5D%5Bprice%5D=price_premium_monthly")
                    || req_str.contains("phases[1][items][0][price]=price_premium_monthly"),
                "Phase 1 item price must be bracket-encoded"
            );
            assert!(
                req_str.contains("phases%5B1%5D%5Bitems%5D%5B0%5D%5Bquantity%5D=2")
                    || req_str.contains("phases[1][items][0][quantity]=2"),
                "Phase 1 item quantity must be bracket-encoded"
            );
            assert!(
                req_str.contains("end_behavior=release"),
                "End behavior must be serialized"
            );

            let resp = serde_json::json!({
                "id": "sub_sched_create_001",
                "object": "subscription_schedule",
                "customer": "cus_vault_test_001",
                "status": "active",
                "created": 1728345600,
                "end_behavior": "release",
                "phases": [
                    {
                        "start_date": 1728345600,
                        "end_date": 1729555200,
                        "iterations": 1,
                        "proration_behavior": "none",
                        "items": [
                            {
                                "price": "price_trial_free",
                                "quantity": 1
                            }
                        ]
                    },
                    {
                        "start_date": 1729555200,
                        "end_date": 1732147200,
                        "iterations": 12,
                        "proration_behavior": "create_prorations",
                        "items": [
                            {
                                "price": "price_premium_monthly",
                                "quantity": 2
                            }
                        ]
                    }
                ],
                "current_phase": {
                    "start_date": 1728345600,
                    "end_date": 1729555200
                }
            })
            .to_string();

            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                        resp.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }

        // Request 2: Create with explicit user-supplied idempotency key
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/subscription_schedules HTTP/1.1"));
            assert!(
                req_str.contains("idempotency-key: custom_sched_idem_abc"),
                "User-supplied Idempotency-Key must be preserved"
            );

            let resp = serde_json::json!({
                "id": "sub_sched_create_002",
                "object": "subscription_schedule",
                "customer": "cus_vault_test_001",
                "status": "not_started",
                "created": 1728345600
            })
            .to_string();

            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                        resp.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
    });

    let config =
        StripeConfig::new("sk_test_sched_create").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // Call 1: Multi-phase schedule creation
    let phase1_item = SubscriptionSchedulePhaseItemParams::new("price_trial_free").quantity(1);
    let phase1 = CreateSubscriptionSchedulePhaseParams::new(vec![phase1_item])
        .iterations(1)
        .proration_behavior(ProrationBehavior::None);

    let phase2_item = SubscriptionSchedulePhaseItemParams::new("price_premium_monthly").quantity(2);
    let phase2 = CreateSubscriptionSchedulePhaseParams::new(vec![phase2_item])
        .iterations(12)
        .proration_behavior(ProrationBehavior::CreateProrations);

    let params = CreateSubscriptionScheduleParams::new()
        .customer("cus_vault_test_001")
        .start_date_now()
        .phases(vec![phase1, phase2])
        .end_behavior(SubscriptionScheduleEndBehavior::Release);

    let sched = client
        .subscription_schedules()
        .create(&params)
        .await
        .unwrap();
    assert_eq!(sched.id(), "sub_sched_create_001");
    assert_eq!(sched.customer(), "cus_vault_test_001");
    assert_eq!(sched.status(), SubscriptionScheduleStatus::Active);
    assert!(sched.is_active());
    assert_eq!(sched.phases().len(), 2);
    assert_eq!(sched.phases()[0].items()[0].price(), "price_trial_free");
    assert_eq!(sched.phases()[1].items()[0].quantity(), Some(2));
    assert!(sched.current_phase().is_some());
    assert_eq!(sched.current_phase().unwrap().start_date(), 1728345600);
    assert_eq!(sched.current_phase().unwrap().duration_seconds(), 1209600);

    // Call 2: Custom idempotency key preservation
    let params2 = CreateSubscriptionScheduleParams::new()
        .customer("cus_vault_test_001")
        .idempotency_key("custom_sched_idem_abc");

    let sched2 = client
        .subscription_schedules()
        .create(&params2)
        .await
        .unwrap();
    assert_eq!(sched2.id(), "sub_sched_create_002");
    assert_eq!(sched2.status(), SubscriptionScheduleStatus::NotStarted);

    server.await.unwrap();
}

#[tokio::test]
async fn test_subscription_schedule_retrieve_no_idempotency_and_path_security() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(
            req_str.starts_with("GET /v1/subscription_schedules/sub_sched_retrieve_100 HTTP/1.1")
        );
        assert!(
            !req_str.to_lowercase().contains("idempotency-key:"),
            "Idempotency-Key must NOT be injected on GET retrieve"
        );

        let resp = serde_json::json!({
            "id": "sub_sched_retrieve_100",
            "object": "subscription_schedule",
            "customer": "cus_vault_retrieve_100",
            "status": "active",
            "created": 1728345600,
            "subscription": "sub_active_100"
        })
        .to_string();

        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                    resp.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config =
        StripeConfig::new("sk_test_sched_create").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // Valid retrieval
    let sched = client
        .subscription_schedules()
        .retrieve("sub_sched_retrieve_100")
        .await
        .unwrap();
    assert_eq!(sched.id(), "sub_sched_retrieve_100");
    assert_eq!(sched.customer(), "cus_vault_retrieve_100");
    assert_eq!(sched.subscription_id(), Some("sub_active_100"));

    // Path traversal rejection (Gate G2)
    assert!(
        client
            .subscription_schedules()
            .retrieve("../evil/traversal")
            .await
            .is_err()
    );
    assert!(
        client
            .subscription_schedules()
            .retrieve("sched\r\ninjection")
            .await
            .is_err()
    );
    assert!(client.subscription_schedules().retrieve("").await.is_err());

    server.await.unwrap();
}

#[tokio::test]
async fn test_subscription_schedule_update_and_idempotency() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("POST /v1/subscription_schedules/sub_sched_upd_001 HTTP/1.1"));
        let header = req_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
            .expect("Idempotency-Key must be injected on update");
        assert!(is_valid_uuid_v4(header.split(':').nth(1).unwrap().trim()));

        assert!(
            req_str.contains("end_behavior=cancel"),
            "Updated end_behavior must be serialized"
        );
        assert!(
            req_str.contains("proration_behavior=always_invoice"),
            "Proration behavior must be serialized"
        );

        let resp = serde_json::json!({
            "id": "sub_sched_upd_001",
            "object": "subscription_schedule",
            "customer": "cus_123",
            "status": "active",
            "created": 1728345600,
            "end_behavior": "cancel"
        })
        .to_string();

        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                    resp.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config =
        StripeConfig::new("sk_test_sched_create").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let params = UpdateSubscriptionScheduleParams::new()
        .end_behavior(SubscriptionScheduleEndBehavior::Cancel)
        .proration_behavior(ProrationBehavior::AlwaysInvoice);

    let sched = client
        .subscription_schedules()
        .update("sub_sched_upd_001", &params)
        .await
        .unwrap();

    assert_eq!(sched.id(), "sub_sched_upd_001");
    assert_eq!(sched.status(), SubscriptionScheduleStatus::Active);

    server.await.unwrap();
}

#[tokio::test]
async fn test_subscription_schedule_release_endpoint() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(
            req_str
                .starts_with("POST /v1/subscription_schedules/sub_sched_rel_777/release HTTP/1.1")
        );
        let header = req_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
            .expect("Idempotency-Key must be injected on release");
        assert!(is_valid_uuid_v4(header.split(':').nth(1).unwrap().trim()));

        assert!(
            req_str.contains("preserve_cancel_date=true"),
            "preserve_cancel_date parameter must be serialized"
        );

        let resp = serde_json::json!({
            "id": "sub_sched_rel_777",
            "object": "subscription_schedule",
            "customer": "cus_released_customer",
            "status": "released",
            "created": 1728345600,
            "released_at": 1728950400,
            "released_subscription": "sub_standalone_777"
        })
        .to_string();

        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                    resp.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config =
        StripeConfig::new("sk_test_sched_create").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let params = ReleaseSubscriptionScheduleParams::new().preserve_cancel_date(true);
    let sched = client
        .subscription_schedules()
        .release("sub_sched_rel_777", &params)
        .await
        .unwrap();

    assert_eq!(sched.id(), "sub_sched_rel_777");
    assert_eq!(sched.status(), SubscriptionScheduleStatus::Released);
    assert!(sched.status().is_terminal());
    assert_eq!(sched.released_subscription_id(), Some("sub_standalone_777"));

    server.await.unwrap();
}

#[tokio::test]
async fn test_subscription_schedule_cancel_endpoint() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(
            req_str
                .starts_with("POST /v1/subscription_schedules/sub_sched_can_888/cancel HTTP/1.1")
        );
        let header = req_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
            .expect("Idempotency-Key must be injected on cancel");
        assert!(is_valid_uuid_v4(header.split(':').nth(1).unwrap().trim()));

        assert!(
            req_str.contains("invoice_now=true"),
            "invoice_now must be serialized"
        );
        assert!(
            req_str.contains("prorate=true"),
            "prorate must be serialized"
        );

        let resp = serde_json::json!({
            "id": "sub_sched_can_888",
            "object": "subscription_schedule",
            "customer": "cus_canceled_customer",
            "status": "canceled",
            "created": 1728345600,
            "canceled_at": 1728950400
        })
        .to_string();

        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                    resp.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });

    let config =
        StripeConfig::new("sk_test_sched_create").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let params = CancelSubscriptionScheduleParams::new()
        .invoice_now(true)
        .prorate(true);

    let sched = client
        .subscription_schedules()
        .cancel("sub_sched_can_888", &params)
        .await
        .unwrap();

    assert_eq!(sched.id(), "sub_sched_can_888");
    assert_eq!(sched.status(), SubscriptionScheduleStatus::Canceled);
    assert!(sched.status().is_terminal());

    server.await.unwrap();
}

#[tokio::test]
async fn test_subscription_schedule_pagination_stream() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Page 1
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("GET /v1/subscription_schedules?limit=2 HTTP/1.1"));

            let resp = serde_json::json!({
                "object": "list",
                "data": [
                    {
                        "id": "sub_sched_page_001",
                        "object": "subscription_schedule",
                        "customer": "cus_001",
                        "status": "active",
                        "created": 1728345600
                    },
                    {
                        "id": "sub_sched_page_002",
                        "object": "subscription_schedule",
                        "customer": "cus_002",
                        "status": "completed",
                        "created": 1728345601
                    }
                ],
                "has_more": true,
                "url": "/v1/subscription_schedules"
            })
            .to_string();

            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                        resp.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }

        // Page 2
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req_str.contains("starting_after=sub_sched_page_002"));

            let resp = serde_json::json!({
                "object": "list",
                "data": [
                    {
                        "id": "sub_sched_page_003",
                        "object": "subscription_schedule",
                        "customer": "cus_003",
                        "status": "released",
                        "created": 1728345602
                    }
                ],
                "has_more": false,
                "url": "/v1/subscription_schedules"
            })
            .to_string();

            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                        resp.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
    });

    let config =
        StripeConfig::new("sk_test_sched_create").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let params = ListSubscriptionSchedulesParams::new().limit(2);
    let mut stream = client.subscription_schedules().list(&params);

    let item1 = stream.next_item().await.unwrap().unwrap();
    assert_eq!(item1.id(), "sub_sched_page_001");
    assert_eq!(item1.status(), SubscriptionScheduleStatus::Active);

    let item2 = stream.next_item().await.unwrap().unwrap();
    assert_eq!(item2.id(), "sub_sched_page_002");
    assert_eq!(item2.status(), SubscriptionScheduleStatus::Completed);

    let item3 = stream.next_item().await.unwrap().unwrap();
    assert_eq!(item3.id(), "sub_sched_page_003");
    assert_eq!(item3.status(), SubscriptionScheduleStatus::Released);

    let item4 = stream.next_item().await.unwrap();
    assert!(item4.is_none());

    server.await.unwrap();
}

#[test]
fn test_gate_g_borrow_zero_clone_schedule_accessors() {
    let json = r#"{
        "id": "sub_sched_borrow_test",
        "object": "subscription_schedule",
        "customer": "cus_borrow_owner",
        "status": "active",
        "created": 1728345600,
        "subscription": "sub_active_ref",
        "released_subscription": "sub_released_ref",
        "phases": [
            {
                "start_date": 1728345600,
                "end_date": 1730937600,
                "iterations": 1,
                "proration_behavior": "create_prorations",
                "items": [
                    {
                        "price": "price_pro_plan",
                        "quantity": 3
                    }
                ]
            }
        ],
        "current_phase": {
            "start_date": 1728345600,
            "end_date": 1730937600
        }
    }"#;

    let sched: SubscriptionSchedule = serde_json::from_str(json).unwrap();

    // Verify all accessors return borrowed slices without heap cloning
    let id_ref: &str = sched.id();
    assert_eq!(id_ref, "sub_sched_borrow_test");

    let cus_ref: &str = sched.customer();
    assert_eq!(cus_ref, "cus_borrow_owner");

    let sub_ref: Option<&str> = sched.subscription_id();
    assert_eq!(sub_ref, Some("sub_active_ref"));

    let rel_ref: Option<&str> = sched.released_subscription_id();
    assert_eq!(rel_ref, Some("sub_released_ref"));

    let status_copy: SubscriptionScheduleStatus = sched.status();
    assert_eq!(status_copy, SubscriptionScheduleStatus::Active);

    let phases_slice = sched.phases();
    assert_eq!(phases_slice.len(), 1);
    assert_eq!(phases_slice[0].start_date(), 1728345600);
    assert_eq!(phases_slice[0].duration_seconds(), 2592000);
    assert_eq!(phases_slice[0].items()[0].price(), "price_pro_plan");
    assert_eq!(phases_slice[0].items()[0].quantity(), Some(3));

    let cur_phase = sched.current_phase().unwrap();
    assert_eq!(cur_phase.start_date(), 1728345600);
    assert_eq!(cur_phase.end_date(), 1730937600);
    assert_eq!(cur_phase.duration_seconds(), 2592000);
}

#[test]
fn test_gate_g_law_schedule_validation_invariants() {
    // 1. Missing both customer and from_subscription
    let invalid_missing = CreateSubscriptionScheduleParams::new();
    assert_eq!(
        invalid_missing.validate(),
        Err(SubscriptionScheduleValidationError::MissingCustomerOrSubscription)
    );

    // 2. Empty phases list
    let invalid_empty_phases = CreateSubscriptionScheduleParams::new()
        .customer("cus_test")
        .phases(vec![]);
    assert_eq!(
        invalid_empty_phases.validate(),
        Err(SubscriptionScheduleValidationError::EmptyPhases)
    );

    // 3. Phase with empty items
    let invalid_empty_items = CreateSubscriptionScheduleParams::new()
        .customer("cus_test")
        .phases(vec![CreateSubscriptionSchedulePhaseParams::new(vec![])]);
    assert_eq!(
        invalid_empty_items.validate(),
        Err(SubscriptionScheduleValidationError::EmptyPhaseItems)
    );

    // 4. Empty price string
    let invalid_empty_price = CreateSubscriptionScheduleParams::new()
        .customer("cus_test")
        .phases(vec![CreateSubscriptionSchedulePhaseParams::new(vec![
            SubscriptionSchedulePhaseItemParams::new("   "),
        ])]);
    assert_eq!(
        invalid_empty_price.validate(),
        Err(SubscriptionScheduleValidationError::EmptyPrice)
    );

    // 5. Zero quantity
    let invalid_zero_qty = CreateSubscriptionScheduleParams::new()
        .customer("cus_test")
        .phases(vec![CreateSubscriptionSchedulePhaseParams::new(vec![
            SubscriptionSchedulePhaseItemParams::new("price_123").quantity(0),
        ])]);
    assert!(matches!(
        invalid_zero_qty.validate(),
        Err(SubscriptionScheduleValidationError::InvalidField { .. })
    ));

    // 6. Zero iterations
    let invalid_zero_iterations = CreateSubscriptionScheduleParams::new()
        .customer("cus_test")
        .phases(vec![
            CreateSubscriptionSchedulePhaseParams::new(vec![
                SubscriptionSchedulePhaseItemParams::new("price_123").quantity(1),
            ])
            .iterations(0),
        ]);
    assert_eq!(
        invalid_zero_iterations.validate(),
        Err(SubscriptionScheduleValidationError::InvalidIterationCount)
    );

    // 7. Invalid date range (end_date <= start_date)
    let invalid_dates = CreateSubscriptionScheduleParams::new()
        .customer("cus_test")
        .phases(vec![
            CreateSubscriptionSchedulePhaseParams::new(vec![
                SubscriptionSchedulePhaseItemParams::new("price_123").quantity(1),
            ])
            .start_date(1730000000)
            .end_date(1720000000),
        ]);
    assert_eq!(
        invalid_dates.validate(),
        Err(SubscriptionScheduleValidationError::InvalidDateRange {
            start: 1730000000,
            end: 1720000000
        })
    );
}
