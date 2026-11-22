//! Stripe currency normalization and minor-unit rounding guardrails (US-9.1.0 Unit 2).
//!
//! Enforces Stripe's exact ISO-4217 currency normalization and minor-unit divisibility rules
//! pre-flight using pure integer arithmetic with zero floating-point operations:
//!
//! - **Lowercase 3-letter ASCII ISO-4217 normalization** via [`normalize_stripe_currency`].
//! - **Strictly positive minor-unit amounts** (`amount > 0`).
//! - **Zero-decimal currencies** (`bif`, `clp`, `djf`, `gnf`, `jpy`, `kmf`, `krw`, `mga`,
//!   `pyg`, `rwf`, `vnd`, `vuv`, `xaf`, `xof`, `xpf`): any positive integer is valid.
//! - **Special Stripe two-decimal `00` backward-compatibility currencies** (`isk`, `ugx`):
//!   minor-unit amounts must end in `00` (`amount % 100 == 0`).
//! - **Three-decimal currencies** (`bhd`, `jod`, `kwd`, `omr`, `tnd`): minor-unit amounts
//!   must be a multiple of `10` (`amount % 10 == 0`).
//! - **Standard two-decimal currencies** (`usd`, `eur`, `gbp`, `sar`, `egp`, etc.): any
//!   positive integer is valid.
//!
//! # Examples
//!
//! ```
//! use some_stripe_types::currency_guardrails::{
//!     StripeCurrencyRule, normalize_stripe_currency, validate_stripe_money,
//! };
//!
//! assert_eq!(normalize_stripe_currency("USD").unwrap(), "usd");
//!
//! let jpy = validate_stripe_money(500, "JPY").unwrap();
//! assert_eq!(jpy.amount, 500);
//! assert_eq!(jpy.currency, "jpy");
//! assert_eq!(jpy.rule, StripeCurrencyRule::ZeroDecimal);
//!
//! let kwd = validate_stripe_money(1050, "KWD").unwrap();
//! assert_eq!(kwd.rule, StripeCurrencyRule::ThreeDecimalMultipleOfTen);
//! assert!(validate_stripe_money(1055, "KWD").is_err());
//!
//! let isk = validate_stripe_money(1000, "ISK").unwrap();
//! assert_eq!(isk.rule, StripeCurrencyRule::TwoDecimalZeroZero);
//! assert!(validate_stripe_money(1050, "ISK").is_err());
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stripe zero-decimal ISO-4217 currency codes (lowercase).
const ZERO_DECIMAL: &[&str] = &[
    "bif", "clp", "djf", "gnf", "jpy", "kmf", "krw", "mga", "pyg", "rwf", "vnd", "vuv", "xaf",
    "xof", "xpf",
];

/// Stripe special two-decimal backward-compatibility currencies whose minor-unit
/// amounts must end in `00` (`amount % 100 == 0`).
const TWO_DECIMAL_ZERO_ZERO: &[&str] = &["isk", "ugx"];

/// Stripe three-decimal currencies whose minor-unit amounts must be a multiple
/// of `10` (`amount % 10 == 0`).
const THREE_DECIMAL_MOD_10: &[&str] = &["bhd", "jod", "kwd", "omr", "tnd"];

/// Pre-flight validation errors for Stripe currency codes and minor-unit amounts.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CurrencyGuardrailError {
    /// Currency code is not a valid 3-letter ASCII ISO-4217 code.
    #[error("invalid ISO-4217 currency code '{code}': expected exactly 3 ASCII letters")]
    InvalidCurrencyCode {
        /// The offending raw currency code supplied by the caller.
        code: String,
    },

    /// Minor-unit amount is zero or negative.
    #[error("minor-unit amount must be strictly positive, got {amount}")]
    NonPositiveAmount {
        /// The offending non-positive minor-unit amount.
        amount: i64,
    },

    /// Minor-unit amount violates a Stripe currency rounding/divisibility rule.
    #[error("amount {amount} for '{currency}' must be a multiple of {required_multiple}")]
    InvalidMinorUnitMultiple {
        /// The offending minor-unit amount.
        amount: i64,
        /// Normalized lowercase 3-letter ISO-4217 currency code.
        currency: String,
        /// Required divisor (`100` for `isk`/`ugx`, `10` for 3-decimal currencies).
        required_multiple: i64,
    },
}

/// Stripe minor-unit rounding classification for an ISO-4217 currency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StripeCurrencyRule {
    /// Zero-decimal currency (e.g. `jpy`, `krw`, `vnd`): any positive integer is valid.
    ZeroDecimal,
    /// Standard two-decimal currency (e.g. `usd`, `eur`, `gbp`, `sar`, `egp`).
    TwoDecimalStandard,
    /// Stripe special two-decimal currency (`isk`, `ugx`): must satisfy `amount % 100 == 0`.
    TwoDecimalZeroZero,
    /// Stripe three-decimal currency (`bhd`, `jod`, `kwd`, `omr`, `tnd`): `amount % 10 == 0`.
    ThreeDecimalMultipleOfTen,
}

impl StripeCurrencyRule {
    /// Returns the required integer multiple for minor-unit amounts under this rule.
    ///
    /// - [`Self::ZeroDecimal`] and [`Self::TwoDecimalStandard`] return `1`.
    /// - [`Self::TwoDecimalZeroZero`] returns `100`.
    /// - [`Self::ThreeDecimalMultipleOfTen`] returns `10`.
    ///
    /// # Examples
    ///
    /// ```
    /// use some_stripe_types::currency_guardrails::StripeCurrencyRule;
    ///
    /// assert_eq!(StripeCurrencyRule::ZeroDecimal.required_multiple(), 1);
    /// assert_eq!(StripeCurrencyRule::TwoDecimalStandard.required_multiple(), 1);
    /// assert_eq!(StripeCurrencyRule::TwoDecimalZeroZero.required_multiple(), 100);
    /// assert_eq!(StripeCurrencyRule::ThreeDecimalMultipleOfTen.required_multiple(), 10);
    /// ```
    #[must_use]
    pub const fn required_multiple(self) -> i64 {
        match self {
            Self::ZeroDecimal | Self::TwoDecimalStandard => 1,
            Self::TwoDecimalZeroZero => 100,
            Self::ThreeDecimalMultipleOfTen => 10,
        }
    }
}

/// A Stripe monetary amount that has passed pre-flight currency normalization
/// and minor-unit divisibility validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedStripeMoney {
    /// Validated strictly positive minor-unit amount.
    pub amount: i64,
    /// Normalized lowercase 3-letter ASCII ISO-4217 currency code.
    pub currency: String,
    /// Stripe minor-unit rounding rule applicable to [`Self::currency`].
    pub rule: StripeCurrencyRule,
}

/// Checks whether `list` contains `code` using ASCII case-insensitive comparison.
fn contains_ignore_ascii_case(list: &[&str], code: &str) -> bool {
    list.iter().any(|entry| entry.eq_ignore_ascii_case(code))
}

/// Classifies an ISO-4217 currency code into its [`StripeCurrencyRule`].
///
/// Matching is ASCII case-insensitive. Currencies outside the explicit zero-decimal,
/// two-decimal `00`, and three-decimal lookup tables default to
/// [`StripeCurrencyRule::TwoDecimalStandard`].
///
/// # Examples
///
/// ```
/// use some_stripe_types::currency_guardrails::{StripeCurrencyRule, currency_rule};
///
/// assert_eq!(currency_rule("jpy"), StripeCurrencyRule::ZeroDecimal);
/// assert_eq!(currency_rule("ISK"), StripeCurrencyRule::TwoDecimalZeroZero);
/// assert_eq!(currency_rule("kwd"), StripeCurrencyRule::ThreeDecimalMultipleOfTen);
/// assert_eq!(currency_rule("USD"), StripeCurrencyRule::TwoDecimalStandard);
/// ```
#[must_use]
pub fn currency_rule(code: &str) -> StripeCurrencyRule {
    if contains_ignore_ascii_case(ZERO_DECIMAL, code) {
        return StripeCurrencyRule::ZeroDecimal;
    }
    if contains_ignore_ascii_case(TWO_DECIMAL_ZERO_ZERO, code) {
        return StripeCurrencyRule::TwoDecimalZeroZero;
    }
    if contains_ignore_ascii_case(THREE_DECIMAL_MOD_10, code) {
        return StripeCurrencyRule::ThreeDecimalMultipleOfTen;
    }
    StripeCurrencyRule::TwoDecimalStandard
}

/// Normalizes a 3-letter ASCII ISO-4217 currency code to lowercase for Stripe API calls.
///
/// # Errors
///
/// Returns [`CurrencyGuardrailError::InvalidCurrencyCode`] if `code` is not exactly
/// 3 ASCII alphabetic characters (`A-Z` or `a-z`).
///
/// # Examples
///
/// ```
/// use some_stripe_types::currency_guardrails::{
///     CurrencyGuardrailError, normalize_stripe_currency,
/// };
///
/// assert_eq!(normalize_stripe_currency("USD"), Ok("usd".to_owned()));
/// assert_eq!(normalize_stripe_currency("Eur"), Ok("eur".to_owned()));
/// assert!(matches!(
///     normalize_stripe_currency("US"),
///     Err(CurrencyGuardrailError::InvalidCurrencyCode { .. })
/// ));
/// ```
pub fn normalize_stripe_currency(code: &str) -> Result<String, CurrencyGuardrailError> {
    let is_valid = code.len() == 3 && code.bytes().all(|b| b.is_ascii_alphabetic());
    if !is_valid {
        return Err(CurrencyGuardrailError::InvalidCurrencyCode {
            code: code.to_owned(),
        });
    }
    Ok(code.to_ascii_lowercase())
}

/// Validates a Stripe minor-unit amount and currency code pre-flight using pure integer math.
///
/// Enforces:
/// 1. Valid 3-letter ASCII ISO-4217 currency code, normalized to lowercase.
/// 2. Strictly positive minor-unit amount (`amount > 0`).
/// 3. Stripe minor-unit divisibility rules:
///    - `isk`, `ugx`: `amount % 100 == 0`
///    - `bhd`, `jod`, `kwd`, `omr`, `tnd`: `amount % 10 == 0`
///    - zero-decimal and standard two-decimal currencies: any positive integer
///
/// # Errors
///
/// - [`CurrencyGuardrailError::InvalidCurrencyCode`] when `currency` is not 3 ASCII letters.
/// - [`CurrencyGuardrailError::NonPositiveAmount`] when `amount <= 0`.
/// - [`CurrencyGuardrailError::InvalidMinorUnitMultiple`] when `amount` violates the
///   currency's Stripe minor-unit multiple requirement.
///
/// # Examples
///
/// ```
/// use some_stripe_types::currency_guardrails::{
///     CurrencyGuardrailError, StripeCurrencyRule, validate_stripe_money,
/// };
///
/// let ok = validate_stripe_money(5000, "UGX").unwrap();
/// assert_eq!(ok.amount, 5000);
/// assert_eq!(ok.currency, "ugx");
/// assert_eq!(ok.rule, StripeCurrencyRule::TwoDecimalZeroZero);
///
/// let err = validate_stripe_money(5005, "UGX").unwrap_err();
/// assert_eq!(
///     err,
///     CurrencyGuardrailError::InvalidMinorUnitMultiple {
///         amount: 5005,
///         currency: "ugx".to_owned(),
///         required_multiple: 100,
///     }
/// );
/// ```
pub fn validate_stripe_money(
    amount: i64,
    currency: &str,
) -> Result<ValidatedStripeMoney, CurrencyGuardrailError> {
    let normalized = normalize_stripe_currency(currency)?;
    if amount <= 0 {
        return Err(CurrencyGuardrailError::NonPositiveAmount { amount });
    }
    let rule = currency_rule(&normalized);
    let required_multiple = rule.required_multiple();
    if amount % required_multiple != 0 {
        return Err(CurrencyGuardrailError::InvalidMinorUnitMultiple {
            amount,
            currency: normalized,
            required_multiple,
        });
    }
    Ok(ValidatedStripeMoney {
        amount,
        currency: normalized,
        rule,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_any_positive_rule(code: &str, amount: i64, rule: StripeCurrencyRule) {
        assert_eq!(currency_rule(code), rule);
        assert_eq!(
            validate_stripe_money(amount, code),
            Ok(ValidatedStripeMoney {
                amount,
                currency: code.to_ascii_lowercase(),
                rule,
            })
        );
    }

    fn assert_multiple_rule(
        code: &str,
        valid_amount: i64,
        invalid_amount: i64,
        rule: StripeCurrencyRule,
    ) {
        let expected_currency = code.to_ascii_lowercase();
        assert_eq!(currency_rule(code), rule);
        assert_eq!(
            validate_stripe_money(valid_amount, code),
            Ok(ValidatedStripeMoney {
                amount: valid_amount,
                currency: expected_currency.clone(),
                rule,
            })
        );
        assert_eq!(
            validate_stripe_money(invalid_amount, code),
            Err(CurrencyGuardrailError::InvalidMinorUnitMultiple {
                amount: invalid_amount,
                currency: expected_currency,
                required_multiple: rule.required_multiple(),
            })
        );
    }

    #[test]
    fn tc_9_1_2_zero_decimal_currencies_accept_any_positive_integer() {
        for &code in ZERO_DECIMAL {
            let upper = code.to_ascii_uppercase();
            assert_any_positive_rule(&upper, 1, StripeCurrencyRule::ZeroDecimal);
            assert_any_positive_rule(code, 1055, StripeCurrencyRule::ZeroDecimal);
        }
    }

    #[test]
    fn tc_9_1_2_standard_two_decimal_currencies_accept_any_positive_integer() {
        for code in ["USD", "eur", "GbP", "SAR", "EGP"] {
            assert_any_positive_rule(code, 1055, StripeCurrencyRule::TwoDecimalStandard);
        }
    }

    #[test]
    fn tc_9_1_2_two_decimal_zero_zero_currencies_enforce_multiple_of_100() {
        for (code, valid, invalid) in [("ISK", 1000, 1050), ("UGX", 5000, 5005)] {
            assert_multiple_rule(code, valid, invalid, StripeCurrencyRule::TwoDecimalZeroZero);
        }
    }

    #[test]
    fn tc_9_1_2_three_decimal_currencies_enforce_multiple_of_10() {
        for code in ["KWD", "BHD", "JOD", "OMR", "TND"] {
            assert_multiple_rule(
                code,
                1050,
                1055,
                StripeCurrencyRule::ThreeDecimalMultipleOfTen,
            );
        }
    }

    #[test]
    fn tc_9_1_2_normalizes_uppercase_and_mixed_case_codes() {
        let cases = [
            ("JPY", "jpy"),
            ("USD", "usd"),
            ("KwD", "kwd"),
            ("IsK", "isk"),
            ("ugx", "ugx"),
        ];
        for (input, expected) in cases {
            assert_eq!(normalize_stripe_currency(input), Ok(expected.to_owned()));
        }
    }

    #[test]
    fn tc_9_1_2_rejects_zero_and_negative_amounts() {
        for invalid_amount in [0, -1, -100, -1050, i64::MIN] {
            assert_eq!(
                validate_stripe_money(invalid_amount, "USD"),
                Err(CurrencyGuardrailError::NonPositiveAmount {
                    amount: invalid_amount,
                })
            );
        }
    }

    #[test]
    fn tc_9_1_2_rejects_invalid_currency_codes() {
        for invalid_code in ["", "U", "US", "USDD", "123", "u$d", "usd ", " usd", "€ur"] {
            let expected = CurrencyGuardrailError::InvalidCurrencyCode {
                code: invalid_code.to_owned(),
            };
            assert_eq!(
                normalize_stripe_currency(invalid_code),
                Err(expected.clone())
            );
            assert_eq!(validate_stripe_money(1000, invalid_code), Err(expected));
        }
    }

    #[test]
    fn error_messages_are_descriptive() {
        let cases = [
            (
                CurrencyGuardrailError::InvalidCurrencyCode {
                    code: "US1".to_owned(),
                },
                "US1",
            ),
            (CurrencyGuardrailError::NonPositiveAmount { amount: 0 }, "0"),
            (
                CurrencyGuardrailError::InvalidMinorUnitMultiple {
                    amount: 1055,
                    currency: "kwd".to_owned(),
                    required_multiple: 10,
                },
                "1055",
            ),
        ];
        for (err, expected) in cases {
            assert!(err.to_string().contains(expected));
        }
    }
}
