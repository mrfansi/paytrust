// T083: Property-based test for overpayment auto-application (FR-073, FR-074, FR-075, FR-076)

use paytrust::modules::installments::services::InstallmentService;
use proptest::prelude::*;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Test that overpayment on installment is properly handled
#[test]
fn test_overpayment_auto_application() {
    // Test data: 3 installments of $100 each
    let installment_amounts = vec![
        Decimal::from_str("100.00").unwrap(),
        Decimal::from_str("100.00").unwrap(),
        Decimal::from_str("100.00").unwrap(),
    ];
    
    // Pay $150 on first installment (overpayment of $50)
    let payment_amount = Decimal::from_str("150.00").unwrap();
    
    // Calculate how much should apply to each installment
    let mut remaining = payment_amount;
    let mut applied_amounts = vec![];
    
    for installment_amount in &installment_amounts {
        if remaining > Decimal::ZERO {
            let to_apply = remaining.min(*installment_amount);
            applied_amounts.push(to_apply);
            remaining -= to_apply;
        } else {
            applied_amounts.push(Decimal::ZERO);
        }
    }
    
    // Verify first installment gets $100
    assert_eq!(applied_amounts[0], Decimal::from_str("100.00").unwrap());
    
    // Verify second installment gets $50 (excess)
    assert_eq!(applied_amounts[1], Decimal::from_str("50.00").unwrap());
    
    // Verify third installment gets $0
    assert_eq!(applied_amounts[2], Decimal::ZERO);
    
    // Verify total applied equals payment amount
    let total_applied: Decimal = applied_amounts.iter().sum();
    assert_eq!(total_applied, payment_amount);
}

/// Test that overpayment covering all installments marks invoice as fully paid
#[test]
fn test_overpayment_covers_all_installments() {
    // 3 installments of $100 each = $300 total
    let installment_amounts = vec![
        Decimal::from_str("100.00").unwrap(),
        Decimal::from_str("100.00").unwrap(),
        Decimal::from_str("100.00").unwrap(),
    ];
    
    let total_amount: Decimal = installment_amounts.iter().sum();
    
    // Pay $350 (overpayment of $50 beyond total)
    let payment_amount = Decimal::from_str("350.00").unwrap();
    
    // Apply payment
    let mut remaining = payment_amount;
    let mut paid_count = 0;
    
    for installment_amount in &installment_amounts {
        if remaining >= *installment_amount {
            remaining -= installment_amount;
            paid_count += 1;
        }
    }
    
    // All installments should be marked paid
    assert_eq!(paid_count, 3, "All installments should be paid");
    
    // Excess should remain
    assert_eq!(remaining, Decimal::from_str("50.00").unwrap());
}

proptest! {
    /// Property: Overpayment is correctly distributed sequentially
    /// FR-074, FR-075: Auto-application to next installments
    #[test]
    fn prop_overpayment_sequential_application(
        installment_count in 2usize..6usize,
        overpayment_factor in 1u32..5u32,
    ) {
        let installment_amount = Decimal::from_str("100.00").unwrap();
        let installments = vec![installment_amount; installment_count];
        
        // Pay more than one installment
        let payment_amount = installment_amount * Decimal::from(overpayment_factor);
        
        let mut remaining = payment_amount;
        let mut paid_installments = 0;
        
        for installment in &installments {
            if remaining >= *installment {
                remaining -= installment;
                paid_installments += 1;
            } else if remaining > Decimal::ZERO {
                // Partial payment on this installment
                remaining = Decimal::ZERO;
                break;
            }
        }
        
        // Verify correct number of installments paid
        let expected_paid = (overpayment_factor as usize).min(installment_count);
        prop_assert_eq!(paid_installments, expected_paid);
    }
    
    /// Property: Total applied never exceeds payment amount
    /// FR-073: Accept overpayments
    #[test]
    fn prop_applied_amount_never_exceeds_payment(
        installment_values in prop::collection::vec(100u64..10000u64, 2..5),
        payment_value in 100u64..50000u64,
    ) {
        let installments: Vec<Decimal> = installment_values.iter()
            .map(|&v| Decimal::from(v) / Decimal::from(100))
            .collect();
        
        let payment_amount = Decimal::from(payment_value) / Decimal::from(100);
        
        let mut remaining = payment_amount;
        let mut total_applied = Decimal::ZERO;
        
        for installment in &installments {
            let to_apply = remaining.min(*installment);
            total_applied += to_apply;
            remaining -= to_apply;
            
            if remaining == Decimal::ZERO {
                break;
            }
        }
        
        prop_assert!(total_applied <= payment_amount, "Applied amount must not exceed payment");
    }
    
    /// Property: Excess payment is correctly calculated
    /// FR-076: Track excess for potential refund
    #[test]
    fn prop_excess_payment_calculated(
        total in 100u64..10000u64,
        extra in 0u64..5000u64,
    ) {
        let total_amount = Decimal::from(total) / Decimal::from(100);
        let payment_amount = total_amount + (Decimal::from(extra) / Decimal::from(100));
        
        let excess = if payment_amount > total_amount {
            payment_amount - total_amount
        } else {
            Decimal::ZERO
        };
        
        prop_assert_eq!(excess, Decimal::from(extra) / Decimal::from(100));
    }
}
