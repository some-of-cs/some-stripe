//! Zero-Clone Dual-Protocol Serialization & Tri-State Nullable Audit Suite
//!
//! Validates Gate G-Borrow zero-copy borrowing semantics during RFC 1738 indexed bracket
//! form serialization and 3-state Nullable field updates.

use serde::Serialize;
use some_stripe_core::patterns::nullable::Nullable;
use some_stripe_core::patterns::serializer::to_form_urlencoded;

#[derive(Serialize)]
struct LineItem<'a> {
    price: &'a str,
    quantity: u32,
}

#[derive(Serialize)]
struct OrderUpdate<'a> {
    customer: &'a str,
    #[serde(skip_serializing_if = "Nullable::is_omit")]
    description: Nullable<&'a str>,
    items: &'a [LineItem<'a>],
}

#[test]
fn test_zero_clone_borrowed_slices_in_bracket_serialization() {
    let items = [
        LineItem {
            price: "price_111",
            quantity: 1,
        },
        LineItem {
            price: "price_222",
            quantity: 5,
        },
    ];

    let update = OrderUpdate {
        customer: "cus_zero_clone",
        description: Nullable::Null,
        items: &items,
    };

    let encoded = to_form_urlencoded(&update).expect("Serialization failed");

    // Must correctly serialize zero-indexed brackets and null string unsetting
    assert!(encoded.contains("customer=cus_zero_clone"));
    assert!(encoded.contains("description="));
    assert!(encoded.contains("items%5B0%5D%5Bprice%5D=price_111"));
    assert!(encoded.contains("items%5B0%5D%5Bquantity%5D=1"));
    assert!(encoded.contains("items%5B1%5D%5Bprice%5D=price_222"));
    assert!(encoded.contains("items%5B1%5D%5Bquantity%5D=5"));
}

#[test]
fn test_nullable_as_ref_zero_clone() {
    let val = Nullable::Value("hello_stripe".to_string());
    let borrowed: Nullable<&String> = val.as_ref();

    match borrowed {
        Nullable::Value(s) => assert_eq!(s, "hello_stripe"),
        _ => panic!("Expected Value variant"),
    }

    // Original value remains untouched and not moved
    assert!(val.is_value());
}
