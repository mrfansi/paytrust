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
        // TODO: Implement tax calculation logic
        // This is a stub that will make tests fail
        Ok(Decimal::ZERO)
    }

    /// Validate tax rate is within acceptable range (0-1.0) with max 4 decimal places
    /// FR-064a: tax_rate >= 0 and <= 1.0, max 4 decimal places
    pub fn validate_tax_rate(&self, tax_rate: Decimal) -> Result<(), AppError> {
        // TODO: Implement validation logic
        // This is a stub that will make tests fail
        Ok(())
    }
}

impl Default for TaxCalculator {
    fn default() -> Self {
        Self::new()
    }
}
