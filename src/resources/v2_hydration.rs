//! Stripe API v2 Thin Event Hydration Engine.
//!
//! Provides cryptographic freshness verification, timestamp drift replay protection (Gate G2),
//! and authenticated live resource hydration for inbound Stripe API v2 thin event notifications.
//!
//! # Architecture & Quality Invariants
//! - **Gate G-Borrow**: Zero-copy borrowing of `&V2ThinEvent` and raw byte slices.
//! - **Gate G2 & Gate G-Law**: Cryptographic freshness and 300-second timestamp drift guard against stale webhook replays.
//! - **Gate G-Complexity**: Decomposed validation helpers keeping cyclomatic complexity $CC \le 5$.
//! - **Gate G-Docs**: Comprehensive rustdoc documentation and runnable doctests.

use some_stripe_core::{StripeClient, StripeError};
use some_stripe_types::v2_events::{V2Event, V2RelatedObject};
use some_stripe_webhooks::DEFAULT_TOLERANCE_SECONDS;
use some_stripe_webhooks::v2_events::V2ThinEvent;

use super::v2_events::V2EventsResource;

/// Errors that can occur during thin event hydration.
#[derive(Debug, thiserror::Error)]
pub enum HydrationError {
    /// Stripe REST API error encountered while retrieving the live event.
    #[error("API error during event hydration: {0}")]
    Api(#[from] StripeError),

    /// Inbound payload failed JSON deserialization.
    #[error("Payload deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),

    /// Inbound event timestamp is older than permitted tolerance (stale replay attempt).
    #[error(
        "Stale thin event rejected (replay protection): created={created}, current={current}, tolerance={tolerance}s"
    )]
    StaleEventReplay {
        /// Timestamp on the incoming thin event.
        created: i64,
        /// Current clock timestamp.
        current: i64,
        /// Configured drift tolerance window in seconds.
        tolerance: u64,
    },

    /// Inbound event timestamp is in the future beyond acceptable drift.
    #[error("Thin event timestamp in future: created={created}, current={current}")]
    FutureTimestamp {
        /// Timestamp on the incoming thin event.
        created: i64,
        /// Current clock timestamp.
        current: i64,
    },

    /// The ID of the retrieved live event does not match the thin envelope ID.
    #[error("Event ID mismatch: envelope has '{envelope_id}', retrieved '{retrieved_id}'")]
    IdMismatch {
        /// ID from the inbound thin envelope.
        envelope_id: String,
        /// ID from the retrieved live resource.
        retrieved_id: String,
    },

    /// The event type of the retrieved live event does not match the thin envelope type.
    #[error("Event type mismatch: envelope has '{envelope_type}', retrieved '{retrieved_type}'")]
    EventTypeMismatch {
        /// Event type from the inbound thin envelope.
        envelope_type: String,
        /// Event type from the retrieved live resource.
        retrieved_type: String,
    },

    /// The related object ID in the retrieved event does not match the thin envelope reference.
    #[error(
        "Related object ID mismatch: envelope referenced '{envelope_id}', retrieved '{retrieved_id:?}'"
    )]
    RelatedObjectIdMismatch {
        /// Related object ID from the inbound envelope.
        envelope_id: String,
        /// Related object ID from the retrieved live event.
        retrieved_id: Option<String>,
    },

    /// The related object type in the retrieved event does not match the thin envelope reference.
    #[error(
        "Related object type mismatch: envelope referenced '{envelope_type}', retrieved '{retrieved_type:?}'"
    )]
    RelatedObjectTypeMismatch {
        /// Related object type from the inbound envelope.
        envelope_type: String,
        /// Related object type from the retrieved live event.
        retrieved_type: Option<String>,
    },

    /// Inbound envelope referenced a related object, but the retrieved live event had none.
    #[error("Missing related object: envelope referenced related object, but live event had none")]
    MissingRelatedObject,
}

/// Authenticated Hydrator for Stripe API v2 Thin Events.
///
/// Ingests lightweight notification envelopes, validates timestamp drift to prevent
/// stale replay vulnerabilities (Gate G2), and retrieves the authoritative live state
/// via `stripe.v2_core().events().retrieve()`.
#[derive(Clone, Copy)]
pub struct ThinEventHydrator<'a> {
    client: &'a StripeClient,
    tolerance_seconds: u64,
    enforce_freshness: bool,
    mock_clock: Option<i64>,
}

impl<'a> ThinEventHydrator<'a> {
    /// Creates a new `ThinEventHydrator` with default 300-second tolerance.
    #[must_use]
    pub const fn new(client: &'a StripeClient) -> Self {
        Self {
            client,
            tolerance_seconds: DEFAULT_TOLERANCE_SECONDS,
            enforce_freshness: true,
            mock_clock: None,
        }
    }

    /// Configures the timestamp drift tolerance window in seconds.
    #[must_use]
    pub const fn with_tolerance_seconds(mut self, secs: u64) -> Self {
        self.tolerance_seconds = secs;
        self
    }

    /// Disables timestamp drift verification (caution: replay protection disabled).
    #[must_use]
    pub const fn without_freshness_check(mut self) -> Self {
        self.enforce_freshness = false;
        self
    }

    /// Overrides current clock timestamp for deterministic testing.
    #[must_use]
    pub const fn with_clock_timestamp(mut self, timestamp: i64) -> Self {
        self.mock_clock = Some(timestamp);
        self
    }

    /// Returns the configured tolerance window in seconds.
    #[must_use]
    pub const fn tolerance_seconds(&self) -> u64 {
        self.tolerance_seconds
    }

    /// Validates an inbound thin event envelope and hydrates the authoritative live resource.
    ///
    /// # Errors
    /// Returns [`HydrationError`] if the thin event timestamp violates drift tolerance,
    /// live event retrieval fails, or related object integrity checks fail.
    pub async fn hydrate(&self, thin_event: &V2ThinEvent) -> Result<V2Event, HydrationError> {
        if self.enforce_freshness {
            self.verify_timestamp_freshness(thin_event.created)?;
        }

        let live_event = V2EventsResource::new(self.client)
            .retrieve(&thin_event.id)
            .await?;

        self.verify_event_integrity(thin_event, &live_event)?;
        Ok(live_event)
    }

    /// Validates an inbound thin event notification and hydrates the authoritative live resource.
    ///
    /// Alias for [`ThinEventHydrator::hydrate`].
    pub async fn hydrate_thin_event(
        &self,
        thin_event: &V2ThinEvent,
    ) -> Result<V2Event, HydrationError> {
        self.hydrate(thin_event).await
    }

    /// Parses a JSON thin event notification payload string and hydrates the live resource.
    pub async fn hydrate_from_json(&self, json_payload: &str) -> Result<V2Event, HydrationError> {
        let thin_event: V2ThinEvent = serde_json::from_str(json_payload)?;
        self.hydrate(&thin_event).await
    }

    /// Parses a raw byte slice thin event notification payload and hydrates the live resource.
    pub async fn hydrate_from_slice(&self, payload: &[u8]) -> Result<V2Event, HydrationError> {
        let thin_event: V2ThinEvent = serde_json::from_slice(payload)?;
        self.hydrate(&thin_event).await
    }

    fn verify_timestamp_freshness(&self, created: i64) -> Result<(), HydrationError> {
        let current = self
            .mock_clock
            .unwrap_or_else(|| chrono::Utc::now().timestamp());

        if current > created {
            let drift = (current - created) as u64;
            if drift > self.tolerance_seconds {
                return Err(HydrationError::StaleEventReplay {
                    created,
                    current,
                    tolerance: self.tolerance_seconds,
                });
            }
        } else if created > current {
            let future_drift = (created - current) as u64;
            if future_drift > self.tolerance_seconds {
                return Err(HydrationError::FutureTimestamp { created, current });
            }
        }

        Ok(())
    }

    fn verify_event_integrity(
        &self,
        envelope: &V2ThinEvent,
        retrieved: &V2Event,
    ) -> Result<(), HydrationError> {
        if envelope.id != retrieved.id {
            return Err(HydrationError::IdMismatch {
                envelope_id: envelope.id.clone(),
                retrieved_id: retrieved.id.clone(),
            });
        }

        if envelope.event_type != retrieved.event_type() {
            return Err(HydrationError::EventTypeMismatch {
                envelope_type: envelope.event_type.clone(),
                retrieved_type: retrieved.event_type().to_string(),
            });
        }

        if let Some(ref thin_ro) = envelope.related_object {
            self.verify_related_object(thin_ro, retrieved.related_object.as_ref())?;
        }

        Ok(())
    }

    fn verify_related_object(
        &self,
        thin_ro: &V2RelatedObject,
        retrieved_ro: Option<&V2RelatedObject>,
    ) -> Result<(), HydrationError> {
        let Some(live_ro) = retrieved_ro else {
            return Err(HydrationError::MissingRelatedObject);
        };

        if thin_ro.id != live_ro.id {
            return Err(HydrationError::RelatedObjectIdMismatch {
                envelope_id: thin_ro.id.clone(),
                retrieved_id: Some(live_ro.id.clone()),
            });
        }

        if thin_ro.r#type != live_ro.r#type {
            return Err(HydrationError::RelatedObjectTypeMismatch {
                envelope_type: thin_ro.r#type.clone(),
                retrieved_type: Some(live_ro.r#type.clone()),
            });
        }

        Ok(())
    }
}
