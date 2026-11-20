//! Webhook secret pre-validation, fail-closed `$VAR_NAME` `.env*` resolution, and sanitization guards.

use crate::error::StripeWebhookError;
use std::borrow::Cow;
use std::path::{Path, PathBuf};

const DOTENV_CANDIDATE_FILES: &[&str] = &[
    ".env.local",
    ".env.dev",
    ".env.development",
    ".env.test",
    ".env.prod",
    ".env.production",
    ".env",
];

const DOTENV_EXCLUDED_FILES: &[&str] = &[".env.example", ".env.sample", ".env.template"];

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

fn parse_webhook_env_ref(secret: &str) -> Option<&str> {
    let trimmed = secret.trim();
    if let Some(inner) = trimmed.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        return Some(inner.trim());
    }
    trimmed.strip_prefix('$').map(str::trim)
}

fn lookup_env_var(var_name: &str) -> Option<String> {
    let val = std::env::var(var_name).ok()?;
    let trimmed = val.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Resolves a webhook secret that may be a literal `"whsec_..."` string (returned as zero-clone
/// `Cow::Borrowed`) or a `$VAR_NAME` / `${VAR_NAME}` reference (resolved from `std::env` or
/// allowed `.env*` files, excluding template files such as `.env.example`).
///
/// Fails closed with [`StripeWebhookError::MissingSecret`] whenever a `$VAR_NAME` /
/// `${VAR_NAME}` reference cannot be resolved, ensuring the literal placeholder string is
/// never used as an HMAC signing key.
///
/// # Errors
/// Returns [`StripeWebhookError::MissingSecret`] if `secret` is an empty or unresolved
/// `$VAR_NAME` / `${VAR_NAME}` reference.
pub fn try_resolve_webhook_secret(secret: &str) -> Result<Cow<'_, str>, StripeWebhookError> {
    let Some(var_name) = parse_webhook_env_ref(secret) else {
        return Ok(Cow::Borrowed(secret));
    };
    if var_name.is_empty() {
        return Err(StripeWebhookError::MissingSecret);
    }
    if let Some(val) = lookup_env_var(var_name) {
        return Ok(Cow::Owned(val));
    }

    let roots = [
        Path::new("."),
        Path::new(".."),
        Path::new("../.."),
        Path::new("../../.."),
    ];
    resolve_in_dirs(var_name, &roots)
        .map(Cow::Owned)
        .ok_or(StripeWebhookError::MissingSecret)
}

/// Resolves a webhook secret strictly from allowed dotenv files inside `base_dir`,
/// failing closed with [`StripeWebhookError::MissingSecret`] if a `$VAR_NAME` / `${VAR_NAME}`
/// reference cannot be resolved.
///
/// Literal secrets are returned as zero-clone `Cow::Borrowed`. Template files such as
/// `.env.example` are never consulted.
///
/// # Errors
/// Returns [`StripeWebhookError::MissingSecret`] if `secret` is an empty or unresolved
/// `$VAR_NAME` / `${VAR_NAME}` reference.
pub fn try_resolve_webhook_secret_from_dir<'a>(
    secret: &'a str,
    base_dir: &Path,
) -> Result<Cow<'a, str>, StripeWebhookError> {
    let Some(var_name) = parse_webhook_env_ref(secret) else {
        return Ok(Cow::Borrowed(secret));
    };
    if var_name.is_empty() {
        return Err(StripeWebhookError::MissingSecret);
    }
    resolve_in_dir(var_name, base_dir)
        .map(Cow::Owned)
        .ok_or(StripeWebhookError::MissingSecret)
}

/// Resolves a webhook secret that may be a literal `"whsec_..."` string (returned as zero-clone
/// `Cow::Borrowed`) or a `$VAR_NAME` / `${VAR_NAME}` reference (resolved from `std::env` or
/// allowed `.env*` files, excluding template files such as `.env.example`).
///
/// Fails closed by returning an empty `Cow::Borrowed("")` when a `$VAR_NAME` / `${VAR_NAME}`
/// reference cannot be resolved, so the literal `"$VAR_NAME"` string is never used as a key.
#[must_use]
pub fn resolve_webhook_secret(secret: &str) -> Cow<'_, str> {
    try_resolve_webhook_secret(secret).unwrap_or(Cow::Borrowed(""))
}

fn resolve_in_dirs(var_name: &str, dirs: &[&Path]) -> Option<String> {
    for dir in dirs {
        if let Some(found) = resolve_in_dir(var_name, dir) {
            return Some(found);
        }
    }
    None
}

fn is_allowed_extra_dotenv(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| {
            n.starts_with(".env")
                && !DOTENV_CANDIDATE_FILES.contains(&n)
                && !DOTENV_EXCLUDED_FILES.contains(&n)
        })
        && path.is_file()
}

fn collect_extra_dotenv_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut extras: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_allowed_extra_dotenv(p))
        .collect();
    extras.sort();
    extras
}

fn lookup_in_file(path: &Path, var_name: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    parse_key(&content, var_name)
}

fn resolve_in_dir(var_name: &str, dir: &Path) -> Option<String> {
    for candidate in DOTENV_CANDIDATE_FILES {
        if let Some(val) = lookup_in_file(&dir.join(candidate), var_name) {
            return Some(val);
        }
    }
    for path in collect_extra_dotenv_files(dir) {
        if let Some(val) = lookup_in_file(&path, var_name) {
            return Some(val);
        }
    }
    None
}

fn clean_value(val: &str) -> &str {
    if let Some(dq) = val.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return dq.trim();
    }
    if let Some(sq) = val.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        return sq.trim();
    }
    val.split(" #").next().unwrap_or(val).trim()
}

fn parse_dotenv_line(raw: &str) -> Option<(&str, &str)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let line = trimmed.strip_prefix("export ").unwrap_or(trimmed).trim();
    let (k, v) = line.split_once('=')?;
    Some((k.trim(), clean_value(v.trim())))
}

fn parse_key(content: &str, target: &str) -> Option<String> {
    for raw in content.lines() {
        if let Some((k, clean)) = parse_dotenv_line(raw)
            && k == target
            && !clean.is_empty()
        {
            return Some(clean.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        TIMESTAMP_PREFIX, ToleranceConfig, V1_SIGNATURE_PREFIX, Webhook,
        compute_streaming_hmac_hex, verify_stripe_signature,
    };
    use std::fs;

    fn create_temp_webhook_dir(suffix: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "some_stripe_webhook_validation_test_{}_{}",
            std::process::id(),
            suffix
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp webhook test dir");
        dir
    }

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

        let tried = try_resolve_webhook_secret("whsec_literal_123").unwrap();
        assert!(matches!(tried, Cow::Borrowed("whsec_literal_123")));
    }

    #[test]
    fn test_env_example_never_consulted_for_webhook_secret() {
        assert!(
            !DOTENV_CANDIDATE_FILES.contains(&".env.example"),
            ".env.example must not be in DOTENV_CANDIDATE_FILES"
        );

        let dir = create_temp_webhook_dir("env_example_ignored");
        fs::write(
            dir.join(".env.example"),
            "STRIPE_WEBHOOK_SECRET=whsec_dummy_from_env_example\n",
        )
        .unwrap();

        assert_eq!(
            try_resolve_webhook_secret_from_dir("$STRIPE_WEBHOOK_SECRET", &dir),
            Err(StripeWebhookError::MissingSecret)
        );
        assert_eq!(
            try_resolve_webhook_secret_from_dir("${STRIPE_WEBHOOK_SECRET}", &dir),
            Err(StripeWebhookError::MissingSecret)
        );

        fs::write(
            dir.join(".env.local"),
            "STRIPE_WEBHOOK_SECRET=whsec_real_local_secret\n",
        )
        .unwrap();
        assert_eq!(
            try_resolve_webhook_secret_from_dir("$STRIPE_WEBHOOK_SECRET", &dir).unwrap(),
            Cow::Owned::<str>("whsec_real_local_secret".to_string())
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_unresolved_webhook_secret_fails_closed_against_forged_signatures() {
        let unset_ref = "$UNSET_WEBHOOK_SECRET_SEC01_TEST";
        let payload = br#"{"id":"evt_sec01","type":"payment_intent.succeeded"}"#;
        let now = chrono::Utc::now().timestamp();

        assert_eq!(
            try_resolve_webhook_secret(unset_ref),
            Err(StripeWebhookError::MissingSecret)
        );
        assert_eq!(resolve_webhook_secret(unset_ref), Cow::Borrowed(""));
        assert_eq!(
            try_resolve_webhook_secret("$"),
            Err(StripeWebhookError::MissingSecret)
        );
        assert_eq!(
            try_resolve_webhook_secret("${}"),
            Err(StripeWebhookError::MissingSecret)
        );

        // Attacker signs payload using the literal "$UNSET_WEBHOOK_SECRET_SEC01_TEST" string
        let forged_literal_sig =
            compute_streaming_hmac_hex(unset_ref.as_bytes(), now, payload).unwrap();
        let forged_literal_header =
            format!("{TIMESTAMP_PREFIX}{now},{V1_SIGNATURE_PREFIX}{forged_literal_sig}");

        let err_literal = Webhook::verify_header_with_config(
            payload,
            &forged_literal_header,
            unset_ref,
            &ToleranceConfig::default(),
        )
        .unwrap_err();
        assert_eq!(err_literal, StripeWebhookError::MissingSecret);
        assert!(!verify_stripe_signature(
            payload,
            &forged_literal_header,
            unset_ref
        ));

        // Attacker signs payload using a dummy .env.example secret
        let forged_example_sig =
            compute_streaming_hmac_hex(b"whsec_dummy_from_env_example", now, payload).unwrap();
        let forged_example_header =
            format!("{TIMESTAMP_PREFIX}{now},{V1_SIGNATURE_PREFIX}{forged_example_sig}");

        let err_example = Webhook::verify_header_with_config(
            payload,
            &forged_example_header,
            unset_ref,
            &ToleranceConfig::default(),
        )
        .unwrap_err();
        assert_eq!(err_example, StripeWebhookError::MissingSecret);
        assert!(!verify_stripe_signature(
            payload,
            &forged_example_header,
            unset_ref
        ));
    }
}
