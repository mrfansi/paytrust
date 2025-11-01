// T060: Property-based test for service fee calculation
//
// Tests FR-009, FR-047:
// - FR-009: Service fees are percentage-based or fixed amounts
// - FR-047: Gateway-specific fee structures (percentage + fixed)
//
// Service fee formula: (subtotal × percentage_rate) + fixed_fee
//
// Uses proptest to validate calculation properties across many inputs

use proptest::prelude::*;
use rust_decimal::Decimal;
use std::str::FromStr;

// Service fee calculation function to be implemented
// For now, this is the expected implementation
fn calculate_service_fee(subtotal: Decimal, percentage_rate: Decimal, fixed_fee: Decimal) -> Decimal {
    let percentage_component = (subtotal * percentage_rate).round_dp(2);
    let total_fee = (percentage_component + fixed_fee).round_dp(2);
    total_fee
}

proptest! {
    #[test]
    fn test_service_fee_calculation_is_deterministic(
        subtotal in 0u64..1_000_000_000u64,
        percentage_rate_basis_points in 0u16..=10000u16,  // 0% to 100% in basis points
        fixed_fee in 0u64..100_000u64
    ) {
        let subtotal = Decimal::from(subtotal);
        let percentage_rate = Decimal::from(percentage_rate_basis_points) / Decimal::from(10000);
        let fixed_fee = Decimal::from(fixed_fee);
        
        // Service fee calculation should always produce same result for same inputs
        let fee1 = calculate_service_fee(subtotal, percentage_rate, fixed_fee);
        let fee2 = calculate_service_fee(subtotal, percentage_rate, fixed_fee);
        
        prop_assert_eq!(fee1, fee2, "Service fee calculation must be deterministic");
    }

    #[test]
    fn test_service_fee_is_non_negative(
        subtotal in 0u64..1_000_000_000u64,
        percentage_rate_basis_points in 0u16..=5000u16,  // 0% to 50%
        fixed_fee in 0u64..100_000u64
    ) {
        let subtotal = Decimal::from(subtotal);
        let percentage_rate = Decimal::from(percentage_rate_basis_points) / Decimal::from(10000);
        let fixed_fee = Decimal::from(fixed_fee);
        
        let fee = calculate_service_fee(subtotal, percentage_rate, fixed_fee);
        
        prop_assert!(fee >= Decimal::ZERO, "Service fee must be non-negative: got {}", fee);
    }

    #[test]
    fn test_zero_percentage_produces_only_fixed_fee(
        subtotal in 0u64..1_000_000_000u64,
        fixed_fee in 0u64..100_000u64
    ) {
        let subtotal = Decimal::from(subtotal);
        let percentage_rate = Decimal::ZERO;
        let fixed_fee = Decimal::from(fixed_fee);
        
        let fee = calculate_service_fee(subtotal, percentage_rate, fixed_fee);
        
        prop_assert_eq!(fee, fixed_fee, "0% percentage rate should produce only fixed fee");
    }

    #[test]
    fn test_zero_fixed_fee_produces_only_percentage(
        subtotal in 1u64..1_000_000_000u64,
        percentage_rate_basis_points in 1u16..=5000u16  // 0.01% to 50%
    ) {
        let subtotal = Decimal::from(subtotal);
        let percentage_rate = Decimal::from(percentage_rate_basis_points) / Decimal::from(10000);
        let fixed_fee = Decimal::ZERO;
        
        let fee = calculate_service_fee(subtotal, percentage_rate, fixed_fee);
        let expected_percentage_component = (subtotal * percentage_rate).round_dp(2);
        
        prop_assert_eq!(fee, expected_percentage_component, "Zero fixed fee should produce only percentage component");
    }

    #[test]
    fn test_both_zero_produces_zero_fee(
        subtotal in 0u64..1_000_000_000u64
    ) {
        let subtotal = Decimal::from(subtotal);
        let percentage_rate = Decimal::ZERO;
        let fixed_fee = Decimal::ZERO;
        
        let fee = calculate_service_fee(subtotal, percentage_rate, fixed_fee);
        
        prop_assert_eq!(fee, Decimal::ZERO, "Zero rate and fee must produce zero service fee");
    }

    #[test]
    fn test_service_fee_scales_with_subtotal(
        base_subtotal in 1u64..1_000_000u64,
        multiplier in 2u32..10u32,
        percentage_rate_basis_points in 1u16..=2500u16,  // 0.01% to 25%
        fixed_fee in 0u64..10_000u64
    ) {
        let subtotal1 = Decimal::from(base_subtotal);
        let subtotal2 = Decimal::from(base_subtotal * multiplier as u64);
        let percentage_rate = Decimal::from(percentage_rate_basis_points) / Decimal::from(10000);
        let fixed_fee = Decimal::from(fixed_fee);
        
        let fee1 = calculate_service_fee(subtotal1, percentage_rate, fixed_fee);
        let fee2 = calculate_service_fee(subtotal2, percentage_rate, fixed_fee);
        
        // Fee2 should be larger than fee1 (if percentage > 0)
        if percentage_rate > Decimal::ZERO {
            prop_assert!(fee2 > fee1, "Service fee should increase with subtotal: fee1={}, fee2={}", fee1, fee2);
        }
    }

    #[test]
    fn test_fixed_fee_independence_from_subtotal(
        subtotal1 in 0u64..100_000u64,
        subtotal2 in 0u64..100_000u64,
        fixed_fee in 1u64..10_000u64
    ) {
        let subtotal1 = Decimal::from(subtotal1);
        let subtotal2 = Decimal::from(subtotal2);
        let percentage_rate = Decimal::ZERO; // No percentage
        let fixed_fee = Decimal::from(fixed_fee);
        
        let fee1 = calculate_service_fee(subtotal1, percentage_rate, fixed_fee);
        let fee2 = calculate_service_fee(subtotal2, percentage_rate, fixed_fee);
        
        // With 0% percentage, fee should always equal fixed_fee regardless of subtotal
        prop_assert_eq!(fee1, fixed_fee, "Fixed fee only: fee1 should equal fixed_fee");
        prop_assert_eq!(fee2, fixed_fee, "Fixed fee only: fee2 should equal fixed_fee");
        prop_assert_eq!(fee1, fee2, "Fixed fee should be independent of subtotal");
    }
}

#[test]
fn test_typical_gateway_fee_structures() {
    // Test real-world gateway fee structures
    
    // Xendit: 2.9% + IDR 2,200
    let subtotal_idr = Decimal::from(1_000_000); // 1 million IDR
    let xendit_percentage = Decimal::from_str("0.029").unwrap(); // 2.9%
    let xendit_fixed = Decimal::from(2200);
    
    let xendit_fee = calculate_service_fee(subtotal_idr, xendit_percentage, xendit_fixed);
    let expected_xendit = (Decimal::from(29000) + Decimal::from(2200)).round_dp(2); // 29,000 + 2,200 = 31,200
    assert_eq!(xendit_fee, expected_xendit, "Xendit fee structure");
    
    // Midtrans: 2.0% + IDR 0 (percentage only)
    let midtrans_percentage = Decimal::from_str("0.02").unwrap(); // 2.0%
    let midtrans_fixed = Decimal::ZERO;
    
    let midtrans_fee = calculate_service_fee(subtotal_idr, midtrans_percentage, midtrans_fixed);
    let expected_midtrans = Decimal::from(20000).round_dp(2); // 20,000
    assert_eq!(midtrans_fee, expected_midtrans, "Midtrans fee structure");
    
    // Custom gateway: 1.5% + MYR 1.00
    let subtotal_myr = Decimal::from_str("1000.00").unwrap();
    let custom_percentage = Decimal::from_str("0.015").unwrap(); // 1.5%
    let custom_fixed = Decimal::from_str("1.00").unwrap();
    
    let custom_fee = calculate_service_fee(subtotal_myr, custom_percentage, custom_fixed);
    let expected_custom = Decimal::from_str("16.00").unwrap(); // 15.00 + 1.00
    assert_eq!(custom_fee, expected_custom, "Custom gateway fee structure");
}

#[test]
fn test_service_fee_precision() {
    // Service fees should have at most 2 decimal places for currency
    
    let subtotal = Decimal::from_str("1234.56").unwrap();
    let percentage_rate = Decimal::from_str("0.0234").unwrap(); // 2.34%
    let fixed_fee = Decimal::from_str("0.99").unwrap();
    
    let fee = calculate_service_fee(subtotal, percentage_rate, fixed_fee);
    
    // Fee should be rounded to 2 decimal places
    assert_eq!(fee.scale(), 2, "Service fee should have exactly 2 decimal places");
    
    // Calculate expected: (1234.56 * 0.0234) + 0.99 = 28.89 + 0.99 = 29.88
    let expected = Decimal::from_str("29.88").unwrap();
    assert_eq!(fee, expected);
}

#[test]
fn test_percentage_and_fixed_combination() {
    // Test that both components are correctly added
    
    let subtotal = Decimal::from(10000);
    let percentage_rate = Decimal::from_str("0.03").unwrap(); // 3%
    let fixed_fee = Decimal::from(500);
    
    let fee = calculate_service_fee(subtotal, percentage_rate, fixed_fee);
    
    // Expected: (10000 * 0.03) + 500 = 300 + 500 = 800
    assert_eq!(fee, Decimal::from(800));
    
    // Verify each component separately
    let percentage_component = (subtotal * percentage_rate).round_dp(2);
    assert_eq!(percentage_component, Decimal::from(300));
    
    let total = percentage_component + fixed_fee;
    assert_eq!(total, Decimal::from(800));
}

#[test]
fn test_basis_points_conversion() {
    // Test common basis point values (1 basis point = 0.01%)
    
    let subtotal = Decimal::from(100_000);
    
    // 100 basis points = 1%
    let rate_100bp = Decimal::from(100) / Decimal::from(10000);
    let fee_100bp = calculate_service_fee(subtotal, rate_100bp, Decimal::ZERO);
    assert_eq!(fee_100bp, Decimal::from(1000)); // 1% of 100,000 = 1,000
    
    // 250 basis points = 2.5%
    let rate_250bp = Decimal::from(250) / Decimal::from(10000);
    let fee_250bp = calculate_service_fee(subtotal, rate_250bp, Decimal::ZERO);
    assert_eq!(fee_250bp, Decimal::from(2500)); // 2.5% of 100,000 = 2,500
    
    // 50 basis points = 0.5%
    let rate_50bp = Decimal::from(50) / Decimal::from(10000);
    let fee_50bp = calculate_service_fee(subtotal, rate_50bp, Decimal::ZERO);
    assert_eq!(fee_50bp, Decimal::from(500)); // 0.5% of 100,000 = 500
}
