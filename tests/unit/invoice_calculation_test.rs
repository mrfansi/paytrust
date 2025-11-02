/// Property-based tests for invoice total calculation
/// Tests FR-001, FR-002, FR-003: subtotal + tax_total + service_fee = total_amount
/// 
/// Uses proptest to verify calculation correctness across wide range of inputs

use proptest::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Calculate invoice total: subtotal + tax_total + service_fee
fn calculate_invoice_total(
    subtotal: Decimal,
    tax_total: Decimal,
    service_fee: Decimal,
) -> Decimal {
    subtotal + tax_total + service_fee
}

/// Calculate service fee: (subtotal × fee_percentage) + fee_fixed
fn calculate_service_fee(
    subtotal: Decimal,
    fee_percentage: Decimal,
    fee_fixed: Decimal,
) -> Decimal {
    (subtotal * fee_percentage) + fee_fixed
}

proptest! {
    /// Property: total = subtotal + tax_total + service_fee (always holds)
    #[test]
    fn test_invoice_total_calculation_property(
        subtotal in 1u32..10000000u32,
        tax_total in 0u32..1000000u32,
        service_fee in 0u32..100000u32,
    ) {
        let sub = Decimal::from(subtotal);
        let tax = Decimal::from(tax_total);
        let fee = Decimal::from(service_fee);
        
        let total = calculate_invoice_total(sub, tax, fee);
        
        // Property: total should equal sum of components
        assert_eq!(total, sub + tax + fee);
        
        // Property: total should be >= subtotal
        assert!(total >= sub);
        
        // Property: total should be non-negative
        assert!(total >= Decimal::ZERO);
    }

    /// Property: service_fee = (subtotal × percentage) + fixed (always holds)
    #[test]
    fn test_service_fee_calculation_property(
        subtotal in 1u32..10000000u32,
        fee_percentage_basis_points in 0u32..1000u32, // 0-10% in basis points
        fee_fixed in 0u32..10000u32,
    ) {
        let sub = Decimal::from(subtotal);
        let percentage = Decimal::from(fee_percentage_basis_points) / dec!(10000);
        let fixed = Decimal::from(fee_fixed);
        
        let fee = calculate_service_fee(sub, percentage, fixed);
        
        // Property: fee should equal (subtotal * percentage) + fixed
        assert_eq!(fee, (sub * percentage) + fixed);
        
        // Property: fee should be >= fixed amount
        assert!(fee >= fixed);
        
        // Property: fee should be non-negative
        assert!(fee >= Decimal::ZERO);
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_simple_invoice_total() {
        let subtotal = dec!(1000);
        let tax_total = dec!(100);
        let service_fee = dec!(50);
        let total = calculate_invoice_total(subtotal, tax_total, service_fee);
        assert_eq!(total, dec!(1150));
    }

    #[test]
    fn test_invoice_with_no_tax() {
        let subtotal = dec!(1000);
        let tax_total = dec!(0);
        let service_fee = dec!(50);
        let total = calculate_invoice_total(subtotal, tax_total, service_fee);
        assert_eq!(total, dec!(1050));
    }

    #[test]
    fn test_invoice_with_no_service_fee() {
        let subtotal = dec!(1000);
        let tax_total = dec!(100);
        let service_fee = dec!(0);
        let total = calculate_invoice_total(subtotal, tax_total, service_fee);
        assert_eq!(total, dec!(1100));
    }

    #[test]
    fn test_service_fee_xendit_2_9_percent() {
        // Xendit: 2.9% + 0 fixed
        let subtotal = dec!(1000000); // IDR 1,000,000
        let fee_percentage = dec!(0.029); // 2.9%
        let fee_fixed = dec!(0);
        let fee = calculate_service_fee(subtotal, fee_percentage, fee_fixed);
        assert_eq!(fee, dec!(29000)); // IDR 29,000
    }

    #[test]
    fn test_service_fee_with_fixed_component() {
        let subtotal = dec!(1000);
        let fee_percentage = dec!(0.029); // 2.9%
        let fee_fixed = dec!(5); // Fixed fee
        let fee = calculate_service_fee(subtotal, fee_percentage, fee_fixed);
        assert_eq!(fee, dec!(34)); // (1000 * 0.029) + 5 = 29 + 5 = 34
    }

    #[test]
    fn test_large_invoice_idr() {
        // Large IDR invoice
        let subtotal = dec!(10000000); // IDR 10,000,000
        let tax_total = dec!(1000000); // IDR 1,000,000 (10%)
        let service_fee = dec!(290000); // IDR 290,000 (2.9%)
        let total = calculate_invoice_total(subtotal, tax_total, service_fee);
        assert_eq!(total, dec!(11290000)); // IDR 11,290,000
    }

    #[test]
    fn test_invoice_myr_with_decimals() {
        // MYR invoice with 2 decimal places
        let subtotal = dec!(1000.50);
        let tax_total = dec!(100.05); // 10%
        let service_fee = dec!(29.01); // 2.9%
        let total = calculate_invoice_total(subtotal, tax_total, service_fee);
        assert_eq!(total, dec!(1129.56));
    }
}
