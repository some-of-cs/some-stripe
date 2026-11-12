//! Shared test harness for `some-stripe` Tonic gRPC integration test suites.
//!
//! Uses `some-stripe`'s native `"$STRIPE_SECRET_KEY"` and `"$STRIPE_WEBHOOK_SECRET"`
//! resolution (which automatically inspects process env, `.env.local`, `.env.dev`,
//! `.env`, `.env.example`, etc.) to initialize [`StripeClient`], [`SecretKey`], and
//! [`MeterIngestionEngine`].

#![allow(dead_code)]

use hmac::{Hmac, Mac};
use sha2::Sha256;
use some_stripe::{MeterIngestionEngine, SecretKey, StripeClient, StripeConfig, Webhook};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Resolved `.env` configuration using native `$VAR_NAME` resolution in [`SecretKey::new`].
#[derive(Debug, Clone)]
pub struct DotEnvConfig {
    /// Resolved Stripe API secret key (`"$STRIPE_SECRET_KEY"`).
    pub stripe_secret_key: SecretKey,
    /// Resolved Stripe Webhook signing secret (`"$STRIPE_WEBHOOK_SECRET"`).
    pub stripe_webhook_secret: SecretKey,
    /// Flag indicating whether optional live Stripe API tests should run.
    pub run_live_tests: bool,
}

impl DotEnvConfig {
    /// Resolves `"$STRIPE_SECRET_KEY"` and `"$STRIPE_WEBHOOK_SECRET"` directly via [`SecretKey::new`].
    #[must_use]
    pub fn load() -> Self {
        let stripe_secret_key = SecretKey::new("$STRIPE_SECRET_KEY");
        let stripe_webhook_secret = SecretKey::new("$STRIPE_WEBHOOK_SECRET");
        let run_live_ref = some_stripe::resolve_env_secret("$STRIPE_RUN_LIVE_TESTS");
        let run_live_tests = run_live_ref == "1" || run_live_ref.eq_ignore_ascii_case("true");

        Self {
            stripe_secret_key,
            stripe_webhook_secret,
            run_live_tests,
        }
    }

    /// Returns `true` when `.env` / `.env.local` / `.env.dev` provides a real `sk_test_...` key
    /// and `STRIPE_RUN_LIVE_TESTS=1` is enabled.
    #[must_use]
    pub fn should_run_live(&self) -> bool {
        let key = self.stripe_secret_key.as_str();
        self.run_live_tests
            && key.starts_with("sk_test_")
            && key != "sk_test_replace_with_your_stripe_secret_key"
    }

    /// Builds a [`StripeClient`] authenticated with `"$STRIPE_SECRET_KEY"` pointed at `api_base`.
    #[must_use]
    pub fn client_for_mock(&self, api_base: &str) -> StripeClient {
        let config = StripeConfig::new("$STRIPE_SECRET_KEY")
            .with_api_base(api_base)
            .with_max_retries(0);
        StripeClient::with_config(config)
    }

    /// Builds a live [`StripeClient`] using `StripeClient::new("$STRIPE_SECRET_KEY")`.
    #[must_use]
    pub fn live_client(&self) -> StripeClient {
        StripeClient::new("$STRIPE_SECRET_KEY")
    }

    /// Builds a [`MeterIngestionEngine`] using `MeterIngestionEngine::new(api_base, "$STRIPE_SECRET_KEY")`.
    #[must_use]
    pub fn ingestion_engine_for_mock(&self, api_base: &str) -> MeterIngestionEngine {
        MeterIngestionEngine::new(api_base, "$STRIPE_SECRET_KEY")
    }
}

/// Generates a valid Stripe `t=<ts>,v1=<hex>` webhook signature header using constant-time
/// HMAC-SHA256 primitives.
#[must_use]
pub fn sign_webhook_payload(payload: &[u8], secret: &str, timestamp: i64) -> String {
    let header = Webhook::generate_test_header_string(payload, secret, Some(timestamp));
    debug_assert!(!header.is_empty());
    header
}

/// Computes a raw hex-encoded HMAC-SHA256 digest for `"{timestamp}.{payload}"`.
#[must_use]
pub fn compute_hmac_sha256_hex(payload: &[u8], secret: &str, timestamp: i64) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("valid HMAC key");
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(payload);
    hex::encode(mac.finalize().into_bytes())
}

/// Captured HTTP request received by [`MockStripeServer`].
#[derive(Debug, Clone)]
pub struct CapturedWireRequest {
    /// First HTTP request line (e.g. `POST /v1/payment_intents HTTP/1.1`).
    pub request_line: String,
    /// Full raw HTTP request headers and body.
    pub raw_request: String,
}

/// Canned HTTP response served by [`MockStripeServer`].
#[derive(Debug, Clone)]
pub struct MockWireResponse {
    /// HTTP status code (e.g. 200, 202, 400, 401, 403, 404, 429).
    pub status: u16,
    /// MIME content type.
    pub content_type: &'static str,
    /// Raw response body bytes.
    pub body: Vec<u8>,
}

impl MockWireResponse {
    /// Creates a JSON [`MockWireResponse`] with the given HTTP status and payload.
    #[must_use]
    pub fn json(status: u16, body: &str) -> Self {
        Self {
            status,
            content_type: "application/json",
            body: body.as_bytes().to_vec(),
        }
    }

    /// Creates a binary PDF [`MockWireResponse`] with HTTP 200 OK.
    #[must_use]
    pub fn pdf(bytes: &[u8]) -> Self {
        Self {
            status: 200,
            content_type: "application/pdf",
            body: bytes.to_vec(),
        }
    }

    /// Creates a simulated Stripe error response (`401`, `403`, `404`, `429`).
    #[must_use]
    pub fn stripe_error(status: u16, code: &str, message: &str) -> Self {
        let payload = format!(
            r#"{{"error":{{"type":"invalid_request_error","code":"{code}","message":"{message}"}}}}"#
        );
        Self::json(status, &payload)
    }
}

/// Maps an HTTP status code to its standard reason phrase.
#[must_use]
fn http_reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        429 => "Too Many Requests",
        _ => "Error",
    }
}

/// Lightweight local async TCP server simulating Stripe API v1 & v2 endpoints and error paths.
pub struct MockStripeServer {
    /// Base URL (`http://127.0.0.1:<port>`) for configuring [`StripeClient`].
    pub base_url: String,
    /// Thread-safe log of captured wire requests.
    pub captured: Arc<Mutex<Vec<CapturedWireRequest>>>,
    handle: tokio::task::JoinHandle<()>,
}

impl MockStripeServer {
    /// Starts a deterministic sequential [`MockStripeServer`] serving the queued `responses`.
    pub async fn start(responses: Vec<MockWireResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock Stripe TCP listener");
        let addr = listener.local_addr().expect("local addr");
        let captured = Arc::new(Mutex::new(Vec::new()));
        let captured_clone = Arc::clone(&captured);

        let handle = tokio::spawn(async move {
            for resp in responses {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let mut buf = vec![0u8; 32768];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let raw = String::from_utf8_lossy(&buf[..n]).into_owned();
                let req_line = raw.lines().next().unwrap_or_default().to_string();
                captured_clone
                    .lock()
                    .expect("lock captured")
                    .push(CapturedWireRequest {
                        request_line: req_line,
                        raw_request: raw,
                    });

                let reason = http_reason_phrase(resp.status);
                let header = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    resp.status,
                    reason,
                    resp.content_type,
                    resp.body.len()
                );
                let _ = socket.write_all(header.as_bytes()).await;
                let _ = socket.write_all(&resp.body).await;
            }
        });

        Self {
            base_url: format!("http://{addr}"),
            captured,
            handle,
        }
    }

    /// Awaits all queued responses and returns the captured wire requests.
    pub async fn finish(self) -> Vec<CapturedWireRequest> {
        let _ = self.handle.await;
        std::mem::take(&mut *self.captured.lock().expect("lock captured"))
    }
}
