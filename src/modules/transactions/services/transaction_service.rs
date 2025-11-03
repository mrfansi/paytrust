use std::sync::Arc;

use rust_decimal::Decimal;
use tracing::info;

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
        info!(
            invoice_id = transaction.invoice_id,
            amount = %transaction.amount,
            status = ?transaction.status,
            gateway_tx_ref = %transaction.gateway_reference,
            "Recording payment transaction"
        );

        // Create transaction (with idempotency check)
        let created_tx = self.transaction_repo.create(&transaction).await?;

        info!(
            transaction_id = created_tx.id,
            invoice_id = created_tx.invoice_id,
            "Payment transaction recorded successfully"
        );

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

    /// Process webhook event from payment gateway
    /// This is called by WebhookHandler with retry logic
    pub async fn process_webhook_event(
        &self,
        gateway: &str,
        webhook_data: &serde_json::Value,
    ) -> Result<(), AppError> {
        // Parse webhook data based on gateway type
        let (invoice_id, transaction_status, amount, _gateway_tx_ref) = 
            self.parse_webhook_data(gateway, webhook_data)?;

        // TODO: Implement actual webhook processing logic
        // 1. Find or create transaction record
        // 2. Update transaction status
        // 3. Update invoice status
        // 4. Handle overpayments if applicable
        
        tracing::info!(
            gateway = %gateway,
            invoice_id = invoice_id,
            status = ?transaction_status,
            amount = %amount,
            "Processing webhook event"
        );

        Ok(())
    }

    /// Parse webhook data based on gateway type
    fn parse_webhook_data(
        &self,
        gateway: &str,
        _data: &serde_json::Value,
    ) -> Result<(i64, TransactionStatus, Decimal, String), AppError> {
        // TODO: Implement actual parsing for Xendit and Midtrans webhooks
        // This is a placeholder that needs gateway-specific implementation
        
        match gateway.to_lowercase().as_str() {
            "xendit" => {
                // Parse Xendit webhook format
                Ok((0, TransactionStatus::Pending, Decimal::ZERO, String::new()))
            }
            "midtrans" => {
                // Parse Midtrans webhook format
                Ok((0, TransactionStatus::Pending, Decimal::ZERO, String::new()))
            }
            _ => Err(AppError::Validation(format!("Unknown gateway: {}", gateway))),
        }
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

    /// Process refund webhook from payment gateway (FR-086)
    /// Handles Xendit invoice.refunded and Midtrans refund events
    pub async fn process_refund_webhook(
        &self,
        gateway: &str,
        data: &serde_json::Value,
    ) -> Result<(), AppError> {
        info!(gateway = %gateway, "Processing refund webhook");

        // Extract refund data based on gateway
        let (gateway_ref, refund_id, refund_amount, refund_reason) = match gateway.to_lowercase().as_str() {
            "xendit" => self.extract_xendit_refund_data(data)?,
            "midtrans" => self.extract_midtrans_refund_data(data)?,
            _ => return Err(AppError::Validation(format!("Unknown gateway: {}", gateway))),
        };

        // Find transaction by gateway reference
        let transaction = self
            .transaction_repo
            .find_by_gateway_reference(&gateway_ref)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Transaction not found for gateway ref: {}", gateway_ref)))?;

        // Verify transaction can be refunded
        if !transaction.can_refund() {
            return Err(AppError::Validation(format!(
                "Transaction {} cannot be refunded (status: {:?}, already refunded: {})",
                transaction.id,
                transaction.status,
                transaction.refund_id.is_some()
            )));
        }

        // Update transaction with refund information
        self.transaction_repo
            .record_refund(
                transaction.id,
                refund_id.clone(),
                refund_amount,
                Some(refund_reason),
            )
            .await?;

        info!(
            transaction_id = transaction.id,
            refund_id = %refund_id,
            refund_amount = %refund_amount,
            "Refund processed successfully"
        );

        Ok(())
    }

    /// Extract refund data from Xendit webhook payload
    fn extract_xendit_refund_data(
        &self,
        data: &serde_json::Value,
    ) -> Result<(String, String, Decimal, String), AppError> {
        let gateway_ref = data["id"]
            .as_str()
            .ok_or_else(|| AppError::Validation("Missing Xendit invoice ID".to_string()))?
            .to_string();

        let refund_id = data["refund_id"]
            .as_str()
            .ok_or_else(|| AppError::Validation("Missing Xendit refund ID".to_string()))?
            .to_string();

        let refund_amount = data["refund_amount"]
            .as_f64()
            .ok_or_else(|| AppError::Validation("Missing Xendit refund amount".to_string()))?;

        let refund_reason = data["refund_reason"]
            .as_str()
            .unwrap_or("No reason provided")
            .to_string();

        Ok((
            gateway_ref,
            refund_id,
            Decimal::from_f64_retain(refund_amount)
                .ok_or_else(|| AppError::Validation("Invalid refund amount".to_string()))?,
            refund_reason,
        ))
    }

    /// Extract refund data from Midtrans webhook payload
    fn extract_midtrans_refund_data(
        &self,
        data: &serde_json::Value,
    ) -> Result<(String, String, Decimal, String), AppError> {
        let gateway_ref = data["transaction_id"]
            .as_str()
            .ok_or_else(|| AppError::Validation("Missing Midtrans transaction ID".to_string()))?
            .to_string();

        let refund_id = data["refund_key"]
            .as_str()
            .ok_or_else(|| AppError::Validation("Missing Midtrans refund key".to_string()))?
            .to_string();

        let refund_amount = data["refund_amount"]
            .as_str()
            .ok_or_else(|| AppError::Validation("Missing Midtrans refund amount".to_string()))?
            .parse::<f64>()
            .map_err(|e| AppError::Validation(format!("Invalid refund amount: {}", e)))?;

        let refund_reason = data["refund_reason"]
            .as_str()
            .unwrap_or("No reason provided")
            .to_string();

        Ok((
            gateway_ref,
            refund_id,
            Decimal::from_f64_retain(refund_amount)
                .ok_or_else(|| AppError::Validation("Invalid refund amount".to_string()))?,
            refund_reason,
        ))
    }

    /// Get refund history for an invoice (FR-086)
    pub async fn get_refund_history(
        &self,
        invoice_id: i64,
    ) -> Result<Vec<RefundRecord>, AppError> {
        let transactions = self
            .transaction_repo
            .list_by_invoice_id(invoice_id)
            .await?;

        let refunds: Vec<RefundRecord> = transactions
            .iter()
            .filter(|tx| tx.refund_id.is_some())
            .map(|tx| RefundRecord {
                refund_id: tx.refund_id.clone().unwrap(),
                refund_amount: tx.refund_amount.unwrap().to_string(),
                refund_timestamp: tx.refund_timestamp.map(|t| t.to_rfc3339()).unwrap_or_default(),
                refund_reason: tx.refund_reason.clone().unwrap_or_default(),
                original_transaction_id: tx.id,
                gateway_reference: tx.gateway_reference.clone(),
            })
            .collect();

        Ok(refunds)
    }
}

/// Refund record for API response (FR-086)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RefundRecord {
    pub refund_id: String,
    pub refund_amount: String,
    pub refund_timestamp: String,
    pub refund_reason: String,
    pub original_transaction_id: i64,
    pub gateway_reference: String,
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
