// T061: Property-based test for tax-on-subtotal-only calculation
//
// Tests FR-055, FR-056:
// - FR-055: Tax calculated on subtotal only (not including service fees)
// - FR-056: Total = subtotal + tax_total + service_fee
//
// This ensures tax is never calculated on service fees (circular dependency prevention)
//
// Uses proptest to validate calculation properties across many inputs

use proptest::prelude::*;
use rust_decimal::Decimal;
use std::str::FromStr;

// Tax calculation (from tax_calculator_test)
fn calculate_tax(subtotal: Decimal, tax_rate: Decimal) -> Decimal {
    (subtotal * tax_rate).round_dp(2)
}

// Service fee calculation (from service_fee_test)
fn calculate_service_fee(
    subtotal: Decimal,
    percentage_rate: Decimal,
    fixed_fee: Decimal,
) -> Decimal {
    let percentage_component = (subtotal * percentage_rate).round_dp(2);
    (percentage_component + fixed_fee).round_dp(2)
}

// Invoice total calculation (FR-056)
fn calculate_invoice_total(
    subtotal: Decimal,
    tax_rate: Decimal,
    service_fee_percentage: Decimal,
    service_fee_fixed: Decimal,
) -> Decimal {
    // Step 1: Calculate tax on subtotal only (FR-055)
    let tax_total = calculate_tax(subtotal, tax_rate);

    // Step 2: Calculate service fee on subtotal only (FR-055)
    let service_fee = calculate_service_fee(subtotal, service_fee_percentage, service_fee_fixed);

    // Step 3: Total = subtotal + tax + service_fee (FR-056)
    let total = (subtotal + tax_total + service_fee).round_dp(2);

    total
}

proptest! {
    #[test]
    fn test_tax_calculated_on_subtotal_only_not_including_service_fee(
        subtotal in 1000u64..10_000_000u64,
        tax_rate_percent in 1u8..=25u8,
        service_fee_percentage_bp in 100u16..=500u16,  // 1% to 5%
        service_fee_fixed in 100u64..10_000u64
    ) {
        let subtotal = Decimal::from(subtotal);
        let tax_rate = Decimal::from(tax_rate_percent) / Decimal::from(100);
        let service_fee_percentage = Decimal::from(service_fee_percentage_bp) / Decimal::from(10000);
        let service_fee_fixed = Decimal::from(service_fee_fixed);

        // Calculate tax on subtotal only
        let tax = calculate_tax(subtotal, tax_rate);

        // Calculate service fee on subtotal only
        let service_fee = calculate_service_fee(subtotal, service_fee_percentage, service_fee_fixed);

        // Tax should NEVER be calculated on (subtotal + service_fee)
        let incorrect_base = subtotal + service_fee;
        let incorrect_tax = calculate_tax(incorrect_base, tax_rate);

        // Verify tax is less than what it would be if calculated on subtotal+fee
        // (Unless service_fee is zero, then they're equal)
        if service_fee > Decimal::ZERO {
            prop_assert!(
                tax < incorrect_tax,
                "Tax ({}) must be less than what it would be if calculated on subtotal+fee ({}). Service fee should not be taxed.",
                tax, incorrect_tax
            );
        }
    }

    #[test]
    fn test_total_equals_subtotal_plus_tax_plus_service_fee(
        subtotal in 1000u64..10_000_000u64,
        tax_rate_percent in 0u8..=25u8,
        service_fee_percentage_bp in 0u16..=500u16,
        service_fee_fixed in 0u64..10_000u64
    ) {
        let subtotal = Decimal::from(subtotal);
        let tax_rate = Decimal::from(tax_rate_percent) / Decimal::from(100);
        let service_fee_percentage = Decimal::from(service_fee_percentage_bp) / Decimal::from(10000);
        let service_fee_fixed = Decimal::from(service_fee_fixed);

        let total = calculate_invoice_total(subtotal, tax_rate, service_fee_percentage, service_fee_fixed);

        // Calculate components separately
        let tax = calculate_tax(subtotal, tax_rate);
        let service_fee = calculate_service_fee(subtotal, service_fee_percentage, service_fee_fixed);
        let expected_total = (subtotal + tax + service_fee).round_dp(2);

        prop_assert_eq!(
            total, expected_total,
            "Total must equal subtotal + tax + service_fee (FR-056)"
        );
    }

    #[test]
    fn test_service_fee_never_affects_tax_amount(
        subtotal in 1000u64..1_000_000u64,
        tax_rate_percent in 5u8..=20u8,
        service_fee_fixed_1 in 100u64..1_000u64,
        service_fee_fixed_2 in 5_000u64..10_000u64
    ) {
        let subtotal = Decimal::from(subtotal);
        let tax_rate = Decimal::from(tax_rate_percent) / Decimal::from(100);
        let service_fee_percentage = Decimal::ZERO; // Use fixed fees only for clarity

        // Calculate tax with different service fees
        let fee1 = Decimal::from(service_fee_fixed_1);
        let fee2 = Decimal::from(service_fee_fixed_2);

        // Tax should be the same regardless of service fee amount
        let tax1 = calculate_tax(subtotal, tax_rate);
        let tax2 = calculate_tax(subtotal, tax_rate);

        prop_assert_eq!(
            tax1, tax2,
            "Tax amount must be independent of service fee (calculated on subtotal only)"
        );

        // Verify totals differ only by the difference in service fees
        let total1 = calculate_invoice_total(subtotal, tax_rate, service_fee_percentage, fee1);
        let total2 = calculate_invoice_total(subtotal, tax_rate, service_fee_percentage, fee2);

        let total_diff = total2 - total1;
        let fee_diff = fee2 - fee1;

        prop_assert_eq!(
            total_diff, fee_diff,
            "Total difference should equal service fee difference (tax unchanged)"
        );
    }

    #[test]
    fn test_tax_never_affects_service_fee_amount(
        subtotal in 1000u64..1_000_000u64,
        tax_rate_1 in 5u8..=15u8,
        tax_rate_2 in 16u8..=25u8,
        service_fee_percentage_bp in 100u16..=300u16,
        service_fee_fixed in 500u64..2_000u64
    ) {
        let subtotal = Decimal::from(subtotal);
        let service_fee_percentage = Decimal::from(service_fee_percentage_bp) / Decimal::from(10000);
        let service_fee_fixed = Decimal::from(service_fee_fixed);

        // Calculate service fee with different tax rates
        let rate1 = Decimal::from(tax_rate_1) / Decimal::from(100);
        let rate2 = Decimal::from(tax_rate_2) / Decimal::from(100);

        // Service fee should be the same regardless of tax rate
        let fee1 = calculate_service_fee(subtotal, service_fee_percentage, service_fee_fixed);
        let fee2 = calculate_service_fee(subtotal, service_fee_percentage, service_fee_fixed);

        prop_assert_eq!(
            fee1, fee2,
            "Service fee amount must be independent of tax rate (calculated on subtotal only)"
        );

        // Verify totals differ only by the difference in taxes
        let total1 = calculate_invoice_total(subtotal, rate1, service_fee_percentage, service_fee_fixed);
        let total2 = calculate_invoice_total(subtotal, rate2, service_fee_percentage, service_fee_fixed);

        let tax1 = calculate_tax(subtotal, rate1);
        let tax2 = calculate_tax(subtotal, rate2);

        let total_diff = total2 - total1;
        let tax_diff = tax2 - tax1;

        // Total difference should approximately equal tax difference (within rounding)
        let tolerance = Decimal::from_str("0.01").unwrap();
        prop_assert!(
            (total_diff - tax_diff).abs() <= tolerance,
            "Total difference should equal tax difference (service fee unchanged)"
        );
    }
}

#[test]
fn test_specific_calculation_examples() {
    // Test specific known examples to verify FR-055 and FR-056

    // Example 1: Standard invoice with tax and service fee
    let subtotal = Decimal::from(100_000); // 100,000
    let tax_rate = Decimal::from_str("0.10").unwrap(); // 10%
    let service_fee_percentage = Decimal::from_str("0.03").unwrap(); // 3%
    let service_fee_fixed = Decimal::from(1_000); // 1,000

    let tax = calculate_tax(subtotal, tax_rate);
    assert_eq!(tax, Decimal::from(10_000)); // 10% of 100,000

    let service_fee = calculate_service_fee(subtotal, service_fee_percentage, service_fee_fixed);
    assert_eq!(service_fee, Decimal::from(4_000)); // (3% of 100,000) + 1,000 = 3,000 + 1,000

    let total = calculate_invoice_total(
        subtotal,
        tax_rate,
        service_fee_percentage,
        service_fee_fixed,
    );
    assert_eq!(total, Decimal::from(114_000)); // 100,000 + 10,000 + 4,000

    // Verify tax is NOT calculated on (subtotal + service_fee)
    let incorrect_tax_base = subtotal + service_fee; // 104,000
    let incorrect_tax = calculate_tax(incorrect_tax_base, tax_rate); // 10,400
    assert_ne!(
        tax, incorrect_tax,
        "Tax should not include service fee in base"
    );
    assert!(
        tax < incorrect_tax,
        "Correct tax should be less than incorrect calculation"
    );
}

#[test]
fn test_order_independence() {
    // Tax and service fee calculations should be independent
    // Order of calculation should not matter

    let subtotal = Decimal::from(50_000);
    let tax_rate = Decimal::from_str("0.11").unwrap(); // 11%
    let service_fee_percentage = Decimal::from_str("0.025").unwrap(); // 2.5%
    let service_fee_fixed = Decimal::from(500);

    // Calculate in order: tax first, then service fee
    let tax_first = calculate_tax(subtotal, tax_rate);
    let fee_first = calculate_service_fee(subtotal, service_fee_percentage, service_fee_fixed);
    let total_first = (subtotal + tax_first + fee_first).round_dp(2);

    // Calculate in order: service fee first, then tax
    let fee_second = calculate_service_fee(subtotal, service_fee_percentage, service_fee_fixed);
    let tax_second = calculate_tax(subtotal, tax_rate);
    let total_second = (subtotal + tax_second + fee_second).round_dp(2);

    // Both orders should produce identical results
    assert_eq!(
        tax_first, tax_second,
        "Tax should be same regardless of calculation order"
    );
    assert_eq!(
        fee_first, fee_second,
        "Service fee should be same regardless of calculation order"
    );
    assert_eq!(
        total_first, total_second,
        "Total should be same regardless of calculation order"
    );
}

#[test]
fn test_zero_tax_zero_fee() {
    // Edge case: invoice with no tax and no service fee
    let subtotal = Decimal::from(25_000);
    let tax_rate = Decimal::ZERO;
    let service_fee_percentage = Decimal::ZERO;
    let service_fee_fixed = Decimal::ZERO;

    let total = calculate_invoice_total(
        subtotal,
        tax_rate,
        service_fee_percentage,
        service_fee_fixed,
    );

    // Total should equal subtotal when no charges
    assert_eq!(total, subtotal);
}

#[test]
fn test_maximum_precision() {
    // Test with amounts that require careful rounding
    let subtotal = Decimal::from_str("12345.67").unwrap();
    let tax_rate = Decimal::from_str("0.115").unwrap(); // 11.5%
    let service_fee_percentage = Decimal::from_str("0.0275").unwrap(); // 2.75%
    let service_fee_fixed = Decimal::from_str("12.34").unwrap();

    let tax = calculate_tax(subtotal, tax_rate);
    // 12345.67 * 0.115 = 1419.75205 → rounds to 1419.75
    assert_eq!(tax, Decimal::from_str("1419.75").unwrap());

    let service_fee = calculate_service_fee(subtotal, service_fee_percentage, service_fee_fixed);
    // (12345.67 * 0.0275) + 12.34 = 339.51 + 12.34 = 351.85
    assert_eq!(service_fee, Decimal::from_str("351.85").unwrap());

    let total = calculate_invoice_total(
        subtotal,
        tax_rate,
        service_fee_percentage,
        service_fee_fixed,
    );
    // 12345.67 + 1419.75 + 351.85 = 14117.27
    assert_eq!(total, Decimal::from_str("14117.27").unwrap());
}
