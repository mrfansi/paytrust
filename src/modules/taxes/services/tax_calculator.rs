use rust_decimal::Decimal;
use crate::core::error::AppError;

/// TaxCalculator handles per-line-item tax calculations (FR-057, FR-058)
pub struct TaxCalculator;

impl TaxCalculator {
    pub fn new() -> Self {
        Self
    }

    /// Calculate tax amount for a line item
    /// FR-057: Each line item has own tax_rate
    /// FR-058: tax_amount = subtotal × tax_rate
    pub fn calculate_tax(&self, subtotal: Decimal, tax_rate: Decimal) -> Result<Decimal, AppError> {
        // Validate tax rate first
        self.validate_tax_rate(tax_rate)?;
        
        // FR-058: tax_amount = subtotal × tax_rate
        let tax_amount = subtotal * tax_rate;
        
        Ok(tax_amount)
    }

    /// Validate tax rate is within acceptable range (0-1.0) with max 4 decimal places
    /// FR-064a: tax_rate >= 0 and <= 1.0, max 4 decimal places
    pub fn validate_tax_rate(&self, tax_rate: Decimal) -> Result<(), AppError> {
        // Check if tax_rate is negative
        if tax_rate < Decimal::ZERO {
            return Err(AppError::Validation(
                "Tax rate cannot be negative".to_string()
            ));
        }
        
        // Check if tax_rate is above 1.0 (100%)
        if tax_rate > Decimal::ONE {
            return Err(AppError::Validation(
                "Tax rate cannot exceed 1.0 (100%)".to_string()
            ));
        }
        
        // Check decimal places (max 4)
        // Convert to string and check decimal places
        let tax_rate_str = tax_rate.to_string();
        if let Some(decimal_pos) = tax_rate_str.find('.') {
            let decimal_places = tax_rate_str.len() - decimal_pos - 1;
            if decimal_places > 4 {
                return Err(AppError::Validation(
                    "Tax rate cannot have more than 4 decimal places".to_string()
                ));
            }
        }
        
        Ok(())
    }
}

impl Default for TaxCalculator {
    fn default() -> Self {
        Self::new()
    }
}
