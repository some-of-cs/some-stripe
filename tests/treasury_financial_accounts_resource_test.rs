//! Integration and wire tests for Treasury Financial Accounts resource (US-7.2.0 Unit 3).
//!
//! Verifies CRUD endpoints, idempotency key generation, multi-tenant `Stripe-Account` header scoping,
//! FDIC insurance balance mechanics, FinCEN/BSA compliance, and PCI-DSS PAN isolation.

use some_stripe::{
    AbaRecord, CloseFinancialAccountParams, CreateFinancialAccountParams, FeatureRequestParam,
    FinancialAccountClosedReason, FinancialAccountFeaturesParams, FinancialAccountStatus,
    FinancialAccountValidationError, FinancialAddress, FinancialAddresses,
    FinancialAddressesFeatureParams, ListFinancialAccountsParams, StripeClient, StripeConfig,
    UpdateFinancialAccountParams,
};
use std::collections::HashMap;
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
async fn test_treasury_financial_accounts_crud_and_close() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        // 1. Create Financial Account
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = vec![0; 8192];
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/treasury/financial_accounts HTTP/1.1"));
        assert!(
            wire.contains("supported_currencies%5B0%5D=usd")
                || wire.contains("supported_currencies[0]=usd")
        );
        assert!(
            wire.to_lowercase()
                .contains("idempotency-key: fa_create_idem_123")
        );
        assert!(
            wire.to_lowercase()
                .contains("stripe-account: acct_connected_test")
        );

        let resp_create = r#"{
            "id": "fa_123",
            "object": "treasury.financial_account",
            "active_features": ["deposit_insurance", "financial_addresses.aba", "inbound_transfers.ach"],
            "balance": {
                "cash": {"usd": 50000000},
                "inbound_pending": {"usd": 500000},
                "outbound_pending": {"usd": 200000}
            },
            "country": "US",
            "created": 1728345600,
            "financial_addresses": [
                {
                    "address_type": "aba",
                    "aba": {
                        "account_number_last4": "6789",
                        "routing_number": "011000015",
                        "bank_name": "Evolve Bank & Trust"
                    }
                }
            ],
            "livemode": false,
            "metadata": {"env": "integration_test"},
            "pending_features": [],
            "restricted_features": [],
            "status": "open",
            "supported_currencies": ["usd"]
        }"#;
        write_json_response(&mut socket, resp_create).await;

        // 2. Retrieve Financial Account
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/treasury/financial_accounts/fa_123 HTTP/1.1"));
        assert!(
            wire.to_lowercase()
                .contains("stripe-account: acct_connected_test")
        );
        write_json_response(&mut socket, resp_create).await;

        // 3. Update Financial Account
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/treasury/financial_accounts/fa_123 HTTP/1.1"));
        assert!(
            wire.contains("metadata%5Bteam%5D=treasury_ops")
                || wire.contains("metadata[team]=treasury_ops")
        );

        let resp_update = r#"{
            "id": "fa_123",
            "object": "treasury.financial_account",
            "active_features": ["deposit_insurance", "financial_addresses.aba", "inbound_transfers.ach"],
            "balance": {
                "cash": {"usd": 50000000},
                "inbound_pending": {"usd": 500000},
                "outbound_pending": {"usd": 200000}
            },
            "country": "US",
            "created": 1728345600,
            "financial_addresses": [
                {
                    "address_type": "aba",
                    "aba": {
                        "account_number_last4": "6789",
                        "routing_number": "011000015",
                        "bank_name": "Evolve Bank & Trust"
                    }
                }
            ],
            "livemode": false,
            "metadata": {"team": "treasury_ops"},
            "pending_features": [],
            "restricted_features": [],
            "status": "open",
            "supported_currencies": ["usd"]
        }"#;
        write_json_response(&mut socket, resp_update).await;

        // 4. List Financial Accounts
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("GET /v1/treasury/financial_accounts?"));
        assert!(wire.contains("limit=10"));
        assert!(wire.contains("status=open"));

        let resp_list = r#"{
            "object": "list",
            "data": [
                {
                    "id": "fa_123",
                    "object": "treasury.financial_account",
                    "active_features": ["deposit_insurance"],
                    "balance": {"cash": {"usd": 1000000}, "inbound_pending": {}, "outbound_pending": {}},
                    "country": "US",
                    "created": 1728345600,
                    "financial_addresses": [],
                    "livemode": false,
                    "pending_features": [],
                    "restricted_features": [],
                    "status": "open",
                    "supported_currencies": ["usd"]
                }
            ],
            "has_more": false,
            "url": "/v1/treasury/financial_accounts"
        }"#;
        write_json_response(&mut socket, resp_list).await;

        // 5. Close Financial Account
        let (mut socket, _) = listener.accept().await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        let wire = String::from_utf8(buf[..n].to_vec()).unwrap();
        assert!(wire.starts_with("POST /v1/treasury/financial_accounts/fa_123/close HTTP/1.1"));
        assert!(
            wire.contains("forwarding_settings%5Bpayment_method%5D=pm_external_123")
                || wire.contains("forwarding_settings[payment_method]=pm_external_123")
        );

        let resp_close = r#"{
            "id": "fa_123",
            "object": "treasury.financial_account",
            "active_features": [],
            "balance": {"cash": {"usd": 0}, "inbound_pending": {}, "outbound_pending": {}},
            "country": "US",
            "created": 1728345600,
            "financial_addresses": [],
            "livemode": false,
            "pending_features": [],
            "restricted_features": [],
            "status": "closed",
            "status_details": {
                "closed": {
                    "reasons": ["closed_by_platform"]
                }
            },
            "supported_currencies": ["usd"]
        }"#;
        write_json_response(&mut socket, resp_close).await;
    });

    let config = StripeConfig::new("sk_test_treasury").with_api_base(format!("http://{address}"));
    let client = StripeClient::with_config(config);
    let fa_resource = client
        .treasury_financial_accounts()
        .with_account("acct_connected_test");

    // 1. Create
    let create_params = CreateFinancialAccountParams::new(vec!["usd".into()])
        .with_features(FinancialAccountFeaturesParams {
            deposit_insurance: Some(FeatureRequestParam { requested: true }),
            financial_addresses: Some(FinancialAddressesFeatureParams {
                aba: Some(FeatureRequestParam { requested: true }),
            }),
            ..Default::default()
        })
        .with_idempotency_key("fa_create_idem_123");

    let fa = fa_resource.create(&create_params).await.unwrap();
    assert_eq!(fa.id, "fa_123");
    assert!(fa.is_open());
    assert!(!fa.is_closed());
    assert!(fa.has_active_feature("deposit_insurance"));
    assert_eq!(fa.routing_number(), Some("011000015"));
    assert_eq!(fa.account_number_last4(), Some("6789"));

    // Check FDIC insurance balance mechanics ($500,000 cash balance)
    assert_eq!(fa.balance.cash_amount("usd"), 50_000_000); // $500k
    assert_eq!(fa.balance.fdic_insured_balance("usd"), 25_000_000); // capped at $250k
    assert_eq!(fa.balance.fdic_excess_balance("usd"), 25_000_000); // excess $250k
    assert!(!fa.balance.is_within_fdic_limit("usd"));
    assert_eq!(fa.balance.net_cash("usd"), 50_300_000); // 500k + 5k - 2k

    // 2. Retrieve
    let retrieved = fa_resource.retrieve("fa_123").await.unwrap();
    assert_eq!(retrieved.id, "fa_123");
    assert_eq!(retrieved.supported_currencies, vec!["usd"]);

    // 3. Update
    let mut update_meta = HashMap::new();
    update_meta.insert("team".into(), "treasury_ops".into());
    let update_params = UpdateFinancialAccountParams::new()
        .with_metadata(update_meta)
        .with_idempotency_key("fa_update_idem_456");

    let updated = fa_resource.update("fa_123", &update_params).await.unwrap();
    assert_eq!(updated.id, "fa_123");
    assert_eq!(
        updated.metadata.as_ref().and_then(|m| m.get("team")),
        Some(&"treasury_ops".to_string())
    );

    // 4. List
    let list_params = ListFinancialAccountsParams::new()
        .with_limit(10)
        .with_status(FinancialAccountStatus::Open);
    let list_result = fa_resource.list(Some(&list_params)).await.unwrap();
    assert_eq!(list_result.data.len(), 1);
    assert_eq!(list_result.data[0].id, "fa_123");
    assert!(!list_result.has_more);

    // 5. Close
    let close_params = CloseFinancialAccountParams::new()
        .with_forwarding_payment_method("pm_external_123")
        .with_idempotency_key("fa_close_idem_789");

    let closed = fa_resource
        .close("fa_123", Some(&close_params))
        .await
        .unwrap();
    assert_eq!(closed.id, "fa_123");
    assert!(closed.is_closed());
    assert!(!closed.is_open());
    let closed_reasons = closed
        .status_details
        .as_ref()
        .and_then(|d| d.closed.as_ref())
        .map(|c| c.reasons.as_slice())
        .unwrap_or(&[]);
    assert_eq!(
        closed_reasons,
        &[FinancialAccountClosedReason::ClosedByPlatform]
    );

    server.await.unwrap();
}

#[test]
fn test_fincen_bsa_compliance_and_aba_validation() {
    let valid_aba = AbaRecord {
        account_number: Some("123456789012".into()),
        account_number_last4: "9012".into(),
        routing_number: "011000015".into(),
        bank_name: "Evolve Bank & Trust".into(),
    };
    assert!(valid_aba.validate_compliance().is_ok());

    // Invalid ABA routing number (bad checksum)
    let bad_routing = AbaRecord {
        routing_number: "111111111".into(),
        ..valid_aba.clone()
    };
    assert!(matches!(
        bad_routing.validate_compliance(),
        Err(FinancialAccountValidationError::InvalidRoutingNumber { .. })
    ));

    // Invalid account last4 (not 4 digits)
    let bad_last4 = AbaRecord {
        account_number_last4: "12".into(),
        ..valid_aba.clone()
    };
    assert!(matches!(
        bad_last4.validate_compliance(),
        Err(FinancialAccountValidationError::InvalidAccountNumber { .. })
    ));

    // PCI-DSS PAN isolation violation: raw credit card number placed in bank account field
    let pan_leakage = AbaRecord {
        account_number: Some("4242424242424242".into()), // Valid Visa PAN via Luhn
        ..valid_aba.clone()
    };
    assert!(matches!(
        pan_leakage.validate_compliance(),
        Err(FinancialAccountValidationError::PanIsolationViolation)
    ));

    // Empty bank name
    let empty_bank = AbaRecord {
        bank_name: "   ".into(),
        ..valid_aba.clone()
    };
    assert!(matches!(
        empty_bank.validate_compliance(),
        Err(FinancialAccountValidationError::InvalidParameter {
            field: "bank_name",
            ..
        })
    ));
}

#[test]
fn test_financial_addresses_container() {
    let container = FinancialAddresses {
        addresses: vec![FinancialAddress {
            address_type: "aba".into(),
            aba: Some(AbaRecord {
                account_number: None,
                account_number_last4: "4321".into(),
                routing_number: "121000358".into(),
                bank_name: "Cross River Bank".into(),
            }),
        }],
    };

    assert_eq!(container.routing_number(), Some("121000358"));
    assert_eq!(container.account_number_last4(), Some("4321"));
    assert!(container.primary_aba().is_some());
    assert_eq!(
        container.primary_aba().unwrap().bank_name,
        "Cross River Bank"
    );
}

#[test]
fn test_borrowing_discipline_and_param_validation() {
    let empty_currencies = CreateFinancialAccountParams::new(vec![]);
    assert!(empty_currencies.validate().is_err());

    let invalid_currency = CreateFinancialAccountParams::new(vec!["US_DOLLAR".into()]);
    assert!(invalid_currency.validate().is_err());

    let list_params_invalid_limit = ListFinancialAccountsParams::new().with_limit(0);
    assert!(list_params_invalid_limit.validate().is_err());

    let list_params_invalid_limit_high = ListFinancialAccountsParams::new().with_limit(200);
    assert!(list_params_invalid_limit_high.validate().is_err());

    let close_params_invalid = CloseFinancialAccountParams {
        forwarding_settings: Some(some_stripe::ForwardingSettingsParams {
            financial_address: None,
            payment_method: None,
            r#type: None,
        }),
        idempotency_key: None,
    };
    assert!(close_params_invalid.validate().is_err());
}
