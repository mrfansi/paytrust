use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported currencies with their decimal precision rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR(3)", rename_all = "UPPERCASE")]
#[serde(rename_all = "UPPERCASE")]
pub enum Currency {
    /// Indonesian Rupiah (no decimal places)
    IDR,
    /// Malaysian Ringgit (2 decimal places)
    MYR,
    /// US Dollar (2 decimal places)
    USD,
}

impl Currency {
    /// Returns the decimal scale for this currency
    /// - IDR: 0 (no decimals)
    /// - MYR/USD: 2 (2 decimal places)
    pub fn scale(&self) -> u32 {
        match self {
            Currency::IDR => 0,
            Currency::MYR | Currency::USD => 2,
        }
    }

    /// Rounds a decimal value to the appropriate scale for this currency
    pub fn round(&self, amount: Decimal) -> Decimal {
        amount.round_dp(self.scale())
    }

    /// Validates that a decimal value has the correct scale for this currency
    pub fn validate_amount(&self, amount: Decimal) -> Result<(), String> {
        let scale = amount.scale();
        let expected_scale = self.scale();

        if scale > expected_scale {
            return Err(format!(
                "{} amounts must have at most {} decimal places, got {}",
                self, expected_scale, scale
            ));
        }

        if amount < Decimal::ZERO {
            return Err(format!("{} amount cannot be negative", self));
        }

        Ok(())
    }

    /// Returns the smallest unit for this currency
    pub fn smallest_unit(&self) -> Decimal {
        match self {
            Currency::IDR => Decimal::ONE,
            Currency::MYR | Currency::USD => Decimal::new(1, 2), // 0.01
        }
    }

    /// Formats an amount for display with the correct decimal places
    pub fn format_amount(&self, amount: Decimal) -> String {
        let scale = self.scale();
        if scale == 0 {
            format!("{} {}", self, amount.round_dp(0))
        } else {
            format!("{} {:.width$}", self, amount, width = scale as usize)
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

impl TryFrom<String> for Currency {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl TryFrom<&str> for Currency {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_scale() {
        assert_eq!(Currency::IDR.scale(), 0);
        assert_eq!(Currency::MYR.scale(), 2);
        assert_eq!(Currency::USD.scale(), 2);
    }

    #[test]
    fn test_currency_rounding() {
        // IDR (0 decimal places): 1000.50 rounds to 1000 (banker's rounding)
        assert_eq!(
            Currency::IDR.round(Decimal::new(100050, 2)),
            Decimal::new(1000, 0)
        );
        // MYR (2 decimal places): 10.0055 rounds to 10.01 (banker's rounding)
        assert_eq!(
            Currency::MYR.round(Decimal::new(100055, 4)),
            Decimal::new(1001, 2)
        );
    }

    #[test]
    fn test_currency_validation() {
        assert!(Currency::IDR
            .validate_amount(Decimal::new(1000000, 0))
            .is_ok());
        assert!(Currency::MYR
            .validate_amount(Decimal::new(100050, 2))
            .is_ok());

        // IDR should not accept decimals
        assert!(Currency::IDR
            .validate_amount(Decimal::new(100050, 2))
            .is_err());

        // Negative amounts should be rejected
        assert!(Currency::IDR
            .validate_amount(Decimal::new(-1000, 0))
            .is_err());
    }

    #[test]
    fn test_currency_formatting() {
        assert_eq!(
            Currency::IDR.format_amount(Decimal::new(1000000, 0)),
            "IDR 1000000"
        );
        assert_eq!(
            Currency::MYR.format_amount(Decimal::new(100050, 2)),
            "MYR 1000.50"
        );
    }
}
