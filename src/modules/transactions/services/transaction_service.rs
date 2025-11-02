use std::sync::Arc;

use rust_decimal::Decimal;

use crate::core::error::AppError;
use crate::modules::invoices::models::InvoiceStatus;
use crate::modules::invoices::repositories::invoice_repository::InvoiceRepository;
use crate::modules::transactions::models::{
    PaymentTransaction, TransactionResponse, TransactionStatus,
};
use crate::modules::transactions::repositories::transaction_repository::TransactionRepository;

/// Service for transaction business logic
pub struct TransactionService {
    transaction_repo: Arc<dyn TransactionRepository>,
    invoice_repo: Arc<dyn InvoiceRepository>,
}

impl TransactionService {
    pub fn new(
        transaction_repo: Arc<dyn TransactionRepository>,
        invoice_repo: Arc<dyn InvoiceRepository>,
    ) -> Self {
        Self {
            transaction_repo,
            invoice_repo,
        }
    }

    /// Record a payment transaction and update invoice status
    /// Implements FR-030 (transaction recording)
    pub async fn record_payment(
        &self,
        transaction: PaymentTransaction,
        tenant_id: &str,
    ) -> Result<TransactionResponse, AppError> {
        // Create transaction (with idempotency check)
        let created_tx = self.transaction_repo.create(&transaction).await?;

        // Update invoice status based on payment
        self.update_invoice_status_from_payment(&created_tx, tenant_id)
            .await?;

        Ok(created_tx.into())
    }

    /// Update transaction status
    pub async fn update_transaction_status(
        &self,
        transaction_id: i64,
        status: TransactionStatus,
        error_message: Option<String>,
        tenant_id: &str,
    ) -> Result<(), AppError> {
        // Update transaction
        self.transaction_repo
            .update_status(transaction_id, status.clone(), error_message)
            .await?;

        // Get updated transaction
        let transaction = self
            .transaction_repo
            .find_by_id(transaction_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Transaction not found".to_string()))?;

        // Update invoice status
        self.update_invoice_status_from_payment(&transaction, tenant_id)
            .await?;

        Ok(())
    }

    /// Get transaction by ID
    pub async fn get_transaction(
        &self,
        transaction_id: i64,
    ) -> Result<TransactionResponse, AppError> {
        let transaction = self
            .transaction_repo
            .find_by_id(transaction_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Transaction not found".to_string()))?;

        Ok(transaction.into())
    }

    /// List transactions for an invoice
    pub async fn list_transactions_for_invoice(
        &self,
        invoice_id: i64,
        tenant_id: &str,
    ) -> Result<Vec<TransactionResponse>, AppError> {
        // Verify invoice belongs to tenant
        let _invoice = self
            .invoice_repo
            .find_by_id(invoice_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Invoice not found".to_string()))?;

        let transactions = self
            .transaction_repo
            .list_by_invoice_id(invoice_id)
            .await?;

        Ok(transactions.into_iter().map(|tx| tx.into()).collect())
    }

    /// Record a refund
    pub async fn record_refund(
        &self,
        transaction_id: i64,
        refund_id: String,
        refund_amount: Decimal,
        refund_reason: Option<String>,
    ) -> Result<(), AppError> {
        self.transaction_repo
            .record_refund(transaction_id, refund_id, refund_amount, refund_reason)
            .await
    }

    /// Update invoice status based on payment transaction
    /// Implements status transition logic
    async fn update_invoice_status_from_payment(
        &self,
        transaction: &PaymentTransaction,
        tenant_id: &str,
    ) -> Result<(), AppError> {
        // Get invoice
        let invoice = self
            .invoice_repo
            .find_by_id(transaction.invoice_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Invoice not found".to_string()))?;

        // Determine new invoice status based on transaction status
        let new_status = match transaction.status {
            TransactionStatus::Paid => {
                // Check if fully paid
                let total_paid = self.calculate_total_paid(transaction.invoice_id).await?;
                
                if total_paid >= invoice.total_amount {
                    InvoiceStatus::Paid
                } else if total_paid > Decimal::ZERO {
                    InvoiceStatus::PartiallyPaid
                } else {
                    invoice.status.clone() // No change
                }
            }
            TransactionStatus::Failed => {
                // Only update to failed if no successful payments exist
                let has_successful_payment = self
                    .has_successful_payment(transaction.invoice_id)
                    .await?;
                
                if !has_successful_payment {
                    InvoiceStatus::Failed
                } else {
                    invoice.status.clone() // Keep current status
                }
            }
            TransactionStatus::Pending => {
                // Update to pending if currently draft
                if invoice.status == InvoiceStatus::Draft {
                    InvoiceStatus::Pending
                } else {
                    invoice.status.clone() // Keep current status
                }
            }
            TransactionStatus::Expired => invoice.status.clone(), // No change
            TransactionStatus::Refunded => invoice.status.clone(), // Keep current status (refunds don't change invoice status)
        };

        // Update invoice status if changed
        if new_status != invoice.status {
            self.invoice_repo
                .update_status(invoice.id, new_status, tenant_id)
                .await?;
        }

        Ok(())
    }

    /// Calculate total paid amount for an invoice
    async fn calculate_total_paid(&self, invoice_id: i64) -> Result<Decimal, AppError> {
        let transactions = self
            .transaction_repo
            .list_by_invoice_id(invoice_id)
            .await?;

        let total = transactions
            .iter()
            .filter(|tx| tx.status == TransactionStatus::Paid)
            .map(|tx| tx.amount)
            .sum();

        Ok(total)
    }

    /// Check if invoice has any successful payment
    async fn has_successful_payment(&self, invoice_id: i64) -> Result<bool, AppError> {
        let transactions = self
            .transaction_repo
            .list_by_invoice_id(invoice_id)
            .await?;

        Ok(transactions
            .iter()
            .any(|tx| tx.status == TransactionStatus::Paid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_compiles() {
        // This test ensures the service compiles
        // Actual business logic tests are in integration tests
    }
}
