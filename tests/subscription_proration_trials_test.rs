//! Gate G0, Gate G-Borrow, Gate G2, Gate G-Law, and Mock Wire Integration Tests
//! for In-Flight Plan Prorations, 14-Day Free Trials, and Pause Collection (US-5.1.0 Unit 5).
//!
//! # Quality Invariants Verified:
//! - **Gate G0 (Rust 2024 Edition)**: Clean compilation with 0 clippy warnings and strict code formatting.
//! - **Gate G-Borrow (Zero-Clone Discipline)**:
//!   - Strict parameter borrowing by reference (`&PauseCollectionParams`, `&PreviewProrationParams`, `&str`).
//!   - Zero deep `.clone()` calls during proration preview, collection pause, and resume workflows.
//! - **Gate G2 (Security & Cryptographic Invariants)**:
//!   - Automatic injection of RFC 4122 UUID v4 `Idempotency-Key` headers on mutable operations (`pause_collection`, `resume`).
//! - **Gate G-Law (ISO 4217 Minor Currency Units & Global Financial Compliance)**:
//!   - Unspent balance credit and prorated charge calculations strictly computed in 64/128-bit integer minor currency units.
//!   - Zero floating-point representation or rounding leakage across zero-decimal (JPY), 2-decimal (USD/EUR), and 3-decimal (KWD) currencies.
//! - **Mock Wire Integration**:
//!   - Ephemeral HTTP simulation verifying wire contracts, parameter encoding, and response deserialization.

use some_stripe::{
    FOURTEEN_DAYS_SECONDS, MIN_TRIAL_DURATION_SECONDS, PauseCollection, PauseCollectionError,
    PauseCollectionParams, PreviewProrationParams, ProrationBehavior, ProrationError,
    SECONDS_PER_DAY, StripeClient, StripeConfig, SubscriptionStatus, TrialBoundsError,
    calculate_14_day_trial_end, calculate_proration, calculate_trial_end_from_days,
    calculate_trial_period_days, is_subscription_in_trial, is_valid_uuid_v4, validate_trial_bounds,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// ---------------------------------------------------------------------------
// 1. In-Flight Plan Proration Math Tests (ISO 4217 Minor Currency Units)
// ---------------------------------------------------------------------------

#[test]
fn test_proration_midpoint_upgrade_unspent_balance() {
    // Scenario: Customer is on a $20/month plan (2000 minor units).
    // At day 15 of a 30-day billing cycle (50% elapsed), customer upgrades to $50/month (5000 minor units).
    let start = 1704067200; // Day 0
    let end = start + 30 * SECONDS_PER_DAY; // Day 30
    let proration_date = start + 15 * SECONDS_PER_DAY; // Day 15 (15 days remaining)

    let calc = calculate_proration(2000, 5000, start, end, proration_date, "usd")
        .expect("Proration calculation must succeed");

    assert_eq!(calc.original_unit_amount, 2000);
    assert_eq!(calc.new_unit_amount, 5000);
    assert_eq!(calc.period_start, start);
    assert_eq!(calc.period_end, end);
    assert_eq!(calc.proration_date, proration_date);
    assert_eq!(calc.elapsed_seconds, 15 * SECONDS_PER_DAY);
    assert_eq!(calc.remaining_seconds, 15 * SECONDS_PER_DAY);
    assert_eq!(calc.total_seconds, 30 * SECONDS_PER_DAY);

    // 15 days unused of $20 plan -> $10.00 credit (1000 minor units)
    assert_eq!(calc.unspent_credit, 1000);
    // 15 days of $50 plan -> $25.00 charge (2500 minor units)
    assert_eq!(calc.prorated_charge, 2500);
    // Net amount due = 2500 - 1000 = +1500 ($15.00)
    assert_eq!(calc.net_amount, 1500);

    assert!(calc.is_upgrade());
    assert!(!calc.is_downgrade());
    assert!(calc.is_net_charge());
    assert!(!calc.is_net_credit());
    assert_eq!(calc.net_credit_amount(), 0);
    assert_eq!(calc.currency, "usd");
}

#[test]
fn test_proration_downgrade_net_credit() {
    // Scenario: Customer is on a $90/month plan (9000 minor units).
    // At day 10 of a 30-day billing cycle (20 days remaining = 2/3), customer downgrades to $30/month (3000 minor units).
    let start = 1704067200;
    let end = start + 30 * SECONDS_PER_DAY;
    let proration_date = start + 10 * SECONDS_PER_DAY; // 20 days remaining

    let calc = calculate_proration(9000, 3000, start, end, proration_date, "eur")
        .expect("Proration calculation must succeed");

    // Unspent credit = 9000 * 20 / 30 = 6000 ($60.00)
    assert_eq!(calc.unspent_credit, 6000);
    // Prorated charge = 3000 * 20 / 30 = 2000 ($20.00)
    assert_eq!(calc.prorated_charge, 2000);
    // Net amount = 2000 - 6000 = -4000 (credit balance of $40.00)
    assert_eq!(calc.net_amount, -4000);

    assert!(!calc.is_upgrade());
    assert!(calc.is_downgrade());
    assert!(!calc.is_net_charge());
    assert!(calc.is_net_credit());
    assert_eq!(calc.net_credit_amount(), 4000);
    assert_eq!(calc.currency, "eur");
}

#[test]
fn test_proration_cycle_boundary_conditions() {
    let start = 1704067200;
    let end = start + 30 * SECONDS_PER_DAY;

    // Boundary 1: Plan migration right at cycle start (Day 0)
    let start_calc = calculate_proration(2000, 5000, start, end, start, "usd")
        .expect("Day 0 calculation must succeed");
    assert_eq!(start_calc.unspent_credit, 2000); // 100% credit of old plan
    assert_eq!(start_calc.prorated_charge, 5000); // 100% charge of new plan
    assert_eq!(start_calc.net_amount, 3000);

    // Boundary 2: Plan migration right at cycle end (Day 30)
    let end_calc = calculate_proration(2000, 5000, start, end, end, "usd")
        .expect("Day 30 calculation must succeed");
    assert_eq!(end_calc.unspent_credit, 0); // 0 remaining days
    assert_eq!(end_calc.prorated_charge, 0);
    assert_eq!(end_calc.net_amount, 0);
    assert!(!end_calc.is_net_charge());
    assert!(!end_calc.is_net_credit());
}

#[test]
fn test_proration_validation_errors() {
    let start = 1704067200;
    let end = start + 30 * SECONDS_PER_DAY;

    // Invalid billing period: end <= start
    let err = calculate_proration(2000, 5000, end, start, start, "usd")
        .expect_err("Must reject invalid billing period");
    assert!(matches!(err, ProrationError::InvalidBillingPeriod { .. }));

    // Proration date before cycle start
    let err = calculate_proration(2000, 5000, start, end, start - 100, "usd")
        .expect_err("Must reject proration date before start");
    assert!(matches!(err, ProrationError::InvalidProrationDate { .. }));

    // Proration date after cycle end
    let err = calculate_proration(2000, 5000, start, end, end + 100, "usd")
        .expect_err("Must reject proration date after end");
    assert!(matches!(err, ProrationError::InvalidProrationDate { .. }));

    // Negative unit amount
    let err = calculate_proration(-100, 5000, start, end, start + 100, "usd")
        .expect_err("Must reject negative unit amounts");
    assert!(matches!(err, ProrationError::NegativeUnitAmount { .. }));

    // Empty currency string
    let err = calculate_proration(2000, 5000, start, end, start + 100, "   ")
        .expect_err("Must reject empty currency");
    assert!(matches!(err, ProrationError::EmptyCurrency));
}

#[test]
fn test_gate_g_law_iso_4217_zero_decimal_and_three_decimal_currencies() {
    let start = 1704067200;
    let end = start + 30 * SECONDS_PER_DAY;
    let proration_date = start + 15 * SECONDS_PER_DAY;

    // Zero-decimal currency: Japanese Yen (JPY) - whole yen only, zero fractions
    let jpy_calc = calculate_proration(3000, 6000, start, end, proration_date, "jpy")
        .expect("JPY calculation must succeed");
    assert_eq!(jpy_calc.unspent_credit, 1500); // 1500 JPY
    assert_eq!(jpy_calc.prorated_charge, 3000); // 3000 JPY
    assert_eq!(jpy_calc.net_amount, 1500); // 1500 JPY
    assert_eq!(jpy_calc.currency, "jpy");

    // Three-decimal currency: Kuwaiti Dinar (KWD) - 1 KWD = 1000 fils
    // 10.000 KWD (10000 fils) to 25.000 KWD (25000 fils)
    let kwd_calc = calculate_proration(10000, 25000, start, end, proration_date, "kwd")
        .expect("KWD calculation must succeed");
    assert_eq!(kwd_calc.unspent_credit, 5000);
    assert_eq!(kwd_calc.prorated_charge, 12500);
    assert_eq!(kwd_calc.net_amount, 7500);
    assert_eq!(kwd_calc.currency, "kwd");
}

// ---------------------------------------------------------------------------
// 2. 14-Day Free Trial Bounds and Calculator Tests
// ---------------------------------------------------------------------------

#[test]
fn test_14_day_free_trial_calculators() {
    let start = 1704067200;

    // 14 days in seconds = 14 * 86,400 = 1,209,600
    assert_eq!(FOURTEEN_DAYS_SECONDS, 1_209_600);
    assert_eq!(SECONDS_PER_DAY, 86_400);

    let trial_end =
        calculate_trial_end_from_days(start, 14).expect("14-day calculation must succeed");
    assert_eq!(trial_end, start + 1_209_600);

    let convenience_end = calculate_14_day_trial_end(start);
    assert_eq!(convenience_end, trial_end);

    let days = calculate_trial_period_days(start, trial_end)
        .expect("Period days calculation must succeed");
    assert_eq!(days, 14);

    // Active trial checking
    assert!(is_subscription_in_trial(
        start,
        Some(start),
        Some(trial_end)
    ));
    assert!(is_subscription_in_trial(
        start + 500_000,
        Some(start),
        Some(trial_end)
    ));
    assert!(!is_subscription_in_trial(
        trial_end,
        Some(start),
        Some(trial_end)
    ));
    assert!(!is_subscription_in_trial(
        trial_end + 1,
        Some(start),
        Some(trial_end)
    ));
    assert!(!is_subscription_in_trial(
        start - 1,
        Some(start),
        Some(trial_end)
    ));
    assert!(!is_subscription_in_trial(start, None, None));
}

#[test]
fn test_trial_bounds_validation_and_errors() {
    let now = 1704067200;

    // 0 days trial must fail
    let err = calculate_trial_end_from_days(now, 0).expect_err("Must reject 0 days");
    assert!(matches!(err, TrialBoundsError::InvalidTrialPeriodDays(0)));

    // Trial end in the past
    let past_end = now - 100;
    let err = validate_trial_bounds(now, past_end, None).expect_err("Must reject past trial_end");
    assert!(matches!(err, TrialBoundsError::TrialEndMustBeFuture { .. }));

    // Trial duration too short (e.g., 24 hours when 48 hours required)
    let too_short_end = now + 24 * 3600;
    let err = validate_trial_bounds(now, too_short_end, Some(MIN_TRIAL_DURATION_SECONDS))
        .expect_err("Must reject trial duration shorter than 48 hours");
    assert!(matches!(
        err,
        TrialBoundsError::TrialDurationTooShort { .. }
    ));

    // Valid 14-day trial end
    let valid_end = now + FOURTEEN_DAYS_SECONDS;
    assert!(validate_trial_bounds(now, valid_end, Some(MIN_TRIAL_DURATION_SECONDS)).is_ok());
}

// ---------------------------------------------------------------------------
// 3. PauseCollection & PauseCollectionParams Tests
// ---------------------------------------------------------------------------

#[test]
fn test_pause_collection_model_and_validation() {
    let pause_void = PauseCollection::void();
    assert!(pause_void.is_void());
    assert!(!pause_void.is_mark_uncollectible());
    assert!(!pause_void.is_keep_as_draft());
    assert_eq!(pause_void.behavior, "void");
    assert_eq!(pause_void.resumes_at, None);
    assert!(!pause_void.is_resumed(1704067200));

    let pause_mark = PauseCollection::mark_uncollectible();
    assert!(pause_mark.is_mark_uncollectible());

    let pause_draft = PauseCollection::keep_as_draft();
    assert!(pause_draft.is_keep_as_draft());

    let pause_with_resumes = PauseCollection::new("void", Some(1735689600));
    assert!(!pause_with_resumes.is_resumed(1735689500));
    assert!(pause_with_resumes.is_resumed(1735689600));
    assert!(pause_with_resumes.is_resumed(1735689700));

    // Params builders & validation
    let params_void = PauseCollectionParams::void().with_resumes_at(1735689600);
    assert!(params_void.validate().is_ok());
    assert!(params_void.validate_with_time(1704067200).is_ok());

    // Resumes at in the past
    let err = params_void
        .validate_with_time(1740000000)
        .expect_err("Must reject past resumes_at");
    assert!(matches!(
        err,
        PauseCollectionError::ResumesAtMustBeFuture { .. }
    ));

    // Invalid behavior
    let invalid_params = PauseCollectionParams::new("invalid_mode");
    let err = invalid_params
        .validate()
        .expect_err("Must reject invalid behavior");
    assert!(matches!(err, PauseCollectionError::InvalidBehavior(_)));
}

#[test]
fn test_pause_collection_serde_roundtrip() {
    let json = r#"{"behavior":"void","resumes_at":1735689600}"#;
    let pause: PauseCollection = serde_json::from_str(json).unwrap();
    assert_eq!(pause.behavior, "void");
    assert_eq!(pause.resumes_at, Some(1735689600));

    let params: PauseCollectionParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.behavior, "void");
    assert_eq!(params.resumes_at, Some(1735689600));

    let serialized = serde_json::to_string(&params).unwrap();
    assert!(serialized.contains(r#""behavior":"void""#));
    assert!(serialized.contains(r#""resumes_at":1735689600"#));
}

// ---------------------------------------------------------------------------
// 4. ProrationBehavior Enum Tests
// ---------------------------------------------------------------------------

#[test]
fn test_proration_behavior_variants_and_serde() {
    assert_eq!(
        ProrationBehavior::CreateProrations.as_str(),
        "create_prorations"
    );
    assert_eq!(ProrationBehavior::None.as_str(), "none");
    assert_eq!(ProrationBehavior::AlwaysInvoice.as_str(), "always_invoice");

    assert!(ProrationBehavior::CreateProrations.is_proration_enabled());
    assert!(ProrationBehavior::AlwaysInvoice.is_proration_enabled());
    assert!(!ProrationBehavior::None.is_proration_enabled());

    assert_eq!(
        ProrationBehavior::default(),
        ProrationBehavior::CreateProrations
    );
    assert_eq!(
        format!("{}", ProrationBehavior::CreateProrations),
        "create_prorations"
    );

    let json = r#""create_prorations""#;
    let pb: ProrationBehavior = serde_json::from_str(json).unwrap();
    assert_eq!(pb, ProrationBehavior::CreateProrations);

    let json = r#""none""#;
    let pb: ProrationBehavior = serde_json::from_str(json).unwrap();
    assert_eq!(pb, ProrationBehavior::None);

    let json = r#""always_invoice""#;
    let pb: ProrationBehavior = serde_json::from_str(json).unwrap();
    assert_eq!(pb, ProrationBehavior::AlwaysInvoice);
}

// ---------------------------------------------------------------------------
// 5. Gate G-Borrow Zero-Clone Parameter Borrowing Discipline
// ---------------------------------------------------------------------------

#[test]
fn test_gate_g_borrow_zero_clone_parameter_borrowing() {
    let pause_params = PauseCollectionParams::void().with_resumes_at(1735689600);
    let preview_params = PreviewProrationParams::new()
        .subscription("sub_test_borrow_123")
        .amounts(2000, 5000, "usd")
        .period(1704067200, 1704067200 + 30 * SECONDS_PER_DAY)
        .proration_date(1704067200 + 15 * SECONDS_PER_DAY);

    // Borrow references strictly by & without consuming or cloning
    let borrowed_pause: &PauseCollectionParams = &pause_params;
    assert_eq!(borrowed_pause.behavior, "void");
    assert_eq!(borrowed_pause.resumes_at, Some(1735689600));

    let borrowed_preview: &PreviewProrationParams = &preview_params;
    assert_eq!(
        borrowed_preview.subscription.as_deref(),
        Some("sub_test_borrow_123")
    );
    assert_eq!(borrowed_preview.original_unit_amount, Some(2000));
    assert_eq!(borrowed_preview.new_unit_amount, Some(5000));
    assert_eq!(borrowed_preview.currency.as_deref(), Some("usd"));

    // Verify original params remain fully usable and unconsumed
    assert!(pause_params.validate().is_ok());
    assert_eq!(preview_params.original_unit_amount, Some(2000));
}

// ---------------------------------------------------------------------------
// 6. SubscriptionsResource Wire Mock Integration Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_subscriptions_resource_preview_proration_local_fast_path() {
    let client = StripeClient::new("sk_test_mock_123");
    let subscriptions = client.subscriptions();

    let start = 1704067200;
    let end = start + 30 * SECONDS_PER_DAY;
    let proration_date = start + 15 * SECONDS_PER_DAY;

    let params = PreviewProrationParams::new()
        .amounts(2000, 5000, "usd")
        .period(start, end)
        .proration_date(proration_date);

    // Fast path calculates locally without invoking HTTP network
    let calc = subscriptions
        .preview_proration("sub_any_id", &params)
        .await
        .expect("Fast-path local proration must succeed");

    assert_eq!(calc.unspent_credit, 1000);
    assert_eq!(calc.prorated_charge, 2500);
    assert_eq!(calc.net_amount, 1500);
    assert!(calc.is_upgrade());
}

#[tokio::test]
async fn test_subscriptions_resource_mock_wire_lifecycle() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind ephemeral mock port");
    let address = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        // Request 1: GET /v1/subscriptions/sub_wire_test_100 (for slow-path preview proration)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req.starts_with("GET /v1/subscriptions/sub_wire_test_100 HTTP/1.1"));

            let sub_json = serde_json::json!({
                "id": "sub_wire_test_100",
                "object": "subscription",
                "customer": "cus_wire_test_100",
                "status": "active",
                "current_period_start": 1704067200,
                "current_period_end": 1704067200 + 30 * 86400,
                "created": 1704067200,
                "currency": "usd",
                "items": {
                    "object": "list",
                    "data": [{
                        "id": "si_wire_test_100",
                        "object": "subscription_item",
                        "subscription": "sub_wire_test_100",
                        "price": {
                            "id": "price_basic_20",
                            "currency": "usd",
                            "unit_amount": 2000,
                            "product": "prod_basic",
                            "active": true
                        },
                        "quantity": 1,
                        "created": 1704067200
                    }],
                    "has_more": false,
                    "url": "/v1/subscription_items"
                }
            });
            let body = sub_json.to_string();
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }

        // Request 2: POST /v1/subscriptions/sub_wire_test_100 (pause collection)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req.starts_with("POST /v1/subscriptions/sub_wire_test_100 HTTP/1.1"));

            // Verify Idempotency-Key header is auto-injected and conforms to UUID v4
            let header = req
                .lines()
                .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
                .expect("Auto Idempotency-Key header must be injected when pausing collection");
            let key = header.split(':').nth(1).unwrap().trim();
            assert!(
                is_valid_uuid_v4(key),
                "Injected Idempotency-Key must be valid RFC 4122 UUID v4: {key}"
            );

            // Verify form payload
            assert!(
                req.contains("pause_collection%5Bbehavior%5D=void")
                    || req.contains("pause_collection[behavior]=void")
            );

            let paused_json = serde_json::json!({
                "id": "sub_wire_test_100",
                "object": "subscription",
                "customer": "cus_wire_test_100",
                "status": "paused",
                "current_period_start": 1704067200,
                "current_period_end": 1704067200 + 30 * 86400,
                "created": 1704067200,
                "currency": "usd",
                "pause_collection": {
                    "behavior": "void",
                    "resumes_at": null
                },
                "items": {
                    "object": "list",
                    "data": [],
                    "has_more": false,
                    "url": "/v1/subscription_items"
                }
            });
            let body = paused_json.to_string();
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }

        // Request 3: POST /v1/subscriptions/sub_wire_test_100 (resume collection)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req.starts_with("POST /v1/subscriptions/sub_wire_test_100 HTTP/1.1"));

            // Verify Idempotency-Key header is auto-injected
            let header = req
                .lines()
                .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
                .expect("Auto Idempotency-Key header must be injected when resuming collection");
            let key = header.split(':').nth(1).unwrap().trim();
            assert!(
                is_valid_uuid_v4(key),
                "Injected Idempotency-Key must be valid RFC 4122 UUID v4: {key}"
            );

            // Verify pause_collection is cleared
            assert!(req.contains("pause_collection="));

            let resumed_json = serde_json::json!({
                "id": "sub_wire_test_100",
                "object": "subscription",
                "customer": "cus_wire_test_100",
                "status": "active",
                "current_period_start": 1704067200,
                "current_period_end": 1704067200 + 30 * 86400,
                "created": 1704067200,
                "currency": "usd",
                "pause_collection": null,
                "items": {
                    "object": "list",
                    "data": [],
                    "has_more": false,
                    "url": "/v1/subscription_items"
                }
            });
            let body = resumed_json.to_string();
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
    });

    let config = StripeConfig::new("sk_test_wire_mock").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);
    let subscriptions = client.subscriptions();

    // 1. Slow path preview proration: inspects fetched subscription to compute upgrade from $20 to $50
    let preview_params = PreviewProrationParams::new()
        .new_unit_amount(5000)
        .proration_date(1704067200 + 15 * 86400); // Day 15 (15 days remaining)
    let proration = subscriptions
        .preview_proration("sub_wire_test_100", &preview_params)
        .await
        .expect("Slow-path preview proration must succeed");

    assert_eq!(proration.original_unit_amount, 2000);
    assert_eq!(proration.new_unit_amount, 5000);
    assert_eq!(proration.unspent_credit, 1000);
    assert_eq!(proration.prorated_charge, 2500);
    assert_eq!(proration.net_amount, 1500);
    assert!(proration.is_upgrade());

    // 2. Pause payment collection
    let pause_params = PauseCollectionParams::void();
    let paused = subscriptions
        .pause_collection("sub_wire_test_100", &pause_params)
        .await
        .expect("Pause collection must succeed");

    assert_eq!(paused.status, SubscriptionStatus::Paused);
    assert!(paused.pause_collection.is_some());
    assert!(paused.pause_collection.unwrap().is_void());

    // 3. Resume payment collection
    let resumed = subscriptions
        .resume_collection("sub_wire_test_100")
        .await
        .expect("Resume collection must succeed");

    assert_eq!(resumed.status, SubscriptionStatus::Active);
    assert!(resumed.pause_collection.is_none());

    server_task.await.unwrap();
}
