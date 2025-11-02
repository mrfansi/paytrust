use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::core::error::AppError;

/// LineItem entity representing a product/service entry in an invoice
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LineItem {
    pub id: i64,
    pub invoice_id: i64,
    pub product_name: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub subtotal: Decimal,
    pub tax_rate: Decimal,
    pub tax_category: Option<String>,
    pub tax_amount: Decimal,
    pub created_at: DateTime<Utc>,
}

impl LineItem {
    /// Calculate subtotal from quantity and unit price
    /// Per FR-005: subtotal = quantity × unit_price
    pub fn calculate_subtotal(quantity: Decimal, unit_price: Decimal) -> Decimal {
        quantity * unit_price
    }

    /// Calculate tax amount from subtotal and tax rate
    /// Per FR-058: tax_amount = subtotal × tax_rate
    pub fn calculate_tax_amount(subtotal: Decimal, tax_rate: Decimal) -> Decimal {
        subtotal * tax_rate
    }

    /// Validate line item data
    pub fn validate(
        quantity: Decimal,
        unit_price: Decimal,
        tax_rate: Decimal,
    ) -> Result<(), AppError> {
        // Quantity must be positive
        if quantity <= Decimal::ZERO {
            return Err(AppError::Validation(
                "Quantity must be greater than zero".to_string(),
            ));
        }

        // Unit price must be non-negative
        if unit_price < Decimal::ZERO {
            return Err(AppError::Validation(
                "Unit price cannot be negative".to_string(),
            ));
        }

        // Tax rate must be between 0 and 1 (0% to 100%)
        // Per FR-064a: tax_rate >= 0 and <= 1.0
        if tax_rate < Decimal::ZERO || tax_rate > Decimal::ONE {
            return Err(AppError::Validation(
                "Invalid tax_rate: must be between 0 and 1.0 with max 4 decimal places"
                    .to_string(),
            ));
        }

        // Tax rate must have maximum 4 decimal places
        // Per FR-064a: validate tax_rate has maximum 4 decimal places (0.0001 precision)
        if tax_rate.scale() > 4 {
            return Err(AppError::Validation(
                "Invalid tax_rate: must be between 0 and 1.0 with max 4 decimal places"
                    .to_string(),
            ));
        }

        Ok(())
    }

    /// Create a new line item with calculated values
    pub fn new(
        invoice_id: i64,
        product_name: String,
        quantity: Decimal,
        unit_price: Decimal,
        tax_rate: Decimal,
        tax_category: Option<String>,
    ) -> Result<Self, AppError> {
        // Validate inputs
        Self::validate(quantity, unit_price, tax_rate)?;

        // Calculate derived values
        let subtotal = Self::calculate_subtotal(quantity, unit_price);
        let tax_amount = Self::calculate_tax_amount(subtotal, tax_rate);

        Ok(LineItem {
            id: 0, // Will be set by database
            invoice_id,
            product_name,
            quantity,
            unit_price,
            subtotal,
            tax_rate,
            tax_category,
            tax_amount,
            created_at: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_subtotal() {
        let quantity = Decimal::new(2, 0); // 2
        let unit_price = Decimal::new(500000, 0); // 500,000
        let subtotal = LineItem::calculate_subtotal(quantity, unit_price);
        assert_eq!(subtotal, Decimal::new(1000000, 0)); // 1,000,000
    }

    #[test]
    fn test_calculate_tax_amount() {
        let subtotal = Decimal::new(1000000, 0); // 1,000,000
        let tax_rate = Decimal::new(10, 2); // 0.10 (10%)
        let tax_amount = LineItem::calculate_tax_amount(subtotal, tax_rate);
        assert_eq!(tax_amount, Decimal::new(100000, 0)); // 100,000
    }

    #[test]
    fn test_validate_positive_quantity() {
        let result = LineItem::validate(
            Decimal::ZERO,
            Decimal::new(100, 0),
            Decimal::new(10, 2),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_tax_rate_range() {
        // Valid tax rate
        assert!(LineItem::validate(
            Decimal::new(1, 0),
            Decimal::new(100, 0),
            Decimal::new(10, 2), // 0.10
        )
        .is_ok());

        // Tax rate too high
        assert!(LineItem::validate(
            Decimal::new(1, 0),
            Decimal::new(100, 0),
            Decimal::new(11, 1), // 1.1 (110%)
        )
        .is_err());

        // Negative tax rate
        assert!(LineItem::validate(
            Decimal::new(1, 0),
            Decimal::new(100, 0),
            Decimal::new(-1, 2),
        )
        .is_err());
    }

    #[test]
    fn test_validate_tax_rate_precision() {
        // Valid: 4 decimal places
        assert!(LineItem::validate(
            Decimal::new(1, 0),
            Decimal::new(100, 0),
            Decimal::new(1234, 4), // 0.1234
        )
        .is_ok());

        // Invalid: 5 decimal places
        assert!(LineItem::validate(
            Decimal::new(1, 0),
            Decimal::new(100, 0),
            Decimal::new(12345, 5), // 0.12345
        )
        .is_err());
    }

    #[test]
    fn test_new_line_item() {
        let line_item = LineItem::new(
            1,
            "Test Product".to_string(),
            Decimal::new(2, 0),
            Decimal::new(500000, 0),
            Decimal::new(10, 2),
            None,
        )
        .unwrap();

        assert_eq!(line_item.invoice_id, 1);
        assert_eq!(line_item.product_name, "Test Product");
        assert_eq!(line_item.quantity, Decimal::new(2, 0));
        assert_eq!(line_item.unit_price, Decimal::new(500000, 0));
        assert_eq!(line_item.subtotal, Decimal::new(1000000, 0));
        assert_eq!(line_item.tax_rate, Decimal::new(10, 2));
        assert_eq!(line_item.tax_amount, Decimal::new(100000, 0));
    }
}
