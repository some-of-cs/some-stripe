//! Stripe Treasury Financial Accounts domain models, balance tracking, and request parameters.
//!
//! Enforces FDIC insurance balance limits ($250,000 USD pass-through coverage), feature gating,
//! FinCEN / Bank Secrecy Act (BSA) customer compliance, ABA routing number validation,
//! PCI-DSS PAN isolation, and Rust 2024 zero-clone borrowing discipline.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Standard FDIC pass-through deposit insurance maximum coverage limit in USD cents ($250,000.00).
pub const FDIC_INSURANCE_LIMIT_USD_CENTS: i64 = 25_000_000;

/// Default object string for [`FinancialAccount`].
#[must_use]
pub fn default_financial_account_object() -> String {
    "treasury.financial_account".to_string()
}

/// Default object string for [`FinancialAccountList`].
#[must_use]
fn default_financial_account_list_object() -> String {
    "list".to_string()
}

/// Operational status of a Treasury financial account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinancialAccountStatus {
    /// Account is open and active for transactions.
    Open,
    /// Account has been closed.
    Closed,
}

impl std::fmt::Display for FinancialAccountStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

/// Reasons why a Treasury financial account was closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinancialAccountClosedReason {
    /// Account was closed by the platform.
    ClosedByPlatform,
    /// Account was closed for other administrative reasons.
    Other,
}

/// Details regarding why an account status was changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FinancialAccountStatusDetails {
    /// Details if the financial account is closed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed: Option<FinancialAccountClosedDetails>,
}

/// Reasons and metadata associated with a closed financial account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FinancialAccountClosedDetails {
    /// Reasons for account closure.
    #[serde(default)]
    pub reasons: Vec<FinancialAccountClosedReason>,
}

/// Balances of the financial account categorized by currency.
///
/// Under federal banking regulations and FDIC guidelines, cash deposits in an eligible
/// depository institution are insured up to $250,000 per depositor.
///
/// # Examples
/// ```
/// use some_stripe_types::FinancialAccountBalance;
/// use std::collections::HashMap;
///
/// let mut cash = HashMap::new();
/// cash.insert("usd".to_string(), 350_000_00); // $350,000.00
///
/// let balance = FinancialAccountBalance {
///     cash,
///     inbound_pending: HashMap::new(),
///     outbound_pending: HashMap::new(),
/// };
///
/// assert_eq!(balance.cash_amount("usd"), 350_000_00);
/// assert_eq!(balance.fdic_insured_balance("usd"), 250_000_00);
/// assert_eq!(balance.fdic_excess_balance("usd"), 100_000_00);
/// assert!(!balance.is_within_fdic_limit("usd"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FinancialAccountBalance {
    /// Funds that are available for use immediately, mapped by 3-letter currency code.
    #[serde(default)]
    pub cash: HashMap<String, i64>,

    /// Funds not yet available in the cash balance, but expected soon (e.g., inbound ACH in transit).
    #[serde(default)]
    pub inbound_pending: HashMap<String, i64>,

    /// Funds in the process of being sent out of the financial account.
    #[serde(default)]
    pub outbound_pending: HashMap<String, i64>,
}

impl FinancialAccountBalance {
    /// Returns the cash balance for a given currency (defaulting to 0 if not present).
    #[must_use]
    pub fn cash_amount(&self, currency: &str) -> i64 {
        self.lookup_currency_amount(&self.cash, currency)
    }

    /// Returns the inbound pending amount for a given currency.
    #[must_use]
    pub fn inbound_pending_amount(&self, currency: &str) -> i64 {
        self.lookup_currency_amount(&self.inbound_pending, currency)
    }

    /// Returns the outbound pending amount for a given currency.
    #[must_use]
    pub fn outbound_pending_amount(&self, currency: &str) -> i64 {
        self.lookup_currency_amount(&self.outbound_pending, currency)
    }

    /// Returns the total net cash position (cash + inbound_pending - outbound_pending).
    #[must_use]
    pub fn net_cash(&self, currency: &str) -> i64 {
        let cash = self.cash_amount(currency);
        let inbound = self.inbound_pending_amount(currency);
        let outbound = self.outbound_pending_amount(currency);
        cash.saturating_add(inbound).saturating_sub(outbound)
    }

    /// Computes the FDIC insured portion of the cash balance for USD accounts.
    ///
    /// Capped at [`FDIC_INSURANCE_LIMIT_USD_CENTS`] ($250,000.00). Returns 0 for non-USD currencies.
    #[must_use]
    pub fn fdic_insured_balance(&self, currency: &str) -> i64 {
        if !currency.eq_ignore_ascii_case("usd") {
            return 0;
        }
        let cash = self.cash_amount(currency);
        cash.clamp(0, FDIC_INSURANCE_LIMIT_USD_CENTS)
    }

    /// Computes the balance exceeding the FDIC insurance coverage limit ($250,000.00).
    ///
    /// Returns 0 if within the limit or if the currency is not USD.
    #[must_use]
    pub fn fdic_excess_balance(&self, currency: &str) -> i64 {
        if !currency.eq_ignore_ascii_case("usd") {
            return 0;
        }
        let cash = self.cash_amount(currency);
        (cash - FDIC_INSURANCE_LIMIT_USD_CENTS).max(0)
    }

    /// Returns `true` if the cash balance is within the statutory FDIC insurance limit.
    #[must_use]
    pub fn is_within_fdic_limit(&self, currency: &str) -> bool {
        if !currency.eq_ignore_ascii_case("usd") {
            return true;
        }
        self.cash_amount(currency) <= FDIC_INSURANCE_LIMIT_USD_CENTS
    }

    fn lookup_currency_amount(&self, map: &HashMap<String, i64>, currency: &str) -> i64 {
        let lower = currency.to_ascii_lowercase();
        map.get(&lower).copied().unwrap_or(0)
    }
}

/// Operational status of a specific Treasury feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FeatureStatus {
    /// Feature is live and operational.
    Active,
    /// Feature activation is pending review or compliance checks.
    #[default]
    Pending,
    /// Feature has been restricted or disabled.
    Restricted,
}

/// Status and onboarding details for an individual feature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FeatureStatusDetails {
    /// Current operational status.
    pub status: FeatureStatus,

    /// Whether the platform requested this feature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested: Option<bool>,
}

/// Feature details for ABA financial addresses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FinancialAddressesFeatureDetails {
    /// ABA routing details feature status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aba: Option<FeatureStatusDetails>,
}

/// Feature details for inbound transfers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InboundTransfersFeatureDetails {
    /// ACH inbound transfer feature status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ach: Option<FeatureStatusDetails>,
}

/// Feature details for outbound payments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OutboundPaymentsFeatureDetails {
    /// ACH outbound payment feature status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ach: Option<FeatureStatusDetails>,

    /// US domestic wire outbound payment feature status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub us_domestic_wire: Option<FeatureStatusDetails>,
}

/// Feature details for outbound transfers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OutboundTransfersFeatureDetails {
    /// ACH outbound transfer feature status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ach: Option<FeatureStatusDetails>,

    /// US domestic wire outbound transfer feature status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub us_domestic_wire: Option<FeatureStatusDetails>,
}

/// Granular feature flags and activation states on a Treasury Financial Account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FinancialAccountFeatures {
    /// Commercial card issuing capability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_issuing: Option<FeatureStatusDetails>,

    /// Pass-through FDIC deposit insurance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deposit_insurance: Option<FeatureStatusDetails>,

    /// Routing and account address provisioning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub financial_addresses: Option<FinancialAddressesFeatureDetails>,

    /// Inbound money movement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbound_transfers: Option<InboundTransfersFeatureDetails>,

    /// Internal Stripe ledger movement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intra_stripe_flows: Option<FeatureStatusDetails>,

    /// Outbound third-party payments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbound_payments: Option<OutboundPaymentsFeatureDetails>,

    /// Outbound account transfers to connected or external accounts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbound_transfers: Option<OutboundTransfersFeatureDetails>,
}

impl FinancialAccountFeatures {
    /// Returns `true` if commercial card issuing is active.
    #[must_use]
    pub fn can_issue_cards(&self) -> bool {
        Self::is_active(&self.card_issuing)
    }

    /// Returns `true` if deposit insurance is active.
    #[must_use]
    pub fn has_deposit_insurance(&self) -> bool {
        Self::is_active(&self.deposit_insurance)
    }

    /// Returns `true` if ACH inbound transfer feature is active.
    #[must_use]
    pub fn can_inbound_transfer_ach(&self) -> bool {
        self.financial_addresses
            .as_ref()
            .and_then(|a| a.aba.as_ref())
            .is_some_and(|s| s.status == FeatureStatus::Active)
    }

    /// Returns `true` if ACH outbound payment is active.
    #[must_use]
    pub fn can_outbound_pay_ach(&self) -> bool {
        self.outbound_payments
            .as_ref()
            .and_then(|p| p.ach.as_ref())
            .is_some_and(|s| s.status == FeatureStatus::Active)
    }

    /// Returns `true` if domestic wire outbound payment is active.
    #[must_use]
    pub fn can_outbound_pay_wire(&self) -> bool {
        self.outbound_payments
            .as_ref()
            .and_then(|p| p.us_domestic_wire.as_ref())
            .is_some_and(|s| s.status == FeatureStatus::Active)
    }

    /// Returns `true` if ACH outbound transfer is active.
    #[must_use]
    pub fn can_outbound_transfer_ach(&self) -> bool {
        self.outbound_transfers
            .as_ref()
            .and_then(|t| t.ach.as_ref())
            .is_some_and(|s| s.status == FeatureStatus::Active)
    }

    /// Returns `true` if domestic wire outbound transfer is active.
    #[must_use]
    pub fn can_outbound_transfer_wire(&self) -> bool {
        self.outbound_transfers
            .as_ref()
            .and_then(|t| t.us_domestic_wire.as_ref())
            .is_some_and(|s| s.status == FeatureStatus::Active)
    }

    fn is_active(opt: &Option<FeatureStatusDetails>) -> bool {
        opt.as_ref()
            .is_some_and(|d| d.status == FeatureStatus::Active)
    }
}

/// Validates ABA routing transit numbers using the Federal Reserve Mod-10 checksum.
///
/// Weight formula: `(3*(d1 + d4 + d7) + 7*(d2 + d5 + d8) + 1*(d3 + d6 + d9)) % 10 == 0`.
#[must_use]
pub fn is_valid_aba_routing_number(routing: &str) -> bool {
    let trimmed = routing.trim();
    if trimmed.len() != 9 || !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    let digits: Vec<u32> = trimmed.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() != 9 {
        return false;
    }

    let weighted_sum = (3 * (digits[0] + digits[3] + digits[6]))
        + (7 * (digits[1] + digits[4] + digits[7]))
        + (digits[2] + digits[5] + digits[8]);

    weighted_sum.is_multiple_of(10)
}

/// Detects whether an account or routing number mistakenly contains a raw Primary Account Number (PAN).
///
use crate::issuing_authorization::contains_raw_pan;

/// Details for an ABA financial address (routing and account number).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AbaRecord {
    /// Full bank account number (only available if specifically revealed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_number: Option<String>,

    /// Last four digits of the bank account number.
    pub account_number_last4: String,

    /// 9-digit ABA routing transit number.
    pub routing_number: String,

    /// Name of the depository bank (e.g. "Evolve Bank & Trust").
    pub bank_name: String,
}

impl AbaRecord {
    /// Validates compliance with FinCEN / BSA banking regulations and PCI-DSS PAN isolation.
    ///
    /// # Errors
    /// Returns [`FinancialAccountValidationError`] if the routing number or account format is invalid.
    pub fn validate_compliance(&self) -> Result<(), FinancialAccountValidationError> {
        if !is_valid_aba_routing_number(&self.routing_number) {
            return Err(FinancialAccountValidationError::InvalidRoutingNumber {
                routing: self.routing_number.clone(),
            });
        }

        if self.account_number_last4.len() != 4
            || !self
                .account_number_last4
                .chars()
                .all(|c| c.is_ascii_digit())
        {
            return Err(FinancialAccountValidationError::InvalidAccountNumber {
                reason: "Account number last4 must contain exactly 4 numeric digits".into(),
            });
        }

        if let Some(ref full_acct) = self.account_number
            && contains_raw_pan(full_acct)
        {
            return Err(FinancialAccountValidationError::PanIsolationViolation);
        }

        if self.bank_name.trim().is_empty() {
            return Err(FinancialAccountValidationError::InvalidParameter {
                field: "bank_name",
                reason: "Depository bank name must not be empty".into(),
            });
        }

        Ok(())
    }
}

/// Financial address attached to a FinancialAccount.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinancialAddress {
    /// The type of financial address (typically "aba").
    pub address_type: String,

    /// ABA routing and account details if `address_type` is "aba".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aba: Option<AbaRecord>,
}

/// Container for financial addresses associated with a Treasury financial account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FinancialAddresses {
    /// The collection of financial addresses.
    #[serde(default)]
    pub addresses: Vec<FinancialAddress>,
}

impl FinancialAddresses {
    /// Returns the primary ABA record if present.
    #[must_use]
    pub fn primary_aba(&self) -> Option<&AbaRecord> {
        self.addresses
            .iter()
            .find(|a| a.address_type == "aba")
            .and_then(|a| a.aba.as_ref())
    }

    /// Returns the primary routing number if present.
    #[must_use]
    pub fn routing_number(&self) -> Option<&str> {
        self.primary_aba().map(|a| a.routing_number.as_str())
    }

    /// Returns the primary account number last4 if present.
    #[must_use]
    pub fn account_number_last4(&self) -> Option<&str> {
        self.primary_aba().map(|a| a.account_number_last4.as_str())
    }
}

/// Flow restrictions placed on financial accounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowRestriction {
    /// Transactions are blocked.
    Restricted,
    /// Transactions are permitted.
    Unrestricted,
}

/// Platform restrictions applied to the financial account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FinancialAccountPlatformRestrictions {
    /// Inbound flow restriction status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbound_flows: Option<FlowRestriction>,

    /// Outbound flow restriction status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbound_flows: Option<FlowRestriction>,
}

/// A Stripe Treasury Financial Account representing an FDIC-insured commercial bank account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinancialAccount {
    /// Unique identifier for the financial account (prefixed `fa_`).
    pub id: String,

    /// String representing the object's type, always `treasury.financial_account`.
    #[serde(default = "default_financial_account_object")]
    pub object: String,

    /// The array of features that are currently active on this financial account.
    #[serde(default)]
    pub active_features: Vec<String>,

    /// Balances of the financial account.
    pub balance: FinancialAccountBalance,

    /// Two-letter country code (ISO 3166-1 alpha-2).
    pub country: String,

    /// Time at which the object was created (Unix timestamp).
    pub created: i64,

    /// Encapsulates feature details and request states.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<FinancialAccountFeatures>,

    /// The set of financial addresses associated with this account.
    #[serde(default)]
    pub financial_addresses: Vec<FinancialAddress>,

    /// Has the value `true` if the object exists in live mode or `false` in test mode.
    pub livemode: bool,

    /// Set of key-value pairs attached to the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// The array of features that are pending activation.
    #[serde(default)]
    pub pending_features: Vec<String>,

    /// The platform restrictions applied to the financial account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_restrictions: Option<FinancialAccountPlatformRestrictions>,

    /// The array of features that are restricted.
    #[serde(default)]
    pub restricted_features: Vec<String>,

    /// Operational status of the financial account.
    pub status: FinancialAccountStatus,

    /// Status details for the financial account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_details: Option<FinancialAccountStatusDetails>,

    /// The currencies the financial account can hold.
    #[serde(default)]
    pub supported_currencies: Vec<String>,
}

impl FinancialAccount {
    /// Returns `true` if the financial account is active and open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.status == FinancialAccountStatus::Open
    }

    /// Returns `true` if the financial account is closed.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.status == FinancialAccountStatus::Closed
    }

    /// Returns `true` if the account has the specified active feature name.
    #[must_use]
    pub fn has_active_feature(&self, feature_name: &str) -> bool {
        self.active_features.iter().any(|f| f == feature_name)
    }

    /// Returns `true` if pass-through deposit insurance is active.
    #[must_use]
    pub fn has_deposit_insurance(&self) -> bool {
        self.has_active_feature("deposit_insurance")
            || self
                .features
                .as_ref()
                .is_some_and(|f| f.has_deposit_insurance())
    }

    /// Convenience accessor for the primary ABA routing number.
    #[must_use]
    pub fn routing_number(&self) -> Option<&str> {
        self.financial_addresses
            .iter()
            .find(|a| a.address_type == "aba")
            .and_then(|a| a.aba.as_ref())
            .map(|aba| aba.routing_number.as_str())
    }

    /// Convenience accessor for the account number last4.
    #[must_use]
    pub fn account_number_last4(&self) -> Option<&str> {
        self.financial_addresses
            .iter()
            .find(|a| a.address_type == "aba")
            .and_then(|a| a.aba.as_ref())
            .map(|aba| aba.account_number_last4.as_str())
    }
}

/// Paginated list of [`FinancialAccount`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinancialAccountList {
    /// String representing the object's type, always `list`.
    #[serde(default = "default_financial_account_list_object")]
    pub object: String,

    /// Details about each object in the list.
    pub data: Vec<FinancialAccount>,

    /// True if this list has another page of items after this one.
    pub has_more: bool,

    /// The URL where this list can be accessed.
    pub url: String,
}

// ============================================================================
// Parameter Structures & Validation
// ============================================================================

/// Feature request options for creating or updating a Financial Account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FeatureRequestParam {
    /// Whether the feature should be requested.
    pub requested: bool,
}

/// Inbound transfer feature parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InboundTransfersFeatureParams {
    /// Request ACH inbound transfers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ach: Option<FeatureRequestParam>,
}

/// Outbound payments feature parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OutboundPaymentsFeatureParams {
    /// Request ACH outbound payments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ach: Option<FeatureRequestParam>,

    /// Request US domestic wire outbound payments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub us_domestic_wire: Option<FeatureRequestParam>,
}

/// Outbound transfers feature parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OutboundTransfersFeatureParams {
    /// Request ACH outbound transfers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ach: Option<FeatureRequestParam>,

    /// Request US domestic wire outbound transfers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub us_domestic_wire: Option<FeatureRequestParam>,
}

/// Financial addresses feature parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FinancialAddressesFeatureParams {
    /// Request ABA routing and account addresses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aba: Option<FeatureRequestParam>,
}

/// Feature settings to request on a Financial Account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FinancialAccountFeaturesParams {
    /// Request commercial card issuing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_issuing: Option<FeatureRequestParam>,

    /// Request pass-through deposit insurance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deposit_insurance: Option<FeatureRequestParam>,

    /// Request financial addresses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub financial_addresses: Option<FinancialAddressesFeatureParams>,

    /// Request inbound transfer capabilities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbound_transfers: Option<InboundTransfersFeatureParams>,

    /// Request intra-Stripe transfer capability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intra_stripe_flows: Option<FeatureRequestParam>,

    /// Request outbound payments capability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbound_payments: Option<OutboundPaymentsFeatureParams>,

    /// Request outbound transfers capability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbound_transfers: Option<OutboundTransfersFeatureParams>,
}

/// Platform restriction parameters for a Financial Account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FinancialAccountPlatformRestrictionsParams {
    /// Inbound flow restriction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbound_flows: Option<FlowRestriction>,

    /// Outbound flow restriction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbound_flows: Option<FlowRestriction>,
}

/// Parameters for creating a Treasury Financial Account (`POST /v1/treasury/financial_accounts`).
///
/// # Example
/// ```
/// use some_stripe_types::CreateFinancialAccountParams;
///
/// let params = CreateFinancialAccountParams::new(vec!["usd".to_string()])
///     .with_idempotency_key("idem_fa_create_123");
///
/// assert!(params.validate().is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateFinancialAccountParams {
    /// The currencies the financial account can hold. At least one must be provided.
    pub supported_currencies: Vec<String>,

    /// Granular features to request on the account upon creation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<FinancialAccountFeaturesParams>,

    /// The platform restrictions to apply to the financial account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_restrictions: Option<FinancialAccountPlatformRestrictionsParams>,

    /// Key-value metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Optional client-side idempotency key.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateFinancialAccountParams {
    /// Creates a new parameter builder with the given supported currencies.
    #[must_use]
    pub fn new(supported_currencies: Vec<String>) -> Self {
        Self {
            supported_currencies,
            features: None,
            platform_restrictions: None,
            metadata: None,
            idempotency_key: None,
        }
    }

    /// Adds a supported currency.
    #[must_use]
    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.supported_currencies.push(currency.into());
        self
    }

    /// Sets feature request parameters.
    #[must_use]
    pub fn with_features(mut self, features: FinancialAccountFeaturesParams) -> Self {
        self.features = Some(features);
        self
    }

    /// Sets platform restrictions.
    #[must_use]
    pub fn with_platform_restrictions(
        mut self,
        restrictions: FinancialAccountPlatformRestrictionsParams,
    ) -> Self {
        self.platform_restrictions = Some(restrictions);
        self
    }

    /// Sets metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets an idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates parameters under ISO 4217 and banking compliance requirements.
    ///
    /// # Errors
    /// Returns [`FinancialAccountValidationError`] if validation checks fail.
    pub fn validate(&self) -> Result<(), FinancialAccountValidationError> {
        if self.supported_currencies.is_empty() {
            return Err(FinancialAccountValidationError::EmptySupportedCurrencies);
        }

        for cur in &self.supported_currencies {
            if cur.trim().len() != 3 {
                return Err(FinancialAccountValidationError::InvalidCurrency {
                    currency: cur.clone(),
                });
            }
        }

        Ok(())
    }
}

/// Parameters for updating a Treasury Financial Account (`POST /v1/treasury/financial_accounts/{id}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UpdateFinancialAccountParams {
    /// Granular features to update or request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<FinancialAccountFeaturesParams>,

    /// Platform restrictions to update.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_restrictions: Option<FinancialAccountPlatformRestrictionsParams>,

    /// Key-value metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Optional client-side idempotency key.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl UpdateFinancialAccountParams {
    /// Creates an empty update parameter builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets feature request parameters.
    #[must_use]
    pub fn with_features(mut self, features: FinancialAccountFeaturesParams) -> Self {
        self.features = Some(features);
        self
    }

    /// Sets platform restrictions.
    #[must_use]
    pub fn with_platform_restrictions(
        mut self,
        restrictions: FinancialAccountPlatformRestrictionsParams,
    ) -> Self {
        self.platform_restrictions = Some(restrictions);
        self
    }

    /// Sets metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets an idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates parameters.
    ///
    /// # Errors
    /// Returns [`FinancialAccountValidationError`] if validation checks fail.
    pub fn validate(&self) -> Result<(), FinancialAccountValidationError> {
        Ok(())
    }
}

/// Parameters for listing Treasury Financial Accounts (`GET /v1/treasury/financial_accounts`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ListFinancialAccountsParams {
    /// Filter accounts by creation timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,

    /// A cursor for pagination: fetch items before this ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,

    /// A cursor for pagination: fetch items after this ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,

    /// Limit on the number of accounts to return (1..100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Filter accounts by status (`open` or `closed`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<FinancialAccountStatus>,
}

impl ListFinancialAccountsParams {
    /// Creates an empty list parameter builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the pagination limit.
    #[must_use]
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the status filter.
    #[must_use]
    pub fn with_status(mut self, status: FinancialAccountStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Sets starting after cursor.
    #[must_use]
    pub fn with_starting_after(mut self, cursor: impl Into<String>) -> Self {
        self.starting_after = Some(cursor.into());
        self
    }

    /// Sets ending before cursor.
    #[must_use]
    pub fn with_ending_before(mut self, cursor: impl Into<String>) -> Self {
        self.ending_before = Some(cursor.into());
        self
    }

    /// Validates pagination parameters.
    ///
    /// # Errors
    /// Returns [`FinancialAccountValidationError`] if limit is out of range.
    pub fn validate(&self) -> Result<(), FinancialAccountValidationError> {
        if let Some(l) = self.limit
            && !(1..=100).contains(&l)
        {
            return Err(FinancialAccountValidationError::InvalidLimit { limit: l });
        }
        Ok(())
    }
}

/// Destination type for residual balance forwarding when closing an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForwardingType {
    /// Forward funds to another financial address.
    FinancialAddress,
    /// Forward funds to an external payment method.
    PaymentMethod,
}

/// Forwarding settings when closing a Financial Account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ForwardingSettingsParams {
    /// Destination financial address ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub financial_address: Option<String>,

    /// Destination payment method ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<String>,

    /// The type of forwarding destination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ForwardingType>,
}

/// Parameters for closing a Treasury Financial Account (`POST /v1/treasury/financial_accounts/{id}/close`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CloseFinancialAccountParams {
    /// Settings for forwarding residual balances.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forwarding_settings: Option<ForwardingSettingsParams>,

    /// Optional client-side idempotency key.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CloseFinancialAccountParams {
    /// Creates an empty close parameter builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures forwarding to an external payment method.
    #[must_use]
    pub fn with_forwarding_payment_method(mut self, pm_id: impl Into<String>) -> Self {
        self.forwarding_settings = Some(ForwardingSettingsParams {
            payment_method: Some(pm_id.into()),
            financial_address: None,
            r#type: Some(ForwardingType::PaymentMethod),
        });
        self
    }

    /// Configures forwarding to another financial address.
    #[must_use]
    pub fn with_forwarding_financial_address(mut self, address_id: impl Into<String>) -> Self {
        self.forwarding_settings = Some(ForwardingSettingsParams {
            payment_method: None,
            financial_address: Some(address_id.into()),
            r#type: Some(ForwardingType::FinancialAddress),
        });
        self
    }

    /// Sets an idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates closure parameters.
    ///
    /// # Errors
    /// Returns [`FinancialAccountValidationError`] if forwarding configuration is invalid.
    pub fn validate(&self) -> Result<(), FinancialAccountValidationError> {
        if let Some(ref f) = self.forwarding_settings
            && f.financial_address.is_none()
            && f.payment_method.is_none()
        {
            return Err(FinancialAccountValidationError::InvalidParameter {
                field: "forwarding_settings",
                reason: "Must specify either financial_address or payment_method for residual balance forwarding".into(),
            });
        }
        Ok(())
    }
}

// ============================================================================
// Errors
// ============================================================================

/// Errors encountered when validating Financial Account parameters or compliance rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinancialAccountValidationError {
    /// At least one supported currency is required.
    EmptySupportedCurrencies,
    /// Invalid currency code.
    InvalidCurrency {
        /// The invalid currency string.
        currency: String,
    },
    /// Pagination limit out of bounds (must be 1..=100).
    InvalidLimit {
        /// The invalid limit supplied.
        limit: u32,
    },
    /// ABA routing number is invalid or fails checksum.
    InvalidRoutingNumber {
        /// The invalid routing number.
        routing: String,
    },
    /// Invalid bank account number structure.
    InvalidAccountNumber {
        /// Reason for failure.
        reason: String,
    },
    /// Invalid financial account identifier.
    InvalidId {
        /// The invalid ID.
        id: String,
    },
    /// Primary Account Number (PAN) isolation violation under PCI-DSS 4.0.
    PanIsolationViolation,
    /// General invalid parameter.
    InvalidParameter {
        /// Field name.
        field: &'static str,
        /// Description of the error.
        reason: String,
    },
}

impl std::fmt::Display for FinancialAccountValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySupportedCurrencies => {
                write!(f, "Financial account must support at least one currency")
            }
            Self::InvalidCurrency { currency } => {
                write!(
                    f,
                    "Supported currency must be a valid 3-letter ISO code, got: '{currency}'"
                )
            }
            Self::InvalidLimit { limit } => {
                write!(f, "List limit must be between 1 and 100, got: {limit}")
            }
            Self::InvalidRoutingNumber { routing } => {
                write!(
                    f,
                    "Invalid 9-digit ABA routing transit number checksum: '{routing}'"
                )
            }
            Self::InvalidAccountNumber { reason } => {
                write!(f, "Invalid bank account number: {reason}")
            }
            Self::InvalidId { id } => {
                write!(
                    f,
                    "Invalid financial account ID (must start with 'fa_'): '{id}'"
                )
            }
            Self::PanIsolationViolation => {
                write!(
                    f,
                    "PCI-DSS PAN isolation violation: raw credit card PAN detected in bank account field"
                )
            }
            Self::InvalidParameter { field, reason } => {
                write!(f, "Invalid parameter '{field}': {reason}")
            }
        }
    }
}

impl std::error::Error for FinancialAccountValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fdic_insurance_balance_calculations() {
        let mut cash = HashMap::new();
        cash.insert("usd".to_string(), 35_000_000); // $350k

        let balance = FinancialAccountBalance {
            cash,
            inbound_pending: HashMap::new(),
            outbound_pending: HashMap::new(),
        };

        assert_eq!(balance.cash_amount("usd"), 35_000_000);
        assert_eq!(balance.fdic_insured_balance("usd"), 25_000_000);
        assert_eq!(balance.fdic_excess_balance("usd"), 10_000_000);
        assert!(!balance.is_within_fdic_limit("usd"));

        let mut within_cash = HashMap::new();
        within_cash.insert("usd".to_string(), 20_000_000); // $200k
        let within_bal = FinancialAccountBalance {
            cash: within_cash,
            inbound_pending: HashMap::new(),
            outbound_pending: HashMap::new(),
        };
        assert_eq!(within_bal.fdic_insured_balance("usd"), 20_000_000);
        assert_eq!(within_bal.fdic_excess_balance("usd"), 0);
        assert!(within_bal.is_within_fdic_limit("usd"));
    }

    #[test]
    fn test_aba_routing_number_validation() {
        // Valid Federal Reserve routing numbers:
        // 011000015 (Federal Reserve Bank of Boston)
        // 3*(0+0+0) + 7*(1+0+1) + 1*(1+0+5) = 0 + 14 + 6 = 20 % 10 == 0
        assert!(is_valid_aba_routing_number("011000015"));

        // 121000358 (Federal Reserve Bank of San Francisco)
        // 3*(1+0+3) + 7*(2+0+5) + 1*(1+0+8) = 12 + 49 + 9 = 70 % 10 == 0
        assert!(is_valid_aba_routing_number("121000358"));

        // Invalid: wrong length
        assert!(!is_valid_aba_routing_number("12345678"));
        assert!(!is_valid_aba_routing_number("1234567890"));

        // Invalid: bad checksum
        assert!(!is_valid_aba_routing_number("123456789"));
    }

    #[test]
    fn test_pan_isolation_detection() {
        // Valid Luhn test card PAN (e.g. Visa 4242 4242 4242 4242) -> should be detected as PAN
        assert!(contains_raw_pan("4242424242424242"));

        // Normal bank account numbers (not matching Luhn card pattern)
        assert!(!contains_raw_pan("123456789"));
        assert!(!contains_raw_pan("987654321012345"));
    }

    #[test]
    fn test_create_params_validation() {
        let empty = CreateFinancialAccountParams::new(vec![]);
        assert!(matches!(
            empty.validate(),
            Err(FinancialAccountValidationError::EmptySupportedCurrencies)
        ));

        let invalid_curr = CreateFinancialAccountParams::new(vec!["usdollar".into()]);
        assert!(matches!(
            invalid_curr.validate(),
            Err(FinancialAccountValidationError::InvalidCurrency { .. })
        ));

        let valid =
            CreateFinancialAccountParams::new(vec!["usd".into()]).with_idempotency_key("idem_123");
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_list_params_validation() {
        let valid = ListFinancialAccountsParams::new().with_limit(50);
        assert!(valid.validate().is_ok());

        let invalid_0 = ListFinancialAccountsParams::new().with_limit(0);
        assert!(invalid_0.validate().is_err());

        let invalid_101 = ListFinancialAccountsParams::new().with_limit(101);
        assert!(invalid_101.validate().is_err());
    }

    #[test]
    fn test_close_params_validation() {
        let empty = CloseFinancialAccountParams::new();
        assert!(empty.validate().is_ok());

        let with_pm = CloseFinancialAccountParams::new().with_forwarding_payment_method("pm_123");
        assert!(with_pm.validate().is_ok());

        let with_fa =
            CloseFinancialAccountParams::new().with_forwarding_financial_address("fa_999");
        assert!(with_fa.validate().is_ok());
    }
}
