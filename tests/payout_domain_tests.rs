//! # Payout Domain Tests
//!
//! Verifies serialization roundtrip, status enum transitions, and parameter serialization for Payouts.

use some_stripe_types::{
    CreatePayoutParams, EventData, EventObject, ListPayoutsParams, Payout, PayoutId, PayoutList,
    PayoutMethod, PayoutStatus, PayoutType, UpdatePayoutParams,
};
use std::collections::HashMap;
use std::str::FromStr;

#[test]
fn test_payout_status_parsing_and_display() {
    let statuses = [
        (PayoutStatus::Paid, "paid"),
        (PayoutStatus::Pending, "pending"),
        (PayoutStatus::InTransit, "in_transit"),
        (PayoutStatus::Canceled, "canceled"),
        (PayoutStatus::Failed, "failed"),
    ];

    for (status, expected_str) in statuses {
        assert_eq!(status.as_str(), expected_str);
        assert_eq!(status.to_string(), expected_str);
        assert_eq!(PayoutStatus::from_str(expected_str).unwrap(), status);

        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, format!("\"{expected_str}\""));
        let deserialized: PayoutStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, status);
    }

    let invalid = PayoutStatus::from_str("unknown_status");
    assert!(invalid.is_err());
}

#[test]
fn test_payout_method_parsing_and_display() {
    let methods = [
        (PayoutMethod::Standard, "standard"),
        (PayoutMethod::Instant, "instant"),
    ];

    for (method, expected_str) in methods {
        assert_eq!(method.as_str(), expected_str);
        assert_eq!(method.to_string(), expected_str);
        assert_eq!(PayoutMethod::from_str(expected_str).unwrap(), method);

        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, format!("\"{expected_str}\""));
        let deserialized: PayoutMethod = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, method);
    }
}

#[test]
fn test_payout_type_parsing_and_display() {
    let types = [
        (PayoutType::BankAccount, "bank_account"),
        (PayoutType::Card, "card"),
    ];

    for (ptype, expected_str) in types {
        assert_eq!(ptype.as_str(), expected_str);
        assert_eq!(ptype.to_string(), expected_str);
        assert_eq!(PayoutType::from_str(expected_str).unwrap(), ptype);

        let json = serde_json::to_string(&ptype).unwrap();
        assert_eq!(json, format!("\"{expected_str}\""));
        let deserialized: PayoutType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ptype);
    }
}

#[test]
fn test_payout_id_wrappers() {
    let id_str = "po_1MtwXYZ";
    let payout_id = PayoutId::from(id_str);
    assert_eq!(payout_id.as_ref(), id_str);
    assert_eq!(payout_id.to_string(), id_str);

    let payout_id_owned: PayoutId = String::from(id_str).into();
    assert_eq!(payout_id_owned, payout_id);
}

#[test]
fn test_payout_domain_model_serde_roundtrip() {
    let mut metadata = HashMap::new();
    metadata.insert("order_id".into(), "ord_999".into());

    let payout = Payout {
        id: "po_test_12345".into(),
        object: "payout".into(),
        amount: 110000,
        arrival_date: 1735776000,
        automatic: false,
        balance_transaction: Some("txn_999".into()),
        created: 1735689600,
        currency: "usd".into(),
        description: Some("STRIPE PAYOUT".into()),
        destination: Some("ba_12345".into()),
        failure_balance_transaction: None,
        failure_code: None,
        failure_message: None,
        livemode: false,
        metadata: Some(metadata),
        method: PayoutMethod::Standard,
        original_payout: None,
        reversed_by: None,
        source_type: Some("bank_account".into()),
        statement_descriptor: Some("MYCORP PAYOUT".into()),
        status: PayoutStatus::Paid,
        type_: PayoutType::BankAccount,
    };

    let json = serde_json::to_string(&payout).unwrap();
    assert!(json.contains("\"id\":\"po_test_12345\""));
    assert!(json.contains("\"object\":\"payout\""));
    assert!(json.contains("\"amount\":110000"));
    assert!(json.contains("\"currency\":\"usd\""));
    assert!(json.contains("\"status\":\"paid\""));
    assert!(json.contains("\"method\":\"standard\""));
    assert!(json.contains("\"type\":\"bank_account\""));

    let deserialized: Payout = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, payout);
}

#[test]
fn test_payout_list_serde() {
    let payout_list = PayoutList {
        object: "list".into(),
        data: vec![],
        has_more: false,
        url: "/v1/payouts".into(),
    };

    let json = serde_json::to_string(&payout_list).unwrap();
    assert!(json.contains("\"object\":\"list\""));
    assert!(json.contains("\"has_more\":false"));

    let deserialized: PayoutList = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, payout_list);
}

#[test]
fn test_create_payout_params_serialization() {
    let params = CreatePayoutParams::new(50000, "usd")
        .with_method(PayoutMethod::Instant)
        .with_statement_descriptor("INSTANT PAY");

    let json = serde_json::to_string(&params).unwrap();
    assert!(json.contains("\"amount\":50000"));
    assert!(json.contains("\"currency\":\"usd\""));
    assert!(json.contains("\"method\":\"instant\""));
    assert!(json.contains("\"statement_descriptor\":\"INSTANT PAY\""));
}

#[test]
fn test_update_and_list_payouts_params() {
    let mut metadata = HashMap::new();
    metadata.insert("reviewed".into(), "true".into());
    let update_params = UpdatePayoutParams {
        metadata: Some(metadata),
        ..Default::default()
    };
    let update_json = serde_json::to_string(&update_params).unwrap();
    assert!(update_json.contains("\"reviewed\":\"true\""));

    let list_params = ListPayoutsParams {
        status: Some(PayoutStatus::Paid),
        limit: Some(10),
        starting_after: Some("po_start".into()),
        ending_before: None,
    };
    let list_json = serde_json::to_string(&list_params).unwrap();
    assert!(list_json.contains("\"status\":\"paid\""));
    assert!(list_json.contains("\"limit\":10"));
    assert!(list_json.contains("\"starting_after\":\"po_start\""));
    assert!(!list_json.contains("ending_before"));
}

#[test]
fn test_event_object_payout_variant() {
    let json = serde_json::json!({
        "id": "po_evt_test_777",
        "object": "payout",
        "amount": 25000,
        "arrival_date": 1735776000,
        "automatic": true,
        "created": 1735689600,
        "currency": "usd",
        "livemode": false,
        "method": "standard",
        "status": "in_transit",
        "type": "bank_account"
    });

    let data = EventData { object: json };
    let obj = data
        .deserialize_object()
        .expect("Failed to deserialize payout event object");

    match obj {
        EventObject::Payout(payout) => {
            assert_eq!(payout.id, "po_evt_test_777");
            assert_eq!(payout.object, "payout");
            assert_eq!(payout.amount, 25000);
            assert_eq!(payout.currency, "usd");
            assert_eq!(payout.status, PayoutStatus::InTransit);
            assert_eq!(payout.method, PayoutMethod::Standard);
            assert_eq!(payout.type_, PayoutType::BankAccount);
            assert!(payout.automatic);
        }
        other => panic!("Expected Payout variant, got {other:?}"),
    }
}
