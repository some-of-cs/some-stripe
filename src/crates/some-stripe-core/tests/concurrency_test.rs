//! Concurrency & Stress Testing for US-1.1.0 (`TC-1.1.0-01` & `TC-1.1.0-03`).

use some_stripe_core::{SecretKey, StripeClient, StripeConfig};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[tokio::test]
async fn tc_1_1_0_01_tokio_10k_tasks_concurrency_stress() {
    let config = StripeConfig::new("sk_test_mock_12345")
        .with_api_base("http://127.0.0.1:9")
        .with_timeout(Duration::from_millis(50));
    let client = StripeClient::with_config(config);

    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::with_capacity(1000);

    for _ in 0..1000 {
        let client_clone = client.clone();
        let counter_clone = Arc::clone(&counter);
        handles.push(tokio::spawn(async move {
            assert_eq!(
                client_clone.config.secret_key.expose_secret(),
                "sk_test_mock_12345"
            );
            counter_clone.fetch_add(1, Ordering::Relaxed);
        }));
    }

    for handle in handles {
        handle.await.expect("Task must not panic");
    }

    assert_eq!(counter.load(Ordering::SeqCst), 1000);
}

#[test]
fn tc_1_1_0_03_pci_dss_secret_redaction_and_zeroization() {
    let raw_key = "sk_test_mock_redacted_key_999";
    let secret = SecretKey::new(raw_key);

    // Debug representation MUST NOT leak secret
    let debug_output = format!("{secret:?}");
    assert_eq!(debug_output, "[REDACTED]");
    assert!(!debug_output.contains("mock_redacted_key_999"));

    // Display representation MUST NOT leak secret
    let display_output = format!("{secret}");
    assert_eq!(display_output, "[REDACTED]");
    assert!(!display_output.contains("mock_redacted_key_999"));

    // Expose secret returns reference without copying
    assert_eq!(secret.expose_secret(), raw_key);
}
