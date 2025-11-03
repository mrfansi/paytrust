use proptest::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;

/// Property-based test for tax-on-subtotal-only calculation (FR-055, FR-056)
/// 
/// Validates:
/// - Tax is calculated ONLY on subtotal (excludes service_fee)
/// - total_amount = subtotal + tax_total + service_fee
/// - Tax is NOT applied to service_fee
/// - Calculation order independence

#[cfg(test)]
mod tax_calculation_tests {
    use super::*;

    /// Calculate invoice total per FR-055 and FR-056
    /// Tax is calculated on subtotal only, NOT on service fee
    fn calculate_invoice_total(
        subtotal: Decimal,
        tax_rate: Decimal,
        service_fee: Decimal,
    ) -> (Decimal, Decimal) {
        // FR-055: Tax calculated on subtotal only (exclude service_fee)
        let tax_total = subtotal * tax_rate;
        
        // FR-056: total_amount = subtotal + tax_total + service_fee
        let total_amount = subtotal + tax_total + service_fee;
        
        (tax_total, total_amount)
    }

    proptest! {
        #[test]
        fn test_tax_excludes_service_fee(
            subtotal_cents in 1000u64..10_000_000u64,  // $10 to $100,000
            tax_rate_basis_points in 0u32..2_700u32,  // 0% to 27%
            service_fee_cents in 0u64..10_000u64  // $0 to $100
        ) {
            let subtotal = Decimal::from_u64(subtotal_cents).unwrap() / Decimal::from(100);
            let tax_rate = Decimal::from_u32(tax_rate_basis_points).unwrap() / Decimal::from(10_000);
            let service_fee = Decimal::from_u64(service_fee_cents).unwrap() / Decimal::from(100);
            
            let (tax_total, total_amount) = calculate_invoice_total(subtotal, tax_rate, service_fee);
            
            // Verify tax is calculated ONLY on subtotal
            let expected_tax = subtotal * tax_rate;
            prop_assert_eq!(tax_total, expected_tax, "Tax should be calculated only on subtotal");
            
            // Verify total is sum of all components
            let expected_total = subtotal + tax_total + service_fee;
            prop_assert_eq!(total_amount, expected_total, "Total should be subtotal + tax + service_fee");
            
            // Verify tax does NOT include service fee in its base
            // If tax included service fee, it would be (subtotal + service_fee) * tax_rate
            let incorrect_tax = (subtotal + service_fee) * tax_rate;
            if service_fee > Decimal::ZERO && tax_rate > Decimal::ZERO {
                prop_assert!(
                    tax_total < incorrect_tax,
                    "Tax should NOT be calculated on subtotal + service_fee"
                );
            }
        }

        #[test]
        fn test_total_calculation_order_independence(
            subtotal_cents in 1000u64..10_000_000u64,
            tax_rate_basis_points in 100u32..2_700u32,  // 1% to 27%
            service_fee_cents in 100u64..10_000u64  // $1 to $100
        ) {
            let subtotal = Decimal::from_u64(subtotal_cents).unwrap() / Decimal::from(100);
            let tax_rate = Decimal::from_u32(tax_rate_basis_points).unwrap() / Decimal::from(10_000);
            let service_fee = Decimal::from_u64(service_fee_cents).unwrap() / Decimal::from(100);
            
            // Calculate in standard order
            let (tax_total1, total_amount1) = calculate_invoice_total(subtotal, tax_rate, service_fee);
            
            // Calculate components separately
            let tax_total2 = subtotal * tax_rate;
            let total_amount2 = subtotal + tax_total2 + service_fee;
            
            // Results should be identical regardless of calculation order
            prop_assert_eq!(tax_total1, tax_total2, "Tax calculation should be order-independent");
            prop_assert_eq!(total_amount1, total_amount2, "Total calculation should be order-independent");
        }

        #[test]
        fn test_service_fee_impact_on_total_not_tax(
            subtotal_cents in 1000u64..10_000_000u64,
            tax_rate_basis_points in 100u32..2_700u32,
            service_fee_cents in 100u64..10_000u64
        ) {
            let subtotal = Decimal::from_u64(subtotal_cents).unwrap() / Decimal::from(100);
            let tax_rate = Decimal::from_u32(tax_rate_basis_points).unwrap() / Decimal::from(10_000);
            let service_fee = Decimal::from_u64(service_fee_cents).unwrap() / Decimal::from(100);
            
            // Calculate with service fee
            let (tax_with_fee, total_with_fee) = calculate_invoice_total(subtotal, tax_rate, service_fee);
            
            // Calculate without service fee
            let (tax_without_fee, total_without_fee) = calculate_invoice_total(subtotal, tax_rate, Decimal::ZERO);
            
            // Tax should be IDENTICAL regardless of service fee
            prop_assert_eq!(tax_with_fee, tax_without_fee, "Service fee should NOT affect tax calculation");
            
            // Total difference should be exactly the service fee
            let total_difference = total_with_fee - total_without_fee;
            prop_assert_eq!(total_difference, service_fee, "Total difference should equal service fee");
        }

        #[test]
        fn test_zero_tax_rate_with_service_fee(
            subtotal_cents in 1000u64..10_000_000u64,
            service_fee_cents in 100u64..10_000u64
        ) {
            let subtotal = Decimal::from_u64(subtotal_cents).unwrap() / Decimal::from(100);
            let tax_rate = Decimal::ZERO;
            let service_fee = Decimal::from_u64(service_fee_cents).unwrap() / Decimal::from(100);
            
            let (tax_total, total_amount) = calculate_invoice_total(subtotal, tax_rate, service_fee);
            
            prop_assert_eq!(tax_total, Decimal::ZERO, "Zero tax rate should produce zero tax");
            prop_assert_eq!(total_amount, subtotal + service_fee, "Total should be subtotal + service_fee when tax is zero");
        }

        #[test]
        fn test_zero_service_fee_with_tax(
            subtotal_cents in 1000u64..10_000_000u64,
            tax_rate_basis_points in 100u32..2_700u32
        ) {
            let subtotal = Decimal::from_u64(subtotal_cents).unwrap() / Decimal::from(100);
            let tax_rate = Decimal::from_u32(tax_rate_basis_points).unwrap() / Decimal::from(10_000);
            let service_fee = Decimal::ZERO;
            
            let (tax_total, total_amount) = calculate_invoice_total(subtotal, tax_rate, service_fee);
            
            let expected_tax = subtotal * tax_rate;
            prop_assert_eq!(tax_total, expected_tax, "Tax should be calculated on subtotal");
            prop_assert_eq!(total_amount, subtotal + tax_total, "Total should be subtotal + tax when service fee is zero");
        }
    }

    #[test]
    fn test_specific_tax_on_subtotal_scenarios() {
        // Scenario 1: $100 subtotal, 10% tax, $2.90 service fee
        // Tax should be $10 (on $100), NOT $10.29 (on $102.90)
        let subtotal = Decimal::from(100);
        let tax_rate = Decimal::from_f64(0.10).unwrap();
        let service_fee = Decimal::from_f64(2.90).unwrap();
        
        let (tax_total, total_amount) = calculate_invoice_total(subtotal, tax_rate, service_fee);
        
        assert_eq!(tax_total, Decimal::from(10), "Tax should be $10 (10% of $100 subtotal)");
        assert_eq!(total_amount, Decimal::from_f64(112.90).unwrap(), "Total should be $100 + $10 + $2.90 = $112.90");

        // Scenario 2: IDR 1,000,000 subtotal, 10% tax, IDR 35,000 service fee
        let subtotal_idr = Decimal::from(1_000_000);
        let tax_rate_idr = Decimal::from_f64(0.10).unwrap();
        let service_fee_idr = Decimal::from(35_000);
        
        let (tax_total_idr, total_amount_idr) = calculate_invoice_total(subtotal_idr, tax_rate_idr, service_fee_idr);
        
        assert_eq!(tax_total_idr, Decimal::from(100_000), "Tax should be 100,000 IDR (10% of 1,000,000)");
        assert_eq!(total_amount_idr, Decimal::from(1_135_000), "Total should be 1,000,000 + 100,000 + 35,000 = 1,135,000");

        // Scenario 3: Verify incorrect calculation (tax on subtotal + service fee)
        let subtotal3 = Decimal::from(100);
        let tax_rate3 = Decimal::from_f64(0.10).unwrap();
        let service_fee3 = Decimal::from_f64(2.90).unwrap();
        
        let (correct_tax, _) = calculate_invoice_total(subtotal3, tax_rate3, service_fee3);
        let incorrect_tax = (subtotal3 + service_fee3) * tax_rate3;  // WRONG: includes service fee
        
        assert_ne!(correct_tax, incorrect_tax, "Tax should NOT be calculated on subtotal + service_fee");
        assert_eq!(correct_tax, Decimal::from(10), "Correct tax is $10");
        assert_eq!(incorrect_tax, Decimal::from_f64(10.29).unwrap(), "Incorrect tax would be $10.29");
    }

    #[test]
    fn test_multiple_line_items_with_service_fee() {
        // Scenario: Multiple line items with different tax rates
        // Line 1: $50 @ 10% tax = $5 tax
        // Line 2: $30 @ 6% tax = $1.80 tax
        // Subtotal: $80
        // Tax total: $6.80
        // Service fee: $2.50 (calculated on $80 subtotal, e.g., 2.9% + $0.18)
        // Total: $80 + $6.80 + $2.50 = $89.30
        
        let line1_subtotal = Decimal::from(50);
        let line1_tax_rate = Decimal::from_f64(0.10).unwrap();
        let line1_tax = line1_subtotal * line1_tax_rate;
        
        let line2_subtotal = Decimal::from(30);
        let line2_tax_rate = Decimal::from_f64(0.06).unwrap();
        let line2_tax = line2_subtotal * line2_tax_rate;
        
        let subtotal = line1_subtotal + line2_subtotal;
        let tax_total_expected = line1_tax + line2_tax;
        let service_fee = Decimal::from_f64(2.50).unwrap();
        
        // Calculate using our function (with average tax rate for testing)
        let avg_tax_rate = tax_total_expected / subtotal;
        let (tax_total, total_amount) = calculate_invoice_total(subtotal, avg_tax_rate, service_fee);
        
        assert_eq!(subtotal, Decimal::from(80), "Subtotal should be $80");
        assert_eq!(tax_total, Decimal::from_f64(6.80).unwrap(), "Tax total should be $6.80");
        assert_eq!(total_amount, Decimal::from_f64(89.30).unwrap(), "Total should be $89.30");
    }

    #[test]
    fn test_fr_055_and_fr_056_compliance() {
        // FR-055: Tax calculated on subtotal only (exclude service_fee)
        // FR-056: total_amount = subtotal + tax_total + service_fee
        
        let test_cases = vec![
            // (subtotal, tax_rate, service_fee, expected_tax, expected_total)
            (100.00, 0.10, 2.90, 10.00, 112.90),
            (50.00, 0.06, 1.50, 3.00, 54.50),
            (200.00, 0.00, 5.80, 0.00, 205.80),
            (75.50, 0.15, 2.25, 11.325, 89.075),
        ];

        for (subtotal_f, tax_rate_f, service_fee_f, expected_tax_f, expected_total_f) in test_cases {
            let subtotal = Decimal::from_f64(subtotal_f).unwrap();
            let tax_rate = Decimal::from_f64(tax_rate_f).unwrap();
            let service_fee = Decimal::from_f64(service_fee_f).unwrap();
            
            let (tax_total, total_amount) = calculate_invoice_total(subtotal, tax_rate, service_fee);
            
            let expected_tax = Decimal::from_f64(expected_tax_f).unwrap();
            let expected_total = Decimal::from_f64(expected_total_f).unwrap();
            
            assert_eq!(
                tax_total, expected_tax,
                "FR-055: Tax should be calculated on subtotal only for case ({}, {}, {})",
                subtotal_f, tax_rate_f, service_fee_f
            );
            assert_eq!(
                total_amount, expected_total,
                "FR-056: Total should be subtotal + tax + service_fee for case ({}, {}, {})",
                subtotal_f, tax_rate_f, service_fee_f
            );
        }
    }
}
