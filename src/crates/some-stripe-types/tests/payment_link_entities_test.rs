//! # PaymentLink Domain Entity & Status State Machine Integration Tests
//!
//! Comprehensive tests verifying:
//! 1. JSON deserialization of official Stripe PaymentLink fixtures (full, minimal, null fields, extra fields).
//! 2. Serde serialization roundtrip without loss of information.
//! 3. Default `object` values (`"payment_link"`).
//! 4. Strongly typed `PaymentLinkStatus` state machine transitions (active <-> inactive).
//! 5. Entity predicate methods (`is_active`, `status`, `activate`, `deactivate`, `toggle_active`, `transition_to`).
//! 6. Gate G-Borrow zero-clone borrowing discipline.

use some_stripe_types::{
    PaymentLink, PaymentLinkAfterCompletionParams, PaymentLinkId, PaymentLinkStatus,
    default_payment_link_object,
};
use std::collections::HashMap;

/// Official Stripe PaymentLink JSON fixture containing all canonical fields.
const OFFICIAL_PAYMENT_LINK_JSON: &str = r#"{
  "id": "plink_1N42iV2eZvKYlo2CUbdf5V4L",
  "object": "payment_link",
  "active": true,
  "after_completion": {
    "hosted_confirmation": {
      "custom_message": "Thank you for supporting our open-source project!"
    },
    "type": "hosted_confirmation"
  },
  "allow_promotion_codes": true,
  "billing_address_collection": "auto",
  "currency": "usd",
  "customer_creation": "if_required",
  "livemode": false,
  "metadata": {
    "campaign": "annual_giving_2026",
    "source": "newsletter"
  },
  "payment_method_types": [
    "card",
    "link"
  ],
  "shipping_address_collection": {
    "allowed_countries": [
      "US",
      "CA",
      "GB"
    ]
  },
  "url": "https://buy.stripe.com/test_1N42iV2eZvKYlo2CUbdf5V4L"
}"#;

/// Official Stripe PaymentLink JSON fixture for a deactivated link with redirect completion.
const OFFICIAL_INACTIVE_PAYMENT_LINK_JSON: &str = r#"{
  "id": "plink_1N42iW2eZvKYlo2CABC999",
  "object": "payment_link",
  "active": false,
  "after_completion": {
    "redirect": {
      "url": "https://example.com/checkout/success"
    },
    "type": "redirect"
  },
  "allow_promotion_codes": false,
  "billing_address_collection": "required",
  "currency": "eur",
  "customer_creation": "always",
  "livemode": true,
  "metadata": {},
  "payment_method_types": [
    "card"
  ],
  "shipping_address_collection": null,
  "url": "https://buy.stripe.com/test_1N42iW2eZvKYlo2CABC999"
}"#;

#[test]
fn test_payment_link_deserialization_official_stripe_fixture() {
    let link: PaymentLink = serde_json::from_str(OFFICIAL_PAYMENT_LINK_JSON)
        .expect("Failed to deserialize canonical Stripe PaymentLink fixture");

    assert_eq!(link.id, "plink_1N42iV2eZvKYlo2CUbdf5V4L");
    assert_eq!(link.object, "payment_link");
    assert!(link.active);
    assert!(link.is_active());
    assert_eq!(link.status(), PaymentLinkStatus::Active);
    assert_eq!(
        link.url,
        "https://buy.stripe.com/test_1N42iV2eZvKYlo2CUbdf5V4L"
    );
    assert_eq!(link.currency.as_deref(), Some("usd"));
    assert_eq!(link.allow_promotion_codes, Some(true));
    assert_eq!(link.billing_address_collection.as_deref(), Some("auto"));
    assert_eq!(link.customer_creation.as_deref(), Some("if_required"));
    assert_eq!(link.livemode, Some(false));

    // Verify metadata mapping
    let metadata = link.metadata.expect("Expected metadata to be populated");
    assert_eq!(
        metadata.get("campaign").map(String::as_str),
        Some("annual_giving_2026")
    );
    assert_eq!(
        metadata.get("source").map(String::as_str),
        Some("newsletter")
    );

    // Verify payment_method_types
    let pm_types = link
        .payment_method_types
        .expect("Expected payment_method_types to be present");
    assert_eq!(pm_types, vec!["card".to_string(), "link".to_string()]);

    // Verify after_completion payload
    let after_comp = link
        .after_completion
        .as_ref()
        .expect("Expected after_completion payload");
    assert_eq!(after_comp.r#type, "hosted_confirmation");
    assert_eq!(
        after_comp
            .hosted_confirmation
            .as_ref()
            .and_then(|h| h.custom_message.as_deref()),
        Some("Thank you for supporting our open-source project!")
    );

    // Verify shipping_address_collection
    let shipping = link
        .shipping_address_collection
        .expect("Expected shipping_address_collection payload");
    let countries = shipping["allowed_countries"]
        .as_array()
        .expect("Expected allowed_countries array");
    assert_eq!(countries.len(), 3);
}

#[test]
fn test_payment_link_deserialization_inactive_fixture() {
    let link: PaymentLink = serde_json::from_str(OFFICIAL_INACTIVE_PAYMENT_LINK_JSON)
        .expect("Failed to deserialize inactive PaymentLink fixture");

    assert_eq!(link.id, "plink_1N42iW2eZvKYlo2CABC999");
    assert_eq!(link.object, "payment_link");
    assert!(!link.active);
    assert!(!link.is_active());
    assert_eq!(link.status(), PaymentLinkStatus::Inactive);
    assert_eq!(link.currency.as_deref(), Some("eur"));
    assert_eq!(link.allow_promotion_codes, Some(false));
    assert_eq!(link.billing_address_collection.as_deref(), Some("required"));
    assert_eq!(link.customer_creation.as_deref(), Some("always"));
    assert_eq!(link.livemode, Some(true));
    assert!(link.shipping_address_collection.is_none());

    let after_comp = link
        .after_completion
        .as_ref()
        .expect("Expected after_completion");
    assert_eq!(after_comp.r#type, "redirect");
    assert_eq!(
        after_comp.redirect.as_ref().map(|r| r.url.as_str()),
        Some("https://example.com/checkout/success")
    );
}

#[test]
fn test_payment_link_deserialization_minimal_fixture_defaults() {
    let minimal_json = r#"{
      "id": "plink_minimal_001",
      "active": true,
      "url": "https://buy.stripe.com/test_min_001"
    }"#;

    let link: PaymentLink = serde_json::from_str(minimal_json)
        .expect("Failed to deserialize minimal PaymentLink fixture");

    assert_eq!(link.id, "plink_minimal_001");
    // Default object must be "payment_link"
    assert_eq!(link.object, "payment_link");
    assert!(link.active);
    assert!(link.is_active());
    assert_eq!(link.status(), PaymentLinkStatus::Active);
    assert_eq!(link.url, "https://buy.stripe.com/test_min_001");

    // All optional fields must default to None
    assert!(link.currency.is_none());
    assert!(link.metadata.is_none());
    assert!(link.payment_method_types.is_none());
    assert!(link.allow_promotion_codes.is_none());
    assert!(link.billing_address_collection.is_none());
    assert!(link.shipping_address_collection.is_none());
    assert!(link.customer_creation.is_none());
    assert!(link.after_completion.is_none());
    assert!(link.livemode.is_none());
}

#[test]
fn test_payment_link_deserialization_with_null_and_unknown_fields() {
    let wire_json = r#"{
      "id": "plink_wire_123",
      "object": "payment_link",
      "active": false,
      "url": "https://buy.stripe.com/test_wire_123",
      "currency": null,
      "metadata": null,
      "payment_method_types": null,
      "allow_promotion_codes": null,
      "billing_address_collection": null,
      "shipping_address_collection": null,
      "customer_creation": null,
      "after_completion": null,
      "livemode": null,
      "created": 1729000000,
      "custom_text": {
        "shipping_address": null,
        "submit": null
      },
      "line_items": {
        "object": "list",
        "data": [],
        "has_more": false
      }
    }"#;

    let link: PaymentLink = serde_json::from_str(wire_json)
        .expect("Deserialization with null and unknown fields should succeed");

    assert_eq!(link.id, "plink_wire_123");
    assert_eq!(link.object, "payment_link");
    assert!(!link.is_active());
    assert_eq!(link.status(), PaymentLinkStatus::Inactive);
    assert!(link.currency.is_none());
    assert!(link.metadata.is_none());
    assert!(link.payment_method_types.is_none());
}

#[test]
fn test_payment_link_serialization_roundtrip_full() {
    let mut metadata = HashMap::new();
    metadata.insert("product_ref".to_string(), "prod_xyz789".to_string());
    metadata.insert("environment".to_string(), "production".to_string());

    let original = PaymentLink {
        id: "plink_roundtrip_full".to_string(),
        object: "payment_link".to_string(),
        active: true,
        url: "https://buy.stripe.com/roundtrip_full".to_string(),
        currency: Some("usd".to_string()),
        metadata: Some(metadata),
        payment_method_types: Some(vec!["card".to_string(), "cashapp".to_string()]),
        allow_promotion_codes: Some(true),
        billing_address_collection: Some("auto".to_string()),
        shipping_address_collection: Some(serde_json::json!({
            "allowed_countries": ["US", "DE"]
        })),
        customer_creation: Some("always".to_string()),
        after_completion: Some(PaymentLinkAfterCompletionParams::hosted_confirmation(Some(
            "Order confirmed!".into(),
        ))),
        livemode: Some(true),
    };

    let serialized = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: PaymentLink =
        serde_json::from_str(&serialized).expect("Deserialization roundtrip failed");

    assert_eq!(original, deserialized);
}

#[test]
fn test_payment_link_serialization_roundtrip_minimal() {
    let original = PaymentLink::new("plink_min_rt", "https://buy.stripe.com/min_rt", false);

    assert!(!original.is_active());
    assert_eq!(original.status(), PaymentLinkStatus::Inactive);

    let serialized = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: PaymentLink =
        serde_json::from_str(&serialized).expect("Deserialization roundtrip failed");

    assert_eq!(original, deserialized);
    assert_eq!(deserialized.object, "payment_link");
    assert!(!deserialized.is_active());
}

#[test]
fn test_default_object_value_invariants() {
    // default function directly
    assert_eq!(default_payment_link_object(), "payment_link");

    // PaymentLink::new defaults object to "payment_link"
    let link = PaymentLink::new("plink_test", "https://buy.stripe.com/test", true);
    assert_eq!(link.object, "payment_link");

    // PaymentLink::default defaults object to "payment_link"
    let default_link = PaymentLink::default();
    assert_eq!(default_link.object, "payment_link");
    assert!(default_link.active);
    assert!(default_link.is_active());

    // Deserialization without object field populates default
    let json_no_obj =
        r#"{"id":"plink_no_obj","active":true,"url":"https://buy.stripe.com/no_obj"}"#;
    let parsed: PaymentLink =
        serde_json::from_str(json_no_obj).expect("Deserialization without object should pass");
    assert_eq!(parsed.object, "payment_link");
}

#[test]
fn test_payment_link_status_enum_invariants_and_transitions() {
    let active = PaymentLinkStatus::Active;
    let inactive = PaymentLinkStatus::Inactive;

    // as_str
    assert_eq!(active.as_str(), "active");
    assert_eq!(inactive.as_str(), "inactive");

    // predicates
    assert!(active.is_active());
    assert!(!active.is_inactive());
    assert!(!inactive.is_active());
    assert!(inactive.is_inactive());

    // toggle
    assert_eq!(active.toggle(), PaymentLinkStatus::Inactive);
    assert_eq!(inactive.toggle(), PaymentLinkStatus::Active);

    // explicit transition methods
    assert_eq!(inactive.activate(), PaymentLinkStatus::Active);
    assert_eq!(active.deactivate(), PaymentLinkStatus::Inactive);
    assert_eq!(
        active.transition_to(PaymentLinkStatus::Inactive),
        PaymentLinkStatus::Inactive
    );
    assert_eq!(
        inactive.transition_to(PaymentLinkStatus::Active),
        PaymentLinkStatus::Active
    );
    assert!(active.can_transition_to(PaymentLinkStatus::Inactive));
    assert!(inactive.can_transition_to(PaymentLinkStatus::Active));

    // conversions: bool <-> PaymentLinkStatus
    assert_eq!(PaymentLinkStatus::from(true), PaymentLinkStatus::Active);
    assert_eq!(PaymentLinkStatus::from(false), PaymentLinkStatus::Inactive);
    assert!(bool::from(PaymentLinkStatus::Active));
    assert!(!bool::from(PaymentLinkStatus::Inactive));

    // FromStr
    assert_eq!(
        PaymentLinkStatus::from_str("active").unwrap(),
        PaymentLinkStatus::Active
    );
    assert_eq!(
        PaymentLinkStatus::from_str("inactive").unwrap(),
        PaymentLinkStatus::Inactive
    );
    assert_eq!(
        PaymentLinkStatus::from_str("true").unwrap(),
        PaymentLinkStatus::Active
    );
    assert_eq!(
        PaymentLinkStatus::from_str("false").unwrap(),
        PaymentLinkStatus::Inactive
    );
    assert!(PaymentLinkStatus::from_str("expired").is_err());

    // Display
    assert_eq!(format!("{active}"), "active");
    assert_eq!(format!("{inactive}"), "inactive");

    // AsRef<str>
    assert_eq!(active.as_ref(), "active");
    assert_eq!(inactive.as_ref(), "inactive");

    // PartialEq helpers
    assert_eq!(active, "active");
    assert_eq!("active", active);
    assert_eq!(active, "active".to_string());
    assert_eq!("active".to_string(), active);
    assert_eq!(active, true);
    assert_eq!(true, active);
    assert_eq!(inactive, "inactive");
    assert_eq!("inactive", inactive);
    assert_eq!(inactive, false);
    assert_eq!(false, inactive);

    // Serde roundtrip for enum
    let json_active = serde_json::to_string(&active).unwrap();
    assert_eq!(json_active, "\"active\"");
    let de_active: PaymentLinkStatus = serde_json::from_str(&json_active).unwrap();
    assert_eq!(de_active, PaymentLinkStatus::Active);

    let json_inactive = serde_json::to_string(&inactive).unwrap();
    assert_eq!(json_inactive, "\"inactive\"");
    let de_inactive: PaymentLinkStatus = serde_json::from_str(&json_inactive).unwrap();
    assert_eq!(de_inactive, PaymentLinkStatus::Inactive);
}

#[test]
fn test_payment_link_entity_state_machine_mutations() {
    let mut link = PaymentLink::new("plink_sm_01", "https://buy.stripe.com/test_sm_01", true);

    // Initial state: Active
    assert!(link.is_active());
    assert_eq!(link.status(), PaymentLinkStatus::Active);

    // Deactivate
    let new_status = link.deactivate();
    assert_eq!(new_status, PaymentLinkStatus::Inactive);
    assert!(!link.is_active());
    assert_eq!(link.status(), PaymentLinkStatus::Inactive);

    // Reactivate
    let reactivated = link.activate();
    assert_eq!(reactivated, PaymentLinkStatus::Active);
    assert!(link.is_active());
    assert_eq!(link.status(), PaymentLinkStatus::Active);

    // Toggle active
    let toggled = link.toggle_active();
    assert_eq!(toggled, PaymentLinkStatus::Inactive);
    assert!(!link.is_active());

    let toggled_back = link.toggle_active();
    assert_eq!(toggled_back, PaymentLinkStatus::Active);
    assert!(link.is_active());

    // Explicit transition_to
    let transitioned = link.transition_to(PaymentLinkStatus::Inactive);
    assert_eq!(transitioned, PaymentLinkStatus::Inactive);
    assert!(!link.is_active());

    let transitioned_back = link.transition_to(PaymentLinkStatus::Active);
    assert_eq!(transitioned_back, PaymentLinkStatus::Active);
    assert!(link.is_active());
}

#[test]
fn test_payment_link_fluent_builder_chain() {
    let mut meta = HashMap::new();
    meta.insert("internal_order_id".to_string(), "ord_999".to_string());

    let link = PaymentLink::new("plink_builder", "https://buy.stripe.com/builder", true)
        .with_currency("eur")
        .with_metadata(meta)
        .with_payment_method_types(vec!["card".to_string(), "sepa_debit".to_string()])
        .with_allow_promotion_codes(true)
        .with_billing_address_collection("required")
        .with_shipping_address_collection(serde_json::json!({ "allowed_countries": ["FR", "IT"] }))
        .with_customer_creation("always")
        .with_after_completion(PaymentLinkAfterCompletionParams::hosted_confirmation(Some(
            "Thank you!".into(),
        )))
        .with_livemode(false);

    assert_eq!(link.id, "plink_builder");
    assert_eq!(link.currency.as_deref(), Some("eur"));
    assert_eq!(link.allow_promotion_codes, Some(true));
    assert_eq!(link.billing_address_collection.as_deref(), Some("required"));
    assert_eq!(link.customer_creation.as_deref(), Some("always"));
    assert_eq!(link.livemode, Some(false));
    assert!(
        link.metadata
            .as_ref()
            .unwrap()
            .contains_key("internal_order_id")
    );
}

#[test]
fn test_payment_link_id_type() {
    let id = PaymentLinkId::from("plink_strongly_typed");
    assert_eq!(id.to_string(), "plink_strongly_typed");
    assert_eq!(id.as_ref(), "plink_strongly_typed");

    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"plink_strongly_typed\"");
    let de: PaymentLinkId = serde_json::from_str(&json).unwrap();
    assert_eq!(de, id);
}

#[test]
fn test_gate_g_borrow_zero_clone_invariants() {
    fn inspect_link(link: &PaymentLink) -> bool {
        link.is_active() && link.status() == PaymentLinkStatus::Active
    }

    let link = PaymentLink::new("plink_borrow", "https://buy.stripe.com/borrow", true);
    // Verified by passing reference: zero deep cloning required
    assert!(inspect_link(&link));
    assert_eq!(link.id, "plink_borrow");
}
