//! High-performance zero-copy serializer for high-throughput meter event streaming.
//!
//! Provides streaming, low-allocation JSON and NDJSON serialization capable of processing
//! over 10,000 meter events per second with sub-millisecond serialization overhead.
//!
//! # Architecture & Quality Invariants:
//! - **Gate G-Borrow**: Zero-copy byte buffer streaming and borrowed slice references (`&[MeterEventPayload]`).
//! - **Gate G-Docs**: Runnable doc-tests and thorough rustdoc documentation on all public components.
//! - **Gate G-Complexity**: Cyclomatic complexity CC <= 6 on all functions with max nesting depth <= 2.
//! - **Gate G2 & G-Law**: Cryptographic drift and timestamp bounds checking.

use crate::error::StripeError;
use some_stripe_types::{MAX_METER_BATCH_EVENTS, MeterEventPayload, verify_timestamp};
use std::io::Write;

/// Estimated byte size per serialized [`MeterEventPayload`] used to pre-allocate buffers.
pub const ESTIMATED_EVENT_BYTES: usize = 160;

/// High-performance batch serializer for Stripe V2 meter events.
///
/// Designed to minimize allocations and support high-throughput event ingestion (>10,000 events/sec).
pub struct MeterBatchSerializer;

impl MeterBatchSerializer {
    /// Validates that the slice of events satisfies Stripe V2 batch size constraints (1 to 1,000 events).
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if `events` is empty or exceeds 1,000 items.
    ///
    /// # Example
    /// ```
    /// use some_stripe_core::patterns::meter_serializer::MeterBatchSerializer;
    /// use some_stripe_types::MeterEventPayload;
    ///
    /// let events = vec![MeterEventPayload::new("tokens", "cus_123", "10")];
    /// assert!(MeterBatchSerializer::validate_batch_size(&events).is_ok());
    ///
    /// let empty: Vec<MeterEventPayload> = vec![];
    /// assert!(MeterBatchSerializer::validate_batch_size(&empty).is_err());
    /// ```
    pub fn validate_batch_size(events: &[MeterEventPayload]) -> Result<(), StripeError> {
        if events.is_empty() {
            return Err(StripeError::InvalidRequest(
                "Meter event batch cannot be empty".into(),
            ));
        }
        if events.len() > MAX_METER_BATCH_EVENTS {
            return Err(StripeError::InvalidRequest(format!(
                "Meter event batch size ({}) exceeds maximum limit of {} events",
                events.len(),
                MAX_METER_BATCH_EVENTS
            )));
        }
        Ok(())
    }

    /// Validates batch size constraints and verifies all event timestamps against allowable drift and retention limits.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if size constraints or timestamp verification fail.
    ///
    /// # Example
    /// ```
    /// use some_stripe_core::patterns::meter_serializer::MeterBatchSerializer;
    /// use some_stripe_types::MeterEventPayload;
    ///
    /// let now = 1_700_000_000;
    /// let events = vec![MeterEventPayload::new("tokens", "cus_123", "10").with_timestamp(now)];
    /// assert!(MeterBatchSerializer::validate_batch_with_timestamps(&events, now).is_ok());
    /// ```
    pub fn validate_batch_with_timestamps(
        events: &[MeterEventPayload],
        current_time: i64,
    ) -> Result<(), StripeError> {
        Self::validate_batch_size(events)?;
        for event in events {
            if let Some(ts) = event.timestamp() {
                verify_timestamp(ts, current_time)
                    .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;
            }
        }
        Ok(())
    }

    /// Serializes a single meter event payload into a JSON string.
    ///
    /// # Errors
    /// Returns [`StripeError::Json`] if serialization fails.
    ///
    /// # Example
    /// ```
    /// use some_stripe_core::patterns::meter_serializer::MeterBatchSerializer;
    /// use some_stripe_types::MeterEventPayload;
    ///
    /// let event = MeterEventPayload::new("tokens", "cus_123", "50");
    /// let json = MeterBatchSerializer::serialize_single_event_json(&event).unwrap();
    /// assert!(json.contains("\"tokens\""));
    /// ```
    pub fn serialize_single_event_json(event: &MeterEventPayload) -> Result<String, StripeError> {
        serde_json::to_string(event).map_err(StripeError::from)
    }

    /// Serializes a batch of meter events into a standard Stripe V2 JSON request body (`{"events": [...]}`).
    ///
    /// Pre-allocates buffer capacity to avoid multiple reallocation passes during string building.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if batch size validation fails.
    /// Returns [`StripeError::Json`] if JSON serialization fails.
    ///
    /// # Example
    /// ```
    /// use some_stripe_core::patterns::meter_serializer::MeterBatchSerializer;
    /// use some_stripe_types::MeterEventPayload;
    ///
    /// let events = vec![MeterEventPayload::new("tokens", "cus_123", "10")];
    /// let json = MeterBatchSerializer::serialize_batch_json(&events).unwrap();
    /// assert!(json.starts_with("{\"events\":["));
    /// ```
    pub fn serialize_batch_json(events: &[MeterEventPayload]) -> Result<String, StripeError> {
        Self::validate_batch_size(events)?;
        let capacity = events
            .len()
            .saturating_mul(ESTIMATED_EVENT_BYTES)
            .saturating_add(32);
        let mut buffer = Vec::with_capacity(capacity);
        Self::serialize_batch_into_writer(events, &mut buffer)?;
        String::from_utf8(buffer).map_err(|e| StripeError::InvalidRequest(e.to_string()))
    }

    /// Streams serialized batch events directly into a [`Write`] sink (socket, file, or buffer)
    /// without intermediate string copies.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if batch size validation fails.
    /// Returns [`StripeError::Json`] or [`StripeError::ConnectionError`] if writing fails.
    pub fn serialize_batch_into_writer<W: Write>(
        events: &[MeterEventPayload],
        writer: &mut W,
    ) -> Result<(), StripeError> {
        Self::validate_batch_size(events)?;

        writer
            .write_all(b"{\"events\":[")
            .map_err(|e| StripeError::ConnectionError(e.to_string()))?;

        for (idx, event) in events.iter().enumerate() {
            if idx > 0 {
                writer
                    .write_all(b",")
                    .map_err(|e| StripeError::ConnectionError(e.to_string()))?;
            }
            serde_json::to_writer(&mut *writer, event)?;
        }

        writer
            .write_all(b"]}")
            .map_err(|e| StripeError::ConnectionError(e.to_string()))?;

        Ok(())
    }

    /// Serializes a slice of meter events into Newline-Delimited JSON (NDJSON) format.
    ///
    /// Each event is serialized on a single line separated by `\n`.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if batch size validation fails.
    /// Returns [`StripeError::Json`] if serialization fails.
    ///
    /// # Example
    /// ```
    /// use some_stripe_core::patterns::meter_serializer::MeterBatchSerializer;
    /// use some_stripe_types::MeterEventPayload;
    ///
    /// let events = vec![
    ///     MeterEventPayload::new("tokens", "cus_1", "10"),
    ///     MeterEventPayload::new("tokens", "cus_2", "20"),
    /// ];
    /// let ndjson = MeterBatchSerializer::serialize_ndjson(&events).unwrap();
    /// assert_eq!(ndjson.lines().count(), 2);
    /// ```
    pub fn serialize_ndjson(events: &[MeterEventPayload]) -> Result<String, StripeError> {
        Self::validate_batch_size(events)?;
        let capacity = events.len().saturating_mul(ESTIMATED_EVENT_BYTES);
        let mut buffer = Vec::with_capacity(capacity);
        Self::serialize_ndjson_into_writer(events, &mut buffer)?;
        String::from_utf8(buffer).map_err(|e| StripeError::InvalidRequest(e.to_string()))
    }

    /// Streams serialized NDJSON events directly into a [`Write`] sink.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if batch size validation fails.
    /// Returns [`StripeError::Json`] or [`StripeError::ConnectionError`] if writing fails.
    pub fn serialize_ndjson_into_writer<W: Write>(
        events: &[MeterEventPayload],
        writer: &mut W,
    ) -> Result<(), StripeError> {
        Self::validate_batch_size(events)?;

        for event in events {
            serde_json::to_writer(&mut *writer, event)?;
            writer
                .write_all(b"\n")
                .map_err(|e| StripeError::ConnectionError(e.to_string()))?;
        }

        Ok(())
    }

    /// Splits a large slice of events into chunks of at most `chunk_size` elements
    /// without cloning.
    ///
    /// # Panics
    /// Panics if `chunk_size == 0`.
    ///
    /// # Example
    /// ```
    /// use some_stripe_core::patterns::meter_serializer::MeterBatchSerializer;
    /// use some_stripe_types::MeterEventPayload;
    ///
    /// let events: Vec<_> = (0..250)
    ///     .map(|i| MeterEventPayload::new("tokens", format!("cus_{i}"), "1"))
    ///     .collect();
    ///
    /// let chunks: Vec<_> = MeterBatchSerializer::chunk_events(&events, 100).collect();
    /// assert_eq!(chunks.len(), 3);
    /// assert_eq!(chunks[0].len(), 100);
    /// assert_eq!(chunks[1].len(), 100);
    /// assert_eq!(chunks[2].len(), 50);
    /// ```
    pub fn chunk_events(
        events: &[MeterEventPayload],
        chunk_size: usize,
    ) -> impl Iterator<Item = &[MeterEventPayload]> {
        assert!(chunk_size > 0, "chunk_size must be greater than zero");
        events.chunks(chunk_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use some_stripe_types::MeterPayload;

    #[test]
    fn test_batch_json_serialization() {
        let events = vec![
            MeterEventPayload::new("tokens", "cus_1", "100")
                .with_identifier("req_1")
                .with_timestamp(1_700_000_000),
            MeterEventPayload::new("tokens", "cus_2", "200").with_identifier("req_2"),
        ];

        let json = MeterBatchSerializer::serialize_batch_json(&events).unwrap();
        assert!(json.starts_with("{\"events\":["));
        assert!(json.ends_with("]}"));
        assert!(json.contains("\"stripe_customer_id\":\"cus_1\""));
        assert!(json.contains("\"stripe_customer_id\":\"cus_2\""));
        assert!(json.contains("\"value\":\"100\""));
        assert!(json.contains("\"value\":\"200\""));
    }

    #[test]
    fn test_single_event_serialization() {
        let event = MeterEventPayload::new("api_call", "cus_single", "1")
            .with_identifier("idem_single_001")
            .with_timestamp(1_700_000_500);

        let json = MeterBatchSerializer::serialize_single_event_json(&event).unwrap();
        assert!(json.contains("\"event_name\":\"api_call\""));
        assert!(json.contains("\"stripe_customer_id\":\"cus_single\""));
        assert!(json.contains("\"identifier\":\"idem_single_001\""));
        assert!(json.contains("\"timestamp\":1700000500"));
    }

    #[test]
    fn test_ndjson_serialization() {
        let events = vec![
            MeterEventPayload::new("tokens", "cus_1", "100"),
            MeterEventPayload::new("tokens", "cus_2", "200"),
        ];

        let ndjson = MeterBatchSerializer::serialize_ndjson(&events).unwrap();
        let lines: Vec<&str> = ndjson.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"cus_1\""));
        assert!(lines[1].contains("\"cus_2\""));
    }

    #[test]
    fn test_empty_batch_rejection() {
        let events: Vec<MeterEventPayload> = vec![];
        let res = MeterBatchSerializer::serialize_batch_json(&events);
        assert!(res.is_err());
    }

    #[test]
    fn test_timestamp_validation_in_batch() {
        let now = 1_700_000_000;
        let valid_events =
            vec![MeterEventPayload::new("tokens", "cus_1", "10").with_timestamp(now)];
        assert!(MeterBatchSerializer::validate_batch_with_timestamps(&valid_events, now).is_ok());

        let future_events =
            vec![MeterEventPayload::new("tokens", "cus_1", "10").with_timestamp(now + 400)];
        assert!(MeterBatchSerializer::validate_batch_with_timestamps(&future_events, now).is_err());
    }

    #[test]
    fn test_chunking() {
        let events: Vec<_> = (0..250)
            .map(|i| MeterEventPayload::new("tokens", format!("cus_{i}"), "1"))
            .collect();

        let chunks: Vec<_> = MeterBatchSerializer::chunk_events(&events, 100).collect();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 100);
        assert_eq!(chunks[1].len(), 100);
        assert_eq!(chunks[2].len(), 50);
    }

    #[test]
    fn test_throughput_simulation() {
        let mut events = Vec::with_capacity(1000);
        for i in 0..1000 {
            events.push(
                MeterEventPayload::from_payload(
                    "alpaca_tokens",
                    MeterPayload::new(format!("cus_{i}"), format!("{}", i * 10)),
                )
                .with_identifier(format!("req_bench_{i}"))
                .with_timestamp(1_700_000_000 + i),
            );
        }

        let start = std::time::Instant::now();
        let json = MeterBatchSerializer::serialize_batch_json(&events).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(
            json.matches("\"event_name\":\"alpaca_tokens\"").count(),
            1000
        );
        // Ensure 1000 events serialize in well under 50 milliseconds
        assert!(
            elapsed.as_millis() < 50,
            "Serialization took {:?}, expected < 50ms",
            elapsed
        );
    }
}
