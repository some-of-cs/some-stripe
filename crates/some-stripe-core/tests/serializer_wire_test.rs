//! Wire-level Contract Tests for US-1.2.0 (`TC-1.2.0-01`, `TC-1.2.0-02`, `TC-1.2.0-LAW`).

use serde::Serialize;
use some_stripe_core::patterns::serializer::{to_form_urlencoded, serialize_v2_json};
use some_stripe_core::Nullable;
use std::collections::BTreeMap;

#[derive(Serialize)]
struct PriceData {
    currency: &'static str,
    unit_amount: i64,
}

#[derive(Serialize)]
struct LineItem {
    price_data: PriceData,
    quantity: u32,
}

#[derive(Serialize)]
struct CreateCheckoutSessionFixture {
    mode: &'static str,
    line_items: Vec<LineItem>,
}

#[test]
fn test_tc_1_2_0_01_indexed_bracket_serialization() {
    let fixture = CreateCheckoutSessionFixture {
        mode: "payment",
        line_items: vec![
            LineItem {
                price_data: PriceData {
                    currency: "usd",
                    unit_amount: 5000,
                },
                quantity: 1,
            },
            LineItem {
                price_data: PriceData {
                    currency: "usd",
                    unit_amount: 2500,
                },
                quantity: 2,
            },
        ],
    };

    let wire = to_form_urlencoded(&fixture).expect("Serialization must succeed");

    assert!(wire.contains("line_items%5B0%5D%5Bprice_data%5D%5Bunit_amount%5D=5000"));
    assert!(wire.contains("line_items%5B0%5D%5Bquantity%5D=1"));
    assert!(wire.contains("line_items%5B1%5D%5Bprice_data%5D%5Bunit_amount%5D=2500"));
    assert!(wire.contains("line_items%5B1%5D%5Bquantity%5D=2"));
    assert!(!wire.contains("line_items%5B%5D"));
}

#[derive(Serialize)]
struct UpdateCustomerFixture {
    name: Nullable<&'static str>,
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    description: Nullable<&'static str>,
    #[serde(default, skip_serializing_if = "Nullable::is_omit")]
    metadata: Nullable<BTreeMap<&'static str, &'static str>>,
}

#[test]
fn test_tc_1_2_0_02_nullable_field_clearing() {
    let fixture = UpdateCustomerFixture {
        name: Nullable::Value("Jane Doe"),
        description: Nullable::Null,
        metadata: Nullable::Omit,
    };

    let wire = to_form_urlencoded(&fixture).expect("Serialization must succeed");
    assert!(wire.contains("description="));
    assert!(wire.contains("name=Jane%20Doe"));
    assert!(!wire.contains("metadata"));
}

#[test]
fn test_tc_1_2_0_law_minor_currency_precision() {
    #[derive(Serialize)]
    struct InvoiceItem {
        amount_cents: i64,
        satoshis: i64,
    }

    let fixture = InvoiceItem {
        amount_cents: 999_999_999_999,
        satoshis: 21_000_000_0000_0000,
    };

    let wire = to_form_urlencoded(&fixture).expect("Serialization must succeed");
    assert!(wire.contains("amount_cents=999999999999"));
    assert!(wire.contains("satoshis=2100000000000000"));

    let json = serialize_v2_json(&fixture).expect("v2 serialization must succeed");
    assert!(json.contains(r#""amount_cents":999999999999"#));
    assert!(json.contains(r#""satoshis":2100000000000000"#));
}
