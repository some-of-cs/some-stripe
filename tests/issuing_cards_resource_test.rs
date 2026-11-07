//! Integration and wire tests for Stripe Issuing Cards & Cardholders resource (US-7.2.0 Unit 1).
//!
//! Verifies:
//! - AC-US-7.2.0-01: Virtual card creation with daily spending controls.
//! - PCI-DSS SAQ A PAN isolation invariants (zero 16-digit card numbers in memory).
//! - Bank Secrecy Act (BSA) & FinCEN customer due diligence (individual DOB, billing, company tax ID).
//! - IssuingCardsResource CRUD (create, retrieve, update, list).
//! - IssuingCardholdersResource CRUD (create, retrieve, update, list).
//! - Header forwarding (`Stripe-Account`, `Idempotency-Key`).

use some_stripe::{
    CreateCardParams, CreateCardholderParams, IssuingCardAddress, IssuingCardCancellationReason,
    IssuingCardStatus, IssuingCardType, IssuingCardholderBillingParams,
    IssuingCardholderCompanyParams, IssuingCardholderIndividualParams, IssuingCardholderStatus,
    IssuingCardholderType, IssuingDateOfBirth, ListIssuingCardsParams, SpendingControlsParams,
    SpendingLimitInterval, SpendingLimitParams, StripeClient, StripeConfig, UpdateCardParams,
    UpdateCardholderParams,
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
async fn test_ac_us_7_2_0_01_create_virtual_card_with_daily_spending_controls() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();

        // Verify POST to /v1/issuing/cards
        assert!(wire.starts_with("POST /v1/issuing/cards HTTP/1.1"));
        assert!(wire.contains("cardholder=ich_123"));
        assert!(wire.contains("currency=usd"));
        assert!(wire.contains("type=virtual"));
        // Check spending controls form parameters (indexed bracket format)
        assert!(
            wire.contains("spending_controls%5Bspending_limits%5D%5B0%5D%5Bamount%5D=50000")
                || wire.contains("spending_controls[spending_limits][0][amount]=50000")
        );
        assert!(
            wire.contains("spending_controls%5Bspending_limits%5D%5B0%5D%5Binterval%5D=daily")
                || wire.contains("spending_controls[spending_limits][0][interval]=daily")
        );

        let resp = r#"{
            "id": "ic_123",
            "object": "issuing.card",
            "brand": "Visa",
            "cardholder": "ich_123",
            "created": 1700000000,
            "currency": "usd",
            "exp_month": 12,
            "exp_year": 2028,
            "last4": "4242",
            "livemode": false,
            "spending_controls": {
                "allowed_categories": null,
                "blocked_categories": null,
                "spending_limits": [
                    {
                        "amount": 50000,
                        "categories": null,
                        "interval": "daily"
                    }
                ],
                "spending_limits_currency": "usd"
            },
            "status": "active",
            "type": "virtual"
        }"#;
        write_json_response(&mut socket, resp).await;
    });

    let config = StripeConfig::new("sk_test_issuing").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    // Gherkin AC-US-7.2.0-01 scenario execution
    let params = CreateCardParams {
        cardholder: "ich_123".into(),
        currency: "usd".into(),
        r#type: "virtual".into(),
        spending_controls: Some(SpendingControlsParams {
            spending_limits: Some(vec![SpendingLimitParams {
                amount: 50000,
                interval: "daily".into(),
                ..Default::default()
            }]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let card = client.issuing().cards().create(&params).await.unwrap();

    assert_eq!(card.id, "ic_123");
    assert_eq!(card.brand, "Visa");
    assert_eq!(card.r#type, IssuingCardType::Virtual);
    assert_eq!(card.status, IssuingCardStatus::Active);
    assert!(card.is_pan_isolated());
    assert_eq!(card.last4, "4242");
    assert_eq!(card.daily_spending_limit(), Some(50000));

    server.await.unwrap();
}

#[tokio::test]
async fn test_issuing_cards_full_crud_and_account_scoping() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Create Card with idempotency and account header
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/issuing/cards HTTP/1.1"));
        assert!(
            wire.to_lowercase()
                .contains("stripe-account: acct_connected_test")
        );
        assert!(
            wire.to_lowercase()
                .contains("idempotency-key: idem_card_create_001")
        );

        let card_json = r#"{
            "id": "ic_crud_1",
            "object": "issuing.card",
            "brand": "Mastercard",
            "cardholder": "ich_abc",
            "created": 1700000000,
            "currency": "usd",
            "exp_month": 10,
            "exp_year": 2029,
            "last4": "9999",
            "livemode": false,
            "status": "active",
            "type": "virtual"
        }"#;
        write_json_response(&mut socket, card_json).await;

        // 2. Retrieve Card
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/issuing/cards/ic_crud_1 HTTP/1.1"));
        assert!(
            wire.to_lowercase()
                .contains("stripe-account: acct_connected_test")
        );
        write_json_response(&mut socket, card_json).await;

        // 3. Update Card (cancel)
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/issuing/cards/ic_crud_1 HTTP/1.1"));
        assert!(wire.contains("status=canceled"));
        assert!(wire.contains("cancellation_reason=lost"));

        let updated_json = r#"{
            "id": "ic_crud_1",
            "object": "issuing.card",
            "brand": "Mastercard",
            "cardholder": "ich_abc",
            "cancellation_reason": "lost",
            "created": 1700000000,
            "currency": "usd",
            "exp_month": 10,
            "exp_year": 2029,
            "last4": "9999",
            "livemode": false,
            "status": "canceled",
            "type": "virtual"
        }"#;
        write_json_response(&mut socket, updated_json).await;

        // 4. List Cards
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/issuing/cards?"));
        assert!(wire.contains("cardholder=ich_abc"));
        assert!(wire.contains("status=active"));

        let list_json = r#"{
            "object": "list",
            "data": [
                {
                    "id": "ic_crud_1",
                    "object": "issuing.card",
                    "brand": "Mastercard",
                    "cardholder": "ich_abc",
                    "created": 1700000000,
                    "currency": "usd",
                    "exp_month": 10,
                    "exp_year": 2029,
                    "last4": "9999",
                    "livemode": false,
                    "status": "active",
                    "type": "virtual"
                }
            ],
            "has_more": false,
            "url": "/v1/issuing/cards"
        }"#;
        write_json_response(&mut socket, list_json).await;
    });

    let config = StripeConfig::new("sk_test_issuing").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);
    let cards_resource = client.issuing_cards().with_account("acct_connected_test");

    // 1. Create
    let create_params = CreateCardParams {
        cardholder: "ich_abc".into(),
        currency: "usd".into(),
        r#type: IssuingCardType::Virtual,
        idempotency_key: Some("idem_card_create_001".into()),
        ..Default::default()
    };
    let card = cards_resource.create(&create_params).await.unwrap();
    assert_eq!(card.id, "ic_crud_1");
    assert_eq!(card.status, IssuingCardStatus::Active);

    // 2. Retrieve
    let retrieved = cards_resource.retrieve("ic_crud_1").await.unwrap();
    assert_eq!(retrieved.id, "ic_crud_1");

    // 3. Update
    let update_params = UpdateCardParams {
        status: Some(IssuingCardStatus::Canceled),
        cancellation_reason: Some(IssuingCardCancellationReason::Lost),
        ..Default::default()
    };
    let updated = cards_resource
        .update("ic_crud_1", &update_params)
        .await
        .unwrap();
    assert_eq!(updated.status, IssuingCardStatus::Canceled);
    assert_eq!(
        updated.cancellation_reason,
        Some(IssuingCardCancellationReason::Lost)
    );

    // 4. List
    let list_params = ListIssuingCardsParams {
        cardholder: Some("ich_abc".into()),
        status: Some(IssuingCardStatus::Active),
        limit: Some(10),
        ..Default::default()
    };
    let list = cards_resource.list(Some(&list_params)).await.unwrap();
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id, "ic_crud_1");

    server.await.unwrap();
}

#[tokio::test]
async fn test_issuing_cardholders_full_crud_and_aml_compliance() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Create Individual Cardholder
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/issuing/cardholders HTTP/1.1"));
        assert!(wire.contains("name=Jenny+Rosen") || wire.contains("name=Jenny%20Rosen"));
        assert!(wire.contains("type=individual"));

        let ch_json = r#"{
            "id": "ich_jenny",
            "object": "issuing.cardholder",
            "billing": {
                "address": {
                    "city": "San Francisco",
                    "country": "US",
                    "line1": "123 Market St",
                    "postal_code": "94105",
                    "state": "CA"
                }
            },
            "created": 1700000000,
            "email": "jenny@example.com",
            "individual": {
                "first_name": "Jenny",
                "last_name": "Rosen",
                "dob": {
                    "day": 1,
                    "month": 1,
                    "year": 1980
                }
            },
            "livemode": false,
            "name": "Jenny Rosen",
            "phone_number": "+15555555555",
            "status": "active",
            "type": "individual"
        }"#;
        write_json_response(&mut socket, ch_json).await;

        // 2. Retrieve Cardholder
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/issuing/cardholders/ich_jenny HTTP/1.1"));
        write_json_response(&mut socket, ch_json).await;

        // 3. Update Cardholder
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/issuing/cardholders/ich_jenny HTTP/1.1"));
        assert!(wire.contains("status=inactive"));

        let updated_ch_json = r#"{
            "id": "ich_jenny",
            "object": "issuing.cardholder",
            "billing": {
                "address": {
                    "city": "San Francisco",
                    "country": "US",
                    "line1": "123 Market St",
                    "postal_code": "94105",
                    "state": "CA"
                }
            },
            "created": 1700000000,
            "email": "jenny@example.com",
            "livemode": false,
            "name": "Jenny Rosen",
            "status": "inactive",
            "type": "individual"
        }"#;
        write_json_response(&mut socket, updated_ch_json).await;

        // 4. List Cardholders
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/issuing/cardholders HTTP/1.1"));

        let list_json = r#"{
            "object": "list",
            "data": [
                {
                    "id": "ich_jenny",
                    "object": "issuing.cardholder",
                    "billing": {
                        "address": {
                            "city": "San Francisco",
                            "country": "US",
                            "line1": "123 Market St",
                            "postal_code": "94105",
                            "state": "CA"
                        }
                    },
                    "created": 1700000000,
                    "livemode": false,
                    "name": "Jenny Rosen",
                    "status": "active",
                    "type": "individual"
                }
            ],
            "has_more": false,
            "url": "/v1/issuing/cardholders"
        }"#;
        write_json_response(&mut socket, list_json).await;
    });

    let config = StripeConfig::new("sk_test_issuing").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);
    let cardholders_resource = client.issuing_cardholders();

    // 1. Create Cardholder
    let create_params = CreateCardholderParams {
        name: "Jenny Rosen".into(),
        billing: IssuingCardholderBillingParams {
            address: IssuingCardAddress {
                line1: "123 Market St".into(),
                line2: None,
                city: "San Francisco".into(),
                state: Some("CA".into()),
                postal_code: "94105".into(),
                country: "US".into(),
            },
        },
        r#type: IssuingCardholderType::Individual,
        email: Some("jenny@example.com".into()),
        phone_number: Some("+15555555555".into()),
        individual: Some(IssuingCardholderIndividualParams {
            first_name: "Jenny".into(),
            last_name: "Rosen".into(),
            dob: Some(IssuingDateOfBirth {
                day: 1,
                month: 1,
                year: 1980,
            }),
            card_issuing: None,
            verification: None,
        }),
        ..Default::default()
    };
    let ch = cardholders_resource.create(&create_params).await.unwrap();
    assert_eq!(ch.id, "ich_jenny");
    assert_eq!(ch.name, "Jenny Rosen");
    assert_eq!(ch.status, IssuingCardholderStatus::Active);

    // Verify KYC data
    let ind = ch.individual.unwrap();
    assert_eq!(ind.first_name.as_deref(), Some("Jenny"));
    assert_eq!(
        ind.dob,
        Some(IssuingDateOfBirth {
            day: 1,
            month: 1,
            year: 1980
        })
    );

    // 2. Retrieve Cardholder
    let retrieved = cardholders_resource.retrieve("ich_jenny").await.unwrap();
    assert_eq!(retrieved.id, "ich_jenny");

    // 3. Update Cardholder
    let update_params = UpdateCardholderParams {
        status: Some(IssuingCardholderStatus::Inactive),
        ..Default::default()
    };
    let updated = cardholders_resource
        .update("ich_jenny", &update_params)
        .await
        .unwrap();
    assert_eq!(updated.status, IssuingCardholderStatus::Inactive);

    // 4. List Cardholders
    let list = cardholders_resource.list(None).await.unwrap();
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id, "ich_jenny");

    server.await.unwrap();
}

#[tokio::test]
async fn test_company_cardholder_and_spending_limits() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/issuing/cardholders HTTP/1.1"));
        assert!(wire.contains("type=company"));

        let ch_company_json = r#"{
            "id": "ich_corp_1",
            "object": "issuing.cardholder",
            "billing": {
                "address": {
                    "city": "New York",
                    "country": "US",
                    "line1": "100 Broadway",
                    "postal_code": "10005",
                    "state": "NY"
                }
            },
            "company": {
                "tax_id_provided": true
            },
            "created": 1700000000,
            "livemode": false,
            "name": "Acme Global Inc",
            "spending_controls": {
                "allowed_categories": null,
                "blocked_categories": null,
                "spending_limits": [
                    {
                        "amount": 1000000,
                        "categories": null,
                        "interval": "monthly"
                    }
                ],
                "spending_limits_currency": "usd"
            },
            "status": "active",
            "type": "company"
        }"#;
        write_json_response(&mut socket, ch_company_json).await;
    });

    let config = StripeConfig::new("sk_test_issuing").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);

    let create_params = CreateCardholderParams {
        name: "Acme Global Inc".into(),
        billing: IssuingCardholderBillingParams {
            address: IssuingCardAddress {
                line1: "100 Broadway".into(),
                line2: None,
                city: "New York".into(),
                state: Some("NY".into()),
                postal_code: "10005".into(),
                country: "US".into(),
            },
        },
        r#type: IssuingCardholderType::Company,
        company: Some(IssuingCardholderCompanyParams {
            tax_id: Some("12-3456789".into()),
        }),
        spending_controls: Some(SpendingControlsParams {
            spending_limits: Some(vec![SpendingLimitParams {
                amount: 1000000,
                interval: SpendingLimitInterval::Monthly,
                categories: None,
            }]),
            spending_limits_currency: Some("usd".into()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let ch = client
        .issuing_cardholders()
        .create(&create_params)
        .await
        .unwrap();
    assert_eq!(ch.id, "ich_corp_1");
    assert_eq!(ch.r#type, IssuingCardholderType::Company);
    assert_eq!(ch.company.unwrap().tax_id_provided, Some(true));

    let limits = ch.spending_controls.unwrap().spending_limits.unwrap();
    assert_eq!(limits.len(), 1);
    assert_eq!(limits[0].amount, 1000000);
    assert_eq!(limits[0].interval, SpendingLimitInterval::Monthly);

    server.await.unwrap();
}
