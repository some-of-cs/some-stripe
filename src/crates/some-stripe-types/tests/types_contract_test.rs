//! Strongly Typed Models Contract & Serialization Verification Suite
//!
//! Validates serde serialization invariants, header newtype conversions,
//! discriminated EventObject union deserialization, and PCI-DSS client secret redaction.

use some_stripe_types::{
    AccountId, CaptureMethod, CardPaymentMethodOptions, CheckoutSession, Event, EventData,
    EventObject, IdempotencyKey, Invoice, PaymentIntent, PaymentIntentStatus, PaymentMethodOptions,
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
        "description": "Integration test payment",
        "created": 1728345600,
        "payment_method_options": {
            "card": {
                "request_three_d_secure": "automatic",
                "capture_method": "automatic"
            }
        }
    });

    let pi: PaymentIntent = serde_json::from_value(pi_json).expect("Deserialization failed");
    assert_eq!(pi.id, "pi_contract_test");
    assert_eq!(pi.amount, 2500);
    assert_eq!(pi.status, "succeeded");
    assert_eq!(pi.status, PaymentIntentStatus::Succeeded);
    assert_eq!(pi.capture_method, Some(CaptureMethod::Automatic));
    assert_eq!(pi.description.as_deref(), Some("Integration test payment"));

    let opts = pi
        .payment_method_options
        .as_ref()
        .expect("Expected payment_method_options");
    let card = opts.card.as_ref().expect("Expected card options");
    assert_eq!(card.request_three_d_secure.as_deref(), Some("automatic"));
    assert_eq!(card.capture_method, Some(CaptureMethod::Automatic));

    let debug = format!("{pi:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("abc123"));
}

#[test]
fn test_payment_intent_status_contract_and_string_conversions() {
    use std::str::FromStr;

    let cases = [
        (
            "requires_payment_method",
            PaymentIntentStatus::RequiresPaymentMethod,
        ),
        (
            "requires_confirmation",
            PaymentIntentStatus::RequiresConfirmation,
        ),
        ("requires_action", PaymentIntentStatus::RequiresAction),
        ("processing", PaymentIntentStatus::Processing),
        ("requires_capture", PaymentIntentStatus::RequiresCapture),
        ("canceled", PaymentIntentStatus::Canceled),
        ("succeeded", PaymentIntentStatus::Succeeded),
    ];

    for (raw_str, expected_variant) in cases {
        assert_eq!(
            PaymentIntentStatus::from_str(raw_str).unwrap(),
            expected_variant
        );
        assert_eq!(expected_variant.as_str(), raw_str);
        assert_eq!(expected_variant.to_string(), raw_str);
        assert_eq!(expected_variant, raw_str);
        assert_eq!(raw_str, expected_variant);

        let serialized = serde_json::to_string(&expected_variant).unwrap();
        assert_eq!(serialized, format!("\"{raw_str}\""));
        let deserialized: PaymentIntentStatus = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, expected_variant);
    }
}

#[test]
fn test_capture_method_contract_and_string_conversions() {
    use std::str::FromStr;

    let cases = [
        ("automatic", CaptureMethod::Automatic),
        ("manual", CaptureMethod::Manual),
    ];

    for (raw_str, expected_variant) in cases {
        assert_eq!(CaptureMethod::from_str(raw_str).unwrap(), expected_variant);
        assert_eq!(expected_variant.as_str(), raw_str);
        assert_eq!(expected_variant.to_string(), raw_str);
        assert_eq!(expected_variant, raw_str);
        assert_eq!(raw_str, expected_variant);

        let serialized = serde_json::to_string(&expected_variant).unwrap();
        assert_eq!(serialized, format!("\"{raw_str}\""));
        let deserialized: CaptureMethod = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, expected_variant);
    }
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
fn test_payment_method_options_contract() {
    let opts = PaymentMethodOptions::new().with_card(
        CardPaymentMethodOptions::new()
            .with_request_three_d_secure("challenge")
            .with_capture_method(CaptureMethod::Manual),
    );

    let json = serde_json::to_string(&opts).expect("Serialization failed");
    let deserialized: PaymentMethodOptions =
        serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(deserialized, opts);
    let card = deserialized.card.unwrap();
    assert_eq!(card.request_three_d_secure.as_deref(), Some("challenge"));
    assert_eq!(card.capture_method, Some(CaptureMethod::Manual));
}
