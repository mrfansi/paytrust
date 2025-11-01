//! Tax model and related types
//!
//! Represents tax configuration with rate percentage and category.
//! Used for calculating taxes on invoice line items.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Tax rate configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tax {
    /// Unique identifier for the tax rate
    pub id: String,
    
    /// Tax category (e.g., "VAT", "GST", "Sales Tax")
    pub category: String,
    
    /// Tax rate as a decimal (e.g., 0.10 for 10%)
    pub rate: Decimal,
    
    /// Currency this tax applies to (IDR, MYR, USD)
    pub currency: String,
    
    /// Date from which this rate is effective (YYYY-MM-DD)
    pub effective_from: String,
    
    /// Whether this tax rate is currently active
    pub is_active: bool,
    
    /// When this tax rate was created
    pub created_at: String,
    
    /// When this tax rate was last updated
    pub updated_at: String,
}

impl Tax {
    /// Create a new tax rate configuration
    pub fn new(
        id: String,
        category: String,
        rate: Decimal,
        currency: String,
        effective_from: String,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        
        Self {
            id,
            category,
            rate,
            currency,
            effective_from,
            is_active: true,
            created_at: now.clone(),
            updated_at: now,
        }
    }
    
    /// Validate tax rate is within acceptable range (0-100%)
    pub fn validate_rate(&self) -> Result<(), String> {
        if self.rate < Decimal::ZERO {
            return Err("Tax rate cannot be negative".to_string());
        }
        
        if self.rate > Decimal::ONE {
            return Err("Tax rate cannot exceed 100%".to_string());
        }
        
        Ok(())
    }
    
    /// Check if tax rate is valid for a given currency
    pub fn is_valid_for_currency(&self, currency: &str) -> bool {
        self.currency == currency && self.is_active
    }
}

/// Tax category enum for common tax types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaxCategory {
    /// Value Added Tax
    VAT,
    
    /// Goods and Services Tax
    GST,
    
    /// Sales Tax
    SalesTax,
    
    /// Service Tax
    ServiceTax,
    
    /// Custom tax category
    Other,
}

impl TaxCategory {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            TaxCategory::VAT => "VAT",
            TaxCategory::GST => "GST",
            TaxCategory::SalesTax => "Sales Tax",
            TaxCategory::ServiceTax => "Service Tax",
            TaxCategory::Other => "Other",
        }
    }
}

impl std::fmt::Display for TaxCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_new_tax() {
        let tax = Tax::new(
            "tax-001".to_string(),
            "VAT".to_string(),
            Decimal::from_str("0.10").unwrap(),
            "IDR".to_string(),
            "2025-01-01".to_string(),
        );

        assert_eq!(tax.id, "tax-001");
        assert_eq!(tax.category, "VAT");
        assert_eq!(tax.rate, Decimal::from_str("0.10").unwrap());
        assert_eq!(tax.currency, "IDR");
        assert!(tax.is_active);
    }

    #[test]
    fn test_validate_rate_valid() {
        let tax = Tax::new(
            "tax-001".to_string(),
            "VAT".to_string(),
            Decimal::from_str("0.10").unwrap(),
            "IDR".to_string(),
            "2025-01-01".to_string(),
        );

        assert!(tax.validate_rate().is_ok());
    }

    #[test]
    fn test_validate_rate_negative() {
        let mut tax = Tax::new(
            "tax-001".to_string(),
            "VAT".to_string(),
            Decimal::from_str("0.10").unwrap(),
            "IDR".to_string(),
            "2025-01-01".to_string(),
        );
        
        tax.rate = Decimal::from_str("-0.05").unwrap();
        
        assert!(tax.validate_rate().is_err());
        assert_eq!(
            tax.validate_rate().unwrap_err(),
            "Tax rate cannot be negative"
        );
    }

    #[test]
    fn test_validate_rate_exceeds_100_percent() {
        let mut tax = Tax::new(
            "tax-001".to_string(),
            "VAT".to_string(),
            Decimal::from_str("0.10").unwrap(),
            "IDR".to_string(),
            "2025-01-01".to_string(),
        );
        
        tax.rate = Decimal::from_str("1.5").unwrap();
        
        assert!(tax.validate_rate().is_err());
        assert_eq!(
            tax.validate_rate().unwrap_err(),
            "Tax rate cannot exceed 100%"
        );
    }

    #[test]
    fn test_is_valid_for_currency() {
        let tax = Tax::new(
            "tax-001".to_string(),
            "VAT".to_string(),
            Decimal::from_str("0.10").unwrap(),
            "IDR".to_string(),
            "2025-01-01".to_string(),
        );

        assert!(tax.is_valid_for_currency("IDR"));
        assert!(!tax.is_valid_for_currency("MYR"));
    }

    #[test]
    fn test_is_valid_for_currency_inactive() {
        let mut tax = Tax::new(
            "tax-001".to_string(),
            "VAT".to_string(),
            Decimal::from_str("0.10").unwrap(),
            "IDR".to_string(),
            "2025-01-01".to_string(),
        );
        
        tax.is_active = false;

        assert!(!tax.is_valid_for_currency("IDR"));
    }

    #[test]
    fn test_tax_category_as_str() {
        assert_eq!(TaxCategory::VAT.as_str(), "VAT");
        assert_eq!(TaxCategory::GST.as_str(), "GST");
        assert_eq!(TaxCategory::SalesTax.as_str(), "Sales Tax");
        assert_eq!(TaxCategory::ServiceTax.as_str(), "Service Tax");
        assert_eq!(TaxCategory::Other.as_str(), "Other");
    }

    #[test]
    fn test_tax_category_display() {
        assert_eq!(format!("{}", TaxCategory::VAT), "VAT");
        assert_eq!(format!("{}", TaxCategory::GST), "GST");
    }
}
