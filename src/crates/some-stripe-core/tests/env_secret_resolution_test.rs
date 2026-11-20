//! Integration tests for fail-closed `$VAR_NAME` automatic `.env`, `.env.local`, and `.env.dev`
//! secret key resolution in `SecretKey`, `StripeConfig`, and `StripeClient`.

use some_stripe_core::env_secret::{
    DOTENV_CANDIDATE_FILES, try_resolve_env_secret, try_resolve_env_secret_from_dir,
};
use some_stripe_core::{
    SecretKey, StripeClient, StripeConfig, StripeError, parse_env_reference, resolve_env_secret,
    resolve_env_secret_from_dir,
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
fn test_env_example_is_never_consulted_and_unresolved_fails_closed() {
    assert!(
        !DOTENV_CANDIDATE_FILES.contains(&".env.example"),
        ".env.example must never be included in DOTENV_CANDIDATE_FILES"
    );

    let dir = create_temp_test_dir("sec01_fail_closed");
    fs::write(
        dir.join(".env.example"),
        "# Example file committed to git\nSTRIPE_SECRET_KEY=sk_test_from_example\n",
    )
    .unwrap();
    fs::write(
        dir.join(".env.sample"),
        "STRIPE_SECRET_KEY=sk_test_from_sample\n",
    )
    .unwrap();
    fs::write(
        dir.join(".env.template"),
        "STRIPE_SECRET_KEY=sk_test_from_template\n",
    )
    .unwrap();

    // 1. try_resolve_env_secret_from_dir must fail closed with StripeError::Config
    let err = try_resolve_env_secret_from_dir("$STRIPE_SECRET_KEY", &dir).unwrap_err();
    assert!(matches!(err, StripeError::Config(_)));

    let err_braced = try_resolve_env_secret_from_dir("${STRIPE_SECRET_KEY}", &dir).unwrap_err();
    assert!(matches!(err_braced, StripeError::Config(_)));

    // 2. resolve_env_secret_from_dir must fail closed with empty string (never literal "$VAR" or example secret)
    let resolved = resolve_env_secret_from_dir("$STRIPE_SECRET_KEY", &dir);
    assert!(resolved.is_empty());
    assert_ne!(resolved, "$STRIPE_SECRET_KEY");
    assert_ne!(resolved, "sk_test_from_example");

    // 3. Process-level resolution of an unset variable must fail closed
    let unset_ref = "$UNSET_STRIPE_SECRET_SEC01_TEST";
    assert!(matches!(
        try_resolve_env_secret(unset_ref),
        Err(StripeError::Config(_))
    ));
    assert!(resolve_env_secret(unset_ref).is_empty());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_resolves_across_dotenv_local_dev_and_env_priority_without_example() {
    let dir = create_temp_test_dir("priority");

    // 1. Only .env.example exists -> must NOT resolve
    fs::write(
        dir.join(".env.example"),
        "# Example file\nSTRIPE_SECRET_KEY=sk_test_from_example\n",
    )
    .unwrap();
    assert_eq!(resolve_env_secret_from_dir("$STRIPE_SECRET_KEY", &dir), "");

    // 2. .env resolves when present
    fs::write(
        dir.join(".env"),
        "STRIPE_SECRET_KEY=\"sk_test_from_dotenv\"\n",
    )
    .unwrap();
    assert_eq!(
        resolve_env_secret_from_dir("$STRIPE_SECRET_KEY", &dir),
        "sk_test_from_dotenv"
    );
    assert_eq!(
        try_resolve_env_secret_from_dir("$STRIPE_SECRET_KEY", &dir).unwrap(),
        "sk_test_from_dotenv"
    );

    // 3. .env.dev overrides .env
    fs::write(
        dir.join(".env.dev"),
        "export STRIPE_SECRET_KEY='sk_test_from_dotenv_dev'\n",
    )
    .unwrap();
    assert_eq!(
        resolve_env_secret_from_dir("$STRIPE_SECRET_KEY", &dir),
        "sk_test_from_dotenv_dev"
    );

    // 4. .env.local overrides .env.dev
    fs::write(
        dir.join(".env.local"),
        "STRIPE_SECRET_KEY=sk_test_from_dotenv_local # inline comment\n",
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
        "CUSTOM_STAGE_STRIPE_KEY=sk_test_staging_999\n",
    )
    .unwrap();

    assert_eq!(
        resolve_env_secret_from_dir("${CUSTOM_STAGE_STRIPE_KEY}", &dir),
        "sk_test_staging_999"
    );
    assert_eq!(
        try_resolve_env_secret_from_dir("${CUSTOM_STAGE_STRIPE_KEY}", &dir).unwrap(),
        "sk_test_staging_999"
    );
    assert_eq!(
        resolve_env_secret_from_dir("sk_test_direct_literal", &dir),
        "sk_test_direct_literal"
    );
    assert_eq!(
        try_resolve_env_secret_from_dir("sk_test_direct_literal", &dir).unwrap(),
        "sk_test_direct_literal"
    );

    let client = StripeClient::with_config(StripeConfig::new("sk_test_direct_literal"));
    assert_eq!(
        client.config.secret_key.expose_secret(),
        "sk_test_direct_literal"
    );

    let _ = fs::remove_dir_all(&dir);
}
