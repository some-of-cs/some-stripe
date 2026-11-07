//! Integration and wire tests for Stripe Terminal, Issuing, and Treasury facades (US-7.2.0 Unit 5).
//!
//! Verifies:
//! - AC-US-7.2.0-03: Generate Terminal connection token for smart reader with location binding.
//! - Readers registration, retrieval, listing, and cancel_action.
//! - Locations creation, retrieval, and listing.
//! - Connection token issuance and location scoping.
//! - Multi-tenant `Stripe-Account` header propagation.
//! - Facades `client.issuing()`, `client.treasury()`, and `client.terminal()` mounting on `StripeClient`.

use some_stripe::{
    CreateConnectionTokenParams, CreateLocationParams, CreateReaderParams,
    ListTerminalLocationsParams, ListTerminalReadersParams, StripeClient, StripeConfig,
    TerminalAddress,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn write_json_response(socket: &mut tokio::net::TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    socket.write_all(response.as_bytes()).await.unwrap();
}

#[tokio::test]
async fn test_terminal_locations_and_connection_token_ac_us_7_2_0_03() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let mut buf = vec![0; 8192];

        // 1. POST /v1/terminal/locations
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let n = socket.read(&mut buf).await.unwrap();
            let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(wire.starts_with("POST /v1/terminal/locations HTTP/1.1"));
            assert!(wire.contains("display_name=Downtown+Store"));
            assert!(wire.to_lowercase().contains("idempotency-key:"));

            let resp = r#"{
                "id": "loc_123",
                "object": "terminal.location",
                "display_name": "Downtown Store",
                "livemode": false
            }"#;
            write_json_response(&mut socket, resp).await;
        }

        // 2. POST /v1/terminal/connection_tokens (AC-US-7.2.0-03)
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let n = socket.read(&mut buf).await.unwrap();
            let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(wire.starts_with("POST /v1/terminal/connection_tokens HTTP/1.1"));
            assert!(wire.contains("location=loc_123"));
            assert!(wire.to_lowercase().contains("idempotency-key:"));

            let resp = r#"{
                "object": "terminal.connection_token",
                "secret": "pst_test_secret_ephemeral_xyz789",
                "location": "loc_123"
            }"#;
            write_json_response(&mut socket, resp).await;
        }

        // 3. GET /v1/terminal/locations/loc_123
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let n = socket.read(&mut buf).await.unwrap();
            let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(wire.starts_with("GET /v1/terminal/locations/loc_123 HTTP/1.1"));

            let resp = r#"{
                "id": "loc_123",
                "object": "terminal.location",
                "display_name": "Downtown Store",
                "livemode": false
            }"#;
            write_json_response(&mut socket, resp).await;
        }

        // 4. GET /v1/terminal/locations
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let n = socket.read(&mut buf).await.unwrap();
            let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(wire.starts_with("GET /v1/terminal/locations HTTP/1.1"));

            let resp = r#"{
                "object": "list",
                "data": [{
                    "id": "loc_123",
                    "object": "terminal.location",
                    "display_name": "Downtown Store",
                    "livemode": false
                }],
                "has_more": false,
                "url": "/v1/terminal/locations"
            }"#;
            write_json_response(&mut socket, resp).await;
        }
    });

    let config = StripeConfig::new("sk_test_terminal").with_api_base(format!("http://{}", address));
    let client = StripeClient::with_config(config);

    // 1. Create Location
    let loc_params = CreateLocationParams::new("Downtown Store")
        .with_address(TerminalAddress::new().with_city("San Francisco"));
    let location = client
        .terminal()
        .locations()
        .create(&loc_params)
        .await
        .expect("Create location");
    assert_eq!(location.id(), "loc_123");
    assert_eq!(location.display_name(), "Downtown Store");

    // 2. AC-US-7.2.0-03: Generate connection token bound to location
    let token_params = CreateConnectionTokenParams::for_location(location.id());
    let token = client
        .terminal()
        .connection_tokens()
        .create(&token_params)
        .await
        .expect("Create connection token");
    assert_eq!(token.secret(), "pst_test_secret_ephemeral_xyz789");
    assert_eq!(token.location(), Some("loc_123"));
    assert!(token.is_bound_to_location("loc_123"));

    // 3. Retrieve Location
    let retrieved_loc = client
        .terminal()
        .locations()
        .retrieve("loc_123")
        .await
        .expect("Retrieve location");
    assert_eq!(retrieved_loc.id(), "loc_123");

    // 4. List Locations
    let list_params = ListTerminalLocationsParams::new();
    let list = client
        .terminal()
        .locations()
        .list(&list_params)
        .await
        .expect("List locations");
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id(), "loc_123");

    server.await.unwrap();
}

#[tokio::test]
async fn test_terminal_readers_crud_and_cancel_action() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let mut buf = vec![0; 8192];

        // 1. POST /v1/terminal/readers
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let n = socket.read(&mut buf).await.unwrap();
            let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(wire.starts_with("POST /v1/terminal/readers HTTP/1.1"));
            assert!(wire.contains("registration_code=simulated-wpos-1"));
            assert!(wire.contains("label=Register+1"));
            assert!(wire.contains("location=loc_123"));
            assert!(wire.to_lowercase().contains("idempotency-key:"));
            assert!(
                wire.to_lowercase()
                    .contains("stripe-account: acct_connected_pos_99")
            );

            let resp = r#"{
                "id": "tmr_pos_456",
                "object": "terminal.reader",
                "device_type": "bbpos_wisepos_e",
                "label": "Register 1",
                "livemode": false,
                "location": "loc_123",
                "serial_number": "WPOS-456",
                "status": "online"
            }"#;
            write_json_response(&mut socket, resp).await;
        }

        // 2. GET /v1/terminal/readers/tmr_pos_456
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let n = socket.read(&mut buf).await.unwrap();
            let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(wire.starts_with("GET /v1/terminal/readers/tmr_pos_456 HTTP/1.1"));

            let resp = r#"{
                "id": "tmr_pos_456",
                "object": "terminal.reader",
                "device_type": "bbpos_wisepos_e",
                "label": "Register 1",
                "livemode": false,
                "location": "loc_123",
                "serial_number": "WPOS-456",
                "status": "online"
            }"#;
            write_json_response(&mut socket, resp).await;
        }

        // 3. GET /v1/terminal/readers?location=loc_123
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let n = socket.read(&mut buf).await.unwrap();
            let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(wire.starts_with("GET /v1/terminal/readers?location=loc_123 HTTP/1.1"));

            let resp = r#"{
                "object": "list",
                "data": [{
                    "id": "tmr_pos_456",
                    "object": "terminal.reader",
                    "device_type": "bbpos_wisepos_e",
                    "label": "Register 1",
                    "livemode": false,
                    "location": "loc_123",
                    "serial_number": "WPOS-456",
                    "status": "online"
                }],
                "has_more": false,
                "url": "/v1/terminal/readers"
            }"#;
            write_json_response(&mut socket, resp).await;
        }

        // 4. POST /v1/terminal/readers/tmr_pos_456/cancel_action
        {
            let (mut socket, _) = listener.accept().await.unwrap();
            let n = socket.read(&mut buf).await.unwrap();
            let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
            assert!(
                wire.starts_with("POST /v1/terminal/readers/tmr_pos_456/cancel_action HTTP/1.1")
            );
            assert!(wire.to_lowercase().contains("idempotency-key:"));

            let resp = r#"{
                "id": "tmr_pos_456",
                "object": "terminal.reader",
                "device_type": "bbpos_wisepos_e",
                "label": "Register 1",
                "livemode": false,
                "location": "loc_123",
                "serial_number": "WPOS-456",
                "status": "online"
            }"#;
            write_json_response(&mut socket, resp).await;
        }
    });

    let config = StripeConfig::new("sk_test_terminal").with_api_base(format!("http://{}", address));
    let client = StripeClient::with_config(config);

    // 1. Create reader with connected account scoping
    let params = CreateReaderParams::new("simulated-wpos-1")
        .with_label("Register 1")
        .with_location("loc_123");

    let reader = client
        .terminal()
        .readers()
        .with_account("acct_connected_pos_99")
        .create(&params)
        .await
        .expect("Create reader");

    assert_eq!(reader.id(), "tmr_pos_456");
    assert_eq!(reader.device_type(), "bbpos_wisepos_e");
    assert!(reader.is_online());
    assert!(reader.is_emv_certified());

    // 2. Retrieve reader
    let retrieved = client
        .terminal()
        .readers()
        .retrieve("tmr_pos_456")
        .await
        .expect("Retrieve reader");
    assert_eq!(retrieved.id(), "tmr_pos_456");

    // 3. List readers for location
    let list_params = ListTerminalReadersParams::for_location("loc_123");
    let readers_list = client
        .terminal()
        .readers()
        .list(&list_params)
        .await
        .expect("List readers");
    assert_eq!(readers_list.data.len(), 1);
    assert_eq!(readers_list.data[0].id(), "tmr_pos_456");

    // 4. Cancel action
    let cancelled = client
        .terminal()
        .readers()
        .cancel_action("tmr_pos_456")
        .await
        .expect("Cancel action");
    assert_eq!(cancelled.id(), "tmr_pos_456");

    server.await.unwrap();
}

#[tokio::test]
async fn test_facades_mounting_and_delegation() {
    let client = StripeClient::new("sk_test_mock_123");

    // Issuing facade
    let issuing = client.issuing();
    let _cards = issuing.cards();
    let _cardholders = issuing.cardholders();
    let _auths = issuing.authorizations();
    let _txns = issuing.transactions();
    assert_eq!(issuing.client().config.api_base, "https://api.stripe.com");

    // Treasury facade
    let treasury = client.treasury();
    let _accounts = treasury.financial_accounts();
    let _transfers = treasury.outbound_transfers();
    let _payments = treasury.outbound_payments();
    assert_eq!(treasury.client().config.api_base, "https://api.stripe.com");

    // Terminal facade
    let terminal = client.terminal();
    let _readers = terminal.readers();
    let _locations = terminal.locations();
    let _tokens = terminal.connection_tokens();
    assert_eq!(terminal.client().config.api_base, "https://api.stripe.com");
}

#[tokio::test]
async fn test_client_side_validation_guard_returns() {
    let client = StripeClient::new("sk_test_mock_123");

    // Invalid registration code
    let bad_reader = CreateReaderParams::new("  ");
    let err = client.terminal().readers().create(&bad_reader).await;
    assert!(err.is_err());

    // Invalid location display name
    let bad_loc = CreateLocationParams::new("");
    let err = client.terminal().locations().create(&bad_loc).await;
    assert!(err.is_err());

    // Invalid location ID for connection token
    let bad_token = CreateConnectionTokenParams::for_location("invalid_prefix_id");
    let err = client
        .terminal()
        .connection_tokens()
        .create(&bad_token)
        .await;
    assert!(err.is_err());
}
