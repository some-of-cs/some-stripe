//! Integration tests for `$VAR_NAME` automatic `.env`, `.env.local`, `.env.dev`, and `.env.example`
//! secret key resolution in `SecretKey`, `StripeConfig`, and `StripeClient`.

use some_stripe_core::{
    SecretKey, StripeClient, StripeConfig, parse_env_reference, resolve_env_secret_from_dir,
};
use std::fs;
use std::path::PathBuf;

fn create_temp_test_dir(suffix: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "some_stripe_env_secret_test_{}_{}",
        std::process::id(),
        suffix
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp test dir");
    dir
}

#[test]
fn test_parse_env_reference_variants() {
    assert_eq!(
        parse_env_reference("$STRIPE_SECRET_KEY"),
        Some("STRIPE_SECRET_KEY")
    );
    assert_eq!(
        parse_env_reference("  ${STRIPE_SECRET_KEY}  "),
        Some("STRIPE_SECRET_KEY")
    );
    assert_eq!(
        parse_env_reference("$MY_CUSTOM_KEY_2"),
        Some("MY_CUSTOM_KEY_2")
    );
    assert_eq!(parse_env_reference("sk_test_literal_key_123"), None);
    assert_eq!(parse_env_reference("$"), None);
    assert_eq!(parse_env_reference("${}"), None);
}

#[test]
fn test_resolves_across_dotenv_local_dev_env_and_example_priority() {
    let dir = create_temp_test_dir("priority");

    // 1. Only .env.example exists
    fs::write(
        dir.join(".env.example"),
        "# Example file
STRIPE_SECRET_KEY=sk_test_from_example
",
    )
    .unwrap();
    assert_eq!(
        resolve_env_secret_from_dir("$STRIPE_SECRET_KEY", &dir),
        "sk_test_from_example"
    );

    // 2. .env overrides .env.example
    fs::write(
        dir.join(".env"),
        "STRIPE_SECRET_KEY="sk_test_from_dotenv"
",
    )
    .unwrap();
    assert_eq!(
        resolve_env_secret_from_dir("$STRIPE_SECRET_KEY", &dir),
        "sk_test_from_dotenv"
    );

    // 3. .env.dev overrides .env
    fs::write(
        dir.join(".env.dev"),
        "export STRIPE_SECRET_KEY='sk_test_from_dotenv_dev'
",
    )
    .unwrap();
    assert_eq!(
        resolve_env_secret_from_dir("$STRIPE_SECRET_KEY", &dir),
        "sk_test_from_dotenv_dev"
    );

    // 4. .env.local overrides .env.dev
    fs::write(
        dir.join(".env.local"),
        "STRIPE_SECRET_KEY=sk_test_from_dotenv_local # inline comment
",
    )
    .unwrap();
    assert_eq!(
        resolve_env_secret_from_dir("$STRIPE_SECRET_KEY", &dir),
        "sk_test_from_dotenv_local"
    );

    let key = SecretKey::from_dir("$STRIPE_SECRET_KEY", &dir);
    assert_eq!(key.expose_secret(), "sk_test_from_dotenv_local");
    assert_eq!(format!("{key:?}"), "[REDACTED]");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_resolves_custom_dotenv_files_and_preserves_literals() {
    let dir = create_temp_test_dir("custom_stage");
    fs::write(
        dir.join(".env.staging"),
        "CUSTOM_STAGE_STRIPE_KEY=sk_test_staging_999
",
    )
    .unwrap();

    assert_eq!(
        resolve_env_secret_from_dir("${CUSTOM_STAGE_STRIPE_KEY}", &dir),
        "sk_test_staging_999"
    );
    assert_eq!(
        resolve_env_secret_from_dir("sk_test_direct_literal", &dir),
        "sk_test_direct_literal"
    );

    let client = StripeClient::with_config(StripeConfig::new("sk_test_direct_literal"));
    assert_eq!(
        client.config.secret_key.expose_secret(),
        "sk_test_direct_literal"
    );

    let _ = fs::remove_dir_all(&dir);
}
