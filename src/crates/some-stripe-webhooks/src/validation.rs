//! Webhook secret pre-validation, `$VAR_NAME` `.env*` resolution, and sanitization guards.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use some_stripe_core::error::StripeWebhookError;
use zeroize::Zeroizing;

const DOTENV_CANDIDATE_FILES: &[&str] = &[
    ".env.local",
    ".env.dev",
    ".env.development",
    ".env.test",
    ".env.prod",
    ".env.production",
    ".env",
];

/// Evaluates whether a webhook signing secret is blank, empty, or consists solely of whitespace.
///
/// Under Gate G2 security rules, empty or whitespace-only keys must be rejected immediately
/// before performing any expensive cryptographic operations or allocations.
///
/// # Examples
///
/// ```
/// use some_stripe_webhooks::validation::is_blank_webhook_secret;
///
/// assert!(is_blank_webhook_secret(""));
/// assert!(is_blank_webhook_secret("   \t\n "));
/// assert!(!is_blank_webhook_secret("whsec_live_test_secret"));
/// ```
#[inline]
pub fn is_blank_webhook_secret(secret: &str) -> bool {
    secret.trim().is_empty()
}

/// Resolves a webhook secret fail-closed into a [`Zeroizing<String>`].
///
/// Literal secrets (not starting with `$`) are returned wrapped in [`Zeroizing`].
/// Environment references (`$VAR_NAME` or `${VAR_NAME}`) are resolved from `std::env`
/// or local `.env*` files (excluding `.env.example`). If an environment reference
/// is empty or cannot be resolved, returns [`StripeWebhookError::MissingSecret`]
/// instead of falling back to the literal `"$VAR"` string.
pub fn try_resolve_webhook_secret(secret: &str) -> Result<Zeroizing<String>, StripeWebhookError> {
    let Some(var_name) = extract_env_var_name(secret) else {
        return Ok(Zeroizing::new(secret.to_string()));
    };
    if var_name.is_empty() {
        return Err(StripeWebhookError::MissingSecret);
    }
    lookup_env_var(var_name)
        .map(Zeroizing::new)
        .ok_or(StripeWebhookError::MissingSecret)
}

/// Resolves a webhook secret that may be a literal `"whsec_..."` string (returned as zero-clone
/// `Cow::Borrowed`) or a `$VAR_NAME` / `${VAR_NAME}` reference (resolved from `std::env` or
/// `.env.local`, `.env.dev`, `.env`, etc., excluding `.env.example`).
#[must_use]
pub fn resolve_webhook_secret(secret: &str) -> Cow<'_, str> {
    let Some(var_name) = extract_env_var_name(secret) else {
        return Cow::Borrowed(secret);
    };
    if var_name.is_empty() {
        return Cow::Borrowed(secret);
    }
    lookup_env_var(var_name).map_or(Cow::Borrowed(secret), Cow::Owned)
}

fn extract_env_var_name(secret: &str) -> Option<&str> {
    let trimmed = secret.trim();
    if let Some(inner) = trimmed.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        return Some(inner.trim());
    }
    trimmed.strip_prefix('$').map(str::trim)
}

fn lookup_env_var(var_name: &str) -> Option<String> {
    if let Ok(val) = std::env::var(var_name).map(Zeroizing::new) {
        let t = val.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }

    let roots = [
        Path::new("."),
        Path::new(".."),
        Path::new("../.."),
        Path::new("../../.."),
    ];
    roots.into_iter().find_map(|dir| resolve_in_dir(var_name, dir))
}

fn is_extra_dotenv_file(path: &Path) -> bool {
    path.is_file()
        && path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
            n.starts_with(".env") && n != ".env.example" && !DOTENV_CANDIDATE_FILES.contains(&n)
        })
}

fn resolve_in_dir(var_name: &str, dir: &Path) -> Option<String> {
    for candidate in DOTENV_CANDIDATE_FILES {
        let path = dir.join(candidate);
        if let Ok(content) = std::fs::read_to_string(&path).map(Zeroizing::new)
            && let Some(val) = parse_key(&content, var_name)
        {
            return Some(val);
        }
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return None;
    };
    let mut extras: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_extra_dotenv_file(p))
        .collect();
    extras.sort();
    extras.into_iter().find_map(|path| {
        let content = std::fs::read_to_string(&path).map(Zeroizing::new).ok()?;
        parse_key(&content, var_name)
    })
}

fn strip_env_quotes(val: &str) -> &str {
    if let Some(dq) = val.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return dq.trim();
    }
    if let Some(sq) = val.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        return sq.trim();
    }
    val.split(" #").next().unwrap_or(val).trim()
}

fn parse_env_line<'a>(raw: &'a str, target: &str) -> Option<&'a str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let line = trimmed.strip_prefix("export ").unwrap_or(trimmed).trim();
    let (k, v) = line.split_once('=')?;
    if k.trim() != target {
        return None;
    }
    let clean = strip_env_quotes(v.trim());
    (!clean.is_empty()).then_some(clean)
}

fn parse_key(content: &str, target: &str) -> Option<String> {
    content
        .lines()
        .find_map(|raw| parse_env_line(raw, target))
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blank_secrets_detected() {
        assert!(is_blank_webhook_secret(""));
        assert!(is_blank_webhook_secret(" "));
        assert!(is_blank_webhook_secret("   \t\r\n  "));
        assert!(!is_blank_webhook_secret("whsec_abc123"));
    }

    #[test]
    fn test_literal_webhook_secret_is_borrowed_zero_clone() {
        let resolved = resolve_webhook_secret("whsec_literal_123");
        assert!(matches!(resolved, Cow::Borrowed("whsec_literal_123")));
    }

    #[test]
    fn test_dotenv_candidates_exclude_example_file() {
        assert!(!DOTENV_CANDIDATE_FILES.contains(&".env.example"));
    }

    #[test]
    fn test_try_resolve_unset_env_var_returns_missing_secret() {
        assert_eq!(
            try_resolve_webhook_secret("$SOME_STRIPE_UNSET_VAR_F01"),
            Err(StripeWebhookError::MissingSecret)
        );
        assert_eq!(
            try_resolve_webhook_secret("${SOME_STRIPE_UNSET_VAR_F01}"),
            Err(StripeWebhookError::MissingSecret)
        );
        assert_eq!(
            try_resolve_webhook_secret("$"),
            Err(StripeWebhookError::MissingSecret)
        );
        assert_eq!(
            try_resolve_webhook_secret("${   }"),
            Err(StripeWebhookError::MissingSecret)
        );
        let literal = try_resolve_webhook_secret("whsec_literal_ok").expect("literal must resolve");
        assert_eq!(literal.as_str(), "whsec_literal_ok");
    }
}
