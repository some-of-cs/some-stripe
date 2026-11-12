//! Webhook secret pre-validation, `$VAR_NAME` `.env*` resolution, and sanitization guards.

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
    ".env.example",
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

/// Resolves a webhook secret that may be a literal `"whsec_..."` string (returned as zero-clone
/// `Cow::Borrowed`) or a `$VAR_NAME` / `${VAR_NAME}` reference (resolved from `std::env` or
/// `.env.local`, `.env.dev`, `.env`, `.env.example`, etc.).
#[must_use]
pub fn resolve_webhook_secret(secret: &str) -> Cow<'_, str> {
    let trimmed = secret.trim();
    let var_name = if let Some(inner) = trimmed.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        inner.trim()
    } else if let Some(rest) = trimmed.strip_prefix('$') {
        rest.trim()
    } else {
        return Cow::Borrowed(secret);
    };

    if var_name.is_empty() {
        return Cow::Borrowed(secret);
    }

    if let Ok(val) = std::env::var(var_name) {
        let t = val.trim();
        if !t.is_empty() {
            return Cow::Owned(t.to_string());
        }
    }

    let roots = [
        Path::new("."),
        Path::new(".."),
        Path::new("../.."),
        Path::new("../../.."),
    ];
    for dir in roots {
        if let Some(found) = resolve_in_dir(var_name, dir) {
            return Cow::Owned(found);
        }
    }

    Cow::Borrowed(secret)
}

fn resolve_in_dir(var_name: &str, dir: &Path) -> Option<String> {
    for candidate in DOTENV_CANDIDATE_FILES {
        let path = dir.join(candidate);
        if let Ok(content) = std::fs::read_to_string(&path)
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
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(".env") && !DOTENV_CANDIDATE_FILES.contains(&n))
                && p.is_file()
        })
        .collect();
    extras.sort();
    for path in extras {
        if let Ok(content) = std::fs::read_to_string(&path)
            && let Some(val) = parse_key(&content, var_name)
        {
            return Some(val);
        }
    }
    None
}

fn parse_key(content: &str, target: &str) -> Option<String> {
    for raw in content.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let line = trimmed.strip_prefix("export ").unwrap_or(trimmed).trim();
        if let Some((k, v)) = line.split_once('=')
            && k.trim() == target
        {
            let val = v.trim();
            let clean = if let Some(dq) = val.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
                dq.trim()
            } else if let Some(sq) = val.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
                sq.trim()
            } else {
                val.split(" #").next().unwrap_or(val).trim()
            };
            if !clean.is_empty() {
                return Some(clean.to_string());
            }
        }
    }
    None
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
}
