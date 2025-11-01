// T093: Integration test for invoice creation with installments
// Tests invoice + installment schedule creation in single transaction

use paytrust::core::{Currency, Result};
use paytrust::modules::{
    gateways::repositories::GatewayRepository,
    installments::{
        models::InstallmentConfig,
        repositories::InstallmentRepository,
    },
    invoices::{
        models::LineItem,
        services::InvoiceService,
    },
};
use rust_decimal::Decimal;
use sqlx::MySqlPool;
use std::str::FromStr;

/// Helper to create test database pool
async fn create_test_pool() -> MySqlPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:password@localhost:3306/paytrust_test".to_string());

    MySqlPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

#[tokio::test]
#[ignore = "Requires test database configuration"]
async fn test_create_invoice_with_installments() -> Result<()> {
    let pool = create_test_pool().await;
    
    let service = InvoiceService::new(pool.clone());
    let installment_repo = InstallmentRepository::new(pool.clone());
    
    // Create invoice with 3 installments
    let line_items = vec![
        LineItem::new(
            "Product A".to_string(),
            1,
            Decimal::from_str("100.00").unwrap(),
            Currency::USD,
        )?,
    ];
    
    let installment_config = InstallmentConfig {
        installment_count: 3,
        custom_amounts: None,
    };
    
    let invoice = service
        .create_invoice(
            "INV-INSTALLMENT-001".to_string(),
            "gateway-xendit".to_string(),
            Currency::USD,
            line_items,
            Some(installment_config),
        )
        .await?;
    
    // Verify invoice created
    let invoice_id = invoice.id.unwrap();
    assert_eq!(invoice.external_id, "INV-INSTALLMENT-001");
    assert_eq!(invoice.subtotal, Some(Decimal::from_str("100.00").unwrap()));
    
    // Verify installment schedules created
    let schedules = installment_repo.find_by_invoice(&invoice_id).await?;
    assert_eq!(schedules.len(), 3, "Should have 3 installment schedules");
    
    // Verify equal distribution
    let expected_amount = Decimal::from_str("33.34").unwrap(); // Last gets 33.32
    assert_eq!(schedules[0].installment_number, 1);
    assert_eq!(schedules[0].amount, expected_amount);
    assert_eq!(schedules[1].installment_number, 2);
    assert_eq!(schedules[1].amount, expected_amount);
    assert_eq!(schedules[2].installment_number, 3);
    assert_eq!(schedules[2].amount, Decimal::from_str("33.32").unwrap()); // Absorbs rounding
    
    // Verify total equals invoice subtotal
    let total: Decimal = schedules.iter().map(|s| s.amount).sum();
    assert_eq!(total, Decimal::from_str("100.00").unwrap());
    
    Ok(())
}

#[tokio::test]
#[ignore = "Requires test database configuration"]
async fn test_create_invoice_with_custom_installment_amounts() -> Result<()> {
    let pool = create_test_pool().await;
    
    let service = InvoiceService::new(pool.clone());
    let installment_repo = InstallmentRepository::new(pool.clone());
    
    // Create invoice with custom installment amounts
    let line_items = vec![
        LineItem::new(
            "Product B".to_string(),
            1,
            Decimal::from_str("200.00").unwrap(),
            Currency::USD,
        )?,
    ];
    
    let installment_config = InstallmentConfig {
        installment_count: 2,
        custom_amounts: Some(vec![
            Decimal::from_str("80.00").unwrap(),
            Decimal::from_str("120.00").unwrap(),
        ]),
    };
    
    let invoice = service
        .create_invoice(
            "INV-CUSTOM-001".to_string(),
            "gateway-xendit".to_string(),
            Currency::USD,
            line_items,
            Some(installment_config),
        )
        .await?;
    
    // Verify installment schedules with custom amounts
    let invoice_id = invoice.id.unwrap();
    let schedules = installment_repo.find_by_invoice(&invoice_id).await?;
    assert_eq!(schedules.len(), 2);
    assert_eq!(schedules[0].amount, Decimal::from_str("80.00").unwrap());
    assert_eq!(schedules[1].amount, Decimal::from_str("120.00").unwrap());
    
    Ok(())
}

#[tokio::test]
#[ignore = "Requires test database configuration"]
async fn test_create_invoice_without_installments() -> Result<()> {
    let pool = create_test_pool().await;
    
    let service = InvoiceService::new(pool.clone());
    let installment_repo = InstallmentRepository::new(pool.clone());
    
    // Create regular invoice without installments
    let line_items = vec![
        LineItem::new(
            "Product C".to_string(),
            1,
            Decimal::from_str("50.00").unwrap(),
            Currency::USD,
        )?,
    ];
    
    let invoice = service
        .create_invoice(
            "INV-REGULAR-001".to_string(),
            "gateway-xendit".to_string(),
            Currency::USD,
            line_items,
            None, // No installment config
        )
        .await?;
    
    // Verify no installment schedules created
    let invoice_id = invoice.id.unwrap();
    let schedules = installment_repo.find_by_invoice(&invoice_id).await?;
    assert_eq!(schedules.len(), 0, "Should have no installment schedules");
    
    Ok(())
}

#[tokio::test]
#[ignore = "Requires test database configuration"]
async fn test_create_invoice_with_installments_and_tax() -> Result<()> {
    let pool = create_test_pool().await;
    
    let service = InvoiceService::new(pool.clone());
    let installment_repo = InstallmentRepository::new(pool.clone());
    
    // Create invoice with tax and installments
    let line_items = vec![
        LineItem::new_with_tax(
            "Product D".to_string(),
            1,
            Decimal::from_str("100.00").unwrap(),
            Currency::USD,
            Decimal::from_str("0.10").unwrap(), // 10% tax
            Some("VAT".to_string()),
        )?,
    ];
    
    let installment_config = InstallmentConfig {
        installment_count: 2,
        custom_amounts: None,
    };
    
    let invoice = service
        .create_invoice(
            "INV-TAX-001".to_string(),
            "gateway-xendit".to_string(),
            Currency::USD,
            line_items,
            Some(installment_config),
        )
        .await?;
    
    // Verify tax is distributed proportionally (FR-059)
    let invoice_id = invoice.id.unwrap();
    let schedules = installment_repo.find_by_invoice(&invoice_id).await?;
    assert_eq!(schedules.len(), 2);
    
    // Each installment should have proportional tax
    // Subtotal: $100, Tax: $10 (10%)
    // Per installment: $50 subtotal, $5 tax
    assert_eq!(schedules[0].amount, Decimal::from_str("50.00").unwrap());
    assert_eq!(schedules[0].tax_amount, Decimal::from_str("5.00").unwrap());
    assert_eq!(schedules[1].amount, Decimal::from_str("50.00").unwrap());
    assert_eq!(schedules[1].tax_amount, Decimal::from_str("5.00").unwrap());
    
    Ok(())
}
