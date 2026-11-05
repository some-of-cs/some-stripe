//! Integration tests for [`CreateCheckoutSessionParams`] builder, serde serialization,
//! and bracket notation flattening with `to_form_params`.
//!
//! Validates Gate G-Law (ISO 4217 minor currency rules), Gate G-Borrow (zero deep clones),
//! and dual-protocol form-encoding compliance for Stripe Checkout Sessions.

use std::collections::HashMap;

use some_stripe_core::patterns::serializer::to_form_params;
use some_stripe_types::{
    CreateCheckoutSessionParams, LineItemParams, LineItemPriceDataParams,
    LineItemProductDataParams, PaymentIntentValidationError,
};

/// Helper to convert form parameter tuples into a lookup map for test assertions.
fn to_map(pairs: Vec<(String, String)>) -> HashMap<String, String> {
    pairs.into_iter().collect()
}

#[test]
fn test_single_line_item_bracket_notation_keys() {
    let params = CreateCheckoutSessionParams::new("https://example.com/success")
        .cancel_url("https://example.com/cancel")
        .mode("payment")
        .add_line_item(LineItemParams::new().quantity(1).price_data(
            LineItemPriceDataParams::with_amount(
                "usd",
                2000,
                LineItemProductDataParams::new("T-shirt"),
            ),
        ));

    let pairs = to_form_params(&params).expect("Flattening to form params should succeed");
    let map = to_map(pairs);

    // Exact keys mandated by specification
    assert_eq!(
        map.get("line_items[0][price_data][currency]")
            .map(String::as_str),
        Some("usd")
    );
    assert_eq!(
        map.get("line_items[0][price_data][unit_amount]")
            .map(String::as_str),
        Some("2000")
    );
    assert_eq!(
        map.get("line_items[0][price_data][product_data][name]")
            .map(String::as_str),
        Some("T-shirt")
    );
    assert_eq!(
        map.get("line_items[0][quantity]").map(String::as_str),
        Some("1")
    );

    // Top-level session keys
    assert_eq!(
        map.get("success_url").map(String::as_str),
        Some("https://example.com/success")
    );
    assert_eq!(
        map.get("cancel_url").map(String::as_str),
        Some("https://example.com/cancel")
    );
    assert_eq!(map.get("mode").map(String::as_str), Some("payment"));

    // Omitted fields must not be present
    assert!(!map.contains_key("client_reference_id"));
    assert!(!map.contains_key("customer"));
    assert!(!map.contains_key("customer_email"));
    assert!(!map.contains_key("expires_at"));
    assert!(!map.contains_key("metadata"));
    assert!(!map.contains_key("idempotency_key"));
}

#[test]
fn test_multiple_line_items_bracket_indexing() {
    let item1 = LineItemParams::from_price_data(
        LineItemPriceDataParams::with_amount("eur", 1500, LineItemProductDataParams::new("Book")),
        2,
    );
    let item2 = LineItemParams::from_price_data(
        LineItemPriceDataParams::with_amount(
            "eur",
            3000,
            LineItemProductDataParams::new("Backpack").description("Waterproof hiking backpack"),
        ),
        1,
    );

    let params = CreateCheckoutSessionParams::new("https://example.com/success")
        .mode("payment")
        .line_items(vec![item1, item2]);

    let pairs = to_form_params(&params).expect("Flattening must succeed");
    let map = to_map(pairs);

    // First item: index 0
    assert_eq!(
        map.get("line_items[0][price_data][currency]")
            .map(String::as_str),
        Some("eur")
    );
    assert_eq!(
        map.get("line_items[0][price_data][unit_amount]")
            .map(String::as_str),
        Some("1500")
    );
    assert_eq!(
        map.get("line_items[0][price_data][product_data][name]")
            .map(String::as_str),
        Some("Book")
    );
    assert_eq!(
        map.get("line_items[0][quantity]").map(String::as_str),
        Some("2")
    );

    // Second item: index 1
    assert_eq!(
        map.get("line_items[1][price_data][currency]")
            .map(String::as_str),
        Some("eur")
    );
    assert_eq!(
        map.get("line_items[1][price_data][unit_amount]")
            .map(String::as_str),
        Some("3000")
    );
    assert_eq!(
        map.get("line_items[1][price_data][product_data][name]")
            .map(String::as_str),
        Some("Backpack")
    );
    assert_eq!(
        map.get("line_items[1][price_data][product_data][description]")
            .map(String::as_str),
        Some("Waterproof hiking backpack")
    );
    assert_eq!(
        map.get("line_items[1][quantity]").map(String::as_str),
        Some("1")
    );
}

#[test]
fn test_line_item_with_price_id() {
    let item = LineItemParams::from_price("price_1N4exampleXYZ", 3);
    let params = CreateCheckoutSessionParams::new("https://example.com/checkout/success")
        .add_line_item(item);

    let pairs = to_form_params(&params).expect("Flattening must succeed");
    let map = to_map(pairs);

    assert_eq!(
        map.get("line_items[0][price]").map(String::as_str),
        Some("price_1N4exampleXYZ")
    );
    assert_eq!(
        map.get("line_items[0][quantity]").map(String::as_str),
        Some("3")
    );
    assert!(!map.contains_key("line_items[0][price_data][currency]"));
}

#[test]
fn test_product_data_with_description_and_images_bracket_notation() {
    let product_data = LineItemProductDataParams::new("Sneakers")
        .description("Comfortable running shoes")
        .images(vec![
            "https://images.example.com/sneakers-front.jpg",
            "https://images.example.com/sneakers-side.jpg",
        ]);

    let price_data = LineItemPriceDataParams::with_amount("usd", 8500, product_data);
    let item = LineItemParams::from_price_data(price_data, 1);
    let params =
        CreateCheckoutSessionParams::new("https://example.com/success").add_line_item(item);

    let pairs = to_form_params(&params).expect("Flattening must succeed");
    let map = to_map(pairs);

    assert_eq!(
        map.get("line_items[0][price_data][product_data][name]")
            .map(String::as_str),
        Some("Sneakers")
    );
    assert_eq!(
        map.get("line_items[0][price_data][product_data][description]")
            .map(String::as_str),
        Some("Comfortable running shoes")
    );
    assert_eq!(
        map.get("line_items[0][price_data][product_data][images][0]")
            .map(String::as_str),
        Some("https://images.example.com/sneakers-front.jpg")
    );
    assert_eq!(
        map.get("line_items[0][price_data][product_data][images][1]")
            .map(String::as_str),
        Some("https://images.example.com/sneakers-side.jpg")
    );
}

#[test]
fn test_metadata_bracket_notation() {
    let params = CreateCheckoutSessionParams::new("https://example.com/success")
        .metadata([("order_id", "ord_789"), ("internal_user_id", "usr_456")]);

    let pairs = to_form_params(&params).expect("Flattening must succeed");
    let map = to_map(pairs);

    assert_eq!(
        map.get("metadata[order_id]").map(String::as_str),
        Some("ord_789")
    );
    assert_eq!(
        map.get("metadata[internal_user_id]").map(String::as_str),
        Some("usr_456")
    );
}

#[test]
fn test_idempotency_key_skipped_from_body_and_form() {
    let params = CreateCheckoutSessionParams::new("https://example.com/success")
        .idempotency_key("idem_secret_key_12345");

    // Must be skipped from form serialization
    let pairs = to_form_params(&params).expect("Flattening must succeed");
    let map = to_map(pairs);
    assert!(!map.contains_key("idempotency_key"));
    assert_eq!(
        params.idempotency_key.as_deref(),
        Some("idem_secret_key_12345")
    );

    // Must be skipped from JSON serialization
    let json = serde_json::to_string(&params).expect("JSON serialization must succeed");
    assert!(!json.contains("idem_secret_key_12345"));
    assert!(!json.contains("idempotency_key"));
}

#[test]
fn test_backward_compatibility_single_item_constructor() {
    let params = CreateCheckoutSessionParams::single_item(
        "https://example.com/checkout/return",
        "USD",
        5000,
        "Wallet deposit",
        1,
    );

    assert_eq!(params.success_url, "https://example.com/checkout/return");
    assert_eq!(params.currency.as_deref(), Some("usd"));
    assert_eq!(params.unit_amount, Some(5000));
    assert_eq!(params.product_name.as_deref(), Some("Wallet deposit"));
    assert_eq!(params.quantity, Some(1));
    assert_eq!(params.mode, "payment");

    // The line_items field should also be synthesized for dual compatibility
    let items = params.line_items.as_ref().expect("line_items synthesized");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].quantity, Some(1));
    let price_data = items[0].price_data.as_ref().unwrap();
    assert_eq!(price_data.currency, "usd");
    assert_eq!(price_data.unit_amount, Some(5000));
    assert_eq!(price_data.product_data.name, "Wallet deposit");

    // Bracket notation serialization should work seamlessly
    let pairs = to_form_params(&params).expect("Flattening must succeed");
    let map = to_map(pairs);
    assert_eq!(
        map.get("line_items[0][price_data][currency]")
            .map(String::as_str),
        Some("usd")
    );
    assert_eq!(
        map.get("line_items[0][price_data][unit_amount]")
            .map(String::as_str),
        Some("5000")
    );
    assert_eq!(
        map.get("line_items[0][price_data][product_data][name]")
            .map(String::as_str),
        Some("Wallet deposit")
    );
    assert_eq!(
        map.get("line_items[0][quantity]").map(String::as_str),
        Some("1")
    );
}

#[test]
fn test_serde_json_roundtrip() {
    let original = CreateCheckoutSessionParams::new("https://example.com/success")
        .cancel_url("https://example.com/cancel")
        .mode("subscription")
        .client_reference_id("ref_cust_123")
        .customer("cus_stripe_123")
        .customer_email("buyer@example.com")
        .expires_at(1735689600)
        .add_metadata("source", "ios_app")
        .add_line_item(LineItemParams::from_price("price_pro_monthly", 1));

    let serialized = serde_json::to_string(&original).expect("JSON serialization");
    let deserialized: CreateCheckoutSessionParams =
        serde_json::from_str(&serialized).expect("JSON deserialization");

    assert_eq!(deserialized.success_url, original.success_url);
    assert_eq!(deserialized.cancel_url, original.cancel_url);
    assert_eq!(deserialized.mode, "subscription");
    assert_eq!(
        deserialized.client_reference_id,
        original.client_reference_id
    );
    assert_eq!(deserialized.customer, original.customer);
    assert_eq!(deserialized.customer_email, original.customer_email);
    assert_eq!(deserialized.expires_at, original.expires_at);
    assert_eq!(deserialized.metadata, original.metadata);
    assert_eq!(deserialized.line_items, original.line_items);
    // Idempotency key is skipped from JSON
    assert_eq!(deserialized.idempotency_key, None);
}

#[test]
fn test_validation_gate_g_law_rules() {
    // Valid session
    let valid = CreateCheckoutSessionParams::new("https://example.com/success").add_line_item(
        LineItemParams::from_price_data(
            LineItemPriceDataParams::with_amount(
                "usd",
                100,
                LineItemProductDataParams::new("Sticker"),
            ),
            1,
        ),
    );
    assert!(valid.validate().is_ok());

    // Empty success_url rejected
    let empty_url = CreateCheckoutSessionParams::new("");
    assert!(matches!(
        empty_url.validate(),
        Err(PaymentIntentValidationError::InvalidParameter {
            field: "success_url",
            ..
        })
    ));

    // Invalid ISO currency rejected
    let bad_curr = CreateCheckoutSessionParams::new("https://example.com/success").add_line_item(
        LineItemParams::from_price_data(
            LineItemPriceDataParams::with_amount(
                "usd1",
                100,
                LineItemProductDataParams::new("Sticker"),
            ),
            1,
        ),
    );
    assert!(matches!(
        bad_curr.validate(),
        Err(PaymentIntentValidationError::InvalidCurrency { .. })
    ));

    // Zero amount rejected (Gate G-Law minor currency integer > 0)
    let zero_amt = CreateCheckoutSessionParams::new("https://example.com/success").add_line_item(
        LineItemParams::from_price_data(
            LineItemPriceDataParams::with_amount(
                "usd",
                0,
                LineItemProductDataParams::new("Sticker"),
            ),
            1,
        ),
    );
    assert!(matches!(
        zero_amt.validate(),
        Err(PaymentIntentValidationError::InvalidAmount { amount: 0 })
    ));

    // Zero quantity rejected
    let zero_qty = CreateCheckoutSessionParams::new("https://example.com/success")
        .add_line_item(LineItemParams::new().quantity(0));
    assert!(matches!(
        zero_qty.validate(),
        Err(PaymentIntentValidationError::InvalidParameter {
            field: "line_items.quantity",
            ..
        })
    ));

    // Empty product name rejected
    let empty_name = CreateCheckoutSessionParams::new("https://example.com/success").add_line_item(
        LineItemParams::from_price_data(
            LineItemPriceDataParams::with_amount("usd", 500, LineItemProductDataParams::new("  ")),
            1,
        ),
    );
    assert!(matches!(
        empty_name.validate(),
        Err(PaymentIntentValidationError::InvalidParameter {
            field: "product_data.name",
            ..
        })
    ));
}
