//! ISO 4217 Minor Currency Integer Arithmetic & Conversion Engine (Gate G-Law).
//!
//! In financial transaction processing, floating-point arithmetic introduces rounding and
//! precision errors (e.g., `0.1 + 0.2 != 0.3`). The Stripe API mandates that all monetary
//! amounts are represented as integer minor currency units (e.g., cents for USD, pence for GBP).
//!
//! This module provides exact, zero-float integer arithmetic utilities for ISO 4217 currencies,
//! supporting:
//! - **Standard 2-decimal currencies** (e.g., USD, EUR, GBP, CAD, AUD): 1 unit = 100 minor units.
//! - **Non-decimal / zero-decimal currencies** (e.g., JPY, KRW, VND, CLP): 1 unit = 1 minor unit.
//! - **Three-decimal currencies** (e.g., BHD, KWD, JOD, OMR, TND): 1 unit = 1000 minor units.
//!
//! # Gate G-Law Invariant
//! All calculations are performed strictly with 64-bit and 128-bit signed integers.
//! Zero floating-point (`f32`, `f64`) operations are used.
//!
//! # Examples
//!
//! ```
//! use some_stripe_types::currency::{decimal_places, format_minor_units, minor_unit_factor, parse_to_minor_units};
//!
//! // Standard 2-decimal currency (USD)
//! assert_eq!(decimal_places("usd"), 2);
//! assert_eq!(minor_unit_factor("usd"), 100);
//! assert_eq!(format_minor_units(1050, "usd"), "10.50");
//! assert_eq!(parse_to_minor_units("10.50", "usd").unwrap(), 1050);
//!
//! // Non-decimal currency (JPY)
//! assert_eq!(decimal_places("jpy"), 0);
//! assert_eq!(minor_unit_factor("jpy"), 1);
//! assert_eq!(format_minor_units(1500, "jpy"), "1500");
//! assert_eq!(parse_to_minor_units("1500", "jpy").unwrap(), 1500);
//!
//! // 3-decimal currency (KWD)
//! assert_eq!(decimal_places("kwd"), 3);
//! assert_eq!(minor_unit_factor("kwd"), 1000);
//! assert_eq!(format_minor_units(5250, "kwd"), "5.250");
//! assert_eq!(parse_to_minor_units("5.250", "kwd").unwrap(), 5250);
//! ```

use std::fmt;

/// Returns the number of decimal places (minor unit exponent) for an ISO 4217 currency.
///
/// Supported categories:
/// - **Zero-decimal currencies**: JPY, KRW, VND, CLP, PYG, UGX, BIF, DJF, GNF, KMF, RWF, XAF, XOF, XPF
/// - **Three-decimal currencies**: BHD, KWD, JOD, OMR, TND
/// - **Standard two-decimal currencies**: USD, EUR, GBP, CAD, AUD, etc. (default: 2)
///
/// # Examples
/// ```
/// use some_stripe_types::currency::decimal_places;
///
/// assert_eq!(decimal_places("USD"), 2);
/// assert_eq!(decimal_places("eur"), 2);
/// assert_eq!(decimal_places("jpy"), 0);
/// assert_eq!(decimal_places("krw"), 0);
/// assert_eq!(decimal_places("bhd"), 3);
/// assert_eq!(decimal_places("kwd"), 3);
/// ```
#[must_use]
pub fn decimal_places(currency: &str) -> u32 {
    let lower = currency.to_ascii_lowercase();
    match lower.as_str() {
        // Zero-decimal currencies
        "bif" | "clp" | "djf" | "gnf" | "jpy" | "kmf" | "krw" | "mga" | "pyg" | "rwf" | "ugx"
        | "vnd" | "vuv" | "xaf" | "xof" | "xpf" => 0,

        // Three-decimal currencies
        "bhd" | "jod" | "kwd" | "omr" | "tnd" => 3,

        // Default: Standard two-decimal currencies
        _ => 2,
    }
}

/// Returns the integer multiplication factor for converting major currency units to minor integer units.
///
/// The factor is $10^{\text{decimal\_places}}$:
/// - `1` for 0-decimal currencies (e.g., JPY, KRW)
/// - `100` for 2-decimal currencies (e.g., USD, EUR)
/// - `1000` for 3-decimal currencies (e.g., BHD, KWD)
///
/// # Examples
/// ```
/// use some_stripe_types::currency::minor_unit_factor;
///
/// assert_eq!(minor_unit_factor("jpy"), 1);
/// assert_eq!(minor_unit_factor("usd"), 100);
/// assert_eq!(minor_unit_factor("bhd"), 1000);
/// ```
#[must_use]
pub fn minor_unit_factor(currency: &str) -> i64 {
    match decimal_places(currency) {
        0 => 1,
        3 => 1000,
        _ => 100,
    }
}

/// Returns `true` if the currency is a zero-decimal (non-decimal) currency (e.g., JPY, KRW).
///
/// # Examples
/// ```
/// use some_stripe_types::currency::is_zero_decimal_currency;
///
/// assert!(is_zero_decimal_currency("jpy"));
/// assert!(is_zero_decimal_currency("krw"));
/// assert!(!is_zero_decimal_currency("usd"));
/// ```
#[must_use]
pub fn is_zero_decimal_currency(currency: &str) -> bool {
    decimal_places(currency) == 0
}

/// Returns `true` if the currency is a three-decimal currency (e.g., BHD, KWD, JOD).
///
/// # Examples
/// ```
/// use some_stripe_types::currency::is_three_decimal_currency;
///
/// assert!(is_three_decimal_currency("bhd"));
/// assert!(is_three_decimal_currency("kwd"));
/// assert!(!is_three_decimal_currency("usd"));
/// ```
#[must_use]
pub fn is_three_decimal_currency(currency: &str) -> bool {
    decimal_places(currency) == 3
}

/// Formats an integer amount in minor currency units into a standard decimal string representation
/// without using any floating-point arithmetic.
///
/// Correctly pads leading fractional zeros and handles negative numbers.
///
/// # Examples
/// ```
/// use some_stripe_types::currency::format_minor_units;
///
/// // Standard 2-decimal currency
/// assert_eq!(format_minor_units(1050, "usd"), "10.50");
/// assert_eq!(format_minor_units(5, "usd"), "0.05");
/// assert_eq!(format_minor_units(0, "usd"), "0.00");
/// assert_eq!(format_minor_units(-2050, "usd"), "-20.50");
/// assert_eq!(format_minor_units(-5, "usd"), "-0.05");
///
/// // Zero-decimal currency
/// assert_eq!(format_minor_units(500, "jpy"), "500");
/// assert_eq!(format_minor_units(-1200, "jpy"), "-1200");
///
/// // Three-decimal currency
/// assert_eq!(format_minor_units(5250, "kwd"), "5.250");
/// assert_eq!(format_minor_units(7, "bhd"), "0.007");
/// assert_eq!(format_minor_units(-15000, "bhd"), "-15.000");
/// ```
#[must_use]
pub fn format_minor_units(amount: i64, currency: &str) -> String {
    let decimals = decimal_places(currency);
    if decimals == 0 {
        return amount.to_string();
    }

    let is_negative = amount < 0;
    // Use u64 magnitude to avoid overflow on i64::MIN
    let abs_amount = if amount == i64::MIN {
        (i64::MAX as u64) + 1
    } else {
        amount.unsigned_abs()
    };

    let factor = match decimals {
        3 => 1000u64,
        _ => 100u64,
    };

    let major = abs_amount / factor;
    let minor = abs_amount % factor;

    let prefix = if is_negative { "-" } else { "" };
    match decimals {
        3 => format!("{prefix}{major}.{minor:03}"),
        _ => format!("{prefix}{major}.{minor:02}"),
    }
}

/// Error returned when parsing a major currency string into integer minor units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurrencyParseError {
    /// Input string is empty or contains invalid characters.
    InvalidFormat(String),
    /// Input string contains more decimal places than allowed for the currency.
    TooManyDecimalPlaces {
        /// Number of decimal places found in input string.
        found: u32,
        /// Maximum allowed decimal places for this currency.
        max_allowed: u32,
    },
    /// The resulting minor unit amount exceeds the `i64` integer range.
    Overflow,
}

impl fmt::Display for CurrencyParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat(msg) => write!(f, "Invalid currency amount format: {msg}"),
            Self::TooManyDecimalPlaces { found, max_allowed } => write!(
                f,
                "Too many decimal places: found {found}, max allowed is {max_allowed}"
            ),
            Self::Overflow => write!(f, "Currency amount exceeds i64 integer limits"),
        }
    }
}

impl std::error::Error for CurrencyParseError {}

fn parse_ascii_digits_u64(s: &str) -> Result<u64, CurrencyParseError> {
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
        return Err(CurrencyParseError::InvalidFormat(format!(
            "invalid character in digits: '{s}'"
        )));
    }
    s.parse::<u64>()
        .map_err(|e| CurrencyParseError::InvalidFormat(e.to_string()))
}

fn apply_sign_and_bounds(minor_u64: u64, is_negative: bool) -> Result<i64, CurrencyParseError> {
    if is_negative {
        if minor_u64 > (i64::MAX as u64) + 1 {
            return Err(CurrencyParseError::Overflow);
        }
        if minor_u64 == (i64::MAX as u64) + 1 {
            Ok(i64::MIN)
        } else {
            Ok(-(minor_u64 as i64))
        }
    } else {
        if minor_u64 > i64::MAX as u64 {
            return Err(CurrencyParseError::Overflow);
        }
        Ok(minor_u64 as i64)
    }
}

fn parse_decimal_parts(
    int_str: &str,
    frac_str: &str,
    decimals: u32,
    factor: u64,
) -> Result<u64, CurrencyParseError> {
    if decimals == 0 {
        return Err(CurrencyParseError::TooManyDecimalPlaces {
            found: frac_str.len() as u32,
            max_allowed: 0,
        });
    }

    if frac_str.len() as u32 > decimals {
        return Err(CurrencyParseError::TooManyDecimalPlaces {
            found: frac_str.len() as u32,
            max_allowed: decimals,
        });
    }

    let int_part = if int_str.is_empty() {
        0u64
    } else {
        parse_ascii_digits_u64(int_str)?
    };

    let mut padded_frac = frac_str.to_string();
    while (padded_frac.len() as u32) < decimals {
        padded_frac.push('0');
    }

    let frac_part = if padded_frac.is_empty() {
        0u64
    } else {
        parse_ascii_digits_u64(&padded_frac)?
    };

    let major_minor = int_part
        .checked_mul(factor)
        .ok_or(CurrencyParseError::Overflow)?;
    major_minor
        .checked_add(frac_part)
        .ok_or(CurrencyParseError::Overflow)
}

/// Parses a human-readable major currency decimal string (e.g. `"10.50"`, `"500"`, `"5.250"`)
/// into integer minor units strictly using integer arithmetic (zero floating-point operations).
///
/// Rejects strings that exceed the currency's allowed decimal precision.
///
/// # Errors
/// Returns [`CurrencyParseError`] if the string is malformed, contains extra decimal places,
/// or overflows `i64`.
///
/// # Examples
/// ```
/// use some_stripe_types::currency::{CurrencyParseError, parse_to_minor_units};
///
/// // 2-decimal USD
/// assert_eq!(parse_to_minor_units("10.50", "usd").unwrap(), 1050);
/// assert_eq!(parse_to_minor_units("0.05", "usd").unwrap(), 5);
/// assert_eq!(parse_to_minor_units("-20.50", "usd").unwrap(), -2050);
/// assert!(matches!(
///     parse_to_minor_units("10.555", "usd"),
///     Err(CurrencyParseError::TooManyDecimalPlaces { found: 3, max_allowed: 2 })
/// ));
///
/// // 0-decimal JPY
/// assert_eq!(parse_to_minor_units("1500", "jpy").unwrap(), 1500);
/// assert!(matches!(
///     parse_to_minor_units("1500.5", "jpy"),
///     Err(CurrencyParseError::TooManyDecimalPlaces { found: 1, max_allowed: 0 })
/// ));
///
/// // 3-decimal KWD
/// assert_eq!(parse_to_minor_units("5.250", "kwd").unwrap(), 5250);
/// assert_eq!(parse_to_minor_units("0.007", "bhd").unwrap(), 7);
/// ```
pub fn parse_to_minor_units(major_str: &str, currency: &str) -> Result<i64, CurrencyParseError> {
    let trimmed = major_str.trim();
    if trimmed.is_empty() {
        return Err(CurrencyParseError::InvalidFormat(
            "amount string is empty".into(),
        ));
    }

    let (is_negative, rest) = if let Some(stripped) = trimmed.strip_prefix('-') {
        (true, stripped)
    } else if let Some(stripped) = trimmed.strip_prefix('+') {
        (false, stripped)
    } else {
        (false, trimmed)
    };

    if rest.is_empty() {
        return Err(CurrencyParseError::InvalidFormat(
            "amount string contains only a sign".into(),
        ));
    }

    let decimals = decimal_places(currency);
    let factor = match decimals {
        0 => 1u64,
        3 => 1000u64,
        _ => 100u64,
    };

    let parts: Vec<&str> = rest.split('.').collect();
    let total_minor = match parts.len() {
        1 => {
            let int_part = parse_ascii_digits_u64(parts[0])?;
            int_part
                .checked_mul(factor)
                .ok_or(CurrencyParseError::Overflow)?
        }
        2 => parse_decimal_parts(parts[0], parts[1], decimals, factor)?,
        _ => {
            return Err(CurrencyParseError::InvalidFormat(
                "multiple decimal points encountered".into(),
            ));
        }
    };

    apply_sign_and_bounds(total_minor, is_negative)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decimal_places_all_categories() {
        assert_eq!(decimal_places("usd"), 2);
        assert_eq!(decimal_places("EUR"), 2);
        assert_eq!(decimal_places("gbp"), 2);
        assert_eq!(decimal_places("cad"), 2);
        assert_eq!(decimal_places("aud"), 2);

        // Zero-decimal
        assert_eq!(decimal_places("jpy"), 0);
        assert_eq!(decimal_places("KRW"), 0);
        assert_eq!(decimal_places("vnd"), 0);
        assert_eq!(decimal_places("clp"), 0);
        assert_eq!(decimal_places("pyg"), 0);

        // Three-decimal
        assert_eq!(decimal_places("bhd"), 3);
        assert_eq!(decimal_places("KWD"), 3);
        assert_eq!(decimal_places("jod"), 3);
        assert_eq!(decimal_places("omr"), 3);
        assert_eq!(decimal_places("tnd"), 3);
    }

    #[test]
    fn test_minor_unit_factor() {
        assert_eq!(minor_unit_factor("usd"), 100);
        assert_eq!(minor_unit_factor("jpy"), 1);
        assert_eq!(minor_unit_factor("kwd"), 1000);
    }

    #[test]
    fn test_format_minor_units_exact() {
        assert_eq!(format_minor_units(100, "usd"), "1.00");
        assert_eq!(format_minor_units(99, "usd"), "0.99");
        assert_eq!(format_minor_units(5, "usd"), "0.05");
        assert_eq!(format_minor_units(0, "usd"), "0.00");
        assert_eq!(format_minor_units(-50, "usd"), "-0.50");
        assert_eq!(format_minor_units(-1250, "usd"), "-12.50");

        assert_eq!(format_minor_units(100, "jpy"), "100");
        assert_eq!(format_minor_units(0, "jpy"), "0");
        assert_eq!(format_minor_units(-1000, "jpy"), "-1000");

        assert_eq!(format_minor_units(5000, "bhd"), "5.000");
        assert_eq!(format_minor_units(1, "bhd"), "0.001");
        assert_eq!(format_minor_units(12, "bhd"), "0.012");
        assert_eq!(format_minor_units(123, "bhd"), "0.123");
        assert_eq!(format_minor_units(-1, "bhd"), "-0.001");
    }

    #[test]
    fn test_parse_to_minor_units_exact() {
        assert_eq!(parse_to_minor_units("1.00", "usd").unwrap(), 100);
        assert_eq!(parse_to_minor_units("1", "usd").unwrap(), 100);
        assert_eq!(parse_to_minor_units(".99", "usd").unwrap(), 99);
        assert_eq!(parse_to_minor_units("0.05", "usd").unwrap(), 5);
        assert_eq!(parse_to_minor_units("-12.50", "usd").unwrap(), -1250);

        assert_eq!(parse_to_minor_units("500", "jpy").unwrap(), 500);
        assert_eq!(parse_to_minor_units("-1500", "jpy").unwrap(), -1500);

        assert_eq!(parse_to_minor_units("5.250", "kwd").unwrap(), 5250);
        assert_eq!(parse_to_minor_units("0.001", "bhd").unwrap(), 1);
        assert_eq!(parse_to_minor_units("0.1", "bhd").unwrap(), 100);
        assert_eq!(parse_to_minor_units("-0.001", "bhd").unwrap(), -1);
    }

    #[test]
    fn test_parse_to_minor_units_errors() {
        assert!(matches!(
            parse_to_minor_units("", "usd"),
            Err(CurrencyParseError::InvalidFormat(_))
        ));
        assert!(matches!(
            parse_to_minor_units("abc", "usd"),
            Err(CurrencyParseError::InvalidFormat(_))
        ));
        assert!(matches!(
            parse_to_minor_units("1.2.3", "usd"),
            Err(CurrencyParseError::InvalidFormat(_))
        ));
        assert!(matches!(
            parse_to_minor_units("1.234", "usd"),
            Err(CurrencyParseError::TooManyDecimalPlaces {
                found: 3,
                max_allowed: 2
            })
        ));
        assert!(matches!(
            parse_to_minor_units("100.5", "jpy"),
            Err(CurrencyParseError::TooManyDecimalPlaces {
                found: 1,
                max_allowed: 0
            })
        ));
        assert!(matches!(
            parse_to_minor_units("-+5", "usd"),
            Err(CurrencyParseError::InvalidFormat(_))
        ));
        assert!(matches!(
            parse_to_minor_units("+-5", "usd"),
            Err(CurrencyParseError::InvalidFormat(_))
        ));
        assert!(matches!(
            parse_to_minor_units("--5", "usd"),
            Err(CurrencyParseError::InvalidFormat(_))
        ));
        assert!(matches!(
            parse_to_minor_units("++5", "usd"),
            Err(CurrencyParseError::InvalidFormat(_))
        ));
    }
}
