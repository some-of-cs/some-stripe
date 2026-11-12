//! Application configuration and Stripe client state management using `some-stripe`'s
//! native `$VAR_NAME` automatic `.env*` secret resolution (`PR #150`).

use some_stripe::{SecretKey, StripeClient, StripeConfig, resolve_env_secret};

/// Global application configuration shared across Rocket request handlers.
#[derive(Clone, Debug)]
pub struct AppConfig {
    /// Initialized, thread-safe Stripe SDK client resolved via `"$STRIPE_SECRET_KEY"`.
    pub client: StripeClient,
    /// Stripe webhook signing endpoint secret resolved via `SecretKey::new("$STRIPE_WEBHOOK_SECRET")`.
    pub webhook_secret: SecretKey,
    /// Default redirect URL on payment success (`https://...` or `http://...`).
    pub default_success_url: String,
    /// Default redirect URL on payment cancellation (`https://...` or `http://...`).
    pub default_cancel_url: String,
}

impl AppConfig {
    /// Initializes configuration using `some-stripe`'s native `$VAR_NAME` secret resolver,
    /// which automatically inspects process environment variables and `.env.local`, `.env.dev`,
    /// `.env`, `.env.example`, and custom `.env.*` files.
    #[must_use]
    pub fn from_env() -> Self {
        let webhook_secret = SecretKey::new("$STRIPE_WEBHOOK_SECRET");

        let success_ref = resolve_env_secret("$STRIPE_SUCCESS_URL");
        let default_success_url = if success_ref == "$STRIPE_SUCCESS_URL" || success_ref.is_empty()
        {
            "https://example.com/checkout/success".to_string()
        } else {
            success_ref
        };

        let cancel_ref = resolve_env_secret("$STRIPE_CANCEL_URL");
        let default_cancel_url = if cancel_ref == "$STRIPE_CANCEL_URL" || cancel_ref.is_empty() {
            "https://example.com/checkout/cancel".to_string()
        } else {
            cancel_ref
        };

        let api_base_ref = resolve_env_secret("$STRIPE_API_BASE");
        let client = if api_base_ref != "$STRIPE_API_BASE" && !api_base_ref.is_empty() {
            StripeClient::with_config(
                StripeConfig::new("$STRIPE_SECRET_KEY").with_api_base(api_base_ref),
            )
        } else {
            StripeClient::new("$STRIPE_SECRET_KEY")
        };

        Self {
            client,
            webhook_secret,
            default_success_url,
            default_cancel_url,
        }
    }

    /// Constructs an `AppConfig` pointing to a custom mock or test server base URL,
    /// resolving `api_key` and `webhook_secret` via `SecretKey::new` (supporting both
    /// `"$STRIPE_SECRET_KEY"` references and literal keys).
    #[must_use]
    pub fn with_api_base(
        api_key: impl Into<SecretKey>,
        webhook_secret: impl Into<SecretKey>,
        api_base: impl Into<String>,
    ) -> Self {
        let config = StripeConfig::new(api_key)
            .with_api_base(api_base)
            .with_max_retries(0);
        Self {
            client: StripeClient::with_config(config),
            webhook_secret: webhook_secret.into(),
            default_success_url: "https://example.com/checkout/success".to_string(),
            default_cancel_url: "https://example.com/checkout/cancel".to_string(),
        }
    }

    /// Returns `true` if a live/sandbox Stripe secret key (`sk_test_...` or `sk_live_...`) is configured
    /// in `.env` / `.env.local` / `.env.dev` (and `STRIPE_RUN_LIVE_TESTS=1` when gated) rather than the placeholder template.
    #[must_use]
    pub fn has_real_stripe_key() -> bool {
        let key = SecretKey::new("$STRIPE_SECRET_KEY");
        let k = key.as_str().trim();
        !k.is_empty()
            && k != "$STRIPE_SECRET_KEY"
            && k != "sk_test_placeholder_key_for_testing"
            && k != "sk_test_your_stripe_secret_key_here"
            && k != "sk_test_replace_with_your_stripe_secret_key"
            && (k.starts_with("sk_test_") || k.starts_with("sk_live_") || k.starts_with("rk_"))
    }
}
