//! Production-grade Axum web server integrating the full `some-stripe` SDK suite.
//!
//! # Architecture & Quality Invariants (Rust Refactoring Specialist / msitarzewski agency)
//! - **Native `$VAR_NAME` Secret Resolution (PR #150)**: Initializes `AppState::new("$STRIPE_SECRET_KEY", "$STRIPE_WEBHOOK_SECRET")`,
//!   which automatically resolves secrets from process environment variables or `.env.local`, `.env.dev`, `.env`, `.env.example`.
//! - **Zero-Clone State Discipline (Gate G-Borrow)**: State is wrapped in `Arc<StripeClient>`,
//!   allowing zero-cost clone operations across worker threads.
//! - **Constant-Time Webhook Verification (Gate G2)**: Webhook endpoints operate on raw `Bytes`
//!   and verify HMAC-SHA256 signatures in constant time.
//! - **Statutory & Legal Compliance (Gate G-Law)**: Built-in validation for EU Omnibus Directive,
//!   EU Statutory Withdrawal, and GDPR Art. 15/17.
//! - **Strict Error Handling**: Unified `AppError` mapping to RFC 7807 JSON responses.
//!
//! # Running the Server
//! ```bash
//! cp .env.example .env
//! cargo run --example axum_stripe_server
//! ```

pub mod routes;
pub mod state;

use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use state::AppState;

/// Main entry point for the unified Axum Stripe integration server.
#[allow(dead_code)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize structured JSON/EnvFilter tracing subscriber
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,axum_stripe_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 2. Construct global AppState using native $VAR_NAME secret resolution from env / .env* files
    let state = AppState::new("$STRIPE_SECRET_KEY", "$STRIPE_WEBHOOK_SECRET");

    // 3. Configure middleware
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 4. Mount comprehensive routes covering all 8 SDK modules
    let app = routes::build_app_router()
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    // 5. Bind TCP socket listener
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("========================================================");
    tracing::info!(" 🚀 some-stripe Axum Integration Server (v0.8.0)        ");
    tracing::info!(" 🌐 Listening on http://localhost:{}                   ", port);
    tracing::info!(" 📖 Endpoints covering all 8 Stripe product modules:    ");
    tracing::info!("    - Webhooks:      POST /webhooks/stripe              ");
    tracing::info!("    - Payments:      /payments/intents, /payments/refunds");
    tracing::info!("    - Checkout:      /checkout/sessions, /checkout/payment-links");
    tracing::info!("    - Billing:       /billing/customers, /billing/subscriptions");
    tracing::info!("    - Connect:       /connect/accounts, /connect/transfers");
    tracing::info!("    - Specialized:   /specialized/tax, /specialized/issuing");
    tracing::info!("    - V2 Services:   /v2/events, /v2/billing/meters     ");
    tracing::info!("========================================================");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Graceful shutdown signal listener for SIGINT (Ctrl+C) and SIGTERM.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C; initiating graceful shutdown...");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM; initiating graceful shutdown...");
        },
    }
}
