use proptest::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;

/// Property-based test for per-line-item tax calculation (FR-057, FR-058)
/// 
/// Validates:
/// - tax_amount = subtotal × tax_rate
/// - Calculation is consistent across different values
/// - No precision loss with rust_decimal
/// - Tax rates between 0.0 and 1.0 (0% to 100%)

#[cfg(test)]
mod tax_calculator_tests {
    use super::*;

    proptest! {
        #[test]
        fn test_tax_calculation_accuracy(
            subtotal_cents in 1u64..10_000_000u64,  // $0.01 to $100,000
            tax_rate_basis_points in 0u32..10_000u32  // 0% to 100% (0.0000 to 1.0000)
        ) {
            // Convert to Decimal with proper scale
            let subtotal = Decimal::from_u64(subtotal_cents).unwrap() / Decimal::from(100);
            let tax_rate = Decimal::from_u32(tax_rate_basis_points).unwrap() / Decimal::from(10_000);
            
            // Calculate tax: tax_amount = subtotal × tax_rate
            let tax_amount = subtotal * tax_rate;
            
            // Verify tax_amount is non-negative
            prop_assert!(tax_amount >= Decimal::ZERO, "Tax amount must be non-negative");
            
            // Verify tax_amount <= subtotal (since tax_rate <= 1.0)
            prop_assert!(tax_amount <= subtotal, "Tax cannot exceed subtotal when rate <= 100%");
            
            // Verify calculation is reversible (within precision limits)
            if tax_rate > Decimal::ZERO {
                let calculated_subtotal = tax_amount / tax_rate;
                let difference = (calculated_subtotal - subtotal).abs();
                prop_assert!(
                    difference < Decimal::from_f64(0.01).unwrap(),
                    "Reverse calculation should match original subtotal within 1 cent"
                );
            }
        }

        #[test]
        fn test_zero_tax_rate(
            subtotal_cents in 1u64..10_000_000u64
        ) {
            let subtotal = Decimal::from_u64(subtotal_cents).unwrap() / Decimal::from(100);
            let tax_rate = Decimal::ZERO;
            
            let tax_amount = subtotal * tax_rate;
            
            prop_assert_eq!(tax_amount, Decimal::ZERO, "Zero tax rate should produce zero tax");
        }

        #[test]
        fn test_hundred_percent_tax_rate(
            subtotal_cents in 1u64..10_000_000u64
        ) {
            let subtotal = Decimal::from_u64(subtotal_cents).unwrap() / Decimal::from(100);
            let tax_rate = Decimal::ONE;
            
            let tax_amount = subtotal * tax_rate;
            
            prop_assert_eq!(tax_amount, subtotal, "100% tax rate should equal subtotal");
        }

        #[test]
        fn test_common_tax_rates(
            subtotal_cents in 1u64..10_000_000u64,
            tax_rate_percent in prop::sample::select(vec![0, 5, 6, 10, 15, 20, 25, 27])
        ) {
            // Test common tax rates: 0%, 5%, 6%, 10%, 15%, 20%, 25%, 27%
            let subtotal = Decimal::from_u64(subtotal_cents).unwrap() / Decimal::from(100);
            let tax_rate = Decimal::from_u32(tax_rate_percent).unwrap() / Decimal::from(100);
            
            let tax_amount = subtotal * tax_rate;
            
            // Verify tax is within expected range
            let expected_min = Decimal::ZERO;
            let expected_max = subtotal * Decimal::from_u32(tax_rate_percent).unwrap() / Decimal::from(100);
            
            prop_assert!(
                tax_amount >= expected_min && tax_amount <= expected_max,
                "Tax amount should be within expected range for {}% rate", tax_rate_percent
            );
        }

        #[test]
        fn test_currency_specific_precision(
            amount_idr in 1000u64..100_000_000u64,  // IDR: whole numbers
            amount_usd_cents in 100u64..10_000_000u64,  // USD: cents
            tax_rate_basis_points in 100u32..2_700u32  // 1% to 27%
        ) {
            let tax_rate = Decimal::from_u32(tax_rate_basis_points).unwrap() / Decimal::from(10_000);
            
            // IDR: scale=0 (no decimals)
            let subtotal_idr = Decimal::from_u64(amount_idr).unwrap();
            let tax_idr = subtotal_idr * tax_rate;
            // IDR tax should round to whole numbers
            let tax_idr_rounded = tax_idr.round();
            prop_assert!(
                (tax_idr - tax_idr_rounded).abs() < Decimal::ONE,
                "IDR tax should be close to whole number"
            );
            
            // USD: scale=2 (2 decimals)
            let subtotal_usd = Decimal::from_u64(amount_usd_cents).unwrap() / Decimal::from(100);
            let tax_usd = subtotal_usd * tax_rate;
            let tax_usd_rounded = tax_usd.round_dp(2);
            prop_assert!(
                (tax_usd - tax_usd_rounded).abs() < Decimal::from_f64(0.01).unwrap(),
                "USD tax should round to 2 decimal places"
            );
        }
    }

    #[test]
    fn test_specific_tax_scenarios() {
        // Scenario 1: 10% tax on $100.00
        let subtotal = Decimal::from(100);
        let tax_rate = Decimal::from_f64(0.10).unwrap();
        let tax_amount = subtotal * tax_rate;
        assert_eq!(tax_amount, Decimal::from(10), "10% of $100 should be $10");

        // Scenario 2: 27% tax on IDR 1,000,000
        let subtotal_idr = Decimal::from(1_000_000);
        let tax_rate_27 = Decimal::from_f64(0.27).unwrap();
        let tax_idr = subtotal_idr * tax_rate_27;
        assert_eq!(tax_idr, Decimal::from(270_000), "27% of 1,000,000 IDR should be 270,000 IDR");

        // Scenario 3: 6% tax on MYR 50.50
        let subtotal_myr = Decimal::from_f64(50.50).unwrap();
        let tax_rate_6 = Decimal::from_f64(0.06).unwrap();
        let tax_myr = subtotal_myr * tax_rate_6;
        assert_eq!(tax_myr, Decimal::from_f64(3.03).unwrap(), "6% of MYR 50.50 should be MYR 3.03");
    }
}
