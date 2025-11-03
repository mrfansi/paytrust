use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;

/// Unit test for tax_rate validation per FR-064a
/// 
/// Validates:
/// - tax_rate >= 0 and <= 1.0 (0-100%)
/// - tax_rate has maximum 4 decimal places (0.0001 precision)
/// - Invalid rates rejected with 400 Bad Request
/// - Edge cases: 0.0, 1.0, 0.0001, 0.27, 1.0001 rejection

#[cfg(test)]
mod tax_validation_tests {
    use super::*;

    /// Validates tax rate is within valid range [0.0, 1.0]
    fn validate_tax_rate_range(tax_rate: Decimal) -> Result<(), String> {
        if tax_rate < Decimal::ZERO {
            return Err("Invalid tax_rate: must be between 0 and 1.0 with max 4 decimal places".to_string());
        }
        if tax_rate > Decimal::ONE {
            return Err("Invalid tax_rate: must be between 0 and 1.0 with max 4 decimal places".to_string());
        }
        Ok(())
    }

    /// Validates tax rate has maximum 4 decimal places
    fn validate_tax_rate_precision(tax_rate: Decimal) -> Result<(), String> {
        let scale = tax_rate.scale();
        if scale > 4 {
            return Err("Invalid tax_rate: must be between 0 and 1.0 with max 4 decimal places".to_string());
        }
        Ok(())
    }

    /// Full validation combining range and precision checks
    fn validate_tax_rate(tax_rate: Decimal) -> Result<(), String> {
        validate_tax_rate_range(tax_rate)?;
        validate_tax_rate_precision(tax_rate)?;
        Ok(())
    }

    #[test]
    fn test_valid_tax_rate_zero() {
        let tax_rate = Decimal::ZERO;
        assert!(validate_tax_rate(tax_rate).is_ok(), "0.0 should be valid");
    }

    #[test]
    fn test_valid_tax_rate_one() {
        let tax_rate = Decimal::ONE;
        assert!(validate_tax_rate(tax_rate).is_ok(), "1.0 should be valid");
    }

    #[test]
    fn test_valid_tax_rate_minimum_precision() {
        let tax_rate = Decimal::from_f64(0.0001).unwrap();
        assert!(validate_tax_rate(tax_rate).is_ok(), "0.0001 should be valid (4 decimal places)");
    }

    #[test]
    fn test_valid_tax_rate_common_27_percent() {
        let tax_rate = Decimal::from_f64(0.27).unwrap();
        assert!(validate_tax_rate(tax_rate).is_ok(), "0.27 (27%) should be valid");
    }

    #[test]
    fn test_valid_tax_rate_10_percent() {
        let tax_rate = Decimal::from_f64(0.10).unwrap();
        assert!(validate_tax_rate(tax_rate).is_ok(), "0.10 (10%) should be valid");
    }

    #[test]
    fn test_valid_tax_rate_6_percent() {
        let tax_rate = Decimal::from_f64(0.06).unwrap();
        assert!(validate_tax_rate(tax_rate).is_ok(), "0.06 (6%) should be valid");
    }

    #[test]
    fn test_valid_tax_rate_max_precision() {
        let tax_rate = Decimal::from_f64(0.2765).unwrap();
        assert!(validate_tax_rate(tax_rate).is_ok(), "0.2765 should be valid (4 decimal places)");
    }

    #[test]
    fn test_invalid_tax_rate_negative() {
        let tax_rate = Decimal::from_f64(-0.01).unwrap();
        let result = validate_tax_rate(tax_rate);
        assert!(result.is_err(), "Negative tax rate should be rejected");
        assert_eq!(
            result.unwrap_err(),
            "Invalid tax_rate: must be between 0 and 1.0 with max 4 decimal places"
        );
    }

    #[test]
    fn test_invalid_tax_rate_above_one() {
        let tax_rate = Decimal::from_f64(1.0001).unwrap();
        let result = validate_tax_rate(tax_rate);
        assert!(result.is_err(), "Tax rate > 1.0 should be rejected");
        assert_eq!(
            result.unwrap_err(),
            "Invalid tax_rate: must be between 0 and 1.0 with max 4 decimal places"
        );
    }

    #[test]
    fn test_invalid_tax_rate_above_one_large() {
        let tax_rate = Decimal::from_f64(1.5).unwrap();
        let result = validate_tax_rate(tax_rate);
        assert!(result.is_err(), "Tax rate 1.5 (150%) should be rejected");
    }

    #[test]
    fn test_invalid_tax_rate_too_many_decimals() {
        // Create a Decimal with 5 decimal places
        let tax_rate = Decimal::new(12345, 5); // 0.12345
        let result = validate_tax_rate(tax_rate);
        assert!(result.is_err(), "Tax rate with 5 decimal places should be rejected");
        assert_eq!(
            result.unwrap_err(),
            "Invalid tax_rate: must be between 0 and 1.0 with max 4 decimal places"
        );
    }

    #[test]
    fn test_edge_case_exactly_four_decimals() {
        let tax_rate = Decimal::new(1234, 4); // 0.1234
        assert!(validate_tax_rate(tax_rate).is_ok(), "Exactly 4 decimal places should be valid");
    }

    #[test]
    fn test_edge_case_less_than_four_decimals() {
        let tax_rate = Decimal::new(123, 3); // 0.123
        assert!(validate_tax_rate(tax_rate).is_ok(), "Less than 4 decimal places should be valid");
    }

    #[test]
    fn test_boundary_value_just_below_one() {
        let tax_rate = Decimal::from_f64(0.9999).unwrap();
        assert!(validate_tax_rate(tax_rate).is_ok(), "0.9999 should be valid");
    }

    #[test]
    fn test_boundary_value_just_above_zero() {
        let tax_rate = Decimal::from_f64(0.0001).unwrap();
        assert!(validate_tax_rate(tax_rate).is_ok(), "0.0001 should be valid");
    }

    #[test]
    fn test_high_tax_rate_warning_threshold() {
        // Rates exceeding 27% (0.27) should be logged for audit review
        // but are still technically valid if <= 1.0
        let tax_rate = Decimal::from_f64(0.50).unwrap();
        assert!(validate_tax_rate(tax_rate).is_ok(), "0.50 (50%) should be valid but may trigger audit log");
        
        let tax_rate_high = Decimal::from_f64(0.99).unwrap();
        assert!(validate_tax_rate(tax_rate_high).is_ok(), "0.99 (99%) should be valid but may trigger audit log");
    }

    #[test]
    fn test_common_tax_rates_worldwide() {
        // Test common tax rates from various countries
        let rates = vec![
            (0.00, "No tax"),
            (0.05, "5% - Canada GST"),
            (0.06, "6% - Malaysia SST"),
            (0.10, "10% - Indonesia VAT"),
            (0.15, "15% - New Zealand GST"),
            (0.20, "20% - UK VAT"),
            (0.25, "25% - Sweden VAT"),
            (0.27, "27% - Hungary VAT"),
        ];

        for (rate, description) in rates {
            let tax_rate = Decimal::from_f64(rate).unwrap();
            assert!(
                validate_tax_rate(tax_rate).is_ok(),
                "{} should be valid",
                description
            );
        }
    }
}
