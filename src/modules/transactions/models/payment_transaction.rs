use crate::core::{AppError, Currency, Result};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::str::FromStr;

/// Payment transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR(20)", rename_all = "lowercase")]
pub enum TransactionStatus {
    /// Transaction pending confirmation
    #[serde(rename = "pending")]
    Pending,
    
    /// Payment successfully completed
    #[serde(rename = "completed")]
    Completed,
    
    /// Payment failed
    #[serde(rename = "failed")]
    Failed,
    
    /// Payment refunded
    #[serde(rename = "refunded")]
    Refunded,
}

impl Default for TransactionStatus {
    fn default() -> Self {
        TransactionStatus::Pending
    }
}

impl std::fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionStatus::Pending => write!(f, "pending"),
            TransactionStatus::Completed => write!(f, "completed"),
            TransactionStatus::Failed => write!(f, "failed"),
            TransactionStatus::Refunded => write!(f, "refunded"),
        }
    }
}

impl std::str::FromStr for TransactionStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "pending" => Ok(TransactionStatus::Pending),
            "completed" => Ok(TransactionStatus::Completed),
            "failed" => Ok(TransactionStatus::Failed),
            "refunded" => Ok(TransactionStatus::Refunded),
            _ => Err(format!("Invalid transaction status: {}", s)),
        }
    }
}

/// Payment transaction record
///
/// Records actual payment attempts and completions for invoices (FR-030, FR-032)
/// Supports idempotency via gateway_transaction_ref uniqueness
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PaymentTransaction {
    /// Unique transaction ID (UUID)
    #[serde(skip_deserializing)]
    pub id: Option<String>,
    
    /// Related invoice ID
    pub invoice_id: String,
    
    /// Related installment ID (if applicable)
    pub installment_id: Option<String>,
    
    /// Gateway's transaction reference (unique, for idempotency)
    pub gateway_transaction_ref: String,
    
    /// Gateway identifier (xendit, midtrans)
    pub gateway_id: String,
    
    /// Actual amount paid
    pub amount_paid: Decimal,
    
    /// Currency
    pub currency: String,
    
    /// Payment method (credit_card, bank_transfer, ewallet)
    pub payment_method: String,
    
    /// Transaction status
    pub status: String,
    
    /// Full gateway webhook payload (JSON)
    pub gateway_response: Option<serde_json::Value>,
    
    /// Transaction creation timestamp
    #[serde(skip_deserializing)]
    pub created_at: Option<DateTime<Utc>>,
    
    /// Last update timestamp
    #[serde(skip_deserializing)]
    pub updated_at: Option<DateTime<Utc>>,
}

impl PaymentTransaction {
    /// Create a new payment transaction
    ///
    /// # Arguments
    /// * `invoice_id` - Invoice ID
    /// * `gateway_transaction_ref` - Gateway's transaction reference
    /// * `gateway_id` - Gateway identifier
    /// * `amount_paid` - Amount paid
    /// * `currency` - Payment currency
    /// * `payment_method` - Payment method used
    /// * `gateway_response` - Full gateway response payload
    ///
    /// # Returns
    /// * `Result<PaymentTransaction>` - New transaction instance
    pub fn new(
        invoice_id: String,
        gateway_transaction_ref: String,
        gateway_id: String,
        amount_paid: Decimal,
        currency: Currency,
        payment_method: String,
        gateway_response: Option<serde_json::Value>,
    ) -> Result<Self> {
        // Validate amount is non-negative
        if amount_paid < Decimal::ZERO {
            return Err(AppError::validation(
                "Amount paid must be non-negative".to_string(),
            ));
        }

        // Validate gateway_transaction_ref is not empty
        if gateway_transaction_ref.trim().is_empty() {
            return Err(AppError::validation(
                "Gateway transaction reference cannot be empty".to_string(),
            ));
        }

        // Validate invoice_id is not empty
        if invoice_id.trim().is_empty() {
            return Err(AppError::validation("Invoice ID cannot be empty".to_string()));
        }

        // Validate gateway_id is not empty
        if gateway_id.trim().is_empty() {
            return Err(AppError::validation("Gateway ID cannot be empty".to_string()));
        }

        // Generate UUID for transaction ID
        let id = uuid::Uuid::new_v4().to_string();

        Ok(Self {
            id: Some(id),
            invoice_id,
            installment_id: None,
            gateway_transaction_ref,
            gateway_id,
            amount_paid,
            currency: currency.to_string(),
            payment_method,
            status: TransactionStatus::Pending.to_string(),
            gateway_response,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        })
    }

    /// Get transaction ID
    pub fn get_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Get transaction status
    pub fn get_status(&self) -> Result<TransactionStatus> {
        TransactionStatus::from_str(&self.status)
            .map_err(|e| AppError::Internal(format!("Invalid transaction status: {}", e)))
    }

    /// Get currency
    pub fn get_currency(&self) -> Result<Currency> {
        Currency::from_str(&self.currency)
            .map_err(|e| AppError::Internal(format!("Invalid currency: {}", e)))
    }

    /// Update transaction status
    ///
    /// # Arguments
    /// * `new_status` - New transaction status
    pub fn update_status(&mut self, new_status: TransactionStatus) -> Result<()> {
        self.status = new_status.to_string();
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    /// Check if transaction is completed
    pub fn is_completed(&self) -> bool {
        matches!(
            self.get_status(),
            Ok(TransactionStatus::Completed)
        )
    }

    /// Check if transaction is failed
    pub fn is_failed(&self) -> bool {
        matches!(
            self.get_status(),
            Ok(TransactionStatus::Failed)
        )
    }

    /// Check if transaction can be refunded
    pub fn can_refund(&self) -> bool {
        self.is_completed()
    }

    /// Link transaction to installment
    ///
    /// # Arguments
    /// * `installment_id` - Installment ID
    pub fn link_to_installment(&mut self, installment_id: String) {
        self.installment_id = Some(installment_id);
        self.updated_at = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_creation_valid() {
        let transaction = PaymentTransaction::new(
            "inv-123".to_string(),
            "gtx-ref-456".to_string(),
            "xendit".to_string(),
            Decimal::new(100000, 0),
            Currency::IDR,
            "credit_card".to_string(),
            None,
        );

        assert!(transaction.is_ok());
        let tx = transaction.unwrap();
        assert!(tx.id.is_some());
        assert_eq!(tx.invoice_id, "inv-123");
        assert_eq!(tx.gateway_transaction_ref, "gtx-ref-456");
        assert_eq!(tx.gateway_id, "xendit");
        assert_eq!(tx.amount_paid, Decimal::new(100000, 0));
        assert_eq!(tx.payment_method, "credit_card");
        assert_eq!(tx.status, "pending");
        assert!(tx.installment_id.is_none());
    }

    #[test]
    fn test_transaction_validation_negative_amount() {
        let transaction = PaymentTransaction::new(
            "inv-123".to_string(),
            "gtx-ref-456".to_string(),
            "xendit".to_string(),
            Decimal::new(-100, 0),
            Currency::IDR,
            "credit_card".to_string(),
            None,
        );

        assert!(transaction.is_err());
    }

    #[test]
    fn test_transaction_validation_empty_gateway_ref() {
        let transaction = PaymentTransaction::new(
            "inv-123".to_string(),
            "".to_string(),
            "xendit".to_string(),
            Decimal::new(100000, 0),
            Currency::IDR,
            "credit_card".to_string(),
            None,
        );

        assert!(transaction.is_err());
    }

    #[test]
    fn test_transaction_validation_empty_invoice_id() {
        let transaction = PaymentTransaction::new(
            "".to_string(),
            "gtx-ref-456".to_string(),
            "xendit".to_string(),
            Decimal::new(100000, 0),
            Currency::IDR,
            "credit_card".to_string(),
            None,
        );

        assert!(transaction.is_err());
    }

    #[test]
    fn test_transaction_status_update() {
        let mut transaction = PaymentTransaction::new(
            "inv-123".to_string(),
            "gtx-ref-456".to_string(),
            "xendit".to_string(),
            Decimal::new(100000, 0),
            Currency::IDR,
            "credit_card".to_string(),
            None,
        )
        .unwrap();

        assert_eq!(transaction.status, "pending");
        assert!(!transaction.is_completed());

        transaction.update_status(TransactionStatus::Completed).unwrap();
        assert_eq!(transaction.status, "completed");
        assert!(transaction.is_completed());
        assert!(transaction.can_refund());
    }

    #[test]
    fn test_transaction_link_to_installment() {
        let mut transaction = PaymentTransaction::new(
            "inv-123".to_string(),
            "gtx-ref-456".to_string(),
            "xendit".to_string(),
            Decimal::new(50000, 0),
            Currency::IDR,
            "credit_card".to_string(),
            None,
        )
        .unwrap();

        assert!(transaction.installment_id.is_none());

        transaction.link_to_installment("inst-789".to_string());
        assert_eq!(transaction.installment_id, Some("inst-789".to_string()));
    }

    #[test]
    fn test_transaction_status_display() {
        assert_eq!(TransactionStatus::Pending.to_string(), "pending");
        assert_eq!(TransactionStatus::Completed.to_string(), "completed");
        assert_eq!(TransactionStatus::Failed.to_string(), "failed");
        assert_eq!(TransactionStatus::Refunded.to_string(), "refunded");
    }

    #[test]
    fn test_transaction_status_from_str() {
        assert_eq!(
            TransactionStatus::from_str("pending").unwrap(),
            TransactionStatus::Pending
        );
        assert_eq!(
            TransactionStatus::from_str("completed").unwrap(),
            TransactionStatus::Completed
        );
        assert_eq!(
            TransactionStatus::from_str("failed").unwrap(),
            TransactionStatus::Failed
        );
        assert_eq!(
            TransactionStatus::from_str("refunded").unwrap(),
            TransactionStatus::Refunded
        );
        assert!(TransactionStatus::from_str("invalid").is_err());
    }
}
