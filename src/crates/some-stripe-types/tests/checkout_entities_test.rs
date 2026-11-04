//! Integration and Contract Test Suite for CheckoutSession & LineItem Domain Entities
//!
//! Validates:
//! - JSON deserialization of official Stripe CheckoutSession fixtures with expanded line items.
//! - Serialization roundtrip preserving field fidelity across all optional and mandatory fields.
//! - Default `object` values (`checkout.session`, `item`, `list`) when omitted in JSON payloads.
//! - Gate G-Borrow zero-clone borrowing invariants for line item inspection and iteration.
//! - EventObject discriminated union integration with expanded CheckoutSession events.

use some_stripe_types::{
    CHECKOUT_SESSION_MODE_PAYMENT, CHECKOUT_SESSION_MODE_SETUP, CHECKOUT_SESSION_MODE_SUBSCRIPTION,
    CHECKOUT_SESSION_STATUS_COMPLETE, CHECKOUT_SESSION_STATUS_EXPIRED,
    CHECKOUT_SESSION_STATUS_OPEN, CheckoutSession, CreateCheckoutSessionParams, EventData,
    EventObject, LineItem, LineItemList, PAYMENT_STATUS_NO_PAYMENT_REQUIRED, PAYMENT_STATUS_PAID,
    PAYMENT_STATUS_UNPAID,
};
use std::collections::HashMap;

/// Official Stripe CheckoutSession fixture with expanded line items matching Stripe API documentation.
const OFFICIAL_STRIPE_CHECKOUT_SESSION_FIXTURE: &str = r#"{
  "id": "cs_test_a1b2c3d4e5f6g7h8",
  "object": "checkout.session",
  "after_expiration": null,
  "allow_promotion_codes": null,
  "amount_subtotal": 2198,
  "amount_total": 2198,
  "automatic_tax": {
    "enabled": false,
    "liability": null,
    "status": null
  },
  "billing_address_collection": null,
  "cancel_url": "https://example.com/cancel",
  "client_reference_id": "client_ref_001",
  "client_secret": null,
  "consent": null,
  "consent_collection": null,
  "created": 1679000000,
  "currency": "usd",
  "currency_conversion": null,
  "custom_fields": [],
  "custom_text": {
    "shipping_address": null,
    "submit": null
  },
  "customer": "cus_12345",
  "customer_creation": null,
  "customer_details": null,
  "customer_email": null,
  "expires_at": 1679086400,
  "invoice": null,
  "invoice_creation": null,
  "line_items": {
    "object": "list",
    "data": [
      {
        "id": "li_1N2O3P4Q5R6S7T8U",
        "object": "item",
        "amount_discount": 0,
        "amount_subtotal": 1099,
        "amount_tax": 0,
        "amount_total": 1099,
        "currency": "usd",
        "description": "Widget A",
        "price": {
          "id": "price_1N2O3P4Q5R6S7T8U",
          "object": "price",
          "active": true,
          "billing_scheme": "per_unit",
          "created": 1679000000,
          "currency": "usd",
          "unit_amount": 1099
        },
        "quantity": 1
      },
      {
        "id": "li_2N2O3P4Q5R6S7T8V",
        "object": "item",
        "amount_discount": 50,
        "amount_subtotal": 1149,
        "amount_tax": 99,
        "amount_total": 1198,
        "currency": "usd",
        "description": "Widget B Deluxe",
        "price": {
          "id": "price_2N2O3P4Q5R6S7T8V",
          "object": "price",
          "active": true,
          "billing_scheme": "per_unit",
          "created": 1679000000,
          "currency": "usd",
          "unit_amount": 1149
        },
        "quantity": 2
      }
    ],
    "has_more": false,
    "url": "/v1/checkout/sessions/cs_test_a1b2c3d4e5f6g7h8/line_items"
  },
  "livemode": false,
  "locale": null,
  "metadata": {
    "order_id": "ord_98765",
    "environment": "staging"
  },
  "mode": "payment",
  "payment_intent": "pi_1234567890",
  "payment_link": null,
  "payment_method_collection": "always",
  "payment_method_configuration_details": null,
  "payment_method_options": {},
  "payment_method_types": [
    "card"
  ],
  "payment_status": "unpaid",
  "phone_number_collection": {
    "enabled": false
  },
  "recovered_from": null,
  "saved_payment_method_options": null,
  "setup_intent": null,
  "shipping_address_collection": null,
  "shipping_cost": null,
  "shipping_details": null,
  "shipping_options": [],
  "status": "open",
  "submit_type": null,
  "subscription": null,
  "success_url": "https://example.com/success",
  "total_details": {
    "amount_discount": 50,
    "amount_shipping": 0,
    "amount_tax": 99
  },
  "ui_mode": "hosted",
  "url": "https://checkout.stripe.com/c/pay/cs_test_a1b2c3d4e5f6g7h8"
}"#;

#[test]
fn test_checkout_session_official_fixture_with_line_items() {
    let session: CheckoutSession = serde_json::from_str(OFFICIAL_STRIPE_CHECKOUT_SESSION_FIXTURE)
        .expect("Official Stripe CheckoutSession fixture must deserialize cleanly");

    // 1. Session scalar fields
    assert_eq!(session.id, "cs_test_a1b2c3d4e5f6g7h8");
    assert_eq!(session.object, "checkout.session");
    assert_eq!(
        session.url.as_deref(),
        Some("https://checkout.stripe.com/c/pay/cs_test_a1b2c3d4e5f6g7h8")
    );
    assert_eq!(session.status.as_deref(), Some("open"));
    assert_eq!(session.payment_status.as_deref(), Some("unpaid"));
    assert_eq!(session.mode.as_deref(), Some("payment"));
    assert_eq!(session.amount_total, Some(2198));
    assert_eq!(session.amount_subtotal, Some(2198));
    assert_eq!(session.currency.as_deref(), Some("usd"));
    assert_eq!(session.customer.as_deref(), Some("cus_12345"));
    assert_eq!(
        session.client_reference_id.as_deref(),
        Some("client_ref_001")
    );
    assert_eq!(
        session.success_url.as_deref(),
        Some("https://example.com/success")
    );
    assert_eq!(
        session.cancel_url.as_deref(),
        Some("https://example.com/cancel")
    );
    assert_eq!(session.payment_intent.as_deref(), Some("pi_1234567890"));
    assert_eq!(session.expires_at, Some(1679086400));
    assert_eq!(session.created, Some(1679000000));

    // 2. Metadata verification
    let metadata = session.metadata.as_ref().expect("Metadata must be present");
    assert_eq!(
        metadata.get("order_id").map(String::as_str),
        Some("ord_98765")
    );
    assert_eq!(
        metadata.get("environment").map(String::as_str),
        Some("staging")
    );

    // 3. Lifecycle helper assertions
    assert!(session.is_open());
    assert!(!session.is_complete());
    assert!(!session.is_expired());
    assert!(session.is_unpaid());
    assert!(!session.is_paid());
    assert!(!session.is_no_payment_required());
    assert!(session.is_payment_mode());
    assert!(!session.is_setup_mode());
    assert!(!session.is_subscription_mode());

    // 4. Line items list verification
    let line_items = session
        .line_items
        .as_ref()
        .expect("Line items must be present in expanded fixture");
    assert_eq!(line_items.object, "list");
    assert!(!line_items.has_more);
    assert_eq!(
        line_items.url,
        "/v1/checkout/sessions/cs_test_a1b2c3d4e5f6g7h8/line_items"
    );
    assert_eq!(line_items.len(), 2);
    assert!(!line_items.is_empty());

    // 5. Line items items verification
    let item1 = &line_items[0];
    assert_eq!(item1.id, "li_1N2O3P4Q5R6S7T8U");
    assert_eq!(item1.object, "item");
    assert_eq!(item1.description.as_deref(), Some("Widget A"));
    assert_eq!(item1.amount_subtotal, 1099);
    assert_eq!(item1.amount_total, 1099);
    assert_eq!(item1.amount_tax, Some(0));
    assert_eq!(item1.amount_discount, Some(0));
    assert_eq!(item1.currency, "usd");
    assert_eq!(item1.quantity, Some(1));

    let item2 = &line_items[1];
    assert_eq!(item2.id, "li_2N2O3P4Q5R6S7T8V");
    assert_eq!(item2.object, "item");
    assert_eq!(item2.description.as_deref(), Some("Widget B Deluxe"));
    assert_eq!(item2.amount_subtotal, 1149);
    assert_eq!(item2.amount_total, 1198);
    assert_eq!(item2.amount_tax, Some(99));
    assert_eq!(item2.amount_discount, Some(50));
    assert_eq!(item2.currency, "usd");
    assert_eq!(item2.quantity, Some(2));

    // 6. Gate G-Borrow slice accessor verification
    let slice = session.line_items_data().expect("Should borrow slice");
    assert_eq!(slice.len(), 2);
    assert_eq!(slice[0].id, "li_1N2O3P4Q5R6S7T8U");
    assert_eq!(slice[1].id, "li_2N2O3P4Q5R6S7T8V");
}

#[test]
fn test_checkout_session_serialization_roundtrip_field_fidelity() {
    let mut metadata = HashMap::new();
    metadata.insert("tenant_id".into(), "cust_alpha_1".into());
    metadata.insert("source".into(), "web_app".into());

    let item1 = LineItem::new("li_rt_1", 3000, 3300, "eur")
        .with_description("Annual Subscription")
        .with_quantity(1)
        .with_tax(300)
        .with_discount(0);

    let item2 = LineItem::new("li_rt_2", 1000, 1000, "eur")
        .with_description("Setup Fee")
        .with_quantity(1);

    let line_items = LineItemList::new(
        vec![item1, item2],
        false,
        "/v1/checkout/sessions/cs_roundtrip_test/line_items",
    );

    let session = CheckoutSession {
        id: "cs_roundtrip_test".into(),
        object: "checkout.session".into(),
        url: Some("https://checkout.stripe.com/pay/cs_roundtrip_test".into()),
        status: Some(CHECKOUT_SESSION_STATUS_OPEN.into()),
        payment_status: Some(PAYMENT_STATUS_UNPAID.into()),
        mode: Some(CHECKOUT_SESSION_MODE_PAYMENT.into()),
        amount_total: Some(4300),
        amount_subtotal: Some(4000),
        currency: Some("eur".into()),
        customer: Some("cus_buyer_999".into()),
        client_reference_id: Some("ref_invoice_44".into()),
        success_url: Some("https://mysite.com/done".into()),
        cancel_url: Some("https://mysite.com/aborted".into()),
        payment_intent: Some("pi_live_intent_007".into()),
        expires_at: Some(1728500000),
        created: Some(1728400000),
        metadata: Some(metadata),
        line_items: Some(line_items),
    };

    // Serialize to JSON string
    let serialized =
        serde_json::to_string_pretty(&session).expect("CheckoutSession serialization must succeed");

    // Deserialize back into CheckoutSession
    let deserialized: CheckoutSession = serde_json::from_str(&serialized)
        .expect("CheckoutSession deserialization from serialized JSON must succeed");

    // Full equality assertion
    assert_eq!(session, deserialized);

    // Verify key fields
    assert_eq!(deserialized.id, "cs_roundtrip_test");
    assert_eq!(deserialized.object, "checkout.session");
    assert_eq!(deserialized.amount_total, Some(4300));
    assert_eq!(deserialized.amount_subtotal, Some(4000));
    assert_eq!(deserialized.currency.as_deref(), Some("eur"));
    assert_eq!(deserialized.mode.as_deref(), Some("payment"));
    assert_eq!(deserialized.line_items.as_ref().unwrap().len(), 2);
    assert_eq!(
        deserialized.line_items.as_ref().unwrap()[0]
            .description
            .as_deref(),
        Some("Annual Subscription")
    );
    assert_eq!(
        deserialized
            .metadata
            .as_ref()
            .unwrap()
            .get("tenant_id")
            .map(String::as_str),
        Some("cust_alpha_1")
    );
}

#[test]
fn test_default_object_values_omitted_in_json() {
    // 1. CheckoutSession without "object" field
    let cs_raw = r#"{
        "id": "cs_default_obj_test",
        "status": "open",
        "payment_status": "unpaid",
        "amount_total": 5000,
        "currency": "usd"
    }"#;
    let cs: CheckoutSession =
        serde_json::from_str(cs_raw).expect("CheckoutSession without object must deserialize");
    assert_eq!(
        cs.object, "checkout.session",
        "CheckoutSession object must default to 'checkout.session'"
    );
    assert_eq!(cs.id, "cs_default_obj_test");
    assert_eq!(cs.amount_total, Some(5000));

    // 2. LineItem without "object" field
    let li_raw = r#"{
        "id": "li_default_obj_test",
        "amount_subtotal": 2000,
        "amount_total": 2000,
        "currency": "usd"
    }"#;
    let li: LineItem =
        serde_json::from_str(li_raw).expect("LineItem without object must deserialize");
    assert_eq!(li.object, "item", "LineItem object must default to 'item'");
    assert_eq!(li.id, "li_default_obj_test");
    assert_eq!(li.amount_total, 2000);
    assert_eq!(li.amount_subtotal, 2000);
    assert_eq!(li.currency, "usd");
    assert!(li.description.is_none());
    assert!(li.quantity.is_none());
    assert!(li.amount_tax.is_none());
    assert!(li.amount_discount.is_none());

    // 3. LineItemList without "object" field
    let list_raw = r#"{
        "data": [
            {
                "id": "li_nested_1",
                "amount_subtotal": 1000,
                "amount_total": 1000,
                "currency": "usd"
            }
        ],
        "has_more": false,
        "url": "/v1/checkout/sessions/cs_xyz/line_items"
    }"#;
    let list: LineItemList =
        serde_json::from_str(list_raw).expect("LineItemList without object must deserialize");
    assert_eq!(
        list.object, "list",
        "LineItemList object must default to 'list'"
    );
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].object, "item");
}

#[test]
fn test_line_item_and_list_serialization_roundtrip() {
    let item = LineItem::new("li_single_test", 4500, 4950, "gbp")
        .with_description("Professional Consultation")
        .with_quantity(3)
        .with_tax(450)
        .with_discount(0);

    let serialized = serde_json::to_string(&item).expect("LineItem serialization must succeed");
    let deserialized: LineItem =
        serde_json::from_str(&serialized).expect("LineItem deserialization must succeed");

    assert_eq!(item, deserialized);
    assert_eq!(deserialized.id, "li_single_test");
    assert_eq!(deserialized.object, "item");
    assert_eq!(deserialized.amount_total, 4950);
    assert_eq!(deserialized.amount_subtotal, 4500);
    assert_eq!(deserialized.amount_tax, Some(450));
    assert_eq!(deserialized.amount_discount, Some(0));
    assert_eq!(deserialized.currency, "gbp");
    assert_eq!(deserialized.quantity, Some(3));
    assert_eq!(
        deserialized.description.as_deref(),
        Some("Professional Consultation")
    );

    // Roundtrip LineItemList
    let list = LineItemList::new(vec![item], true, "/v1/line_items?page=1");
    let list_serialized =
        serde_json::to_string(&list).expect("LineItemList serialization must succeed");
    let list_deserialized: LineItemList =
        serde_json::from_str(&list_serialized).expect("LineItemList deserialization must succeed");

    assert_eq!(list, list_deserialized);
    assert_eq!(list_deserialized.object, "list");
    assert!(list_deserialized.has_more);
    assert_eq!(list_deserialized.url, "/v1/line_items?page=1");
    assert_eq!(list_deserialized.len(), 1);
}

#[test]
fn test_gate_g_borrow_zero_clone_invariants() {
    let item1 = LineItem::new("li_g_borrow_1", 100, 100, "usd");
    let item2 = LineItem::new("li_g_borrow_2", 200, 200, "usd");
    let list = LineItemList::new(vec![item1, item2], false, "/v1/items");

    // Borrow slice via Deref without cloning
    let slice: &[LineItem] = &list;
    assert_eq!(slice.len(), 2);
    assert_eq!(slice[0].id, "li_g_borrow_1");

    // Borrow slice via AsRef
    let as_ref_slice: &[LineItem] = list.as_ref();
    assert_eq!(as_ref_slice.len(), 2);

    // Borrow slice via as_slice
    assert_eq!(list.as_slice().len(), 2);

    // Iterate references directly (Gate G-Borrow)
    let ids: Vec<&str> = (&list).into_iter().map(|item| item.id.as_str()).collect();
    assert_eq!(ids, vec!["li_g_borrow_1", "li_g_borrow_2"]);

    // Session line_items_data zero-clone borrowing
    let session = CheckoutSession::new("cs_borrow_test").with_line_items(list);
    let session_slice = session
        .line_items_data()
        .expect("line_items_data must borrow slice");
    assert_eq!(session_slice.len(), 2);
    assert_eq!(session_slice[1].id, "li_g_borrow_2");
}

#[test]
fn test_checkout_session_status_and_mode_matrix() {
    let mut cs = CheckoutSession::new("cs_matrix_test");

    // Open & unpaid & payment mode
    cs.status = Some(CHECKOUT_SESSION_STATUS_OPEN.into());
    cs.payment_status = Some(PAYMENT_STATUS_UNPAID.into());
    cs.mode = Some(CHECKOUT_SESSION_MODE_PAYMENT.into());
    assert!(cs.is_open());
    assert!(!cs.is_complete());
    assert!(!cs.is_expired());
    assert!(cs.is_unpaid());
    assert!(!cs.is_paid());
    assert!(!cs.is_no_payment_required());
    assert!(cs.is_payment_mode());
    assert!(!cs.is_setup_mode());
    assert!(!cs.is_subscription_mode());

    // Complete & paid & subscription mode
    cs.status = Some(CHECKOUT_SESSION_STATUS_COMPLETE.into());
    cs.payment_status = Some(PAYMENT_STATUS_PAID.into());
    cs.mode = Some(CHECKOUT_SESSION_MODE_SUBSCRIPTION.into());
    assert!(!cs.is_open());
    assert!(cs.is_complete());
    assert!(!cs.is_expired());
    assert!(!cs.is_unpaid());
    assert!(cs.is_paid());
    assert!(!cs.is_no_payment_required());
    assert!(!cs.is_payment_mode());
    assert!(!cs.is_setup_mode());
    assert!(cs.is_subscription_mode());

    // Expired & no_payment_required & setup mode
    cs.status = Some(CHECKOUT_SESSION_STATUS_EXPIRED.into());
    cs.payment_status = Some(PAYMENT_STATUS_NO_PAYMENT_REQUIRED.into());
    cs.mode = Some(CHECKOUT_SESSION_MODE_SETUP.into());
    assert!(!cs.is_open());
    assert!(!cs.is_complete());
    assert!(cs.is_expired());
    assert!(!cs.is_unpaid());
    assert!(!cs.is_paid());
    assert!(cs.is_no_payment_required());
    assert!(!cs.is_payment_mode());
    assert!(cs.is_setup_mode());
    assert!(!cs.is_subscription_mode());
}

#[test]
fn test_event_object_checkout_session_with_line_items() {
    let event_payload = serde_json::json!({
        "id": "cs_event_test_777",
        "object": "checkout.session",
        "status": "complete",
        "payment_status": "paid",
        "mode": "payment",
        "amount_total": 8500,
        "amount_subtotal": 8500,
        "currency": "usd",
        "customer": "cus_evt_customer",
        "line_items": {
            "object": "list",
            "data": [
                {
                    "id": "li_evt_item_1",
                    "object": "item",
                    "description": "Premium License",
                    "amount_subtotal": 8500,
                    "amount_total": 8500,
                    "currency": "usd",
                    "quantity": 1
                }
            ],
            "has_more": false,
            "url": "/v1/checkout/sessions/cs_event_test_777/line_items"
        }
    });

    let data = EventData {
        object: event_payload,
    };
    let event_obj = data
        .deserialize_object()
        .expect("EventData must deserialize into EventObject");

    match event_obj {
        EventObject::CheckoutSession(cs) => {
            assert_eq!(cs.id, "cs_event_test_777");
            assert_eq!(cs.status.as_deref(), Some("complete"));
            assert_eq!(cs.payment_status.as_deref(), Some("paid"));
            assert_eq!(cs.mode.as_deref(), Some("payment"));
            assert_eq!(cs.amount_total, Some(8500));
            assert_eq!(cs.amount_subtotal, Some(8500));
            assert_eq!(cs.currency.as_deref(), Some("usd"));
            assert_eq!(cs.customer.as_deref(), Some("cus_evt_customer"));
            assert!(cs.is_complete());
            assert!(cs.is_paid());

            let line_items = cs.line_items.expect("Line items must be present");
            assert_eq!(line_items.len(), 1);
            assert_eq!(line_items[0].id, "li_evt_item_1");
            assert_eq!(
                line_items[0].description.as_deref(),
                Some("Premium License")
            );
        }
        other => panic!("Expected EventObject::CheckoutSession, got {other:?}"),
    }
}

#[test]
fn test_create_checkout_session_params_defaults_and_serde() {
    let params = CreateCheckoutSessionParams {
        success_url: "https://example.com/checkout/success".into(),
        cancel_url: Some("https://example.com/checkout/cancel".into()),
        mode: "payment".into(),
        client_reference_id: Some("user_99".into()),
        currency: Some("usd".into()),
        unit_amount: Some(1500),
        product_name: Some("Coffee Mug".into()),
        quantity: Some(1),
        expires_at: Some(1728400000),
        idempotency_key: Some("idem_key_coffee".into()),
        ..Default::default()
    };

    let serialized = serde_json::to_string(&params)
        .expect("CreateCheckoutSessionParams serialization must succeed");
    // Idempotency key must be skipped in JSON
    assert!(!serialized.contains("idempotency_key"));
    assert!(!serialized.contains("idem_key_coffee"));

    let deserialized: CreateCheckoutSessionParams = serde_json::from_str(&serialized)
        .expect("CreateCheckoutSessionParams deserialization must succeed");

    assert_eq!(
        deserialized.success_url,
        "https://example.com/checkout/success"
    );
    assert_eq!(
        deserialized.cancel_url.as_deref(),
        Some("https://example.com/checkout/cancel")
    );
    assert_eq!(deserialized.mode, "payment");
    assert_eq!(deserialized.currency.as_deref(), Some("usd"));
    assert_eq!(deserialized.unit_amount, Some(1500));
    assert_eq!(deserialized.product_name.as_deref(), Some("Coffee Mug"));
    assert_eq!(deserialized.quantity, Some(1));
    assert_eq!(deserialized.expires_at, Some(1728400000));
    assert!(deserialized.idempotency_key.is_none());
}
