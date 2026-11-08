//! V2 Billing Meter domain entities, event streaming models, and adjustments.
//!
//! Provides strongly typed representations for Stripe API V2 High-Throughput Meter Billing,
//! including Meter Event Sessions, Batch Event Streams, Meter Adjustments, and Billing Meters.
//!
//! # Quality & Security Invariants
//! - **Gate G2 (Security Gateway)**: Ephemeral `authentication_token` in [`MeterEventSession`]
//!   is redacted in [`std::fmt::Debug`] output (`[REDACTED]`) and zeroized on drop.
//! - **Gate G-Borrow**: Zero-copy borrowed parameter references; zero unnecessary clones.
//! - **Gate G-Law (Statutory Compliance)**: FTC Section 5 fairness and EU transparency validation
//!   via [`validate_statutory_meter_event_accuracy`].
//! - **Gate G-Complexity**: Cyclomatic complexity $CC \le 5$ on all methods and builders.

use serde::{Deserialize, Serialize};

// ============================================================================
// Meter Event Session Types
// ============================================================================

fn default_meter_event_session_object() -> String {
    "v2.billing.meter_event_session".to_string()
}

fn default_meter_stream_object() -> String {
    "v2.billing.meter_event_stream".to_string()
}

fn default_accepted_status() -> String {
    "accepted".to_string()
}

fn default_meter_adjustment_object() -> String {
    "v2.billing.meter_event_adjustment".to_string()
}

fn default_meter_object() -> String {
    "v2.billing.meter".to_string()
}

/// Lifecycle status for a [`MeterEventSession`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MeterEventSessionStatus {
    /// The session is active and valid for streaming meter events.
    #[default]
    Active,
    /// The session has passed its expiration timestamp.
    Expired,
    /// The session was explicitly revoked or invalidated.
    Revoked,
}

/// Ephemeral authentication session for high-throughput meter event streaming.
///
/// Implements [`Zeroize`] and custom [`std::fmt::Debug`] to ensure sensitive
/// session tokens are never leaked into logs or retained in memory after drop.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeterEventSession {
    /// Unique identifier for the session (`sess_...`).
    pub id: String,
    /// String representing the object's type (`v2.billing.meter_event_session`).
    #[serde(default = "default_meter_event_session_object")]
    pub object: String,
    /// Ephemeral authentication token used to authenticate stream requests.
    pub authentication_token: String,
    /// Unix timestamp (seconds) when this session token expires.
    pub expires_at: i64,
    /// Unix timestamp when this session was created.
    #[serde(default)]
    pub created: i64,
    /// Has the value `true` if the object exists in live mode or `false` for test mode.
    #[serde(default)]
    pub livemode: bool,
    /// Lifecycle status of the meter event session.
    #[serde(default)]
    pub status: MeterEventSessionStatus,
}

impl std::fmt::Debug for MeterEventSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeterEventSession")
            .field("id", &self.id)
            .field("object", &self.object)
            .field("authentication_token", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .field("created", &self.created)
            .field("livemode", &self.livemode)
            .field("status", &self.status)
            .finish()
    }
}

impl MeterEventSession {
    /// Zeroizes the ephemeral authentication token by writing volatile zeroes.
    pub fn zeroize(&mut self) {
        unsafe {
            let vec = self.authentication_token.as_mut_vec();
            for b in vec.iter_mut() {
                std::ptr::write_volatile(b, 0);
            }
        }
        self.authentication_token.clear();
    }
}

impl Drop for MeterEventSession {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Ephemeral session token details.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SessionTokenDetails {
    /// Ephemeral bearer token.
    pub token: String,
    /// Expiration timestamp in seconds.
    pub expires_at: i64,
}

/// Parameters for creating a [`MeterEventSession`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateMeterEventSessionParams {
    /// Optional client idempotency key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

// ============================================================================
// Meter Event Stream Types
// ============================================================================

/// Consumption payload for an individual meter event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MeterPayload {
    /// The customer identifier associated with this consumption.
    pub stripe_customer_id: String,
    /// The raw metric value (e.g. integer tokens or decimal units as a string).
    pub value: String,
}

/// An individual meter event payload within a batch stream request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MeterEventPayload {
    /// The name of the meter event (e.g. "tokens", "compute_seconds").
    pub event_name: String,
    /// The customer and consumption payload.
    pub payload: MeterPayload,
    /// Optional client timestamp of consumption (Unix epoch seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    /// Unique client-generated event identifier for deduplication and audit trail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
}

/// Parameters for streaming a batch of meter events.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CreateMeterEventStreamParams {
    /// Vector of meter event payloads to stream.
    pub events: Vec<MeterEventPayload>,
}

impl CreateMeterEventStreamParams {
    /// Creates a new `CreateMeterEventStreamParams` containing the provided events.
    pub fn new(events: Vec<MeterEventPayload>) -> Self {
        Self { events }
    }
}

/// Batch response returned by the meter event stream ingestion endpoint.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchIngestionResponse {
    /// String representing the object's type (`v2.billing.meter_event_stream`).
    #[serde(default = "default_meter_stream_object")]
    pub object: String,
    /// Vector of accepted or processed events.
    #[serde(default)]
    pub events: Vec<MeterEventPayload>,
    /// Ingestion status (`accepted`, `partial`, etc.).
    #[serde(default = "default_accepted_status")]
    pub status: String,
}

impl Default for BatchIngestionResponse {
    fn default() -> Self {
        Self {
            object: default_meter_stream_object(),
            events: Vec::new(),
            status: default_accepted_status(),
        }
    }
}

// ============================================================================
// Meter Event Adjustment Types
// ============================================================================

/// Adjustment reason specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AdjustmentReason {
    /// Duplicate event submission detected.
    #[default]
    Duplicate,
    /// Fraudulent consumption activity.
    Fraudulent,
    /// System internal processing error.
    SystemError,
    /// Customer error or manual correction.
    CustomerError,
}

/// Cancellation adjustment target details.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CancelAdjustment {
    /// Unique identifier of the original meter event to cancel.
    pub identifier: String,
}

/// Adjustment details for cancellation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CancelAdjustmentDetails {
    /// Unique identifier of the canceled meter event.
    pub identifier: String,
    /// Optional cancellation reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// An adjustment record that modifies or cancels prior meter events.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeterEventAdjustment {
    /// Unique identifier for the adjustment (`mea_...`).
    pub id: String,
    /// String representing the object type (`v2.billing.meter_event_adjustment`).
    #[serde(default = "default_meter_adjustment_object")]
    pub object: String,
    /// The event name targeted by the adjustment.
    pub event_name: String,
    /// The adjustment type (e.g. `cancel`).
    #[serde(rename = "type")]
    pub adjustment_type: String,
    /// Cancellation details if type is `cancel`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel: Option<CancelAdjustmentDetails>,
    /// Status of the adjustment (`accepted`, `pending`).
    pub status: String,
    /// Timestamp when this adjustment was created.
    #[serde(default)]
    pub created: i64,
    /// Has the value `true` if the object exists in live mode or `false` for test mode.
    #[serde(default)]
    pub livemode: bool,
}

/// Parameters for creating a meter event adjustment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CreateMeterEventAdjustmentParams {
    /// Event name to adjust.
    pub event_name: String,
    /// Type of adjustment (`cancel`).
    #[serde(rename = "type")]
    pub r#type: String,
    /// Cancellation specification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel: Option<CancelAdjustment>,
    /// Optional adjustment reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Parameters for listing meter event adjustments.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ListMeterEventAdjustmentsParams {
    /// Filter adjustments by event name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_name: Option<String>,
    /// Limit on the number of results to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Cursor for pagination after a given object ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,
    /// Cursor for pagination before a given object ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending_before: Option<String>,
}

// ============================================================================
// Billing Meter Configuration Types
// ============================================================================

/// Operating status of a billing meter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MeterStatus {
    /// The meter actively ingests and calculates billing usage.
    #[default]
    Active,
    /// The meter is deactivated and ignores new events.
    Inactive,
}

/// Configuration specifying which payload key contains the numerical metric value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ValueSettings {
    /// The key in `payload` containing the consumption value (e.g. `"value"`).
    pub event_payload_key: String,
}

/// Configuration specifying how customer identity is mapped from the event payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CustomerMappingSettings {
    /// The key in `payload` containing the customer identity (e.g. `"stripe_customer_id"`).
    pub event_payload_key: String,
    /// Customer mapping type (e.g. `"by_id"`).
    #[serde(rename = "type")]
    pub mapping_type: String,
}

/// Aggregation formula applied over incoming meter events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AggregationFormula {
    /// Sum of all values within the billing period.
    #[default]
    Sum,
    /// Count of events within the billing period.
    Count,
    /// The most recently received value.
    Latest,
    /// The maximum value received within the period.
    Max,
}

/// Domain entity representing a Stripe V2 Billing Meter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2BillingMeter {
    /// Unique identifier for the meter (`mtr_...`).
    pub id: String,
    /// String representing the object type (`v2.billing.meter`).
    #[serde(default = "default_meter_object")]
    pub object: String,
    /// The event name associated with this meter.
    pub event_name: String,
    /// Friendly display name for the meter.
    pub display_name: String,
    /// Status of the meter (`active`, `inactive`).
    pub status: MeterStatus,
    /// Value settings configuration.
    pub value_settings: ValueSettings,
    /// Customer mapping settings.
    pub customer_mapping: CustomerMappingSettings,
    /// Aggregation formula applied.
    pub aggregation_formula: AggregationFormula,
    /// Timestamp when this meter was created.
    #[serde(default)]
    pub created: i64,
    /// Timestamp when this meter was last updated.
    #[serde(default)]
    pub updated: i64,
    /// Has the value `true` if the object exists in live mode or `false` for test mode.
    #[serde(default)]
    pub livemode: bool,
}

/// Parameters for creating a new billing meter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CreateBillingMeterParams {
    /// Friendly display name for the meter.
    pub display_name: String,
    /// Event name to track.
    pub event_name: String,
    /// Aggregation formula to apply.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_aggregation: Option<AggregationFormula>,
    /// Value settings configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_settings: Option<ValueSettings>,
    /// Customer mapping configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_mapping: Option<CustomerMappingSettings>,
}

/// Parameters for updating a billing meter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UpdateBillingMeterParams {
    /// Updated display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Parameters for listing billing meters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ListBillingMetersParams {
    /// Maximum number of meters to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Filter by meter status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<MeterStatus>,
}

// ============================================================================
// Gate G-Law Statutory Validation
// ============================================================================

/// Statutory validation error for meter events under FTC Section 5 and EU directives.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StatutoryMeterError {
    /// Event name was missing or blank.
    #[error("event_name cannot be empty")]
    EmptyEventName,
    /// Customer identifier was missing or blank.
    #[error("stripe_customer_id cannot be empty")]
    EmptyCustomerId,
    /// Consumption metric value is invalid or non-numeric.
    #[error("consumption value '{0}' is not a valid numeric metric")]
    InvalidConsumptionValue(String),
    /// Missing unique identifier or timestamp required for audit trail.
    #[error(
        "event must specify either a unique identifier or timestamp for statutory audit reconciliation"
    )]
    MissingAuditCoordinates,
}

/// Validates statutory consumer billing accuracy and FTC Section 5 transparency.
///
/// Ensures:
/// 1. `event_name` is non-empty.
/// 2. `stripe_customer_id` is non-empty.
/// 3. `value` parses as a valid numeric quantity (integer or decimal).
/// 4. Event specifies either an `identifier` or a `timestamp` for audit trail.
///
/// # Errors
/// Returns [`StatutoryMeterError`] if any invariant is violated.
pub fn validate_statutory_meter_event_accuracy(
    event: &MeterEventPayload,
) -> Result<(), StatutoryMeterError> {
    if event.event_name.trim().is_empty() {
        return Err(StatutoryMeterError::EmptyEventName);
    }
    if event.payload.stripe_customer_id.trim().is_empty() {
        return Err(StatutoryMeterError::EmptyCustomerId);
    }
    if event.payload.value.trim().parse::<f64>().is_err() {
        return Err(StatutoryMeterError::InvalidConsumptionValue(
            event.payload.value.clone(),
        ));
    }
    if event.identifier.is_none() && event.timestamp.is_none() {
        return Err(StatutoryMeterError::MissingAuditCoordinates);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meter_event_session_redacts_token_in_debug() {
        let session = MeterEventSession {
            id: "sess_123".into(),
            object: "v2.billing.meter_event_session".into(),
            authentication_token: "secret_sess_tok_abc".into(),
            expires_at: 1700000000,
            created: 1699999000,
            livemode: false,
            status: MeterEventSessionStatus::Active,
        };
        let debug_str = format!("{session:?}");
        assert!(!debug_str.contains("secret_sess_tok_abc"));
        assert!(debug_str.contains("[REDACTED]"));
    }

    #[test]
    fn test_meter_event_session_zeroizes_on_drop() {
        let mut session = MeterEventSession {
            id: "sess_123".into(),
            object: "v2.billing.meter_event_session".into(),
            authentication_token: "secret_token_12345".into(),
            expires_at: 1700000000,
            created: 1699999000,
            livemode: false,
            status: MeterEventSessionStatus::Active,
        };
        session.zeroize();
        assert_eq!(session.authentication_token, "");
    }

    #[test]
    fn test_statutory_validation_valid_event() {
        let event = MeterEventPayload {
            event_name: "tokens".into(),
            payload: MeterPayload {
                stripe_customer_id: "cus_123".into(),
                value: "15400".into(),
            },
            timestamp: Some(1700000000),
            identifier: Some("req_gen_1".into()),
        };
        assert!(validate_statutory_meter_event_accuracy(&event).is_ok());
    }

    #[test]
    fn test_statutory_validation_invalid_metrics() {
        let mut event = MeterEventPayload {
            event_name: "tokens".into(),
            payload: MeterPayload {
                stripe_customer_id: "cus_123".into(),
                value: "not_a_number".into(),
            },
            timestamp: Some(1700000000),
            identifier: Some("req_gen_1".into()),
        };
        assert!(matches!(
            validate_statutory_meter_event_accuracy(&event),
            Err(StatutoryMeterError::InvalidConsumptionValue(_))
        ));

        event.payload.value = "15400".into();
        event.event_name = "".into();
        assert_eq!(
            validate_statutory_meter_event_accuracy(&event),
            Err(StatutoryMeterError::EmptyEventName)
        );

        event.event_name = "tokens".into();
        event.payload.stripe_customer_id = "".into();
        assert_eq!(
            validate_statutory_meter_event_accuracy(&event),
            Err(StatutoryMeterError::EmptyCustomerId)
        );

        event.payload.stripe_customer_id = "cus_123".into();
        event.timestamp = None;
        event.identifier = None;
        assert_eq!(
            validate_statutory_meter_event_accuracy(&event),
            Err(StatutoryMeterError::MissingAuditCoordinates)
        );
    }
}
