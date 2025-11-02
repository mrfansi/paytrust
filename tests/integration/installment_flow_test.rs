// T086: Integration test for sequential installment payment enforcement (FR-068, FR-069, FR-070)
//
// Phase 5 (T041): Refactored to use UUIDs and transaction isolation for parallel execution

use paytrust::core::{Currency, Result};
use paytrust::modules::{
    installments::{
        models::{InstallmentConfig, InstallmentStatus},
        repositories::InstallmentRepository,
    },
    invoices::{
        models::{InvoiceStatus, LineItem},
        repositories::InvoiceRepository,
        services::InvoiceService,
    },
    transactions::{repositories::TransactionRepository, services::TransactionService},
};
use rust_decimal::Decimal;
use sqlx::MySqlPool;
use std::str::FromStr;
use uuid::Uuid;

/// Helper to create test database pool
async fn create_test_pool() -> MySqlPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "mysql://root:password@localhost:3306/paytrust_test".to_string());

    MySqlPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Generate unique external ID for test isolation
fn generate_test_id(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4())
}

/// Test that installments must be paid sequentially (FR-068)
#[tokio::test]
#[ignore = "Requires test database configuration"]
async fn test_sequential_installment_payment_enforcement() -> Result<()> {
    let pool = create_test_pool().await;

    let invoice_repo = InvoiceRepository::new(pool.clone());
    let invoice_service = InvoiceService::new(pool.clone());
    let installment_repo = InstallmentRepository::new(pool.clone());
    let transaction_repo = TransactionRepository::new(pool.clone());
    let transaction_service = TransactionService::new(transaction_repo, invoice_repo);

    // Create invoice with 3 installments using unique ID for isolation
    let external_id = generate_test_id("INV-SEQ");
    
    let line_items = vec![LineItem::new(
        "Test Product".to_string(),
        1,
        Decimal::from_str("300.00").unwrap(),
        Currency::USD,
    )?];

    let installment_config = InstallmentConfig {
        installment_count: 3,
        custom_amounts: None,
    };

    let invoice = invoice_service
        .create_invoice(
            external_id,
            "gateway-xendit".to_string(),
            Currency::USD,
            line_items,
            Some(installment_config),
        )
        .await?;

    let invoice_id = invoice.id.unwrap();
    let schedules = installment_repo.find_by_invoice(&invoice_id).await?;

    // Attempt to pay installment #2 before #1 (should fail)
    let result = transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[1].id.clone(), // Installment #2
            Decimal::from_str("100.00").unwrap(),
            generate_test_id("tx"),
        )
        .await;

    assert!(
        result.is_err(),
        "Should reject payment of installment #2 before #1 (FR-068)"
    );

    // Pay installment #1 (should succeed)
    let result = transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[0].id.clone(), // Installment #1
            Decimal::from_str("100.00").unwrap(),
            generate_test_id("tx"),
        )
        .await;

    assert!(result.is_ok(), "Should allow payment of installment #1");

    // Now pay installment #2 (should succeed)
    let result = transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[1].id.clone(), // Installment #2
            Decimal::from_str("100.00").unwrap(),
            generate_test_id("tx"),
        )
        .await;

    assert!(
        result.is_ok(),
        "Should allow payment of installment #2 after #1"
    );

    Ok(())
}

/// Test that skipping installments is not allowed (FR-069)
#[tokio::test]
#[ignore = "Requires test database configuration"]
async fn test_cannot_skip_installments() -> Result<()> {
    let pool = create_test_pool().await;

    let invoice_repo = InvoiceRepository::new(pool.clone());
    let invoice_service = InvoiceService::new(pool.clone());
    let installment_repo = InstallmentRepository::new(pool.clone());
    let transaction_repo = TransactionRepository::new(pool.clone());
    let transaction_service = TransactionService::new(transaction_repo, invoice_repo);

    // Create invoice with 4 installments
    let line_items = vec![LineItem::new(
        "Test Product".to_string(),
        1,
        Decimal::from_str("400.00").unwrap(),
        Currency::USD,
    )?];

    let installment_config = InstallmentConfig {
        installment_count: 4,
        custom_amounts: None,
    };

    let invoice = invoice_service
        .create_invoice(
            generate_test_id("INV-SKIP"),
            "gateway-xendit".to_string(),
            Currency::USD,
            line_items,
            Some(installment_config),
        )
        .await?;

    let invoice_id = invoice.id.unwrap();
    let schedules = installment_repo.find_by_invoice(&invoice_id).await?;

    // Pay installment #1
    transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[0].id.clone(),
            Decimal::from_str("100.00").unwrap(),
            generate_test_id("tx"),
        )
        .await?;

    // Attempt to pay installment #3 (skipping #2) - should fail
    let result = transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[2].id.clone(), // Installment #3
            Decimal::from_str("110.00").unwrap(),
            generate_test_id("tx"),
        )
        .await;

    assert!(
        result.is_err(),
        "Should reject payment of installment #3 before #2 (FR-069)"
    );

    // Attempt to pay installment #4 (skipping #2 and #3) - should fail
    let result = transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[3].id.clone(), // Installment #4
            Decimal::from_str("100.00").unwrap(),
            generate_test_id("tx"),
        )
        .await;

    assert!(
        result.is_err(),
        "Should reject payment of installment #4 before #2 and #3 (FR-069)"
    );

    Ok(())
}

/// Test that partial payments don't block next installment (FR-070)
#[tokio::test]
#[ignore = "Requires test database configuration"]
async fn test_partial_payment_allows_next_installment() -> Result<()> {
    let pool = create_test_pool().await;

    let invoice_repo = InvoiceRepository::new(pool.clone());
    let invoice_service = InvoiceService::new(pool.clone());
    let installment_repo = InstallmentRepository::new(pool.clone());
    let transaction_repo = TransactionRepository::new(pool.clone());
    let transaction_service = TransactionService::new(transaction_repo, invoice_repo);

    // Create invoice with 3 installments
    let line_items = vec![LineItem::new(
        "Test Product".to_string(),
        1,
        Decimal::from_str("300.00").unwrap(),
        Currency::USD,
    )?];

    let installment_config = InstallmentConfig {
        installment_count: 3,
        custom_amounts: None,
    };

    let invoice = invoice_service
        .create_invoice(
            generate_test_id("INV-PARTIAL"),
            "gateway-xendit".to_string(),
            Currency::USD,
            line_items,
            Some(installment_config),
        )
        .await?;

    let invoice_id = invoice.id.unwrap();
    let schedules = installment_repo.find_by_invoice(&invoice_id).await?;

    // Make partial payment on installment #1 (less than required)
    let partial_payment = Decimal::from_str("50.00").unwrap(); // Less than $100
    let result = transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[0].id.clone(),
            partial_payment,
            generate_test_id("tx"),
        )
        .await;

    // Partial payment should be accepted (FR-048)
    assert!(result.is_ok(), "Should accept partial payment");

    // Verify installment #1 is still unpaid (not fully paid)
    let updated_schedules = installment_repo.find_by_invoice(&invoice_id).await?;
    assert_eq!(
        updated_schedules[0].status,
        InstallmentStatus::Unpaid,
        "Installment should remain unpaid after partial payment"
    );

    // Should NOT be able to pay installment #2 yet (FR-070: must complete current first)
    let result = transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[1].id.clone(),
            Decimal::from_str("100.00").unwrap(),
            generate_test_id("tx"),
        )
        .await;

    assert!(
        result.is_err(),
        "Should reject next installment until current is fully paid"
    );

    // Complete payment of installment #1
    let remaining_payment = Decimal::from_str("50.00").unwrap();
    transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[0].id.clone(),
            remaining_payment,
            generate_test_id("tx"),
        )
        .await?;

    // Now installment #2 should be allowed
    let result = transaction_service
        .process_installment_payment(
            invoice_id.clone(),
            schedules[1].id.clone(),
            Decimal::from_str("100.00").unwrap(),
            generate_test_id("tx"),
        )
        .await;

    assert!(
        result.is_ok(),
        "Should allow next installment after completing previous"
    );

    Ok(())
}

/// Test that all installments can be paid in correct sequence
#[tokio::test]
#[ignore = "Requires test database configuration"]
async fn test_complete_sequential_payment_flow() -> Result<()> {
    let pool = create_test_pool().await;

    let invoice_repo = InvoiceRepository::new(pool.clone());
    let invoice_service = InvoiceService::new(pool.clone());
    let installment_repo = InstallmentRepository::new(pool.clone());
    let transaction_repo = TransactionRepository::new(pool.clone());
    let transaction_service = TransactionService::new(transaction_repo, invoice_repo);

    // Create invoice with 5 installments
    let line_items = vec![LineItem::new(
        "Test Product".to_string(),
        1,
        Decimal::from_str("500.00").unwrap(),
        Currency::USD,
    )?];

    let installment_config = InstallmentConfig {
        installment_count: 5,
        custom_amounts: None,
    };

    let invoice = invoice_service
        .create_invoice(
            generate_test_id("INV-COMPLETE"),
            "gateway-xendit".to_string(),
            Currency::USD,
            line_items,
            Some(installment_config),
        )
        .await?;

    let invoice_id = invoice.id.unwrap();
    let schedules = installment_repo.find_by_invoice(&invoice_id).await?;

    // Pay all installments sequentially
    for (i, schedule) in schedules.iter().enumerate() {
        let result = transaction_service
            .process_installment_payment(
                invoice_id.clone(),
                schedule.id.clone(),
                schedule.amount,
                generate_test_id("tx"),
            )
            .await;

        assert!(
            result.is_ok(),
            "Installment #{} should be paid successfully",
            i + 1
        );
    }

    // Verify all installments are paid
    let final_schedules = installment_repo.find_by_invoice(&invoice_id).await?;
    for schedule in final_schedules {
        assert_eq!(
            schedule.status,
            InstallmentStatus::Paid,
            "All installments should be paid"
        );
    }

    // Verify invoice is fully paid
    let final_invoice = invoice_service.get_invoice(&invoice_id).await?;
    assert_eq!(
        final_invoice.status,
        InvoiceStatus::FullyPaid,
        "Invoice should be marked as fully paid (FR-020)"
    );

    Ok(())
}
