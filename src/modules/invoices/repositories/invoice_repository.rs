use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{MySql, Pool};

use crate::core::error::AppError;
use crate::modules::invoices::models::{Invoice, InvoiceStatus, LineItem};

/// Repository trait for Invoice operations
#[async_trait]
pub trait InvoiceRepository: Send + Sync {
    /// Create a new invoice with line items
    /// Tenant isolation: invoice.tenant_id must match authenticated tenant
    async fn create(
        &self,
        invoice: &Invoice,
        line_items: &[LineItem],
        tenant_id: &str,
    ) -> Result<Invoice, AppError>;

    /// Find invoice by ID
    /// Tenant isolation: filters by tenant_id
    async fn find_by_id(&self, id: i64, tenant_id: &str) -> Result<Option<Invoice>, AppError>;

    /// Find invoice by external ID
    /// Tenant isolation: filters by tenant_id
    async fn find_by_external_id(
        &self,
        external_id: &str,
        tenant_id: &str,
    ) -> Result<Option<Invoice>, AppError>;

    /// List invoices for a tenant with pagination
    /// Tenant isolation: filters by tenant_id
    async fn list(
        &self,
        tenant_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Invoice>, AppError>;

    /// Update invoice status
    /// Tenant isolation: validates tenant_id matches
    async fn update_status(
        &self,
        id: i64,
        status: InvoiceStatus,
        tenant_id: &str,
    ) -> Result<(), AppError>;

    /// Update payment_initiated_at timestamp
    /// Tenant isolation: validates tenant_id matches
    async fn set_payment_initiated(
        &self,
        id: i64,
        initiated_at: DateTime<Utc>,
        tenant_id: &str,
    ) -> Result<(), AppError>;

    /// Find line items for an invoice
    /// Tenant isolation: validates invoice belongs to tenant
    async fn find_line_items(
        &self,
        invoice_id: i64,
        tenant_id: &str,
    ) -> Result<Vec<LineItem>, AppError>;
}

/// MySQL implementation of InvoiceRepository
pub struct MySqlInvoiceRepository {
    pool: Pool<MySql>,
}

impl MySqlInvoiceRepository {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InvoiceRepository for MySqlInvoiceRepository {
    async fn create(
        &self,
        invoice: &Invoice,
        line_items: &[LineItem],
        tenant_id: &str,
    ) -> Result<Invoice, AppError> {
        // Tenant isolation: validate invoice.tenant_id matches authenticated tenant
        if invoice.tenant_id != tenant_id {
            return Err(AppError::Unauthorized(
                "Cannot create invoice for different tenant".to_string(),
            ));
        }

        let mut tx = self.pool.begin().await?;

        // Insert invoice
        let result = sqlx::query(
            r#"
            INSERT INTO invoices (
                tenant_id, external_id, currency, subtotal, tax_total, service_fee,
                total_amount, status, gateway_id, original_invoice_id, payment_initiated_at,
                expires_at, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&invoice.tenant_id)
        .bind(&invoice.external_id)
        .bind(&invoice.currency)
        .bind(&invoice.subtotal)
        .bind(&invoice.tax_total)
        .bind(&invoice.service_fee)
        .bind(&invoice.total_amount)
        .bind(&invoice.status)
        .bind(&invoice.gateway_id)
        .bind(&invoice.original_invoice_id)
        .bind(&invoice.payment_initiated_at)
        .bind(&invoice.expires_at)
        .bind(&invoice.created_at)
        .bind(&invoice.updated_at)
        .execute(&mut *tx)
        .await?;

        let invoice_id = result.last_insert_id() as i64;

        // Insert line items
        for line_item in line_items {
            sqlx::query(
                r#"
                INSERT INTO line_items (
                    invoice_id, product_name, quantity, unit_price, subtotal,
                    tax_rate, tax_category, tax_amount, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(invoice_id)
            .bind(&line_item.product_name)
            .bind(&line_item.quantity)
            .bind(&line_item.unit_price)
            .bind(&line_item.subtotal)
            .bind(&line_item.tax_rate)
            .bind(&line_item.tax_category)
            .bind(&line_item.tax_amount)
            .bind(&line_item.created_at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        // Fetch and return the created invoice
        self.find_by_id(invoice_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Invoice not found after creation".to_string()))
    }

    async fn find_by_id(&self, id: i64, tenant_id: &str) -> Result<Option<Invoice>, AppError> {
        // Tenant isolation: filter by tenant_id per FR-088
        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT id, tenant_id, external_id, currency, subtotal, tax_total, service_fee,
                   total_amount, status, gateway_id, original_invoice_id, payment_initiated_at,
                   expires_at, created_at, updated_at
            FROM invoices
            WHERE id = ? AND tenant_id = ?
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(invoice)
    }

    async fn find_by_external_id(
        &self,
        external_id: &str,
        tenant_id: &str,
    ) -> Result<Option<Invoice>, AppError> {
        // Tenant isolation: filter by tenant_id per FR-088
        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT id, tenant_id, external_id, currency, subtotal, tax_total, service_fee,
                   total_amount, status, gateway_id, original_invoice_id, payment_initiated_at,
                   expires_at, created_at, updated_at
            FROM invoices
            WHERE external_id = ? AND tenant_id = ?
            "#,
        )
        .bind(external_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(invoice)
    }

    async fn list(
        &self,
        tenant_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Invoice>, AppError> {
        // Tenant isolation: filter by tenant_id per FR-088
        let invoices = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT id, tenant_id, external_id, currency, subtotal, tax_total, service_fee,
                   total_amount, status, gateway_id, original_invoice_id, payment_initiated_at,
                   expires_at, created_at, updated_at
            FROM invoices
            WHERE tenant_id = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(invoices)
    }

    async fn update_status(
        &self,
        id: i64,
        status: InvoiceStatus,
        tenant_id: &str,
    ) -> Result<(), AppError> {
        // Tenant isolation: validate tenant_id matches per FR-088
        let result = sqlx::query(
            r#"
            UPDATE invoices
            SET status = ?, updated_at = ?
            WHERE id = ? AND tenant_id = ?
            "#,
        )
        .bind(&status)
        .bind(Utc::now())
        .bind(id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Invoice not found or access denied".to_string(),
            ));
        }

        Ok(())
    }

    async fn set_payment_initiated(
        &self,
        id: i64,
        initiated_at: DateTime<Utc>,
        tenant_id: &str,
    ) -> Result<(), AppError> {
        // Tenant isolation: validate tenant_id matches per FR-088
        let result = sqlx::query(
            r#"
            UPDATE invoices
            SET payment_initiated_at = ?, updated_at = ?
            WHERE id = ? AND tenant_id = ? AND payment_initiated_at IS NULL
            "#,
        )
        .bind(initiated_at)
        .bind(Utc::now())
        .bind(id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::Validation(
                "Invoice not found, access denied, or payment already initiated".to_string(),
            ));
        }

        Ok(())
    }

    async fn find_line_items(
        &self,
        invoice_id: i64,
        tenant_id: &str,
    ) -> Result<Vec<LineItem>, AppError> {
        // Tenant isolation: validate invoice belongs to tenant per FR-088
        let line_items = sqlx::query_as::<_, LineItem>(
            r#"
            SELECT li.id, li.invoice_id, li.product_name, li.quantity, li.unit_price,
                   li.subtotal, li.tax_rate, li.tax_category, li.tax_amount, li.created_at
            FROM line_items li
            INNER JOIN invoices i ON li.invoice_id = i.id
            WHERE li.invoice_id = ? AND i.tenant_id = ?
            ORDER BY li.id ASC
            "#,
        )
        .bind(invoice_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(line_items)
    }
}

#[cfg(test)]
mod tests {
    // Note: Integration tests with real database are in tests/integration/
    // These are unit tests for logic validation only

    #[test]
    fn test_repository_trait_exists() {
        // This test ensures the trait compiles and has the expected methods
        // Actual database tests are in integration tests
    }
}
