//! Shared `.env` configuration loader and deterministic async HTTP wire-mock harness
//! for all `some-stripe` Actix-Web integration test suites.
//!
//! Automatically loads `STRIPE_SECRET_KEY` and `STRIPE_WEBHOOK_SECRET` from `.env`
//! at the workspace root while enforcing `ZeroizeOnDrop` secret hygiene.

#![allow(dead_code)]

use some_stripe::{MeterIngestionEngine, SecretKey, StripeClient, StripeConfig};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Parsed `.env` configuration for Actix-Web integration tests.
#[derive(Debug, Clone)]
pub struct DotEnvConfig {
    pub stripe_secret_key: SecretKey,
    pub stripe_webhook_secret: SecretKey,
    pub run_live_tests: bool,
    pub loaded_from_dotenv_file: bool,
}

impl DotEnvConfig {
    /// Loads `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, and `STRIPE_RUN_LIVE_TESTS`
    /// from the workspace `.env` file (or process environment variables).
    #[must_use]
    pub fn load() -> Self {
        let mut file_vars = HashMap::new();
        let mut loaded_from_file = false;

        for candidate in [".env", "../.env", "../../.env", "../../../.env"] {
            let path = Path::new(candidate);
            if let Ok(contents) = std::fs::read_to_string(path) {
                loaded_from_file = true;
                parse_dotenv_contents(&contents, &mut file_vars);
                break;
            }
        }

        let raw_secret = resolve_var("STRIPE_SECRET_KEY", &file_vars)
            .unwrap_or_else(|| "sk_test_dotenv_default_key".to_string());
        let raw_webhook = resolve_var("STRIPE_WEBHOOK_SECRET", &file_vars)
            .unwrap_or_else(|| "whsec_test_dotenv_default_secret".to_string());
        let run_live = resolve_var("STRIPE_RUN_LIVE_TESTS", &file_vars)
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));

        Self {
            stripe_secret_key: SecretKey::new(raw_secret),
            stripe_webhook_secret: SecretKey::new(raw_webhook),
            run_live_tests: run_live,
            loaded_from_dotenv_file: loaded_from_file,
        }
    }

    /// Returns `true` when `.env` provides a non-placeholder `sk_test_...` key and
    /// `STRIPE_RUN_LIVE_TESTS=1` is enabled.
    #[must_use]
    pub fn should_run_live(&self) -> bool {
        let key = self.stripe_secret_key.as_str();
        self.run_live_tests
            && key.starts_with("sk_test_")
            && key != "sk_test_dotenv_default_key"
            && key != "sk_test_replace_with_your_stripe_secret_key"
    }

    /// Builds a [`StripeClient`] authenticated with the `.env` `STRIPE_SECRET_KEY`
    /// pointed at the given mock server base URL.
    #[must_use]
    pub fn client_for_mock(&self, api_base: &str) -> StripeClient {
        let config = StripeConfig::new(self.stripe_secret_key.as_str())
            .with_api_base(api_base)
            .with_max_retries(0);
        StripeClient::with_config(config)
    }

    /// Builds a live [`StripeClient`] authenticated with the `.env` `STRIPE_SECRET_KEY`
    /// pointed at `https://api.stripe.com`.
    #[must_use]
    pub fn live_client(&self) -> StripeClient {
        StripeClient::new(self.stripe_secret_key.as_str())
    }

    /// Builds a [`MeterIngestionEngine`] pointed at the given mock server base URL.
    #[must_use]
    pub fn ingestion_engine_for_mock(&self, api_base: &str) -> MeterIngestionEngine {
        MeterIngestionEngine::new(api_base, self.stripe_secret_key.as_str())
    }
}

/// Parses `.env` key-value lines while ignoring comments and trimming quotes.
fn parse_dotenv_contents(contents: &str, out: &mut HashMap<String, String>) {
    for raw_line in contents.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let line = trimmed.strip_prefix("export ").unwrap_or(trimmed).trim();
        if let Some((key, val)) = line.split_once('=') {
            let clean_key = key.trim().to_string();
            let clean_val = val
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            if !clean_key.is_empty() && !clean_val.is_empty() {
                out.insert(clean_key, clean_val);
            }
        }
    }
}

/// Resolves an environment variable from `std::env` first, then `.env` map.
fn resolve_var(name: &str, dotenv_map: &HashMap<String, String>) -> Option<String> {
    if let Ok(val) = std::env::var(name) {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    dotenv_map.get(name).cloned()
}

/// Captured HTTP request received by [`MockStripeServer`].
#[derive(Debug, Clone)]
pub struct CapturedWireRequest {
    pub request_line: String,
    pub raw_request: String,
}

/// Canned HTTP response served by [`MockStripeServer`].
#[derive(Debug, Clone)]
pub struct MockWireResponse {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

impl MockWireResponse {
    /// Creates a `200 OK` JSON response.
    #[must_use]
    pub fn json(status: u16, body: &str) -> Self {
        Self {
            status,
            content_type: "application/json",
            body: body.as_bytes().to_vec(),
        }
    }

    /// Creates a binary `application/pdf` response.
    #[must_use]
    pub fn pdf(bytes: &[u8]) -> Self {
        Self {
            status: 200,
            content_type: "application/pdf",
            body: bytes.to_vec(),
        }
    }
}

/// Lightweight local async TCP server simulating Stripe API v1 & v2 endpoints.
pub struct MockStripeServer {
    pub base_url: String,
    pub captured: Arc<Mutex<Vec<CapturedWireRequest>>>,
    handle: tokio::task::JoinHandle<()>,
}

impl MockStripeServer {
    /// Spawns a local mock Stripe server that serves the queued [`MockWireResponse`] items in order.
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

                let reason = match resp.status {
                    200 => "OK",
                    201 => "Created",
                    202 => "Accepted",
                    400 => "Bad Request",
                    404 => "Not Found",
                    _ => "Error",
                };
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

    /// Waits for all queued responses to be served and returns the captured wire requests.
    pub async fn finish(self) -> Vec<CapturedWireRequest> {
        let _ = self.handle.await;
        self.captured.lock().expect("lock captured").clone()
    }
}
