use super::super::models::{PaymentTransaction, TransactionStatus};
use super::super::repositories::TransactionRepository;
use crate::core::{AppError, Currency, Result};
use crate::modules::invoices::models::InvoiceStatus;
use crate::modules::invoices::repositories::InvoiceRepository;
use rust_decimal::Decimal;

/// Transaction service for business logic
///
/// Orchestrates transaction recording and invoice status updates
pub struct TransactionService {
    transaction_repo: TransactionRepository,
    invoice_repo: InvoiceRepository,
}

impl TransactionService {
    /// Create a new TransactionService
    ///
    /// # Arguments
    /// * `transaction_repo` - Transaction repository
    /// * `invoice_repo` - Invoice repository
    pub fn new(transaction_repo: TransactionRepository, invoice_repo: InvoiceRepository) -> Self {
        Self {
            transaction_repo,
            invoice_repo,
        }
    }

    /// Record a payment transaction (FR-030) with pessimistic locking (FR-053, FR-054)
    ///
    /// Creates a transaction record and updates invoice status if payment is complete.
    /// Uses pessimistic locking to prevent concurrent payment processing.
    ///
    /// # Arguments
    /// * `invoice_id` - Invoice ID
    /// * `gateway_transaction_ref` - Gateway transaction reference (for idempotency - FR-032)
    /// * `gateway_id` - Gateway identifier
    /// * `amount_paid` - Amount paid
    /// * `currency` - Payment currency
    /// * `payment_method` - Payment method used
    /// * `status` - Transaction status
    /// * `gateway_response` - Full gateway response
    ///
    /// # Returns
    /// * `Result<PaymentTransaction>` - Created or existing transaction
    ///
    /// # Errors
    /// * `409 Conflict` - If payment is already in progress for this invoice (FR-054)
    pub async fn record_payment(
        &self,
        invoice_id: String,
        gateway_transaction_ref: String,
        gateway_id: String,
        amount_paid: Decimal,
        currency: Currency,
        payment_method: String,
        status: TransactionStatus,
        gateway_response: Option<serde_json::Value>,
    ) -> Result<PaymentTransaction> {
        // Check for idempotency first (before acquiring lock)
        if let Some(existing) = self
            .transaction_repo
            .find_by_gateway_ref(&gateway_transaction_ref)
            .await?
        {
            tracing::info!(
                gateway_ref = gateway_transaction_ref,
                transaction_id = existing.id,
                "Transaction already exists (idempotent request)"
            );
            return Ok(existing);
        }

        // Start transaction with pessimistic locking (FR-053)
        let pool = self.transaction_repo.pool();
        let mut tx = pool.begin().await
            .map_err(|e| AppError::Internal(format!("Failed to start transaction: {}", e)))?;

        // Acquire lock on invoice (FOR UPDATE)
        let invoice = crate::modules::invoices::repositories::InvoiceRepository::find_by_id_for_update(
            &mut tx,
            &invoice_id,
        )
        .await?
        .ok_or_else(|| AppError::not_found(format!("Invoice '{}' not found", invoice_id)))?;

        // Check if payment is already in progress (FR-054)
        if invoice.status == crate::modules::invoices::models::InvoiceStatus::Processing {
            tracing::warn!(
                invoice_id = invoice_id,
                "Payment already in progress for invoice"
            );
            return Err(AppError::Conflict(
                "Payment already in progress for this invoice".to_string(),
            ));
        }

        // Verify currency matches
        if invoice.currency.to_string() != currency.to_string() {
            return Err(AppError::validation(format!(
                "Payment currency '{}' does not match invoice currency '{}'",
                currency, invoice.currency
            )));
        }

        // Create transaction
        let mut transaction = PaymentTransaction::new(
            invoice_id.clone(),
            gateway_transaction_ref,
            gateway_id,
            amount_paid,
            currency,
            payment_method,
            gateway_response,
        )?;

        // Update transaction status if not pending
        if status != TransactionStatus::Pending {
            transaction.update_status(status)?;
        }

        // Save transaction within transaction
        let saved_transaction = self.transaction_repo.create_with_tx(&transaction, &mut *tx).await?;

        // Update invoice status to Processing (marks payment in progress)
        if saved_transaction.status != TransactionStatus::Failed.to_string() {
            sqlx::query(
                r#"
                UPDATE invoices
                SET status = 'processing', updated_at = NOW()
                WHERE id = ?
                "#
            )
            .bind(&invoice_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update invoice status: {}", e)))?;
        }

        // Commit transaction
        tx.commit().await
            .map_err(|e| AppError::Internal(format!("Failed to commit transaction: {}", e)))?;

        // Update invoice status if payment is completed (outside the lock)
        if saved_transaction.is_completed() {
            self.update_invoice_status_after_payment(&invoice_id).await?;
        }

        Ok(saved_transaction)
    }

    /// Get transaction by ID
    ///
    /// # Arguments
    /// * `id` - Transaction ID
    ///
    /// # Returns
    /// * `Result<PaymentTransaction>` - Transaction
    pub async fn get_transaction(&self, id: &str) -> Result<PaymentTransaction> {
        self.transaction_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Transaction '{}' not found", id)))
    }

    /// Get transaction by gateway reference (for idempotency - FR-032)
    ///
    /// # Arguments
    /// * `gateway_ref` - Gateway transaction reference
    ///
    /// # Returns
    /// * `Result<Option<PaymentTransaction>>` - Transaction if found
    pub async fn get_transaction_by_gateway_ref(
        &self,
        gateway_ref: &str,
    ) -> Result<Option<PaymentTransaction>> {
        self.transaction_repo.find_by_gateway_ref(gateway_ref).await
    }

    /// List all transactions for an invoice
    ///
    /// # Arguments
    /// * `invoice_id` - Invoice ID
    ///
    /// # Returns
    /// * `Result<Vec<PaymentTransaction>>` - List of transactions
    pub async fn list_invoice_transactions(&self, invoice_id: &str) -> Result<Vec<PaymentTransaction>> {
        // Verify invoice exists
        self.invoice_repo
            .find_by_id(invoice_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Invoice '{}' not found", invoice_id)))?;

        self.transaction_repo.find_by_invoice_id(invoice_id).await
    }

    /// Update transaction status
    ///
    /// # Arguments
    /// * `id` - Transaction ID
    /// * `new_status` - New status
    ///
    /// # Returns
    /// * `Result<PaymentTransaction>` - Updated transaction
    pub async fn update_transaction_status(
        &self,
        id: &str,
        new_status: TransactionStatus,
    ) -> Result<PaymentTransaction> {
        // Update transaction status
        self.transaction_repo.update_status(id, new_status).await?;

        // Get updated transaction
        let transaction = self.get_transaction(id).await?;

        // If transaction is completed, update invoice status
        if transaction.is_completed() {
            self.update_invoice_status_after_payment(&transaction.invoice_id)
                .await?;
        }

        Ok(transaction)
    }

    /// Update invoice status after a completed payment
    ///
    /// Checks if invoice is fully paid and updates status accordingly
    ///
    /// # Arguments
    /// * `invoice_id` - Invoice ID
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    async fn update_invoice_status_after_payment(&self, invoice_id: &str) -> Result<()> {
        // Get invoice
        let invoice = self
            .invoice_repo
            .find_by_id(invoice_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Invoice '{}' not found", invoice_id)))?;

        // Calculate total paid
        let total_paid = self.transaction_repo.calculate_total_paid(invoice_id).await?;

        // Get invoice total
        let invoice_total = invoice.total.unwrap_or_default();

        // Update invoice status based on payment amount
        let new_status = if total_paid >= invoice_total {
            InvoiceStatus::Paid
        } else {
            // Partial payment - keep as Processing
            InvoiceStatus::Processing
        };

        // Only update if status has changed
        if invoice.status != new_status {
            self.invoice_repo.update_status(invoice_id, new_status).await?;
        }

        Ok(())
    }

    /// Check if invoice is fully paid
    ///
    /// # Arguments
    /// * `invoice_id` - Invoice ID
    ///
    /// # Returns
    /// * `Result<bool>` - True if fully paid
    pub async fn is_invoice_fully_paid(&self, invoice_id: &str) -> Result<bool> {
        let invoice = self
            .invoice_repo
            .find_by_id(invoice_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Invoice '{}' not found", invoice_id)))?;

        let total_paid = self.transaction_repo.calculate_total_paid(invoice_id).await?;
        let invoice_total = invoice.total.unwrap_or_default();

        Ok(total_paid >= invoice_total)
    }

    /// Get payment statistics for an invoice
    ///
    /// # Arguments
    /// * `invoice_id` - Invoice ID
    ///
    /// # Returns
    /// * `Result<PaymentStats>` - Payment statistics
    pub async fn get_payment_stats(&self, invoice_id: &str) -> Result<PaymentStats> {
        let invoice = self
            .invoice_repo
            .find_by_id(invoice_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Invoice '{}' not found", invoice_id)))?;

        let total_paid = self.transaction_repo.calculate_total_paid(invoice_id).await?;
        let invoice_total = invoice.total.unwrap_or_default();
        let transactions = self.transaction_repo.find_by_invoice_id(invoice_id).await?;

        let completed_count = transactions
            .iter()
            .filter(|t| t.is_completed())
            .count();

        let pending_count = transactions
            .iter()
            .filter(|t| matches!(t.get_status(), Ok(TransactionStatus::Pending)))
            .count();

        let failed_count = transactions
            .iter()
            .filter(|t| t.is_failed())
            .count();

        Ok(PaymentStats {
            invoice_total,
            total_paid,
            balance: invoice_total - total_paid,
            is_fully_paid: total_paid >= invoice_total,
            transaction_count: transactions.len(),
            completed_count,
            pending_count,
            failed_count,
        })
    }
}

/// Payment statistics for an invoice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentStats {
    pub invoice_total: Decimal,
    pub total_paid: Decimal,
    pub balance: Decimal,
    pub is_fully_paid: bool,
    pub transaction_count: usize,
    pub completed_count: usize,
    pub pending_count: usize,
    pub failed_count: usize,
}

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    // Service creation is tested in integration tests
}
