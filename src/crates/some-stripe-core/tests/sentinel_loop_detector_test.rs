//! Sentinel Cursor Loop Detector Unit Test Suite
//!
//! Validates TC-1.3.0-02 sentinel guard intercepting cyclical pagination
//! cursors and preventing infinite DoS memory exhaustion loops.

use some_stripe_core::error::StripeError;
use some_stripe_core::patterns::pagination::CursorLoopDetector;

#[test]
fn test_sentinel_allows_unique_ascending_cursors() {
    let mut detector = CursorLoopDetector::new();

    assert!(detector.record_cursor("cus_001").is_ok());
    assert!(detector.record_cursor("cus_002").is_ok());
    assert!(detector.record_cursor("cus_003").is_ok());
}

#[test]
fn test_sentinel_detects_and_halts_cyclic_cursor_loop() {
    let mut detector = CursorLoopDetector::new();

    assert!(detector.record_cursor("ch_aaa").is_ok());
    assert!(detector.record_cursor("ch_bbb").is_ok());
    assert!(detector.record_cursor("ch_ccc").is_ok());

    // Cyclic repeat of ch_aaa
    let err = detector
        .record_cursor("ch_aaa")
        .expect_err("Sentinel must halt duplicate cursor");

    match err {
        StripeError::PaginationLoopDetected { cursor, page } => {
            assert_eq!(cursor, "ch_aaa");
            assert_eq!(page, 4);
        }
        other => panic!("Unexpected error returned by sentinel: {other:?}"),
    }
}
