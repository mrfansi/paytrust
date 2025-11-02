// Performance test for NFR-001: Invoice creation < 2s
// Tests invoice creation time under various scenarios

use paytrust::config::database::DatabaseConfig;
use paytrust::core::Currency;
use paytrust::modules::invoices::{
    models::LineItem,
    services::InvoiceService,
};
use rust_decimal::Decimal;
use std::time::Instant;

async fn create_test_pool() -> sqlx::MySqlPool {
    // Load .env file
    dotenvy::dotenv().ok();
    
    let db_config = DatabaseConfig::from_env().expect("Failed to load database config");
    db_config.create_pool().await.expect("Failed to create pool")
}

#[tokio::test]
async fn test_nfr001_invoice_creation_single_item_under_2s() {
    let pool = create_test_pool().await;
    let service = InvoiceService::new(pool.clone());

    // Create invoice with 1 line item
    let line_items = vec![LineItem::new(
        "Test Product".to_string(),
        1,
        Decimal::new(100000, 0), // 100,000 IDR
        Currency::IDR,
    )
    .expect("Failed to create line item")];

    let start = Instant::now();
    let result = service
        .create_invoice(
            format!("PERF-TEST-{}", uuid::Uuid::new_v4()),
            "xendit".to_string(),
            Currency::IDR,
            line_items,
            None, // No installments
        )
        .await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Invoice creation failed: {:?}", result.err());
    assert!(
        duration.as_secs_f64() < 2.0,
        "NFR-001 VIOLATION: Invoice creation took {:.3}s (limit: 2s)",
        duration.as_secs_f64()
    );

    println!(
        "✅ Single item invoice created in {:.3}s",
        duration.as_secs_f64()
    );

    // Cleanup
    if let Ok(invoice) = result {
        let _ = sqlx::query("DELETE FROM line_items WHERE invoice_id = ?")
            .bind(invoice.id.as_ref().unwrap())
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM invoices WHERE id = ?")
            .bind(invoice.id.as_ref().unwrap())
            .execute(&pool)
            .await;
    }
}

#[tokio::test]
async fn test_nfr001_invoice_creation_multiple_items_under_2s() {
    let pool = create_test_pool().await;
    let service = InvoiceService::new(pool.clone());

    // Create invoice with 10 line items (realistic scenario)
    let mut line_items = Vec::new();
    for i in 1..=10 {
        line_items.push(
            LineItem::new(
                format!("Product {}", i),
                2,
                Decimal::new(50000 + (i * 1000), 0), // Varying prices
                Currency::IDR,
            )
            .expect("Failed to create line item"),
        );
    }

    let start = Instant::now();
    let result = service
        .create_invoice(
            format!("PERF-TEST-{}", uuid::Uuid::new_v4()),
            "xendit".to_string(),
            Currency::IDR,
            line_items,
            None,
        )
        .await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Invoice creation failed: {:?}", result.err());
    assert!(
        duration.as_secs_f64() < 2.0,
        "NFR-001 VIOLATION: Invoice creation with 10 items took {:.3}s (limit: 2s)",
        duration.as_secs_f64()
    );

    println!(
        "✅ 10-item invoice created in {:.3}s",
        duration.as_secs_f64()
    );

    // Cleanup
    if let Ok(invoice) = result {
        let _ = sqlx::query("DELETE FROM line_items WHERE invoice_id = ?")
            .bind(invoice.id.as_ref().unwrap())
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM invoices WHERE id = ?")
            .bind(invoice.id.as_ref().unwrap())
            .execute(&pool)
            .await;
    }
}

#[tokio::test]
async fn test_nfr001_invoice_creation_with_taxes_under_2s() {
    let pool = create_test_pool().await;
    let service = InvoiceService::new(pool.clone());

    // Create invoice with line items having different tax rates
    let line_items = vec![
        LineItem::new_with_tax(
            "Product A".to_string(),
            3,
            Decimal::new(100000, 0),
            Currency::IDR,
            Decimal::new(10, 2), // 10% tax
            Some("VAT".to_string()),
        )
        .expect("Failed to create line item"),
        LineItem::new_with_tax(
            "Product B".to_string(),
            2,
            Decimal::new(75000, 0),
            Currency::IDR,
            Decimal::new(5, 2), // 5% tax
            Some("GST".to_string()),
        )
        .expect("Failed to create line item"),
        LineItem::new(
            "Product C (no tax)".to_string(),
            1,
            Decimal::new(50000, 0),
            Currency::IDR,
        )
        .expect("Failed to create line item"),
    ];

    let start = Instant::now();
    let result = service
        .create_invoice(
            format!("PERF-TEST-{}", uuid::Uuid::new_v4()),
            "xendit".to_string(),
            Currency::IDR,
            line_items,
            None,
        )
        .await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Invoice creation failed: {:?}", result.err());
    assert!(
        duration.as_secs_f64() < 2.0,
        "NFR-001 VIOLATION: Invoice creation with taxes took {:.3}s (limit: 2s)",
        duration.as_secs_f64()
    );

    println!(
        "✅ Invoice with mixed tax rates created in {:.3}s",
        duration.as_secs_f64()
    );

    // Cleanup
    if let Ok(invoice) = result {
        let _ = sqlx::query("DELETE FROM line_items WHERE invoice_id = ?")
            .bind(invoice.id.as_ref().unwrap())
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM invoices WHERE id = ?")
            .bind(invoice.id.as_ref().unwrap())
            .execute(&pool)
            .await;
    }
}

#[tokio::test]
async fn test_nfr001_invoice_creation_with_installments_under_2s() {
    let pool = create_test_pool().await;
    let service = InvoiceService::new(pool.clone());

    use paytrust::modules::installments::models::InstallmentConfig;

    // Create invoice with 6 equal installments
    let line_items = vec![
        LineItem::new(
            "Premium Product".to_string(),
            1,
            Decimal::new(600000, 0), // 600,000 IDR
            Currency::IDR,
        )
        .expect("Failed to create line item"),
    ];

    let installment_config = InstallmentConfig {
        installment_count: 6,
        custom_amounts: None,
    };

    let start = Instant::now();
    let result = service
        .create_invoice(
            format!("PERF-TEST-{}", uuid::Uuid::new_v4()),
            "xendit".to_string(),
            Currency::IDR,
            line_items,
            Some(installment_config),
        )
        .await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Invoice creation failed: {:?}", result.err());
    assert!(
        duration.as_secs_f64() < 2.0,
        "NFR-001 VIOLATION: Invoice creation with 6 installments took {:.3}s (limit: 2s)",
        duration.as_secs_f64()
    );

    println!(
        "✅ Invoice with 6 installments created in {:.3}s",
        duration.as_secs_f64()
    );

    // Cleanup
    if let Ok(invoice) = result {
        let _ = sqlx::query("DELETE FROM installment_schedules WHERE invoice_id = ?")
            .bind(invoice.id.as_ref().unwrap())
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM line_items WHERE invoice_id = ?")
            .bind(invoice.id.as_ref().unwrap())
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM invoices WHERE id = ?")
            .bind(invoice.id.as_ref().unwrap())
            .execute(&pool)
            .await;
    }
}

#[tokio::test]
async fn test_nfr001_invoice_creation_max_complexity_under_2s() {
    let pool = create_test_pool().await;
    let service = InvoiceService::new(pool.clone());

    use paytrust::modules::installments::models::InstallmentConfig;

    // Maximum complexity: 10 items + taxes + 12 installments
    let mut line_items = Vec::new();
    for i in 1..=10 {
        line_items.push(
            LineItem::new_with_tax(
                format!("Product {}", i),
                (i % 3) + 1, // Varying quantities
                Decimal::new(50000 + (i as i64 * 5000), 0),
                Currency::IDR,
                Decimal::new(10, 2), // 10% tax
                Some("VAT".to_string()),
            )
            .expect("Failed to create line item"),
        );
    }

    let installment_config = InstallmentConfig {
        installment_count: 12, // Maximum installments
        custom_amounts: None,
    };

    let start = Instant::now();
    let result = service
        .create_invoice(
            format!("PERF-TEST-{}", uuid::Uuid::new_v4()),
            "xendit".to_string(),
            Currency::IDR,
            line_items,
            Some(installment_config),
        )
        .await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Invoice creation failed: {:?}", result.err());
    assert!(
        duration.as_secs_f64() < 2.0,
        "NFR-001 VIOLATION: Max complexity invoice (10 items + taxes + 12 installments) took {:.3}s (limit: 2s)",
        duration.as_secs_f64()
    );

    println!(
        "✅ Maximum complexity invoice created in {:.3}s",
        duration.as_secs_f64()
    );

    // Cleanup
    if let Ok(invoice) = result {
        let _ = sqlx::query("DELETE FROM installment_schedules WHERE invoice_id = ?")
            .bind(invoice.id.as_ref().unwrap())
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM line_items WHERE invoice_id = ?")
            .bind(invoice.id.as_ref().unwrap())
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM invoices WHERE id = ?")
            .bind(invoice.id.as_ref().unwrap())
            .execute(&pool)
            .await;
    }
}

#[tokio::test]
async fn test_performance_invoice_lookup_by_id() {
    let pool = create_test_pool().await;
    let service = InvoiceService::new(pool.clone());

    // Create test invoice
    let line_items = vec![LineItem::new(
        "Test Product".to_string(),
        1,
        Decimal::new(100000, 0),
        Currency::IDR,
    )
    .expect("Failed to create line item")];

    let invoice = service
        .create_invoice(
            format!("PERF-TEST-{}", uuid::Uuid::new_v4()),
            "xendit".to_string(),
            Currency::IDR,
            line_items,
            None,
        )
        .await
        .expect("Failed to create invoice");

    let invoice_id = invoice.id.as_ref().unwrap();

    // Measure lookup time
    let start = Instant::now();
    let result = service.get_invoice(invoice_id).await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Invoice lookup failed");
    assert!(
        duration.as_secs_f64() < 0.1, // Lookups should be < 100ms
        "Invoice lookup took {:.3}s (expected: <0.1s)",
        duration.as_secs_f64()
    );

    println!("✅ Invoice lookup completed in {:.3}s", duration.as_secs_f64());

    // Cleanup
    let _ = sqlx::query("DELETE FROM line_items WHERE invoice_id = ?")
        .bind(invoice_id)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM invoices WHERE id = ?")
        .bind(invoice_id)
        .execute(&pool)
        .await;
}
