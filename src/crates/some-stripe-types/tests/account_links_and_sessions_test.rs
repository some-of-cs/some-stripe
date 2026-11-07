//! Unit and contract tests for Stripe Connect Account Links and Account Sessions.
//!
//! Enforces:
//! - PCI-DSS 4.0 credential masking (`client_secret` redacted in `Debug`, Gate G2 / Gate G-Law).
//! - Zero-clone borrowing discipline (Gate G-Borrow).
//! - Open redirect and CRLF injection prevention on return/refresh URLs (Gate G2).
//! - Serde JSON wire compatibility and round-trip verification.

use some_stripe_types::account_link::{
    default_account_link_object, validate_account_link_url, AccountLink,
    AccountLinkCollectFields, AccountLinkCollectionOptions, AccountLinkFutureRequirements,
    AccountLinkParseError, AccountLinkType, AccountLinkValidationError,
    CreateAccountLinkParams,
};
use some_stripe_types::account_session::{
    default_account_session_object, AccountManagementComponent, AccountOnboardingComponent,
    AccountSession, AccountSessionComponents, AccountSessionValidationError, BalancesComponent,
    BalancesFeatures, CreateAccountSessionParams, DocumentsComponent,
    NotificationBannerComponent, PaymentDetailsComponent, PaymentDetailsFeatures,
    PaymentsComponent, PaymentsFeatures, PayoutsComponent, PayoutsFeatures,
    PayoutsListComponent, TaxRegistrationsComponent, TaxSettingsComponent,
};
use std::str::FromStr;

// ============================================================================
// ACCOUNT LINK TESTS
// ============================================================================

#[test]
fn test_default_account_link_object() {
    assert_eq!(default_account_link_object(), "account_link");
}

#[test]
fn test_account_link_type_display_and_parse() {
    let onboarding = AccountLinkType::AccountOnboarding;
    assert_eq!(onboarding.as_str(), "account_onboarding");
    assert_eq!(onboarding.to_string(), "account_onboarding");
    assert_eq!(
        AccountLinkType::from_str("account_onboarding").unwrap(),
        AccountLinkType::AccountOnboarding
    );

    let update = AccountLinkType::AccountUpdate;
    assert_eq!(update.as_str(), "account_update");
    assert_eq!(update.to_string(), "account_update");
    assert_eq!(
        AccountLinkType::from_str("account_update").unwrap(),
        AccountLinkType::AccountUpdate
    );

    let err = AccountLinkType::from_str("invalid_type").unwrap_err();
    assert_eq!(
        err,
        AccountLinkParseError::InvalidType("invalid_type".into())
    );
    assert!(err.to_string().contains("invalid_type"));
}

#[test]
fn test_account_link_collect_fields_and_future_requirements() {
    let curr = AccountLinkCollectFields::CurrentlyDue;
    assert_eq!(curr.as_str(), "currently_due");
    assert_eq!(curr.to_string(), "currently_due");
    assert_eq!(
        AccountLinkCollectFields::from_str("currently_due").unwrap(),
        AccountLinkCollectFields::CurrentlyDue
    );

    let event = AccountLinkCollectFields::EventuallyDue;
    assert_eq!(event.as_str(), "eventually_due");
    assert_eq!(event.to_string(), "eventually_due");
    assert_eq!(
        AccountLinkCollectFields::from_str("eventually_due").unwrap(),
        AccountLinkCollectFields::EventuallyDue
    );

    let err_field = AccountLinkCollectFields::from_str("bad").unwrap_err();
    assert_eq!(
        err_field,
        AccountLinkParseError::InvalidCollectField("bad".into())
    );

    let inc = AccountLinkFutureRequirements::Include;
    assert_eq!(inc.as_str(), "include");
    assert_eq!(inc.to_string(), "include");
    assert_eq!(
        AccountLinkFutureRequirements::from_str("include").unwrap(),
        AccountLinkFutureRequirements::Include
    );

    let omit = AccountLinkFutureRequirements::Omit;
    assert_eq!(omit.as_str(), "omit");
    assert_eq!(omit.to_string(), "omit");
    assert_eq!(
        AccountLinkFutureRequirements::from_str("omit").unwrap(),
        AccountLinkFutureRequirements::Omit
    );

    let err_req = AccountLinkFutureRequirements::from_str("bad").unwrap_err();
    assert_eq!(
        err_req,
        AccountLinkParseError::InvalidFutureRequirement("bad".into())
    );
}

#[test]
fn test_account_link_collection_options_builder_and_serde() {
    let opts = AccountLinkCollectionOptions::new()
        .with_fields(AccountLinkCollectFields::CurrentlyDue)
        .with_future_requirements(AccountLinkFutureRequirements::Include);

    assert_eq!(opts.fields, Some(AccountLinkCollectFields::CurrentlyDue));
    assert_eq!(
        opts.future_requirements,
        Some(AccountLinkFutureRequirements::Include)
    );

    let json = serde_json::to_string(&opts).unwrap();
    assert!(json.contains("\"fields\":\"currently_due\""));
    assert!(json.contains("\"future_requirements\":\"include\""));

    let deserialized: AccountLinkCollectionOptions = serde_json::from_str(&json).unwrap();
    assert_eq!(opts, deserialized);
}

#[test]
fn test_account_link_deserialization_and_accessors() {
    let raw = r#"{
        "object": "account_link",
        "created": 1700000000,
        "expires_at": 1700000300,
        "url": "https://connect.stripe.com/setup/s/test_session_token_123"
    }"#;

    let link: AccountLink = serde_json::from_str(raw).expect("deserialization should succeed");
    assert_eq!(link.object, "account_link");
    assert_eq!(link.created(), 1700000000);
    assert_eq!(link.expires_at(), 1700000300);
    assert_eq!(
        link.url(),
        "https://connect.stripe.com/setup/s/test_session_token_123"
    );

    // Expiration boundary check
    assert!(!link.is_expired(1699999999));
    assert!(!link.is_expired(1700000000));
    assert!(!link.is_expired(1700000299));
    assert!(link.is_expired(1700000300));
    assert!(link.is_expired(1700000301));
}

#[test]
fn test_account_link_serde_round_trip() {
    let original = AccountLink {
        object: "account_link".into(),
        created: 1712345678,
        expires_at: 1712345978,
        url: "https://connect.stripe.com/setup/s/abc_xyz".into(),
    };

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: AccountLink = serde_json::from_str(&serialized).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn test_validate_account_link_url_security_invariants() {
    // Valid HTTPS URLs
    assert!(validate_account_link_url("https://example.com/return").is_ok());
    assert!(validate_account_link_url("https://dashboard.example.com/onboarding/complete?session=123").is_ok());

    // Valid local development URLs
    assert!(validate_account_link_url("http://localhost:3000/callback").is_ok());
    assert!(validate_account_link_url("http://127.0.0.1:8080/stripe/reauth").is_ok());

    // Insecure HTTP (non-localhost)
    assert!(validate_account_link_url("http://insecure-domain.com/path").is_err());
    assert_eq!(
        validate_account_link_url("http://insecure-domain.com/path").unwrap_err(),
        "Account link URL must strictly use HTTPS protocol (or localhost for development)"
    );

    // Empty URL
    assert_eq!(
        validate_account_link_url("").unwrap_err(),
        "URL cannot be empty"
    );

    // Malicious URI schemes
    assert!(validate_account_link_url("javascript:alert(document.cookie)").is_err());
    assert!(validate_account_link_url("data:text/html,<script>alert(1)</script>").is_err());
    assert!(validate_account_link_url("vbscript:MsgBox(1)").is_err());

    // CRLF Injection prevention
    assert!(validate_account_link_url("https://example.com/path\r\nSet-Cookie: malicious").is_err());
    assert!(validate_account_link_url("https://example.com/path\nInjected").is_err());
}

#[test]
fn test_create_account_link_params_builder_and_validation() {
    let mut params = CreateAccountLinkParams::new(
        "acct_1032D82eZvKYlo2C",
        "https://example.com/reauth",
        "https://example.com/return",
        AccountLinkType::AccountOnboarding,
    );

    assert!(params.validate().is_ok());

    // Fluent builder attachments
    params = params
        .with_collection_options(
            AccountLinkCollectionOptions::new()
                .with_fields(AccountLinkCollectFields::CurrentlyDue)
                .with_future_requirements(AccountLinkFutureRequirements::Include),
        )
        .with_expand("account")
        .with_idempotency_key("idem_test_key_123");

    assert_eq!(params.expand, Some(vec!["account".into()]));
    assert_eq!(params.idempotency_key, Some("idem_test_key_123".into()));

    // Verify JSON serialization includes 'type' and omits idempotency_key
    let json = serde_json::to_string(&params).unwrap();
    assert!(json.contains("\"type\":\"account_onboarding\""));
    assert!(json.contains("\"account\":\"acct_1032D82eZvKYlo2C\""));
    assert!(json.contains("\"refresh_url\":\"https://example.com/reauth\""));
    assert!(json.contains("\"return_url\":\"https://example.com/return\""));
    assert!(!json.contains("idempotency_key"));

    // Deserialization round trip
    let deserialized: CreateAccountLinkParams = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.account, params.account);
    assert_eq!(deserialized.type_, params.type_);
    assert_eq!(deserialized.refresh_url, params.refresh_url);
    assert_eq!(deserialized.return_url, params.return_url);
}

#[test]
fn test_create_account_link_params_for_account() {
    let params = CreateAccountLinkParams::for_account(
        "acct_update12345",
        AccountLinkType::AccountUpdate,
    )
    .with_refresh_url("https://example.com/refresh")
    .with_return_url("https://example.com/success");

    assert_eq!(params.type_, AccountLinkType::AccountUpdate);
    assert!(params.validate().is_ok());
}

#[test]
fn test_create_account_link_params_validation_errors() {
    // Empty account
    let empty_acct = CreateAccountLinkParams::new(
        "",
        "https://example.com/refresh",
        "https://example.com/return",
        AccountLinkType::AccountOnboarding,
    );
    assert_eq!(
        empty_acct.validate().unwrap_err(),
        AccountLinkValidationError::EmptyAccount
    );

    // Whitespace only account
    let ws_acct = CreateAccountLinkParams::new(
        "   ",
        "https://example.com/refresh",
        "https://example.com/return",
        AccountLinkType::AccountOnboarding,
    );
    assert_eq!(
        ws_acct.validate().unwrap_err(),
        AccountLinkValidationError::EmptyAccount
    );

    // Invalid account prefix
    let invalid_prefix = CreateAccountLinkParams::new(
        "cus_12345",
        "https://example.com/refresh",
        "https://example.com/return",
        AccountLinkType::AccountOnboarding,
    );
    assert_eq!(
        invalid_prefix.validate().unwrap_err(),
        AccountLinkValidationError::InvalidAccount("cus_12345".into())
    );

    // Insecure refresh URL
    let bad_refresh = CreateAccountLinkParams::new(
        "acct_12345",
        "http://insecure.com/refresh",
        "https://example.com/return",
        AccountLinkType::AccountOnboarding,
    );
    assert!(matches!(
        bad_refresh.validate().unwrap_err(),
        AccountLinkValidationError::InvalidUrl {
            field: "refresh_url",
            ..
        }
    ));

    // Insecure return URL
    let bad_return = CreateAccountLinkParams::new(
        "acct_12345",
        "https://example.com/refresh",
        "javascript:alert(1)",
        AccountLinkType::AccountOnboarding,
    );
    assert!(matches!(
        bad_return.validate().unwrap_err(),
        AccountLinkValidationError::InvalidUrl {
            field: "return_url",
            ..
        }
    ));
}

// ============================================================================
// ACCOUNT SESSION & EMBEDDED COMPONENTS TESTS
// ============================================================================

#[test]
fn test_default_account_session_object() {
    assert_eq!(default_account_session_object(), "account_session");
}

#[test]
fn test_account_session_embedded_components_configuration() {
    let components = AccountSessionComponents::new()
        .with_account_onboarding(AccountOnboardingComponent::enabled())
        .with_account_management(AccountManagementComponent::new(true))
        .with_balances(BalancesComponent::enabled().with_features(BalancesFeatures {
            instant_payouts: Some(true),
            standard_payouts: Some(true),
            edit_payout_schedule: Some(false),
        }))
        .with_documents(DocumentsComponent::enabled())
        .with_notification_banner(NotificationBannerComponent::enabled())
        .with_payment_details(PaymentDetailsComponent::enabled().with_features(
            PaymentDetailsFeatures {
                refund_payments: Some(true),
                dispute_management: Some(true),
                capture_payments: Some(false),
            },
        ))
        .with_payments(PaymentsComponent::enabled().with_features(PaymentsFeatures {
            refund_payments: Some(true),
            dispute_management: Some(false),
            capture_payments: Some(true),
        }))
        .with_payouts(PayoutsComponent::enabled().with_features(PayoutsFeatures {
            instant_payouts: Some(true),
            standard_payouts: Some(true),
            edit_payout_schedule: Some(true),
            external_account_collection: Some(false),
        }))
        .with_payouts_list(PayoutsListComponent::enabled())
        .with_tax_registrations(TaxRegistrationsComponent::enabled())
        .with_tax_settings(TaxSettingsComponent::enabled());

    assert!(components.has_any_enabled());
    assert!(components.is_component_enabled("account_onboarding"));
    assert!(components.is_component_enabled("account_management"));
    assert!(components.is_component_enabled("balances"));
    assert!(components.is_component_enabled("documents"));
    assert!(components.is_component_enabled("notification_banner"));
    assert!(components.is_component_enabled("payment_details"));
    assert!(components.is_component_enabled("payments"));
    assert!(components.is_component_enabled("payouts"));
    assert!(components.is_component_enabled("payouts_list"));
    assert!(components.is_component_enabled("tax_registrations"));
    assert!(components.is_component_enabled("tax_settings"));
    assert!(!components.is_component_enabled("nonexistent_component"));

    // Serialization & round-trip
    let json = serde_json::to_string(&components).unwrap();
    assert!(json.contains("\"account_onboarding\":{\"enabled\":true"));
    assert!(json.contains("\"instant_payouts\":true"));
    let deserialized: AccountSessionComponents = serde_json::from_str(&json).unwrap();
    assert_eq!(components, deserialized);
}

#[test]
fn test_account_session_empty_components() {
    let empty = AccountSessionComponents::new();
    assert!(!empty.has_any_enabled());
    assert!(!empty.is_component_enabled("payments"));

    let all_disabled = AccountSessionComponents::new()
        .with_account_onboarding(AccountOnboardingComponent::new(false))
        .with_payments(PaymentsComponent::new(false));
    assert!(!all_disabled.has_any_enabled());
}

#[test]
fn test_account_session_deserialization_and_debug_redaction() {
    let raw = r#"{
        "object": "account_session",
        "account": "acct_1032D82eZvKYlo2C",
        "client_secret": "secret_live_super_sensitive_token_999888",
        "created": 1700000000,
        "expires_at": 1700003600,
        "livemode": true,
        "components": {
            "payments": {
                "enabled": true,
                "features": {
                    "refund_payments": true
                }
            }
        }
    }"#;

    let session: AccountSession =
        serde_json::from_str(raw).expect("session deserialization should succeed");

    assert_eq!(session.object, "account_session");
    assert_eq!(session.account(), "acct_1032D82eZvKYlo2C");
    assert_eq!(
        session.client_secret(),
        "secret_live_super_sensitive_token_999888"
    );
    assert_eq!(session.created(), 1700000000);
    assert_eq!(session.expires_at(), 1700003600);
    assert!(session.is_livemode());
    assert!(session.components().is_component_enabled("payments"));

    // Expiry checks
    assert!(!session.is_expired(1700000000));
    assert!(!session.is_expired(1700003599));
    assert!(session.is_expired(1700003600));
    assert!(session.is_expired(1700003601));

    // Gate G2 & PCI-DSS 4.0: client_secret MUST be redacted in Debug representation!
    let debug_str = format!("{session:?}");
    assert!(
        !debug_str.contains("secret_live_super_sensitive_token_999888"),
        "PCI-DSS Violation: client_secret must never be printed in Debug output!"
    );
    assert!(
        debug_str.contains("[REDACTED]"),
        "PCI-DSS Requirement: client_secret must be masked as [REDACTED]!"
    );
}

#[test]
fn test_account_session_serde_round_trip() {
    let session = AccountSession {
        object: "account_session".into(),
        account: "acct_test_7788".into(),
        client_secret: "secret_sample_key".into(),
        created: 1711111111,
        expires_at: 1711114711,
        livemode: false,
        components: AccountSessionComponents::new()
            .with_payments(PaymentsComponent::enabled()),
    };

    let serialized = serde_json::to_string(&session).unwrap();
    let deserialized: AccountSession = serde_json::from_str(&serialized).unwrap();
    assert_eq!(session, deserialized);
}

#[test]
fn test_create_account_session_params_builder_and_validation() {
    let components = AccountSessionComponents::new()
        .with_account_onboarding(AccountOnboardingComponent::enabled());

    let params = CreateAccountSessionParams::new("acct_1032D82eZvKYlo2C", components)
        .with_payments(PaymentsComponent::enabled())
        .with_payouts(PayoutsComponent::enabled())
        .with_balances(BalancesComponent::enabled())
        .with_documents(DocumentsComponent::enabled())
        .with_account_management(AccountManagementComponent::enabled())
        .with_expand("components")
        .with_idempotency_key("req_idem_key_456");

    assert!(params.validate().is_ok());
    assert_eq!(params.expand, Some(vec!["components".into()]));
    assert_eq!(params.idempotency_key, Some("req_idem_key_456".into()));

    // Verify JSON serialization includes components and omits idempotency_key
    let json = serde_json::to_string(&params).unwrap();
    assert!(json.contains("\"account\":\"acct_1032D82eZvKYlo2C\""));
    assert!(json.contains("\"account_onboarding\":{\"enabled\":true"));
    assert!(json.contains("\"payments\":{\"enabled\":true"));
    assert!(!json.contains("idempotency_key"));

    let deserialized: CreateAccountSessionParams = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.account, params.account);
    assert_eq!(deserialized.components, params.components);
}

#[test]
fn test_create_account_session_params_for_account() {
    let mut params = CreateAccountSessionParams::for_account("acct_sample_account_99");
    assert_eq!(
        params.validate().unwrap_err(),
        AccountSessionValidationError::NoComponentsEnabled
    );

    params = params.with_payments(PaymentsComponent::enabled());
    assert!(params.validate().is_ok());
}

#[test]
fn test_create_account_session_params_validation_errors() {
    // Empty account
    let empty = CreateAccountSessionParams::new(
        "",
        AccountSessionComponents::new().with_payments(PaymentsComponent::enabled()),
    );
    assert_eq!(
        empty.validate().unwrap_err(),
        AccountSessionValidationError::EmptyAccount
    );

    // Whitespace account
    let ws = CreateAccountSessionParams::new(
        "   ",
        AccountSessionComponents::new().with_payments(PaymentsComponent::enabled()),
    );
    assert_eq!(
        ws.validate().unwrap_err(),
        AccountSessionValidationError::EmptyAccount
    );

    // Invalid account prefix
    let invalid_prefix = CreateAccountSessionParams::new(
        "cus_customer_id_123",
        AccountSessionComponents::new().with_payments(PaymentsComponent::enabled()),
    );
    assert_eq!(
        invalid_prefix.validate().unwrap_err(),
        AccountSessionValidationError::InvalidAccount("cus_customer_id_123".into())
    );

    // No components enabled
    let no_comp = CreateAccountSessionParams::new(
        "acct_valid_account",
        AccountSessionComponents::new(),
    );
    assert_eq!(
        no_comp.validate().unwrap_err(),
        AccountSessionValidationError::NoComponentsEnabled
    );
}
