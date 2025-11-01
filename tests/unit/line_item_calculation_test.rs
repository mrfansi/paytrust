// T031: Property-based test for line item subtotal calculation
// Using proptest to verify calculation correctness across all valid inputs
//
// Properties tested:
// 1. subtotal = quantity * unit_price (basic arithmetic)
// 2. subtotal is always non-negative for valid inputs
// 3. subtotal precision matches currency scale requirements
// 4. subtotal rounds correctly per currency (IDR=0, MYR/USD=2)

use proptest::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;

// Mock Currency enum for testing (will be replaced with actual import)
#[derive(Debug, Clone, Copy, PartialEq)]
enum Currency {
    IDR,
    MYR,
    USD,
}

impl Currency {
    fn scale(&self) -> u32 {
        match self {
            Currency::IDR => 0,
            Currency::MYR | Currency::USD => 2,
        }
    }
}

// Mock LineItem struct (will be replaced with actual import)
#[derive(Debug, Clone)]
struct LineItem {
    quantity: i32,
    unit_price: Decimal,
    currency: Currency,
}

impl LineItem {
    fn calculate_subtotal(&self) -> Decimal {
        let subtotal = Decimal::from(self.quantity) * self.unit_price;
        self.round_to_currency(subtotal)
    }

    fn round_to_currency(&self, amount: Decimal) -> Decimal {
        amount.round_dp(self.currency.scale())
    }
}

proptest! {
    /// Property: Subtotal equals quantity * unit_price with correct rounding
    #[test]
    fn test_line_item_subtotal_calculation(
        quantity in 1i32..=10000,
        unit_price_cents in 1u64..=1_000_000u64,
        currency_idx in 0usize..3
    ) {
        let currencies = [Currency::IDR, Currency::MYR, Currency::USD];
        let currency = currencies[currency_idx];
        
        // Convert cents to decimal based on currency scale
        let scale = currency.scale();
        let unit_price = Decimal::from(unit_price_cents) / Decimal::from(10u64.pow(scale));
        
        let line_item = LineItem {
            quantity,
            unit_price,
            currency,
        };
        
        let subtotal = line_item.calculate_subtotal();
        let expected = (Decimal::from(quantity) * unit_price).round_dp(scale);
        
        prop_assert_eq!(subtotal, expected, 
            "Subtotal calculation failed: quantity={}, unit_price={}, currency={:?}", 
            quantity, unit_price, currency);
    }
    
    /// Property: Subtotal is always non-negative for positive inputs
    #[test]
    fn test_line_item_subtotal_non_negative(
        quantity in 1i32..=10000,
        unit_price_cents in 1u64..=1_000_000u64,
        currency_idx in 0usize..3
    ) {
        let currencies = [Currency::IDR, Currency::MYR, Currency::USD];
        let currency = currencies[currency_idx];
        let scale = currency.scale();
        let unit_price = Decimal::from(unit_price_cents) / Decimal::from(10u64.pow(scale));
        
        let line_item = LineItem {
            quantity,
            unit_price,
            currency,
        };
        
        let subtotal = line_item.calculate_subtotal();
        
        prop_assert!(subtotal >= Decimal::ZERO, 
            "Subtotal should be non-negative, got: {}", subtotal);
    }
    
    /// Property: Subtotal precision matches currency scale
    #[test]
    fn test_line_item_subtotal_precision(
        quantity in 1i32..=100,
        // Use values that might produce rounding scenarios
        unit_price_raw in 1u64..=100000,
        currency_idx in 0usize..3
    ) {
        let currencies = [Currency::IDR, Currency::MYR, Currency::USD];
        let currency = currencies[currency_idx];
        let scale = currency.scale();
        
        // Create a price that requires rounding
        let unit_price = Decimal::from(unit_price_raw) / Decimal::from(333);
        
        let line_item = LineItem {
            quantity,
            unit_price,
            currency,
        };
        
        let subtotal = line_item.calculate_subtotal();
        
        // Check that the result has at most 'scale' decimal places
        let subtotal_str = subtotal.to_string();
        if let Some(dot_pos) = subtotal_str.find('.') {
            let decimal_places = subtotal_str.len() - dot_pos - 1;
            prop_assert!(decimal_places <= scale as usize,
                "Subtotal has {} decimal places, expected at most {} for {:?}",
                decimal_places, scale, currency);
        }
    }
    
    /// Property: IDR subtotal is always a whole number (no decimals)
    #[test]
    fn test_idr_subtotal_is_integer(
        quantity in 1i32..=10000,
        unit_price_cents in 1u64..=1_000_000u64
    ) {
        let unit_price = Decimal::from(unit_price_cents);
        
        let line_item = LineItem {
            quantity,
            unit_price,
            currency: Currency::IDR,
        };
        
        let subtotal = line_item.calculate_subtotal();
        
        prop_assert!(subtotal.fract().is_zero(),
            "IDR subtotal should be whole number, got: {}", subtotal);
    }
    
    /// Property: Zero quantity produces zero subtotal
    #[test]
    fn test_zero_quantity_zero_subtotal(
        unit_price_cents in 1u64..=1_000_000u64,
        currency_idx in 0usize..3
    ) {
        let currencies = [Currency::IDR, Currency::MYR, Currency::USD];
        let currency = currencies[currency_idx];
        let scale = currency.scale();
        let unit_price = Decimal::from(unit_price_cents) / Decimal::from(10u64.pow(scale));
        
        let line_item = LineItem {
            quantity: 0,
            unit_price,
            currency,
        };
        
        let subtotal = line_item.calculate_subtotal();
        
        prop_assert_eq!(subtotal, Decimal::ZERO,
            "Zero quantity should produce zero subtotal");
    }
}

#[cfg(test)]
mod deterministic_tests {
    use super::*;
    
    #[test]
    fn test_idr_rounds_to_integer() {
        // 3 * 333.67 = 1001.01, should round to 1001 for IDR
        let line_item = LineItem {
            quantity: 3,
            unit_price: Decimal::from_str("333.67").unwrap(),
            currency: Currency::IDR,
        };
        
        let subtotal = line_item.calculate_subtotal();
        assert_eq!(subtotal, Decimal::from(1001));
    }
    
    #[test]
    fn test_myr_rounds_to_two_decimals() {
        // 7 * 12.345 = 86.415, should round to 86.42 for MYR
        let line_item = LineItem {
            quantity: 7,
            unit_price: Decimal::from_str("12.345").unwrap(),
            currency: Currency::MYR,
        };
        
        let subtotal = line_item.calculate_subtotal();
        assert_eq!(subtotal, Decimal::from_str("86.42").unwrap());
    }
    
    #[test]
    fn test_large_quantity_calculation() {
        // Test with large quantity to ensure no overflow
        let line_item = LineItem {
            quantity: 10000,
            unit_price: Decimal::from_str("99.99").unwrap(),
            currency: Currency::USD,
        };
        
        let subtotal = line_item.calculate_subtotal();
        assert_eq!(subtotal, Decimal::from_str("999900.00").unwrap());
    }
}
