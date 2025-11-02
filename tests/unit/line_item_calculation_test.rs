/// Property-based tests for line item subtotal calculation
/// Tests FR-001 and FR-005: quantity × unit_price = subtotal
/// 
/// Uses proptest to verify calculation correctness across wide range of inputs

use proptest::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Line item subtotal calculation: quantity × unit_price
fn calculate_line_item_subtotal(quantity: Decimal, unit_price: Decimal) -> Decimal {
    quantity * unit_price
}

/// Line item tax calculation: subtotal × tax_rate
fn calculate_line_item_tax(subtotal: Decimal, tax_rate: Decimal) -> Decimal {
    subtotal * tax_rate
}

proptest! {
    /// Property: subtotal = quantity × unit_price (always holds)
    #[test]
    fn test_subtotal_calculation_property(
        quantity in 1u32..10000u32,
        unit_price in 1u32..1000000u32,
    ) {
        let qty = Decimal::from(quantity);
        let price = Decimal::from(unit_price);
        let subtotal = calculate_line_item_subtotal(qty, price);
        
        // Property: subtotal should equal quantity * unit_price
        assert_eq!(subtotal, qty * price);
        
        // Property: subtotal should be non-negative
        assert!(subtotal >= Decimal::ZERO);
        
        // Property: if quantity > 0 and price > 0, subtotal > 0
        if qty > Decimal::ZERO && price > Decimal::ZERO {
            assert!(subtotal > Decimal::ZERO);
        }
    }

    /// Property: tax = subtotal × tax_rate (always holds)
    #[test]
    fn test_tax_calculation_property(
        subtotal in 1u32..10000000u32,
        tax_rate_percent in 0u32..100u32, // 0-100%
    ) {
        let subtotal_dec = Decimal::from(subtotal);
        let tax_rate = Decimal::from(tax_rate_percent) / dec!(100);
        let tax = calculate_line_item_tax(subtotal_dec, tax_rate);
        
        // Property: tax should equal subtotal * tax_rate
        assert_eq!(tax, subtotal_dec * tax_rate);
        
        // Property: tax should be non-negative
        assert!(tax >= Decimal::ZERO);
        
        // Property: tax should not exceed subtotal (when rate <= 1.0)
        if tax_rate <= Decimal::ONE {
            assert!(tax <= subtotal_dec);
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_simple_subtotal() {
        let quantity = dec!(5);
        let unit_price = dec!(1000);
        let subtotal = calculate_line_item_subtotal(quantity, unit_price);
        assert_eq!(subtotal, dec!(5000));
    }

    #[test]
    fn test_decimal_quantity() {
        let quantity = dec!(2.5);
        let unit_price = dec!(1000);
        let subtotal = calculate_line_item_subtotal(quantity, unit_price);
        assert_eq!(subtotal, dec!(2500));
    }

    #[test]
    fn test_idr_no_decimals() {
        // IDR should have scale=0
        let quantity = dec!(3);
        let unit_price = dec!(1000000); // IDR 1,000,000
        let subtotal = calculate_line_item_subtotal(quantity, unit_price);
        assert_eq!(subtotal, dec!(3000000));
        assert_eq!(subtotal.scale(), 0);
    }

    #[test]
    fn test_myr_two_decimals() {
        // MYR should have scale=2
        let quantity = dec!(2);
        let unit_price = dec!(1000.50); // MYR 1,000.50
        let subtotal = calculate_line_item_subtotal(quantity, unit_price);
        assert_eq!(subtotal, dec!(2001.00));
    }

    #[test]
    fn test_tax_calculation_10_percent() {
        let subtotal = dec!(1000);
        let tax_rate = dec!(0.10); // 10%
        let tax = calculate_line_item_tax(subtotal, tax_rate);
        assert_eq!(tax, dec!(100));
    }

    #[test]
    fn test_tax_calculation_zero_rate() {
        let subtotal = dec!(1000);
        let tax_rate = dec!(0);
        let tax = calculate_line_item_tax(subtotal, tax_rate);
        assert_eq!(tax, dec!(0));
    }
}
