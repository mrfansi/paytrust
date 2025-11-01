// T039: Invoice model with validation
// Implements FR-001 (invoice creation), FR-004 (expiration), FR-051 (immutability)
//
// An invoice represents a payment request with multiple line items.
// Invoices track their status, expiration, and total amount.
// Once payment is initiated, invoices become immutable (FR-051).

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::line_item::LineItem;
use crate::core::{AppError, Currency, Result};

/// Invoice status lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR(20)", rename_all = "lowercase")]
pub enum InvoiceStatus {
    /// Invoice created but no payment initiated
    #[serde(rename = "pending")]
    Pending,

    /// Payment initiated with gateway
    #[serde(rename = "processing")]
    Processing,

    /// Payment successfully completed (full or single payment)
    #[serde(rename = "paid")]
    Paid,

    /// First installment paid, remaining installments pending (FR-019, T094)
    #[serde(rename = "partially_paid")]
    PartiallyPaid,

    /// All installments completed (FR-020, T095)
    #[serde(rename = "fully_paid")]
    FullyPaid,

    /// Invoice expired without payment (FR-044)
    #[serde(rename = "expired")]
    Expired,

    /// Payment failed or cancelled
    #[serde(rename = "failed")]
    Failed,
}

impl Default for InvoiceStatus {
    fn default() -> Self {
        InvoiceStatus::Pending
    }
}

impl std::fmt::Display for InvoiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvoiceStatus::Pending => write!(f, "pending"),
            InvoiceStatus::Processing => write!(f, "processing"),
            InvoiceStatus::Paid => write!(f, "paid"),
            InvoiceStatus::PartiallyPaid => write!(f, "partially_paid"),
            InvoiceStatus::FullyPaid => write!(f, "fully_paid"),
            InvoiceStatus::Expired => write!(f, "expired"),
            InvoiceStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for InvoiceStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "pending" => Ok(InvoiceStatus::Pending),
            "processing" => Ok(InvoiceStatus::Processing),
            "paid" => Ok(InvoiceStatus::Paid),
            "partially_paid" => Ok(InvoiceStatus::PartiallyPaid),
            "fully_paid" => Ok(InvoiceStatus::FullyPaid),
            "expired" => Ok(InvoiceStatus::Expired),
            "failed" => Ok(InvoiceStatus::Failed),
            _ => Err(format!("Invalid invoice status: {}", s)),
        }
    }
}

/// Represents a payment invoice
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invoice {
    /// Unique invoice ID (UUID)
    #[serde(skip_deserializing)]
    pub id: Option<String>,

    /// External reference ID from merchant
    pub external_id: String,

    /// Payment gateway ID to use
    pub gateway_id: String,

    /// Currency for the entire invoice
    #[sqlx(try_from = "String")]
    pub currency: Currency,

    /// Subtotal amount (sum of line item subtotals, before tax and fees)
    #[serde(skip_deserializing)]
    #[sqlx(try_from = "rust_decimal::Decimal")]
    pub subtotal: Option<Decimal>,

    /// Total tax amount (sum of line item taxes, FR-057, FR-058)
    #[serde(skip_deserializing)]
    #[sqlx(try_from = "rust_decimal::Decimal")]
    pub tax_total: Option<Decimal>,

    /// Service fee charged by payment gateway (FR-009, FR-047)
    #[serde(skip_deserializing)]
    #[sqlx(try_from = "rust_decimal::Decimal")]
    pub service_fee: Option<Decimal>,

    /// Total amount (subtotal + tax_total + service_fee, FR-056)
    #[serde(skip_deserializing)]
    #[sqlx(try_from = "rust_decimal::Decimal")]
    pub total: Option<Decimal>,

    /// Current status
    #[serde(skip_deserializing)]
    pub status: InvoiceStatus,

    /// When invoice expires (FR-044: 24 hours default)
    #[serde(skip_deserializing)]
    pub expires_at: Option<DateTime<Utc>>,

    /// Reference to original invoice for supplementary invoices (FR-082, T103)
    /// When overpayment exceeds all installments, a supplementary invoice can be created
    pub original_invoice_id: Option<String>,

    /// When invoice was created
    #[serde(skip_deserializing)]
    pub created_at: Option<DateTime<Utc>>,

    /// When invoice was last updated
    #[serde(skip_deserializing)]
    pub updated_at: Option<DateTime<Utc>>,

    /// Line items (not stored in this table, joined from line_items table)
    #[sqlx(skip)]
    #[serde(default)]
    pub line_items: Vec<LineItem>,
}

impl Invoice {
    /// Create a new invoice with validation
    ///
    /// # Arguments
    /// * `external_id` - Merchant's reference ID (must be unique per merchant)
    /// * `gateway_id` - Payment gateway to use
    /// * `currency` - Invoice currency
    /// * `line_items` - List of line items (must not be empty)
    ///
    /// # Returns
    /// * `Result<Self>` - Validated invoice with calculated total and expiration
    pub fn new(
        external_id: String,
        gateway_id: String,
        currency: Currency,
        line_items: Vec<LineItem>,
    ) -> Result<Self> {
        // FR-001: Validate invoice data
        Self::validate_external_id(&external_id)?;
        Self::validate_gateway_id(&gateway_id)?;
        Self::validate_line_items(&line_items)?;
        Self::validate_line_items_currency(&line_items, currency)?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // FR-044: Set expiration to 24 hours from now
        let expires_at = now + Duration::hours(24);

        let mut invoice = Self {
            id: Some(id),
            external_id,
            gateway_id,
            currency,
            subtotal: None,
            tax_total: None,
            service_fee: None,
            total: None,
            status: InvoiceStatus::Pending,
            expires_at: Some(expires_at),
            original_invoice_id: None, // T103: null for regular invoices
            created_at: Some(now),
            updated_at: Some(now),
            line_items,
        };

        // Calculate subtotal immediately (total calculated after taxes/fees set)
        invoice.calculate_subtotal();

        Ok(invoice)
    }

    /// Calculate subtotal from all line items (before tax and fees)
    ///
    /// Formula: subtotal = sum(line_item.subtotal)
    /// Rounding: Per currency scale
    ///
    /// # Updates
    /// * Sets `self.subtotal` to calculated value
    pub fn calculate_subtotal(&mut self) {
        let raw_subtotal: Decimal = self
            .line_items
            .iter_mut()
            .map(|item| item.get_subtotal())
            .sum();

        self.subtotal = Some(self.currency.round(raw_subtotal));
    }

    /// Calculate tax total from all line items (FR-057, FR-058)
    ///
    /// Formula: tax_total = sum(line_item.tax_amount)
    /// Rounding: Per currency scale
    ///
    /// # Updates
    /// * Sets `self.tax_total` to calculated value
    pub fn calculate_tax_total(&mut self) {
        let raw_tax: Decimal = self
            .line_items
            .iter()
            .map(|item| item.tax_amount.unwrap_or(Decimal::ZERO))
            .sum();

        self.tax_total = Some(self.currency.round(raw_tax));
    }

    /// Calculate final total (FR-056)
    ///
    /// Formula: total = subtotal + tax_total + service_fee
    /// Rounding: Per currency scale
    ///
    /// # Updates
    /// * Sets `self.total` to calculated value
    pub fn calculate_total(&mut self) {
        let subtotal = self.subtotal.unwrap_or(Decimal::ZERO);
        let tax_total = self.tax_total.unwrap_or(Decimal::ZERO);
        let service_fee = self.service_fee.unwrap_or(Decimal::ZERO);

        let raw_total = subtotal + tax_total + service_fee;
        self.total = Some(self.currency.round(raw_total));
    }

    /// Get the subtotal, calculating if not set
    pub fn get_subtotal(&mut self) -> Decimal {
        if self.subtotal.is_none() {
            self.calculate_subtotal();
        }
        self.subtotal.unwrap_or(Decimal::ZERO)
    }

    /// Get the total, calculating if not set
    pub fn get_total(&mut self) -> Decimal {
        if self.total.is_none() {
            self.calculate_total();
        }
        self.total.unwrap_or(Decimal::ZERO)
    }

    /// Check if invoice is expired (FR-045)
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Check if invoice is mutable (FR-051)
    ///
    /// Invoice becomes immutable once payment is initiated (status != Pending)
    pub fn is_mutable(&self) -> bool {
        self.status == InvoiceStatus::Pending
    }

    /// Update status with immutability check (FR-051, FR-052)
    pub fn update_status(&mut self, new_status: InvoiceStatus) -> Result<()> {
        // Allow status transitions only in valid directions
        match (self.status, new_status) {
            (InvoiceStatus::Pending, _) => {
                // Pending can transition to any status
                self.status = new_status;
                self.updated_at = Some(Utc::now());
                Ok(())
            }
            (InvoiceStatus::Processing, InvoiceStatus::Paid)
            | (InvoiceStatus::Processing, InvoiceStatus::Failed) => {
                // Processing can only go to Paid or Failed
                self.status = new_status;
                self.updated_at = Some(Utc::now());
                Ok(())
            }
            _ => Err(AppError::validation(format!(
                "Invalid status transition from {:?} to {:?}",
                self.status, new_status
            ))),
        }
    }

    // Validation methods

    fn validate_external_id(external_id: &str) -> Result<()> {
        if external_id.trim().is_empty() {
            return Err(AppError::validation("External ID cannot be empty"));
        }

        if external_id.len() > 100 {
            return Err(AppError::validation(
                "External ID cannot exceed 100 characters",
            ));
        }

        Ok(())
    }

    fn validate_gateway_id(gateway_id: &str) -> Result<()> {
        if gateway_id.trim().is_empty() {
            return Err(AppError::validation("Gateway ID cannot be empty"));
        }

        Ok(())
    }

    fn validate_line_items(line_items: &[LineItem]) -> Result<()> {
        if line_items.is_empty() {
            return Err(AppError::validation(
                "Invoice must have at least one line item",
            ));
        }

        Ok(())
    }

    fn validate_line_items_currency(
        line_items: &[LineItem],
        invoice_currency: Currency,
    ) -> Result<()> {
        // FR-007: All line items must match invoice currency
        for (idx, item) in line_items.iter().enumerate() {
            if item.currency != invoice_currency {
                return Err(AppError::validation(format!(
                    "Line item {} currency ({:?}) does not match invoice currency ({:?})",
                    idx, item.currency, invoice_currency
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn create_test_line_item(
        description: &str,
        quantity: i32,
        price: i32,
        currency: Currency,
    ) -> LineItem {
        LineItem::new(
            description.to_string(),
            quantity,
            Decimal::from(price),
            currency,
        )
        .unwrap()
    }

    #[test]
    fn test_invoice_creation_valid() {
        let line_items = vec![
            create_test_line_item("Product A", 2, 1000, Currency::IDR),
            create_test_line_item("Product B", 1, 500, Currency::IDR),
        ];

        let invoice = Invoice::new(
            "INV-001".to_string(),
            "xendit".to_string(),
            Currency::IDR,
            line_items,
        );

        assert!(invoice.is_ok());
        let mut inv = invoice.unwrap();
        assert_eq!(inv.external_id, "INV-001");
        assert_eq!(inv.status, InvoiceStatus::Pending);
        assert_eq!(inv.get_total(), Decimal::from(2500)); // (2*1000) + (1*500)
        assert!(inv.expires_at.is_some());
    }

    #[test]
    fn test_invoice_total_calculation() {
        let line_items = vec![
            create_test_line_item("Item 1", 3, 15000, Currency::IDR),
            create_test_line_item("Item 2", 2, 25000, Currency::IDR),
            create_test_line_item("Item 3", 1, 10000, Currency::IDR),
        ];

        let mut invoice = Invoice::new(
            "INV-002".to_string(),
            "midtrans".to_string(),
            Currency::IDR,
            line_items,
        )
        .unwrap();

        // (3*15000) + (2*25000) + (1*10000) = 45000 + 50000 + 10000 = 105000
        assert_eq!(invoice.get_total(), Decimal::from(105000));
    }

    #[test]
    fn test_invoice_validation_empty_line_items() {
        let result = Invoice::new(
            "INV-003".to_string(),
            "xendit".to_string(),
            Currency::USD,
            vec![],
        );

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least one line item"));
    }

    #[test]
    fn test_invoice_validation_currency_mismatch() {
        let line_items = vec![
            create_test_line_item("Product A", 1, 100, Currency::IDR),
            create_test_line_item("Product B", 1, 100, Currency::MYR), // Different currency!
        ];

        let result = Invoice::new(
            "INV-004".to_string(),
            "xendit".to_string(),
            Currency::IDR,
            line_items,
        );

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("does not match invoice currency"));
    }

    #[test]
    fn test_invoice_status_transition_valid() {
        let line_items = vec![create_test_line_item("Product", 1, 1000, Currency::IDR)];
        let mut invoice = Invoice::new(
            "INV-005".to_string(),
            "xendit".to_string(),
            Currency::IDR,
            line_items,
        )
        .unwrap();

        // Pending -> Processing
        assert!(invoice.update_status(InvoiceStatus::Processing).is_ok());
        assert_eq!(invoice.status, InvoiceStatus::Processing);

        // Processing -> Paid
        assert!(invoice.update_status(InvoiceStatus::Paid).is_ok());
        assert_eq!(invoice.status, InvoiceStatus::Paid);
    }

    #[test]
    fn test_invoice_status_transition_invalid() {
        let line_items = vec![create_test_line_item("Product", 1, 1000, Currency::IDR)];
        let mut invoice = Invoice::new(
            "INV-006".to_string(),
            "xendit".to_string(),
            Currency::IDR,
            line_items,
        )
        .unwrap();

        // Move to Paid
        invoice.update_status(InvoiceStatus::Paid).unwrap();

        // Try to go back to Pending (invalid)
        let result = invoice.update_status(InvoiceStatus::Pending);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid status transition"));
    }

    #[test]
    fn test_invoice_immutability() {
        let line_items = vec![create_test_line_item("Product", 1, 1000, Currency::IDR)];
        let mut invoice = Invoice::new(
            "INV-007".to_string(),
            "xendit".to_string(),
            Currency::IDR,
            line_items,
        )
        .unwrap();

        // Initially mutable
        assert!(invoice.is_mutable());

        // After payment initiated, becomes immutable
        invoice.update_status(InvoiceStatus::Processing).unwrap();
        assert!(!invoice.is_mutable());
    }
}
