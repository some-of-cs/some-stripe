//! Comprehensive contract and unit tests for Stripe Connect Account domain entities,
//! parameters, and serialization mechanics (Gate G0, G-Borrow, G-Law, G1, G4).

use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_types::{
    Account, AccountCapabilities, AccountCapabilitiesParams, AccountRequirements,
    AccountType, BusinessType, CapabilityStatus, CreateAccountParams, Nullable,
    UpdateAccountParams,
};
use std::collections::HashMap;

#[test]
fn test_account_type_serde_and_helpers() {
    // Assert lowercase serialization
    assert_eq!(serde_json::to_string(&AccountType::Standard).unwrap(), r#""standard""#);
    assert_eq!(serde_json::to_string(&AccountType::Express).unwrap(), r#""express""#);
    assert_eq!(serde_json::to_string(&AccountType::Custom).unwrap(), r#""custom""#);
    assert_eq!(serde_json::to_string(&AccountType::None).unwrap(), r#""none""#);

    // Assert deserialization
    assert_eq!(serde_json::from_str::<AccountType>(r#""standard""#).unwrap(), AccountType::Standard);
    assert_eq!(serde_json::from_str::<AccountType>(r#""express""#).unwrap(), AccountType::Express);
    assert_eq!(serde_json::from_str::<AccountType>(r#""custom""#).unwrap(), AccountType::Custom);

    // Helpers
    assert!(AccountType::Standard.is_standard());
    assert!(!AccountType::Standard.is_custom());
    assert!(AccountType::Express.is_express());
    assert!(AccountType::Custom.is_custom());
    assert_eq!(AccountType::Standard.as_str(), "standard");
}

#[test]
fn test_business_type_fincen_dac7_compliance() {
    assert_eq!(BusinessType::Individual.as_str(), "individual");
    assert_eq!(BusinessType::Company.as_str(), "company");
    assert_eq!(BusinessType::NonProfit.as_str(), "non_profit");
    assert_eq!(BusinessType::GovernmentEntity.as_str(), "government_entity");

    let val: BusinessType = serde_json::from_str(r#""company""#).unwrap();
    assert_eq!(val, BusinessType::Company);
}

#[test]
fn test_account_capabilities_and_status() {
    let mut caps = AccountCapabilities::new();
    assert!(!caps.is_card_payments_active());
    assert!(!caps.is_transfers_active());

    caps.card_payments = Some(CapabilityStatus::Active);
    caps.transfers = Some(CapabilityStatus::Pending);

    assert!(caps.is_card_payments_active());
    assert!(!caps.is_transfers_active());

    let json = serde_json::to_string(&caps).unwrap();
    assert!(json.contains(r#""card_payments":"active""#));
    assert!(json.contains(r#""transfers":"pending""#));
}

#[test]
fn test_account_requirements_verification() {
    let reqs = AccountRequirements::new()
        .with_currently_due(vec!["individual.verification.document".into()])
        .with_eventually_due(vec!["company.tax_id".into()])
        .with_disabled_reason("requirements.past_due");

    assert!(!reqs.is_complete());
    assert!(reqs.has_disabled_reason());
    assert_eq!(reqs.disabled_reason(), Some("requirements.past_due"));
    assert_eq!(reqs.currently_due().len(), 1);
    assert_eq!(reqs.eventually_due().len(), 1);
}

#[test]
fn test_create_account_params_form_serialization() {
    let mut metadata = HashMap::new();
    metadata.insert("platform_user_id".into(), "user_9918".into());

    let params = CreateAccountParams::new()
        .account_type(AccountType::Custom)
        .country("US")
        .email("merchant@example.com")
        .business_type(BusinessType::Company)
        .default_currency("usd")
        .capabilities(AccountCapabilitiesParams::new().card_payments(CapabilityStatus::Requested))
        .metadata(metadata);

    // Bracket form serialization check (Dual-Protocol)
    let form_pairs = to_form_params(&params).expect("Flattening to form params must succeed");
    let form_map: HashMap<String, String> = form_pairs.into_iter().collect();

    assert_eq!(form_map.get("type").map(String::as_str), Some("custom"));
    assert_eq!(form_map.get("country").map(String::as_str), Some("US"));
    assert_eq!(form_map.get("email").map(String::as_str), Some("merchant@example.com"));
    assert_eq!(form_map.get("business_type").map(String::as_str), Some("company"));
    assert_eq!(form_map.get("default_currency").map(String::as_str), Some("usd"));
    assert_eq!(form_map.get("capabilities[card_payments]").map(String::as_str), Some("requested"));
    assert_eq!(form_map.get("metadata[platform_user_id]").map(String::as_str), Some("user_9918"));
}

#[test]
fn test_update_account_params_nullable_mechanics() {
    // 1. Omit state
    let omit_params = UpdateAccountParams::new();
    let json = serde_json::to_string(&omit_params).unwrap();
    assert_eq!(json, "{}");

    // 2. Null state (clearing fields)
    let clear_params = UpdateAccountParams::new()
        .email(Nullable::Null)
        .default_currency(Nullable::Null);
    let clear_json = serde_json::to_string(&clear_params).unwrap();
    assert!(clear_json.contains(r#""email":null"#));
    assert!(clear_json.contains(r#""default_currency":null"#));

    // 3. Value state
    let update_params = UpdateAccountParams::new()
        .email(Nullable::Value("new_email@example.com".into()))
        .business_type(Nullable::Value(BusinessType::Individual));
    let update_json = serde_json::to_string(&update_params).unwrap();
    assert!(update_json.contains(r#""email":"new_email@example.com""#));
    assert!(update_json.contains(r#""business_type":"individual""#));
}

#[test]
fn test_account_full_lifecycle_json_deserialization() {
    let payload = r#"{
        "id": "acct_1032D82eZvKYlo2C",
        "object": "account",
        "type": "custom",
        "country": "US",
        "default_currency": "usd",
        "charges_enabled": true,
        "payouts_enabled": false,
        "details_submitted": true,
        "business_type": "company",
        "email": "owner@platform.io",
        "capabilities": {
            "card_payments": "active",
            "transfers": "inactive"
        },
        "requirements": {
            "currently_due": [],
            "eventually_due": ["company.tax_id"],
            "past_due": [],
            "pending_verification": []
        }
    }"#;

    let account: Account = serde_json::from_str(payload).expect("Deserialization must succeed");
    assert_eq!(account.id, "acct_1032D82eZvKYlo2C");
    assert_eq!(account.object, "account");
    assert_eq!(account.account_type, Some(AccountType::Custom));
    assert_eq!(account.country.as_deref(), Some("US"));
    assert_eq!(account.default_currency.as_deref(), Some("usd"));
    assert_eq!(account.charges_enabled, Some(true));
    assert_eq!(account.payouts_enabled, Some(false));
    assert_eq!(account.business_type, Some(BusinessType::Company));
    assert_eq!(account.email.as_deref(), Some("owner@platform.io"));

    let caps = account.capabilities.expect("Capabilities must be present");
    assert!(caps.is_card_payments_active());
    assert!(!caps.is_transfers_active());

    let reqs = account.requirements.expect("Requirements must be present");
    assert!(reqs.is_complete()); // currently_due is empty
}

#[test]
fn test_event_object_account_deserialization() {
    let payload = r#"{
        "id": "evt_test_123",
        "object": "event",
        "type": "account.updated",
        "created": 1700000000,
        "data": {
            "object": {
                "id": "acct_hook123",
                "object": "account",
                "type": "express",
                "charges_enabled": true,
                "payouts_enabled": true
            }
        }
    }"#;

    let event: some_stripe_types::Event = serde_json::from_str(payload).expect("Event parse must succeed");
    match event.deserialize_object().expect("Deserializing object must succeed") {
        some_stripe_types::EventObject::Account(acct) => {
            assert_eq!(acct.id, "acct_hook123");
            assert_eq!(acct.account_type, Some(AccountType::Express));
            assert_eq!(acct.charges_enabled, Some(true));
        }
        other => panic!("Expected Account event object, got {:?}", other),
    }
}
