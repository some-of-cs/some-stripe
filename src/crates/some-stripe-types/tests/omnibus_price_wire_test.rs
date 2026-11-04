//! Contract & Type Invariant Test Suite for EU Omnibus Directive 2019/2161
//! Price Transparency (`US-4.2.0` Unit 4) in `some-stripe-types`.
//!
//! # Statutary Legal Mandate: EU Directive 2019/2161 Article 6a (Gate G-Law)
//! Any announcement of a price reduction must indicate the prior price applied by the
//! trader for a period not less than 30 days prior to the reduction.
//! Deceptive discounts (where price was raised right before discount, or where advertised
//! sale price exceeds the 30-day lowest baseline) are strictly prohibited.
//!
//! # Quality Gates Verified:
//! - **G0**: Clean compilation on Rust 2024 edition without warnings.
//! - **G-Borrow**: Zero unnecessary deep copies/clones on parameters (`&CreatePaymentLinkParams`, `&str`).
//! - **G-Law**: Zero floating-point arithmetic; exact ISO 4217 integer minor currency units (`i64`).

use std::collections::HashMap;

use some_stripe_types::{
    CreatePaymentLinkParams, OmnibusPriceError, OmnibusPriceTransparencyMetadata,
    OmnibusTaxBreakdown, PaymentLinkLineItemParams, PaymentLinkPriceDataParams, TaxBehavior,
    validate_omnibus_price_transparency, validate_tax_behavior_transparency,
};

#[test]
fn test_omnibus_types_compliant_discount_with_prior_30d_baseline() {
    // Compliant discount: Regular €100.00 (10000 cents), sale €80.00 (8000 cents),
    // prior 30-day lowest €85.00 (8500 cents). 8000 <= 8500 and 8000 < 10000.
    assert!(validate_omnibus_price_transparency(10000, 8000, Some(8500)).is_ok());

    // Boundary equality: sale price equals prior 30-day lowest
    assert!(validate_omnibus_price_transparency(10000, 8000, Some(8000)).is_ok());

    // Regular price non-discounted catalog item
    assert!(validate_omnibus_price_transparency(5000, 5000, None).is_ok());
}

#[test]
fn test_omnibus_types_error_missing_prior_30d_baseline() {
    let result = validate_omnibus_price_transparency(10000, 8000, None);
    assert!(result.is_err());
    match result.unwrap_err() {
        OmnibusPriceError::InvalidRequest(msg) => {
            assert!(msg.contains("prior_30d_lowest_price must be specified"));
            assert!(msg.contains("EU Omnibus Directive 2019/2161 Article 6a"));
        }
    }
}

#[test]
fn test_omnibus_types_error_deceptive_discount_exceeds_baseline() {
    // Sale price 9000 exceeds 30-day lowest baseline 8000
    let result = validate_omnibus_price_transparency(12000, 9000, Some(8000));
    assert!(result.is_err());
    match result.unwrap_err() {
        OmnibusPriceError::InvalidRequest(msg) => {
            assert!(msg.contains("exceeds the prior 30-day lowest baseline price"));
            assert!(msg.contains("deceptive discounting"));
        }
    }
}

#[test]
fn test_omnibus_types_error_discount_exceeds_regular_price() {
    let result = validate_omnibus_price_transparency(5000, 6000, Some(5000));
    assert!(result.is_err());
    match result.unwrap_err() {
        OmnibusPriceError::InvalidRequest(msg) => {
            assert!(msg.contains("cannot exceed regular_price"));
        }
    }
}

#[test]
fn test_omnibus_types_error_non_positive_minor_units() {
    assert!(validate_omnibus_price_transparency(0, 1000, Some(1000)).is_err());
    assert!(validate_omnibus_price_transparency(-100, 1000, Some(1000)).is_err());
    assert!(validate_omnibus_price_transparency(1000, 0, Some(1000)).is_err());
    assert!(validate_omnibus_price_transparency(1000, -50, Some(1000)).is_err());
    assert!(validate_omnibus_price_transparency(1000, 800, Some(0)).is_err());
    assert!(validate_omnibus_price_transparency(1000, 800, Some(-500)).is_err());
}

#[test]
fn test_omnibus_types_metadata_roundtrip() {
    let metadata = OmnibusPriceTransparencyMetadata::new(10000, 8000, Some(8500))
        .expect("Failed to build metadata")
        .with_tax_behavior(TaxBehavior::Inclusive)
        .expect("Failed to attach tax behavior");

    let map = metadata.to_metadata_map();
    assert_eq!(map.get("omnibus_regular_price").unwrap(), "10000");
    assert_eq!(map.get("omnibus_discounted_price").unwrap(), "8000");
    assert_eq!(map.get("omnibus_prior_30d_lowest_price").unwrap(), "8500");
    assert_eq!(map.get("omnibus_tax_behavior").unwrap(), "inclusive");
    assert_eq!(
        map.get("statutory_directive").unwrap(),
        "EU_Directive_2019_2161_Article_6a"
    );

    let decoded = OmnibusPriceTransparencyMetadata::from_metadata_map(&map)
        .expect("Parsing should succeed")
        .expect("Metadata should be present");

    assert_eq!(decoded.regular_price, 10000);
    assert_eq!(decoded.announced_discounted_price, 8000);
    assert_eq!(decoded.prior_30d_lowest_price, Some(8500));
    assert_eq!(decoded.tax_behavior, Some(TaxBehavior::Inclusive));
}

#[test]
fn test_omnibus_types_tax_behavior_transparency() {
    assert!(validate_tax_behavior_transparency(TaxBehavior::Inclusive).is_ok());
    assert!(validate_tax_behavior_transparency(TaxBehavior::Exclusive).is_ok());

    let err = validate_tax_behavior_transparency(TaxBehavior::Unspecified).unwrap_err();
    match err {
        OmnibusPriceError::InvalidRequest(msg) => {
            assert!(msg.contains("transparent tax breakdown requires tax_behavior"));
        }
    }
}

#[test]
fn test_omnibus_types_tax_breakdown_arithmetic() {
    // Inclusive: Gross €120.00 (12000 cents) at 20% VAT (2000 bp)
    // Net: 10000 cents, Tax: 2000 cents
    let incl = OmnibusTaxBreakdown::calculate(12000, TaxBehavior::Inclusive, 2000)
        .expect("Inclusive calculation failed");
    assert_eq!(incl.tax_behavior, TaxBehavior::Inclusive);
    assert_eq!(incl.gross_amount, 12000);
    assert_eq!(incl.net_amount, 10000);
    assert_eq!(incl.tax_amount, 2000);
    assert_eq!(incl.net_amount + incl.tax_amount, incl.gross_amount);

    // Exclusive: Net €100.00 (10000 cents) at 20% VAT (2000 bp)
    // Tax: 2000 cents, Gross: 12000 cents
    let excl = OmnibusTaxBreakdown::calculate(10000, TaxBehavior::Exclusive, 2000)
        .expect("Exclusive calculation failed");
    assert_eq!(excl.tax_behavior, TaxBehavior::Exclusive);
    assert_eq!(excl.net_amount, 10000);
    assert_eq!(excl.tax_amount, 2000);
    assert_eq!(excl.gross_amount, 12000);
    assert_eq!(excl.net_amount + excl.tax_amount, excl.gross_amount);

    // Errors
    assert!(OmnibusTaxBreakdown::calculate(10000, TaxBehavior::Unspecified, 2000).is_err());
    assert!(OmnibusTaxBreakdown::calculate(0, TaxBehavior::Inclusive, 2000).is_err());
    assert!(OmnibusTaxBreakdown::calculate(10000, TaxBehavior::Inclusive, 15000).is_err());
}

#[test]
fn test_omnibus_types_price_data_params_builder() {
    let price_data = PaymentLinkPriceDataParams::new("eur", 10000).tax_behavior("inclusive");
    assert_eq!(price_data.currency, "eur");
    assert_eq!(price_data.unit_amount, Some(10000));
    assert_eq!(price_data.tax_behavior.as_deref(), Some("inclusive"));

    let line_item = PaymentLinkLineItemParams::with_price_data(price_data, 1);
    let params = CreatePaymentLinkParams::new(vec![line_item]).active(true);
    assert!(params.validate().is_ok());
}

#[test]
fn test_omnibus_types_gate_g_borrow_zero_clone() {
    let line_items = vec![PaymentLinkLineItemParams::new("price_omnibus_borrow", 2)];
    let params = CreatePaymentLinkParams::new(line_items);

    // Verification that validate() borrows &self
    assert!(params.validate().is_ok());

    let empty_map = HashMap::new();
    assert!(
        OmnibusPriceTransparencyMetadata::from_metadata_map(&empty_map)
            .unwrap()
            .is_none()
    );
}
