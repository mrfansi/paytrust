// T040: LineItem model with subtotal calculation
// Implements FR-001 (line items), FR-005 (quantity/price), FR-007 (currency per line)
//
// A line item represents a single product or service in an invoice.
// Each line item maintains its own currency and calculates its subtotal
// based on quantity × unit_price with proper rounding per currency scale.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::core::{AppError, Currency, Result};

/// Represents a single line item in an invoice
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LineItem {
    /// Unique identifier for the line item
    #[serde(skip_deserializing)]
    pub id: Option<String>,
    
    /// Foreign key to the invoice
    #[serde(skip_deserializing)]
    pub invoice_id: Option<String>,
    
    /// Description of the product or service
    pub description: String,
    
    /// Quantity of items
    #[sqlx(try_from = "i32")]
    pub quantity: i32,
    
    /// Price per unit
    #[sqlx(try_from = "rust_decimal::Decimal")]
    pub unit_price: Decimal,
    
    /// Currency for this line item (IDR, MYR, USD)
    #[sqlx(try_from = "String")]
    pub currency: Currency,
    
    /// Calculated subtotal (quantity × unit_price, rounded per currency)
    #[serde(skip_deserializing)]
    #[sqlx(try_from = "rust_decimal::Decimal")]
    pub subtotal: Option<Decimal>,
}

impl LineItem {
    /// Create a new line item with validation
    /// 
    /// # Arguments
    /// * `description` - Product/service description (max 255 chars)
    /// * `quantity` - Must be positive
    /// * `unit_price` - Must be non-negative
    /// * `currency` - Currency for this line item
    /// 
    /// # Returns
    /// * `Result<Self>` - Validated line item or error
    pub fn new(
        description: String,
        quantity: i32,
        unit_price: Decimal,
        currency: Currency,
    ) -> Result<Self> {
        // FR-001: Validate line item data
        Self::validate_description(&description)?;
        Self::validate_quantity(quantity)?;
        Self::validate_unit_price(unit_price)?;
        
        let mut line_item = Self {
            id: None,
            invoice_id: None,
            description,
            quantity,
            unit_price,
            currency,
            subtotal: None,
        };
        
        // Calculate subtotal immediately
        line_item.calculate_subtotal();
        
        Ok(line_item)
    }
    
    /// Calculate subtotal for this line item
    /// 
    /// Formula: subtotal = quantity × unit_price
    /// Rounding: Per currency scale (IDR=0, MYR/USD=2 decimals)
    /// 
    /// # Updates
    /// * Sets `self.subtotal` to calculated value
    pub fn calculate_subtotal(&mut self) {
        let raw_subtotal = Decimal::from(self.quantity) * self.unit_price;
        self.subtotal = Some(self.currency.round(raw_subtotal));
    }
    
    /// Get the subtotal, calculating if not set
    pub fn get_subtotal(&mut self) -> Decimal {
        if self.subtotal.is_none() {
            self.calculate_subtotal();
        }
        self.subtotal.unwrap_or(Decimal::ZERO)
    }
    
    /// Validate description
    fn validate_description(description: &str) -> Result<()> {
        if description.trim().is_empty() {
            return Err(AppError::validation("Line item description cannot be empty"));
        }
        
        if description.len() > 255 {
            return Err(AppError::validation(
                "Line item description cannot exceed 255 characters"
            ));
        }
        
        Ok(())
    }
    
    /// Validate quantity (must be positive)
    fn validate_quantity(quantity: i32) -> Result<()> {
        if quantity <= 0 {
            return Err(AppError::validation(
                format!("Quantity must be positive, got: {}", quantity)
            ));
        }
        
        Ok(())
    }
    
    /// Validate unit price (must be non-negative)
    fn validate_unit_price(unit_price: Decimal) -> Result<()> {
        if unit_price < Decimal::ZERO {
            return Err(AppError::validation(
                format!("Unit price must be non-negative, got: {}", unit_price)
            ));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    
    #[test]
    fn test_line_item_creation_valid() {
        let line_item = LineItem::new(
            "Test Product".to_string(),
            3,
            Decimal::from(1000),
            Currency::IDR,
        );
        
        assert!(line_item.is_ok());
        let mut item = line_item.unwrap();
        assert_eq!(item.description, "Test Product");
        assert_eq!(item.quantity, 3);
        assert_eq!(item.get_subtotal(), Decimal::from(3000));
    }
    
    #[test]
    fn test_line_item_subtotal_calculation_idr() {
        let mut line_item = LineItem::new(
            "Product".to_string(),
            5,
            Decimal::from_str("1500.67").unwrap(),
            Currency::IDR,
        ).unwrap();
        
        // 5 * 1500.67 = 7503.35, should round to 7503 for IDR
        assert_eq!(line_item.get_subtotal(), Decimal::from(7503));
    }
    
    #[test]
    fn test_line_item_subtotal_calculation_myr() {
        let mut line_item = LineItem::new(
            "Service".to_string(),
            7,
            Decimal::from_str("12.345").unwrap(),
            Currency::MYR,
        ).unwrap();
        
        // 7 * 12.345 = 86.415, should round to 86.42 for MYR
        assert_eq!(line_item.get_subtotal(), Decimal::from_str("86.42").unwrap());
    }
    
    #[test]
    fn test_line_item_validation_empty_description() {
        let result = LineItem::new(
            "".to_string(),
            1,
            Decimal::from(100),
            Currency::USD,
        );
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("description cannot be empty"));
    }
    
    #[test]
    fn test_line_item_validation_negative_quantity() {
        let result = LineItem::new(
            "Product".to_string(),
            -1,
            Decimal::from(100),
            Currency::USD,
        );
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Quantity must be positive"));
    }
    
    #[test]
    fn test_line_item_validation_negative_price() {
        let result = LineItem::new(
            "Product".to_string(),
            1,
            Decimal::from(-100),
            Currency::USD,
        );
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unit price must be non-negative"));
    }
}
