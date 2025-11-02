use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported currencies with specific decimal handling
/// IDR: scale=0 (no decimals), MYR/USD: scale=2 (2 decimals)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR(3)", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Currency {
    #[serde(rename = "IDR")]
    IDR,
    #[serde(rename = "MYR")]
    MYR,
    #[serde(rename = "USD")]
    USD,
}

impl Currency {
    /// Get the decimal scale for this currency
    /// IDR: 0 decimals, MYR/USD: 2 decimals
    pub fn scale(&self) -> u32 {
        match self {
            Currency::IDR => 0,
            Currency::MYR | Currency::USD => 2,
        }
    }

    /// Round amount to currency-specific precision
    pub fn round(&self, amount: Decimal) -> Decimal {
        amount.round_dp(self.scale())
    }

    /// Validate that amount has correct decimal places for currency
    pub fn validate_precision(&self, amount: Decimal) -> bool {
        amount.scale() <= self.scale()
    }

    /// Format amount for display with currency symbol
    pub fn format(&self, amount: Decimal) -> String {
        let rounded = self.round(amount);
        match self {
            Currency::IDR => format!("IDR {}", rounded),
            Currency::MYR => format!("MYR {:.2}", rounded),
            Currency::USD => format!("USD {:.2}", rounded),
        }
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Currency::IDR => write!(f, "IDR"),
            Currency::MYR => write!(f, "MYR"),
            Currency::USD => write!(f, "USD"),
        }
    }
}

impl std::str::FromStr for Currency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "IDR" => Ok(Currency::IDR),
            "MYR" => Ok(Currency::MYR),
            "USD" => Ok(Currency::USD),
            _ => Err(format!("Invalid currency: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_idr_scale() {
        assert_eq!(Currency::IDR.scale(), 0);
    }

    #[test]
    fn test_myr_scale() {
        assert_eq!(Currency::MYR.scale(), 2);
    }

    #[test]
    fn test_usd_scale() {
        assert_eq!(Currency::USD.scale(), 2);
    }

    #[test]
    fn test_idr_rounding() {
        let amount = dec!(1000000.567);
        let rounded = Currency::IDR.round(amount);
        assert_eq!(rounded, dec!(1000001)); // Rounds to nearest whole number
    }

    #[test]
    fn test_myr_rounding() {
        let amount = dec!(1000.567);
        let rounded = Currency::MYR.round(amount);
        assert_eq!(rounded, dec!(1000.57)); // Rounds to 2 decimal places
    }

    #[test]
    fn test_idr_precision_validation() {
        assert!(Currency::IDR.validate_precision(dec!(1000000)));
        assert!(!Currency::IDR.validate_precision(dec!(1000000.50)));
    }

    #[test]
    fn test_myr_precision_validation() {
        assert!(Currency::MYR.validate_precision(dec!(1000.50)));
        assert!(Currency::MYR.validate_precision(dec!(1000)));
        assert!(!Currency::MYR.validate_precision(dec!(1000.567)));
    }

    #[test]
    fn test_currency_formatting() {
        assert_eq!(Currency::IDR.format(dec!(1000000)), "IDR 1000000");
        assert_eq!(Currency::MYR.format(dec!(1000.50)), "MYR 1000.50");
        assert_eq!(Currency::USD.format(dec!(1000.50)), "USD 1000.50");
    }

    #[test]
    fn test_currency_from_str() {
        assert_eq!("IDR".parse::<Currency>().unwrap(), Currency::IDR);
        assert_eq!("MYR".parse::<Currency>().unwrap(), Currency::MYR);
        assert_eq!("USD".parse::<Currency>().unwrap(), Currency::USD);
        assert_eq!("idr".parse::<Currency>().unwrap(), Currency::IDR); // Case insensitive
        assert!("EUR".parse::<Currency>().is_err());
    }
}
