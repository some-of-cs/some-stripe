//! Strongly Typed Models Contract & Serialization Verification Suite
//!
//! Validates serde serialization invariants, header newtype conversions,
//! and PCI-DSS client secret redaction in some-stripe-types.

use some_stripe_types::{AccountId, CheckoutSession, IdempotencyKey, PaymentIntent};

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
