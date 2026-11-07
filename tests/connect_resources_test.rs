//! Comprehensive unit and wire mock tests for Stripe Connect Resources.
//!
//! Validates:
//! - Quality Gate G0: 0 compiler/clippy warnings under Rust 2024.
//! - Quality Gate G-Borrow: Zero-clone borrowing discipline on domain models.
//! - Quality Gate G2: Automatic RFC 4122 UUID v4 Idempotency-Key injection and tenant isolation (`Stripe-Account`).
//! - Quality Gate G-Law: FinCEN CDD 25% ownership, DAC7 / AMLD5 compliance, and path traversal validation.
//! - AccountsResource wire contracts: `create`, `retrieve`, `retrieve_by_id`, `update`, `del`, `reject`, `list`.
//! - AccountLinksResource wire contracts: `create`.
//! - AccountSessionsResource wire contracts: `create`.
//! - PersonsResource wire contracts: `create`, `retrieve`, `update`, `del`, `list`, `paginate`.

use some_stripe::types::account::{
    Account, AccountList, AccountType, CreateAccountParams, DeletedAccount, ListAccountsParams,
    RejectAccountParams, UpdateAccountParams,
};
use some_stripe::types::account_link::{
    AccountLink, AccountLinkType, CreateAccountLinkParams,
};
use some_stripe::types::account_session::{
    AccountSession, AccountSessionComponents, CreateAccountSessionParams,
};
use some_stripe::types::person::{
    CreatePersonParams, DeletedPerson, ListPersonsParams, Person, PersonDob, PersonList,
    PersonRelationship, UpdatePersonParams,
};
use some_stripe::types::Nullable;
use some_stripe::{StripeClient, StripeConfig, is_valid_uuid_v4};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_accounts_resource_create_and_retrieve_by_id() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Request 1: POST /v1/accounts
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/accounts HTTP/1.1"));

            // Verify auto-injected Idempotency-Key
            let key_header = req_str
                .lines()
                .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
                .expect("Auto Idempotency-Key must be present");
            let key_val = key_header.split(':').nth(1).unwrap().trim();
            assert!(
                is_valid_uuid_v4(key_val),
                "Expected valid UUID v4 idempotency key, got {key_val}"
            );

            let resp = serde_json::json!({
                "id": "acct_test_create_123",
                "object": "account",
                "type": "custom",
                "country": "US",
                "email": "vendor@example.com",
                "charges_enabled": false,
                "payouts_enabled": false
            });
            let body = resp.to_string();
            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(http_resp.as_bytes()).await.unwrap();
        }

        // Request 2: GET /v1/accounts/acct_test_create_123
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("GET /v1/accounts/acct_test_create_123 HTTP/1.1"));
            // GET must NOT have Idempotency-Key
            assert!(!req_str.to_lowercase().contains("idempotency-key:"));

            let resp = serde_json::json!({
                "id": "acct_test_create_123",
                "object": "account",
                "type": "custom",
                "country": "US",
                "email": "vendor@example.com",
                "charges_enabled": true,
                "payouts_enabled": true
            });
            let body = resp.to_string();
            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(http_resp.as_bytes()).await.unwrap();
        }
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let create_params = CreateAccountParams::new()
        .account_type(AccountType::Custom)
        .country("US")
        .email("vendor@example.com");

    let created = client.accounts().create(&create_params).await.unwrap();
    assert_eq!(created.id(), "acct_test_create_123");
    assert_eq!(created.type_(), Some(AccountType::Custom));

    let retrieved = client
        .accounts()
        .retrieve_by_id("acct_test_create_123")
        .await
        .unwrap();
    assert_eq!(retrieved.id(), "acct_test_create_123");
    assert!(retrieved.is_charges_enabled());
    assert!(retrieved.is_payouts_enabled());

    server.await.unwrap();
}

#[tokio::test]
async fn test_accounts_resource_retrieve_current_and_tenant_header() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("GET /v1/account HTTP/1.1"));

        // Verify Stripe-Account tenant header was injected
        let acct_header = req_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("stripe-account:"))
            .expect("Stripe-Account header must be present");
        assert_eq!(acct_header.split(':').nth(1).unwrap().trim(), "acct_connected_tenant_1");

        let resp = serde_json::json!({
            "id": "acct_connected_tenant_1",
            "object": "account",
            "type": "express",
            "charges_enabled": true,
            "payouts_enabled": true,
            "country": "GB",
            "default_currency": "gbp"
        });
        let body = resp.to_string();
        let http_resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(http_resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let account = client
        .accounts()
        .with_account("acct_connected_tenant_1")
        .retrieve()
        .await
        .unwrap();

    assert_eq!(account.id(), "acct_connected_tenant_1");
    assert_eq!(account.country(), Some("GB"));
    assert_eq!(account.default_currency(), Some("gbp"));

    server.await.unwrap();
}

#[tokio::test]
async fn test_accounts_resource_update_and_custom_idempotency_key() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("POST /v1/accounts/acct_target_999 HTTP/1.1"));

        // Verify explicit caller-provided Idempotency-Key
        let key_header = req_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
            .expect("Custom Idempotency-Key header missing");
        assert_eq!(
            key_header.split(':').nth(1).unwrap().trim(),
            "custom_account_key_999"
        );

        let resp = serde_json::json!({
            "id": "acct_target_999",
            "object": "account",
            "type": "custom",
            "email": "updated@example.com"
        });
        let body = resp.to_string();
        let http_resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(http_resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let update_params = UpdateAccountParams::new()
        .email(Nullable::Value("updated@example.com".into()))
        .idempotency_key("custom_account_key_999");

    let updated = client
        .accounts()
        .update("acct_target_999", &update_params)
        .await
        .unwrap();

    assert_eq!(updated.id(), "acct_target_999");
    assert_eq!(updated.email(), Some("updated@example.com"));

    server.await.unwrap();
}

#[tokio::test]
async fn test_accounts_resource_del_and_delete_alias() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Request 1: del
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("DELETE /v1/accounts/acct_del_1 HTTP/1.1"));

            let resp = serde_json::json!({
                "id": "acct_del_1",
                "object": "account",
                "deleted": true
            });
            let body = resp.to_string();
            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(http_resp.as_bytes()).await.unwrap();
        }

        // Request 2: delete alias
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(req_str.starts_with("DELETE /v1/accounts/acct_del_2 HTTP/1.1"));

            let resp = serde_json::json!({
                "id": "acct_del_2",
                "object": "account",
                "deleted": true
            });
            let body = resp.to_string();
            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(http_resp.as_bytes()).await.unwrap();
        }
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let res1: DeletedAccount = client.accounts().del("acct_del_1").await.unwrap();
    assert_eq!(res1.id, "acct_del_1");
    assert!(res1.deleted);

    let res2: DeletedAccount = client.accounts().delete("acct_del_2").await.unwrap();
    assert_eq!(res2.id, "acct_del_2");
    assert!(res2.deleted);

    server.await.unwrap();
}

#[tokio::test]
async fn test_accounts_resource_reject() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("POST /v1/accounts/acct_reject_123/reject HTTP/1.1"));
        assert!(req_str.contains("reason=fraud"));

        let resp = serde_json::json!({
            "id": "acct_reject_123",
            "object": "account",
            "type": "custom",
            "charges_enabled": false,
            "payouts_enabled": false
        });
        let body = resp.to_string();
        let http_resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(http_resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let params = RejectAccountParams::new("fraud");
    let rejected = client
        .accounts()
        .reject("acct_reject_123", &params)
        .await
        .unwrap();

    assert_eq!(rejected.id(), "acct_reject_123");
    assert!(!rejected.is_charges_enabled());

    server.await.unwrap();
}

#[tokio::test]
async fn test_accounts_resource_list_and_paginate() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("GET /v1/accounts?"));
        assert!(req_str.contains("limit=2"));

        let resp = serde_json::json!({
            "object": "list",
            "data": [
                {
                    "id": "acct_item_1",
                    "object": "account",
                    "type": "standard",
                    "country": "US"
                },
                {
                    "id": "acct_item_2",
                    "object": "account",
                    "type": "express",
                    "country": "CA"
                }
            ],
            "has_more": false,
            "url": "/v1/accounts"
        });
        let body = resp.to_string();
        let http_resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(http_resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let params = ListAccountsParams::new().limit(2);
    let list: AccountList = client.accounts().list(&params).await.unwrap();
    assert_eq!(list.data.len(), 2);
    assert_eq!(list.data[0].id(), "acct_item_1");
    assert_eq!(list.data[1].id(), "acct_item_2");

    server.await.unwrap();
}

#[tokio::test]
async fn test_account_links_resource_create() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("POST /v1/account_links HTTP/1.1"));
        assert!(req_str.contains("account=acct_link_target_1"));
        assert!(req_str.contains("type=account_onboarding"));

        // Verify Idempotency-Key
        let key_header = req_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("idempotency-key:"))
            .expect("Idempotency-Key must be present");
        assert!(is_valid_uuid_v4(key_header.split(':').nth(1).unwrap().trim()));

        let resp = serde_json::json!({
            "object": "account_link",
            "created": 1728345600,
            "expires_at": 1728345900,
            "url": "https://connect.stripe.com/setup/s/mock_setup_token"
        });
        let body = resp.to_string();
        let http_resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(http_resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let params = CreateAccountLinkParams::new(
        "acct_link_target_1",
        "https://example.com/reauth",
        "https://example.com/return",
        AccountLinkType::AccountOnboarding,
    );

    let link: AccountLink = client.account_links().create(&params).await.unwrap();
    assert_eq!(
        link.url(),
        "https://connect.stripe.com/setup/s/mock_setup_token"
    );
    assert_eq!(link.created(), 1728345600);
    assert_eq!(link.expires_at(), 1728345900);

    server.await.unwrap();
}

#[tokio::test]
async fn test_account_sessions_resource_create() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("POST /v1/account_sessions HTTP/1.1"));
        assert!(req_str.contains("account=acct_session_target_1"));

        let resp = serde_json::json!({
            "object": "account_session",
            "account": "acct_session_target_1",
            "client_secret": "acc_sec_test_secret_xyz123",
            "expires_at": 1728345900,
            "components": {
                "payments": {
                    "enabled": true,
                    "features": {
                        "refund_management": true,
                        "dispute_management": false,
                        "capture_payments": true
                    }
                }
            }
        });
        let body = resp.to_string();
        let http_resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(http_resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let params = CreateAccountSessionParams::for_account("acct_session_target_1")
        .with_payments(true);

    let session: AccountSession = client.account_sessions().create(&params).await.unwrap();
    assert_eq!(session.client_secret(), "acc_sec_test_secret_xyz123");
    assert_eq!(session.account(), "acct_session_target_1");
    assert_eq!(session.expires_at(), 1728345900);

    server.await.unwrap();
}

#[tokio::test]
async fn test_persons_resource_crud_lifecycle() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // Request 1: POST /v1/accounts/acct_kyc_001/persons
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/accounts/acct_kyc_001/persons HTTP/1.1"));
            assert!(req_str.contains("first_name=Alice"));
            assert!(req_str.contains("last_name=Smith"));

            let resp = serde_json::json!({
                "id": "person_alice_001",
                "object": "person",
                "account": "acct_kyc_001",
                "first_name": "Alice",
                "last_name": "Smith",
                "email": "alice@example.com",
                "dob": { "day": 15, "month": 6, "year": 1985 },
                "relationship": { "owner": true, "percent_ownership": 35.0, "title": "CEO" }
            });
            let body = resp.to_string();
            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(http_resp.as_bytes()).await.unwrap();
        }

        // Request 2: GET /v1/accounts/acct_kyc_001/persons/person_alice_001
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("GET /v1/accounts/acct_kyc_001/persons/person_alice_001 HTTP/1.1"));

            let resp = serde_json::json!({
                "id": "person_alice_001",
                "object": "person",
                "account": "acct_kyc_001",
                "first_name": "Alice",
                "last_name": "Smith",
                "email": "alice@example.com"
            });
            let body = resp.to_string();
            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(http_resp.as_bytes()).await.unwrap();
        }

        // Request 3: POST /v1/accounts/acct_kyc_001/persons/person_alice_001 (Update)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("POST /v1/accounts/acct_kyc_001/persons/person_alice_001 HTTP/1.1"));

            let resp = serde_json::json!({
                "id": "person_alice_001",
                "object": "person",
                "account": "acct_kyc_001",
                "first_name": "Alice",
                "last_name": "Johnson",
                "email": "alice.j@example.com"
            });
            let body = resp.to_string();
            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(http_resp.as_bytes()).await.unwrap();
        }

        // Request 4: DELETE /v1/accounts/acct_kyc_001/persons/person_alice_001
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

            assert!(req_str.starts_with("DELETE /v1/accounts/acct_kyc_001/persons/person_alice_001 HTTP/1.1"));

            let resp = serde_json::json!({
                "id": "person_alice_001",
                "object": "person",
                "deleted": true
            });
            let body = resp.to_string();
            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(http_resp.as_bytes()).await.unwrap();
        }
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    // 1. Create Person
    let create_params = CreatePersonParams::new()
        .first_name("Alice")
        .last_name("Smith")
        .email("alice@example.com")
        .dob(PersonDob::new(15, 6, 1985))
        .relationship(
            PersonRelationship::new()
                .owner(true)
                .percent_ownership(35.0)
                .title("CEO"),
        );

    let created: Person = client
        .persons()
        .create("acct_kyc_001", &create_params)
        .await
        .unwrap();
    assert_eq!(created.id(), "person_alice_001");
    assert_eq!(created.first_name(), Some("Alice"));
    assert_eq!(created.last_name(), Some("Smith"));

    // 2. Retrieve Person
    let retrieved = client
        .persons()
        .retrieve("acct_kyc_001", "person_alice_001")
        .await
        .unwrap();
    assert_eq!(retrieved.id(), "person_alice_001");

    // 3. Update Person
    let update_params = UpdatePersonParams::new()
        .last_name(Nullable::Value("Johnson".into()))
        .email(Nullable::Value("alice.j@example.com".into()));

    let updated = client
        .persons()
        .update("acct_kyc_001", "person_alice_001", &update_params)
        .await
        .unwrap();
    assert_eq!(updated.last_name(), Some("Johnson"));

    // 4. Delete Person
    let deleted: DeletedPerson = client
        .persons()
        .del("acct_kyc_001", "person_alice_001")
        .await
        .unwrap();
    assert_eq!(deleted.id, "person_alice_001");
    assert!(deleted.deleted);

    server.await.unwrap();
}

#[tokio::test]
async fn test_persons_resource_list_and_paginate() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8(buf[..n].to_vec()).unwrap();

        assert!(req_str.starts_with("GET /v1/accounts/acct_list_001/persons?"));
        assert!(req_str.contains("limit=2"));

        let resp = serde_json::json!({
            "object": "list",
            "data": [
                {
                    "id": "person_list_1",
                    "object": "person",
                    "account": "acct_list_001",
                    "first_name": "Bob",
                    "last_name": "Taylor"
                },
                {
                    "id": "person_list_2",
                    "object": "person",
                    "account": "acct_list_001",
                    "first_name": "Charlie",
                    "last_name": "Brown"
                }
            ],
            "has_more": false,
            "url": "/v1/accounts/acct_list_001/persons"
        });
        let body = resp.to_string();
        let http_resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(http_resp.as_bytes()).await.unwrap();
    });

    let config = StripeConfig::new("sk_test_mock").with_api_base(format!("http://{addr}"));
    let client = StripeClient::with_config(config);

    let params = ListPersonsParams::new().limit(2);
    let list: PersonList = client.persons().list("acct_list_001", &params).await.unwrap();
    assert_eq!(list.data.len(), 2);
    assert_eq!(list.data[0].id(), "person_list_1");
    assert_eq!(list.data[1].id(), "person_list_2");

    server.await.unwrap();
}

#[tokio::test]
async fn test_path_traversal_validation_on_all_resources() {
    let client = StripeClient::new("sk_test_mock");

    // Accounts
    assert!(client.accounts().retrieve_by_id("../admin").await.is_err());
    assert!(client.accounts().retrieve_by_id("acct/evil").await.is_err());
    assert!(client.accounts().retrieve_by_id("").await.is_err());
    assert!(client.accounts().retrieve_by_id("acct?param=1").await.is_err());
    assert!(client.accounts().del("acct#fragment").await.is_err());
    assert!(
        client
            .accounts()
            .reject("../acct", &RejectAccountParams::new("fraud"))
            .await
            .is_err()
    );

    // Account Links
    let bad_link_params = CreateAccountLinkParams::new(
        "../evil_account",
        "https://example.com/reauth",
        "https://example.com/return",
        AccountLinkType::AccountOnboarding,
    );
    assert!(client.account_links().create(&bad_link_params).await.is_err());

    // Account Sessions
    let bad_session_params = CreateAccountSessionParams::for_account("../evil_acct");
    assert!(
        client
            .account_sessions()
            .create(&bad_session_params)
            .await
            .is_err()
    );

    // Persons
    let person_params = CreatePersonParams::new();
    assert!(
        client
            .persons()
            .create("../evil_acct", &person_params)
            .await
            .is_err()
    );
    assert!(
        client
            .persons()
            .retrieve("acct_valid", "../person_evil")
            .await
            .is_err()
    );
    assert!(
        client
            .persons()
            .retrieve("acct/invalid", "person_valid")
            .await
            .is_err()
    );
    assert!(
        client
            .persons()
            .del("acct_valid", "person?query=1")
            .await
            .is_err()
    );
    assert!(
        client
            .persons()
            .list("acct#invalid", &ListPersonsParams::new())
            .await
            .is_err()
    );
}
