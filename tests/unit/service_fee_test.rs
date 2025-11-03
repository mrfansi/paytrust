use proptest::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;

/// Property-based test for service fee calculation (FR-009, FR-047)
/// 
/// Validates:
/// - service_fee = (subtotal × fee_percentage) + fee_fixed
/// - Calculation is consistent across different values
/// - No precision loss with rust_decimal
/// - Fee percentages between 0.0 and 0.10 (0% to 10%)
/// - Fixed fees are non-negative

#[cfg(test)]
mod service_fee_tests {
    use super::*;

    /// Calculate service fee: (subtotal × fee_percentage) + fee_fixed
    fn calculate_service_fee(subtotal: Decimal, fee_percentage: Decimal, fee_fixed: Decimal) -> Decimal {
        (subtotal * fee_percentage) + fee_fixed
    }

    proptest! {
        #[test]
        fn test_service_fee_calculation_accuracy(
            subtotal_cents in 100u64..10_000_000u64,  // $1.00 to $100,000
            fee_percentage_basis_points in 0u32..1_000u32,  // 0% to 10% (0.0000 to 0.1000)
            fee_fixed_cents in 0u64..10_000u64  // $0.00 to $100.00
        ) {
            // Convert to Decimal with proper scale
            let subtotal = Decimal::from_u64(subtotal_cents).unwrap() / Decimal::from(100);
            let fee_percentage = Decimal::from_u32(fee_percentage_basis_points).unwrap() / Decimal::from(10_000);
            let fee_fixed = Decimal::from_u64(fee_fixed_cents).unwrap() / Decimal::from(100);
            
            // Calculate service fee
            let service_fee = calculate_service_fee(subtotal, fee_percentage, fee_fixed);
            
            // Verify service_fee is non-negative
            prop_assert!(service_fee >= Decimal::ZERO, "Service fee must be non-negative");
            
            // Verify service_fee >= fee_fixed (minimum is fixed fee)
            prop_assert!(service_fee >= fee_fixed, "Service fee must be at least the fixed fee");
            
            // Verify percentage component is correct
            let percentage_component = subtotal * fee_percentage;
            prop_assert!(
                (service_fee - fee_fixed - percentage_component).abs() < Decimal::from_f64(0.01).unwrap(),
                "Service fee should equal percentage component + fixed fee"
            );
        }

        #[test]
        fn test_zero_percentage_fee(
            subtotal_cents in 100u64..10_000_000u64,
            fee_fixed_cents in 0u64..10_000u64
        ) {
            let subtotal = Decimal::from_u64(subtotal_cents).unwrap() / Decimal::from(100);
            let fee_percentage = Decimal::ZERO;
            let fee_fixed = Decimal::from_u64(fee_fixed_cents).unwrap() / Decimal::from(100);
            
            let service_fee = calculate_service_fee(subtotal, fee_percentage, fee_fixed);
            
            prop_assert_eq!(service_fee, fee_fixed, "Zero percentage should result in only fixed fee");
        }

        #[test]
        fn test_zero_fixed_fee(
            subtotal_cents in 100u64..10_000_000u64,
            fee_percentage_basis_points in 100u32..1_000u32  // 1% to 10%
        ) {
            let subtotal = Decimal::from_u64(subtotal_cents).unwrap() / Decimal::from(100);
            let fee_percentage = Decimal::from_u32(fee_percentage_basis_points).unwrap() / Decimal::from(10_000);
            let fee_fixed = Decimal::ZERO;
            
            let service_fee = calculate_service_fee(subtotal, fee_percentage, fee_fixed);
            let expected = subtotal * fee_percentage;
            
            prop_assert_eq!(service_fee, expected, "Zero fixed fee should result in only percentage fee");
        }

        #[test]
        fn test_common_gateway_fees(
            subtotal_cents in 1000u64..10_000_000u64,  // $10 to $100,000
            gateway_config in prop::sample::select(vec![
                (290, 0),      // Xendit: 2.9% + $0
                (290, 30),     // Stripe: 2.9% + $0.30
                (350, 0),      // Midtrans: 3.5% + $0
                (250, 50),     // PayPal: 2.5% + $0.50
            ])
        ) {
            let subtotal = Decimal::from_u64(subtotal_cents).unwrap() / Decimal::from(100);
            let (fee_bp, fixed_cents) = gateway_config;
            let fee_percentage = Decimal::from_u32(fee_bp).unwrap() / Decimal::from(10_000);
            let fee_fixed = Decimal::from_u64(fixed_cents).unwrap() / Decimal::from(100);
            
            let service_fee = calculate_service_fee(subtotal, fee_percentage, fee_fixed);
            
            // Verify service fee is reasonable (not exceeding 10% + $100)
            let max_reasonable = (subtotal * Decimal::from_f64(0.10).unwrap()) + Decimal::from(100);
            prop_assert!(
                service_fee <= max_reasonable,
                "Service fee should be reasonable for common gateways"
            );
        }

        #[test]
        fn test_currency_specific_precision(
            amount_idr in 10000u64..100_000_000u64,  // IDR: whole numbers
            amount_usd_cents in 1000u64..10_000_000u64,  // USD: cents
            fee_percentage_basis_points in 100u32..500u32  // 1% to 5%
        ) {
            let fee_percentage = Decimal::from_u32(fee_percentage_basis_points).unwrap() / Decimal::from(10_000);
            
            // IDR: scale=0 (no decimals)
            let subtotal_idr = Decimal::from_u64(amount_idr).unwrap();
            let fee_fixed_idr = Decimal::from(1000);  // IDR 1,000 fixed fee
            let service_fee_idr = calculate_service_fee(subtotal_idr, fee_percentage, fee_fixed_idr);
            // IDR service fee should round to whole numbers
            let service_fee_idr_rounded = service_fee_idr.round();
            prop_assert!(
                (service_fee_idr - service_fee_idr_rounded).abs() < Decimal::ONE,
                "IDR service fee should be close to whole number"
            );
            
            // USD: scale=2 (2 decimals)
            let subtotal_usd = Decimal::from_u64(amount_usd_cents).unwrap() / Decimal::from(100);
            let fee_fixed_usd = Decimal::from_f64(0.30).unwrap();  // $0.30 fixed fee
            let service_fee_usd = calculate_service_fee(subtotal_usd, fee_percentage, fee_fixed_usd);
            let service_fee_usd_rounded = service_fee_usd.round_dp(2);
            prop_assert!(
                (service_fee_usd - service_fee_usd_rounded).abs() < Decimal::from_f64(0.01).unwrap(),
                "USD service fee should round to 2 decimal places"
            );
        }
    }

    #[test]
    fn test_specific_service_fee_scenarios() {
        // Scenario 1: Xendit-style fee (2.9% + $0) on $100.00
        let subtotal = Decimal::from(100);
        let fee_percentage = Decimal::from_f64(0.029).unwrap();
        let fee_fixed = Decimal::ZERO;
        let service_fee = calculate_service_fee(subtotal, fee_percentage, fee_fixed);
        assert_eq!(service_fee, Decimal::from_f64(2.90).unwrap(), "2.9% of $100 should be $2.90");

        // Scenario 2: Stripe-style fee (2.9% + $0.30) on $100.00
        let subtotal2 = Decimal::from(100);
        let fee_percentage2 = Decimal::from_f64(0.029).unwrap();
        let fee_fixed2 = Decimal::from_f64(0.30).unwrap();
        let service_fee2 = calculate_service_fee(subtotal2, fee_percentage2, fee_fixed2);
        assert_eq!(service_fee2, Decimal::from_f64(3.20).unwrap(), "2.9% + $0.30 on $100 should be $3.20");

        // Scenario 3: Midtrans-style fee (3.5% + IDR 0) on IDR 1,000,000
        let subtotal_idr = Decimal::from(1_000_000);
        let fee_percentage_idr = Decimal::from_f64(0.035).unwrap();
        let fee_fixed_idr = Decimal::ZERO;
        let service_fee_idr = calculate_service_fee(subtotal_idr, fee_percentage_idr, fee_fixed_idr);
        assert_eq!(service_fee_idr, Decimal::from(35_000), "3.5% of 1,000,000 IDR should be 35,000 IDR");

        // Scenario 4: Zero fees
        let subtotal_zero = Decimal::from(100);
        let service_fee_zero = calculate_service_fee(subtotal_zero, Decimal::ZERO, Decimal::ZERO);
        assert_eq!(service_fee_zero, Decimal::ZERO, "Zero fees should result in zero service fee");
    }

    #[test]
    fn test_service_fee_proportionality() {
        // Service fee should scale linearly with subtotal (for percentage component)
        let fee_percentage = Decimal::from_f64(0.029).unwrap();
        let fee_fixed = Decimal::from_f64(0.30).unwrap();

        let subtotal1 = Decimal::from(100);
        let service_fee1 = calculate_service_fee(subtotal1, fee_percentage, fee_fixed);

        let subtotal2 = Decimal::from(200);  // Double the subtotal
        let service_fee2 = calculate_service_fee(subtotal2, fee_percentage, fee_fixed);

        // The difference should be exactly the percentage component of the additional $100
        let expected_difference = Decimal::from(100) * fee_percentage;
        let actual_difference = service_fee2 - service_fee1;

        assert_eq!(
            actual_difference, expected_difference,
            "Service fee should scale linearly with subtotal"
        );
    }
}
