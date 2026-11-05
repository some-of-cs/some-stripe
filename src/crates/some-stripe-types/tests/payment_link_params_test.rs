//! Contract & wire-level tests for Stripe PaymentLink request parameters,
//! multi-currency pricing schemas, and indexed bracket form serialization.
//!
//! Validates Gate G-Law (ISO 4217 minor currency units), Gate G1 (Dual-Protocol Serde),
//! and Gate G-Borrow (zero-clone referencing).

use some_stripe_core::patterns::serializer::{to_form_params, to_form_urlencoded};
use some_stripe_types::{
    AdjustableQuantityParams, CreatePaymentLinkParams, CurrencyOptionParams,
    CustomUnitAmountParams, EventData, EventObject, HostedConfirmationParams, PaymentLink,
    PaymentLinkAfterCompletionParams, PaymentLinkCurrencyOptionsParams,
    PaymentLinkCustomTextMessageParams, PaymentLinkCustomTextParams, PaymentLinkLineItemParams,
    PaymentLinkPriceDataParams, PaymentLinkRedirectParams, PaymentLinkStatus,
    PaymentLinkValidationError, UpdatePaymentLinkParams,
};
use std::collections::HashMap;

#[test]
fn test_create_payment_link_params_json_roundtrip() {
    let mut metadata = HashMap::new();
    metadata.insert("order_ref".into(), "order_9988".into());
    metadata.insert("channel".into(), "instagram".into());

    let params = CreatePaymentLinkParams::new(vec![
        PaymentLinkLineItemParams::new("price_1N42iV2eZvKYlo2CUbdf5V4L", 2)
            .adjustable_quantity(AdjustableQuantityParams::with_limits(1, 10)),
        PaymentLinkLineItemParams::new("price_addon_sticker", 1),
    ])
    .active(true)
    .after_completion(PaymentLinkAfterCompletionParams::redirect(
        "https://example.com/checkout/success",
    ))
    .allow_promotion_codes(true)
    .billing_address_collection("auto")
    .customer_creation("if_required")
    .metadata(metadata)
    .payment_method_types(vec!["card".into(), "ideal".into()])
    .idempotency_key("idem_plink_create_123");

    // 1. Validate parameters
    assert!(params.validate().is_ok());

    // 2. Serialize to JSON
    let json = serde_json::to_string_pretty(&params).expect("JSON serialization must succeed");

    // Assert that idempotency_key is omitted from serialization (Gate G1)
    assert!(!json.contains("idem_plink_create_123"));
    assert!(!json.contains("idempotency_key"));

    // Assert required fields exist in JSON
    assert!(json.contains(r#""price": "price_1N42iV2eZvKYlo2CUbdf5V4L""#));
    assert!(json.contains(r#""quantity": 2"#));
    assert!(json.contains(r#""enabled": true"#));
    assert!(json.contains(r#""minimum": 1"#));
    assert!(json.contains(r#""maximum": 10"#));
    assert!(json.contains(r#""price": "price_addon_sticker""#));
    assert!(json.contains(r#""active": true"#));
    assert!(json.contains(r#""type": "redirect""#));
    assert!(json.contains(r#""url": "https://example.com/checkout/success""#));
    assert!(json.contains(r#""allow_promotion_codes": true"#));
    assert!(json.contains(r#""billing_address_collection": "auto""#));
    assert!(json.contains(r#""customer_creation": "if_required""#));
    assert!(json.contains(r#""order_ref": "order_9988""#));
    assert!(json.contains(r#""card""#));
    assert!(json.contains(r#""ideal""#));

    // 3. Deserialize back from JSON
    let deserialized: CreatePaymentLinkParams =
        serde_json::from_str(&json).expect("JSON deserialization must succeed");

    assert_eq!(deserialized.line_items.len(), 2);
    assert_eq!(
        deserialized.line_items[0].price.as_deref(),
        Some("price_1N42iV2eZvKYlo2CUbdf5V4L")
    );
    assert_eq!(deserialized.line_items[0].quantity, 2);
    let adj = deserialized.line_items[0]
        .adjustable_quantity
        .as_ref()
        .unwrap();
    assert!(adj.enabled);
    assert_eq!(adj.minimum, Some(1));
    assert_eq!(adj.maximum, Some(10));

    assert_eq!(
        deserialized.line_items[1].price.as_deref(),
        Some("price_addon_sticker")
    );
    assert_eq!(deserialized.line_items[1].quantity, 1);
    assert!(deserialized.line_items[1].adjustable_quantity.is_none());

    assert_eq!(deserialized.active, Some(true));
    let after = deserialized.after_completion.as_ref().unwrap();
    assert_eq!(after.r#type, "redirect");
    assert_eq!(
        after.redirect.as_ref().unwrap().url,
        "https://example.com/checkout/success"
    );
    assert_eq!(deserialized.allow_promotion_codes, Some(true));
    assert_eq!(
        deserialized.billing_address_collection.as_deref(),
        Some("auto")
    );
    assert_eq!(
        deserialized.customer_creation.as_deref(),
        Some("if_required")
    );
    assert_eq!(
        deserialized.payment_method_types.as_deref(),
        Some(&["card".to_string(), "ideal".to_string()][..])
    );
    assert!(deserialized.idempotency_key.is_none());
}

#[test]
fn test_hosted_confirmation_params_json_roundtrip() {
    let direct_hc = HostedConfirmationParams::new(Some("Direct test".into()));
    assert_eq!(direct_hc.custom_message.as_deref(), Some("Direct test"));

    let params =
        CreatePaymentLinkParams::new(vec![PaymentLinkLineItemParams::new("price_hosted_test", 1)])
            .after_completion(PaymentLinkAfterCompletionParams::hosted_confirmation(Some(
                "Thank you for backing our campaign!".into(),
            )));

    assert!(params.validate().is_ok());

    let json = serde_json::to_string(&params).expect("Serialization");
    assert!(json.contains(r#""type":"hosted_confirmation""#));
    assert!(json.contains(r#""custom_message":"Thank you for backing our campaign!""#));

    let deserialized: CreatePaymentLinkParams =
        serde_json::from_str(&json).expect("Deserialization");
    let after = deserialized.after_completion.unwrap();
    assert_eq!(after.r#type, "hosted_confirmation");
    assert_eq!(
        after.hosted_confirmation.unwrap().custom_message.as_deref(),
        Some("Thank you for backing our campaign!")
    );
}

#[test]
fn test_stripe_v1_indexed_bracket_encoding_full() {
    let mut metadata = HashMap::new();
    metadata.insert("sku_ref".into(), "SKU_7711".into());

    let params = CreatePaymentLinkParams::new(vec![
        PaymentLinkLineItemParams::new("price_main_tier", 3)
            .adjustable_quantity(AdjustableQuantityParams::with_limits(1, 5)),
        PaymentLinkLineItemParams::new("price_secondary_tier", 1),
    ])
    .active(true)
    .after_completion(PaymentLinkAfterCompletionParams::redirect(
        "https://example.com/done",
    ))
    .allow_promotion_codes(true)
    .billing_address_collection("required")
    .customer_creation("always")
    .metadata(metadata)
    .payment_method_types(vec!["card".into(), "sofort".into()])
    .idempotency_key("idem_skip_secret");

    // 1. Serialize into flat key-value pairs matching Stripe v1 indexed bracket convention
    let form_pairs = to_form_params(&params).expect("to_form_params must succeed");
    let map: HashMap<String, String> = form_pairs.into_iter().collect();

    // Line items indexed bracket checks: line_items[0]... and line_items[1]...
    assert_eq!(
        map.get("line_items[0][price]").map(String::as_str),
        Some("price_main_tier")
    );
    assert_eq!(
        map.get("line_items[0][quantity]").map(String::as_str),
        Some("3")
    );
    assert_eq!(
        map.get("line_items[0][adjustable_quantity][enabled]")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        map.get("line_items[0][adjustable_quantity][minimum]")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        map.get("line_items[0][adjustable_quantity][maximum]")
            .map(String::as_str),
        Some("5")
    );

    assert_eq!(
        map.get("line_items[1][price]").map(String::as_str),
        Some("price_secondary_tier")
    );
    assert_eq!(
        map.get("line_items[1][quantity]").map(String::as_str),
        Some("1")
    );
    assert!(!map.contains_key("line_items[1][adjustable_quantity][enabled]"));

    // Top-level fields
    assert_eq!(map.get("active").map(String::as_str), Some("true"));
    assert_eq!(
        map.get("after_completion[type]").map(String::as_str),
        Some("redirect")
    );
    assert_eq!(
        map.get("after_completion[redirect][url]")
            .map(String::as_str),
        Some("https://example.com/done")
    );
    assert_eq!(
        map.get("allow_promotion_codes").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        map.get("billing_address_collection").map(String::as_str),
        Some("required")
    );
    assert_eq!(
        map.get("customer_creation").map(String::as_str),
        Some("always")
    );
    assert_eq!(
        map.get("metadata[sku_ref]").map(String::as_str),
        Some("SKU_7711")
    );
    assert_eq!(
        map.get("payment_method_types[0]").map(String::as_str),
        Some("card")
    );
    assert_eq!(
        map.get("payment_method_types[1]").map(String::as_str),
        Some("sofort")
    );

    // Idempotency key MUST NOT exist in form params (Gate G1)
    assert!(!map.contains_key("idempotency_key"));

    // 2. Serialize to application/x-www-form-urlencoded
    let urlencoded = to_form_urlencoded(&params).expect("to_form_urlencoded must succeed");

    assert!(urlencoded.contains("line_items%5B0%5D%5Bprice%5D=price_main_tier"));
    assert!(urlencoded.contains("line_items%5B0%5D%5Bquantity%5D=3"));
    assert!(urlencoded.contains("line_items%5B0%5D%5Badjustable_quantity%5D%5Benabled%5D=true"));
    assert!(urlencoded.contains("line_items%5B0%5D%5Badjustable_quantity%5D%5Bminimum%5D=1"));
    assert!(urlencoded.contains("line_items%5B0%5D%5Badjustable_quantity%5D%5Bmaximum%5D=5"));
    assert!(urlencoded.contains("line_items%5B1%5D%5Bprice%5D=price_secondary_tier"));
    assert!(urlencoded.contains("after_completion%5Btype%5D=redirect"));
    assert!(
        urlencoded
            .contains("after_completion%5Bredirect%5D%5Burl%5D=https%3A%2F%2Fexample.com%2Fdone")
    );
    assert!(!urlencoded.contains("idem_skip_secret"));
    assert!(!urlencoded.contains("idempotency_key"));
}

#[test]
fn test_multi_currency_pricing_schemas_and_bracket_encoding() {
    let mut currency_options = HashMap::new();

    // EUR option: 950 cents (€9.50)
    currency_options.insert(
        "eur".to_string(),
        CurrencyOptionParams {
            unit_amount: Some(950),
            tax_behavior: Some("inclusive".into()),
            custom_unit_amount: None,
        },
    );

    // GBP option: 800 pence (£8.00) with custom unit amount / donation enabled
    currency_options.insert(
        "gbp".to_string(),
        CurrencyOptionParams {
            unit_amount: Some(800),
            tax_behavior: Some("exclusive".into()),
            custom_unit_amount: Some(CustomUnitAmountParams {
                enabled: true,
                minimum: Some(500),
                maximum: Some(50000),
                preset: Some(1500),
            }),
        },
    );

    let price_data = PaymentLinkPriceDataParams::new("usd", 1000)
        .product("prod_multi_curr_widget")
        .currency_options(currency_options);

    assert!(price_data.validate().is_ok());

    let line_item = PaymentLinkLineItemParams::with_price_data(price_data, 1);
    let params = CreatePaymentLinkParams::new(vec![line_item]);

    assert!(params.validate().is_ok());

    // Verify form serialization with nested multi-currency brackets
    let form_pairs = to_form_params(&params).expect("to_form_params must succeed");
    let map: HashMap<String, String> = form_pairs.into_iter().collect();

    assert_eq!(
        map.get("line_items[0][price_data][currency]")
            .map(String::as_str),
        Some("usd")
    );
    assert_eq!(
        map.get("line_items[0][price_data][unit_amount]")
            .map(String::as_str),
        Some("1000")
    );
    assert_eq!(
        map.get("line_items[0][price_data][product]")
            .map(String::as_str),
        Some("prod_multi_curr_widget")
    );

    // EUR sub-bracket
    assert_eq!(
        map.get("line_items[0][price_data][currency_options][eur][unit_amount]")
            .map(String::as_str),
        Some("950")
    );
    assert_eq!(
        map.get("line_items[0][price_data][currency_options][eur][tax_behavior]")
            .map(String::as_str),
        Some("inclusive")
    );

    // GBP sub-bracket with custom_unit_amount
    assert_eq!(
        map.get("line_items[0][price_data][currency_options][gbp][unit_amount]")
            .map(String::as_str),
        Some("800")
    );
    assert_eq!(
        map.get("line_items[0][price_data][currency_options][gbp][tax_behavior]")
            .map(String::as_str),
        Some("exclusive")
    );
    assert_eq!(
        map.get("line_items[0][price_data][currency_options][gbp][custom_unit_amount][enabled]")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        map.get("line_items[0][price_data][currency_options][gbp][custom_unit_amount][minimum]")
            .map(String::as_str),
        Some("500")
    );
    assert_eq!(
        map.get("line_items[0][price_data][currency_options][gbp][custom_unit_amount][maximum]")
            .map(String::as_str),
        Some("50000")
    );
    assert_eq!(
        map.get("line_items[0][price_data][currency_options][gbp][custom_unit_amount][preset]")
            .map(String::as_str),
        Some("1500")
    );
}

#[test]
fn test_payment_link_currency_options_presentation() {
    let mut custom_options = HashMap::new();
    custom_options.insert(
        "eur".to_string(),
        PaymentLinkCurrencyOptionsParams {
            custom_text: Some(PaymentLinkCustomTextParams {
                shipping_address: Some(PaymentLinkCustomTextMessageParams::new(
                    "Free shipping within EU",
                )),
                submit: Some(PaymentLinkCustomTextMessageParams::new("Zahlen")),
            }),
        },
    );

    let params = CreatePaymentLinkParams::new(vec![PaymentLinkLineItemParams::new("price_eur", 1)])
        .currency_options(custom_options);

    assert!(params.validate().is_ok());

    let form_pairs = to_form_params(&params).expect("to_form_params");
    let map: HashMap<String, String> = form_pairs.into_iter().collect();

    assert_eq!(
        map.get("currency_options[eur][custom_text][shipping_address][message]")
            .map(String::as_str),
        Some("Free shipping within EU")
    );
    assert_eq!(
        map.get("currency_options[eur][custom_text][submit][message]")
            .map(String::as_str),
        Some("Zahlen")
    );
}

#[test]
fn test_update_payment_link_params_deactivation_scenario() {
    // Scenario [AC-US-4.2.0-02]: Deactivate an existing payment link
    let update = UpdatePaymentLinkParams::new()
        .active(false)
        .allow_promotion_codes(false)
        .metadata_entry("reason", "campaign_ended")
        .idempotency_key("idem_deactivate_plink");

    assert!(update.validate().is_ok());

    let form_pairs = to_form_params(&update).expect("to_form_params must succeed");
    let map: HashMap<String, String> = form_pairs.into_iter().collect();

    assert_eq!(map.get("active").map(String::as_str), Some("false"));
    assert_eq!(
        map.get("allow_promotion_codes").map(String::as_str),
        Some("false")
    );
    assert_eq!(
        map.get("metadata[reason]").map(String::as_str),
        Some("campaign_ended")
    );
    assert!(!map.contains_key("idempotency_key"));
}

#[test]
fn test_payment_link_validation_invariants() {
    // 1. Empty line items
    let empty_items = CreatePaymentLinkParams::new(vec![]);
    assert_eq!(
        empty_items.validate(),
        Err(PaymentLinkValidationError::EmptyLineItems)
    );

    // 2. Zero quantity
    let zero_qty = CreatePaymentLinkParams::new(vec![PaymentLinkLineItemParams {
        price: Some("price_123".into()),
        quantity: 0,
        adjustable_quantity: None,
        price_data: None,
    }]);
    assert_eq!(
        zero_qty.validate(),
        Err(PaymentLinkValidationError::InvalidQuantity { quantity: 0 })
    );

    // 3. Missing both price and price_data
    let missing_price = CreatePaymentLinkParams::new(vec![PaymentLinkLineItemParams {
        price: None,
        quantity: 1,
        adjustable_quantity: None,
        price_data: None,
    }]);
    assert_eq!(
        missing_price.validate(),
        Err(PaymentLinkValidationError::MissingPriceOrPriceData)
    );

    // 4. Inverted adjustable quantity limits
    let inverted_limits = CreatePaymentLinkParams::new(vec![
        PaymentLinkLineItemParams::new("price_123", 1)
            .adjustable_quantity(AdjustableQuantityParams::with_limits(50, 10)),
    ]);
    assert_eq!(
        inverted_limits.validate(),
        Err(
            PaymentLinkValidationError::InvalidAdjustableQuantityLimits {
                minimum: 50,
                maximum: 10,
            }
        )
    );

    // 5. Invalid after_completion redirect with empty URL
    let empty_redirect =
        CreatePaymentLinkParams::new(vec![PaymentLinkLineItemParams::new("price_123", 1)])
            .after_completion(PaymentLinkAfterCompletionParams {
                r#type: "redirect".into(),
                redirect: Some(PaymentLinkRedirectParams::new("   ")),
                hosted_confirmation: None,
            });
    assert!(matches!(
        empty_redirect.validate(),
        Err(PaymentLinkValidationError::InvalidAfterCompletion { .. })
    ));

    // 6. Invalid after_completion type
    let invalid_type =
        CreatePaymentLinkParams::new(vec![PaymentLinkLineItemParams::new("price_123", 1)])
            .after_completion(PaymentLinkAfterCompletionParams {
                r#type: "unsupported_type".into(),
                redirect: None,
                hosted_confirmation: None,
            });
    assert!(matches!(
        invalid_type.validate(),
        Err(PaymentLinkValidationError::InvalidAfterCompletion { .. })
    ));

    // 7. Invalid ISO 4217 currency
    let invalid_curr_pd = PaymentLinkPriceDataParams::new("invalid_currency_xyz", 1000);
    assert!(matches!(
        invalid_curr_pd.validate(),
        Err(PaymentLinkValidationError::InvalidCurrency { .. })
    ));

    // 8. Non-positive amount
    let zero_amt_pd = PaymentLinkPriceDataParams::new("usd", 0);
    assert_eq!(
        zero_amt_pd.validate(),
        Err(PaymentLinkValidationError::InvalidAmount { amount: 0 })
    );

    let neg_amt_pd = PaymentLinkPriceDataParams::new("usd", -500);
    assert_eq!(
        neg_amt_pd.validate(),
        Err(PaymentLinkValidationError::InvalidAmount { amount: -500 })
    );
}

#[test]
fn test_event_object_payment_link_variant() {
    let evt_data = EventData {
        object: serde_json::json!({
            "id": "plink_evt_contract_999",
            "object": "payment_link",
            "active": true,
            "url": "https://buy.stripe.com/plink_evt_contract_999"
        }),
    };

    let obj = evt_data
        .deserialize_object()
        .expect("EventObject deserialization must succeed");

    match obj {
        EventObject::PaymentLink(link) => {
            let mut pl: PaymentLink = link;
            assert_eq!(pl.id, "plink_evt_contract_999");
            assert!(pl.is_active());
            assert_eq!(pl.status(), PaymentLinkStatus::Active);
            assert_eq!(pl.url, "https://buy.stripe.com/plink_evt_contract_999");
            pl.deactivate();
            assert_eq!(pl.status(), PaymentLinkStatus::Inactive);
        }
        other => panic!("Expected EventObject::PaymentLink, got {other:?}"),
    }
}
