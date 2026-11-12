//! Shared zero-dependency `.env` loader for all `some-stripe` Actix-Web examples.
//!
//! Automatically loads `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, and optional
//! `STRIPE_API_BASE` from a `.env` file in the workspace root (or process environment)
//! while enforcing `ZeroizeOnDrop` secret hygiene (`[REDACTED]` Debug output).

#![allow(dead_code)]

use some_stripe::{MeterIngestionEngine, SecretKey, StripeClient, StripeConfig};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

/// Resolved `.env` runtime configuration for Actix-Web example servers.
#[derive(Debug, Clone)]
pub struct ExampleDotEnv {
    pub secret_key: SecretKey,
    pub webhook_secret: SecretKey,
    pub api_base: Option<String>,
    pub dotenv_loaded: bool,
}

impl ExampleDotEnv {
    /// Loads `.env` from the current or parent workspace directory and resolves
    /// `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, and `STRIPE_API_BASE`.
    #[must_use]
    pub fn load() -> Self {
        let mut map = HashMap::new();
        let mut loaded = false;

        for candidate in [".env", "../.env", "../../.env", "../../../.env"] {
            if let Ok(content) = std::fs::read_to_string(Path::new(candidate)) {
                loaded = true;
                parse_env_lines(&content, &mut map);
                break;
            }
        }

        let secret = resolve_key("STRIPE_SECRET_KEY", &map)
            .unwrap_or_else(|| "sk_test_example_key".to_string());
        let webhook = resolve_key("STRIPE_WEBHOOK_SECRET", &map)
            .unwrap_or_else(|| "whsec_test_secret".to_string());
        let api_base = resolve_key("STRIPE_API_BASE", &map);

        Self {
            secret_key: SecretKey::new(secret),
            webhook_secret: SecretKey::new(webhook),
            api_base,
            dotenv_loaded: loaded,
        }
    }

    /// Builds a thread-safe [`StripeClient`] configured from `.env`.
    #[must_use]
    pub fn build_client(&self) -> StripeClient {
        let mut cfg = StripeConfig::new(self.secret_key.as_str())
            .with_timeout(Duration::from_secs(15))
            .with_max_retries(3);
        if let Some(base) = self.api_base.as_deref() {
            cfg = cfg.with_api_base(base);
        }
        StripeClient::with_config(cfg)
    }

    /// Builds a [`MeterIngestionEngine`] configured from `.env`.
    #[must_use]
    pub fn build_ingestion_engine(&self, client: &StripeClient) -> MeterIngestionEngine {
        match self.api_base.as_deref() {
            Some(base) => MeterIngestionEngine::new(base, self.secret_key.as_str()),
            None => client.v2().billing().create_ingestion_engine(),
        }
    }
}

fn parse_env_lines(content: &str, out: &mut HashMap<String, String>) {
    for raw in content.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let line = trimmed.strip_prefix("export ").unwrap_or(trimmed).trim();
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim().to_string();
            let val = v.trim().trim_matches('"').trim_matches('\'').to_string();
            if !key.is_empty() && !val.is_empty() {
                out.insert(key, val);
            }
        }
    }
}

fn resolve_key(name: &str, map: &HashMap<String, String>) -> Option<String> {
    if let Ok(val) = std::env::var(name) {
        let t = val.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    map.get(name).cloned()
}
