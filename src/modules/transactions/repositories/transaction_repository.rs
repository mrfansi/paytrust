use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{MySql, Pool};

use crate::core::error::AppError;
use crate::modules::transactions::models::{PaymentTransaction, TransactionStatus};

/// Repository trait for PaymentTransaction operations
#[async_trait]
pub trait TransactionRepository: Send + Sync {
    /// Create a new payment transaction
    /// Implements idempotency check per FR-032
    async fn create(
        &self,
        transaction: &PaymentTransaction,
    ) -> Result<PaymentTransaction, AppError>;

    /// Find transaction by ID
    async fn find_by_id(&self, id: i64) -> Result<Option<PaymentTransaction>, AppError>;

    /// Find transaction by idempotency key (for idempotency check per FR-032)
    async fn find_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<PaymentTransaction>, AppError>;

    /// Find transaction by gateway reference
    async fn find_by_gateway_reference(
        &self,
        gateway_reference: &str,
    ) -> Result<Option<PaymentTransaction>, AppError>;

    /// List transactions for an invoice
    async fn list_by_invoice_id(
        &self,
        invoice_id: i64,
    ) -> Result<Vec<PaymentTransaction>, AppError>;

    /// Update transaction status
    async fn update_status(
        &self,
        id: i64,
        status: TransactionStatus,
        error_message: Option<String>,
    ) -> Result<(), AppError>;

    /// Update transaction with gateway response
    async fn update_gateway_response(
        &self,
        id: i64,
        gateway_response: String,
    ) -> Result<(), AppError>;

    /// Record refund information
    async fn record_refund(
        &self,
        id: i64,
        refund_id: String,
        refund_amount: Decimal,
        refund_reason: Option<String>,
    ) -> Result<(), AppError>;
}

/// MySQL implementation of TransactionRepository
pub struct MySqlTransactionRepository {
    pool: Pool<MySql>,
}

impl MySqlTransactionRepository {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TransactionRepository for MySqlTransactionRepository {
    async fn create(
        &self,
        transaction: &PaymentTransaction,
    ) -> Result<PaymentTransaction, AppError> {
        // FR-032: Check for existing transaction with same idempotency key
        if let Some(existing) = self
            .find_by_idempotency_key(&transaction.idempotency_key)
            .await?
        {
            // Return existing transaction (idempotent behavior)
            return Ok(existing);
        }

        // Insert new transaction
        let result = sqlx::query(
            r#"
            INSERT INTO payment_transactions (
                invoice_id, gateway_reference, idempotency_key, amount, currency,
                status, payment_method, gateway_response, error_message,
                refund_id, refund_amount, refund_timestamp, refund_reason,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(transaction.invoice_id)
        .bind(&transaction.gateway_reference)
        .bind(&transaction.idempotency_key)
        .bind(&transaction.amount)
        .bind(&transaction.currency)
        .bind(&transaction.status)
        .bind(&transaction.payment_method)
        .bind(&transaction.gateway_response)
        .bind(&transaction.error_message)
        .bind(&transaction.refund_id)
        .bind(&transaction.refund_amount)
        .bind(&transaction.refund_timestamp)
        .bind(&transaction.refund_reason)
        .bind(&transaction.created_at)
        .bind(&transaction.updated_at)
        .execute(&self.pool)
        .await?;

        let transaction_id = result.last_insert_id() as i64;

        // Fetch and return the created transaction
        self.find_by_id(transaction_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Transaction not found after creation".to_string()))
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<PaymentTransaction>, AppError> {
        let transaction = sqlx::query_as::<_, PaymentTransaction>(
            r#"
            SELECT id, invoice_id, gateway_reference, idempotency_key, amount, currency,
                   status, payment_method, gateway_response, error_message,
                   refund_id, refund_amount, refund_timestamp, refund_reason,
                   created_at, updated_at
            FROM payment_transactions
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(transaction)
    }

    async fn find_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<PaymentTransaction>, AppError> {
        let transaction = sqlx::query_as::<_, PaymentTransaction>(
            r#"
            SELECT id, invoice_id, gateway_reference, idempotency_key, amount, currency,
                   status, payment_method, gateway_response, error_message,
                   refund_id, refund_amount, refund_timestamp, refund_reason,
                   created_at, updated_at
            FROM payment_transactions
            WHERE idempotency_key = ?
            "#,
        )
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(transaction)
    }

    async fn find_by_gateway_reference(
        &self,
        gateway_reference: &str,
    ) -> Result<Option<PaymentTransaction>, AppError> {
        let transaction = sqlx::query_as::<_, PaymentTransaction>(
            r#"
            SELECT id, invoice_id, gateway_reference, idempotency_key, amount, currency,
                   status, payment_method, gateway_response, error_message,
                   refund_id, refund_amount, refund_timestamp, refund_reason,
                   created_at, updated_at
            FROM payment_transactions
            WHERE gateway_reference = ?
            "#,
        )
        .bind(gateway_reference)
        .fetch_optional(&self.pool)
        .await?;

        Ok(transaction)
    }

    async fn list_by_invoice_id(
        &self,
        invoice_id: i64,
    ) -> Result<Vec<PaymentTransaction>, AppError> {
        let transactions = sqlx::query_as::<_, PaymentTransaction>(
            r#"
            SELECT id, invoice_id, gateway_reference, idempotency_key, amount, currency,
                   status, payment_method, gateway_response, error_message,
                   refund_id, refund_amount, refund_timestamp, refund_reason,
                   created_at, updated_at
            FROM payment_transactions
            WHERE invoice_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(invoice_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(transactions)
    }

    async fn update_status(
        &self,
        id: i64,
        status: TransactionStatus,
        error_message: Option<String>,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            UPDATE payment_transactions
            SET status = ?, error_message = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&status)
        .bind(&error_message)
        .bind(Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Transaction not found".to_string()));
        }

        Ok(())
    }

    async fn update_gateway_response(
        &self,
        id: i64,
        gateway_response: String,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            UPDATE payment_transactions
            SET gateway_response = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&gateway_response)
        .bind(Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Transaction not found".to_string()));
        }

        Ok(())
    }

    async fn record_refund(
        &self,
        id: i64,
        refund_id: String,
        refund_amount: Decimal,
        refund_reason: Option<String>,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            UPDATE payment_transactions
            SET refund_id = ?, refund_amount = ?, refund_timestamp = ?,
                refund_reason = ?, status = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&refund_id)
        .bind(&refund_amount)
        .bind(Utc::now())
        .bind(&refund_reason)
        .bind(TransactionStatus::Refunded)
        .bind(Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Transaction not found".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_trait_exists() {
        // This test ensures the trait compiles and has the expected methods
        // Actual database tests are in integration tests
    }
}
