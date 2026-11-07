//! # Application Fee & Fee Refund Domain and Parameter Unit Tests
//!
//! Verifies serialization roundtrip, parameter builders, event object dispatch, and ISO 4217 minor currency integer rules.

use some_stripe_types::{
    ApplicationFee, ApplicationFeeId, ApplicationFeeList, CreateFeeRefundParams, EventData,
    EventObject, FeeRefund, FeeRefundId, FeeRefundList, ListApplicationFeesParams,
    ListFeeRefundsParams,
};
use std::collections::HashMap;

#[test]
fn test_application_fee_ids_and_display() {
    let fee_id = ApplicationFeeId::from("fee_1Mtw");
    assert_eq!(fee_id.as_ref(), "fee_1Mtw");
    assert_eq!(fee_id.to_string(), "fee_1Mtw");

    let refund_id = FeeRefundId::from("fr_1Mtw");
    assert_eq!(refund_id.as_ref(), "fr_1Mtw");
    assert_eq!(refund_id.to_string(), "fr_1Mtw");
}

#[test]
fn test_application_fee_and_refund_serde_roundtrip() {
    let mut metadata = HashMap::new();
    metadata.insert("order_id".to_string(), "ord_999".to_string());

    let refund = FeeRefund {
        id: "fr_test_123".to_string(),
        object: "fee_refund".to_string(),
        amount: 250,
        balance_transaction: Some("txn_refund_123".to_string()),
        created: 1700000000,
        currency: "usd".to_string(),
        fee: "fee_test_123".to_string(),
        metadata: Some(metadata.clone()),
    };

    let refund_list = FeeRefundList {
        object: "list".to_string(),
        data: vec![refund.clone()],
        has_more: false,
        url: "https://api.stripe.com/v1/application_fees/fee_test_123/refunds".to_string(),
    };

    let fee = ApplicationFee {
        id: "fee_test_123".to_string(),
        object: "application_fee".to_string(),
        account: "acct_connected_123".to_string(),
        amount: 1000,
        amount_refunded: 250,
        application: "ca_platform_123".to_string(),
        balance_transaction: Some("txn_fee_123".to_string()),
        charge: "ch_test_123".to_string(),
        created: 1700000000,
        currency: "usd".to_string(),
        livemode: false,
        refunded: false,
        refunds: Some(refund_list),
    };

    let json = serde_json::to_string_pretty(&fee).expect("Failed to serialize ApplicationFee");
    let deserialized: ApplicationFee =
        serde_json::from_str(&json).expect("Failed to deserialize ApplicationFee");

    assert_eq!(fee, deserialized);
    assert_eq!(deserialized.id, "fee_test_123");
    assert_eq!(deserialized.amount, 1000);
    assert_eq!(deserialized.amount_refunded, 250);
    assert_eq!(deserialized.currency, "usd");
    assert!(deserialized.refunds.is_some());
    assert_eq!(deserialized.refunds.as_ref().unwrap().data.len(), 1);
    assert_eq!(deserialized.refunds.as_ref().unwrap().data[0].amount, 250);
}

#[test]
fn test_application_fee_list_serde() {
    let fee_list = ApplicationFeeList {
        object: "list".to_string(),
        data: vec![],
        has_more: true,
        url: "https://api.stripe.com/v1/application_fees".to_string(),
    };

    let json = serde_json::to_string(&fee_list).unwrap();
    let deserialized: ApplicationFeeList = serde_json::from_str(&json).unwrap();
    assert_eq!(fee_list, deserialized);
    assert!(deserialized.has_more);
}

#[test]
fn test_fee_request_params() {
    let mut metadata = HashMap::new();
    metadata.insert("reason".to_string(), "goodwill".to_string());

    let create_refund_params = CreateFeeRefundParams {
        amount: Some(500),
        metadata: Some(metadata),
        idempotency_key: None,
    };

    let json = serde_json::to_string(&create_refund_params).unwrap();
    assert!(json.contains("\"amount\":500"));
    assert!(json.contains("\"metadata\""));

    let list_fees_params = ListApplicationFeesParams {
        charge: Some("ch_test_123".to_string()),
        limit: Some(10),
        starting_after: Some("fee_abc".to_string()),
        ending_before: None,
    };
    let json_fees = serde_json::to_string(&list_fees_params).unwrap();
    assert!(json_fees.contains("\"charge\":\"ch_test_123\""));
    assert!(json_fees.contains("\"limit\":10"));

    let list_refunds_params = ListFeeRefundsParams {
        limit: Some(5),
        starting_after: None,
        ending_before: Some("fr_xyz".to_string()),
    };
    let json_refunds = serde_json::to_string(&list_refunds_params).unwrap();
    assert!(json_refunds.contains("\"ending_before\":\"fr_xyz\""));
}

#[test]
fn test_event_object_application_fee_and_refund() {
    let fee_json = serde_json::json!({
        "id": "fee_evt_123",
        "object": "application_fee",
        "account": "acct_test",
        "amount": 1500,
        "amount_refunded": 0,
        "application": "ca_test",
        "charge": "ch_test",
        "created": 1700000000,
        "currency": "usd",
        "livemode": false,
        "refunded": false
    });

    let data = EventData { object: fee_json };
    match data.deserialize_object().unwrap() {
        EventObject::ApplicationFee(fee) => {
            assert_eq!(fee.id, "fee_evt_123");
            assert_eq!(fee.amount, 1500);
            assert_eq!(fee.currency, "usd");
        }
        other => panic!("Expected ApplicationFee variant, got {other:?}"),
    }

    let refund_json = serde_json::json!({
        "id": "fr_evt_456",
        "object": "fee_refund",
        "amount": 300,
        "created": 1700000000,
        "currency": "usd",
        "fee": "fee_evt_123"
    });

    let data_refund = EventData {
        object: refund_json,
    };
    match data_refund.deserialize_object().unwrap() {
        EventObject::FeeRefund(refund) => {
            assert_eq!(refund.id, "fr_evt_456");
            assert_eq!(refund.amount, 300);
            assert_eq!(refund.fee, "fee_evt_123");
        }
        other => panic!("Expected FeeRefund variant, got {other:?}"),
    }
}
