//! Integration tests for Stripe Transfer and Transfer Reversal domain entities and parameters.
//!
//! Verifies:
//! - JSON deserialization of Transfer and TransferReversal fixtures
//! - ISO 4217 integer currency amounts (`i64`)
//! - CreateTransferParams serialization & validation
//! - Gate G-Borrow (borrowing by reference `&CreateTransferParams`)

use some_stripe_types::{
    CreateTransferParams, CreateTransferReversalParams, ListTransferReversalsParams,
    ListTransfersParams, Transfer, TransferId, TransferList, TransferReversal, TransferReversalId,
    TransferReversalList, UpdateTransferParams,
};

#[test]
fn test_transfer_domain_entity_deserialization() {
    let json = r#"{
        "id": "tr_1MtwXYZ123456789",
        "object": "transfer",
        "amount": 5000,
        "amount_reversed": 0,
        "balance_transaction": "txn_1MtwXYZ",
        "currency": "usd",
        "destination": "acct_1032D82eZvKYlo2C",
        "destination_payment": "py_1MtwXYZ",
        "livemode": false,
        "metadata": {
            "order_id": "ord_987"
        },
        "reversed": false,
        "source_transaction": null,
        "transfer_group": "group_order_123"
    }"#;

    let transfer: Transfer = serde_json::from_str(json).expect("Failed to deserialize transfer");
    assert_eq!(transfer.id.as_str(), "tr_1MtwXYZ123456789");
    assert_eq!(transfer.object, "transfer");
    assert_eq!(transfer.amount, 5000);
    assert_eq!(transfer.amount_reversed, 0);
    assert_eq!(transfer.currency, "usd");
    assert_eq!(transfer.destination.as_str(), "acct_1032D82eZvKYlo2C");
    assert!(!transfer.reversed);
    assert_eq!(transfer.transfer_group.as_deref(), Some("group_order_123"));
    assert_eq!(
        transfer
            .metadata
            .as_ref()
            .and_then(|m| m.get("order_id"))
            .map(String::as_str),
        Some("ord_987")
    );
}

#[test]
fn test_transfer_reversal_domain_entity_deserialization() {
    let json = r#"{
        "id": "trr_1MtwXYZ987654321",
        "object": "transfer_reversal",
        "amount": 1500,
        "balance_transaction": "txn_reversal_123",
        "currency": "usd",
        "metadata": {
            "reason": "customer_refund"
        },
        "transfer": "tr_1MtwXYZ123456789"
    }"#;

    let reversal: TransferReversal =
        serde_json::from_str(json).expect("Failed to deserialize transfer reversal");
    assert_eq!(reversal.id.as_str(), "trr_1MtwXYZ987654321");
    assert_eq!(reversal.object, "transfer_reversal");
    assert_eq!(reversal.amount, 1500);
    assert_eq!(reversal.currency, "usd");
    assert_eq!(reversal.transfer.as_str(), "tr_1MtwXYZ123456789");
    assert_eq!(
        reversal
            .metadata
            .as_ref()
            .and_then(|m| m.get("reason"))
            .map(String::as_str),
        Some("customer_refund")
    );
}

#[test]
fn test_transfer_lists_deserialization() {
    let list_json = r#"{
        "object": "list",
        "data": [
            {
                "id": "tr_1",
                "object": "transfer",
                "amount": 1000,
                "amount_reversed": 0,
                "currency": "usd",
                "destination": "acct_1",
                "reversed": false
            }
        ],
        "has_more": false,
        "url": "/v1/transfers"
    }"#;

    let transfer_list: TransferList =
        serde_json::from_str(list_json).expect("Failed to deserialize TransferList");
    assert_eq!(transfer_list.data.len(), 1);
    assert_eq!(transfer_list.data[0].id.as_str(), "tr_1");
    assert!(!transfer_list.has_more);

    let reversal_list_json = r#"{
        "object": "list",
        "data": [
            {
                "id": "trr_1",
                "object": "transfer_reversal",
                "amount": 500,
                "currency": "usd",
                "transfer": "tr_1"
            }
        ],
        "has_more": true,
        "url": "/v1/transfers/tr_1/reversals"
    }"#;

    let reversal_list: TransferReversalList = serde_json::from_str(reversal_list_json)
        .expect("Failed to deserialize TransferReversalList");
    assert_eq!(reversal_list.data.len(), 1);
    assert_eq!(reversal_list.data[0].id.as_str(), "trr_1");
    assert!(reversal_list.has_more);
}

/// Helper function demonstrating Gate G-Borrow: borrowing by reference `&CreateTransferParams`.
fn inspect_transfer_params(params: &CreateTransferParams) -> Result<(), String> {
    params.validate().map_err(|e| e.to_string())?;
    assert_eq!(params.amount, 10000);
    assert_eq!(params.currency, "usd");
    assert_eq!(params.destination, "acct_test_999");
    Ok(())
}

#[test]
fn test_create_transfer_params_and_g_borrow() {
    let params = CreateTransferParams::new(10000, "usd", "acct_test_999")
        .with_description("Test Transfer")
        .with_transfer_group("group_abc");

    // Enforce Gate G-Borrow: pass by reference without cloning
    assert!(inspect_transfer_params(&params).is_ok());

    let json = serde_json::to_string(&params).expect("Failed to serialize CreateTransferParams");
    assert!(json.contains("\"amount\":10000"));
    assert!(json.contains("\"currency\":\"usd\""));
    assert!(json.contains("\"destination\":\"acct_test_999\""));
    assert!(json.contains("\"transfer_group\":\"group_abc\""));
}

#[test]
fn test_update_and_reversal_params() {
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("key".into(), "value".into());
    let update_params = UpdateTransferParams::new().with_metadata(metadata);
    let json = serde_json::to_string(&update_params).unwrap();
    assert!(json.contains("metadata"));

    let reversal_params = CreateTransferReversalParams::new()
        .with_amount(500)
        .with_description("Partial refund");
    assert_eq!(reversal_params.amount, Some(500));
    assert!(reversal_params.validate().is_ok());

    let list_transfers = ListTransfersParams::new().with_destination("acct_123");
    assert_eq!(list_transfers.destination.as_deref(), Some("acct_123"));

    let list_reversals = ListTransferReversalsParams::new().with_limit(10);
    assert_eq!(list_reversals.limit, Some(10));
}
