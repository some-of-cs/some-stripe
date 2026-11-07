//! Stripe Treasury Outbound Transfers, Outbound Payments, and Received Credits.
//!
//! Models the commercial banking money movement lifecycle under Stripe Treasury:
//! - [`OutboundTransfer`]: Funds movement from a FinancialAccount to another FinancialAccount or external bank account.
//! - [`OutboundPayment`]: Funds movement initiated via payment rails (e.g., ACH, Fedwire) to an external counterparty.
//! - [`ReceivedCredit`]: Inbound credit received into a FinancialAccount.
//!
//! # Legal and Regulatory Compliance (Gate G-Law & G2)
//!
//! - **Bank Secrecy Act (BSA) & AML (31 U.S.C. § 5311 et seq.)**:
//!   - FinCEN Travel Rule (31 CFR § 1010.410(e)): Outbound wire transfers of $3,000.00 (300,000 cents)
//!     or more require originator and transmittor verification.
//!   - Currency Transaction Reporting (CTR, 31 CFR § 1010.311): Transfers exceeding $10,000.00 (1,000,000 cents)
//!     are classified as CTR reportable.
//! - **Balance Non-Negativity**: Available funds verification prevents overdraft / balance deficits.
//! - **Zero PAN Leakage (PCI-DSS 4.0 Requirement 3.4)**: Explicit scanning verifies Primary Account Numbers
//!   are never injected into arbitrary string fields (descriptions, descriptors, metadata).
//!
//! # Example
//!
//! ```
//! use some_stripe_types::{
//!     CreateOutboundTransferParams, OutboundTransferStatus, classify_bsa_aml_transfer,
//!     BsaAmlClassification,
//! };
//!
//! let params = CreateOutboundTransferParams::new("fa_123", 250000, "usd")
//!     .with_destination_payment_method("pm_bank_wire_123");
//!
//! assert!(params.validate().is_ok());
//! assert!(params.verify_balance(1000000).is_ok());
//! assert_eq!(classify_bsa_aml_transfer(250000), BsaAmlClassification::Standard);
//! assert_eq!(classify_bsa_aml_transfer(300000), BsaAmlClassification::TravelRuleReportable);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

// ============================================================================
// 1. Regulatory Constants & Compliance Thresholds (Gate G-Law)
// ============================================================================

/// FinCEN Travel Rule threshold for funds transmittals in minor currency units ($3,000.00 = 300,000 cents).
pub const FINCEN_TRAVEL_RULE_THRESHOLD_CENTS: i64 = 300_000;

/// Currency Transaction Reporting (CTR) threshold in minor currency units ($10,000.00 = 1,000,000 cents).
pub const CTR_REPORTING_THRESHOLD_CENTS: i64 = 1_000_000;

/// Default window duration in seconds for velocity tracking (24 hours = 86,400 seconds).
pub const DEFAULT_VELOCITY_WINDOW_SECONDS: i64 = 86_400;

// ============================================================================
// 2. Status Enums & Lifecycle States
// ============================================================================

/// Status of an OutboundTransfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboundTransferStatus {
    /// The transfer has been canceled.
    Canceled,
    /// The transfer has failed.
    Failed,
    /// The transfer has posted and balance has been debited.
    Posted,
    /// The transfer is actively processing on banking rails.
    Processing,
    /// The transfer was returned by the receiving institution.
    Returned,
}

impl OutboundTransferStatus {
    /// Returns the static string slice representation for this status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Canceled => "canceled",
            Self::Failed => "failed",
            Self::Posted => "posted",
            Self::Processing => "processing",
            Self::Returned => "returned",
        }
    }
}

impl std::fmt::Display for OutboundTransferStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for OutboundTransferStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for OutboundTransferStatus {
    type Err = ParseTreasuryStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "canceled" => Ok(Self::Canceled),
            "failed" => Ok(Self::Failed),
            "posted" => Ok(Self::Posted),
            "processing" => Ok(Self::Processing),
            "returned" => Ok(Self::Returned),
            _ => Err(ParseTreasuryStatusError(s.to_string())),
        }
    }
}

impl PartialEq<str> for OutboundTransferStatus {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for OutboundTransferStatus {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

/// Status of an OutboundPayment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboundPaymentStatus {
    /// The payment has been canceled.
    Canceled,
    /// The payment has failed.
    Failed,
    /// The payment has posted and funds have been debited.
    Posted,
    /// The payment is actively processing on banking networks.
    Processing,
    /// The payment was returned.
    Returned,
}

impl OutboundPaymentStatus {
    /// Returns the static string slice representation for this status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Canceled => "canceled",
            Self::Failed => "failed",
            Self::Posted => "posted",
            Self::Processing => "processing",
            Self::Returned => "returned",
        }
    }
}

impl std::fmt::Display for OutboundPaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for OutboundPaymentStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for OutboundPaymentStatus {
    type Err = ParseTreasuryStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "canceled" => Ok(Self::Canceled),
            "failed" => Ok(Self::Failed),
            "posted" => Ok(Self::Posted),
            "processing" => Ok(Self::Processing),
            "returned" => Ok(Self::Returned),
            _ => Err(ParseTreasuryStatusError(s.to_string())),
        }
    }
}

impl PartialEq<str> for OutboundPaymentStatus {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for OutboundPaymentStatus {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

/// Status of a ReceivedCredit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceivedCreditStatus {
    /// The credit succeeded and balance has been credited.
    Succeeded,
    /// The credit failed.
    Failed,
}

impl ReceivedCreditStatus {
    /// Returns the static string slice representation for this status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

impl std::fmt::Display for ReceivedCreditStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for ReceivedCreditStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for ReceivedCreditStatus {
    type Err = ParseTreasuryStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            _ => Err(ParseTreasuryStatusError(s.to_string())),
        }
    }
}

/// Error returned when parsing an unrecognized treasury status string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseTreasuryStatusError(pub String);

impl std::fmt::Display for ParseTreasuryStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown Stripe Treasury status: '{}'", self.0)
    }
}

impl std::error::Error for ParseTreasuryStatusError {}

// ============================================================================
// 3. Supporting Sub-models
// ============================================================================

/// Address structure for Treasury billing details.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreasuryBillingAddress {
    /// City, district, suburb, town, or village.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// Two-letter country code (ISO 3166-1 alpha-2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Address line 1 (e.g., street, PO Box, or company name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line1: Option<String>,
    /// Address line 2 (e.g., apartment, suite, unit, or building).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line2: Option<String>,
    /// ZIP or postal code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    /// State, county, province, or region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

/// Billing details for an outbound transfer or payment.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreasuryBillingDetails {
    /// Billing address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<TreasuryBillingAddress>,
    /// Email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Full name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Summary of a FinancialAccount destination.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreasuryFinancialAccountSummary {
    /// The FinancialAccount ID.
    pub id: String,
    /// Network name (e.g., "stripe").
    #[serde(default = "default_stripe_network")]
    pub network: String,
}

fn default_stripe_network() -> String {
    "stripe".into()
}

/// Details of a US bank account payment method.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreasuryUsBankAccountDetails {
    /// Account holder name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_holder_name: Option<String>,
    /// Account type (`checking` or `savings`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_type: Option<String>,
    /// Name of the bank.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bank_name: Option<String>,
    /// Unique fingerprint for the account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    /// Last 4 digits of the account number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last4: Option<String>,
    /// Routing transit number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing_number: Option<String>,
}

/// Destination payment method details for OutboundTransfers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundTransferDestinationPaymentMethodDetails {
    /// Type of destination payment method (`financial_account`, `us_bank_account`).
    #[serde(rename = "type")]
    pub type_: String,
    /// Billing details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing_details: Option<TreasuryBillingDetails>,
    /// Financial account details if destination is a FinancialAccount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub financial_account: Option<TreasuryFinancialAccountSummary>,
    /// US bank account details if destination is an external bank account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub us_bank_account: Option<TreasuryUsBankAccountDetails>,
}

/// Details when an OutboundTransfer is returned.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundTransferReturnedDetails {
    /// Reason code for the return (e.g., `account_closed`, `insufficient_funds`).
    pub code: String,
    /// The Transaction associated with this return.
    pub transaction: String,
}

/// Status transition timestamps for OutboundTransfers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundTransferStatusTransitions {
    /// Timestamp when transfer was canceled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canceled_at: Option<i64>,
    /// Timestamp when transfer failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed_at: Option<i64>,
    /// Timestamp when transfer posted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub posted_at: Option<i64>,
    /// Timestamp when transfer was returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returned_at: Option<i64>,
}

/// ACH tracking details.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AchTrackingDetails {
    /// ACH trace number.
    pub trace_number: String,
}

/// US Domestic Wire tracking details.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsDomesticWireTrackingDetails {
    /// CHIPS reference identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chips: Option<String>,
    /// Input Message Accountability Data (IMAD).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imad: Option<String>,
    /// Output Message Accountability Data (OMAD).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub omad: Option<String>,
}

/// Tracking details for banking network transmittals.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundTransferTrackingDetails {
    /// Type of tracking details (`ach`, `us_domestic_wire`).
    #[serde(rename = "type")]
    pub type_: String,
    /// ACH tracking details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ach: Option<AchTrackingDetails>,
    /// US Domestic Wire tracking details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub us_domestic_wire: Option<UsDomesticWireTrackingDetails>,
}

/// Destination payment method details for OutboundPayments.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundPaymentDestinationPaymentMethodDetails {
    /// Type of destination payment method (`financial_account`, `us_bank_account`).
    #[serde(rename = "type")]
    pub type_: String,
    /// Billing details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing_details: Option<TreasuryBillingDetails>,
    /// Financial account details if destination is a FinancialAccount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub financial_account: Option<TreasuryFinancialAccountSummary>,
    /// US bank account details if destination is an external bank account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub us_bank_account: Option<TreasuryUsBankAccountDetails>,
}

/// Details when an OutboundPayment is returned.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundPaymentReturnedDetails {
    /// Reason code for the return.
    pub code: String,
    /// The Transaction associated with this return.
    pub transaction: String,
}

/// Status transition timestamps for OutboundPayments.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundPaymentStatusTransitions {
    /// Timestamp when payment was canceled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canceled_at: Option<i64>,
    /// Timestamp when payment failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed_at: Option<i64>,
    /// Timestamp when payment posted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub posted_at: Option<i64>,
    /// Timestamp when payment was returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returned_at: Option<i64>,
}

/// Tracking details for OutboundPayments.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundPaymentTrackingDetails {
    /// Type of tracking details (`ach`, `us_domestic_wire`).
    #[serde(rename = "type")]
    pub type_: String,
    /// ACH tracking details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ach: Option<AchTrackingDetails>,
    /// US Domestic Wire tracking details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub us_domestic_wire: Option<UsDomesticWireTrackingDetails>,
}

/// End user details required for FinCEN / BSA compliance.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndUserDetails {
    /// IP address of the end user initiating the transfer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    /// `true` if the end user is physically present or actively operating the session.
    pub present: bool,
}

/// Initiating payment method details for ReceivedCredit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitiatingPaymentMethodDetails {
    /// Balance type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance: Option<String>,
    /// Method type (`us_bank_account`, etc.).
    #[serde(rename = "type")]
    pub type_: String,
    /// US bank account details if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub us_bank_account: Option<TreasuryUsBankAccountDetails>,
}

// ============================================================================
// 4. Primary Domain Entities
// ============================================================================

/// Treasury OutboundTransfer object (`treasury.outbound_transfer`).
///
/// Represents an outbound funds movement from a FinancialAccount to another
/// FinancialAccount or to an external bank account via ACH or wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundTransfer {
    /// Unique identifier for the object (`obt_...`).
    pub id: String,
    /// String representing the object's type. Always `treasury.outbound_transfer`.
    #[serde(default = "default_outbound_transfer_object")]
    pub object: String,
    /// Amount transferred in minor currency units (e.g., cents).
    pub amount: i64,
    /// Returns `true` if the transfer can be canceled.
    pub cancelable: bool,
    /// Time at which the object was created in UTC seconds.
    pub created: i64,
    /// Three-letter ISO currency code, in lowercase.
    pub currency: String,
    /// An arbitrary string attached to the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The PaymentMethod ID used as the destination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_payment_method: Option<String>,
    /// Details about the destination payment method.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_payment_method_details: Option<OutboundTransferDestinationPaymentMethodDetails>,
    /// The date the transfer is expected to arrive at the destination, in UTC seconds.
    pub expected_arrival_date: i64,
    /// The FinancialAccount ID from which funds were transferred.
    pub financial_account: String,
    /// URL to a hosted receipt for regulatory and statutory compliance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosted_regulatory_receipt_url: Option<String>,
    /// Has the value `true` if live mode or `false` if test mode.
    pub livemode: bool,
    /// Set of key-value pairs attached to the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Details about the return if transfer was returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returned_details: Option<OutboundTransferReturnedDetails>,
    /// Extra information to be displayed on the bank statement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_descriptor: Option<String>,
    /// Current status of the transfer.
    pub status: OutboundTransferStatus,
    /// Timestamp transitions for lifecycle states.
    pub status_transitions: OutboundTransferStatusTransitions,
    /// Tracking details on banking network rails.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracking_details: Option<OutboundTransferTrackingDetails>,
    /// The Transaction ID associated with this transfer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transaction: Option<String>,
}

fn default_outbound_transfer_object() -> String {
    "treasury.outbound_transfer".into()
}

/// Treasury OutboundPayment object (`treasury.outbound_payment`).
///
/// Represents an outbound payment sent to an external counterparty
/// over standard payment rails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundPayment {
    /// Unique identifier for the object (`obp_...`).
    pub id: String,
    /// String representing the object's type. Always `treasury.outbound_payment`.
    #[serde(default = "default_outbound_payment_object")]
    pub object: String,
    /// Amount sent in minor currency units (e.g., cents).
    pub amount: i64,
    /// Returns `true` if the payment can be canceled.
    pub cancelable: bool,
    /// Time at which the object was created in UTC seconds.
    pub created: i64,
    /// Three-letter ISO currency code, in lowercase.
    pub currency: String,
    /// ID of the customer associated with this payment if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,
    /// An arbitrary string attached to the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The PaymentMethod ID used as the destination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_payment_method: Option<String>,
    /// Details about the destination payment method.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_payment_method_details: Option<OutboundPaymentDestinationPaymentMethodDetails>,
    /// End user details required under FinCEN BSA regulations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_user_details: Option<EndUserDetails>,
    /// Expected arrival date in UTC seconds.
    pub expected_arrival_date: i64,
    /// The FinancialAccount ID from which funds were paid.
    pub financial_account: String,
    /// URL to a hosted receipt for statutory compliance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosted_regulatory_receipt_url: Option<String>,
    /// Has the value `true` if live mode or `false` if test mode.
    pub livemode: bool,
    /// Set of key-value pairs attached to the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Details about the return if payment was returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returned_details: Option<OutboundPaymentReturnedDetails>,
    /// Statement descriptor shown on counterparty bank statements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_descriptor: Option<String>,
    /// Current status of the payment.
    pub status: OutboundPaymentStatus,
    /// Timestamp transitions for lifecycle states.
    pub status_transitions: OutboundPaymentStatusTransitions,
    /// Tracking details on payment rails.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracking_details: Option<OutboundPaymentTrackingDetails>,
    /// The Transaction ID associated with this payment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transaction: Option<String>,
}

fn default_outbound_payment_object() -> String {
    "treasury.outbound_payment".into()
}

/// Treasury ReceivedCredit object (`treasury.received_credit`).
///
/// Represents an inbound credit transaction received by a FinancialAccount.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceivedCredit {
    /// Unique identifier for the object (`rc_...`).
    pub id: String,
    /// String representing the object's type. Always `treasury.received_credit`.
    #[serde(default = "default_received_credit_object")]
    pub object: String,
    /// Amount credited in minor currency units (e.g., cents).
    pub amount: i64,
    /// Time at which the object was created in UTC seconds.
    pub created: i64,
    /// Three-letter ISO currency code, in lowercase.
    pub currency: String,
    /// An arbitrary string attached to the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Reason code for failure if credit failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_code: Option<String>,
    /// The FinancialAccount ID that received the credit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub financial_account: Option<String>,
    /// URL to a hosted receipt for statutory compliance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosted_regulatory_receipt_url: Option<String>,
    /// Details about the initiating payment method.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initiating_payment_method_details: Option<InitiatingPaymentMethodDetails>,
    /// Has the value `true` if live mode or `false` if test mode.
    pub livemode: bool,
    /// Network rail on which credit was received (e.g., `ach`, `us_domestic_wire`).
    pub network: String,
    /// Current status of the credit.
    pub status: ReceivedCreditStatus,
    /// The Transaction ID associated with this credit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transaction: Option<String>,
}

fn default_received_credit_object() -> String {
    "treasury.received_credit".into()
}

// ============================================================================
// 5. List Collections
// ============================================================================

/// Paginated list of [`OutboundTransfer`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundTransferList {
    /// String representing the object's type. Always "list".
    pub object: String,
    /// Array of outbound transfers.
    pub data: Vec<OutboundTransfer>,
    /// `true` if more items are available.
    pub has_more: bool,
    /// URL for the list endpoint.
    pub url: String,
}

/// Paginated list of [`OutboundPayment`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundPaymentList {
    /// String representing the object's type. Always "list".
    pub object: String,
    /// Array of outbound payments.
    pub data: Vec<OutboundPayment>,
    /// `true` if more items are available.
    pub has_more: bool,
    /// URL for the list endpoint.
    pub url: String,
}

/// Paginated list of [`ReceivedCredit`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceivedCreditList {
    /// String representing the object's type. Always "list".
    pub object: String,
    /// Array of received credits.
    pub data: Vec<ReceivedCredit>,
    /// `true` if more items are available.
    pub has_more: bool,
    /// URL for the list endpoint.
    pub url: String,
}

// ============================================================================
// 6. Request Parameter Schemas & Builders
// ============================================================================

/// Destination payment method options for OutboundTransfers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundTransferDestinationPaymentMethodOptions {
    /// US bank account payment options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub us_bank_account: Option<OutboundTransferUsBankAccountOptions>,
}

/// Options for US bank account transfers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundTransferUsBankAccountOptions {
    /// Banking network to route over (`ach` or `us_domestic_wire`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
}

/// Parameters for creating an [`OutboundTransfer`] (`POST /v1/treasury/outbound_transfers`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateOutboundTransferParams {
    /// The FinancialAccount ID from which to transfer funds (`fa_...`).
    pub financial_account: String,
    /// Amount to transfer in minor currency units (e.g., cents).
    pub amount: i64,
    /// Three-letter ISO currency code, in lowercase.
    pub currency: String,
    /// The destination PaymentMethod ID (`pm_...` or `ba_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_payment_method: Option<String>,
    /// Payment method options (e.g., specifying wire vs ACH).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_payment_method_options: Option<OutboundTransferDestinationPaymentMethodOptions>,
    /// An arbitrary description attached to the transfer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Statement descriptor shown on recipient bank statements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_descriptor: Option<String>,
    /// Key-value metadata pairs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Idempotency key for safe request retries.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateOutboundTransferParams {
    /// Creates a new `CreateOutboundTransferParams` with required parameters.
    pub fn new(
        financial_account: impl Into<String>,
        amount: i64,
        currency: impl Into<String>,
    ) -> Self {
        Self {
            financial_account: financial_account.into(),
            amount,
            currency: currency.into(),
            destination_payment_method: None,
            destination_payment_method_options: None,
            description: None,
            statement_descriptor: None,
            metadata: None,
            idempotency_key: None,
        }
    }

    /// Sets the destination payment method ID.
    pub fn with_destination_payment_method(mut self, pm: impl Into<String>) -> Self {
        self.destination_payment_method = Some(pm.into());
        self
    }

    /// Sets the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Sets the statement descriptor.
    pub fn with_statement_descriptor(mut self, desc: impl Into<String>) -> Self {
        self.statement_descriptor = Some(desc.into());
        self
    }

    /// Sets the idempotency key.
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates parameter invariants: amount > 0, currency ISO 4217, zero PAN leakage.
    pub fn validate(&self) -> Result<(), TreasuryTransferValidationError> {
        validate_positive_amount(self.amount)?;
        validate_currency_code(&self.currency)?;
        validate_financial_account_id(&self.financial_account)?;
        validate_zero_pan(self.description.as_deref(), "description")?;
        validate_zero_pan(self.statement_descriptor.as_deref(), "statement_descriptor")?;
        validate_zero_pan_in_metadata(self.metadata.as_ref())?;
        Ok(())
    }

    /// Verifies that the source FinancialAccount has sufficient available balance.
    pub fn verify_balance(
        &self,
        available_balance: i64,
    ) -> Result<(), TreasuryTransferValidationError> {
        verify_sufficient_balance(available_balance, self.amount)
    }
}

/// Parameters for canceling an [`OutboundTransfer`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelOutboundTransferParams {
    /// Optional idempotency key for safe request retries.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

/// End user parameters for OutboundPayments.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndUserDetailsParams {
    /// IP address of the end user initiating the transfer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    /// `true` if the end user is physically present or operating the session.
    pub present: bool,
}

/// Parameters for creating an [`OutboundPayment`] (`POST /v1/treasury/outbound_payments`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateOutboundPaymentParams {
    /// The FinancialAccount ID from which funds are paid (`fa_...`).
    pub financial_account: String,
    /// Amount to pay in minor currency units (e.g., cents).
    pub amount: i64,
    /// Three-letter ISO currency code, in lowercase.
    pub currency: String,
    /// Destination PaymentMethod ID (`pm_...` or `ba_...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_payment_method: Option<String>,
    /// Customer ID associated with this payment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,
    /// Arbitrary description attached to the payment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Statement descriptor shown on counterparty bank statements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_descriptor: Option<String>,
    /// End user details required under FinCEN BSA regulations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_user_details: Option<EndUserDetailsParams>,
    /// Key-value metadata pairs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Idempotency key for safe request retries.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateOutboundPaymentParams {
    /// Creates a new `CreateOutboundPaymentParams` with required parameters.
    pub fn new(
        financial_account: impl Into<String>,
        amount: i64,
        currency: impl Into<String>,
    ) -> Self {
        Self {
            financial_account: financial_account.into(),
            amount,
            currency: currency.into(),
            destination_payment_method: None,
            customer: None,
            description: None,
            statement_descriptor: None,
            end_user_details: None,
            metadata: None,
            idempotency_key: None,
        }
    }

    /// Sets the destination payment method ID.
    pub fn with_destination_payment_method(mut self, pm: impl Into<String>) -> Self {
        self.destination_payment_method = Some(pm.into());
        self
    }

    /// Sets the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Sets the statement descriptor.
    pub fn with_statement_descriptor(mut self, desc: impl Into<String>) -> Self {
        self.statement_descriptor = Some(desc.into());
        self
    }

    /// Sets the idempotency key.
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Validates parameter invariants: amount > 0, currency ISO 4217, zero PAN leakage.
    pub fn validate(&self) -> Result<(), TreasuryTransferValidationError> {
        validate_positive_amount(self.amount)?;
        validate_currency_code(&self.currency)?;
        validate_financial_account_id(&self.financial_account)?;
        validate_zero_pan(self.description.as_deref(), "description")?;
        validate_zero_pan(self.statement_descriptor.as_deref(), "statement_descriptor")?;
        validate_zero_pan_in_metadata(self.metadata.as_ref())?;
        Ok(())
    }

    /// Verifies that the source FinancialAccount has sufficient available balance.
    pub fn verify_balance(
        &self,
        available_balance: i64,
    ) -> Result<(), TreasuryTransferValidationError> {
        verify_sufficient_balance(available_balance, self.amount)
    }
}

/// Parameters for canceling an [`OutboundPayment`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelOutboundPaymentParams {
    /// Optional idempotency key for safe request retries.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

/// Parameters for listing [`OutboundTransfer`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListOutboundTransfersParams {
    /// The FinancialAccount ID to list transfers for.
    pub financial_account: String,
    /// Filter transfers by status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<OutboundTransferStatus>,
    /// Limit on the number of objects returned (1 to 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Cursor for pagination after this ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,
    /// Cursor for pagination before this ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListOutboundTransfersParams {
    /// Creates a new `ListOutboundTransfersParams` for the specified FinancialAccount.
    pub fn new(financial_account: impl Into<String>) -> Self {
        Self {
            financial_account: financial_account.into(),
            status: None,
            limit: None,
            starting_after: None,
            ending_before: None,
        }
    }

    /// Filters by transfer status.
    pub fn with_status(mut self, status: OutboundTransferStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Limits the number of results returned.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Parameters for listing [`OutboundPayment`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListOutboundPaymentsParams {
    /// The FinancialAccount ID to list payments for.
    pub financial_account: String,
    /// Filter payments by status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<OutboundPaymentStatus>,
    /// Filter payments by customer ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,
    /// Limit on the number of objects returned (1 to 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Cursor for pagination after this ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,
    /// Cursor for pagination before this ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListOutboundPaymentsParams {
    /// Creates a new `ListOutboundPaymentsParams` for the specified FinancialAccount.
    pub fn new(financial_account: impl Into<String>) -> Self {
        Self {
            financial_account: financial_account.into(),
            status: None,
            customer: None,
            limit: None,
            starting_after: None,
            ending_before: None,
        }
    }

    /// Filters by payment status.
    pub fn with_status(mut self, status: OutboundPaymentStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Limits the number of results returned.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Parameters for listing [`ReceivedCredit`] objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListReceivedCreditsParams {
    /// The FinancialAccount ID to list received credits for.
    pub financial_account: String,
    /// Filter credits by status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ReceivedCreditStatus>,
    /// Filter credits by linked flows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_flows: Option<String>,
    /// Limit on the number of objects returned (1 to 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Cursor for pagination after this ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,
    /// Cursor for pagination before this ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListReceivedCreditsParams {
    /// Creates a new `ListReceivedCreditsParams` for the specified FinancialAccount.
    pub fn new(financial_account: impl Into<String>) -> Self {
        Self {
            financial_account: financial_account.into(),
            status: None,
            linked_flows: None,
            limit: None,
            starting_after: None,
            ending_before: None,
        }
    }

    /// Filters by received credit status.
    pub fn with_status(mut self, status: ReceivedCreditStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Limits the number of results returned.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

// ============================================================================
// 7. Validation Errors & Invariant Verification (Gate G-Law & G2)
// ============================================================================

/// Validation errors for Treasury Outbound Transfers and Payments operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreasuryTransferValidationError {
    /// Amount must be a positive integer in minor currency units (> 0).
    InvalidAmount {
        /// The invalid amount supplied.
        amount: i64,
    },
    /// Currency must be a valid 3-letter lowercase ISO 4217 code.
    InvalidCurrency {
        /// The invalid currency code.
        currency: String,
    },
    /// Financial account ID must be non-empty and start with `fa_`.
    InvalidFinancialAccount {
        /// The invalid financial account ID.
        financial_account: String,
    },
    /// Insufficient available funds in the financial account.
    InsufficientFunds {
        /// Available balance in the account.
        available: i64,
        /// Amount required for the outbound transfer.
        required: i64,
    },
    /// Potential Primary Account Number (PAN) leakage detected in parameter string (PCI-DSS 4.0).
    PanLeakageDetected {
        /// Parameter field where leakage was detected.
        field: &'static str,
    },
    /// Velocity threshold or rate limit exceeded.
    VelocityLimitExceeded {
        /// Detailed reason.
        reason: String,
    },
    /// Generic invalid parameter.
    InvalidParameter {
        /// Field name.
        field: &'static str,
        /// Reason for error.
        reason: String,
    },
}

impl std::fmt::Display for TreasuryTransferValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAmount { amount } => {
                write!(
                    f,
                    "Treasury transfer amount must be a positive integer minor unit (> 0), got: {amount}"
                )
            }
            Self::InvalidCurrency { currency } => {
                write!(
                    f,
                    "Treasury transfer currency must be a valid 3-letter lowercase code, got: '{currency}'"
                )
            }
            Self::InvalidFinancialAccount { financial_account } => {
                write!(
                    f,
                    "Treasury financial_account must be a valid ID starting with 'fa_', got: '{financial_account}'"
                )
            }
            Self::InsufficientFunds {
                available,
                required,
            } => {
                write!(
                    f,
                    "Insufficient Treasury balance: available {available} cents is less than required {required} cents"
                )
            }
            Self::PanLeakageDetected { field } => {
                write!(
                    f,
                    "PCI-DSS 4.0 violation: Potential Primary Account Number (PAN) detected in '{field}'"
                )
            }
            Self::VelocityLimitExceeded { reason } => {
                write!(f, "Treasury velocity limit exceeded: {reason}")
            }
            Self::InvalidParameter { field, reason } => {
                write!(f, "Invalid parameter '{field}': {reason}")
            }
        }
    }
}

impl std::error::Error for TreasuryTransferValidationError {}

/// Verifies that an amount is positive (> 0).
#[inline]
pub fn validate_positive_amount(amount: i64) -> Result<(), TreasuryTransferValidationError> {
    if amount <= 0 {
        return Err(TreasuryTransferValidationError::InvalidAmount { amount });
    }
    Ok(())
}

/// Verifies 3-letter lowercase ISO 4217 currency code.
#[inline]
pub fn validate_currency_code(currency: &str) -> Result<(), TreasuryTransferValidationError> {
    let bytes = currency.as_bytes();
    if bytes.len() != 3 || !bytes.iter().all(u8::is_ascii_lowercase) {
        return Err(TreasuryTransferValidationError::InvalidCurrency {
            currency: currency.to_string(),
        });
    }
    Ok(())
}

/// Verifies FinancialAccount identifier.
#[inline]
pub fn validate_financial_account_id(fa: &str) -> Result<(), TreasuryTransferValidationError> {
    let trimmed = fa.trim();
    if trimmed.is_empty() || !trimmed.starts_with("fa_") {
        return Err(TreasuryTransferValidationError::InvalidFinancialAccount {
            financial_account: fa.to_string(),
        });
    }
    Ok(())
}

/// Verifies that available balance is sufficient to cover requested transfer amount.
#[inline]
pub fn verify_sufficient_balance(
    available: i64,
    requested: i64,
) -> Result<(), TreasuryTransferValidationError> {
    if requested <= 0 {
        return Err(TreasuryTransferValidationError::InvalidAmount { amount: requested });
    }
    if available < requested {
        return Err(TreasuryTransferValidationError::InsufficientFunds {
            available,
            required: requested,
        });
    }
    Ok(())
}

// ============================================================================
// 8. Zero PAN Leakage Verification (PCI-DSS 4.0 & Gate G2)
// ============================================================================

/// Detects whether a string slice contains potential Primary Account Numbers (PANs).
///
/// Scans for digit sequences between 13 and 19 characters and runs the Luhn algorithm.
pub fn detect_pan_leakage(text: &str) -> bool {
    let mut digit_buf: Vec<u8> = Vec::with_capacity(20);

    for &byte in text.as_bytes() {
        if byte.is_ascii_digit() {
            digit_buf.push(byte - b'0');
        } else if byte != b'-' && byte != b' ' {
            if is_potential_pan(&digit_buf) {
                return true;
            }
            digit_buf.clear();
        }
    }

    is_potential_pan(&digit_buf)
}

/// Checks if a buffer of digits has valid PAN length and passes Luhn checksum.
fn is_potential_pan(digits: &[u8]) -> bool {
    let len = digits.len();
    if !(13..=19).contains(&len) {
        return false;
    }
    luhn_verify(digits)
}

/// Performs Luhn mod 10 verification on a slice of decimal digits.
fn luhn_verify(digits: &[u8]) -> bool {
    let mut sum: u32 = 0;
    let mut should_double = false;

    for &d in digits.iter().rev() {
        let mut val = d as u32;
        if should_double {
            val *= 2;
            if val > 9 {
                val -= 9;
            }
        }
        sum += val;
        should_double = !should_double;
    }

    sum.is_multiple_of(10)
}

/// Validates that an optional string field does not contain raw PAN data.
#[inline]
pub fn validate_zero_pan(
    text: Option<&str>,
    field: &'static str,
) -> Result<(), TreasuryTransferValidationError> {
    if text.is_some_and(detect_pan_leakage) {
        return Err(TreasuryTransferValidationError::PanLeakageDetected { field });
    }
    Ok(())
}

/// Validates that metadata key-value pairs do not leak raw PAN data.
pub fn validate_zero_pan_in_metadata(
    metadata: Option<&HashMap<String, String>>,
) -> Result<(), TreasuryTransferValidationError> {
    if let Some(map) = metadata {
        for val in map.values() {
            if detect_pan_leakage(val) {
                return Err(TreasuryTransferValidationError::PanLeakageDetected {
                    field: "metadata",
                });
            }
        }
    }
    Ok(())
}

// ============================================================================
// 9. Bank Secrecy Act / AML Classification & Velocity Tracker (Gate G-Law)
// ============================================================================

/// Regulatory classification for Treasury money movement under BSA / AML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BsaAmlClassification {
    /// Standard money movement below regulatory reporting thresholds.
    Standard,
    /// Money movement $\ge \$3,000.00$ requiring FinCEN Travel Rule data retention.
    TravelRuleReportable,
    /// Money movement $\ge \$10,000.00$ requiring Currency Transaction Report (CTR) filing.
    CtrReportable,
}

/// Classifies a Treasury transfer amount according to FinCEN BSA/AML thresholds.
#[must_use]
pub fn classify_bsa_aml_transfer(amount: i64) -> BsaAmlClassification {
    if amount >= CTR_REPORTING_THRESHOLD_CENTS {
        BsaAmlClassification::CtrReportable
    } else if amount >= FINCEN_TRAVEL_RULE_THRESHOLD_CENTS {
        BsaAmlClassification::TravelRuleReportable
    } else {
        BsaAmlClassification::Standard
    }
}

/// Velocity tracker monitoring Treasury money movement to detect structuring or threshold breaches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreasuryVelocityTracker {
    /// Maximum allowable cumulative volume in minor units per window.
    pub max_volume_cents: i64,
    /// Maximum allowable transfer count per window.
    pub max_transaction_count: u32,
    /// Cumulative volume in the current window.
    pub current_volume_cents: i64,
    /// Cumulative count in the current window.
    pub current_transaction_count: u32,
    /// Start timestamp of the current window in UTC seconds.
    pub window_start_seconds: i64,
    /// Duration of each window in seconds.
    pub window_duration_seconds: i64,
}

impl TreasuryVelocityTracker {
    /// Creates a new velocity tracker with specified caps and window duration.
    pub fn new(
        max_volume_cents: i64,
        max_transaction_count: u32,
        window_duration_seconds: i64,
        now_seconds: i64,
    ) -> Self {
        Self {
            max_volume_cents,
            max_transaction_count,
            current_volume_cents: 0,
            current_transaction_count: 0,
            window_start_seconds: now_seconds,
            window_duration_seconds,
        }
    }

    /// Resets the tracker window if the current timestamp has surpassed the window boundary.
    pub fn refresh_window(&mut self, now_seconds: i64) {
        if now_seconds >= self.window_start_seconds + self.window_duration_seconds {
            self.current_volume_cents = 0;
            self.current_transaction_count = 0;
            self.window_start_seconds = now_seconds;
        }
    }

    /// Evaluates whether an outbound transfer would breach velocity limits.
    pub fn check_velocity(
        &mut self,
        amount: i64,
        now_seconds: i64,
    ) -> Result<(), TreasuryTransferValidationError> {
        self.refresh_window(now_seconds);

        if self.current_transaction_count >= self.max_transaction_count {
            return Err(TreasuryTransferValidationError::VelocityLimitExceeded {
                reason: format!(
                    "Daily transaction count limit of {} reached",
                    self.max_transaction_count
                ),
            });
        }

        if self.current_volume_cents + amount > self.max_volume_cents {
            return Err(TreasuryTransferValidationError::VelocityLimitExceeded {
                reason: format!(
                    "Daily volume limit of {} cents exceeded (attempted: {} cents)",
                    self.max_volume_cents,
                    self.current_volume_cents + amount
                ),
            });
        }

        Ok(())
    }

    /// Records a successful outbound transfer against the velocity window.
    pub fn record_transfer(
        &mut self,
        amount: i64,
        now_seconds: i64,
    ) -> Result<(), TreasuryTransferValidationError> {
        self.check_velocity(amount, now_seconds)?;
        self.current_volume_cents += amount;
        self.current_transaction_count += 1;
        Ok(())
    }
}
