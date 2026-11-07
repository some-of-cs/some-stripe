//! Stripe API V2 Billing Meters domain models and request parameter schemas.
//!
//! Provides strongly typed representations of V2 Billing Meters (`stripe.v2.billing.meters`),
//! which define how usage events are aggregated, mapped to customers, and transformed into
//! billable metric totals for usage-based invoicing.
//!
//! # Architecture & Quality Gates:
//! - **Gate G-Borrow (Zero-Clone Discipline)**: Parameter borrowing by reference (`&CreateBillingMeterParams`),
//!   zero deep `.clone()` calls on entities in request pipelines, and borrowed string slices for accessors.
//! - **Gate G2 (Security & Idempotency)**: Idempotency keys handled safely without logging secrets.
//! - **Gate G-Complexity**: Cyclomatic complexity <= 6 across all methods.
//! - **Gate G-Docs**: Runnable doc-tests verified by `cargo test --doc`.

use serde::{Deserialize, Serialize};

/// Status of a Stripe V2 Billing Meter.
///
/// Indicates whether the meter is actively collecting and aggregating usage events.
///
/// # Example
/// ```
/// use some_stripe_types::MeterStatus;
///
/// let status = MeterStatus::Active;
/// assert_eq!(status.as_str(), "active");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeterStatus {
    /// Meter is actively accepting and aggregating usage events.
    Active,
    /// Meter is deactivated and rejecting new events.
    Inactive,
}

impl MeterStatus {
    /// Returns the string representation of the meter status.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
        }
    }
}

impl std::fmt::Display for MeterStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Aggregation formula defining how raw event values are combined.
///
/// # Example
/// ```
/// use some_stripe_types::AggregationFormula;
///
/// let formula = AggregationFormula::Sum;
/// assert_eq!(formula.as_str(), "sum");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationFormula {
    /// Sums up all event values within the billing period.
    Sum,
    /// Counts the total number of events within the billing period.
    Count,
    /// Takes the most recently reported event value.
    Latest,
}

impl AggregationFormula {
    /// Returns the string representation of the aggregation formula.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Sum => "sum",
            Self::Count => "count",
            Self::Latest => "latest",
        }
    }
}

impl std::fmt::Display for AggregationFormula {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Value settings defining which key inside event payloads carries the metric numerical value.
///
/// # Example
/// ```
/// use some_stripe_types::ValueSettings;
///
/// let settings = ValueSettings::new("tokens");
/// assert_eq!(settings.event_payload_key, "tokens");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueSettings {
    /// The key in the event payload that contains the numerical usage value.
    pub event_payload_key: String,
}

impl ValueSettings {
    /// Creates a new `ValueSettings` with the specified event payload key.
    pub fn new(event_payload_key: impl Into<String>) -> Self {
        Self {
            event_payload_key: event_payload_key.into(),
        }
    }
}

/// Customer mapping settings defining how events are associated with Stripe customers.
///
/// # Example
/// ```
/// use some_stripe_types::CustomerMappingSettings;
///
/// let mapping = CustomerMappingSettings::by_id("stripe_customer_id");
/// assert_eq!(mapping.event_payload_key, "stripe_customer_id");
/// assert_eq!(mapping.mapping_type, "by_id");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerMappingSettings {
    /// The key in the event payload that identifies the customer.
    pub event_payload_key: String,
    /// The method used to associate the event with a customer (e.g. "by_id").
    #[serde(rename = "type")]
    pub mapping_type: String,
}

impl CustomerMappingSettings {
    /// Creates a new `CustomerMappingSettings`.
    pub fn new(event_payload_key: impl Into<String>, mapping_type: impl Into<String>) -> Self {
        Self {
            event_payload_key: event_payload_key.into(),
            mapping_type: mapping_type.into(),
        }
    }

    /// Creates a new `CustomerMappingSettings` using default "by_id" mapping.
    pub fn by_id(event_payload_key: impl Into<String>) -> Self {
        Self {
            event_payload_key: event_payload_key.into(),
            mapping_type: "by_id".to_string(),
        }
    }
}

/// Stripe API V2 Billing Meter object (`billing.meter`).
///
/// Defines a billable metric that tracks, aggregates, and attributes customer usage.
///
/// # Example
/// ```
/// use some_stripe_types::{
///     AggregationFormula, CustomerMappingSettings, MeterStatus, V2BillingMeter, ValueSettings,
/// };
///
/// let meter = V2BillingMeter {
///     id: "mtr_123".to_string(),
///     object: "billing.meter".to_string(),
///     display_name: "LLM Token Usage".to_string(),
///     event_name: "tokens".to_string(),
///     status: MeterStatus::Active,
///     value_settings: ValueSettings::new("value"),
///     customer_mapping: CustomerMappingSettings::by_id("stripe_customer_id"),
///     aggregation_formula: AggregationFormula::Sum,
///     created: 1700000000,
///     updated: 1700000000,
///     livemode: false,
/// };
/// assert_eq!(meter.id, "mtr_123");
/// assert_eq!(meter.status, MeterStatus::Active);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2BillingMeter {
    /// Unique identifier for the meter (e.g. "mtr_...").
    pub id: String,
    /// String representing the object's type ("billing.meter").
    pub object: String,
    /// Human-readable name for the billing meter.
    pub display_name: String,
    /// The event name that this meter tracks.
    pub event_name: String,
    /// Status of the billing meter (`active` or `inactive`).
    pub status: MeterStatus,
    /// Configuration specifying the numerical usage property in the event payload.
    pub value_settings: ValueSettings,
    /// Configuration specifying how events are bound to customer IDs.
    pub customer_mapping: CustomerMappingSettings,
    /// The aggregation formula applied to compute totals (`sum`, `count`, `latest`).
    pub aggregation_formula: AggregationFormula,
    /// Time at which the object was created (Unix timestamp).
    pub created: i64,
    /// Time at which the object was last updated (Unix timestamp).
    pub updated: i64,
    /// Has the value `true` if the object exists in live mode or `false` if in test mode.
    pub livemode: bool,
}

/// Parameters for creating a Stripe API V2 Billing Meter (`POST /v2/billing/meters`).
///
/// # Example
/// ```
/// use some_stripe_types::{
///     AggregationFormula, CreateBillingMeterParams, CustomerMappingSettings, ValueSettings,
/// };
///
/// let params = CreateBillingMeterParams::new(
///     "Inference Tokens",
///     "llm_inference",
///     AggregationFormula::Sum,
///     ValueSettings::new("token_count"),
///     CustomerMappingSettings::by_id("customer_id"),
/// );
/// assert_eq!(params.display_name, "Inference Tokens");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateBillingMeterParams {
    /// Human-readable name for the billing meter.
    pub display_name: String,
    /// The event name that this meter tracks.
    pub event_name: String,
    /// The aggregation formula applied to compute totals.
    pub aggregation_formula: AggregationFormula,
    /// Configuration specifying the numerical usage property in the event payload.
    pub value_settings: ValueSettings,
    /// Configuration specifying how events are bound to customer IDs.
    pub customer_mapping: CustomerMappingSettings,
    /// Optional explicit idempotency key for state-mutating requests.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl CreateBillingMeterParams {
    /// Creates a new `CreateBillingMeterParams` with required fields.
    pub fn new(
        display_name: impl Into<String>,
        event_name: impl Into<String>,
        aggregation_formula: AggregationFormula,
        value_settings: ValueSettings,
        customer_mapping: CustomerMappingSettings,
    ) -> Self {
        Self {
            display_name: display_name.into(),
            event_name: event_name.into(),
            aggregation_formula,
            value_settings,
            customer_mapping,
            idempotency_key: None,
        }
    }

    /// Sets an explicit idempotency key for the request.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for updating a Stripe API V2 Billing Meter (`POST /v2/billing/meters/{id}`).
///
/// # Example
/// ```
/// use some_stripe_types::UpdateBillingMeterParams;
///
/// let params = UpdateBillingMeterParams::new().display_name("Updated Token Meter");
/// assert_eq!(params.display_name.as_deref(), Some("Updated Token Meter"));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateBillingMeterParams {
    /// New human-readable display name for the billing meter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Optional explicit idempotency key for state-mutating requests.
    #[serde(skip)]
    pub idempotency_key: Option<String>,
}

impl UpdateBillingMeterParams {
    /// Creates a new empty `UpdateBillingMeterParams`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the new display name.
    pub fn display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    /// Sets an explicit idempotency key for the request.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// Parameters for listing Stripe API V2 Billing Meters (`GET /v2/billing/meters`).
///
/// # Example
/// ```
/// use some_stripe_types::{ListBillingMetersParams, MeterStatus};
///
/// let params = ListBillingMetersParams::new()
///     .limit(20)
///     .status(MeterStatus::Active);
/// assert_eq!(params.limit, Some(20));
/// assert_eq!(params.status, Some(MeterStatus::Active));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListBillingMetersParams {
    /// Maximum number of meters to return (between 1 and 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Filter meters by active/inactive status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<MeterStatus>,
    /// Cursor for pagination: fetch items created after this meter ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,
    /// Cursor for pagination: fetch items created before this meter ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

impl ListBillingMetersParams {
    /// Creates a new empty `ListBillingMetersParams`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum number of items to return.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the meter status filter.
    pub fn status(mut self, status: MeterStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Sets the `starting_after` pagination cursor.
    pub fn starting_after(mut self, cursor: impl Into<String>) -> Self {
        self.starting_after = Some(cursor.into());
        self
    }

    /// Sets the `ending_before` pagination cursor.
    pub fn ending_before(mut self, cursor: impl Into<String>) -> Self {
        self.ending_before = Some(cursor.into());
        self
    }
}

/// Paginated list response wrapper for Stripe API V2 Billing Meters.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2BillingMeterList {
    /// List of billing meter objects.
    pub data: Vec<V2BillingMeter>,
    /// URL to retrieve the next page of results, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page_url: Option<String>,
    /// URL to retrieve the previous page of results, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_page_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meter_status_serialization() {
        assert_eq!(
            serde_json::to_string(&MeterStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&MeterStatus::Inactive).unwrap(),
            "\"inactive\""
        );

        let active: MeterStatus = serde_json::from_str("\"active\"").unwrap();
        assert_eq!(active, MeterStatus::Active);
        let inactive: MeterStatus = serde_json::from_str("\"inactive\"").unwrap();
        assert_eq!(inactive, MeterStatus::Inactive);
    }

    #[test]
    fn test_aggregation_formula_serialization() {
        assert_eq!(
            serde_json::to_string(&AggregationFormula::Sum).unwrap(),
            "\"sum\""
        );
        assert_eq!(
            serde_json::to_string(&AggregationFormula::Count).unwrap(),
            "\"count\""
        );
        assert_eq!(
            serde_json::to_string(&AggregationFormula::Latest).unwrap(),
            "\"latest\""
        );

        let sum: AggregationFormula = serde_json::from_str("\"sum\"").unwrap();
        assert_eq!(sum, AggregationFormula::Sum);
    }

    #[test]
    fn test_customer_mapping_serialization() {
        let mapping = CustomerMappingSettings::by_id("stripe_customer_id");
        let json = serde_json::to_value(&mapping).unwrap();
        assert_eq!(json["event_payload_key"], "stripe_customer_id");
        assert_eq!(json["type"], "by_id");

        let deserialized: CustomerMappingSettings = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, mapping);
    }

    #[test]
    fn test_create_params_serialization() {
        let params = CreateBillingMeterParams::new(
            "AI Inference",
            "tokens",
            AggregationFormula::Sum,
            ValueSettings::new("token_count"),
            CustomerMappingSettings::by_id("stripe_customer_id"),
        )
        .idempotency_key("idem_123");

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["display_name"], "AI Inference");
        assert_eq!(json["event_name"], "tokens");
        assert_eq!(json["aggregation_formula"], "sum");
        assert_eq!(json["value_settings"]["event_payload_key"], "token_count");
        assert_eq!(json["customer_mapping"]["type"], "by_id");
        // Idempotency key should be skipped from json payload
        assert!(json.get("idempotency_key").is_none());
    }

    #[test]
    fn test_v2_billing_meter_roundtrip() {
        let json_str = r#"{
            "id": "mtr_test_123",
            "object": "billing.meter",
            "display_name": "API Requests",
            "event_name": "api_call",
            "status": "active",
            "value_settings": {
                "event_payload_key": "calls"
            },
            "customer_mapping": {
                "event_payload_key": "customer_id",
                "type": "by_id"
            },
            "aggregation_formula": "count",
            "created": 1710000000,
            "updated": 1710000000,
            "livemode": false
        }"#;

        let meter: V2BillingMeter = serde_json::from_str(json_str).unwrap();
        assert_eq!(meter.id, "mtr_test_123");
        assert_eq!(meter.object, "billing.meter");
        assert_eq!(meter.display_name, "API Requests");
        assert_eq!(meter.status, MeterStatus::Active);
        assert_eq!(meter.aggregation_formula, AggregationFormula::Count);
        assert_eq!(meter.customer_mapping.mapping_type, "by_id");
        assert_eq!(meter.value_settings.event_payload_key, "calls");
        assert!(!meter.livemode);
    }

    #[test]
    fn test_list_params_builder() {
        let params = ListBillingMetersParams::new()
            .limit(50)
            .status(MeterStatus::Inactive)
            .starting_after("mtr_123")
            .ending_before("mtr_456");

        assert_eq!(params.limit, Some(50));
        assert_eq!(params.status, Some(MeterStatus::Inactive));
        assert_eq!(params.starting_after.as_deref(), Some("mtr_123"));
        assert_eq!(params.ending_before.as_deref(), Some("mtr_456"));
    }
}
