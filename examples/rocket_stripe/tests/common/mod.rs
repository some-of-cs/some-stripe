//! Shared integration test harness and mock Stripe TCP server utilities for the Rocket.rs example.
//!
//! Uses `some-stripe`'s native `"$STRIPE_SECRET_KEY"` and `"$STRIPE_WEBHOOK_SECRET"`
//! secret resolution (`PR #150`), which automatically inspects process environment variables,
//! `.env.local`, `.env.dev`, `.env`, `.env.example`, and custom `.env.*` files.

#![allow(dead_code)]

use rocket::local::asynchronous::Client;
use rocket_stripe_example::{build_rocket, config::AppConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Reads a complete HTTP request from an accepted TCP stream, returning `(request_line_and_headers, body)`.
pub async fn read_http_request(socket: &mut TcpStream) -> (String, String) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = socket.read(&mut chunk).await.unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let header_str = String::from_utf8_lossy(&buf[..pos]).to_string();
            let mut content_length = 0usize;
            for line in header_str.lines() {
                let lower = line.to_ascii_lowercase();
                if let Some(val) = lower.strip_prefix("content-length:") {
                    content_length = val.trim().parse::<usize>().unwrap_or(0);
                }
            }
            let body_start = pos + 4;
            if buf.len() >= body_start + content_length {
                let body_str =
                    String::from_utf8_lossy(&buf[body_start..body_start + content_length])
                        .to_string();
                return (header_str, body_str);
            }
        }
    }
    (String::from_utf8_lossy(&buf).to_string(), String::new())
}

/// Writes an HTTP JSON response with the given status line (e.g. `"200 OK"`) to the socket.
pub async fn write_json_response(socket: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    socket.write_all(response.as_bytes()).await.unwrap();
    socket.flush().await.unwrap();
}

/// Spawns a mock Stripe API TCP listener and builds an untracked Rocket `Client` wired to it,
/// authenticating with `"$STRIPE_SECRET_KEY"` (resolved automatically by `SecretKey::new`).
pub async fn setup_mock_rocket_client(webhook_secret: &str) -> (TcpListener, Client) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind mock Stripe TCP listener");
    let addr = listener.local_addr().unwrap();
    let app_config = AppConfig::with_api_base(
        "$STRIPE_SECRET_KEY",
        webhook_secret,
        format!("http://{addr}"),
    );
    let rocket_client = Client::untracked(build_rocket(app_config))
        .await
        .expect("Failed to launch Rocket test client");
    (listener, rocket_client)
}

/// Builds a Rocket `Client` configured directly via `AppConfig::from_env()` (which calls
/// `StripeClient::new("$STRIPE_SECRET_KEY")` and `SecretKey::new("$STRIPE_WEBHOOK_SECRET")`).
/// Returns `Some(Client)` if a real `sk_test_` / `rk_test_` key is present in `.env`, or `None` if using placeholders.
pub async fn setup_live_env_rocket_client() -> Option<Client> {
    if !AppConfig::has_real_stripe_key() {
        return None;
    }
    let config = AppConfig::from_env();
    let client = Client::untracked(build_rocket(config))
        .await
        .expect("Failed to launch live .env Rocket client");
    Some(client)
}
