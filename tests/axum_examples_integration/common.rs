//! Shared integration test harness and deterministic wire-mock HTTP server utilities
//! for the `some-stripe` Axum examples suite.
//!
//! # Native `$VAR_NAME` Secret Resolution (PR #150) & Dual-Mode Execution
//! 1. **Native `$VAR_NAME` Resolution**: Uses `StripeClient::new("$STRIPE_SECRET_KEY")`,
//!    `StripeConfig::new("$STRIPE_SECRET_KEY")`, and `SecretKey::new("$STRIPE_WEBHOOK_SECRET")`,
//!    which automatically resolve secrets from the process environment or `.env.local`, `.env.dev`,
//!    `.env`, `.env.example`, etc.
//! 2. **Deterministic Wire-Mock Mode (Always Active)**: Spawns an isolated local TCP
//!    HTTP listener per test, mounts the actual Axum example router with a `StripeClient`
//!    configured via `StripeConfig::new("$STRIPE_SECRET_KEY").with_api_base(...)`, and verifies
//!    every HTTP request/response, header (`Idempotency-Key`, `Stripe-Account`, `Stripe-Signature`),
//!    and error branch.
//! 3. **Live `.env` Stripe API Mode**: When `.env` / `.env.local` / `.env.dev` provides a real
//!    `sk_test_...` key and `STRIPE_RUN_LIVE_TESTS=1` (or non-placeholder key), live E2E scenarios
//!    execute against `https://api.stripe.com`.

use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
    Router,
};
use serde_json::Value;
use some_stripe::{resolve_env_secret, SecretKey, StripeClient, StripeConfig};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tower::ServiceExt;

/// Resolved configuration using native `$VAR_NAME` resolution in [`SecretKey::new`].
#[derive(Debug, Clone)]
pub struct AxumDotEnvConfig {
    /// Resolved Stripe API secret key (`$STRIPE_SECRET_KEY`).
    pub stripe_secret_key: SecretKey,
    /// Resolved Stripe Webhook signing secret (`$STRIPE_WEBHOOK_SECRET`).
    pub stripe_webhook_secret: SecretKey,
    /// Whether live end-to-end Stripe probes are enabled (`$STRIPE_RUN_LIVE_TESTS`).
    pub run_live_tests: bool,
}

impl AxumDotEnvConfig {
    /// Resolves `"$STRIPE_SECRET_KEY"` and `"$STRIPE_WEBHOOK_SECRET"` directly via [`SecretKey::new`].
    #[must_use]
    pub fn load() -> Self {
        let stripe_secret_key = SecretKey::new("$STRIPE_SECRET_KEY");
        let stripe_webhook_secret = SecretKey::new("$STRIPE_WEBHOOK_SECRET");
        let run_live_ref = resolve_env_secret("$STRIPE_RUN_LIVE_TESTS");
        let run_live_tests = run_live_ref == "1" || run_live_ref.eq_ignore_ascii_case("true");

        Self {
            stripe_secret_key,
            stripe_webhook_secret,
            run_live_tests,
        }
    }
}

/// Returns `Some(SecretKey)` if native `$STRIPE_SECRET_KEY` resolution finds a real,
/// non-placeholder `sk_test_...` or `rk_test_...` key and live tests are enabled.
pub fn get_live_stripe_secret_key() -> Option<SecretKey> {
    let cfg = AxumDotEnvConfig::load();
    let trimmed = cfg.stripe_secret_key.expose_secret().trim();
    if trimmed.is_empty()
        || trimmed == "sk_test_replace_with_your_stripe_secret_key"
        || trimmed == "sk_test_your_stripe_secret_key_here"
        || trimmed.starts_with("sk_test_mock")
    {
        return None;
    }
    if cfg.run_live_tests && (trimmed.starts_with("sk_test_") || trimmed.starts_with("rk_test_")) {
        Some(cfg.stripe_secret_key)
    } else {
        None
    }
}

/// Constructs a [`StripeClient`] using native `StripeConfig::new("$STRIPE_SECRET_KEY")`
/// pointed at the given local mock server `api_base`.
pub fn build_mock_stripe_client(api_base: impl Into<String>) -> StripeClient {
    let config = StripeConfig::new("$STRIPE_SECRET_KEY")
        .with_api_base(api_base)
        .with_timeout(Duration::from_secs(5))
        .with_max_retries(0);
    StripeClient::with_config(config)
}

/// Constructs a live [`StripeClient`] using `StripeClient::new("$STRIPE_SECRET_KEY")`
/// when a real key is configured in `.env` and live tests are enabled.
pub fn build_live_stripe_client() -> Option<StripeClient> {
    let secret = get_live_stripe_secret_key()?;
    let config = StripeConfig::new(secret)
        .with_timeout(Duration::from_secs(20))
        .with_max_retries(2);
    Some(StripeClient::with_config(config))
}

/// Reads a complete HTTP request (headers + body) from an accepted mock TCP socket.
pub async fn read_http_request(socket: &mut TcpStream) -> (String, String) {
    let mut buf = Vec::with_capacity(8192);
    let mut chunk = [0u8; 4096];
    loop {
        let n = socket.read(&mut chunk).await.unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some((headers, body)) = try_split_http_frame(&buf) {
            return (headers, body);
        }
    }
    (String::from_utf8_lossy(&buf).to_string(), String::new())
}

/// Checks if the accumulated buffer contains a complete HTTP header + `Content-Length` body.
fn try_split_http_frame(buf: &[u8]) -> Option<(String, String)> {
    let pos = buf.windows(4).position(|w| w == b"

")?;
    let header_str = String::from_utf8_lossy(&buf[..pos]).to_string();
    let content_len = extract_content_length(&header_str);
    if buf.len() >= pos + 4 + content_len {
        let body_slice = &buf[pos + 4..pos + 4 + content_len];
        let body_str = String::from_utf8_lossy(body_slice).to_string();
        return Some((header_str, body_str));
    }
    None
}

/// Extracts the `Content-Length` value from raw HTTP request headers.
fn extract_content_length(headers: &str) -> usize {
    for line in headers.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            return rest.trim().parse::<usize>().unwrap_or(0);
        }
    }
    0
}

/// Writes an HTTP response with the given status line and JSON body to the mock socket.
pub async fn write_http_response(socket: &mut TcpStream, status_line: &str, json_body: &str) {
    let frame = format!(
        "HTTP/1.1 {status_line}
Content-Type: application/json
Content-Length: {}
Connection: close

{}",
        json_body.len(),
        json_body
    );
    socket
        .write_all(frame.as_bytes())
        .await
        .expect("Failed to write mock HTTP response");
    let _ = socket.flush().await;
}

/// Spawns a single-request mock Stripe API server that returns `status_line` and `json_body`,
/// returning `(api_base_url, JoinHandle<(headers, body)>)`.
pub async fn spawn_single_mock_stripe_server(
    status_line: &'static str,
    json_body: &'static str,
) -> (String, tokio::task::JoinHandle<(String, String)>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind local mock listener");
    let addr = listener.local_addr().expect("Failed to read local addr");
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("Failed to accept connection");
        let (headers, body) = read_http_request(&mut socket).await;
        write_http_response(&mut socket, status_line, json_body).await;
        (headers, body)
    });
    (format!("http://{addr}"), handle)
}

/// Dispatches a JSON HTTP request to an Axum [`Router`] via `tower::ServiceExt::oneshot`
/// and returns `(StatusCode, serde_json::Value)`.
pub async fn send_json_request(
    app: Router,
    method: Method,
    uri: &str,
    payload: Option<Value>,
) -> (StatusCode, Value) {
    let body_bytes = match payload {
        Some(v) => serde_json::to_vec(&v).expect("Failed to serialize JSON request"),
        None => Vec::new(),
    };
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body_bytes))
        .expect("Failed to build HTTP request");

    let resp = app
        .oneshot(req)
        .await
        .expect("Axum router oneshot execution failed");
    let status = resp.status();
    let raw = to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .expect("Failed to read response body");
    let parsed = serde_json::from_slice::<Value>(&raw).unwrap_or_else(|_| {
        serde_json::json!({ "raw_text": String::from_utf8_lossy(&raw).to_string() })
    });
    (status, parsed)
}

/// Dispatches a raw webhook HTTP POST request with an optional `Stripe-Signature` header.
pub async fn send_raw_webhook_request(
    app: Router,
    uri: &str,
    signature_header: Option<&str>,
    raw_body: &[u8],
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(sig) = signature_header {
        builder = builder.header("Stripe-Signature", sig);
    }
    let req = builder
        .body(Body::from(raw_body.to_vec()))
        .expect("Failed to build webhook request");

    let resp = app
        .oneshot(req)
        .await
        .expect("Axum webhook oneshot failed");
    let status = resp.status();
    let raw = to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .expect("Failed to read webhook response");
    let parsed = serde_json::from_slice::<Value>(&raw).unwrap_or_else(|_| {
        serde_json::json!({ "raw_text": String::from_utf8_lossy(&raw).to_string() })
    });
    (status, parsed)
}
