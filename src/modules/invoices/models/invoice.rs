use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::core::currency::Currency;
use crate::core::error::AppError;

/// Invoice status enum representing the lifecycle of an invoice
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum InvoiceStatus {
    Draft,
    Pending,
    PartiallyPaid,
    Paid,
    Failed,
    Expired,
}

impl std::fmt::Display for InvoiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvoiceStatus::Draft => write!(f, "draft"),
            InvoiceStatus::Pending => write!(f, "pending"),
            InvoiceStatus::PartiallyPaid => write!(f, "partially_paid"),
            InvoiceStatus::Paid => write!(f, "paid"),
            InvoiceStatus::Failed => write!(f, "failed"),
            InvoiceStatus::Expired => write!(f, "expired"),
        }
    }
}

/// Invoice entity representing a payment request
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invoice {
    pub id: i64,
    pub tenant_id: String,
    pub external_id: String,
    pub currency: Currency,
    pub subtotal: Decimal,
    pub tax_total: Decimal,
    pub service_fee: Decimal,
    pub total_amount: Decimal,
    pub status: InvoiceStatus,
    pub gateway_id: i64,
    pub original_invoice_id: Option<i64>,
    pub payment_initiated_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Invoice {
    /// Check if invoice is immutable (payment has been initiated)
    /// Per FR-051: Invoice becomes immutable once payment_initiated_at is set
    pub fn is_immutable(&self) -> bool {
        self.payment_initiated_at.is_some()
    }

    /// Validate invoice state transitions
    pub fn can_transition_to(&self, new_status: &InvoiceStatus) -> Result<(), AppError> {
        use InvoiceStatus::*;

        let valid = match (&self.status, new_status) {
            // Draft can transition to pending
            (Draft, Pending) => true,
            // Pending can transition to partially_paid, paid, failed, or expired
            (Pending, PartiallyPaid) | (Pending, Paid) | (Pending, Failed) | (Pending, Expired) => {
                true
            }
            // PartiallyPaid can transition to paid or expired
            (PartiallyPaid, Paid) | (PartiallyPaid, Expired) => true,
            // Terminal states cannot transition
            (Paid, _) | (Failed, _) | (Expired, _) => false,
            // Same status is allowed (idempotent)
            (a, b) if a == b => true,
            // All other transitions are invalid
            _ => false,
        };

        if !valid {
            return Err(AppError::Validation(format!(
                "Invalid status transition from {} to {}",
                self.status, new_status
            )));
        }

        Ok(())
    }

    /// Check if invoice has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Validate invoice can be modified
    /// Per FR-051, FR-052: Reject modifications when payment initiated
    pub fn validate_can_modify(&self) -> Result<(), AppError> {
        if self.is_immutable() {
            return Err(AppError::Validation(
                "Cannot modify invoice after payment has been initiated".to_string(),
            ));
        }
        Ok(())
    }
}

/// Request to create a new invoice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvoiceRequest {
    pub external_id: String,
    pub currency: Currency,
    pub gateway_id: i64,
    pub line_items: Vec<CreateLineItemRequest>,
    pub expires_at: Option<DateTime<Utc>>,
    pub installment_count: Option<u32>,
    pub installment_custom_amounts: Option<Vec<Decimal>>,
}

/// Request to create a line item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLineItemRequest {
    pub product_name: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Decimal,
    pub tax_category: Option<String>,
}

/// Response when creating or retrieving an invoice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceResponse {
    pub id: i64,
    pub external_id: String,
    pub tenant_id: String,
    pub currency: Currency,
    pub subtotal: String,
    pub tax_total: String,
    pub service_fee: String,
    pub total_amount: String,
    pub status: InvoiceStatus,
    pub payment_url: Option<String>,
    pub is_immutable: bool,
    pub expires_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub line_items: Vec<LineItemResponse>,
}

/// Line item in invoice response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItemResponse {
    pub id: i64,
    pub product_name: String,
    pub quantity: String,
    pub unit_price: String,
    pub subtotal: String,
    pub tax_rate: String,
    pub tax_category: Option<String>,
    pub tax_amount: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoice_immutability() {
        let mut invoice = Invoice {
            id: 1,
            tenant_id: "tenant-1".to_string(),
            external_id: "INV-001".to_string(),
            currency: Currency::IDR,
            subtotal: Decimal::new(1000000, 0),
            tax_total: Decimal::new(100000, 0),
            service_fee: Decimal::new(31000, 0),
            total_amount: Decimal::new(1131000, 0),
            status: InvoiceStatus::Draft,
            gateway_id: 1,
            original_invoice_id: None,
            payment_initiated_at: None,
            expires_at: Utc::now() + chrono::Duration::hours(24),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Before payment initiated, invoice is mutable
        assert!(!invoice.is_immutable());
        assert!(invoice.validate_can_modify().is_ok());

        // After payment initiated, invoice is immutable
        invoice.payment_initiated_at = Some(Utc::now());
        assert!(invoice.is_immutable());
        assert!(invoice.validate_can_modify().is_err());
    }

    #[test]
    fn test_status_transitions() {
        let invoice = Invoice {
            id: 1,
            tenant_id: "tenant-1".to_string(),
            external_id: "INV-001".to_string(),
            currency: Currency::IDR,
            subtotal: Decimal::new(1000000, 0),
            tax_total: Decimal::new(100000, 0),
            service_fee: Decimal::new(31000, 0),
            total_amount: Decimal::new(1131000, 0),
            status: InvoiceStatus::Pending,
            gateway_id: 1,
            original_invoice_id: None,
            payment_initiated_at: Some(Utc::now()),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Valid transitions from Pending
        assert!(invoice.can_transition_to(&InvoiceStatus::PartiallyPaid).is_ok());
        assert!(invoice.can_transition_to(&InvoiceStatus::Paid).is_ok());
        assert!(invoice.can_transition_to(&InvoiceStatus::Failed).is_ok());
        assert!(invoice.can_transition_to(&InvoiceStatus::Expired).is_ok());

        // Invalid transition from Pending to Draft
        assert!(invoice.can_transition_to(&InvoiceStatus::Draft).is_err());
    }
}
