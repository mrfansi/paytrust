// T080: Property-based test for proportional tax distribution (FR-059)
// T081: Property-based test for proportional service fee distribution (FR-060)
// T082: Property-based test for rounding and last installment absorption (FR-071, FR-072)

use paytrust::modules::installments::services::InstallmentCalculator;
use proptest::prelude::*;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Test that proportional tax distribution maintains total tax amount
#[test]
fn test_proportional_tax_distribution() {
    let calculator = InstallmentCalculator::new();

    let total_amount = Decimal::from_str("1000.00").unwrap();
    let total_tax = Decimal::from_str("100.00").unwrap();
    let installment_count = 3;

    let installments = calculator
        .calculate_equal_installments(total_amount, installment_count, Some(total_tax), None)
        .expect("Failed to calculate installments");

    // Verify tax amounts sum to total tax
    let calculated_tax_total: Decimal = installments.iter().map(|i| i.tax_amount).sum();

    assert_eq!(
        calculated_tax_total, total_tax,
        "Tax amounts must sum to total tax"
    );
}

/// Test that proportional service fee distribution maintains total fee
#[test]
fn test_proportional_service_fee_distribution() {
    let calculator = InstallmentCalculator::new();

    let total_amount = Decimal::from_str("1000.00").unwrap();
    let total_service_fee = Decimal::from_str("50.00").unwrap();
    let installment_count = 4;

    let installments = calculator
        .calculate_equal_installments(
            total_amount,
            installment_count,
            None,
            Some(total_service_fee),
        )
        .expect("Failed to calculate installments");

    // Verify service fee amounts sum to total service fee
    let calculated_fee_total: Decimal = installments.iter().map(|i| i.service_fee_amount).sum();

    assert_eq!(
        calculated_fee_total, total_service_fee,
        "Service fees must sum to total fee"
    );
}

/// Test that last installment absorbs rounding differences
#[test]
fn test_last_installment_absorbs_rounding() {
    let calculator = InstallmentCalculator::new();

    // Amount that doesn't divide evenly by 3
    let total_amount = Decimal::from_str("100.00").unwrap();
    let installment_count = 3;

    let installments = calculator
        .calculate_equal_installments(total_amount, installment_count, None, None)
        .expect("Failed to calculate installments");

    // Verify total equals exactly the input amount
    let calculated_total: Decimal = installments.iter().map(|i| i.amount).sum();

    assert_eq!(
        calculated_total, total_amount,
        "Last installment must absorb rounding difference"
    );

    // Verify first two are equal
    assert_eq!(
        installments[0].amount, installments[1].amount,
        "First installments should be equal"
    );

    // Last installment may differ
    assert!(
        installments[2].amount >= Decimal::ZERO,
        "Last installment must be non-negative"
    );
}

proptest! {
    /// Property: Total of installments always equals input total
    /// FR-072: Last installment absorbs rounding
    #[test]
    fn prop_installment_sum_equals_total(
        total in 100u64..1000000u64,
        count in 2usize..12usize,
    ) {
        let calculator = InstallmentCalculator::new();
        let total_amount = Decimal::from(total) / Decimal::from(100); // Convert cents to dollars

        let installments = calculator.calculate_equal_installments(
            total_amount,
            count,
            None,
            None,
        ).expect("Failed to calculate installments");

        let calculated_total: Decimal = installments.iter()
            .map(|i| i.amount)
            .sum();

        prop_assert_eq!(calculated_total, total_amount, "Installments must sum exactly to total");
    }

    /// Property: Tax distribution maintains total tax
    /// FR-059: Proportional tax distribution
    #[test]
    fn prop_tax_distribution_maintains_total(
        total in 100u64..1000000u64,
        tax in 0u64..100000u64,
        count in 2usize..12usize,
    ) {
        let calculator = InstallmentCalculator::new();
        let total_amount = Decimal::from(total) / Decimal::from(100);
        let total_tax = Decimal::from(tax) / Decimal::from(100);

        let installments = calculator.calculate_equal_installments(
            total_amount,
            count,
            Some(total_tax),
            None,
        ).expect("Failed to calculate installments");

        let calculated_tax: Decimal = installments.iter()
            .map(|i| i.tax_amount)
            .sum();

        prop_assert_eq!(calculated_tax, total_tax, "Tax must sum to total tax");
    }

    /// Property: Service fee distribution maintains total fee
    /// FR-060: Proportional service fee distribution
    #[test]
    fn prop_service_fee_distribution_maintains_total(
        total in 100u64..1000000u64,
        fee in 0u64..50000u64,
        count in 2usize..12usize,
    ) {
        let calculator = InstallmentCalculator::new();
        let total_amount = Decimal::from(total) / Decimal::from(100);
        let total_fee = Decimal::from(fee) / Decimal::from(100);

        let installments = calculator.calculate_equal_installments(
            total_amount,
            count,
            None,
            Some(total_fee),
        ).expect("Failed to calculate installments");

        let calculated_fee: Decimal = installments.iter()
            .map(|i| i.service_fee_amount)
            .sum();

        prop_assert_eq!(calculated_fee, total_fee, "Service fees must sum to total fee");
    }

    /// Property: Custom amounts must sum to total
    #[test]
    fn prop_custom_amounts_sum_to_total(
        amounts in prop::collection::vec(100u64..100000u64, 2..5),
    ) {
        let calculator = InstallmentCalculator::new();

        let custom_amounts: Vec<Decimal> = amounts.iter()
            .map(|&a| Decimal::from(a) / Decimal::from(100))
            .collect();

        let total_amount: Decimal = custom_amounts.iter().sum();
        let count = custom_amounts.len();

        let installments = calculator.calculate_custom_installments(
            total_amount,
            count,
            custom_amounts.clone(),
            None,
            None,
        ).expect("Failed to calculate custom installments");

        let calculated_total: Decimal = installments.iter()
            .map(|i| i.amount)
            .sum();

        prop_assert_eq!(calculated_total, total_amount, "Custom installments must sum to total");
    }

    /// Property: All installments are non-negative
    #[test]
    fn prop_no_negative_installments(
        total in 100u64..1000000u64,
        count in 2usize..12usize,
    ) {
        let calculator = InstallmentCalculator::new();
        let total_amount = Decimal::from(total) / Decimal::from(100);

        let installments = calculator.calculate_equal_installments(
            total_amount,
            count,
            None,
            None,
        ).expect("Failed to calculate installments");

        for installment in installments {
            prop_assert!(installment.amount >= Decimal::ZERO, "All installments must be non-negative");
        }
    }
}
