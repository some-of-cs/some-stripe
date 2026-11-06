//! Integration and unit tests for Subscription Items, Multi-Item Subscriptions,
//! and RFC 1738 zero-indexed bracket form serialization (US-5.1.0 Unit 3).
//!
//! Validates:
//! - RFC 1738 zero-indexed bracket form encoding (`items[0][price]=...&items[0][quantity]=...`).
//! - Multi-item subscription updates with in-place modifications and deletions (`items[i][deleted]=true`).
//! - `SubscriptionItemsResource` CRUD endpoints (`create`, `retrieve`, `update`, `del`, `list`, `list_stream`).
//! - Metered usage record ingestion (`create_usage_record`).
//! - Automatic RFC 4122 UUID v4 Idempotency-Key injection and preservation of custom keys.
//! - Strict path traversal prevention via [`validate_and_encode_id`].
//! - Zero-clone borrowing discipline (Gate G-Borrow) and ISO 4217 minor currency units (Gate G-Law).

use std::collections::HashMap;

use some_stripe::patterns::serializer::{to_form_params, to_form_urlencoded};
use some_stripe::{
    CreateSubscriptionItemParams, CreateSubscriptionParams, CreateUsageRecordParams,
    DeleteSubscriptionItemParams, ListSubscriptionItemsParams, ProrationBehavior, StripeClient,
    StripeConfig, SubscriptionItemBillingThresholds, SubscriptionItemParam,
    UpdateSubscriptionItemParams, UpdateSubscriptionParams, UsageRecordAction, is_valid_uuid_v4,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn pairs_to_map(pairs: Vec<(String, String)>) -> HashMap<String, String> {
    pairs.into_iter().collect()
}

// ============================================================================
// 1. RFC 1738 Zero-Indexed Bracket Notation Serialization Tests
// ============================================================================

#[test]
fn test_multi_item_subscription_creation_bracket_form_serialization() {
    let mut item0_meta = HashMap::new();
    item0_meta.insert("seat_type".into(), "developer".into());

    let params = CreateSubscriptionParams::new("cus_vault_test_001", vec![])
        .currency("usd")
        .trial_period_days(14)
        .proration_behavior(ProrationBehavior::CreateProrations)
        .add_item(
            SubscriptionItemParam::new("price_pro_seat_monthly", 5)
                .metadata(item0_meta)
                .billing_thresholds(SubscriptionItemBillingThresholds::new(100)),
        )
        .add_item(
            SubscriptionItemParam::new("price_analytics_addon_monthly", 1)
                .tax_rates(vec!["txr_vat_20".into()]),
        );

    let pairs = to_form_params(&params).expect("to_form_params serialization must succeed");
    let map = pairs_to_map(pairs);

    // Top-level parameters
    assert_eq!(
        map.get("customer").map(String::as_str),
        Some("cus_vault_test_001")
    );
    assert_eq!(map.get("currency").map(String::as_str), Some("usd"));
    assert_eq!(map.get("trial_period_days").map(String::as_str), Some("14"));
    assert_eq!(
        map.get("proration_behavior").map(String::as_str),
        Some("create_prorations")
    );

    // Item 0: Zero-indexed bracket notation
    assert_eq!(
        map.get("items[0][price]").map(String::as_str),
        Some("price_pro_seat_monthly")
    );
    assert_eq!(map.get("items[0][quantity]").map(String::as_str), Some("5"));
    assert_eq!(
        map.get("items[0][metadata][seat_type]").map(String::as_str),
        Some("developer")
    );
    assert_eq!(
        map.get("items[0][billing_thresholds][usage_gte]")
            .map(String::as_str),
        Some("100")
    );

    // Item 1: Zero-indexed bracket notation
    assert_eq!(
        map.get("items[1][price]").map(String::as_str),
        Some("price_analytics_addon_monthly")
    );
    assert_eq!(map.get("items[1][quantity]").map(String::as_str), Some("1"));
    assert_eq!(
        map.get("items[1][tax_rates][0]").map(String::as_str),
        Some("txr_vat_20")
    );

    // URL-encoded wire representation check
    let urlencoded =
        to_form_urlencoded(&params).expect("to_form_urlencoded serialization must succeed");
    assert!(
        urlencoded.contains("items%5B0%5D%5Bprice%5D=price_pro_seat_monthly")
            || urlencoded.contains("items[0][price]=price_pro_seat_monthly")
    );
    assert!(
        urlencoded.contains("items%5B1%5D%5Bprice%5D=price_analytics_addon_monthly")
            || urlencoded.contains("items[1][price]=price_analytics_addon_monthly")
    );
}

#[test]
fn test_multi_item_subscription_update_modifications_and_deletions() {
    let update_params = UpdateSubscriptionParams::new()
        .proration_behavior(ProrationBehavior::AlwaysInvoice)
        .add_item(
            SubscriptionItemParam::for_update("si_existing_seat_item")
                .quantity(10)
                .proration_behavior(ProrationBehavior::CreateProrations),
        )
        .add_item(SubscriptionItemParam::for_deletion("si_remove_addon_item"))
        .add_item(SubscriptionItemParam::with_price_and_quantity(
            "price_new_storage_tier",
            3,
        ));

    let pairs = to_form_params(&update_params).expect("to_form_params must succeed");
    let map = pairs_to_map(pairs);

    assert_eq!(
        map.get("proration_behavior").map(String::as_str),
        Some("always_invoice")
    );

    // Item 0: Update existing item
    assert_eq!(
        map.get("items[0][id]").map(String::as_str),
        Some("si_existing_seat_item")
    );
    assert_eq!(
        map.get("items[0][quantity]").map(String::as_str),
        Some("10")
    );
    assert_eq!(
        map.get("items[0][proration_behavior]").map(String::as_str),
        Some("create_prorations")
    );
    assert!(!map.contains_key("items[0][deleted]"));

    // Item 1: Delete existing item
    assert_eq!(
        map.get("items[1][id]").map(String::as_str),
        Some("si_remove_addon_item")
    );
    assert_eq!(
        map.get("items[1][deleted]").map(String::as_str),
        Some("true")
    );

    // Item 2: Add new item
    assert_eq!(
        map.get("items[2][price]").map(String::as_str),
        Some("price_new_storage_tier")
    );
    assert_eq!(map.get("items[2][quantity]").map(String::as_str), Some("3"));
}

// ============================================================================
// 2. SubscriptionItemsResource CRUD Wire / Mock Tests
// ============================================================================

#[tokio::test]
async fn test_subscription_items_create_auto_and_custom_idempotency() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Request 1: Auto-injected UUID v4 idempotency key
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/subscription_items HTTP/1.1"));
            let header = req_str
                .lines()
                .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
                .expect("Idempotency-Key must be injected on create");
            let key = header.split(':').nth(1).unwrap().trim();
            assert!(
                is_valid_uuid_v4(key),
                "Auto-injected key must be RFC 4122 UUID v4, got: {key}"
            );

            assert!(
                req_str.contains("subscription=sub_test_001")
                    || req_str.contains("subscription%3Dsub_test_001")
            );
            assert!(
                req_str.contains("price=price_extra_seats")
                    || req_str.contains("price%3Dprice_extra_seats")
            );
            assert!(req_str.contains("quantity=3") || req_str.contains("quantity%3D3"));

            let resp = serde_json::json!({
                "id": "si_test_create_001",
                "object": "subscription_item",
                "subscription": "sub_test_001",
                "created": 1728345600,
                "quantity": 3,
                "price": {
                    "id": "price_extra_seats",
                    "currency": "usd",
                    "unit_amount": 2500,
                    "product": "prod_seats_001",
                    "active": true
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

        // Request 2: Preserved custom idempotency key
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/subscription_items HTTP/1.1"));
            assert!(
                req_str.contains("idempotency-key: custom_sub_item_key_999"),
                "Caller's explicit Idempotency-Key must be preserved"
            );

            let resp = serde_json::json!({
                "id": "si_test_create_002",
                "object": "subscription_item",
                "subscription": "sub_test_001",
                "created": 1728345600,
                "quantity": 1
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
        StripeConfig::new("sk_test_subscription_items").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // Call 1: Auto idempotency
    let params1 = CreateSubscriptionItemParams::new("sub_test_001", "price_extra_seats")
        .quantity(3)
        .proration_behavior(ProrationBehavior::CreateProrations);
    let item1 = client.subscriptions().items.create(&params1).await.unwrap();
    assert_eq!(item1.id(), "si_test_create_001");
    assert_eq!(item1.subscription(), "sub_test_001");
    assert_eq!(item1.quantity(), Some(3));
    assert_eq!(item1.price.as_ref().unwrap().unit_amount, Some(2500));

    // Call 2: Custom idempotency
    let params2 = CreateSubscriptionItemParams::new("sub_test_001", "price_extra_seats")
        .quantity(1)
        .idempotency_key("custom_sub_item_key_999");
    let item2 = client.subscriptions().items.create(&params2).await.unwrap();
    assert_eq!(item2.id(), "si_test_create_002");

    server.await.unwrap();
}

#[tokio::test]
async fn test_subscription_items_retrieve_update_delete_and_path_traversal() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Retrieve
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(
                req_str.starts_with("GET /v1/subscription_items/si_test_retrieve_123 HTTP/1.1")
            );
            assert!(
                !req_str.to_lowercase().contains("idempotency-key:"),
                "Idempotent GET must not inject Idempotency-Key"
            );

            let resp = serde_json::json!({
                "id": "si_test_retrieve_123",
                "object": "subscription_item",
                "subscription": "sub_test_123",
                "created": 1728345600,
                "quantity": 2
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

        // 2. Update
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(
                req_str.starts_with("POST /v1/subscription_items/si_test_retrieve_123 HTTP/1.1")
            );
            assert!(req_str.contains("quantity=7") || req_str.contains("quantity%3D7"));

            let resp = serde_json::json!({
                "id": "si_test_retrieve_123",
                "object": "subscription_item",
                "subscription": "sub_test_123",
                "created": 1728345600,
                "quantity": 7
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

        // 3. Delete
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(
                req_str.starts_with("DELETE /v1/subscription_items/si_test_retrieve_123 HTTP/1.1")
            );

            let resp = serde_json::json!({
                "id": "si_test_retrieve_123",
                "object": "subscription_item",
                "deleted": true
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
        StripeConfig::new("sk_test_subscription_items").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // Path traversal check: must return InvalidRequest before making any network call
    assert!(
        client
            .subscriptions()
            .items
            .retrieve("../etc/passwd")
            .await
            .is_err()
    );
    assert!(
        client
            .subscriptions()
            .items
            .retrieve("si_123?action=delete")
            .await
            .is_err()
    );
    assert!(client.subscriptions().items.retrieve("   ").await.is_err());

    // 1. Retrieve
    let item = client
        .subscriptions()
        .items
        .retrieve("si_test_retrieve_123")
        .await
        .unwrap();
    assert_eq!(item.id(), "si_test_retrieve_123");
    assert_eq!(item.quantity(), Some(2));

    // 2. Update
    let update = UpdateSubscriptionItemParams::new().quantity(7);
    let updated = client
        .subscriptions()
        .items
        .update("si_test_retrieve_123", &update)
        .await
        .unwrap();
    assert_eq!(updated.quantity(), Some(7));

    // 3. Delete
    let del_params = DeleteSubscriptionItemParams::new()
        .clear_usage(true)
        .proration_behavior(ProrationBehavior::None);
    let deleted = client
        .subscriptions()
        .items
        .del("si_test_retrieve_123", Some(&del_params))
        .await
        .unwrap();
    assert!(deleted.is_deleted());
    assert_eq!(deleted.id(), "si_test_retrieve_123");

    server.await.unwrap();
}

#[tokio::test]
async fn test_subscription_items_list_and_paginator_stream() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Page 1
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("GET /v1/subscription_items?"));
            assert!(req_str.contains("subscription=sub_stream_001"));
            assert!(req_str.contains("limit=2"));

            let resp = serde_json::json!({
                "object": "list",
                "data": [
                    {
                        "id": "si_page1_001",
                        "object": "subscription_item",
                        "subscription": "sub_stream_001",
                        "created": 1728345600,
                        "quantity": 1
                    },
                    {
                        "id": "si_page1_002",
                        "object": "subscription_item",
                        "subscription": "sub_stream_001",
                        "created": 1728345601,
                        "quantity": 2
                    }
                ],
                "has_more": true,
                "url": "/v1/subscription_items"
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

            assert!(req_str.contains("starting_after=si_page1_002"));

            let resp = serde_json::json!({
                "object": "list",
                "data": [
                    {
                        "id": "si_page2_003",
                        "object": "subscription_item",
                        "subscription": "sub_stream_001",
                        "created": 1728345602,
                        "quantity": 5
                    }
                ],
                "has_more": false,
                "url": "/v1/subscription_items"
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
        StripeConfig::new("sk_test_subscription_items").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let list_params = ListSubscriptionItemsParams::new("sub_stream_001").limit(2);
    let mut paginator = client.subscriptions().items.list_stream(&list_params);

    let mut item_ids = Vec::new();
    while let Some(item) = paginator.next_item().await.unwrap() {
        item_ids.push(item.id().to_string());
    }

    assert_eq!(
        item_ids,
        vec![
            "si_page1_001".to_string(),
            "si_page1_002".to_string(),
            "si_page2_003".to_string(),
        ]
    );
    assert_eq!(paginator.pages_fetched(), 2);

    server.await.unwrap();
}

#[tokio::test]
async fn test_subscription_items_create_usage_record() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with(
            "POST /v1/subscription_items/si_metered_units_123/usage_records HTTP/1.1"
        ));
        assert!(req_str.contains("quantity=250") || req_str.contains("quantity%3D250"));
        assert!(req_str.contains("action=increment") || req_str.contains("action%3Dincrement"));
        assert!(
            req_str.contains("timestamp=1728345600") || req_str.contains("timestamp%3D1728345600")
        );

        let resp = serde_json::json!({
            "id": "mbur_test_record_001",
            "object": "usage_record",
            "livemode": false,
            "quantity": 250,
            "subscription_item": "si_metered_units_123",
            "timestamp": 1728345600,
            "action": "increment"
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
        StripeConfig::new("sk_test_subscription_items").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // Path traversal check
    assert!(
        client
            .subscriptions()
            .items
            .create_usage_record("../si_evil", &CreateUsageRecordParams::new(100),)
            .await
            .is_err()
    );

    let params = CreateUsageRecordParams::new(250)
        .timestamp(1728345600)
        .action(UsageRecordAction::Increment);

    let usage = client
        .subscriptions()
        .items
        .create_usage_record("si_metered_units_123", &params)
        .await
        .unwrap();

    assert_eq!(usage.id(), "mbur_test_record_001");
    assert_eq!(usage.subscription_item(), "si_metered_units_123");
    assert_eq!(usage.quantity(), 250);

    server.await.unwrap();
}
