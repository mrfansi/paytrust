use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Payment transaction status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TransactionStatus {
    Pending,
    Paid,
    Failed,
    Expired,
    Refunded,
}

impl std::fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionStatus::Pending => write!(f, "pending"),
            TransactionStatus::Paid => write!(f, "paid"),
            TransactionStatus::Failed => write!(f, "failed"),
            TransactionStatus::Expired => write!(f, "expired"),
            TransactionStatus::Refunded => write!(f, "refunded"),
        }
    }
}

/// Payment transaction entity
/// Implements FR-030 (transaction recording) and FR-032 (idempotency)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PaymentTransaction {
    pub id: i64,
    pub invoice_id: i64,
    pub gateway_reference: String,
    pub idempotency_key: String,
    pub amount: Decimal,
    pub currency: String,
    pub status: TransactionStatus,
    pub payment_method: String,
    pub gateway_response: Option<String>,
    pub error_message: Option<String>,
    pub refund_id: Option<String>,
    pub refund_amount: Option<Decimal>,
    pub refund_timestamp: Option<DateTime<Utc>>,
    pub refund_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PaymentTransaction {
    /// Create a new payment transaction
    pub fn new(
        invoice_id: i64,
        gateway_reference: String,
        idempotency_key: String,
        amount: Decimal,
        currency: String,
        payment_method: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: 0, // Will be set by database
            invoice_id,
            gateway_reference,
            idempotency_key,
            amount,
            currency,
            status: TransactionStatus::Pending,
            payment_method,
            gateway_response: None,
            error_message: None,
            refund_id: None,
            refund_amount: None,
            refund_timestamp: None,
            refund_reason: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if transaction is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            TransactionStatus::Paid | TransactionStatus::Failed | TransactionStatus::Expired
        )
    }

    /// Check if transaction can be refunded
    pub fn can_refund(&self) -> bool {
        self.status == TransactionStatus::Paid && self.refund_id.is_none()
    }
}

/// Request to create a payment transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTransactionRequest {
    pub invoice_id: i64,
    pub gateway_reference: String,
    pub idempotency_key: String,
    pub amount: Decimal,
    pub currency: String,
    pub payment_method: String,
}

/// Response for payment transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub id: i64,
    pub invoice_id: i64,
    pub gateway_reference: String,
    pub amount: String,
    pub currency: String,
    pub status: TransactionStatus,
    pub payment_method: String,
    pub error_message: Option<String>,
    pub refund_id: Option<String>,
    pub refund_amount: Option<String>,
    pub refund_timestamp: Option<String>,
    pub refund_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<PaymentTransaction> for TransactionResponse {
    fn from(tx: PaymentTransaction) -> Self {
        Self {
            id: tx.id,
            invoice_id: tx.invoice_id,
            gateway_reference: tx.gateway_reference,
            amount: tx.amount.to_string(),
            currency: tx.currency,
            status: tx.status,
            payment_method: tx.payment_method,
            error_message: tx.error_message,
            refund_id: tx.refund_id,
            refund_amount: tx.refund_amount.map(|a| a.to_string()),
            refund_timestamp: tx.refund_timestamp.map(|t| t.to_rfc3339()),
            refund_reason: tx.refund_reason,
            created_at: tx.created_at.to_rfc3339(),
            updated_at: tx.updated_at.to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_new_transaction() {
        let tx = PaymentTransaction::new(
            1,
            "gw-ref-123".to_string(),
            "idempotency-key-123".to_string(),
            dec!(1000000),
            "IDR".to_string(),
            "bank_transfer".to_string(),
        );

        assert_eq!(tx.invoice_id, 1);
        assert_eq!(tx.status, TransactionStatus::Pending);
        assert!(!tx.is_terminal());
        assert!(!tx.can_refund()); // Can't refund pending transaction
    }

    #[test]
    fn test_terminal_states() {
        let mut tx = PaymentTransaction::new(
            1,
            "ref".to_string(),
            "key".to_string(),
            dec!(1000),
            "IDR".to_string(),
            "bank".to_string(),
        );

        assert!(!tx.is_terminal());

        tx.status = TransactionStatus::Paid;
        assert!(tx.is_terminal());
        assert!(tx.can_refund());

        tx.status = TransactionStatus::Failed;
        assert!(tx.is_terminal());
        assert!(!tx.can_refund());
    }

    #[test]
    fn test_refund_eligibility() {
        let mut tx = PaymentTransaction::new(
            1,
            "ref".to_string(),
            "key".to_string(),
            dec!(1000),
            "IDR".to_string(),
            "bank".to_string(),
        );

        tx.status = TransactionStatus::Paid;
        assert!(tx.can_refund());

        tx.refund_id = Some("refund-123".to_string());
        assert!(!tx.can_refund()); // Already refunded
    }
}
