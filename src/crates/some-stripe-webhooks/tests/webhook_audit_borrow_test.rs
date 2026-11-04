//! Gate G-Borrow Zero-Clone Event Traversal & ISO 27001 Audit Logging Audit Suite
//!
//! Validates:
//! 1. Gate G-Borrow: `order_id()`, `customer_id()`, `payment_intent_id()`, and `amount_total()`
//!    borrow without cloning the underlying event buffer.
//! 2. ISO/IEC 27001:2022 Control A.8.15: Event verification produces immutable, timestamped
//!    audit records with verification status.

use some_stripe_webhooks::{StripeWebhookWrapper, Webhook, WebhookAuditRecord};

#[test]
fn test_zero_clone_event_pointer_accessors() {
    let payload = br#"{
        "id": "evt_test_borrow_123",
        "object": "event",
        "created": 1728345600,
        "type": "checkout.session.completed",
        "data": {
            "object": {
                "id": "cs_test_order_789",
                "customer": "cus_test_borrow_456",
                "payment_intent": "pi_test_intent_123",
                "amount_total": 12500,
                "currency": "usd",
                "status": "complete",
                "payment_status": "paid"
            }
        }
    }"#;

    let secret = "whsec_test_borrow_secret_key";
    let now = chrono::Utc::now().timestamp();
    let header = Webhook::generate_test_header_string(payload, secret, Some(now));

    let wrapper = StripeWebhookWrapper::from_raw(payload, &header, secret, None)
        .expect("Failed to verify and construct wrapper");

    // 1. Assert values match payload
    assert_eq!(wrapper.order_id(), Some("cs_test_order_789"));
    assert_eq!(wrapper.customer_id(), Some("cus_test_borrow_456"));
    assert_eq!(wrapper.payment_intent_id(), Some("pi_test_intent_123"));
    assert_eq!(wrapper.amount_total(), Some(12500));

    // 2. Gate G-Borrow Verification: Assert pointer addresses match underlying JSON buffer
    let raw_order_id_ptr = wrapper
        .event
        .data
        .object
        .pointer("/id")
        .and_then(|v| v.as_str())
        .map(|s| s.as_ptr())
        .expect("Failed to retrieve raw /id pointer");
    assert_eq!(
        wrapper.order_id().unwrap().as_ptr(),
        raw_order_id_ptr,
        "Gate G-Borrow violation: order_id() must borrow directly from event buffer without allocation"
    );

    let raw_customer_id_ptr = wrapper
        .event
        .data
        .object
        .pointer("/customer")
        .and_then(|v| v.as_str())
        .map(|s| s.as_ptr())
        .expect("Failed to retrieve raw /customer pointer");
    assert_eq!(
        wrapper.customer_id().unwrap().as_ptr(),
        raw_customer_id_ptr,
        "Gate G-Borrow violation: customer_id() must borrow directly from event buffer without allocation"
    );

    let raw_pi_id_ptr = wrapper
        .event
        .data
        .object
        .pointer("/payment_intent")
        .and_then(|v| v.as_str())
        .map(|s| s.as_ptr())
        .expect("Failed to retrieve raw /payment_intent pointer");
    assert_eq!(
        wrapper.payment_intent_id().unwrap().as_ptr(),
        raw_pi_id_ptr,
        "Gate G-Borrow violation: payment_intent_id() must borrow directly from event buffer without allocation"
    );

    // 3. Test missing fields yield None cleanly
    let empty_payload = br#"{
        "id": "evt_test_empty",
        "object": "event",
        "created": 1728345600,
        "type": "customer.created",
        "data": {
            "object": {}
        }
    }"#;
    let empty_header = Webhook::generate_test_header_string(empty_payload, secret, Some(now));
    let empty_wrapper = StripeWebhookWrapper::from_raw(empty_payload, &empty_header, secret, None)
        .expect("Failed to construct empty wrapper");

    assert_eq!(empty_wrapper.order_id(), None);
    assert_eq!(empty_wrapper.customer_id(), None);
    assert_eq!(empty_wrapper.payment_intent_id(), None);
    assert_eq!(empty_wrapper.amount_total(), None);
}

#[test]
fn test_tc_2_2_0_law_iso_27001_audit_logging() {
    let payload = br#"{
        "id": "evt_iso27001_audit_pass",
        "object": "event",
        "created": 1728345600,
        "type": "checkout.session.completed",
        "data": {
            "object": {
                "id": "cs_audit_iso",
                "status": "complete",
                "payment_status": "paid"
            }
        }
    }"#;

    let secret = "whsec_audit_iso27001_secret";
    let now = chrono::Utc::now().timestamp();
    let header = Webhook::generate_test_header_string(payload, secret, Some(now));

    // 1. Verified Webhook Event Audit Record
    let wrapper = StripeWebhookWrapper::from_raw(payload, &header, secret, None)
        .expect("Verification must succeed");

    let audit: WebhookAuditRecord<'_> = wrapper.audit_record();

    assert_eq!(audit.event_id, "evt_iso27001_audit_pass");
    assert_eq!(audit.event_type, "checkout.session.completed");
    assert!(audit.is_verified());
    assert_eq!(audit.status, "verified");
    assert!(audit.verified_at.is_some());
    assert!(
        (audit.verified_at.unwrap() - now).abs() <= 2,
        "Verification timestamp must match current clock"
    );

    // Gate G-Borrow: Audit record borrows directly from the wrapper/event buffers
    assert_eq!(
        audit.event_id.as_ptr(),
        wrapper.event.id.as_ptr(),
        "Audit record event_id must borrow from wrapper without allocation"
    );
    assert_eq!(
        audit.event_type.as_ptr(),
        wrapper.event.event_type.as_ptr(),
        "Audit record event_type must borrow from wrapper without allocation"
    );

    // From trait implementation check
    let audit_from: WebhookAuditRecord<'_> = (&wrapper).into();
    assert_eq!(audit, audit_from);

    // 2. Unverified Event Audit Record
    let unverified_wrapper = StripeWebhookWrapper::new(wrapper.event.clone(), None);
    let unverified_audit = unverified_wrapper.audit_record();
    assert_eq!(unverified_audit.status, "unverified");
    assert_eq!(unverified_audit.verified_at, None);
    assert!(!unverified_audit.is_verified());

    // 3. Security Non-Repudiation Check: Tampered Payload Rejected
    let tampered_payload = br#"{
        "id": "evt_tampered_payload",
        "object": "event",
        "created": 1728345600,
        "type": "checkout.session.completed",
        "data": { "object": {} }
    }"#;
    let bad_header = "t=1728345600,v1=bad_signature_digest_attempt";
    let rejected = StripeWebhookWrapper::from_raw(tampered_payload, bad_header, secret, None);
    assert!(
        rejected.is_err(),
        "Tampered payload must be rejected prior to wrapper construction"
    );
}
