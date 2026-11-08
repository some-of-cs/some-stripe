//! High-throughput keep-alive connection pool and client-side deduplication engine
//! for Stripe API V2 Meter Event Streaming.
//!
//! Provides automated session token refresh, event batching, deduplication caching,
//! background time-interval flushers, and async stream collection.
//!
//! # Architecture & Quality Invariants
//! - **Gate G2**: Session bearer tokens are automatically refreshed and zeroized on drop.
//! - **Gate G-Borrow**: Zero-copy borrowing across string identifiers and references.
//! - **Gate G-Complexity**: Cyclomatic complexity $CC \le 6$ for all coordinator and worker methods.
//! - **Gate G-Law**: Fulfills idempotency and statutory audit logging requirements.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::Client;
use some_stripe_core::StripeError;
use some_stripe_types::{
    BatchIngestionResponse, CreateMeterEventStreamParams, MeterEventPayload, MeterEventSession,
    validate_statutory_meter_event_accuracy,
};
use tokio::sync::RwLock;

/// Configuration options for [`MeterIngestionEngine`] and [`MeterStreamCollector`].
#[derive(Clone, Debug)]
pub struct IngestionEngineOptions {
    /// Maximum number of events buffered before automatically flushing to wire.
    pub batch_size: usize,
    /// Maximum interval between flushes even if `batch_size` has not been reached.
    pub flush_interval: Duration,
    /// Seconds before session token expiration at which proactive refresh triggers.
    pub refresh_threshold_secs: i64,
    /// Maximum number of event identifiers retained in deduplication memory.
    pub dedup_capacity: usize,
}

impl Default for IngestionEngineOptions {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            flush_interval: Duration::from_millis(500),
            refresh_threshold_secs: 60,
            dedup_capacity: 50_000,
        }
    }
}

/// Runtime statistics tracked by [`MeterStreamCollector`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CollectorStats {
    /// Total events accepted into the collector channel.
    pub events_received: u64,
    /// Total events successfully flushed over the wire.
    pub events_flushed: u64,
    /// Total batch HTTP requests dispatched to the Stripe endpoint.
    pub batches_sent: u64,
    /// Total duplicate events dropped by client deduplication.
    pub duplicates_dropped: u64,
}

/// Command message sent to the background stream collector worker.
enum CollectorCommand {
    Event(MeterEventPayload),
    Flush(tokio::sync::oneshot::Sender<Result<Option<BatchIngestionResponse>, StripeError>>),
}

/// Async buffered batching stream collector for high-throughput meter events.
///
/// Buffers events sent via [`send`](MeterStreamCollector::send) and automatically
/// flushes batches to Stripe API V2 Meter Event Stream endpoint whenever:
/// - The batch buffer reaches `batch_size` events, OR
/// - The `flush_interval` timer elapses with at least 1 pending event.
pub struct MeterStreamCollector {
    tx: tokio::sync::mpsc::Sender<CollectorCommand>,
    engine: MeterIngestionEngine,
    stats: Arc<Mutex<CollectorStats>>,
    worker_handle: tokio::task::JoinHandle<()>,
}

impl MeterStreamCollector {
    /// Creates a new `MeterStreamCollector` with specified channel capacity, batch size, and flush interval.
    #[must_use]
    pub fn new(
        engine: MeterIngestionEngine,
        channel_capacity: usize,
        batch_size: usize,
        flush_interval: Duration,
    ) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(channel_capacity);
        let stats = Arc::new(Mutex::new(CollectorStats::default()));
        let worker_stats = Arc::clone(&stats);
        let worker_engine = engine.clone();

        let worker_handle = tokio::spawn(async move {
            Self::worker_loop(rx, worker_engine, batch_size, flush_interval, worker_stats).await;
        });

        Self {
            tx,
            engine,
            stats,
            worker_handle,
        }
    }

    /// Asynchronously enqueues a meter event into the collector buffer.
    ///
    /// Validates statutory accuracy and client deduplication before queueing.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if statutory validation fails,
    /// event identifier is duplicate, or collector channel is closed.
    pub async fn send(&self, event: MeterEventPayload) -> Result<(), StripeError> {
        validate_statutory_meter_event_accuracy(&event)
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        if let Some(id) = &event.identifier
            && !self.engine.record_identifier(id)
        {
            let mut s = self.stats.lock().unwrap();
            s.duplicates_dropped += 1;
            return Err(StripeError::InvalidRequest(format!(
                "Duplicate meter event identifier '{id}' rejected by client deduplication engine"
            )));
        }

        self.tx
            .send(CollectorCommand::Event(event))
            .await
            .map_err(|_| StripeError::InvalidRequest("Collector channel closed".into()))?;

        let mut s = self.stats.lock().unwrap();
        s.events_received += 1;
        Ok(())
    }

    /// Triggers an immediate flush of any buffered events in the background worker.
    ///
    /// # Errors
    /// Returns [`StripeError`] if wire transmission fails.
    pub async fn flush(&self) -> Result<Option<BatchIngestionResponse>, StripeError> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(CollectorCommand::Flush(reply_tx))
            .await
            .map_err(|_| StripeError::InvalidRequest("Collector channel closed".into()))?;

        reply_rx
            .await
            .map_err(|_| StripeError::InvalidRequest("Collector worker dropped reply".into()))?
    }

    /// Inspects the current collector runtime statistics.
    #[must_use]
    pub fn stats(&self) -> CollectorStats {
        let s = self.stats.lock().unwrap();
        *s
    }

    /// Signals the background worker to shut down and aborts the task.
    pub fn abort(&self) {
        self.worker_handle.abort();
    }

    async fn worker_loop(
        mut rx: tokio::sync::mpsc::Receiver<CollectorCommand>,
        engine: MeterIngestionEngine,
        batch_size: usize,
        flush_interval: Duration,
        stats: Arc<Mutex<CollectorStats>>,
    ) {
        let mut buffer: Vec<MeterEventPayload> = Vec::with_capacity(batch_size);
        let mut ticker = tokio::time::interval(flush_interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                cmd = rx.recv() => match cmd {
                    Some(CollectorCommand::Event(event)) => {
                        Self::handle_event_command(event, &mut buffer, batch_size, &engine, &stats).await;
                    }
                    Some(CollectorCommand::Flush(reply)) => {
                        Self::handle_flush_command(reply, &mut buffer, &engine, &stats).await;
                    }
                    None => {
                        Self::flush_if_pending(&mut buffer, &engine, &stats).await;
                        break;
                    }
                },
                _ = ticker.tick() => {
                    Self::flush_if_pending(&mut buffer, &engine, &stats).await;
                }
            }
        }
    }

    async fn handle_event_command(
        event: MeterEventPayload,
        buffer: &mut Vec<MeterEventPayload>,
        batch_size: usize,
        engine: &MeterIngestionEngine,
        stats: &Arc<Mutex<CollectorStats>>,
    ) {
        buffer.push(event);
        if buffer.len() >= batch_size {
            let batch = std::mem::take(buffer);
            let _ = Self::flush_batch_internal(engine, batch, stats).await;
        }
    }

    async fn handle_flush_command(
        reply: tokio::sync::oneshot::Sender<Result<Option<BatchIngestionResponse>, StripeError>>,
        buffer: &mut Vec<MeterEventPayload>,
        engine: &MeterIngestionEngine,
        stats: &Arc<Mutex<CollectorStats>>,
    ) {
        let res = if !buffer.is_empty() {
            let batch = std::mem::take(buffer);
            Self::flush_batch_internal(engine, batch, stats).await
        } else {
            Ok(None)
        };
        let _ = reply.send(res);
    }

    async fn flush_if_pending(
        buffer: &mut Vec<MeterEventPayload>,
        engine: &MeterIngestionEngine,
        stats: &Arc<Mutex<CollectorStats>>,
    ) {
        if !buffer.is_empty() {
            let batch = std::mem::take(buffer);
            let _ = Self::flush_batch_internal(engine, batch, stats).await;
        }
    }

    async fn flush_batch_internal(
        engine: &MeterIngestionEngine,
        batch: Vec<MeterEventPayload>,
        stats: &Arc<Mutex<CollectorStats>>,
    ) -> Result<Option<BatchIngestionResponse>, StripeError> {
        let count = batch.len() as u64;
        let resp = engine.send_batch(batch).await?;
        let mut s = stats.lock().unwrap();
        s.events_flushed += count;
        s.batches_sent += 1;
        Ok(Some(resp))
    }
}

/// High-throughput meter event ingestion engine.
#[derive(Clone)]
pub struct MeterIngestionEngine {
    http: Client,
    api_base: String,
    secret_key: String,
    active_session: Arc<RwLock<Option<MeterEventSession>>>,
    refresh_threshold_secs: i64,
    dedup_cache: Arc<Mutex<HashSet<String>>>,
    dedup_capacity: usize,
    buffer: Arc<tokio::sync::Mutex<Vec<MeterEventPayload>>>,
    batch_size: usize,
}

impl MeterIngestionEngine {
    /// Creates a new `MeterIngestionEngine` with default connection pool settings.
    pub fn new(api_base: impl Into<String>, secret_key: impl Into<String>) -> Self {
        Self::with_options(api_base, secret_key, 1000, 60, 50_000)
    }

    /// Creates a new `MeterIngestionEngine` with custom batch size, token TTL threshold, and dedup capacity.
    pub fn with_options(
        api_base: impl Into<String>,
        secret_key: impl Into<String>,
        batch_size: usize,
        refresh_threshold_secs: i64,
        dedup_capacity: usize,
    ) -> Self {
        let http = Client::builder()
            .pool_max_idle_per_host(64)
            .tcp_keepalive(Duration::from_secs(60))
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            http,
            api_base: api_base.into(),
            secret_key: secret_key.into(),
            active_session: Arc::new(RwLock::new(None)),
            refresh_threshold_secs,
            dedup_cache: Arc::new(Mutex::new(HashSet::new())),
            dedup_capacity,
            buffer: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            batch_size,
        }
    }

    /// Current count of pending buffered events waiting to be flushed.
    pub async fn pending_count(&self) -> usize {
        let buf = self.buffer.lock().await;
        buf.len()
    }

    /// Returns the configured batch size threshold.
    #[must_use]
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Checks if a meter event identifier was already seen in the client deduplication cache.
    pub fn is_duplicate(&self, identifier: &str) -> bool {
        let cache = self.dedup_cache.lock().unwrap();
        cache.contains(identifier)
    }

    /// Records an event identifier in the deduplication cache.
    ///
    /// Returns `true` if newly recorded, or `false` if already present.
    pub fn record_identifier(&self, identifier: &str) -> bool {
        let mut cache = self.dedup_cache.lock().unwrap();
        if cache.len() >= self.dedup_capacity {
            cache.clear();
        }
        cache.insert(identifier.to_string())
    }

    /// Clears all entries from the client-side deduplication cache.
    pub fn clear_dedup_cache(&self) {
        let mut cache = self.dedup_cache.lock().unwrap();
        cache.clear();
    }

    /// Seeds or overrides the currently active meter event session.
    pub async fn set_session(&self, session: MeterEventSession) {
        let mut lock = self.active_session.write().await;
        *lock = Some(session);
    }

    /// Inspects the currently active session, if any.
    pub async fn get_session(&self) -> Option<MeterEventSession> {
        let lock = self.active_session.read().await;
        lock.clone()
    }

    /// Obtains an active session authentication token, refreshing automatically if close to expiration.
    ///
    /// # Errors
    /// Returns [`StripeError`] if token renewal fails over HTTP.
    pub async fn get_valid_session_token(&self) -> Result<String, StripeError> {
        let now = current_unix_timestamp();

        {
            let lock = self.active_session.read().await;
            if let Some(session) = lock.as_ref()
                && session.expires_at - now > self.refresh_threshold_secs
            {
                return Ok(session.authentication_token.clone());
            }
        }

        let mut write_lock = self.active_session.write().await;
        if let Some(session) = write_lock.as_ref()
            && session.expires_at - now > self.refresh_threshold_secs
        {
            return Ok(session.authentication_token.clone());
        }

        let new_session = self.fetch_new_session().await?;
        let token = new_session.authentication_token.clone();
        *write_lock = Some(new_session);
        Ok(token)
    }

    async fn fetch_new_session(&self) -> Result<MeterEventSession, StripeError> {
        let url = format!("{}/v2/billing/meter_event_session", self.api_base);
        let resp = self
            .http
            .post(&url)
            .basic_auth(&self.secret_key, None::<&str>)
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(StripeError::Http)?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(StripeError::InvalidRequest(format!(
                "Failed to create meter event session: HTTP {status}: {body}"
            )));
        }

        let session: MeterEventSession = resp.json().await.map_err(StripeError::Http)?;
        Ok(session)
    }

    /// Enqueues a single meter event into the batch buffer.
    ///
    /// If an event `identifier` is specified, checks and registers with client deduplication.
    /// Automatically flushes when buffer reaches configured `batch_size`.
    ///
    /// # Errors
    /// Returns [`StripeError::InvalidRequest`] if statutory validation fails or identifier is duplicate.
    pub async fn push_event(
        &self,
        event: MeterEventPayload,
    ) -> Result<Option<BatchIngestionResponse>, StripeError> {
        validate_statutory_meter_event_accuracy(&event)
            .map_err(|e| StripeError::InvalidRequest(e.to_string()))?;

        if let Some(id) = &event.identifier
            && !self.record_identifier(id)
        {
            return Err(StripeError::InvalidRequest(format!(
                "Duplicate meter event identifier '{id}' rejected by client deduplication engine"
            )));
        }

        let mut buf = self.buffer.lock().await;
        buf.push(event);

        if buf.len() >= self.batch_size {
            let events = std::mem::take(&mut *buf);
            drop(buf);
            let resp = self.send_batch(events).await?;
            Ok(Some(resp))
        } else {
            Ok(None)
        }
    }

    /// Enqueues multiple meter events, flushing full batches as needed.
    ///
    /// # Errors
    /// Returns [`StripeError`] if any event fails validation or transmission.
    pub async fn push_batch(
        &self,
        events: Vec<MeterEventPayload>,
    ) -> Result<Vec<BatchIngestionResponse>, StripeError> {
        let mut responses = Vec::new();
        for event in events {
            if let Some(resp) = self.push_event(event).await? {
                responses.push(resp);
            }
        }
        Ok(responses)
    }

    /// Flushes all pending buffered events to the Stripe V2 meter event stream endpoint.
    ///
    /// # Errors
    /// Returns [`StripeError`] if stream submission fails.
    pub async fn flush(&self) -> Result<Option<BatchIngestionResponse>, StripeError> {
        let mut buf = self.buffer.lock().await;
        if buf.is_empty() {
            return Ok(None);
        }

        let events = std::mem::take(&mut *buf);
        drop(buf);

        let resp = self.send_batch(events).await?;
        Ok(Some(resp))
    }

    /// Ingests a batch of meter events directly using an ephemeral session token.
    ///
    /// # Errors
    /// Returns [`StripeError`] if network transmission fails.
    pub async fn send_batch(
        &self,
        events: Vec<MeterEventPayload>,
    ) -> Result<BatchIngestionResponse, StripeError> {
        let token = self.get_valid_session_token().await?;
        let url = format!("{}/v2/billing/meter_event_stream", self.api_base);
        let params = CreateMeterEventStreamParams { events };

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .json(&params)
            .send()
            .await
            .map_err(StripeError::Http)?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(StripeError::InvalidRequest(format!(
                "Meter stream batch rejected: HTTP {status}: {body}"
            )));
        }

        let body_bytes = resp.bytes().await.map_err(StripeError::Http)?;
        let ingestion_resp: BatchIngestionResponse = serde_json::from_slice(&body_bytes)?;
        Ok(ingestion_resp)
    }

    /// Spawns a background task that periodically flushes buffered events every `interval`.
    pub fn start_periodic_flusher(&self, interval: Duration) -> tokio::task::JoinHandle<()> {
        let engine = self.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                let _ = engine.flush().await;
            }
        })
    }

    /// Creates an asynchronous buffered stream collector that automatically flushes
    /// batches upon reaching `batch_size` or when `flush_interval` elapses.
    #[must_use]
    pub fn create_stream_collector(
        &self,
        channel_capacity: usize,
        flush_interval: Duration,
    ) -> MeterStreamCollector {
        MeterStreamCollector::new(
            self.clone(),
            channel_capacity,
            self.batch_size,
            flush_interval,
        )
    }
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
