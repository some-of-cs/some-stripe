//! Strongly Typed Models Contract & Serialization Verification Suite
//!
//! Validates serde serialization invariants, header newtype conversions,
//! discriminated EventObject union deserialization, and PCI-DSS client secret redaction.

use some_stripe_types::{
    AccountId, CancelRefundParams, CheckoutSession, CreateRefundParams, Event, EventData,
    EventObject, IdempotencyKey, Invoice, PaymentIntent, Refund, RefundReason, UpdateRefundParams,
    is_statutory_withdrawal_compliant,
};

#[test]
fn test_newtypes_conversion_and_display() {
    let acct = AccountId::from("acct_1234567890");
    assert_eq!(acct.to_string(), "acct_1234567890");
    assert_eq!(acct.0, "acct_1234567890");

    let idem = IdempotencyKey::from("idem_abc_xyz");
    assert_eq!(idem.to_string(), "idem_abc_xyz");
    assert_eq!(idem.0, "idem_abc_xyz");
}

#[test]
fn test_payment_intent_json_roundtrip() {
    let pi_json = serde_json::json!({
        "id": "pi_contract_test",
        "object": "payment_intent",
        "amount": 2500,
        "amount_received": 2500,
        "currency": "usd",
        "status": "succeeded",
        "capture_method": "automatic",
        "client_secret": "pi_contract_test_secret_abc123",
        "created": 1728345600
    });

    let pi: PaymentIntent = serde_json::from_value(pi_json).expect("Deserialization failed");
    assert_eq!(pi.id, "pi_contract_test");
    assert_eq!(pi.amount, 2500);
    assert_eq!(pi.status, "succeeded");

    let debug = format!("{pi:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("abc123"));
}

#[test]
fn test_checkout_session_json_roundtrip() {
    let cs_json = serde_json::json!({
        "id": "cs_contract_test",
        "object": "checkout.session",
        "url": "https://checkout.stripe.com/pay/cs_contract_test",
        "status": "open",
        "payment_status": "unpaid",
        "amount_total": 4900,
        "currency": "eur"
    });

    let cs: CheckoutSession = serde_json::from_value(cs_json).expect("Deserialization failed");
    assert_eq!(cs.id, "cs_contract_test");
    assert_eq!(cs.amount_total, Some(4900));
    assert_eq!(cs.currency, Some("eur".to_string()));
}

#[test]
fn test_invoice_json_roundtrip_with_id() {
    let inv_json = serde_json::json!({
        "id": "in_contract_test_99",
        "object": "invoice",
        "customer": "cus_contract_client",
        "amount_due": 7500,
        "amount_paid": 0,
        "amount_remaining": 7500,
        "total": 7500,
        "subtotal": 7500,
        "currency": "usd",
        "status": "open",
        "paid": false
    });

    let inv: Invoice = serde_json::from_value(inv_json).expect("Invoice deserialization failed");
    assert_eq!(inv.id.as_deref(), Some("in_contract_test_99"));
    assert_eq!(inv.customer.as_deref(), Some("cus_contract_client"));
    assert_eq!(inv.amount_due, Some(7500));
    assert_eq!(inv.status.as_deref(), Some("open"));
    assert_eq!(inv.paid, Some(false));
}

#[test]
fn test_invoice_upcoming_null_id_contract() {
    let upcoming_json = serde_json::json!({
        "id": null,
        "object": "invoice",
        "customer": "cus_upcoming_preview_1",
        "amount_due": 3200,
        "currency": "gbp",
        "period_start": 1728300000,
        "period_end": 1730900000
    });

    let inv: Invoice =
        serde_json::from_value(upcoming_json).expect("invoice.upcoming deserialization failed");
    assert!(inv.id.is_none(), "Expected None for upcoming invoice id");
    assert_eq!(inv.customer.as_deref(), Some("cus_upcoming_preview_1"));
    assert_eq!(inv.amount_due, Some(3200));
    assert_eq!(inv.currency.as_deref(), Some("gbp"));
}

#[test]
fn test_event_object_discriminated_union_contract() {
    // 1. CheckoutSession variant
    let cs_raw = serde_json::json!({
        "id": "cs_contract_evt",
        "object": "checkout.session",
        "status": "complete"
    });
    let data_cs = EventData { object: cs_raw };
    match data_cs.deserialize_object().unwrap() {
        EventObject::CheckoutSession(cs) => assert_eq!(cs.id, "cs_contract_evt"),
        other => panic!("Expected CheckoutSession, got {other:?}"),
    }

    // 2. PaymentIntent variant
    let pi_raw = serde_json::json!({
        "id": "pi_contract_evt",
        "object": "payment_intent",
        "amount": 5000,
        "currency": "usd",
        "status": "succeeded"
    });
    let data_pi = EventData { object: pi_raw };
    match data_pi.deserialize_object().unwrap() {
        EventObject::PaymentIntent(pi) => assert_eq!(pi.id, "pi_contract_evt"),
        other => panic!("Expected PaymentIntent, got {other:?}"),
    }

    // 3. Invoice variant with null id (invoice.upcoming)
    let inv_raw = serde_json::json!({
        "id": null,
        "object": "invoice",
        "customer": "cus_preview_contract",
        "amount_due": 1500
    });
    let data_inv = EventData { object: inv_raw };
    match data_inv.deserialize_object().unwrap() {
        EventObject::Invoice(inv) => {
            assert!(inv.id.is_none());
            assert_eq!(inv.customer.as_deref(), Some("cus_preview_contract"));
            assert_eq!(inv.amount_due, Some(1500));
        }
        other => panic!("Expected Invoice, got {other:?}"),
    }

    // 4. Fallback to Unknown for unmapped object
    let unknown_raw = serde_json::json!({
        "id": "sub_99999",
        "object": "subscription",
        "status": "active"
    });
    let data_unknown = EventData {
        object: unknown_raw,
    };
    assert!(matches!(
        data_unknown.deserialize_object().unwrap(),
        EventObject::Unknown
    ));
}

#[test]
fn test_event_object_unknown_fallback_contract() {
    let event = Event {
        id: "evt_contract_unsupported".into(),
        object: "event".into(),
        api_version: Some("2024-06-20".into()),
        created: 1728345600,
        event_type: "radar.early_fraud_warning.created".into(),
        data: EventData {
            object: serde_json::json!({
                "id": "issfr_12345",
                "object": "radar.early_fraud_warning",
                "actionable": true
            }),
        },
    };

    let obj = event
        .deserialize_object()
        .expect("Should safely resolve to Unknown");
    assert!(matches!(obj, EventObject::Unknown));
}

#[test]
fn test_refund_json_roundtrip_contract() {
    let refund_json = serde_json::json!({
        "id": "re_contract_test_01",
        "object": "refund",
        "amount": 4200,
        "currency": "eur",
        "status": "succeeded",
        "charge": "ch_source_charge_99",
        "payment_intent": "pi_source_pi_88",
        "reason": "requested_by_customer",
        "description": "Statutory withdrawal within 14 days",
        "receipt_number": "REC-EU-14D",
        "balance_transaction": "txn_settle_42",
        "created": 1728345600
    });

    let refund: Refund =
        serde_json::from_value(refund_json).expect("Refund deserialization failed");
    assert_eq!(refund.id, "re_contract_test_01");
    assert_eq!(refund.object, "refund");
    assert_eq!(refund.amount, 4200);
    assert_eq!(refund.currency, "eur");
    assert_eq!(refund.status, "succeeded");
    assert!(refund.is_succeeded());
    assert_eq!(refund.charge.as_deref(), Some("ch_source_charge_99"));
    assert_eq!(refund.payment_intent.as_deref(), Some("pi_source_pi_88"));
    assert_eq!(
        refund.parsed_reason(),
        Some(RefundReason::RequestedByCustomer)
    );
    assert_eq!(
        refund.description.as_deref(),
        Some("Statutory withdrawal within 14 days")
    );
    assert_eq!(refund.receipt_number.as_deref(), Some("REC-EU-14D"));
    assert_eq!(refund.balance_transaction.as_deref(), Some("txn_settle_42"));
}

#[test]
fn test_event_object_refund_contract() {
    let refund_raw = serde_json::json!({
        "id": "re_contract_evt_55",
        "object": "refund",
        "amount": 9900,
        "currency": "usd",
        "status": "succeeded"
    });

    let data = EventData { object: refund_raw };
    match data.deserialize_object().unwrap() {
        EventObject::Refund(r) => {
            assert_eq!(r.id, "re_contract_evt_55");
            assert_eq!(r.amount, 9900);
            assert_eq!(r.currency, "usd");
            assert_eq!(r.status, "succeeded");
        }
        other => panic!("Expected EventObject::Refund, got {other:?}"),
    }
}

#[test]
fn test_statutory_withdrawal_eu_directive_contract() {
    let charge_ts = 1_710_000_000;
    let charge_amount = 7500; // $75.00

    // Full refund 10 days later -> valid
    let refund_ts = charge_ts + (10 * 86_400);
    assert!(is_statutory_withdrawal_compliant(
        charge_amount,
        charge_amount,
        charge_ts,
        refund_ts
    ));

    // Boundary: exactly 14 days (14 * 86,400s) -> valid
    let boundary_ts = charge_ts + (14 * 86_400);
    assert!(is_statutory_withdrawal_compliant(
        charge_amount,
        charge_amount,
        charge_ts,
        boundary_ts
    ));

    // Exceeded boundary: 14 days + 1 second -> invalid
    assert!(!is_statutory_withdrawal_compliant(
        charge_amount,
        charge_amount,
        charge_ts,
        boundary_ts + 1
    ));

    // Partial refund (deduction violation under Directive 2011/83/EU Art 13) -> invalid
    assert!(!is_statutory_withdrawal_compliant(
        charge_amount - 100,
        charge_amount,
        charge_ts,
        refund_ts
    ));
}

#[test]
fn test_refund_params_zero_clone_contract() {
    let params = CreateRefundParams::for_charge("ch_test_zero_clone")
        .with_amount(5000)
        .with_reason(RefundReason::Fraudulent)
        .with_idempotency_key("idem_zc_1");

    // Borrowing references directly without cloning
    let charge_ref: Option<&str> = params.charge.as_deref();
    assert_eq!(charge_ref, Some("ch_test_zero_clone"));
    assert_eq!(params.amount, Some(5000));
    assert_eq!(params.reason.as_deref(), Some("fraudulent"));

    let update = UpdateRefundParams::new().with_idempotency_key("idem_zc_upd");
    assert_eq!(update.idempotency_key.as_deref(), Some("idem_zc_upd"));

    let cancel = CancelRefundParams::new().with_idempotency_key("idem_zc_cancel");
    assert_eq!(cancel.idempotency_key.as_deref(), Some("idem_zc_cancel"));
}
