//! Integration and contract tests for Stripe Terminal domain models, EMV invariants, and parameters.

use some_stripe_types::terminal::{
    ConnectionToken, CreateConnectionTokenParams, CreateLocationParams, CreateReaderParams,
    ListTerminalLocationsParams, ListTerminalReadersParams, TerminalAddress, TerminalLocation,
    TerminalLocationList, TerminalReader, TerminalReaderList, TerminalValidationError,
    is_emv_compliant_device, validate_location_id, validate_registration_code,
};
use std::collections::HashMap;

#[test]
fn test_terminal_address_builder_and_serde() {
    let address = TerminalAddress::new()
        .with_line1("100 Market St")
        .with_city("San Francisco")
        .with_state("CA")
        .with_postal_code("94105")
        .with_country("US");

    assert_eq!(address.line1.as_deref(), Some("100 Market St"));
    assert_eq!(address.city.as_deref(), Some("San Francisco"));
    assert_eq!(address.state.as_deref(), Some("CA"));
    assert_eq!(address.postal_code.as_deref(), Some("94105"));
    assert_eq!(address.country.as_deref(), Some("US"));

    let json = serde_json::to_string(&address).expect("Serialize address");
    let deserialized: TerminalAddress = serde_json::from_str(&json).expect("Deserialize address");
    assert_eq!(address, deserialized);
}

#[test]
fn test_terminal_location_deserialization_and_borrowing() {
    let json = serde_json::json!({
        "id": "loc_12345",
        "object": "terminal.location",
        "display_name": "SOMA Flagship Store",
        "livemode": false,
        "address": {
            "line1": "100 Market St",
            "city": "San Francisco",
            "state": "CA",
            "postal_code": "94105",
            "country": "US"
        }
    });

    let location: TerminalLocation =
        serde_json::from_value(json).expect("Deserialize TerminalLocation");
    assert_eq!(location.id(), "loc_12345");
    assert_eq!(location.display_name(), "SOMA Flagship Store");
    assert!(!location.is_livemode());
    assert!(location.address().is_some());
    assert_eq!(
        location.address().unwrap().city.as_deref(),
        Some("San Francisco")
    );
}

#[test]
fn test_create_location_params_validation() {
    let valid = CreateLocationParams::new("Mission Store")
        .with_idempotency_key("idemp_loc_1")
        .with_address(TerminalAddress::new().with_city("San Francisco"));
    assert!(valid.validate().is_ok());

    let empty = CreateLocationParams::new("   ");
    assert_eq!(
        empty.validate().unwrap_err(),
        TerminalValidationError::EmptyDisplayName
    );
}

#[test]
fn test_terminal_reader_deserialization_and_emv_certification() {
    let json = serde_json::json!({
        "id": "tmr_wisepos_999",
        "object": "terminal.reader",
        "device_type": "bbpos_wisepos_e",
        "device_sw_version": "2.1.0",
        "ip_address": "192.168.1.105",
        "label": "Checkout Lane 1",
        "livemode": false,
        "location": "loc_12345",
        "serial_number": "WPOS-SN-999",
        "status": "online",
        "action": {
            "type": "process_payment_intent",
            "status": "in_progress"
        }
    });

    let reader: TerminalReader = serde_json::from_value(json).expect("Deserialize TerminalReader");
    assert_eq!(reader.id(), "tmr_wisepos_999");
    assert_eq!(reader.device_type(), "bbpos_wisepos_e");
    assert_eq!(reader.serial_number(), "WPOS-SN-999");
    assert_eq!(reader.status(), "online");
    assert!(reader.is_online());
    assert_eq!(reader.location(), Some("loc_12345"));
    assert_eq!(reader.label(), Some("Checkout Lane 1"));
    assert!(reader.is_emv_certified());

    let action = reader.action.as_ref().unwrap();
    assert_eq!(action.action_type, "process_payment_intent");
    assert_eq!(action.status, "in_progress");
}

#[test]
fn test_emv_device_support_matrix() {
    assert!(is_emv_compliant_device("bbpos_chipper2x"));
    assert!(is_emv_compliant_device("bbpos_wisepad3"));
    assert!(is_emv_compliant_device("bbpos_wisepos_e"));
    assert!(is_emv_compliant_device("verifone_p400"));
    assert!(is_emv_compliant_device("stripe_s700"));
    assert!(is_emv_compliant_device("stripe_m2"));
    assert!(is_emv_compliant_device("simulated_wisepos_e"));
    assert!(is_emv_compliant_device("simulated_s700"));

    assert!(!is_emv_compliant_device("unsupported_custom_terminal"));
    assert!(!is_emv_compliant_device("magstripe_reader_only"));
}

#[test]
fn test_registration_code_sanitization() {
    assert!(validate_registration_code("simulated-wpos-1").is_ok());
    assert!(validate_registration_code("PAIRING_CODE_123").is_ok());
    assert!(validate_registration_code("abc").is_ok());

    assert_eq!(
        validate_registration_code("").unwrap_err(),
        TerminalValidationError::EmptyRegistrationCode
    );
    assert_eq!(
        validate_registration_code("   ").unwrap_err(),
        TerminalValidationError::EmptyRegistrationCode
    );
    assert!(matches!(
        validate_registration_code("ab").unwrap_err(),
        TerminalValidationError::InvalidRegistrationCode(_)
    ));
    assert!(matches!(
        validate_registration_code("code with spaces").unwrap_err(),
        TerminalValidationError::InvalidRegistrationCode(_)
    ));
    assert!(matches!(
        validate_registration_code("code;DROP TABLE readers;").unwrap_err(),
        TerminalValidationError::InvalidRegistrationCode(_)
    ));
}

#[test]
fn test_location_id_validation() {
    assert!(validate_location_id(None).is_ok());
    assert!(validate_location_id(Some("loc_valid_123")).is_ok());

    assert!(matches!(
        validate_location_id(Some("invalid_loc")).unwrap_err(),
        TerminalValidationError::InvalidLocationId(_)
    ));
    assert!(matches!(
        validate_location_id(Some("loc_")).unwrap_err(),
        TerminalValidationError::InvalidLocationId(_)
    ));
}

#[test]
fn test_create_reader_params_validation_and_builder() {
    let mut meta = HashMap::new();
    meta.insert("floor".to_string(), "2".to_string());

    let params = CreateReaderParams::new("simulated-wpos-1")
        .with_label("Kiosk 4")
        .with_location("loc_valid_456")
        .with_metadata(meta)
        .with_idempotency_key("idemp_reader_1");

    assert!(params.validate().is_ok());

    let bad_loc = CreateReaderParams::new("simulated-wpos-1").with_location("bad_location");
    assert!(bad_loc.validate().is_err());
}

#[test]
fn test_connection_token_ac_us_7_2_0_03() {
    // AC-US-7.2.0-03: Generate Terminal connection token for smart reader with location binding
    let token = ConnectionToken {
        object: "terminal.connection_token".into(),
        secret: "pst_test_secret_abc123".into(),
        location: Some("loc_123".into()),
    };

    assert_eq!(token.secret(), "pst_test_secret_abc123");
    assert_eq!(token.location(), Some("loc_123"));
    assert!(token.is_bound_to_location("loc_123"));
    assert!(!token.is_bound_to_location("loc_other"));

    // Ensure secret is redacted in Debug output (Gate G2)
    let debug_output = format!("{token:?}");
    assert!(!debug_output.contains("pst_test_secret_abc123"));
    assert!(debug_output.contains("[REDACTED]"));
    assert!(debug_output.contains("loc_123"));
}

#[test]
fn test_create_connection_token_params() {
    let empty = CreateConnectionTokenParams::new();
    assert!(empty.validate().is_ok());
    assert!(empty.location.is_none());

    let scoped = CreateConnectionTokenParams::for_location("loc_store_1")
        .with_idempotency_key("idemp_tok_1");
    assert!(scoped.validate().is_ok());
    assert_eq!(scoped.location.as_deref(), Some("loc_store_1"));

    let invalid = CreateConnectionTokenParams::for_location("store_without_loc_prefix");
    assert!(invalid.validate().is_err());
}

#[test]
fn test_list_params_and_lists_serde() {
    let reader_list_json = serde_json::json!({
        "object": "list",
        "data": [{
            "id": "tmr_1",
            "object": "terminal.reader",
            "device_type": "stripe_s700",
            "serial_number": "S700-1",
            "status": "online"
        }],
        "has_more": false,
        "url": "/v1/terminal/readers"
    });

    let reader_list: TerminalReaderList =
        serde_json::from_value(reader_list_json).expect("Deserialize reader list");
    assert_eq!(reader_list.data.len(), 1);
    assert_eq!(reader_list.data[0].id(), "tmr_1");

    let location_list_json = serde_json::json!({
        "object": "list",
        "data": [{
            "id": "loc_1",
            "object": "terminal.location",
            "display_name": "Store 1",
            "livemode": false
        }],
        "has_more": false,
        "url": "/v1/terminal/locations"
    });

    let loc_list: TerminalLocationList =
        serde_json::from_value(location_list_json).expect("Deserialize location list");
    assert_eq!(loc_list.data.len(), 1);
    assert_eq!(loc_list.data[0].id(), "loc_1");

    let r_params = ListTerminalReadersParams::new()
        .with_limit(20)
        .with_status("online");
    assert_eq!(r_params.limit, Some(20));
    assert_eq!(r_params.status.as_deref(), Some("online"));

    let l_params = ListTerminalLocationsParams::new().with_limit(10);
    assert_eq!(l_params.limit, Some(10));
}
