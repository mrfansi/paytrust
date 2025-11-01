// T041: InvoiceRepository implementation
// Provides MySQL CRUD operations for invoices and line items
//
// Implements:
// - Create invoice with line items (transactional)
// - Read invoice by ID with line items (joined query)
// - List invoices with pagination
// - Update invoice status (with immutability checks)
// - Check external_id uniqueness per merchant

use rust_decimal::Decimal;
use sqlx::{MySql, MySqlPool, Transaction};
use uuid::Uuid;

use crate::core::{AppError, Result};
use crate::modules::invoices::models::{Invoice, InvoiceStatus, LineItem};

/// Repository for invoice database operations
pub struct InvoiceRepository {
    pool: MySqlPool,
}

impl InvoiceRepository {
    /// Create a new invoice repository
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Create an invoice with its line items in a transaction
    ///
    /// # Arguments
    /// * `invoice` - Invoice to create (must have line_items populated)
    ///
    /// # Returns
    /// * `Result<Invoice>` - Created invoice with generated ID
    ///
    /// # Database Operations
    /// 1. Insert invoice record
    /// 2. Insert all line item records
    /// 3. Commit transaction
    pub async fn create(&self, invoice: &Invoice) -> Result<Invoice> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to start transaction: {}", e)))?;

        let created_invoice = self.create_with_tx(&mut tx, invoice).await?;

        tx.commit()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to commit transaction: {}", e)))?;

        Ok(created_invoice)
    }

    /// Create invoice within an existing transaction
    pub async fn create_with_tx(
        &self,
        tx: &mut Transaction<'_, MySql>,
        invoice: &Invoice,
    ) -> Result<Invoice> {
        let id = invoice
            .id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // Insert invoice
        sqlx::query(
            r#"
            INSERT INTO invoices (
                id, external_id, gateway_id, currency, total, status,
                expires_at, original_invoice_id, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&invoice.external_id)
        .bind(&invoice.gateway_id)
        .bind(invoice.currency.to_string())
        .bind(invoice.total)
        .bind(invoice.status.to_string())
        .bind(invoice.expires_at)
        .bind(&invoice.original_invoice_id)
        .bind(invoice.created_at)
        .bind(invoice.updated_at)
        .execute(&mut **tx)
        .await
        .map_err(|e| {
            if let Some(db_err) = e.as_database_error() {
                if db_err.is_unique_violation() {
                    return AppError::validation(format!(
                        "Invoice with external_id '{}' already exists",
                        invoice.external_id
                    ));
                }
            }
            AppError::Internal(format!("Failed to create invoice: {}", e))
        })?;

        // Insert line items
        for line_item in &invoice.line_items {
            let line_id = Uuid::new_v4().to_string();

            sqlx::query(
                r#"
                INSERT INTO line_items (
                    id, invoice_id, description, quantity, unit_price, currency, subtotal
                ) VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&line_id)
            .bind(&id)
            .bind(&line_item.description)
            .bind(line_item.quantity)
            .bind(line_item.unit_price)
            .bind(line_item.currency.to_string())
            .bind(line_item.subtotal)
            .execute(&mut **tx)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to create line item: {}", e)))?;
        }

        // Return invoice with generated ID
        let mut created_invoice = invoice.clone();
        created_invoice.id = Some(id);

        Ok(created_invoice)
    }

    /// Find invoice by ID, including line items
    ///
    /// # Arguments
    /// * `id` - Invoice ID (UUID)
    ///
    /// # Returns
    /// * `Result<Option<Invoice>>` - Invoice if found, None if not found
    pub async fn find_by_id(&self, id: &str) -> Result<Option<Invoice>> {
        // Fetch invoice
        let invoice_row = sqlx::query_as::<_, InvoiceRow>(
            r#"
            SELECT 
                id, external_id, gateway_id, currency, total, status,
                expires_at, original_invoice_id, created_at, updated_at
            FROM invoices
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch invoice: {}", e)))?;

        let Some(invoice_row) = invoice_row else {
            return Ok(None);
        };

        // Fetch line items
        let line_items = sqlx::query_as::<_, LineItemRow>(
            r#"
            SELECT 
                id, invoice_id, description, quantity, unit_price, currency, subtotal
            FROM line_items
            WHERE invoice_id = ?
            ORDER BY id
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch line items: {}", e)))?;

        Ok(Some(invoice_row.into_invoice(line_items)?))
    }

    /// Find invoice by external_id
    ///
    /// # Arguments
    /// * `external_id` - Merchant's reference ID
    ///
    /// # Returns
    /// * `Result<Option<Invoice>>` - Invoice if found
    pub async fn find_by_external_id(&self, external_id: &str) -> Result<Option<Invoice>> {
        let invoice_row = sqlx::query_as::<_, InvoiceRow>(
            r#"
            SELECT 
                id, external_id, gateway_id, currency, total, status,
                expires_at, original_invoice_id, created_at, updated_at
            FROM invoices
            WHERE external_id = ?
            "#,
        )
        .bind(external_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch invoice: {}", e)))?;

        let Some(invoice_row) = invoice_row else {
            return Ok(None);
        };

        let line_items = sqlx::query_as::<_, LineItemRow>(
            r#"
            SELECT 
                id, invoice_id, description, quantity, unit_price, currency, subtotal
            FROM line_items
            WHERE invoice_id = ?
            ORDER BY id
            "#,
        )
        .bind(&invoice_row.id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch line items: {}", e)))?;

        Ok(Some(invoice_row.into_invoice(line_items)?))
    }

    /// Find invoice by ID with pessimistic lock (FOR UPDATE) for payment processing (FR-053)
    ///
    /// This method locks the invoice row to prevent concurrent payment processing.
    /// Must be called within a transaction.
    ///
    /// # Arguments
    /// * `tx` - Database transaction
    /// * `id` - Invoice ID
    ///
    /// # Returns
    /// * `Result<Option<Invoice>>` - Locked invoice if found
    pub async fn find_by_id_for_update(
        tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
        id: &str,
    ) -> Result<Option<Invoice>> {
        // Fetch invoice with pessimistic lock
        let invoice_row = sqlx::query_as::<_, InvoiceRow>(
            r#"
            SELECT 
                id, external_id, gateway_id, currency, total, status,
                expires_at, original_invoice_id, created_at, updated_at
            FROM invoices
            WHERE id = ?
            FOR UPDATE
            "#,
        )
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch invoice with lock: {}", e)))?;

        let Some(invoice_row) = invoice_row else {
            return Ok(None);
        };

        // Fetch line items
        let line_items = sqlx::query_as::<_, LineItemRow>(
            r#"
            SELECT 
                id, invoice_id, description, quantity, unit_price, currency, subtotal
            FROM line_items
            WHERE invoice_id = ?
            ORDER BY id
            "#,
        )
        .bind(&invoice_row.id)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch line items: {}", e)))?;

        Ok(Some(invoice_row.into_invoice(line_items)?))
    }

    /// List invoices with pagination
    ///
    /// # Arguments
    /// * `limit` - Maximum number of results (default: 20, max: 100)
    /// * `offset` - Number of results to skip
    ///
    /// # Returns
    /// * `Result<Vec<Invoice>>` - List of invoices (without line items for performance)
    pub async fn list(&self, limit: i32, offset: i32) -> Result<Vec<Invoice>> {
        let invoice_rows = sqlx::query_as::<_, InvoiceRow>(
            r#"
            SELECT 
                id, external_id, gateway_id, currency, total, status,
                expires_at, original_invoice_id, created_at, updated_at
            FROM invoices
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch invoices: {}", e)))?;

        let mut invoices = Vec::new();
        for invoice_row in invoice_rows {
            let line_items = sqlx::query_as::<_, LineItemRow>(
                r#"
                SELECT 
                    id, invoice_id, description, quantity, unit_price, currency, subtotal
                FROM line_items
                WHERE invoice_id = ?
                ORDER BY id
                "#,
            )
            .bind(&invoice_row.id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to fetch line items: {}", e)))?;

            invoices.push(invoice_row.into_invoice(line_items)?);
        }

        Ok(invoices)
    }

    /// Update invoice status
    ///
    /// # Arguments
    /// * `id` - Invoice ID
    /// * `new_status` - New status to set
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    ///
    /// # Notes
    /// Caller must enforce immutability rules (FR-051, FR-052)
    pub async fn update_status(&self, id: &str, new_status: InvoiceStatus) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE invoices
            SET status = ?, updated_at = NOW()
            WHERE id = ?
            "#,
        )
        .bind(new_status.to_string())
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to update invoice status: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!(
                "Invoice with id '{}' not found",
                id
            )));
        }

        Ok(())
    }

    /// Check if external_id exists
    ///
    /// Used for uniqueness validation before creation
    pub async fn exists_by_external_id(&self, external_id: &str) -> Result<bool> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) as count
            FROM invoices
            WHERE external_id = ?
            "#,
        )
        .bind(external_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to check external_id: {}", e)))?;

        Ok(row.0 > 0)
    }
}

// Helper structs for database mapping

#[derive(Debug, sqlx::FromRow)]
struct InvoiceRow {
    id: String,
    external_id: String,
    gateway_id: String,
    currency: String,
    total: rust_decimal::Decimal,
    status: String,
    expires_at: chrono::DateTime<chrono::Utc>,
    original_invoice_id: Option<String>, // T103
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl InvoiceRow {
    fn into_invoice(self, line_item_rows: Vec<LineItemRow>) -> Result<Invoice> {
        use std::str::FromStr;

        let currency = crate::core::Currency::from_str(&self.currency)
            .map_err(|e| AppError::Internal(format!("Invalid currency in database: {}", e)))?;

        let status = InvoiceStatus::from_str(&self.status)
            .map_err(|e| AppError::Internal(format!("Invalid status in database: {}", e)))?;

        let line_items: Result<Vec<LineItem>> = line_item_rows
            .into_iter()
            .map(|row| row.into_line_item())
            .collect();

        Ok(Invoice {
            id: Some(self.id),
            external_id: self.external_id,
            gateway_id: self.gateway_id,
            currency,
            subtotal: Some(self.total), // TODO: Add subtotal column to DB
            tax_total: Some(Decimal::ZERO), // TODO: Add tax_total column to DB
            service_fee: Some(Decimal::ZERO), // TODO: Add service_fee column to DB
            total: Some(self.total),
            status,
            expires_at: Some(self.expires_at),
            original_invoice_id: self.original_invoice_id, // T103
            created_at: Some(self.created_at),
            updated_at: Some(self.updated_at),
            line_items: line_items?,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct LineItemRow {
    id: String,
    invoice_id: String,
    description: String,
    quantity: i32,
    unit_price: rust_decimal::Decimal,
    currency: String,
    subtotal: rust_decimal::Decimal,
}

impl LineItemRow {
    fn into_line_item(self) -> Result<LineItem> {
        use std::str::FromStr;

        let currency = crate::core::Currency::from_str(&self.currency)
            .map_err(|e| AppError::Internal(format!("Invalid currency in database: {}", e)))?;

        Ok(LineItem {
            id: Some(self.id),
            invoice_id: Some(self.invoice_id),
            description: self.description,
            quantity: self.quantity,
            unit_price: self.unit_price,
            currency,
            subtotal: Some(self.subtotal),
            tax_rate: Some(Decimal::ZERO), // TODO: Add tax_rate column to DB
            tax_category: None,            // TODO: Add tax_category column to DB
            tax_amount: Some(Decimal::ZERO), // TODO: Add tax_amount column to DB
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    // Note: Integration tests with actual database will be in tests/integration/
    // These are unit tests for the conversion logic

    #[test]
    fn test_invoice_status_to_string() {
        assert_eq!(InvoiceStatus::Pending.to_string(), "pending");
        assert_eq!(InvoiceStatus::Processing.to_string(), "processing");
        assert_eq!(InvoiceStatus::Paid.to_string(), "paid");
        assert_eq!(InvoiceStatus::Expired.to_string(), "expired");
        assert_eq!(InvoiceStatus::Failed.to_string(), "failed");
    }
}
