// T059: Property-based test for per-line-item tax calculation
//
// Tests FR-057, FR-058:
// - FR-057: Tax calculated per line item (item_subtotal × tax_rate)
// - FR-058: Tax is applied to each line item independently
//
// Uses proptest to validate calculation properties across many inputs

use proptest::prelude::*;
use rust_decimal::Decimal;
use std::str::FromStr;

// Tax calculation function to be implemented
// For now, this is the expected implementation
fn calculate_line_item_tax(subtotal: Decimal, tax_rate: Decimal) -> Decimal {
    (subtotal * tax_rate).round_dp(2)
}

proptest! {
    #[test]
    fn test_tax_calculation_is_deterministic(
        subtotal in 0u64..1_000_000_000u64,
        tax_rate_percent in 0u8..=100u8
    ) {
        // Convert to Decimal
        let subtotal = Decimal::from(subtotal);
        let tax_rate = Decimal::from(tax_rate_percent) / Decimal::from(100);

        // Tax calculation should always produce the same result for same inputs
        let tax1 = calculate_line_item_tax(subtotal, tax_rate);
        let tax2 = calculate_line_item_tax(subtotal, tax_rate);

        prop_assert_eq!(tax1, tax2, "Tax calculation must be deterministic");
    }

    #[test]
    fn test_tax_is_non_negative(
        subtotal in 0u64..1_000_000_000u64,
        tax_rate_percent in 0u8..=100u8
    ) {
        let subtotal = Decimal::from(subtotal);
        let tax_rate = Decimal::from(tax_rate_percent) / Decimal::from(100);

        let tax = calculate_line_item_tax(subtotal, tax_rate);

        prop_assert!(tax >= Decimal::ZERO, "Tax must be non-negative: got {}", tax);
    }

    #[test]
    fn test_tax_never_exceeds_subtotal(
        subtotal in 0u64..1_000_000_000u64,
        tax_rate_percent in 0u8..=100u8
    ) {
        let subtotal = Decimal::from(subtotal);
        let tax_rate = Decimal::from(tax_rate_percent) / Decimal::from(100);

        let tax = calculate_line_item_tax(subtotal, tax_rate);

        // For rates up to 100%, tax should not exceed subtotal
        prop_assert!(tax <= subtotal, "Tax {} should not exceed subtotal {} at rate {}", tax, subtotal, tax_rate);
    }

    #[test]
    fn test_zero_rate_produces_zero_tax(
        subtotal in 0u64..1_000_000_000u64
    ) {
        let subtotal = Decimal::from(subtotal);
        let tax_rate = Decimal::ZERO;

        let tax = calculate_line_item_tax(subtotal, tax_rate);

        prop_assert_eq!(tax, Decimal::ZERO, "0% tax rate must produce zero tax");
    }

    #[test]
    fn test_zero_subtotal_produces_zero_tax(
        tax_rate_percent in 0u8..=100u8
    ) {
        let subtotal = Decimal::ZERO;
        let tax_rate = Decimal::from(tax_rate_percent) / Decimal::from(100);

        let tax = calculate_line_item_tax(subtotal, tax_rate);

        prop_assert_eq!(tax, Decimal::ZERO, "Zero subtotal must produce zero tax");
    }

    #[test]
    fn test_tax_scales_linearly_with_subtotal(
        base_subtotal in 1u64..1_000_000u64,
        multiplier in 2u32..10u32,
        tax_rate_percent in 1u8..=50u8
    ) {
        let subtotal1 = Decimal::from(base_subtotal);
        let subtotal2 = Decimal::from(base_subtotal * multiplier as u64);
        let tax_rate = Decimal::from(tax_rate_percent) / Decimal::from(100);

        let tax1 = calculate_line_item_tax(subtotal1, tax_rate);
        let tax2 = calculate_line_item_tax(subtotal2, tax_rate);

        // Due to rounding, we can't assert exact proportionality
        // But tax2 should be approximately multiplier times tax1
        let ratio = tax2 / tax1;
        let expected_ratio = Decimal::from(multiplier);
        let tolerance = Decimal::from_str("0.01").unwrap(); // 1% tolerance for rounding

        prop_assert!(
            (ratio - expected_ratio).abs() <= tolerance,
            "Tax should scale linearly: tax1={}, tax2={}, ratio={}, expected={}",
            tax1, tax2, ratio, expected_ratio
        );
    }

    #[test]
    fn test_standard_vat_rates(
        subtotal in 1_000u64..10_000_000u64  // 10.00 to 100,000.00
    ) {
        let subtotal = Decimal::from(subtotal) / Decimal::from(100); // Convert to decimal currency

        // Test common VAT rates
        let vat_rates = vec![
            (Decimal::from_str("0.10").unwrap(), "10%"),
            (Decimal::from_str("0.11").unwrap(), "11%"),
            (Decimal::from_str("0.15").unwrap(), "15%"),
        ];

        for (rate, label) in vat_rates {
            let tax = calculate_line_item_tax(subtotal, rate);

            // Tax should be positive for positive subtotal
            prop_assert!(tax > Decimal::ZERO, "{} VAT should produce positive tax", label);

            // Tax should be less than subtotal for rates < 100%
            prop_assert!(tax < subtotal, "{} VAT should be less than subtotal", label);

            // Tax should have at most 2 decimal places (currency precision)
            let scale = tax.scale();
            prop_assert!(scale <= 2, "{} VAT tax should have at most 2 decimal places, got scale {}", label, scale);
        }
    }
}

#[test]
fn test_specific_tax_calculations() {
    // Test specific known values to ensure correctness

    // 10% VAT on 1000 = 100
    assert_eq!(
        calculate_line_item_tax(Decimal::from(1000), Decimal::from_str("0.10").unwrap()),
        Decimal::from(100)
    );

    // 11% VAT on 1000 = 110
    assert_eq!(
        calculate_line_item_tax(Decimal::from(1000), Decimal::from_str("0.11").unwrap()),
        Decimal::from(110)
    );

    // 15% VAT on 500 = 75
    assert_eq!(
        calculate_line_item_tax(Decimal::from(500), Decimal::from_str("0.15").unwrap()),
        Decimal::from(75)
    );

    // Test rounding: 10% of 333 = 33.30
    assert_eq!(
        calculate_line_item_tax(Decimal::from(333), Decimal::from_str("0.10").unwrap()),
        Decimal::from_str("33.30").unwrap()
    );
}

#[test]
fn test_per_line_item_independence() {
    // FR-058: Tax is applied per line item independently
    // Each line item's tax should be calculated separately

    let tax_rate = Decimal::from_str("0.10").unwrap(); // 10%

    let line1_subtotal = Decimal::from(1000);
    let line2_subtotal = Decimal::from(2000);
    let line3_subtotal = Decimal::from(500);

    let line1_tax = calculate_line_item_tax(line1_subtotal, tax_rate);
    let line2_tax = calculate_line_item_tax(line2_subtotal, tax_rate);
    let line3_tax = calculate_line_item_tax(line3_subtotal, tax_rate);

    // Each line item gets its own tax calculation
    assert_eq!(line1_tax, Decimal::from(100));
    assert_eq!(line2_tax, Decimal::from(200));
    assert_eq!(line3_tax, Decimal::from(50));

    // Total tax is sum of individual line taxes
    let total_tax = line1_tax + line2_tax + line3_tax;
    assert_eq!(total_tax, Decimal::from(350));
}
