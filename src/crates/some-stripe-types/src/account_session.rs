//! # Stripe Connect Account Session Types & Embedded Components
//!
//! Strongly typed domain models, component configurations, and parameters for Stripe Account Sessions.
//!
//! An Account Session creates a short-lived session token allowing connected accounts
//! to securely access Stripe embedded components (such as embedded onboarding, payments,
//! payouts, and account management) within a platform's application.
//!
//! Under Gate G2 (Security Gateway & PCI-DSS 4.0) and Gate G-Borrow (Zero-Clone Borrowing Discipline):
//! - `client_secret` is automatically masked as `[REDACTED]` in custom `Debug` implementations.
//! - Read operations provide zero-copy reference borrowing (`&str`, `&AccountSessionComponents`).
//! - All public structures include working rustdoc doctests and full serialization round-trip support.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Default object identifier for an Account Session (`"account_session"`).
#[must_use]
pub fn default_account_session_object() -> String {
    "account_session".into()
}

/// Features configuration for the Account Onboarding embedded component.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountOnboardingFeatures {}

/// Configuration for the Account Onboarding embedded component.
///
/// # Example
/// ```
/// use some_stripe_types::account_session::AccountOnboardingComponent;
///
/// let comp = AccountOnboardingComponent::enabled();
/// assert!(comp.enabled);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountOnboardingComponent {
    /// Whether the embedded component is enabled.
    pub enabled: bool,

    /// Optional features configuration for onboarding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<AccountOnboardingFeatures>,
}

impl AccountOnboardingComponent {
    /// Constructs an enabled Account Onboarding component.
    #[must_use]
    pub const fn enabled() -> Self {
        Self {
            enabled: true,
            features: None,
        }
    }

    /// Constructs an Account Onboarding component with specified enabled status.
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            features: None,
        }
    }
}

/// Features configuration for the Account Management embedded component.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountManagementFeatures {}

/// Configuration for the Account Management embedded component.
///
/// # Example
/// ```
/// use some_stripe_types::account_session::AccountManagementComponent;
///
/// let comp = AccountManagementComponent::enabled();
/// assert!(comp.enabled);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountManagementComponent {
    /// Whether the embedded component is enabled.
    pub enabled: bool,

    /// Optional features configuration for account management.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<AccountManagementFeatures>,
}

impl AccountManagementComponent {
    /// Constructs an enabled Account Management component.
    #[must_use]
    pub const fn enabled() -> Self {
        Self {
            enabled: true,
            features: None,
        }
    }

    /// Constructs an Account Management component with specified enabled status.
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            features: None,
        }
    }
}

/// Features configuration for the Balances embedded component.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalancesFeatures {
    /// Whether instant payouts are enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instant_payouts: Option<bool>,

    /// Whether standard payouts are enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard_payouts: Option<bool>,

    /// Whether editing payout schedules is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit_payout_schedule: Option<bool>,
}

/// Configuration for the Balances embedded component.
///
/// # Example
/// ```
/// use some_stripe_types::account_session::BalancesComponent;
///
/// let comp = BalancesComponent::enabled();
/// assert!(comp.enabled);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalancesComponent {
    /// Whether the embedded component is enabled.
    pub enabled: bool,

    /// Optional features configuration for balances.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<BalancesFeatures>,
}

impl BalancesComponent {
    /// Constructs an enabled Balances component.
    #[must_use]
    pub const fn enabled() -> Self {
        Self {
            enabled: true,
            features: None,
        }
    }

    /// Constructs a Balances component with specified enabled status.
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            features: None,
        }
    }

    /// Attaches features configuration.
    #[must_use]
    pub fn with_features(mut self, features: BalancesFeatures) -> Self {
        self.features = Some(features);
        self
    }
}

/// Features configuration for the Documents embedded component.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentsFeatures {}

/// Configuration for the Documents embedded component.
///
/// # Example
/// ```
/// use some_stripe_types::account_session::DocumentsComponent;
///
/// let comp = DocumentsComponent::enabled();
/// assert!(comp.enabled);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentsComponent {
    /// Whether the embedded component is enabled.
    pub enabled: bool,

    /// Optional features configuration for documents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<DocumentsFeatures>,
}

impl DocumentsComponent {
    /// Constructs an enabled Documents component.
    #[must_use]
    pub const fn enabled() -> Self {
        Self {
            enabled: true,
            features: None,
        }
    }

    /// Constructs a Documents component with specified enabled status.
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            features: None,
        }
    }
}

/// Features configuration for the Notification Banner embedded component.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationBannerFeatures {}

/// Configuration for the Notification Banner embedded component.
///
/// # Example
/// ```
/// use some_stripe_types::account_session::NotificationBannerComponent;
///
/// let comp = NotificationBannerComponent::enabled();
/// assert!(comp.enabled);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationBannerComponent {
    /// Whether the embedded component is enabled.
    pub enabled: bool,

    /// Optional features configuration for notification banners.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<NotificationBannerFeatures>,
}

impl NotificationBannerComponent {
    /// Constructs an enabled Notification Banner component.
    #[must_use]
    pub const fn enabled() -> Self {
        Self {
            enabled: true,
            features: None,
        }
    }

    /// Constructs a Notification Banner component with specified enabled status.
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            features: None,
        }
    }
}

/// Features configuration for the Payment Details embedded component.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentDetailsFeatures {
    /// Whether refunding payments is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refund_payments: Option<bool>,

    /// Whether dispute management is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispute_management: Option<bool>,

    /// Whether capturing payments is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_payments: Option<bool>,
}

/// Configuration for the Payment Details embedded component.
///
/// # Example
/// ```
/// use some_stripe_types::account_session::PaymentDetailsComponent;
///
/// let comp = PaymentDetailsComponent::enabled();
/// assert!(comp.enabled);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentDetailsComponent {
    /// Whether the embedded component is enabled.
    pub enabled: bool,

    /// Optional features configuration for payment details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<PaymentDetailsFeatures>,
}

impl PaymentDetailsComponent {
    /// Constructs an enabled Payment Details component.
    #[must_use]
    pub const fn enabled() -> Self {
        Self {
            enabled: true,
            features: None,
        }
    }

    /// Constructs a Payment Details component with specified enabled status.
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            features: None,
        }
    }

    /// Attaches features configuration.
    #[must_use]
    pub fn with_features(mut self, features: PaymentDetailsFeatures) -> Self {
        self.features = Some(features);
        self
    }
}

/// Features configuration for the Payments embedded component.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentsFeatures {
    /// Whether refunding payments is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refund_payments: Option<bool>,

    /// Whether dispute management is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispute_management: Option<bool>,

    /// Whether capturing payments is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_payments: Option<bool>,
}

/// Configuration for the Payments embedded component.
///
/// # Example
/// ```
/// use some_stripe_types::account_session::PaymentsComponent;
///
/// let comp = PaymentsComponent::enabled();
/// assert!(comp.enabled);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentsComponent {
    /// Whether the embedded component is enabled.
    pub enabled: bool,

    /// Optional features configuration for payments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<PaymentsFeatures>,
}

impl PaymentsComponent {
    /// Constructs an enabled Payments component.
    #[must_use]
    pub const fn enabled() -> Self {
        Self {
            enabled: true,
            features: None,
        }
    }

    /// Constructs a Payments component with specified enabled status.
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            features: None,
        }
    }

    /// Attaches features configuration.
    #[must_use]
    pub fn with_features(mut self, features: PaymentsFeatures) -> Self {
        self.features = Some(features);
        self
    }
}

/// Features configuration for the Payouts embedded component.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayoutsFeatures {
    /// Whether instant payouts are enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instant_payouts: Option<bool>,

    /// Whether standard payouts are enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard_payouts: Option<bool>,

    /// Whether editing payout schedules is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit_payout_schedule: Option<bool>,

    /// Whether external account collection is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_account_collection: Option<bool>,
}

/// Configuration for the Payouts embedded component.
///
/// # Example
/// ```
/// use some_stripe_types::account_session::PayoutsComponent;
///
/// let comp = PayoutsComponent::enabled();
/// assert!(comp.enabled);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayoutsComponent {
    /// Whether the embedded component is enabled.
    pub enabled: bool,

    /// Optional features configuration for payouts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<PayoutsFeatures>,
}

impl PayoutsComponent {
    /// Constructs an enabled Payouts component.
    #[must_use]
    pub const fn enabled() -> Self {
        Self {
            enabled: true,
            features: None,
        }
    }

    /// Constructs a Payouts component with specified enabled status.
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            features: None,
        }
    }

    /// Attaches features configuration.
    #[must_use]
    pub fn with_features(mut self, features: PayoutsFeatures) -> Self {
        self.features = Some(features);
        self
    }
}

/// Features configuration for the Payouts List embedded component.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayoutsListFeatures {}

/// Configuration for the Payouts List embedded component.
///
/// # Example
/// ```
/// use some_stripe_types::account_session::PayoutsListComponent;
///
/// let comp = PayoutsListComponent::enabled();
/// assert!(comp.enabled);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayoutsListComponent {
    /// Whether the embedded component is enabled.
    pub enabled: bool,

    /// Optional features configuration for payouts list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<PayoutsListFeatures>,
}

impl PayoutsListComponent {
    /// Constructs an enabled Payouts List component.
    #[must_use]
    pub const fn enabled() -> Self {
        Self {
            enabled: true,
            features: None,
        }
    }

    /// Constructs a Payouts List component with specified enabled status.
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            features: None,
        }
    }
}

/// Features configuration for the Tax Registrations embedded component.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxRegistrationsFeatures {}

/// Configuration for the Tax Registrations embedded component.
///
/// # Example
/// ```
/// use some_stripe_types::account_session::TaxRegistrationsComponent;
///
/// let comp = TaxRegistrationsComponent::enabled();
/// assert!(comp.enabled);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxRegistrationsComponent {
    /// Whether the embedded component is enabled.
    pub enabled: bool,

    /// Optional features configuration for tax registrations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<TaxRegistrationsFeatures>,
}

impl TaxRegistrationsComponent {
    /// Constructs an enabled Tax Registrations component.
    #[must_use]
    pub const fn enabled() -> Self {
        Self {
            enabled: true,
            features: None,
        }
    }

    /// Constructs a Tax Registrations component with specified enabled status.
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            features: None,
        }
    }
}

/// Features configuration for the Tax Settings embedded component.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxSettingsFeatures {}

/// Configuration for the Tax Settings embedded component.
///
/// # Example
/// ```
/// use some_stripe_types::account_session::TaxSettingsComponent;
///
/// let comp = TaxSettingsComponent::enabled();
/// assert!(comp.enabled);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxSettingsComponent {
    /// Whether the embedded component is enabled.
    pub enabled: bool,

    /// Optional features configuration for tax settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<TaxSettingsFeatures>,
}

impl TaxSettingsComponent {
    /// Constructs an enabled Tax Settings component.
    #[must_use]
    pub const fn enabled() -> Self {
        Self {
            enabled: true,
            features: None,
        }
    }

    /// Constructs a Tax Settings component with specified enabled status.
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            features: None,
        }
    }
}

/// Component access configurations enabled for an [`AccountSession`].
///
/// # Example
/// ```
/// use some_stripe_types::account_session::{AccountSessionComponents, AccountOnboardingComponent, PaymentsComponent};
///
/// let components = AccountSessionComponents::new()
///     .with_account_onboarding(AccountOnboardingComponent::enabled())
///     .with_payments(PaymentsComponent::enabled());
///
/// assert!(components.has_any_enabled());
/// assert!(components.is_component_enabled("payments"));
/// assert!(!components.is_component_enabled("payouts"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountSessionComponents {
    /// Configuration for the account onboarding embedded component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_onboarding: Option<AccountOnboardingComponent>,

    /// Configuration for the account management embedded component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_management: Option<AccountManagementComponent>,

    /// Configuration for the balances embedded component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balances: Option<BalancesComponent>,

    /// Configuration for the documents embedded component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documents: Option<DocumentsComponent>,

    /// Configuration for the notification banner embedded component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notification_banner: Option<NotificationBannerComponent>,

    /// Configuration for the payment details embedded component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_details: Option<PaymentDetailsComponent>,

    /// Configuration for the payments embedded component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payments: Option<PaymentsComponent>,

    /// Configuration for the payouts embedded component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payouts: Option<PayoutsComponent>,

    /// Configuration for the payouts list embedded component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payouts_list: Option<PayoutsListComponent>,

    /// Configuration for the tax registrations embedded component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_registrations: Option<TaxRegistrationsComponent>,

    /// Configuration for the tax settings embedded component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_settings: Option<TaxSettingsComponent>,
}

impl AccountSessionComponents {
    /// Constructs empty components options.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            account_onboarding: None,
            account_management: None,
            balances: None,
            documents: None,
            notification_banner: None,
            payment_details: None,
            payments: None,
            payouts: None,
            payouts_list: None,
            tax_registrations: None,
            tax_settings: None,
        }
    }

    /// Attaches account onboarding component configuration.
    #[must_use]
    pub fn with_account_onboarding(mut self, c: AccountOnboardingComponent) -> Self {
        self.account_onboarding = Some(c);
        self
    }

    /// Attaches account management component configuration.
    #[must_use]
    pub fn with_account_management(mut self, c: AccountManagementComponent) -> Self {
        self.account_management = Some(c);
        self
    }

    /// Attaches balances component configuration.
    #[must_use]
    pub fn with_balances(mut self, c: BalancesComponent) -> Self {
        self.balances = Some(c);
        self
    }

    /// Attaches documents component configuration.
    #[must_use]
    pub fn with_documents(mut self, c: DocumentsComponent) -> Self {
        self.documents = Some(c);
        self
    }

    /// Attaches notification banner component configuration.
    #[must_use]
    pub fn with_notification_banner(mut self, c: NotificationBannerComponent) -> Self {
        self.notification_banner = Some(c);
        self
    }

    /// Attaches payment details component configuration.
    #[must_use]
    pub fn with_payment_details(mut self, c: PaymentDetailsComponent) -> Self {
        self.payment_details = Some(c);
        self
    }

    /// Attaches payments component configuration.
    #[must_use]
    pub fn with_payments(mut self, c: PaymentsComponent) -> Self {
        self.payments = Some(c);
        self
    }

    /// Attaches payouts component configuration.
    #[must_use]
    pub fn with_payouts(mut self, c: PayoutsComponent) -> Self {
        self.payouts = Some(c);
        self
    }

    /// Attaches payouts list component configuration.
    #[must_use]
    pub fn with_payouts_list(mut self, c: PayoutsListComponent) -> Self {
        self.payouts_list = Some(c);
        self
    }

    /// Attaches tax registrations component configuration.
    #[must_use]
    pub fn with_tax_registrations(mut self, c: TaxRegistrationsComponent) -> Self {
        self.tax_registrations = Some(c);
        self
    }

    /// Attaches tax settings component configuration.
    #[must_use]
    pub fn with_tax_settings(mut self, c: TaxSettingsComponent) -> Self {
        self.tax_settings = Some(c);
        self
    }

    /// Returns `true` if any component in this structure is enabled.
    #[must_use]
    pub fn has_any_enabled(&self) -> bool {
        self.account_onboarding.as_ref().is_some_and(|c| c.enabled)
            || self.account_management.as_ref().is_some_and(|c| c.enabled)
            || self.balances.as_ref().is_some_and(|c| c.enabled)
            || self.documents.as_ref().is_some_and(|c| c.enabled)
            || self.notification_banner.as_ref().is_some_and(|c| c.enabled)
            || self.payment_details.as_ref().is_some_and(|c| c.enabled)
            || self.payments.as_ref().is_some_and(|c| c.enabled)
            || self.payouts.as_ref().is_some_and(|c| c.enabled)
            || self.payouts_list.as_ref().is_some_and(|c| c.enabled)
            || self.tax_registrations.as_ref().is_some_and(|c| c.enabled)
            || self.tax_settings.as_ref().is_some_and(|c| c.enabled)
    }

    /// Returns `true` if the specified component name is enabled.
    #[must_use]
    pub fn is_component_enabled(&self, name: &str) -> bool {
        match name {
            "account_onboarding" => self.account_onboarding.as_ref().is_some_and(|c| c.enabled),
            "account_management" => self.account_management.as_ref().is_some_and(|c| c.enabled),
            "balances" => self.balances.as_ref().is_some_and(|c| c.enabled),
            "documents" => self.documents.as_ref().is_some_and(|c| c.enabled),
            "notification_banner" => self.notification_banner.as_ref().is_some_and(|c| c.enabled),
            "payment_details" => self.payment_details.as_ref().is_some_and(|c| c.enabled),
            "payments" => self.payments.as_ref().is_some_and(|c| c.enabled),
            "payouts" => self.payouts.as_ref().is_some_and(|c| c.enabled),
            "payouts_list" => self.payouts_list.as_ref().is_some_and(|c| c.enabled),
            "tax_registrations" => self.tax_registrations.as_ref().is_some_and(|c| c.enabled),
            "tax_settings" => self.tax_settings.as_ref().is_some_and(|c| c.enabled),
            _ => false,
        }
    }
}

/// Represents an Account Session object returned by Stripe (`account_session`).
///
/// Implements custom `Debug` to mask `client_secret` to `[REDACTED]` (PCI-DSS 4.0 / Gate G2).
///
/// # Example
/// ```
/// use some_stripe_types::account_session::AccountSession;
///
/// let json = r#"{
///   "object": "account_session",
///   "account": "acct_1032D82eZvKYlo2C",
///   "client_secret": "secret_test_token_abc123",
///   "created": 1700000000,
///   "expires_at": 1700003600,
///   "livemode": false,
///   "components": {
///     "payments": { "enabled": true }
///   }
/// }"#;
/// let session: AccountSession = serde_json::from_str(json).unwrap();
/// assert_eq!(session.account(), "acct_1032D82eZvKYlo2C");
/// assert_eq!(session.client_secret(), "secret_test_token_abc123");
/// assert!(!format!("{session:?}").contains("secret_test_token_abc123"));
/// assert!(format!("{session:?}").contains("[REDACTED]"));
/// ```
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountSession {
    /// String representing the object's type. Always `account_session`.
    #[serde(default = "default_account_session_object")]
    pub object: String,

    /// The ID of the account the Account Session was created for.
    pub account: String,

    /// The client secret of this Account Session used on the client to authenticate.
    pub client_secret: String,

    /// Time at which the object was created (Unix timestamp in seconds).
    pub created: i64,

    /// The timestamp at which this Account Session will expire.
    pub expires_at: i64,

    /// Has the value `true` if the object exists in live mode or `false` if test mode.
    pub livemode: bool,

    /// The components that are enabled for the embedded component.
    pub components: AccountSessionComponents,
}

impl AccountSession {
    /// Returns a borrowed reference to the account ID (Gate G-Borrow).
    #[must_use]
    pub fn account(&self) -> &str {
        &self.account
    }

    /// Returns a borrowed reference to the client secret (Gate G-Borrow).
    #[must_use]
    pub fn client_secret(&self) -> &str {
        &self.client_secret
    }

    /// Returns the creation timestamp in seconds.
    #[must_use]
    pub const fn created(&self) -> i64 {
        self.created
    }

    /// Returns the expiration timestamp in seconds.
    #[must_use]
    pub const fn expires_at(&self) -> i64 {
        self.expires_at
    }

    /// Returns whether this session was created in livemode.
    #[must_use]
    pub const fn is_livemode(&self) -> bool {
        self.livemode
    }

    /// Returns a borrowed reference to the components configuration (Gate G-Borrow).
    #[must_use]
    pub const fn components(&self) -> &AccountSessionComponents {
        &self.components
    }

    /// Returns `true` if the session is expired relative to the given current timestamp.
    #[must_use]
    pub const fn is_expired(&self, current_timestamp: i64) -> bool {
        current_timestamp >= self.expires_at
    }
}

impl fmt::Debug for AccountSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccountSession")
            .field("object", &self.object)
            .field("account", &self.account)
            .field("client_secret", &"[REDACTED]")
            .field("created", &self.created)
            .field("expires_at", &self.expires_at)
            .field("livemode", &self.livemode)
            .field("components", &self.components)
            .finish()
    }
}

/// Errors occurring during Account Session parameter validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountSessionValidationError {
    /// The specified account identifier was empty.
    EmptyAccount,
    /// The specified account identifier is invalid (must start with `acct_`).
    InvalidAccount(String),
    /// No components were enabled in the account session request.
    NoComponentsEnabled,
}

impl fmt::Display for AccountSessionValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAccount => write!(f, "Connected account identifier cannot be empty"),
            Self::InvalidAccount(acct) => write!(
                f,
                "Invalid account identifier '{acct}': must start with 'acct_'"
            ),
            Self::NoComponentsEnabled => write!(
                f,
                "At least one component must be specified and enabled for AccountSession"
            ),
        }
    }
}

impl std::error::Error for AccountSessionValidationError {}

/// Parameters for creating a new [`AccountSession`] (`POST /v1/account_sessions`).
///
/// # Example
/// ```
/// use some_stripe_types::account_session::{AccountSessionComponents, CreateAccountSessionParams, PaymentsComponent};
///
/// let components = AccountSessionComponents::new().with_payments(PaymentsComponent::enabled());
/// let params = CreateAccountSessionParams::new("acct_1032D82eZvKYlo2C", components);
/// assert!(params.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateAccountSessionParams {
    /// The identifier of the account to create an account session for.
    pub account: String,

    /// Each key of the dictionary represents an embedded component, and each value
    /// contains configuration for that component.
    pub components: AccountSessionComponents,

    /// Specifies which fields in the response should be expanded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expand: Option<Vec<String>>,

    /// Optional idempotency key for safe retries (omitted from body serialization).
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateAccountSessionParams {
    /// Constructs parameters for creating an account session with account and components.
    #[must_use]
    pub fn new(account: impl Into<String>, components: AccountSessionComponents) -> Self {
        Self {
            account: account.into(),
            components,
            expand: None,
            idempotency_key: None,
        }
    }

    /// Constructs parameters with an empty component set for the given account.
    #[must_use]
    pub fn for_account(account: impl Into<String>) -> Self {
        Self {
            account: account.into(),
            components: AccountSessionComponents::new(),
            expand: None,
            idempotency_key: None,
        }
    }

    /// Sets the entire components configuration.
    #[must_use]
    pub fn with_components(mut self, components: AccountSessionComponents) -> Self {
        self.components = components;
        self
    }

    /// Attaches an account onboarding component.
    #[must_use]
    pub fn with_account_onboarding(mut self, c: AccountOnboardingComponent) -> Self {
        self.components.account_onboarding = Some(c);
        self
    }

    /// Attaches an account management component.
    #[must_use]
    pub fn with_account_management(mut self, c: AccountManagementComponent) -> Self {
        self.components.account_management = Some(c);
        self
    }

    /// Attaches a balances component.
    #[must_use]
    pub fn with_balances(mut self, c: BalancesComponent) -> Self {
        self.components.balances = Some(c);
        self
    }

    /// Attaches a documents component.
    #[must_use]
    pub fn with_documents(mut self, c: DocumentsComponent) -> Self {
        self.components.documents = Some(c);
        self
    }

    /// Attaches a payments component.
    #[must_use]
    pub fn with_payments(mut self, c: PaymentsComponent) -> Self {
        self.components.payments = Some(c);
        self
    }

    /// Attaches a payouts component.
    #[must_use]
    pub fn with_payouts(mut self, c: PayoutsComponent) -> Self {
        self.components.payouts = Some(c);
        self
    }

    /// Adds an expansion field.
    #[must_use]
    pub fn with_expand(mut self, field: impl Into<String>) -> Self {
        self.expand.get_or_insert_with(Vec::new).push(field.into());
        self
    }

    /// Sets an explicit idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Alias for [`with_idempotency_key`](Self::with_idempotency_key).
    #[must_use]
    pub fn idempotency_key(self, key: impl Into<String>) -> Self {
        self.with_idempotency_key(key)
    }

    /// Validates the parameters against Stripe Connect API invariants.
    ///
    /// # Errors
    /// Returns [`AccountSessionValidationError`] if the account identifier is invalid
    /// or no components are enabled.
    pub fn validate(&self) -> Result<(), AccountSessionValidationError> {
        if self.account.trim().is_empty() {
            return Err(AccountSessionValidationError::EmptyAccount);
        }
        if !self.account.starts_with("acct_") {
            return Err(AccountSessionValidationError::InvalidAccount(
                self.account.clone(),
            ));
        }
        if !self.components.has_any_enabled() {
            return Err(AccountSessionValidationError::NoComponentsEnabled);
        }

        Ok(())
    }
}
