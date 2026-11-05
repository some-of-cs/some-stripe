//! # EU Omnibus Directive 2019/2161 Price Transparency Engine (Gate G-Law)
//!
//! Article 6a of Directive 98/6/EC (as amended by EU Directive 2019/2161, known as the
//! "Omnibus Directive" or "Enforcement and Modernisation Directive") establishes strict
//! consumer protection standards for price reduction announcements across the European Union.
//!
//! ## Statutory Mandates Enforced
//! 1. **Prior Baseline Price Disclosure**: Any announcement of a price reduction must indicate
//!    the prior lowest price applied by the trader during a period not less than 30 days prior
//!    to the application of the price reduction (`prior_30d_lowest_price`).
//! 2. **Prohibition of Deceptive Price Escalation**: The announced discounted price must not exceed
//!    the prior 30-day lowest baseline price. Raising prices immediately before announcing a discount
//!    to fabricate a false discount percentage constitutes an unfair and deceptive commercial practice.
//! 3. **Non-Negative Minor Unit Integrity (Gate G-Law)**: All monetary amounts must be valid positive
//!    non-zero integers (`i64 > 0`) representing ISO 4217 minor currency units (zero floating-point operations).
//! 4. **Logical Consistency**: The announced discounted price cannot exceed the current regular price.
//!
//! # Examples
//!
//! ```
//! use some_stripe_types::validate_omnibus_price_transparency;
//!
//! // Compliant discount: Regular €100 (10000 cents), discounted to €80 (8000 cents),
//! // with documented 30-day baseline lowest price of €85 (8500 cents).
//! assert!(validate_omnibus_price_transparency(10000, 8000, Some(8500)).is_ok());
//!
//! // Compliant regular pricing without discount (no prior lowest price required).
//! assert!(validate_omnibus_price_transparency(10000, 10000, None).is_ok());
//!
//! // Non-compliant: Discount announced without prior 30-day baseline price.
//! assert!(validate_omnibus_price_transparency(10000, 8000, None).is_err());
//!
//! // Non-compliant deceptive discount: Discounted price €90 exceeds 30-day lowest €85.
//! assert!(validate_omnibus_price_transparency(10000, 9000, Some(8500)).is_err());
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// Error type for EU Omnibus statutory compliance validations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmnibusPriceError {
    /// Invalid request parameter or statutory price transparency rule violation.
    InvalidRequest(String),
}

impl std::fmt::Display for OmnibusPriceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest(msg) => write!(f, "Invalid request parameter: {msg}"),
        }
    }
}

impl std::error::Error for OmnibusPriceError {}

/// Tax behavior specifying how tax is treated in price calculation and disclosure.
///
/// In compliance with EU Omnibus Directive 2019/2161 and consumer price disclosure regulations,
/// consumer-facing prices must transparently disclose whether taxes (e.g. VAT) are included in the price
/// (`inclusive`) or calculated and added at checkout (`exclusive`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaxBehavior {
    /// Tax is included in the unit amount (gross pricing / VAT inclusive).
    #[serde(rename = "inclusive")]
    Inclusive,
    /// Tax is calculated and added on top of the unit amount (net pricing / VAT exclusive).
    #[serde(rename = "exclusive")]
    Exclusive,
    /// Tax behavior is unspecified and inferred from account tax settings.
    #[default]
    #[serde(rename = "unspecified")]
    Unspecified,
}

impl TaxBehavior {
    /// Returns the static string representation of the tax behavior.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Inclusive => "inclusive",
            Self::Exclusive => "exclusive",
            Self::Unspecified => "unspecified",
        }
    }

    /// Returns `true` if the tax behavior is transparently specified (`inclusive` or `exclusive`).
    #[must_use]
    pub const fn is_specified(&self) -> bool {
        matches!(self, Self::Inclusive | Self::Exclusive)
    }

    /// Returns `true` if the price is inclusive of taxes.
    #[must_use]
    pub const fn is_inclusive(&self) -> bool {
        matches!(self, Self::Inclusive)
    }

    /// Returns `true` if the price is exclusive of taxes.
    #[must_use]
    pub const fn is_exclusive(&self) -> bool {
        matches!(self, Self::Exclusive)
    }
}

impl std::fmt::Display for TaxBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for TaxBehavior {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for TaxBehavior {
    type Err = OmnibusPriceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "inclusive" => Ok(Self::Inclusive),
            "exclusive" => Ok(Self::Exclusive),
            "unspecified" => Ok(Self::Unspecified),
            _ => Err(OmnibusPriceError::InvalidRequest(format!(
                "Invalid tax_behavior '{s}': expected 'inclusive', 'exclusive', or 'unspecified'"
            ))),
        }
    }
}

/// Validates transparent tax behavior disclosure under EU consumer protection regulations.
///
/// In B2C / EU Omnibus context, tax behavior cannot be `unspecified`.
pub fn validate_tax_behavior_transparency(
    tax_behavior: TaxBehavior,
) -> Result<(), OmnibusPriceError> {
    if !tax_behavior.is_specified() {
        return Err(OmnibusPriceError::InvalidRequest(
            "EU Omnibus Directive violation: transparent tax breakdown requires tax_behavior to be 'inclusive' or 'exclusive', but got 'unspecified'".into(),
        ));
    }
    Ok(())
}

/// Transparent tax breakdown under EU Omnibus Directive 2019/2161.
///
/// Computes integer-exact minor unit allocations between net amount, tax, and gross amount
/// using basis points (1 basis point = 0.01%, 100 bp = 1%, e.g., 2000 bp = 20% VAT)
/// adhering to Gate G-Law (zero floating-point operations).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmnibusTaxBreakdown {
    /// The tax behavior (`inclusive` or `exclusive`).
    pub tax_behavior: TaxBehavior,
    /// Net amount in minor currency units before tax.
    pub net_amount: i64,
    /// Tax amount (e.g. VAT) in minor currency units.
    pub tax_amount: i64,
    /// Total gross amount in minor currency units paid by the consumer.
    pub gross_amount: i64,
    /// Tax rate in basis points (e.g., 2000 for 20.00%).
    pub tax_rate_basis_points: u32,
}

impl OmnibusTaxBreakdown {
    /// Calculates a transparent tax breakdown for a given amount and tax rate in basis points.
    ///
    /// # Arguments
    /// * `amount` - Amount in minor currency units (e.g., cents). For `inclusive`, this is the gross amount;
    ///   for `exclusive`, this is the net amount.
    /// * `tax_behavior` - `TaxBehavior::Inclusive` or `TaxBehavior::Exclusive`.
    /// * `tax_rate_basis_points` - Tax rate in basis points (e.g., 2000 for 20% VAT, 1900 for 19% MwSt).
    ///
    /// # Errors
    /// Returns [`OmnibusPriceError::InvalidRequest`] if `amount <= 0`, `tax_behavior` is `Unspecified`,
    /// or `tax_rate_basis_points > 10_000` (exceeds 100%).
    pub fn calculate(
        amount: i64,
        tax_behavior: TaxBehavior,
        tax_rate_basis_points: u32,
    ) -> Result<Self, OmnibusPriceError> {
        if amount <= 0 {
            return Err(OmnibusPriceError::InvalidRequest(format!(
                "Tax breakdown amount must be a positive integer minor currency unit (> 0), got: {amount}"
            )));
        }
        if !tax_behavior.is_specified() {
            return Err(OmnibusPriceError::InvalidRequest(
                "EU Omnibus Directive transparent tax breakdown requires tax_behavior to be 'inclusive' or 'exclusive', got 'unspecified'".into(),
            ));
        }
        if tax_rate_basis_points > 10_000 {
            return Err(OmnibusPriceError::InvalidRequest(format!(
                "Tax rate basis points cannot exceed 10,000 (100%), got: {tax_rate_basis_points}"
            )));
        }

        match tax_behavior {
            TaxBehavior::Inclusive => {
                let gross_amount = amount;
                let divisor = 10_000i64 + tax_rate_basis_points as i64;
                let net_amount = (gross_amount * 10_000 + divisor / 2) / divisor;
                let tax_amount = gross_amount - net_amount;
                Ok(Self {
                    tax_behavior,
                    net_amount,
                    tax_amount,
                    gross_amount,
                    tax_rate_basis_points,
                })
            }
            TaxBehavior::Exclusive => {
                let net_amount = amount;
                let tax_amount = (net_amount * (tax_rate_basis_points as i64) + 5_000) / 10_000;
                let gross_amount = net_amount + tax_amount;
                Ok(Self {
                    tax_behavior,
                    net_amount,
                    tax_amount,
                    gross_amount,
                    tax_rate_basis_points,
                })
            }
            TaxBehavior::Unspecified => unreachable!(),
        }
    }
}

/// Validates compliance with EU Omnibus Directive 2019/2161 Article 6a regarding price reductions.
///
/// Any announcement of a price reduction must indicate the prior price applied for at least 30 days
/// before the application of the price reduction.
///
/// # Requirements (Gate G-Law & EU Directive 2019/2161 Article 6a):
/// 1. Amounts must be positive integer minor units (`> 0`).
/// 2. If `announced_discounted_price < regular_price`, `prior_30d_lowest_price` MUST be `Some(...)`.
/// 3. `announced_discounted_price` must NOT be higher than `prior_30d_lowest_price`
///    (prohibits deceptive discounts where price was raised immediately prior to the discount).
/// 4. `announced_discounted_price` must NOT exceed `regular_price`.
///
/// # Arguments
/// * `regular_price` - The regular non-discounted catalog price in minor currency units (e.g., cents).
/// * `announced_discounted_price` - The announced promotional/sale price in minor currency units.
/// * `prior_30d_lowest_price` - The lowest price applied in the preceding 30 days in minor currency units.
///
/// # Errors
/// Returns [`OmnibusPriceError::InvalidRequest`] if any of the statutory conditions are violated.
///
/// # Example
/// ```
/// use some_stripe_types::validate_omnibus_price_transparency;
///
/// // Compliant discount: regular €100 (10000 cents), announced discounted €80 (8000 cents),
/// // prior 30-day lowest price was €85 (8500 cents). Discounted price <= prior 30d lowest.
/// assert!(validate_omnibus_price_transparency(10000, 8000, Some(8500)).is_ok());
///
/// // Compliant discount where prior 30-day lowest was the regular price €100.
/// assert!(validate_omnibus_price_transparency(10000, 8000, Some(10000)).is_ok());
///
/// // Compliant regular pricing without discount (no prior lowest price required).
/// assert!(validate_omnibus_price_transparency(10000, 10000, None).is_ok());
///
/// // Non-compliant: discount announced but prior 30-day baseline is missing.
/// assert!(validate_omnibus_price_transparency(10000, 8000, None).is_err());
///
/// // Non-compliant deceptive discount: advertised discounted price €90 (9000 cents)
/// // exceeds prior 30-day lowest price €85 (8500 cents).
/// assert!(validate_omnibus_price_transparency(10000, 9000, Some(8500)).is_err());
/// ```
pub fn validate_omnibus_price_transparency(
    regular_price: i64,
    announced_discounted_price: i64,
    prior_30d_lowest_price: Option<i64>,
) -> Result<(), OmnibusPriceError> {
    // 1. Amounts must be positive non-negative values (> 0)
    if regular_price <= 0 {
        return Err(OmnibusPriceError::InvalidRequest(format!(
            "EU Omnibus Directive 2019/2161 violation: regular_price must be a positive integer minor currency unit (> 0), got: {regular_price}"
        )));
    }

    if announced_discounted_price <= 0 {
        return Err(OmnibusPriceError::InvalidRequest(format!(
            "EU Omnibus Directive 2019/2161 violation: announced_discounted_price must be a positive integer minor currency unit (> 0), got: {announced_discounted_price}"
        )));
    }

    if let Some(prior) = prior_30d_lowest_price
        && prior <= 0
    {
        return Err(OmnibusPriceError::InvalidRequest(format!(
            "EU Omnibus Directive 2019/2161 violation: prior_30d_lowest_price must be a positive integer minor currency unit (> 0), got: {prior}"
        )));
    }

    // 2. Discounted price cannot exceed regular catalog price
    if announced_discounted_price > regular_price {
        return Err(OmnibusPriceError::InvalidRequest(format!(
            "EU Omnibus Directive 2019/2161 violation: announced_discounted_price ({announced_discounted_price}) cannot exceed regular_price ({regular_price})"
        )));
    }

    // 3. If a price reduction is announced, statutory 30-day prior baseline is mandatory
    if announced_discounted_price < regular_price {
        let Some(prior_lowest) = prior_30d_lowest_price else {
            return Err(OmnibusPriceError::InvalidRequest(
                "EU Omnibus Directive 2019/2161 Article 6a violation: prior_30d_lowest_price must be specified when announcing a discounted price".into(),
            ));
        };

        // 4. Deceptive discount prohibition: discounted price cannot exceed 30-day lowest baseline
        if announced_discounted_price > prior_lowest {
            return Err(OmnibusPriceError::InvalidRequest(format!(
                "EU Omnibus Directive 2019/2161 Article 6a violation: announced discounted price ({announced_discounted_price}) exceeds the prior 30-day lowest baseline price ({prior_lowest}), constituting deceptive discounting"
            )));
        }
    }

    Ok(())
}

/// Statutory price transparency metadata helper for Payment Links under EU Directive 2019/2161.
///
/// Facilitates encoding and decoding statutory price reduction baselines into Stripe `metadata`
/// key-value pairs (`HashMap<String, String>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmnibusPriceTransparencyMetadata {
    /// Regular retail catalog price in minor currency units.
    pub regular_price: i64,
    /// Announced promotional/discounted price in minor currency units.
    pub announced_discounted_price: i64,
    /// Lowest price applied in the preceding 30 days in minor currency units.
    pub prior_30d_lowest_price: Option<i64>,
    /// Tax behavior transparency (`inclusive` or `exclusive`).
    pub tax_behavior: Option<TaxBehavior>,
}

impl OmnibusPriceTransparencyMetadata {
    /// Constructs and validates a new [`OmnibusPriceTransparencyMetadata`].
    ///
    /// # Errors
    /// Returns [`OmnibusPriceError::InvalidRequest`] if statutory transparency rules are violated.
    pub fn new(
        regular_price: i64,
        announced_discounted_price: i64,
        prior_30d_lowest_price: Option<i64>,
    ) -> Result<Self, OmnibusPriceError> {
        validate_omnibus_price_transparency(
            regular_price,
            announced_discounted_price,
            prior_30d_lowest_price,
        )?;
        Ok(Self {
            regular_price,
            announced_discounted_price,
            prior_30d_lowest_price,
            tax_behavior: None,
        })
    }

    /// Builder method to specify transparent tax behavior (`inclusive` or `exclusive`).
    ///
    /// # Errors
    /// Returns [`OmnibusPriceError::InvalidRequest`] if `tax_behavior` is `TaxBehavior::Unspecified`.
    pub fn with_tax_behavior(
        mut self,
        tax_behavior: TaxBehavior,
    ) -> Result<Self, OmnibusPriceError> {
        validate_tax_behavior_transparency(tax_behavior)?;
        self.tax_behavior = Some(tax_behavior);
        Ok(self)
    }

    /// Converts this transparency metadata into Stripe metadata key-value pairs.
    #[must_use]
    pub fn to_metadata_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert(
            "omnibus_regular_price".into(),
            self.regular_price.to_string(),
        );
        map.insert(
            "omnibus_discounted_price".into(),
            self.announced_discounted_price.to_string(),
        );
        if let Some(prior) = self.prior_30d_lowest_price {
            map.insert("omnibus_prior_30d_lowest_price".into(), prior.to_string());
        }
        if let Some(tax) = self.tax_behavior {
            map.insert("omnibus_tax_behavior".into(), tax.as_str().into());
        }
        map.insert(
            "statutory_directive".into(),
            "EU_Directive_2019_2161_Article_6a".into(),
        );
        map
    }

    /// Extracts and validates Omnibus price transparency metadata from a Stripe metadata map.
    ///
    /// Returns `Ok(None)` if Omnibus metadata keys are not present in the map.
    ///
    /// # Errors
    /// Returns [`OmnibusPriceError::InvalidRequest`] if the metadata is present but invalid or deceptive.
    pub fn from_metadata_map(
        map: &HashMap<String, String>,
    ) -> Result<Option<Self>, OmnibusPriceError> {
        let regular_str = match map.get("omnibus_regular_price") {
            Some(s) => s,
            None => return Ok(None),
        };
        let discounted_str = match map.get("omnibus_discounted_price") {
            Some(s) => s,
            None => return Ok(None),
        };

        let regular_price = regular_str.parse::<i64>().map_err(|e| {
            OmnibusPriceError::InvalidRequest(format!("Failed to parse omnibus_regular_price: {e}"))
        })?;
        let announced_discounted_price = discounted_str.parse::<i64>().map_err(|e| {
            OmnibusPriceError::InvalidRequest(format!(
                "Failed to parse omnibus_discounted_price: {e}"
            ))
        })?;

        let prior_30d_lowest_price = match map.get("omnibus_prior_30d_lowest_price") {
            Some(s) => Some(s.parse::<i64>().map_err(|e| {
                OmnibusPriceError::InvalidRequest(format!(
                    "Failed to parse omnibus_prior_30d_lowest_price: {e}"
                ))
            })?),
            None => None,
        };

        let tax_behavior = match map.get("omnibus_tax_behavior") {
            Some(s) => Some(s.parse::<TaxBehavior>()?),
            None => None,
        };

        let mut meta = Self::new(
            regular_price,
            announced_discounted_price,
            prior_30d_lowest_price,
        )?;
        if let Some(tb) = tax_behavior {
            meta = meta.with_tax_behavior(tb)?;
        }
        Ok(Some(meta))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliant_discount_with_baseline() {
        // Regular €100, discounted €80, prior lowest €85 (discounted <= prior lowest)
        assert!(validate_omnibus_price_transparency(10000, 8000, Some(8500)).is_ok());

        // Regular €100, discounted €80, prior lowest €80 (discounted == prior lowest)
        assert!(validate_omnibus_price_transparency(10000, 8000, Some(8000)).is_ok());

        // Regular €100, discounted €80, prior lowest €100 (never discounted before)
        assert!(validate_omnibus_price_transparency(10000, 8000, Some(10000)).is_ok());
    }

    #[test]
    fn test_regular_price_no_discount() {
        // Regular €100, no discount (€100), no prior lowest required
        assert!(validate_omnibus_price_transparency(10000, 10000, None).is_ok());

        // Regular €100, no discount (€100), prior lowest provided
        assert!(validate_omnibus_price_transparency(10000, 10000, Some(10000)).is_ok());
    }

    #[test]
    fn test_error_missing_prior_lowest_when_discounted() {
        // Regular €100, discounted €80, None prior lowest
        let err = validate_omnibus_price_transparency(10000, 8000, None).unwrap_err();
        match err {
            OmnibusPriceError::InvalidRequest(msg) => {
                assert!(msg.contains("prior_30d_lowest_price must be specified"));
            }
        }
    }

    #[test]
    fn test_error_deceptive_discount_exceeds_30d_baseline() {
        // Merchant raises regular price to €120, announces discount to €90,
        // but lowest price in past 30 days was €80. Advertised €90 > baseline €80.
        let err = validate_omnibus_price_transparency(12000, 9000, Some(8000)).unwrap_err();
        match err {
            OmnibusPriceError::InvalidRequest(msg) => {
                assert!(msg.contains("exceeds the prior 30-day lowest baseline price"));
                assert!(msg.contains("deceptive discounting"));
            }
        }
    }

    #[test]
    fn test_error_discounted_price_exceeds_regular() {
        let err = validate_omnibus_price_transparency(5000, 6000, Some(5000)).unwrap_err();
        match err {
            OmnibusPriceError::InvalidRequest(msg) => {
                assert!(msg.contains("cannot exceed regular_price"));
            }
        }
    }

    #[test]
    fn test_error_non_positive_amounts() {
        assert!(validate_omnibus_price_transparency(0, 1000, Some(1000)).is_err());
        assert!(validate_omnibus_price_transparency(-500, 1000, Some(1000)).is_err());
        assert!(validate_omnibus_price_transparency(1000, 0, Some(1000)).is_err());
        assert!(validate_omnibus_price_transparency(1000, -200, Some(1000)).is_err());
        assert!(validate_omnibus_price_transparency(1000, 800, Some(0)).is_err());
        assert!(validate_omnibus_price_transparency(1000, 800, Some(-100)).is_err());
    }

    #[test]
    fn test_omnibus_metadata_roundtrip() {
        let meta = OmnibusPriceTransparencyMetadata::new(10000, 7500, Some(8000)).unwrap();
        let map = meta.to_metadata_map();

        assert_eq!(map.get("omnibus_regular_price").unwrap(), "10000");
        assert_eq!(map.get("omnibus_discounted_price").unwrap(), "7500");
        assert_eq!(map.get("omnibus_prior_30d_lowest_price").unwrap(), "8000");
        assert_eq!(
            map.get("statutory_directive").unwrap(),
            "EU_Directive_2019_2161_Article_6a"
        );

        let parsed = OmnibusPriceTransparencyMetadata::from_metadata_map(&map)
            .unwrap()
            .unwrap();
        assert_eq!(parsed, meta);
    }

    #[test]
    fn test_omnibus_metadata_empty_when_no_keys() {
        let map = HashMap::new();
        assert!(
            OmnibusPriceTransparencyMetadata::from_metadata_map(&map)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn test_tax_behavior_state_and_parsing() {
        assert_eq!(TaxBehavior::Inclusive.as_str(), "inclusive");
        assert_eq!(TaxBehavior::Exclusive.as_str(), "exclusive");
        assert_eq!(TaxBehavior::Unspecified.as_str(), "unspecified");

        assert!(TaxBehavior::Inclusive.is_specified());
        assert!(TaxBehavior::Exclusive.is_specified());
        assert!(!TaxBehavior::Unspecified.is_specified());

        assert!(TaxBehavior::Inclusive.is_inclusive());
        assert!(!TaxBehavior::Inclusive.is_exclusive());
        assert!(TaxBehavior::Exclusive.is_exclusive());
        assert!(!TaxBehavior::Exclusive.is_inclusive());

        assert_eq!(
            "inclusive".parse::<TaxBehavior>().unwrap(),
            TaxBehavior::Inclusive
        );
        assert_eq!(
            "exclusive".parse::<TaxBehavior>().unwrap(),
            TaxBehavior::Exclusive
        );
        assert_eq!(
            "unspecified".parse::<TaxBehavior>().unwrap(),
            TaxBehavior::Unspecified
        );
        assert!("invalid".parse::<TaxBehavior>().is_err());
    }

    #[test]
    fn test_tax_behavior_transparency_validation() {
        assert!(validate_tax_behavior_transparency(TaxBehavior::Inclusive).is_ok());
        assert!(validate_tax_behavior_transparency(TaxBehavior::Exclusive).is_ok());

        let err = validate_tax_behavior_transparency(TaxBehavior::Unspecified).unwrap_err();
        match err {
            OmnibusPriceError::InvalidRequest(msg) => {
                assert!(msg.contains("transparent tax breakdown requires tax_behavior"));
            }
        }
    }

    #[test]
    fn test_omnibus_tax_breakdown_inclusive() {
        // Gross €120.00 (12000 cents) with 20% VAT (2000 basis points)
        // net = (12000 * 10000 + 6000) / 12000 = 10000 cents (€100.00)
        // tax = 12000 - 10000 = 2000 cents (€20.00)
        let breakdown =
            OmnibusTaxBreakdown::calculate(12000, TaxBehavior::Inclusive, 2000).unwrap();
        assert_eq!(breakdown.tax_behavior, TaxBehavior::Inclusive);
        assert_eq!(breakdown.gross_amount, 12000);
        assert_eq!(breakdown.net_amount, 10000);
        assert_eq!(breakdown.tax_amount, 2000);
        assert_eq!(breakdown.tax_rate_basis_points, 2000);
        assert_eq!(
            breakdown.net_amount + breakdown.tax_amount,
            breakdown.gross_amount
        );

        // German 19% VAT (1900 basis points) on €119.00 (11900 cents)
        let de_breakdown =
            OmnibusTaxBreakdown::calculate(11900, TaxBehavior::Inclusive, 1900).unwrap();
        assert_eq!(de_breakdown.gross_amount, 11900);
        assert_eq!(de_breakdown.net_amount, 10000);
        assert_eq!(de_breakdown.tax_amount, 1900);
    }

    #[test]
    fn test_omnibus_tax_breakdown_exclusive() {
        // Net €100.00 (10000 cents) with 20% VAT (2000 basis points)
        // tax = (10000 * 2000 + 5000) / 10000 = 2000 cents (€20.00)
        // gross = 10000 + 2000 = 12000 cents (€120.00)
        let breakdown =
            OmnibusTaxBreakdown::calculate(10000, TaxBehavior::Exclusive, 2000).unwrap();
        assert_eq!(breakdown.tax_behavior, TaxBehavior::Exclusive);
        assert_eq!(breakdown.net_amount, 10000);
        assert_eq!(breakdown.tax_amount, 2000);
        assert_eq!(breakdown.gross_amount, 12000);
        assert_eq!(
            breakdown.net_amount + breakdown.tax_amount,
            breakdown.gross_amount
        );
    }

    #[test]
    fn test_omnibus_tax_breakdown_errors() {
        // Unspecified tax behavior
        assert!(OmnibusTaxBreakdown::calculate(10000, TaxBehavior::Unspecified, 2000).is_err());
        // Non-positive amount
        assert!(OmnibusTaxBreakdown::calculate(0, TaxBehavior::Inclusive, 2000).is_err());
        assert!(OmnibusTaxBreakdown::calculate(-100, TaxBehavior::Inclusive, 2000).is_err());
        // Tax rate > 10,000 basis points (> 100%)
        assert!(OmnibusTaxBreakdown::calculate(10000, TaxBehavior::Inclusive, 15000).is_err());
    }

    #[test]
    fn test_omnibus_metadata_with_tax_behavior_roundtrip() {
        let meta = OmnibusPriceTransparencyMetadata::new(10000, 8000, Some(8500))
            .unwrap()
            .with_tax_behavior(TaxBehavior::Inclusive)
            .unwrap();
        let map = meta.to_metadata_map();

        assert_eq!(map.get("omnibus_regular_price").unwrap(), "10000");
        assert_eq!(map.get("omnibus_discounted_price").unwrap(), "8000");
        assert_eq!(map.get("omnibus_prior_30d_lowest_price").unwrap(), "8500");
        assert_eq!(map.get("omnibus_tax_behavior").unwrap(), "inclusive");

        let parsed = OmnibusPriceTransparencyMetadata::from_metadata_map(&map)
            .unwrap()
            .unwrap();
        assert_eq!(parsed, meta);
        assert_eq!(parsed.tax_behavior, Some(TaxBehavior::Inclusive));
    }
}
