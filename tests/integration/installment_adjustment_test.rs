// T087: Integration test for installment adjustment after first payment (FR-077, FR-078, FR-079, FR-080)

use paytrust::core::{Currency, Result};
use paytrust::modules::{
    installments::{
        models::{InstallmentConfig, InstallmentStatus},
        repositories::InstallmentRepository,
        services::InstallmentService,
    },
    invoices::{
        models::LineItem,
        services::InvoiceService,
    },
    transactions::services::TransactionService,
};
use rust_decimal::Decimal;
use sqlx::MySqlPool;
use std::str::FromStr;

/// Helper to create test database pool
async fn create_test_pool() -> MySqlPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql::root:password@localhost:3306/paytrust_test".to_string());

    MySqlPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Test that unpaid installments can be adjusted after first payment (FR-077)
#[tokio::test]
#[ignore = "Requires test database configuration"]
async fn test_adjust_unpaid_installments_after_first_payment() -> Result<()> {
    let pool = create_test_pool().await;
    
    let invoice_service = InvoiceService::new(pool.clone());
    let installment_service = InstallmentService::new(pool.clone());
    let installment_repo = InstallmentRepository::new(pool.clone());
    let transaction_service = TransactionService::new(pool.clone());
    
    // Create invoice with 3 equal installments of $100 each
    let line_items = vec![
        LineItem::new(
            "Test Product".to_string(),
            1,
            Decimal::from_str("300.00").unwrap(),
            Currency::USD,
        )?,
    ];
    
    let installment_config = InstallmentConfig {
        installment_count: 3,
        custom_amounts: None,
    };
    
    let invoice = invoice_service
        .create_invoice(
            "INV-ADJUST-001".to_string(),
            "gateway-xendit".to_string(),
            Currency::USD,
            line_items,
            Some(installment_config),
        )
        .await?;
    
    let invoice_id = invoice.id.unwrap();
    let schedules = installment_repo.find_by_invoice(&invoice_id).await?;
    
    // Pay first installment
    transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[0].id.unwrap(),
            Decimal::from_str("100.00").unwrap(),
            "gateway-tx-adjust-001".to_string(),
        )
        .await?;
    
    // Adjust remaining installments (change $100, $100 to $80, $120)
    let adjustments = vec![
        (2, Decimal::from_str("80.00").unwrap()),  // Installment #2
        (3, Decimal::from_str("120.00").unwrap()), // Installment #3
    ];
    
    let result = installment_service
        .adjust_installments(&invoice_id, adjustments)
        .await;
    
    assert!(result.is_ok(), "Should allow adjustment of unpaid installments");
    
    // Verify adjustments applied
    let updated_schedules = installment_repo.find_by_invoice(&invoice_id).await?;
    assert_eq!(updated_schedules[1].amount, Decimal::from_str("80.00").unwrap());
    assert_eq!(updated_schedules[2].amount, Decimal::from_str("120.00").unwrap());
    
    // Verify first installment unchanged (already paid)
    assert_eq!(updated_schedules[0].amount, Decimal::from_str("100.00").unwrap());
    assert_eq!(updated_schedules[0].status, InstallmentStatus::Paid);
    
    Ok(())
}

/// Test that paid installments cannot be adjusted (FR-078)
#[tokio::test]
#[ignore = "Requires test database configuration"]
async fn test_cannot_adjust_paid_installments() -> Result<()> {
    let pool = create_test_pool().await;
    
    let invoice_service = InvoiceService::new(pool.clone());
    let installment_service = InstallmentService::new(pool.clone());
    let installment_repo = InstallmentRepository::new(pool.clone());
    let transaction_service = TransactionService::new(pool.clone());
    
    // Create invoice with 3 installments
    let line_items = vec![
        LineItem::new(
            "Test Product".to_string(),
            1,
            Decimal::from_str("300.00").unwrap(),
            Currency::USD,
        )?,
    ];
    
    let installment_config = InstallmentConfig {
        installment_count: 3,
        custom_amounts: None,
    };
    
    let invoice = invoice_service
        .create_invoice(
            "INV-LOCKED-001".to_string(),
            "gateway-xendit".to_string(),
            Currency::USD,
            line_items,
            Some(installment_config),
        )
        .await?;
    
    let invoice_id = invoice.id.unwrap();
    let schedules = installment_repo.find_by_invoice(&invoice_id).await?;
    
    // Pay first two installments
    transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[0].id.unwrap(),
            Decimal::from_str("100.00").unwrap(),
            "gateway-tx-locked-001".to_string(),
        )
        .await?;
    
    transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[1].id.unwrap(),
            Decimal::from_str("100.00").unwrap(),
            "gateway-tx-locked-002".to_string(),
        )
        .await?;
    
    // Attempt to adjust first installment (already paid) - should fail
    let adjustments = vec![
        (1, Decimal::from_str("150.00").unwrap()), // Try to adjust paid installment
    ];
    
    let result = installment_service
        .adjust_installments(&invoice_id, adjustments)
        .await;
    
    assert!(result.is_err(), "Should reject adjustment of paid installment (FR-078)");
    
    // Attempt to adjust second installment (already paid) - should fail
    let adjustments = vec![
        (2, Decimal::from_str("80.00").unwrap()), // Try to adjust paid installment
    ];
    
    let result = installment_service
        .adjust_installments(&invoice_id, adjustments)
        .await;
    
    assert!(result.is_err(), "Should reject adjustment of paid installment (FR-078)");
    
    Ok(())
}

/// Test that adjusted amounts must sum to remaining balance (FR-079)
#[tokio::test]
#[ignore = "Requires test database configuration"]
async fn test_adjusted_amounts_must_match_remaining_balance() -> Result<()> {
    let pool = create_test_pool().await;
    
    let invoice_service = InvoiceService::new(pool.clone());
    let installment_service = InstallmentService::new(pool.clone());
    let installment_repo = InstallmentRepository::new(pool.clone());
    let transaction_service = TransactionService::new(pool.clone());
    
    // Create invoice with 4 installments of $100 each = $400 total
    let line_items = vec![
        LineItem::new(
            "Test Product".to_string(),
            1,
            Decimal::from_str("400.00").unwrap(),
            Currency::USD,
        )?,
    ];
    
    let installment_config = InstallmentConfig {
        installment_count: 4,
        custom_amounts: None,
    };
    
    let invoice = invoice_service
        .create_invoice(
            "INV-BALANCE-001".to_string(),
            "gateway-xendit".to_string(),
            Currency::USD,
            line_items,
            Some(installment_config),
        )
        .await?;
    
    let invoice_id = invoice.id.unwrap();
    let schedules = installment_repo.find_by_invoice(&invoice_id).await?;
    
    // Pay first installment ($100 paid, $300 remaining)
    transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[0].id.unwrap(),
            Decimal::from_str("100.00").unwrap(),
            "gateway-tx-balance-001".to_string(),
        )
        .await?;
    
    // Attempt to adjust remaining installments with invalid total
    // Original: $100, $100, $100 (total $300)
    // Invalid: $150, $100, $100 (total $350 != $300)
    let invalid_adjustments = vec![
        (2, Decimal::from_str("150.00").unwrap()),
        (3, Decimal::from_str("100.00").unwrap()),
        (4, Decimal::from_str("100.00").unwrap()),
    ];
    
    let result = installment_service
        .adjust_installments(&invoice_id, invalid_adjustments)
        .await;
    
    assert!(
        result.is_err(),
        "Should reject adjustment when sum doesn't match remaining balance (FR-079)"
    );
    
    // Valid adjustment: $150, $80, $70 (total $300)
    let valid_adjustments = vec![
        (2, Decimal::from_str("150.00").unwrap()),
        (3, Decimal::from_str("80.00").unwrap()),
        (4, Decimal::from_str("70.00").unwrap()),
    ];
    
    let result = installment_service
        .adjust_installments(&invoice_id, valid_adjustments)
        .await;
    
    assert!(result.is_ok(), "Should accept adjustment when sum matches remaining balance");
    
    // Verify adjustments applied
    let updated_schedules = installment_repo.find_by_invoice(&invoice_id).await?;
    let remaining_total: Decimal = updated_schedules[1..].iter()
        .map(|s| s.amount)
        .sum();
    
    assert_eq!(
        remaining_total,
        Decimal::from_str("300.00").unwrap(),
        "Remaining installments must sum to remaining balance"
    );
    
    Ok(())
}

/// Test that all unpaid installments can be adjusted simultaneously (FR-080)
#[tokio::test]
#[ignore = "Requires test database configuration"]
async fn test_adjust_all_unpaid_installments_simultaneously() -> Result<()> {
    let pool = create_test_pool().await;
    
    let invoice_service = InvoiceService::new(pool.clone());
    let installment_service = InstallmentService::new(pool.clone());
    let installment_repo = InstallmentRepository::new(pool.clone());
    let transaction_service = TransactionService::new(pool.clone());
    
    // Create invoice with 6 installments
    let line_items = vec![
        LineItem::new(
            "Test Product".to_string(),
            1,
            Decimal::from_str("600.00").unwrap(),
            Currency::USD,
        )?,
    ];
    
    let installment_config = InstallmentConfig {
        installment_count: 6,
        custom_amounts: None,
    };
    
    let invoice = invoice_service
        .create_invoice(
            "INV-MULTI-001".to_string(),
            "gateway-xendit".to_string(),
            Currency::USD,
            line_items,
            Some(installment_config),
        )
        .await?;
    
    let invoice_id = invoice.id.unwrap();
    let schedules = installment_repo.find_by_invoice(&invoice_id).await?;
    
    // Pay first two installments
    transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[0].id.unwrap(),
            Decimal::from_str("100.00").unwrap(),
            "gateway-tx-multi-001".to_string(),
        )
        .await?;
    
    transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[1].id.unwrap(),
            Decimal::from_str("100.00").unwrap(),
            "gateway-tx-multi-002".to_string(),
        )
        .await?;
    
    // Adjust all remaining 4 unpaid installments
    // Remaining balance: $400 (original $600 - $200 paid)
    let adjustments = vec![
        (3, Decimal::from_str("50.00").unwrap()),
        (4, Decimal::from_str("75.00").unwrap()),
        (5, Decimal::from_str("125.00").unwrap()),
        (6, Decimal::from_str("150.00").unwrap()),
    ];
    
    let result = installment_service
        .adjust_installments(&invoice_id, adjustments)
        .await;
    
    assert!(result.is_ok(), "Should allow simultaneous adjustment of all unpaid installments");
    
    // Verify all adjustments applied
    let updated_schedules = installment_repo.find_by_invoice(&invoice_id).await?;
    assert_eq!(updated_schedules[2].amount, Decimal::from_str("50.00").unwrap());
    assert_eq!(updated_schedules[3].amount, Decimal::from_str("75.00").unwrap());
    assert_eq!(updated_schedules[4].amount, Decimal::from_str("125.00").unwrap());
    assert_eq!(updated_schedules[5].amount, Decimal::from_str("150.00").unwrap());
    
    // Verify first two remain unchanged
    assert_eq!(updated_schedules[0].amount, Decimal::from_str("100.00").unwrap());
    assert_eq!(updated_schedules[1].amount, Decimal::from_str("100.00").unwrap());
    
    // Verify total still matches invoice total
    let total: Decimal = updated_schedules.iter().map(|s| s.amount).sum();
    assert_eq!(total, Decimal::from_str("600.00").unwrap());
    
    Ok(())
}

/// Test proportional tax and fee distribution after adjustment
#[tokio::test]
#[ignore = "Requires test database configuration"]
async fn test_adjustment_maintains_proportional_tax_and_fees() -> Result<()> {
    let pool = create_test_pool().await;
    
    let invoice_service = InvoiceService::new(pool.clone());
    let installment_service = InstallmentService::new(pool.clone());
    let installment_repo = InstallmentRepository::new(pool.clone());
    let transaction_service = TransactionService::new(pool.clone());
    
    // Create invoice with tax and service fee
    let line_items = vec![
        LineItem::new_with_tax(
            "Test Product".to_string(),
            1,
            Decimal::from_str("200.00").unwrap(),
            Currency::USD,
            Decimal::from_str("0.10").unwrap(), // 10% tax = $20
            Some("VAT".to_string()),
        )?,
    ];
    
    // Service fee will be calculated by gateway (assume 3% + $1 = $7)
    // Total: $200 + $20 (tax) + $7 (fee) = $227
    
    let installment_config = InstallmentConfig {
        installment_count: 3,
        custom_amounts: None,
    };
    
    let invoice = invoice_service
        .create_invoice(
            "INV-PROP-001".to_string(),
            "gateway-xendit".to_string(),
            Currency::USD,
            line_items,
            Some(installment_config),
        )
        .await?;
    
    let invoice_id = invoice.id.unwrap();
    let schedules = installment_repo.find_by_invoice(&invoice_id).await?;
    
    // Pay first installment
    transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[0].id.unwrap(),
            schedules[0].amount,
            "gateway-tx-prop-001".to_string(),
        )
        .await?;
    
    // Adjust remaining installments
    let total_remaining = schedules[1].amount + schedules[2].amount;
    let adjustments = vec![
        (2, total_remaining * Decimal::from_str("0.6").unwrap()), // 60%
        (3, total_remaining * Decimal::from_str("0.4").unwrap()), // 40%
    ];
    
    installment_service
        .adjust_installments(&invoice_id, adjustments)
        .await?;
    
    // Verify tax and fees redistributed proportionally
    let updated_schedules = installment_repo.find_by_invoice(&invoice_id).await?;
    
    // Total tax and fees for remaining installments should match original
    let remaining_tax: Decimal = updated_schedules[1..].iter()
        .map(|s| s.tax_amount)
        .sum();
    let remaining_fee: Decimal = updated_schedules[1..].iter()
        .map(|s| s.service_fee_amount)
        .sum();
    
    let original_remaining_tax = schedules[1].tax_amount + schedules[2].tax_amount;
    let original_remaining_fee = schedules[1].service_fee_amount + schedules[2].service_fee_amount;
    
    assert_eq!(remaining_tax, original_remaining_tax, "Tax should be redistributed proportionally");
    assert_eq!(remaining_fee, original_remaining_fee, "Service fee should be redistributed proportionally");
    
    Ok(())
}
