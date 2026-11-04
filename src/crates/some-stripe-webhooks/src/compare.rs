//! Constant-time byte slice comparison to mitigate timing side-channel attacks.
//!
//! Enforces Gate G2 Invariant 1: all webhook signature comparisons must execute
//! in constant time to prevent attackers from inferring valid signature bytes
//! through execution timing variance.

use subtle::ConstantTimeEq;

/// Compares two byte slices in constant time (Gate G2 Invariant 1).
///
/// Uses [`subtle::ConstantTimeEq`] to guarantee that execution time is independent
/// of the content of the slices, preventing timing side-channel attacks against
/// webhook signatures.
///
/// If slice lengths differ, the function immediately returns `false` without
/// leaking content information (since expected HMAC signature lengths are public
/// fixed sizes, e.g., 64 hex characters for SHA-256).
///
/// # Examples
///
/// ```
/// use some_stripe_webhooks::compare::safe_constant_time_eq;
///
/// assert!(safe_constant_time_eq(b"valid_signature_hash", b"valid_signature_hash"));
/// assert!(!safe_constant_time_eq(b"valid_signature_hash", b"forged_signature_hash"));
/// assert!(!safe_constant_time_eq(b"short", b"longer_slice"));
/// ```
#[inline]
pub fn safe_constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_slices() {
        let sig1 = b"5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165";
        let sig2 = b"5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165";
        assert!(safe_constant_time_eq(sig1, sig2));
    }

    #[test]
    fn test_one_byte_delta_at_start() {
        let sig1 = b"5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165";
        let sig2 = b"6257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165";
        assert!(!safe_constant_time_eq(sig1, sig2));
    }

    #[test]
    fn test_one_byte_delta_in_middle() {
        let sig1 = b"5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165";
        let sig2 = b"5257a869e7ecebeda32bffa62cd45081b2283ddf6e6f5170c844f9152229f165";
        assert!(!safe_constant_time_eq(sig1, sig2));
    }

    #[test]
    fn test_one_byte_delta_at_end() {
        let sig1 = b"5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165";
        let sig2 = b"5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f166";
        assert!(!safe_constant_time_eq(sig1, sig2));
    }

    #[test]
    fn test_completely_different_slices() {
        let sig1 = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let sig2 = b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        assert!(!safe_constant_time_eq(sig1, sig2));
    }

    #[test]
    fn test_length_mismatch() {
        let sig = b"5257a869e7ecebeda32affa62cd45081b2283ddf6e6f5170c844f9152229f165";
        assert!(!safe_constant_time_eq(sig, &sig[..63]));
        assert!(!safe_constant_time_eq(&sig[..63], sig));
        assert!(!safe_constant_time_eq(b"", sig));
        assert!(!safe_constant_time_eq(sig, b""));
    }

    #[test]
    fn test_empty_slices() {
        assert!(safe_constant_time_eq(b"", b""));
    }
}
