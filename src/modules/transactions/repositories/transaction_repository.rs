use super::super::models::{PaymentTransaction, TransactionStatus};
use crate::core::{AppError, Result};
use sqlx::{MySql, MySqlPool, Transaction};

/// Repository for payment transaction persistence
///
/// Provides CRUD operations with idempotency support via gateway_transaction_ref
pub struct TransactionRepository {
    pool: MySqlPool,
}

impl TransactionRepository {
    /// Create a new TransactionRepository
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Get the database connection pool
    ///
    /// # Returns
    /// * `&MySqlPool` - Database connection pool
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    /// Create a new transaction with idempotency check (FR-032)
    ///
    /// # Arguments
    /// * `transaction` - Transaction to create
    ///
    /// # Returns
    /// * `Result<PaymentTransaction>` - Created transaction or existing if duplicate gateway_transaction_ref
    ///
    /// # Notes
    /// Idempotent via UNIQUE constraint on gateway_transaction_ref
    pub async fn create(&self, transaction: &PaymentTransaction) -> Result<PaymentTransaction> {
        self.create_with_tx(transaction, &self.pool).await
    }

    /// Create transaction within an existing database transaction
    ///
    /// # Arguments
    /// * `transaction` - Transaction to create
    /// * `executor` - Database connection or transaction
    pub async fn create_with_tx<'a, E>(
        &self,
        transaction: &PaymentTransaction,
        executor: E,
    ) -> Result<PaymentTransaction>
    where
        E: sqlx::Executor<'a, Database = MySql>,
    {
        // Check if transaction already exists (idempotency - FR-032)
        if let Some(existing) = self
            .find_by_gateway_ref(&transaction.gateway_transaction_ref)
            .await?
        {
            return Ok(existing);
        }

        // Insert new transaction
        let id = transaction.id.as_ref().ok_or_else(|| {
            AppError::Internal("Transaction ID is required for creation".to_string())
        })?;

        sqlx::query(
            r#"
            INSERT INTO payment_transactions (
                id, invoice_id, installment_id, gateway_transaction_ref,
                gateway_id, amount_paid, currency, payment_method,
                status, gateway_response
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(&transaction.invoice_id)
        .bind(&transaction.installment_id)
        .bind(&transaction.gateway_transaction_ref)
        .bind(&transaction.gateway_id)
        .bind(&transaction.amount_paid)
        .bind(&transaction.currency)
        .bind(&transaction.payment_method)
        .bind(&transaction.status)
        .bind(&transaction.gateway_response)
        .execute(executor)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to create transaction: {}", e)))?;

        // Fetch and return created transaction
        self.find_by_id(id).await?.ok_or_else(|| {
            AppError::Internal("Transaction was created but not found".to_string())
        })
    }

    /// Find transaction by ID
    ///
    /// # Arguments
    /// * `id` - Transaction ID
    ///
    /// # Returns
    /// * `Result<Option<PaymentTransaction>>` - Transaction if found
    pub async fn find_by_id(&self, id: &str) -> Result<Option<PaymentTransaction>> {
        let transaction = sqlx::query_as::<_, PaymentTransaction>(
            r#"
            SELECT 
                id, invoice_id, installment_id, gateway_transaction_ref,
                gateway_id, amount_paid, currency, payment_method,
                status, gateway_response, created_at, updated_at
            FROM payment_transactions
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch transaction: {}", e)))?;

        Ok(transaction)
    }

    /// Find transaction by gateway reference (for idempotency - FR-032)
    ///
    /// # Arguments
    /// * `gateway_ref` - Gateway transaction reference
    ///
    /// # Returns
    /// * `Result<Option<PaymentTransaction>>` - Transaction if found
    pub async fn find_by_gateway_ref(&self, gateway_ref: &str) -> Result<Option<PaymentTransaction>> {
        let transaction = sqlx::query_as::<_, PaymentTransaction>(
            r#"
            SELECT 
                id, invoice_id, installment_id, gateway_transaction_ref,
                gateway_id, amount_paid, currency, payment_method,
                status, gateway_response, created_at, updated_at
            FROM payment_transactions
            WHERE gateway_transaction_ref = ?
            "#,
        )
        .bind(gateway_ref)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch transaction by gateway ref: {}", e)))?;

        Ok(transaction)
    }

    /// Find all transactions for an invoice
    ///
    /// # Arguments
    /// * `invoice_id` - Invoice ID
    ///
    /// # Returns
    /// * `Result<Vec<PaymentTransaction>>` - List of transactions
    pub async fn find_by_invoice_id(&self, invoice_id: &str) -> Result<Vec<PaymentTransaction>> {
        let transactions = sqlx::query_as::<_, PaymentTransaction>(
            r#"
            SELECT 
                id, invoice_id, installment_id, gateway_transaction_ref,
                gateway_id, amount_paid, currency, payment_method,
                status, gateway_response, created_at, updated_at
            FROM payment_transactions
            WHERE invoice_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(invoice_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch transactions for invoice: {}", e)))?;

        Ok(transactions)
    }

    /// Find all transactions for an installment
    ///
    /// # Arguments
    /// * `installment_id` - Installment ID
    ///
    /// # Returns
    /// * `Result<Vec<PaymentTransaction>>` - List of transactions
    pub async fn find_by_installment_id(&self, installment_id: &str) -> Result<Vec<PaymentTransaction>> {
        let transactions = sqlx::query_as::<_, PaymentTransaction>(
            r#"
            SELECT 
                id, invoice_id, installment_id, gateway_transaction_ref,
                gateway_id, amount_paid, currency, payment_method,
                status, gateway_response, created_at, updated_at
            FROM payment_transactions
            WHERE installment_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(installment_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch transactions for installment: {}", e)))?;

        Ok(transactions)
    }

    /// Update transaction status
    ///
    /// # Arguments
    /// * `id` - Transaction ID
    /// * `new_status` - New status
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    pub async fn update_status(&self, id: &str, new_status: TransactionStatus) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE payment_transactions
            SET status = ?, updated_at = NOW()
            WHERE id = ?
            "#,
        )
        .bind(new_status.to_string())
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to update transaction status: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!(
                "Transaction with id '{}' not found",
                id
            )));
        }

        Ok(())
    }

    /// Calculate total amount paid for an invoice
    ///
    /// # Arguments
    /// * `invoice_id` - Invoice ID
    ///
    /// # Returns
    /// * `Result<rust_decimal::Decimal>` - Total amount paid (only completed transactions)
    pub async fn calculate_total_paid(&self, invoice_id: &str) -> Result<rust_decimal::Decimal> {
        let row: (Option<rust_decimal::Decimal>,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(amount_paid), 0) as total
            FROM payment_transactions
            WHERE invoice_id = ? AND status = 'completed'
            "#,
        )
        .bind(invoice_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to calculate total paid: {}", e)))?;

        Ok(row.0.unwrap_or_default())
    }

    /// Check if a gateway transaction reference exists (for idempotency)
    ///
    /// # Arguments
    /// * `gateway_ref` - Gateway transaction reference
    ///
    /// # Returns
    /// * `Result<bool>` - True if exists
    pub async fn exists_by_gateway_ref(&self, gateway_ref: &str) -> Result<bool> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) as count
            FROM payment_transactions
            WHERE gateway_transaction_ref = ?
            "#,
        )
        .bind(gateway_ref)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to check gateway ref existence: {}", e)))?;

        Ok(row.0 > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Currency;
    use rust_decimal::Decimal;

    // Note: These are unit tests for the repository interface.
    // Integration tests with actual database will be in tests/integration/

    // Repository creation is tested in integration tests
}
