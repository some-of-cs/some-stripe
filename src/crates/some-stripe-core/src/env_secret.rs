//! Automatic `$VAR_NAME` environment and `.env*` file secret resolver.
//!
//! Enables callers to pass `"$STRIPE_SECRET_KEY"` (or `"${STRIPE_SECRET_KEY}"`) directly
//! to `StripeClient::new("$STRIPE_SECRET_KEY")`, `StripeConfig::new("$STRIPE_SECRET_KEY")`,
//! or `SecretKey::new("$STRIPE_SECRET_KEY")`.
//!
//! When a `$`-prefixed variable reference is detected, the resolver automatically looks up
//! the variable in:
//! 1. The process environment (`std::env::var`), and
//! 2. Standard dotenv files across the current and parent workspace directories in priority order:
//!    `.env.local`, `.env.dev`, `.env.development`, `.env.test`, `.env.prod`, `.env.production`,
//!    `.env`, `.env.example`, plus any other `.env.*` file in the directory.

use std::path::{Path, PathBuf};

/// Ordered list of standard dotenv filenames checked before scanning custom `.env.*` files.
pub const DOTENV_CANDIDATE_FILES: &[&str] = &[
    ".env.local",
    ".env.dev",
    ".env.development",
    ".env.test",
    ".env.prod",
    ".env.production",
    ".env",
    ".env.example",
];

/// Extracts the environment variable name if `input` starts with `$` (e.g. `"$STRIPE_SECRET_KEY"`
/// or `"${STRIPE_SECRET_KEY}"`).
///
/// Returns `None` for literal keys (such as `"sk_test_123"`).
///
/// # Examples
/// ```
/// use some_stripe_core::env_secret::parse_env_reference;
///
/// assert_eq!(parse_env_reference("$STRIPE_SECRET_KEY"), Some("STRIPE_SECRET_KEY"));
/// assert_eq!(parse_env_reference("${STRIPE_SECRET_KEY}"), Some("STRIPE_SECRET_KEY"));
/// assert_eq!(parse_env_reference("sk_test_literal_key"), None);
/// ```
#[must_use]
pub fn parse_env_reference(input: &str) -> Option<&str> {
    let trimmed = input.trim();
    let raw_name = if let Some(inner) = trimmed.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        inner.trim()
    } else if let Some(rest) = trimmed.strip_prefix('$') {
        rest.trim()
    } else {
        return None;
    };

    if is_valid_env_var_name(raw_name) {
        Some(raw_name)
    } else {
        None
    }
}

/// Validates that a candidate variable name is non-empty and consists of ASCII alphanumeric
/// or underscore characters (`[A-Za-z_][A-Za-z0-9_]*`).
#[must_use]
fn is_valid_env_var_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Resolves a secret string that may be either a literal key (`"sk_test_..."`) or a `$`-prefixed
/// environment variable reference (`"$STRIPE_SECRET_KEY"` or `"${STRIPE_SECRET_KEY}"`).
///
/// Searches `std::env::var` first, followed by `.env.local`, `.env.dev`, `.env.development`,
/// `.env.test`, `.env.prod`, `.env.production`, `.env`, `.env.example`, and any custom `.env.*`
/// files in the current working directory and up to 3 parent directories.
#[must_use]
pub fn resolve_env_secret(input: &str) -> String {
    let Some(var_name) = parse_env_reference(input) else {
        return input.to_string();
    };

    if let Ok(val) = std::env::var(var_name) {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let roots = [
        Path::new("."),
        Path::new(".."),
        Path::new("../.."),
        Path::new("../../.."),
    ];
    resolve_var_from_dirs(var_name, &roots).unwrap_or_else(|| input.to_string())
}

/// Resolves a `$`-prefixed variable reference strictly from dotenv files inside `base_dir`
/// (useful for deterministic unit tests and isolated directory contexts).
#[must_use]
pub fn resolve_env_secret_from_dir(input: &str, base_dir: &Path) -> String {
    let Some(var_name) = parse_env_reference(input) else {
        return input.to_string();
    };
    resolve_var_from_dirs(var_name, &[base_dir]).unwrap_or_else(|| input.to_string())
}

/// Searches `dirs` in order for `var_name` across priority and custom `.env*` files.
#[must_use]
pub fn resolve_var_from_dirs(var_name: &str, dirs: &[&Path]) -> Option<String> {
    for dir in dirs {
        if let Some(found) = resolve_var_in_single_dir(var_name, dir) {
            return Some(found);
        }
    }
    None
}

fn resolve_var_in_single_dir(var_name: &str, dir: &Path) -> Option<String> {
    for candidate in DOTENV_CANDIDATE_FILES {
        let path = dir.join(candidate);
        if let Some(val) = lookup_var_in_file(&path, var_name) {
            return Some(val);
        }
    }

    for custom_path in collect_custom_dotenv_files(dir) {
        if let Some(val) = lookup_var_in_file(&custom_path, var_name) {
            return Some(val);
        }
    }
    None
}

fn collect_custom_dotenv_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut custom = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with(".env") && !DOTENV_CANDIDATE_FILES.contains(&name) && path.is_file() {
            custom.push(path);
        }
    }
    custom.sort();
    custom
}

fn lookup_var_in_file(path: &Path, target_key: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    parse_dotenv_for_key(&content, target_key)
}

/// Parses dotenv file content for `target_key`, supporting `export KEY=VAL`, single/double quotes,
/// and inline `#` comments.
#[must_use]
pub fn parse_dotenv_for_key(content: &str, target_key: &str) -> Option<String> {
    for raw_line in content.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let line = trimmed.strip_prefix("export ").unwrap_or(trimmed).trim();
        let Some((key_part, val_part)) = line.split_once('=') else {
            continue;
        };
        if key_part.trim() == target_key {
            let cleaned = clean_dotenv_value(val_part.trim());
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }
    None
}

fn clean_dotenv_value(raw_val: &str) -> String {
    if let Some(dq) = raw_val.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return dq.trim().to_string();
    }
    if let Some(sq) = raw_val.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        return sq.trim().to_string();
    }
    let without_comment = raw_val.split(" #").next().unwrap_or(raw_val).trim();
    without_comment.to_string()
}
