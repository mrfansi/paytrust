// T090: InstallmentRepository implementation
// Provides MySQL CRUD operations for installment schedules
//
// Implements:
// - Create single installment schedule
// - Create batch of installment schedules (transactional)
// - Read installments by invoice ID
// - Read single installment by ID
// - Update installment status and payment details
// - Query unpaid installments in sequence
// - Update installment amounts for adjustment (FR-077)

use sqlx::{MySql, MySqlPool, Transaction};
use uuid::Uuid;

use crate::core::{AppError, Result};
use crate::modules::installments::models::{InstallmentSchedule, InstallmentStatus};

/// Repository for installment schedule database operations
pub struct InstallmentRepository {
    pool: MySqlPool,
}

impl InstallmentRepository {
    /// Create a new installment repository
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Create a batch of installment schedules in a transaction
    /// 
    /// # Arguments
    /// * `installments` - Vector of installment schedules to create
    /// 
    /// # Returns
    /// * `Result<()>` - Success or database error
    /// 
    /// # Database Operations
    /// Inserts all installment records in a single transaction
    pub async fn create_batch(&self, installments: &[InstallmentSchedule]) -> Result<()> {
        if installments.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await
            .map_err(|e| AppError::Internal(format!("Failed to start transaction: {}", e)))?;

        for installment in installments {
            self.insert_with_tx(&mut tx, installment).await?;
        }

        tx.commit().await
            .map_err(|e| AppError::Internal(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Insert a single installment within a transaction
    async fn insert_with_tx(
        &self,
        tx: &mut Transaction<'_, MySql>,
        installment: &InstallmentSchedule,
    ) -> Result<()> {
        let id = &installment.id;

        sqlx::query(
            r#"
            INSERT INTO installment_schedules (
                id, invoice_id, installment_number, amount, tax_amount, 
                service_fee_amount, due_date, status, payment_url, 
                gateway_reference, paid_at, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(id)
        .bind(&installment.invoice_id)
        .bind(installment.installment_number)
        .bind(installment.amount)
        .bind(installment.tax_amount)
        .bind(installment.service_fee_amount)
        .bind(installment.due_date)
        .bind(installment.status.to_string())
        .bind(&installment.payment_url)
        .bind(&installment.gateway_reference)
        .bind(installment.paid_at)
        .bind(installment.created_at)
        .bind(installment.updated_at)
        .execute(tx.as_mut())
        .await
        .map_err(|e| AppError::Internal(format!("Failed to insert installment: {}", e)))?;

        Ok(())
    }

    /// Find all installments for an invoice
    /// 
    /// # Arguments
    /// * `invoice_id` - Invoice ID to query
    /// 
    /// # Returns
    /// * `Result<Vec<InstallmentSchedule>>` - Ordered list of installments (by installment_number)
    pub async fn find_by_invoice(&self, invoice_id: &str) -> Result<Vec<InstallmentSchedule>> {
        let rows = sqlx::query_as::<_, InstallmentScheduleRow>(
            r#"
            SELECT 
                id, invoice_id, installment_number, amount, tax_amount,
                service_fee_amount, due_date, status, payment_url,
                gateway_reference, paid_at, created_at, updated_at
            FROM installment_schedules
            WHERE invoice_id = ?
            ORDER BY installment_number ASC
            "#
        )
        .bind(invoice_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch installments: {}", e)))?;

        rows.into_iter()
            .map(|row| row.try_into())
            .collect()
    }

    /// Find a single installment by ID
    /// 
    /// # Arguments
    /// * `id` - Installment ID
    /// 
    /// # Returns
    /// * `Result<Option<InstallmentSchedule>>` - Installment if found, None otherwise
    pub async fn find_by_id(&self, id: &str) -> Result<Option<InstallmentSchedule>> {
        let row = sqlx::query_as::<_, InstallmentScheduleRow>(
            r#"
            SELECT 
                id, invoice_id, installment_number, amount, tax_amount,
                service_fee_amount, due_date, status, payment_url,
                gateway_reference, paid_at, created_at, updated_at
            FROM installment_schedules
            WHERE id = ?
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch installment: {}", e)))?;

        match row {
            Some(r) => Ok(Some(r.try_into()?)),
            None => Ok(None),
        }
    }

    /// Update installment status and payment details
    /// 
    /// # Arguments
    /// * `installment` - Installment with updated fields
    /// 
    /// # Returns
    /// * `Result<()>` - Success or database error
    pub async fn update(&self, installment: &InstallmentSchedule) -> Result<()> {
        let id = &installment.id;

        let rows_affected = sqlx::query(
            r#"
            UPDATE installment_schedules
            SET 
                amount = ?,
                tax_amount = ?,
                service_fee_amount = ?,
                status = ?,
                payment_url = ?,
                gateway_reference = ?,
                paid_at = ?,
                updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(installment.amount)
        .bind(installment.tax_amount)
        .bind(installment.service_fee_amount)
        .bind(installment.status.to_string())
        .bind(&installment.payment_url)
        .bind(&installment.gateway_reference)
        .bind(installment.paid_at)
        .bind(installment.updated_at)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to update installment: {}", e)))?
        .rows_affected();

        if rows_affected == 0 {
            return Err(AppError::not_found("Installment not found"));
        }

        Ok(())
    }

    /// Find unpaid installments in sequence for an invoice
    /// Used for sequential payment enforcement (FR-068)
    /// 
    /// # Arguments
    /// * `invoice_id` - Invoice ID to query
    /// 
    /// # Returns
    /// * `Result<Vec<InstallmentSchedule>>` - Unpaid installments ordered by number
    pub async fn find_unpaid_in_sequence(&self, invoice_id: &str) -> Result<Vec<InstallmentSchedule>> {
        let rows = sqlx::query_as::<_, InstallmentScheduleRow>(
            r#"
            SELECT 
                id, invoice_id, installment_number, amount, tax_amount,
                service_fee_amount, due_date, status, payment_url,
                gateway_reference, paid_at, created_at, updated_at
            FROM installment_schedules
            WHERE invoice_id = ? AND status = 'unpaid'
            ORDER BY installment_number ASC
            "#
        )
        .bind(invoice_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch unpaid installments: {}", e)))?;

        rows.into_iter()
            .map(|row| row.try_into())
            .collect()
    }

    /// Batch update multiple installments (for adjustment - FR-077)
    /// 
    /// # Arguments
    /// * `installments` - Vector of installments with updated amounts
    /// 
    /// # Returns
    /// * `Result<()>` - Success or database error
    pub async fn update_batch(&self, installments: &[InstallmentSchedule]) -> Result<()> {
        if installments.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await
            .map_err(|e| AppError::Internal(format!("Failed to start transaction: {}", e)))?;

        for installment in installments {
            let id = &installment.id;

            sqlx::query(
                r#"
                UPDATE installment_schedules
                SET 
                    amount = ?,
                    tax_amount = ?,
                    service_fee_amount = ?,
                    updated_at = ?
                WHERE id = ?
                "#
            )
            .bind(installment.amount)
            .bind(installment.tax_amount)
            .bind(installment.service_fee_amount)
            .bind(installment.updated_at)
            .bind(id)
            .execute(tx.as_mut())
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update installment: {}", e)))?;
        }

        tx.commit().await
            .map_err(|e| AppError::Internal(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }
}

/// Database row representation for installment_schedules table
#[derive(sqlx::FromRow)]
struct InstallmentScheduleRow {
    id: String,
    invoice_id: String,
    installment_number: i32,
    amount: rust_decimal::Decimal,
    tax_amount: rust_decimal::Decimal,
    service_fee_amount: rust_decimal::Decimal,
    due_date: chrono::NaiveDate,
    status: String,
    payment_url: Option<String>,
    gateway_reference: Option<String>,
    paid_at: Option<chrono::NaiveDateTime>,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
}

impl TryFrom<InstallmentScheduleRow> for InstallmentSchedule {
    type Error = AppError;

    fn try_from(row: InstallmentScheduleRow) -> Result<Self> {
        let status = match row.status.as_str() {
            "unpaid" => InstallmentStatus::Unpaid,
            "paid" => InstallmentStatus::Paid,
            "overdue" => InstallmentStatus::Overdue,
            _ => return Err(AppError::Internal(format!("Invalid installment status: {}", row.status))),
        };

        Ok(InstallmentSchedule {
            id: row.id,
            invoice_id: row.invoice_id,
            installment_number: row.installment_number,
            amount: row.amount,
            tax_amount: row.tax_amount,
            service_fee_amount: row.service_fee_amount,
            due_date: row.due_date,
            status,
            payment_url: row.payment_url,
            gateway_reference: row.gateway_reference,
            paid_at: row.paid_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use chrono::NaiveDate;

    #[test]
    fn test_installment_row_conversion() {
        let row = InstallmentScheduleRow {
            id: "inst-001".to_string(),
            invoice_id: "inv-001".to_string(),
            installment_number: 1,
            amount: Decimal::new(500000, 0),
            tax_amount: Decimal::new(55000, 0),
            service_fee_amount: Decimal::new(10000, 0),
            due_date: NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
            status: "unpaid".to_string(),
            payment_url: Some("https://pay.example.com/123".to_string()),
            gateway_reference: None,
            paid_at: None,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };

        let installment: InstallmentSchedule = row.try_into().unwrap();
        assert_eq!(installment.id, "inst-001");
        assert_eq!(installment.invoice_id, "inv-001");
        assert_eq!(installment.installment_number, 1);
        assert_eq!(installment.amount, Decimal::new(500000, 0));
        assert_eq!(installment.status, InstallmentStatus::Unpaid);
    }

    #[test]
    fn test_invalid_status_conversion() {
        let row = InstallmentScheduleRow {
            id: "inst-001".to_string(),
            invoice_id: "inv-001".to_string(),
            installment_number: 1,
            amount: Decimal::new(500000, 0),
            tax_amount: Decimal::new(55000, 0),
            service_fee_amount: Decimal::new(10000, 0),
            due_date: NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
            status: "invalid_status".to_string(),
            payment_url: None,
            gateway_reference: None,
            paid_at: None,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };

        let result: Result<InstallmentSchedule> = row.try_into();
        assert!(result.is_err());
    }
}
