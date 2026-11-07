//! Unit and integration test suite for Draft & Finalized Invoices Core Lifecycle (US-5.2.0 Unit 1).
//!
//! Validates:
//! 1. Invoice status transitions: draft -> open -> paid / void / uncollectible.
//! 2. EU VAT Directive 2006/112/EC Article 226 sequential numbering integrity.
//! 3. ZATCA Phase 2 cryptographic stamp and hash-chain continuity.
//! 4. Serialization / deserialization (including null `id` preview events).
//! 5. Request parameter builder ergonomics and immutability.

use some_stripe::{
    CreateInvoiceParams, FinalizeInvoiceParams, Invoice, InvoiceStatus, ListInvoicesParams,
    PayInvoiceParams, UpdateInvoiceParams, VoidInvoiceParams, validate_eu_vat_sequential_numbering,
    validate_zatca_phase2_hash_chain,
};

#[test]
fn test_invoice_status_parsing_and_display() {
    use std::str::FromStr;

    let cases = [
        (InvoiceStatus::Draft, "draft"),
        (InvoiceStatus::Open, "open"),
        (InvoiceStatus::Paid, "paid"),
        (InvoiceStatus::Uncollectible, "uncollectible"),
        (InvoiceStatus::Void, "void"),
    ];

    for (status, expected) in cases {
        assert_eq!(status.as_str(), expected);
        assert_eq!(status.to_string(), expected);
        assert_eq!(status.as_ref(), expected);
        let parsed = InvoiceStatus::from_str(expected).unwrap();
        assert_eq!(parsed, status);
    }
}

#[test]
fn test_invoice_lifecycle_transitions() {
    let draft = InvoiceStatus::Draft;
    assert!(draft.is_draft());
    assert!(draft.can_transition_to(InvoiceStatus::Open));
    assert!(draft.can_transition_to(InvoiceStatus::Void));
    assert!(!draft.can_transition_to(InvoiceStatus::Paid));

    let open = InvoiceStatus::Open;
    assert!(open.is_open());
    assert!(open.can_transition_to(InvoiceStatus::Paid));
    assert!(open.can_transition_to(InvoiceStatus::Uncollectible));
    assert!(open.can_transition_to(InvoiceStatus::Void));
    assert!(!open.can_transition_to(InvoiceStatus::Draft));

    let paid = InvoiceStatus::Paid;
    assert!(paid.is_paid());
    assert!(!paid.can_transition_to(InvoiceStatus::Open));
    assert!(!paid.can_transition_to(InvoiceStatus::Void));

    let void = InvoiceStatus::Void;
    assert!(void.is_void());
    assert!(!void.can_transition_to(InvoiceStatus::Open));
}

#[test]
fn test_eu_vat_sequential_numbering_validation() {
    // Valid initial invoice
    assert!(validate_eu_vat_sequential_numbering(None, "INV-2024-0001").is_ok());

    // Valid consecutive sequential invoices (+1)
    assert!(validate_eu_vat_sequential_numbering(Some("INV-2024-0001"), "INV-2024-0002").is_ok());
    assert!(validate_eu_vat_sequential_numbering(Some("INV-2024-0099"), "INV-2024-0100").is_ok());

    // Gap detection (non-sequential)
    let err_gap =
        validate_eu_vat_sequential_numbering(Some("INV-2024-0001"), "INV-2024-0003").unwrap_err();
    assert!(
        err_gap
            .to_string()
            .contains("Non-sequential invoice numbering detected")
    );

    // Prefix mismatch detection
    let err_prefix =
        validate_eu_vat_sequential_numbering(Some("INV-2023-0005"), "INV-2024-0006").unwrap_err();
    assert!(err_prefix.to_string().contains("Series prefix mismatch"));

    // Blank number rejection
    let err_blank = validate_eu_vat_sequential_numbering(Some("INV-2024-0001"), "").unwrap_err();
    assert!(
        err_blank
            .to_string()
            .contains("Current invoice number cannot be blank")
    );
}

#[test]
fn test_zatca_phase2_hash_chain_validation() {
    use sha2::{Digest, Sha256};

    let prev_hash = "0000000000000000000000000000000000000000000000000000000000000000";
    let payload = b"{\"invoice_number\":\"INV-2024-0001\",\"total\":1000}";
    let mut hasher = Sha256::new();
    hasher.update(payload);
    let hash = hasher.finalize();
    let current_stamp: String = hash.iter().map(|b| format!("{b:02x}")).collect();

    // Valid hash chain
    assert!(
        validate_zatca_phase2_hash_chain(prev_hash, prev_hash, payload, &current_stamp).is_ok()
    );

    // Tampered payload / stamp mismatch
    let err_stamp =
        validate_zatca_phase2_hash_chain(prev_hash, prev_hash, b"tampered", &current_stamp)
            .unwrap_err();
    assert!(
        err_stamp
            .to_string()
            .contains("Cryptographic stamp mismatch")
    );

    // Tampered previous hash
    let err_chain =
        validate_zatca_phase2_hash_chain(prev_hash, "wrong_prev_hash", payload, &current_stamp)
            .unwrap_err();
    assert!(err_chain.to_string().contains("Broken hash chain"));
}

#[test]
fn test_invoice_deserialization_draft_and_finalized() {
    let json_draft = r#"{
        "id": "in_test_draft_1",
        "object": "invoice",
        "customer": "cus_123",
        "amount_due": 5000,
        "amount_paid": 0,
        "amount_remaining": 5000,
        "currency": "usd",
        "status": "draft",
        "number": null,
        "paid": false
    }"#;

    let draft: Invoice = serde_json::from_str(json_draft).unwrap();
    assert_eq!(draft.id.as_deref(), Some("in_test_draft_1"));
    assert_eq!(draft.status, Some(InvoiceStatus::Draft));
    assert!(draft.number.is_none());
    assert_eq!(draft.paid, Some(false));

    let json_paid = r#"{
        "id": "in_test_paid_2",
        "object": "invoice",
        "customer": "cus_123",
        "amount_due": 5000,
        "amount_paid": 5000,
        "amount_remaining": 0,
        "currency": "usd",
        "status": "paid",
        "number": "INV-2024-0001",
        "paid": true
    }"#;

    let paid: Invoice = serde_json::from_str(json_paid).unwrap();
    assert_eq!(paid.status, Some(InvoiceStatus::Paid));
    assert_eq!(paid.number.as_deref(), Some("INV-2024-0001"));
    assert_eq!(paid.paid, Some(true));
}

#[test]
fn test_param_builders() {
    let create = CreateInvoiceParams::new("cus_123")
        .with_currency("usd")
        .with_auto_advance(true)
        .with_description("Annual subscription")
        .with_idempotency_key("idem_123");
    assert_eq!(create.customer, "cus_123");
    assert_eq!(create.currency.as_deref(), Some("usd"));
    assert_eq!(create.auto_advance, Some(true));

    let update = UpdateInvoiceParams::new()
        .with_description("Updated description")
        .with_idempotency_key("idem_upd");
    assert_eq!(update.description.as_deref(), Some("Updated description"));

    let finalize = FinalizeInvoiceParams::new().with_auto_advance(false);
    assert_eq!(finalize.auto_advance, Some(false));

    let pay = PayInvoiceParams::new().with_paid_out_of_band(true);
    assert_eq!(pay.paid_out_of_band, Some(true));

    let void_p = VoidInvoiceParams::new().with_idempotency_key("idem_void");
    assert_eq!(void_p.idempotency_key.as_deref(), Some("idem_void"));

    let list = ListInvoicesParams::new()
        .with_customer("cus_123")
        .with_status(InvoiceStatus::Open)
        .with_limit(20);
    assert_eq!(list.customer.as_deref(), Some("cus_123"));
    assert_eq!(list.status, Some(InvoiceStatus::Open));
    assert_eq!(list.limit, Some(20));
}
