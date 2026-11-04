//! Integration tests for PaymentIntent parameter schemas, builders, and minor currency validation.
//!
//! Validates Gate G-Law (ISO 4217 minor currency rules), Gate G2 (Debug credential masking),
//! and Gate G-Borrow (zero-clone referencing).

use some_stripe_types::{
    CancelPaymentIntentParams, CancellationReason, CaptureMethod, CapturePaymentIntentParams,
    ConfirmPaymentIntentParams, CreatePaymentIntentParams, PaymentIntentError,
    PaymentIntentValidationError, validate_amount, validate_capture_amount, validate_currency,
};

#[test]
fn test_minor_currency_amount_validation() {
    // Valid positive amounts in minor currency units (cents, yen, etc.)
    assert_eq!(validate_amount(1).unwrap(), 1);
    assert_eq!(validate_amount(50).unwrap(), 50);
    assert_eq!(validate_amount(1000).unwrap(), 1000);
    assert_eq!(validate_amount(99_999_999).unwrap(), 99_999_999);

    // Invalid zero or negative amounts
    assert_eq!(
        validate_amount(0),
        Err(PaymentIntentValidationError::InvalidAmount { amount: 0 })
    );
    assert_eq!(
        validate_amount(-1),
        Err(PaymentIntentValidationError::InvalidAmount { amount: -1 })
    );
    assert_eq!(
        validate_amount(-10_000),
        Err(PaymentIntentValidationError::InvalidAmount { amount: -10_000 })
    );
}

#[test]
fn test_capture_amount_validation() {
    // Valid capture amounts
    assert_eq!(validate_capture_amount(1).unwrap(), 1);
    assert_eq!(validate_capture_amount(2500).unwrap(), 2500);

    // Invalid capture amounts (zero or negative)
    assert_eq!(
        validate_capture_amount(0),
        Err(PaymentIntentValidationError::InvalidCaptureAmount { amount: 0 })
    );
    assert_eq!(
        validate_capture_amount(-500),
        Err(PaymentIntentValidationError::InvalidCaptureAmount { amount: -500 })
    );
}

#[test]
fn test_iso_4217_currency_validation_and_normalization() {
    // Valid lowercase
    assert_eq!(validate_currency("usd").unwrap(), "usd");
    assert_eq!(validate_currency("eur").unwrap(), "eur");
    assert_eq!(validate_currency("gbp").unwrap(), "gbp");
    assert_eq!(validate_currency("jpy").unwrap(), "jpy");
    assert_eq!(validate_currency("cad").unwrap(), "cad");

    // Valid uppercase converted to lowercase
    assert_eq!(validate_currency("USD").unwrap(), "usd");
    assert_eq!(validate_currency("EUR").unwrap(), "eur");
    assert_eq!(validate_currency("GBP").unwrap(), "gbp");
    assert_eq!(validate_currency("AUD").unwrap(), "aud");

    // Whitespace trimmed and normalized
    assert_eq!(validate_currency(" usd ").unwrap(), "usd");
    assert_eq!(validate_currency("\tEUR\n").unwrap(), "eur");

    // Invalid length
    assert!(matches!(
        validate_currency(""),
        Err(PaymentIntentValidationError::InvalidCurrency { .. })
    ));
    assert!(matches!(
        validate_currency("u"),
        Err(PaymentIntentValidationError::InvalidCurrency { .. })
    ));
    assert!(matches!(
        validate_currency("us"),
        Err(PaymentIntentValidationError::InvalidCurrency { .. })
    ));
    assert!(matches!(
        validate_currency("usdt"),
        Err(PaymentIntentValidationError::InvalidCurrency { .. })
    ));
    assert!(matches!(
        validate_currency("dollars"),
        Err(PaymentIntentValidationError::InvalidCurrency { .. })
    ));

    // Invalid characters (non-ASCII alphabetic)
    assert!(matches!(
        validate_currency("123"),
        Err(PaymentIntentValidationError::InvalidCurrency { .. })
    ));
    assert!(matches!(
        validate_currency("u$d"),
        Err(PaymentIntentValidationError::InvalidCurrency { .. })
    ));
    assert!(matches!(
        validate_currency("us1"),
        Err(PaymentIntentValidationError::InvalidCurrency { .. })
    ));
    assert!(matches!(
        validate_currency("€uro"),
        Err(PaymentIntentValidationError::InvalidCurrency { .. })
    ));
}

#[test]
fn test_create_params_constructor_and_invariants() {
    // Valid construction
    let params = CreatePaymentIntentParams::new(5000, "USD").expect("Valid creation");
    assert_eq!(params.amount, 5000);
    assert_eq!(params.currency, "usd");
    assert!(params.validate().is_ok());

    // Error on zero amount
    assert_eq!(
        CreatePaymentIntentParams::new(0, "usd").unwrap_err(),
        PaymentIntentError::InvalidAmount { amount: 0 }
    );

    // Error on negative amount
    assert_eq!(
        CreatePaymentIntentParams::new(-200, "usd").unwrap_err(),
        PaymentIntentError::InvalidAmount { amount: -200 }
    );

    // Error on invalid currency code
    assert!(matches!(
        CreatePaymentIntentParams::new(1000, "INVALID").unwrap_err(),
        PaymentIntentError::InvalidCurrency { .. }
    ));
}

#[test]
fn test_create_params_fluent_builder_chaining() {
    let params = CreatePaymentIntentParams::new(10000, "eur")
        .unwrap()
        .capture_method("manual")
        .description("Luxury Watch Order")
        .payment_method("pm_123456789")
        .customer("cus_vip_999")
        .receipt_email("buyer@example.com")
        .return_url("https://example.com/checkout/complete")
        .confirm(true)
        .metadata([("order_ref", "ord_lux_88"), ("channel", "mobile")])
        .idempotency_key("idem_watch_999");

    assert_eq!(params.amount, 10000);
    assert_eq!(params.currency, "eur");
    assert_eq!(params.capture_method.as_deref(), Some("manual"));
    assert_eq!(params.description.as_deref(), Some("Luxury Watch Order"));
    assert_eq!(params.payment_method.as_deref(), Some("pm_123456789"));
    assert_eq!(params.customer.as_deref(), Some("cus_vip_999"));
    assert_eq!(params.receipt_email.as_deref(), Some("buyer@example.com"));
    assert_eq!(
        params.return_url.as_deref(),
        Some("https://example.com/checkout/complete")
    );
    assert_eq!(params.confirm, Some(true));
    assert_eq!(params.idempotency_key.as_deref(), Some("idem_watch_999"));

    let meta = params.metadata.as_ref().unwrap();
    assert_eq!(meta.get("order_ref").unwrap(), "ord_lux_88");
    assert_eq!(meta.get("channel").unwrap(), "mobile");

    assert!(params.validate().is_ok());
}

#[test]
fn test_create_params_builder_pattern() {
    let params = CreatePaymentIntentParams::builder(7500, "GBP")
        .unwrap()
        .capture_method_type(CaptureMethod::Automatic)
        .description("Consulting fee")
        .payment_method("pm_card_gbp")
        .idempotency_key("idem_consult_01")
        .build()
        .unwrap();

    assert_eq!(params.amount, 7500);
    assert_eq!(params.currency, "gbp");
    assert_eq!(params.capture_method.as_deref(), Some("automatic"));
    assert_eq!(params.description.as_deref(), Some("Consulting fee"));
}

#[test]
fn test_capture_params_validation_and_builder() {
    // Full capture
    let full = CapturePaymentIntentParams::new();
    assert!(full.amount_to_capture.is_none());
    assert!(full.validate().is_ok());

    // Partial capture
    let partial = CapturePaymentIntentParams::with_amount(3000)
        .unwrap()
        .idempotency_key("idem_hotel_capture");

    assert_eq!(partial.amount_to_capture, Some(3000));
    assert_eq!(
        partial.idempotency_key.as_deref(),
        Some("idem_hotel_capture")
    );
    assert!(partial.validate().is_ok());

    // Reject <= 0 capture amounts
    assert_eq!(
        CapturePaymentIntentParams::with_amount(0).unwrap_err(),
        PaymentIntentValidationError::InvalidCaptureAmount { amount: 0 }
    );
    assert_eq!(
        CapturePaymentIntentParams::with_amount(-150).unwrap_err(),
        PaymentIntentValidationError::InvalidCaptureAmount { amount: -150 }
    );

    // Fluent setter rejection
    let builder_err = CapturePaymentIntentParams::new().amount_to_capture(0);
    assert_eq!(
        builder_err.unwrap_err(),
        PaymentIntentValidationError::InvalidCaptureAmount { amount: 0 }
    );
}

#[test]
fn test_cancel_params_builder_and_reasons() {
    let cancel = CancelPaymentIntentParams::new()
        .reason(CancellationReason::Fraudulent)
        .idempotency_key("idem_fraud_cancel");

    assert_eq!(cancel.cancellation_reason.as_deref(), Some("fraudulent"));
    assert_eq!(cancel.idempotency_key.as_deref(), Some("idem_fraud_cancel"));
    assert!(cancel.validate().is_ok());

    let cancel_str = CancelPaymentIntentParams::with_reason("requested_by_customer");
    assert_eq!(
        cancel_str.cancellation_reason.as_deref(),
        Some("requested_by_customer")
    );
}

#[test]
fn test_confirm_params_and_debug_redaction() {
    let confirm = ConfirmPaymentIntentParams::new()
        .payment_method("pm_card_mastercard")
        .return_url("https://example.com/3ds-complete")
        .receipt_email("receipt@example.com")
        .setup_future_usage("off_session")
        .client_secret("pi_secret_CRITICAL_SECRET_KEY_12345")
        .idempotency_key("idem_confirm_auth");

    assert_eq!(
        confirm.payment_method.as_deref(),
        Some("pm_card_mastercard")
    );
    assert_eq!(
        confirm.return_url.as_deref(),
        Some("https://example.com/3ds-complete")
    );
    assert_eq!(
        confirm.idempotency_key.as_deref(),
        Some("idem_confirm_auth")
    );
    assert!(confirm.validate().is_ok());

    // Gate G2 & PCI-DSS 4.0: Secret redaction in Debug output
    let debug_repr = format!("{confirm:?}");
    assert!(
        debug_repr.contains("[REDACTED]"),
        "Debug representation must redact client_secret"
    );
    assert!(
        !debug_repr.contains("CRITICAL_SECRET_KEY_12345"),
        "Debug representation must NOT contain raw client_secret"
    );
}

#[test]
fn test_serde_json_spec_compliance() {
    let params = CreatePaymentIntentParams::new(4500, "usd")
        .unwrap()
        .capture_method("automatic")
        .description("Widget payment")
        .payment_method("pm_card_visa")
        .idempotency_key("idem_do_not_serialize_body");

    let json = serde_json::to_string(&params).expect("Serialization failed");

    // Required fields present
    assert!(json.contains(r#""amount":4500"#));
    assert!(json.contains(r#""currency":"usd""#));
    assert!(json.contains(r#""capture_method":"automatic""#));
    assert!(json.contains(r#""description":"Widget payment""#));
    assert!(json.contains(r#""payment_method":"pm_card_visa""#));

    // Idempotency key omitted from body
    assert!(!json.contains("idem_do_not_serialize_body"));
    assert!(!json.contains("idempotency_key"));

    // Optional None fields omitted
    assert!(!json.contains("customer"));
    assert!(!json.contains("receipt_email"));
    assert!(!json.contains("return_url"));

    // Roundtrip deserialize
    let parsed: CreatePaymentIntentParams =
        serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(parsed.amount, 4500);
    assert_eq!(parsed.currency, "usd");
    assert_eq!(parsed.capture_method.as_deref(), Some("automatic"));
    assert_eq!(parsed.description.as_deref(), Some("Widget payment"));
    assert_eq!(parsed.payment_method.as_deref(), Some("pm_card_visa"));
    assert!(parsed.customer.is_none());
    assert!(parsed.idempotency_key.is_none());
}

#[test]
fn test_zero_clone_referencing() {
    let params = CreatePaymentIntentParams::new(3200, "usd").unwrap();
    // Verify parameters can be validated and inspected by reference without cloning
    fn check_ref(p: &CreatePaymentIntentParams) -> bool {
        p.validate().is_ok() && p.amount > 0 && p.currency == "usd"
    }
    assert!(check_ref(&params));
}
