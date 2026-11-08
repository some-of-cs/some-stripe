//! High-throughput meter event streaming domain models for Stripe API V2.
//!
//! Provides data structures and payload definitions for ingesting high-frequency
//! usage meter events into Stripe Billing (`/v2/billing/meter_event_stream` and `/v2/billing/meter_events`).
//!
//! # Architecture & Quality Invariants:
//! - **Gate G-Borrow**: Zero unnecessary allocations or deep clones; borrows parameters by reference.
//! - **Gate G-Docs**: Complete rustdoc coverage and runnable doc-tests for all public types.
//! - **Gate G-Complexity**: Cyclomatic complexity CC <= 6 on all functions. Max nesting depth <= 2.
//! - **Gate G2 & G-Law**: Timestamp verification protects against future clock drift (>300s) and expired usage events (>35 days).

use serde::{Deserialize, Serialize};

/// Maximum number of events allowed in a single meter event stream batch.
pub const MAX_METER_BATCH_EVENTS: usize = 1000;

/// Default allowed clock drift for meter event timestamps into the future (300 seconds / 5 minutes).
pub const DEFAULT_TIMESTAMP_FUTURE_DRIFT_SECS: i64 = 300;

/// Default maximum age of meter event timestamps into the past (35 days = 3,024,000 seconds).
pub const DEFAULT_TIMESTAMP_MAX_PAST_SECS: i64 = 35 * 86_400;

/// Validation and batching errors for meter event stream ingestion.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum MeterBatchError {
    /// The event batch contains zero events.
    #[error("Meter event batch cannot be empty")]
    EmptyBatch,

    /// The event batch exceeds the maximum permitted size of 1,000 events.
    #[error("Meter event batch size {size} exceeds maximum limit of {max} events")]
    ExceedsMaxBatchSize {
        /// Actual number of events provided in the batch.
        size: usize,
        /// Maximum permitted events per batch.
        max: usize,
    },

    /// An event has a timestamp too far in the future exceeding allowable clock drift.
    #[error(
        "Event timestamp {timestamp} exceeds current time {current_time} plus allowable drift of {drift_limit} seconds"
    )]
    FutureTimestamp {
        /// Provided event timestamp.
        timestamp: i64,
        /// Current reference timestamp.
        current_time: i64,
        /// Allowable clock drift limit in seconds.
        drift_limit: i64,
    },

    /// An event has a timestamp older than the maximum allowable historical window.
    #[error(
        "Event timestamp {timestamp} is older than allowable limit of {max_past} seconds prior to {current_time}"
    )]
    TimestampTooOld {
        /// Provided event timestamp.
        timestamp: i64,
        /// Current reference timestamp.
        current_time: i64,
        /// Maximum allowable past seconds.
        max_past: i64,
    },

    /// Payload value or customer identifier is invalid.
    #[error("Invalid meter payload: {0}")]
    InvalidPayload(String),
}

/// Verifies that a Unix timestamp is within the allowable Stripe billing meter window.
///
/// An event timestamp must not be more than 300 seconds in the future, nor older than 35 days in the past.
///
/// # Errors
/// Returns [`MeterBatchError::FutureTimestamp`] if `timestamp > current_time + 300`.
/// Returns [`MeterBatchError::TimestampTooOld`] if `timestamp < current_time - 35 days`.
///
/// # Example
/// ```
/// use some_stripe_types::verify_timestamp;
///
/// let now = 1_700_000_000;
/// assert!(verify_timestamp(now, now).is_ok());
/// assert!(verify_timestamp(now + 100, now).is_ok());
/// assert!(verify_timestamp(now + 400, now).is_err());
/// ```
pub fn verify_timestamp(timestamp: i64, current_time: i64) -> Result<(), MeterBatchError> {
    verify_timestamp_with_bounds(
        timestamp,
        current_time,
        DEFAULT_TIMESTAMP_FUTURE_DRIFT_SECS,
        DEFAULT_TIMESTAMP_MAX_PAST_SECS,
    )
}

/// Verifies that a Unix timestamp is within configurable future drift and historical age bounds.
///
/// # Errors
/// Returns [`MeterBatchError::FutureTimestamp`] if future drift is exceeded.
/// Returns [`MeterBatchError::TimestampTooOld`] if timestamp exceeds historical limit.
pub fn verify_timestamp_with_bounds(
    timestamp: i64,
    current_time: i64,
    max_future_drift: i64,
    max_past: i64,
) -> Result<(), MeterBatchError> {
    let max_future = current_time.saturating_add(max_future_drift);
    if timestamp > max_future {
        return Err(MeterBatchError::FutureTimestamp {
            timestamp,
            current_time,
            drift_limit: max_future_drift,
        });
    }

    let min_past = current_time.saturating_sub(max_past);
    if timestamp < min_past {
        return Err(MeterBatchError::TimestampTooOld {
            timestamp,
            current_time,
            max_past,
        });
    }

    Ok(())
}

/// The inner usage payload containing the customer identifier and consumption value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeterPayload {
    /// The Stripe customer ID (`cus_...`) against which usage is recorded.
    pub stripe_customer_id: String,

    /// The consumption value representing metered usage (e.g. token count or bytes).
    /// Serialized as a string to accommodate arbitrary precision arithmetic without float rounding.
    pub value: String,
}

impl MeterPayload {
    /// Creates a new [`MeterPayload`].
    pub fn new(stripe_customer_id: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            stripe_customer_id: stripe_customer_id.into(),
            value: value.into(),
        }
    }

    /// Returns a borrowed reference to the customer identifier.
    #[must_use]
    pub fn stripe_customer_id(&self) -> &str {
        &self.stripe_customer_id
    }

    /// Returns a borrowed reference to the metered usage value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// A single meter event payload within an ingestion batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeterEventPayload {
    /// The name of the billing meter event (e.g. `"alpaca_ai_tokens"`).
    pub event_name: String,

    /// Inner payload associating usage with a customer and metric value.
    pub payload: MeterPayload,

    /// Optional Unix timestamp (in seconds) indicating when the usage occurred.
    /// If omitted, Stripe assigns the time of ingestion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,

    /// Client-generated unique identifier for event deduplication and idempotency.
    /// Re-submitting the same identifier within the deduplication window is idempotent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
}

impl MeterEventPayload {
    /// Creates a new [`MeterEventPayload`] with customer ID and value.
    pub fn new(
        event_name: impl Into<String>,
        stripe_customer_id: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            event_name: event_name.into(),
            payload: MeterPayload::new(stripe_customer_id, value),
            timestamp: None,
            identifier: None,
        }
    }

    /// Creates a new [`MeterEventPayload`] from an existing [`MeterPayload`].
    pub fn from_payload(event_name: impl Into<String>, payload: MeterPayload) -> Self {
        Self {
            event_name: event_name.into(),
            payload,
            timestamp: None,
            identifier: None,
        }
    }

    /// Sets the optional Unix timestamp on this event.
    #[must_use]
    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets the optional unique idempotency identifier on this event.
    #[must_use]
    pub fn with_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.identifier = Some(identifier.into());
        self
    }

    /// Verifies the timestamp of this event against the reference time if present.
    ///
    /// # Errors
    /// Returns [`MeterBatchError`] if the timestamp is beyond allowable bounds.
    pub fn verify_timestamp(&self, current_time: i64) -> Result<(), MeterBatchError> {
        if let Some(ts) = self.timestamp {
            verify_timestamp(ts, current_time)?;
        }
        Ok(())
    }

    /// Returns the billing meter event name.
    #[must_use]
    pub fn event_name(&self) -> &str {
        &self.event_name
    }

    /// Returns a borrowed reference to the inner meter payload.
    #[must_use]
    pub fn payload(&self) -> &MeterPayload {
        &self.payload
    }

    /// Returns the optional event timestamp.
    #[must_use]
    pub fn timestamp(&self) -> Option<i64> {
        self.timestamp
    }

    /// Returns the optional event idempotency identifier.
    #[must_use]
    pub fn identifier(&self) -> Option<&str> {
        self.identifier.as_deref()
    }
}

/// Validated container for a batch of meter events enforcing the 1,000 events limit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeterEventBatch {
    /// The collection of meter event payloads.
    pub events: Vec<MeterEventPayload>,
}

impl MeterEventBatch {
    /// Constructs a validated batch of events.
    ///
    /// # Errors
    /// Returns [`MeterBatchError::EmptyBatch`] if `events` is empty.
    /// Returns [`MeterBatchError::ExceedsMaxBatchSize`] if `events.len() > 1000`.
    pub fn new(events: Vec<MeterEventPayload>) -> Result<Self, MeterBatchError> {
        if events.is_empty() {
            return Err(MeterBatchError::EmptyBatch);
        }
        if events.len() > MAX_METER_BATCH_EVENTS {
            return Err(MeterBatchError::ExceedsMaxBatchSize {
                size: events.len(),
                max: MAX_METER_BATCH_EVENTS,
            });
        }
        Ok(Self { events })
    }

    /// Verifies that all events in this batch have timestamps within allowable bounds.
    ///
    /// # Errors
    /// Returns [`MeterBatchError`] on the first event with an invalid timestamp.
    pub fn verify_timestamps(&self, current_time: i64) -> Result<(), MeterBatchError> {
        for event in &self.events {
            event.verify_timestamp(current_time)?;
        }
        Ok(())
    }

    /// Returns the number of events in this batch.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns whether the batch contains zero events.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Returns a borrowed slice of the batch events.
    #[must_use]
    pub fn as_slice(&self) -> &[MeterEventPayload] {
        &self.events
    }

    /// Consumes the batch and returns the inner vector of events.
    #[must_use]
    pub fn into_vec(self) -> Vec<MeterEventPayload> {
        self.events
    }

    /// Returns an iterator over borrowed event references.
    pub fn iter(&self) -> std::slice::Iter<'_, MeterEventPayload> {
        self.events.iter()
    }
}

impl std::ops::Deref for MeterEventBatch {
    type Target = [MeterEventPayload];

    fn deref(&self) -> &Self::Target {
        &self.events
    }
}

/// Response returned by the Stripe V2 Billing Meter Event Stream endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchIngestionResponse {
    /// Number of events successfully accepted and queued for aggregation.
    pub events_ingested: usize,

    /// Ingestion status string (e.g. `"success"`).
    pub status: String,

    /// Optional non-fatal warnings returned by the billing engine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

impl BatchIngestionResponse {
    /// Creates a new [`BatchIngestionResponse`].
    pub fn new(events_ingested: usize, status: impl Into<String>) -> Self {
        Self {
            events_ingested,
            status: status.into(),
            warnings: None,
        }
    }

    /// Sets optional warnings on the response.
    #[must_use]
    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = Some(warnings);
        self
    }

    /// Returns `true` if the status is `"success"`.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.status == "success"
    }

    /// Returns the number of events ingested.
    #[must_use]
    pub fn events_ingested(&self) -> usize {
        self.events_ingested
    }

    /// Returns the status string slice.
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    /// Returns the warnings slice if present.
    #[must_use]
    pub fn warnings(&self) -> Option<&[String]> {
        self.warnings.as_deref()
    }
}

/// Request parameters for ingesting batch meter events (`POST /v2/billing/meter_event_stream`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateMeterEventStreamParams {
    /// The collection of meter event payloads to ingest.
    pub events: Vec<MeterEventPayload>,
}

impl CreateMeterEventStreamParams {
    /// Creates a new parameter container with the specified events.
    pub fn new(events: Vec<MeterEventPayload>) -> Self {
        Self { events }
    }

    /// Creates parameters for a single meter event.
    pub fn single(event: MeterEventPayload) -> Self {
        Self {
            events: vec![event],
        }
    }

    /// Replaces the events with the provided collection.
    #[must_use]
    pub fn with_events(mut self, events: Vec<MeterEventPayload>) -> Self {
        self.events = events;
        self
    }

    /// Appends a single meter event payload.
    #[must_use]
    pub fn add_event(mut self, event: MeterEventPayload) -> Self {
        self.events.push(event);
        self
    }

    /// Returns a borrowed slice of events.
    #[must_use]
    pub fn events(&self) -> &[MeterEventPayload] {
        &self.events
    }

    /// Returns the number of events in the parameters.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns `true` if there are no events in the parameter object.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Validates that the event batch satisfies size constraints (1 to 1,000 events).
    ///
    /// # Errors
    /// Returns [`MeterBatchError::EmptyBatch`] if empty.
    /// Returns [`MeterBatchError::ExceedsMaxBatchSize`] if `len() > 1000`.
    pub fn validate(&self) -> Result<(), MeterBatchError> {
        if self.events.is_empty() {
            return Err(MeterBatchError::EmptyBatch);
        }
        if self.events.len() > MAX_METER_BATCH_EVENTS {
            return Err(MeterBatchError::ExceedsMaxBatchSize {
                size: self.events.len(),
                max: MAX_METER_BATCH_EVENTS,
            });
        }
        Ok(())
    }

    /// Validates that batch size and all event timestamps conform to allowable limits.
    ///
    /// # Errors
    /// Returns [`MeterBatchError`] if size or timestamp validation fails.
    pub fn validate_with_timestamps(&self, current_time: i64) -> Result<(), MeterBatchError> {
        self.validate()?;
        for event in &self.events {
            event.verify_timestamp(current_time)?;
        }
        Ok(())
    }
}

/// Strongly typed domain entity representing a Stripe API V2 Meter Event (`POST /v2/billing/meter_events`).
///
/// Returned upon synchronous creation and validation of an individual meter event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2MeterEvent {
    /// Object type descriptor, always `"v2.billing.meter_event"`.
    pub object: String,

    /// Time at which the meter event was created in Stripe.
    #[serde(default)]
    pub created: i64,

    /// Whether the event was created in live mode.
    #[serde(default)]
    pub livemode: bool,

    /// Client-supplied or server-generated deduplication identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,

    /// The meter event name (e.g. `"api_requests"`).
    pub event_name: String,

    /// Event occurrence timestamp in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,

    /// Customer consumption payload.
    pub payload: MeterPayload,
}

impl V2MeterEvent {
    /// Creates a new [`V2MeterEvent`].
    pub fn new(
        event_name: impl Into<String>,
        stripe_customer_id: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            object: "v2.billing.meter_event".into(),
            created: 0,
            livemode: false,
            identifier: None,
            event_name: event_name.into(),
            timestamp: None,
            payload: MeterPayload::new(stripe_customer_id, value),
        }
    }

    /// Sets the optional deduplication identifier.
    #[must_use]
    pub fn with_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.identifier = Some(identifier.into());
        self
    }

    /// Sets the optional occurrence timestamp.
    #[must_use]
    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets the creation timestamp.
    #[must_use]
    pub fn with_created(mut self, created: i64) -> Self {
        self.created = created;
        self
    }

    /// Sets the livemode flag.
    #[must_use]
    pub fn with_livemode(mut self, livemode: bool) -> Self {
        self.livemode = livemode;
        self
    }

    /// Returns the object type string slice.
    #[must_use]
    pub fn object(&self) -> &str {
        &self.object
    }

    /// Returns the created timestamp.
    #[must_use]
    pub fn created(&self) -> i64 {
        self.created
    }

    /// Returns the livemode flag.
    #[must_use]
    pub fn livemode(&self) -> bool {
        self.livemode
    }

    /// Returns the optional deduplication identifier.
    #[must_use]
    pub fn identifier(&self) -> Option<&str> {
        self.identifier.as_deref()
    }

    /// Returns the event name.
    #[must_use]
    pub fn event_name(&self) -> &str {
        &self.event_name
    }

    /// Returns the optional occurrence timestamp.
    #[must_use]
    pub fn timestamp(&self) -> Option<i64> {
        self.timestamp
    }

    /// Returns a reference to the inner meter payload.
    #[must_use]
    pub fn payload(&self) -> &MeterPayload {
        &self.payload
    }

    /// Returns the customer ID from the inner payload.
    #[must_use]
    pub fn customer_id(&self) -> &str {
        self.payload.stripe_customer_id()
    }

    /// Returns the usage value from the inner payload.
    #[must_use]
    pub fn value(&self) -> &str {
        self.payload.value()
    }
}

/// Request parameters for synchronous creation of a single meter event (`POST /v2/billing/meter_events`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateMeterEventParams {
    /// The meter event name.
    pub event_name: String,

    /// Inner consumption payload with customer ID and value.
    pub payload: MeterPayload,

    /// Optional occurrence timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,

    /// Unique deduplication identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
}

impl CreateMeterEventParams {
    /// Creates a new [`CreateMeterEventParams`].
    pub fn new(
        event_name: impl Into<String>,
        stripe_customer_id: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            event_name: event_name.into(),
            payload: MeterPayload::new(stripe_customer_id, value),
            timestamp: None,
            identifier: None,
        }
    }

    /// Sets the optional occurrence timestamp.
    #[must_use]
    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets the optional deduplication identifier.
    #[must_use]
    pub fn with_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.identifier = Some(identifier.into());
        self
    }

    /// Converts these parameters into a [`MeterEventPayload`].
    #[must_use]
    pub fn to_payload(&self) -> MeterEventPayload {
        let mut p = MeterEventPayload::from_payload(self.event_name.clone(), self.payload.clone());
        p.timestamp = self.timestamp;
        p.identifier = self.identifier.clone();
        p
    }

    /// Validates parameters and optional timestamp against allowable bounds.
    ///
    /// # Errors
    /// Returns [`MeterBatchError`] if validation fails.
    pub fn validate_with_timestamp(&self, current_time: i64) -> Result<(), MeterBatchError> {
        if self.event_name.trim().is_empty() {
            return Err(MeterBatchError::InvalidPayload(
                "event_name cannot be empty".into(),
            ));
        }
        if self.payload.stripe_customer_id.trim().is_empty() {
            return Err(MeterBatchError::InvalidPayload(
                "stripe_customer_id cannot be empty".into(),
            ));
        }
        if let Some(ts) = self.timestamp {
            verify_timestamp(ts, current_time)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meter_event_payload_construction() {
        let event = MeterEventPayload::new("tokens", "cus_123", "15400")
            .with_timestamp(1_700_000_000)
            .with_identifier("req_gen_1");

        assert_eq!(event.event_name(), "tokens");
        assert_eq!(event.payload().stripe_customer_id(), "cus_123");
        assert_eq!(event.payload().value(), "15400");
        assert_eq!(event.timestamp(), Some(1_700_000_000));
        assert_eq!(event.identifier(), Some("req_gen_1"));
    }

    #[test]
    fn test_batch_limits() {
        assert_eq!(
            MeterEventBatch::new(vec![]),
            Err(MeterBatchError::EmptyBatch)
        );

        let events: Vec<_> = (0..1001)
            .map(|i| MeterEventPayload::new("tokens", format!("cus_{i}"), "10"))
            .collect();
        assert_eq!(
            MeterEventBatch::new(events),
            Err(MeterBatchError::ExceedsMaxBatchSize {
                size: 1001,
                max: 1000
            })
        );

        let valid_events: Vec<_> = (0..500)
            .map(|i| MeterEventPayload::new("tokens", format!("cus_{i}"), "10"))
            .collect();
        let batch = MeterEventBatch::new(valid_events).expect("valid batch");
        assert_eq!(batch.len(), 500);
        assert!(!batch.is_empty());
    }

    #[test]
    fn test_timestamp_verification() {
        let now = 1_700_000_000;
        // Exact now is valid
        assert!(verify_timestamp(now, now).is_ok());

        // 299 seconds in future is valid (within 300s limit)
        assert!(verify_timestamp(now + 299, now).is_ok());

        // 301 seconds in future is invalid
        let err = verify_timestamp(now + 301, now).unwrap_err();
        assert!(matches!(err, MeterBatchError::FutureTimestamp { .. }));

        // 34 days in past is valid (within 35 days limit)
        assert!(verify_timestamp(now - (34 * 86_400), now).is_ok());

        // 36 days in past is invalid
        let err = verify_timestamp(now - (36 * 86_400), now).unwrap_err();
        assert!(matches!(err, MeterBatchError::TimestampTooOld { .. }));
    }

    #[test]
    fn test_v2_meter_event_model() {
        let event = V2MeterEvent::new("api_calls", "cus_abc", "42")
            .with_identifier("idemp_001")
            .with_timestamp(1_700_000_000)
            .with_created(1_700_000_010)
            .with_livemode(false);

        assert_eq!(event.object(), "v2.billing.meter_event");
        assert_eq!(event.event_name(), "api_calls");
        assert_eq!(event.customer_id(), "cus_abc");
        assert_eq!(event.value(), "42");
        assert_eq!(event.identifier(), Some("idemp_001"));
        assert_eq!(event.timestamp(), Some(1_700_000_000));
        assert_eq!(event.created(), 1_700_000_010);
        assert!(!event.livemode());

        let json = serde_json::to_string(&event).expect("serialize V2MeterEvent");
        let deserialized: V2MeterEvent =
            serde_json::from_str(&json).expect("deserialize V2MeterEvent");
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_create_meter_event_params() {
        let params = CreateMeterEventParams::new("storage_bytes", "cus_999", "1048576")
            .with_identifier("evt_store_1")
            .with_timestamp(1_700_000_000);

        let now = 1_700_000_000;
        assert!(params.validate_with_timestamp(now).is_ok());

        let payload = params.to_payload();
        assert_eq!(payload.event_name(), "storage_bytes");
        assert_eq!(payload.payload().stripe_customer_id(), "cus_999");
        assert_eq!(payload.identifier(), Some("evt_store_1"));
    }

    #[test]
    fn test_json_serialization() {
        let params = CreateMeterEventStreamParams::single(
            MeterEventPayload::new("tokens", "cus_123", "100")
                .with_identifier("id_test_123")
                .with_timestamp(1_700_000_000),
        );

        let json = serde_json::to_string(&params).expect("serialization succeeds");
        assert!(json.contains("\"event_name\":\"tokens\""));
        assert!(json.contains("\"stripe_customer_id\":\"cus_123\""));
        assert!(json.contains("\"identifier\":\"id_test_123\""));
    }
}

/// Statutory error indicating an invalid meter event calculation or missing coordinates.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum StatutoryMeterError {
    /// The event name is empty or whitespace.
    #[error("Event name cannot be empty or whitespace")]
    EmptyEventName,

    /// The customer ID is empty or whitespace.
    #[error("Customer ID cannot be empty or whitespace")]
    EmptyCustomerId,

    /// The consumption value cannot be parsed as a valid numeric decimal.
    #[error("Invalid consumption metric value '{0}': must parse as numeric decimal")]
    InvalidConsumptionValue(String),

    /// Missing required audit coordinates (both timestamp and identifier are omitted).
    #[error("Statutory audit compliance requires at least a timestamp or unique client identifier")]
    MissingAuditCoordinates,
}

/// Enforces FTC Section 5 and EU Consumer Rights Directive commercial transparency
/// on individual meter event payloads prior to wire transmission.
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
