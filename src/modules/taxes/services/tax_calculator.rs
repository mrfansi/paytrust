//! Tax calculation service
//!
//! Implements per-line-item tax calculation following FR-057 and FR-058.
//! Taxes are calculated on subtotal only (FR-055), not including service fees.

use rust_decimal::Decimal;

/// Tax calculator for invoice line items
pub struct TaxCalculator;

impl TaxCalculator {
    /// Calculate tax for a single line item
    ///
    /// Formula: tax_amount = subtotal × tax_rate (rounded to 2 decimal places)
    ///
    /// # Arguments
    /// * `subtotal` - The line item subtotal (quantity × unit_price)
    /// * `tax_rate` - The tax rate as a decimal (e.g., 0.10 for 10%)
    ///
    /// # Returns
    /// The calculated tax amount, rounded to 2 decimal places for currency
    ///
    /// # Examples
    /// ```
    /// use rust_decimal::Decimal;
    /// use std::str::FromStr;
    /// use paytrust::modules::taxes::services::TaxCalculator;
    ///
    /// let subtotal = Decimal::from_str("100000").unwrap();
    /// let tax_rate = Decimal::from_str("0.10").unwrap();
    /// let tax = TaxCalculator::calculate_line_item_tax(subtotal, tax_rate);
    ///
    /// assert_eq!(tax, Decimal::from_str("10000").unwrap());
    /// ```
    pub fn calculate_line_item_tax(subtotal: Decimal, tax_rate: Decimal) -> Decimal {
        let tax = subtotal * tax_rate;
        tax.round_dp(2)
    }

    /// Calculate total tax for an invoice from multiple line items
    ///
    /// # Arguments
    /// * `line_items` - Vector of (subtotal, tax_rate) tuples
    ///
    /// # Returns
    /// The sum of all line item taxes, rounded to 2 decimal places
    ///
    /// # Examples
    /// ```
    /// use rust_decimal::Decimal;
    /// use std::str::FromStr;
    /// use paytrust::modules::taxes::services::TaxCalculator;
    ///
    /// let line_items = vec![
    ///     (Decimal::from_str("100000").unwrap(), Decimal::from_str("0.10").unwrap()),
    ///     (Decimal::from_str("50000").unwrap(), Decimal::from_str("0.11").unwrap()),
    /// ];
    ///
    /// let total_tax = TaxCalculator::calculate_invoice_tax(line_items);
    /// assert_eq!(total_tax, Decimal::from_str("15500").unwrap()); // 10000 + 5500
    /// ```
    pub fn calculate_invoice_tax(line_items: Vec<(Decimal, Decimal)>) -> Decimal {
        line_items
            .iter()
            .map(|(subtotal, tax_rate)| Self::calculate_line_item_tax(*subtotal, *tax_rate))
            .sum::<Decimal>()
            .round_dp(2)
    }

    /// Calculate invoice total including taxes and service fee
    ///
    /// Formula: total = subtotal + tax_total + service_fee (FR-056)
    /// Tax calculated on subtotal only, not including service fee (FR-055)
    ///
    /// # Arguments
    /// * `subtotal` - Sum of all line item subtotals
    /// * `tax_total` - Sum of all line item taxes
    /// * `service_fee` - Payment gateway service fee
    ///
    /// # Returns
    /// The final invoice total
    ///
    /// # Examples
    /// ```
    /// use rust_decimal::Decimal;
    /// use std::str::FromStr;
    /// use paytrust::modules::taxes::services::TaxCalculator;
    ///
    /// let subtotal = Decimal::from_str("100000").unwrap();
    /// let tax_total = Decimal::from_str("10000").unwrap();
    /// let service_fee = Decimal::from_str("5100").unwrap();
    ///
    /// let total = TaxCalculator::calculate_invoice_total(subtotal, tax_total, service_fee);
    /// assert_eq!(total, Decimal::from_str("115100").unwrap());
    /// ```
    pub fn calculate_invoice_total(
        subtotal: Decimal,
        tax_total: Decimal,
        service_fee: Decimal,
    ) -> Decimal {
        (subtotal + tax_total + service_fee).round_dp(2)
    }

    /// Validate that a tax rate is within acceptable range (0-100%)
    ///
    /// # Arguments
    /// * `tax_rate` - The tax rate to validate
    ///
    /// # Returns
    /// `Ok(())` if valid, `Err(String)` with error message if invalid
    pub fn validate_tax_rate(tax_rate: Decimal) -> Result<(), String> {
        if tax_rate < Decimal::ZERO {
            return Err("Tax rate cannot be negative".to_string());
        }

        if tax_rate > Decimal::ONE {
            return Err("Tax rate cannot exceed 100%".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_calculate_line_item_tax_10_percent() {
        let subtotal = Decimal::from_str("100000").unwrap();
        let tax_rate = Decimal::from_str("0.10").unwrap();
        
        let tax = TaxCalculator::calculate_line_item_tax(subtotal, tax_rate);
        
        assert_eq!(tax, Decimal::from_str("10000").unwrap());
    }

    #[test]
    fn test_calculate_line_item_tax_11_percent() {
        let subtotal = Decimal::from_str("100000").unwrap();
        let tax_rate = Decimal::from_str("0.11").unwrap();
        
        let tax = TaxCalculator::calculate_line_item_tax(subtotal, tax_rate);
        
        assert_eq!(tax, Decimal::from_str("11000").unwrap());
    }

    #[test]
    fn test_calculate_line_item_tax_with_rounding() {
        let subtotal = Decimal::from_str("333").unwrap();
        let tax_rate = Decimal::from_str("0.10").unwrap();
        
        let tax = TaxCalculator::calculate_line_item_tax(subtotal, tax_rate);
        
        assert_eq!(tax, Decimal::from_str("33.30").unwrap());
    }

    #[test]
    fn test_calculate_line_item_tax_zero_rate() {
        let subtotal = Decimal::from_str("100000").unwrap();
        let tax_rate = Decimal::ZERO;
        
        let tax = TaxCalculator::calculate_line_item_tax(subtotal, tax_rate);
        
        assert_eq!(tax, Decimal::ZERO);
    }

    #[test]
    fn test_calculate_invoice_tax_multiple_items() {
        let line_items = vec![
            (Decimal::from_str("100000").unwrap(), Decimal::from_str("0.10").unwrap()),
            (Decimal::from_str("50000").unwrap(), Decimal::from_str("0.11").unwrap()),
        ];

        let total_tax = TaxCalculator::calculate_invoice_tax(line_items);
        
        // 100000 × 0.10 = 10000
        // 50000 × 0.11 = 5500
        // Total = 15500
        assert_eq!(total_tax, Decimal::from_str("15500").unwrap());
    }

    #[test]
    fn test_calculate_invoice_tax_different_rates() {
        let line_items = vec![
            (Decimal::from_str("100000").unwrap(), Decimal::from_str("0.10").unwrap()),
            (Decimal::from_str("50000").unwrap(), Decimal::from_str("0.06").unwrap()),
        ];

        let total_tax = TaxCalculator::calculate_invoice_tax(line_items);
        
        // 100000 × 0.10 = 10000
        // 50000 × 0.06 = 3000
        // Total = 13000
        assert_eq!(total_tax, Decimal::from_str("13000").unwrap());
    }

    #[test]
    fn test_calculate_invoice_total() {
        let subtotal = Decimal::from_str("100000").unwrap();
        let tax_total = Decimal::from_str("10000").unwrap();
        let service_fee = Decimal::from_str("5100").unwrap();

        let total = TaxCalculator::calculate_invoice_total(subtotal, tax_total, service_fee);
        
        assert_eq!(total, Decimal::from_str("115100").unwrap());
    }

    #[test]
    fn test_calculate_invoice_total_without_service_fee() {
        let subtotal = Decimal::from_str("100000").unwrap();
        let tax_total = Decimal::from_str("10000").unwrap();
        let service_fee = Decimal::ZERO;

        let total = TaxCalculator::calculate_invoice_total(subtotal, tax_total, service_fee);
        
        assert_eq!(total, Decimal::from_str("110000").unwrap());
    }

    #[test]
    fn test_validate_tax_rate_valid() {
        let tax_rate = Decimal::from_str("0.10").unwrap();
        assert!(TaxCalculator::validate_tax_rate(tax_rate).is_ok());
    }

    #[test]
    fn test_validate_tax_rate_zero() {
        let tax_rate = Decimal::ZERO;
        assert!(TaxCalculator::validate_tax_rate(tax_rate).is_ok());
    }

    #[test]
    fn test_validate_tax_rate_one_hundred_percent() {
        let tax_rate = Decimal::ONE;
        assert!(TaxCalculator::validate_tax_rate(tax_rate).is_ok());
    }

    #[test]
    fn test_validate_tax_rate_negative() {
        let tax_rate = Decimal::from_str("-0.05").unwrap();
        let result = TaxCalculator::validate_tax_rate(tax_rate);
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Tax rate cannot be negative");
    }

    #[test]
    fn test_validate_tax_rate_exceeds_100_percent() {
        let tax_rate = Decimal::from_str("1.5").unwrap();
        let result = TaxCalculator::validate_tax_rate(tax_rate);
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Tax rate cannot exceed 100%");
    }
}
