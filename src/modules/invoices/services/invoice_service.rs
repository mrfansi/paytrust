// T042: InvoiceService implementation
// Business logic layer for invoice operations
//
// Implements:
// - Invoice creation with validation
// - Gateway currency validation (FR-046)
// - Expiration management (FR-044, FR-045)
// - Status transition enforcement (FR-051, FR-052)
// - Total calculation

use chrono::Utc;
use sqlx::MySqlPool;
use tracing::{info, warn, error};

use crate::core::{AppError, Currency, Result};
use crate::modules::invoices::{
    models::{Invoice, InvoiceStatus, LineItem},
    repositories::InvoiceRepository,
};
use crate::modules::gateways::repositories::GatewayRepository;

/// Service for invoice business logic
pub struct InvoiceService {
    invoice_repo: InvoiceRepository,
    gateway_repo: GatewayRepository,
}

impl InvoiceService {
    /// Create a new invoice service
    pub fn new(pool: MySqlPool) -> Self {
        Self {
            invoice_repo: InvoiceRepository::new(pool.clone()),
            gateway_repo: GatewayRepository::new(pool),
        }
    }

    /// Create a new invoice
    /// 
    /// # Arguments
    /// * `external_id` - Merchant's reference ID (must be unique)
    /// * `gateway_id` - Payment gateway to use
    /// * `currency` - Invoice currency
    /// * `line_items` - List of line items
    /// 
    /// # Returns
    /// * `Result<Invoice>` - Created invoice
    /// 
    /// # Business Rules
    /// - FR-001: Validates all invoice and line item data
    /// - FR-007: Ensures all line items match invoice currency
    /// - FR-044: Sets expiration to 24 hours from creation
    /// - FR-046: Validates gateway supports the currency
    /// - External ID must be unique per merchant
    pub async fn create_invoice(
        &self,
        external_id: String,
        gateway_id: String,
        currency: Currency,
        line_items: Vec<LineItem>,
    ) -> Result<Invoice> {
        info!(
            external_id = external_id.as_str(),
            gateway_id = gateway_id.as_str(),
            currency = %currency,
            line_items_count = line_items.len(),
            "Creating new invoice"
        );

        // Check external_id uniqueness
        if self.invoice_repo.exists_by_external_id(&external_id).await? {
            warn!(
                external_id = external_id.as_str(),
                "Invoice creation failed - external_id already exists"
            );
            return Err(AppError::validation(format!(
                "Invoice with external_id '{}' already exists",
                external_id
            )));
        }

        // FR-046: Validate gateway supports the currency
        self.validate_gateway_currency(&gateway_id, currency).await?;

        // Create invoice model (performs validation)
        let invoice = Invoice::new(external_id.clone(), gateway_id, currency, line_items)?;

        // Persist to database
        let created_invoice = self.invoice_repo.create(&invoice).await?;
        
        info!(
            invoice_id = created_invoice.id.as_ref().map(|s| s.as_str()),
            external_id = external_id.as_str(),
            total = %created_invoice.total.unwrap_or_default(),
            "Invoice created successfully"
        );

        Ok(created_invoice)
    }

    /// Get invoice by ID
    /// 
    /// # Arguments
    /// * `id` - Invoice ID (UUID)
    /// 
    /// # Returns
    /// * `Result<Invoice>` - Invoice if found, error if not found
    pub async fn get_invoice(&self, id: &str) -> Result<Invoice> {
        self.invoice_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Invoice with id '{}' not found", id)))
    }

    /// Get invoice by external ID
    /// 
    /// # Arguments
    /// * `external_id` - Merchant's reference ID
    /// 
    /// # Returns
    /// * `Result<Invoice>` - Invoice if found
    pub async fn get_invoice_by_external_id(&self, external_id: &str) -> Result<Invoice> {
        self.invoice_repo
            .find_by_external_id(external_id)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "Invoice with external_id '{}' not found",
                    external_id
                ))
            })
    }

    /// List invoices with pagination
    /// 
    /// # Arguments
    /// * `limit` - Maximum results (default: 20, max: 100)
    /// * `offset` - Results to skip
    /// 
    /// # Returns
    /// * `Result<Vec<Invoice>>` - List of invoices
    pub async fn list_invoices(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Invoice>> {
        let limit = limit.unwrap_or(20).min(100) as i32;
        let offset = offset.unwrap_or(0) as i32;
        self.invoice_repo.list(limit, offset).await
    }

    /// Update invoice status with validation
    /// 
    /// # Arguments
    /// * `id` - Invoice ID
    /// * `new_status` - New status to set
    /// 
    /// # Returns
    /// * `Result<Invoice>` - Updated invoice
    /// 
    /// # Business Rules
    /// - FR-051: Enforces immutability (cannot modify after payment initiated)
    /// - FR-052: Validates status transitions
    pub async fn update_invoice_status(
        &self,
        id: &str,
        new_status: InvoiceStatus,
    ) -> Result<Invoice> {
        let mut invoice = self.get_invoice(id).await?;

        let old_status = invoice.status.clone();
        info!(
            invoice_id = id,
            old_status = %old_status,
            new_status = %new_status,
            "Updating invoice status"
        );

        // Validate status transition
        invoice.update_status(new_status)?;

        // Persist status change
        self.invoice_repo.update_status(id, new_status).await?;

        info!(
            invoice_id = id,
            status = %new_status,
            "Invoice status updated successfully"
        );

        // Return updated invoice
        self.get_invoice(id).await
    }

    /// Mark invoice as expired if past expiration time
    /// 
    /// # Arguments
    /// * `id` - Invoice ID
    /// 
    /// # Returns
    /// * `Result<Invoice>` - Updated invoice if expired, unchanged if not
    /// 
    /// # Business Rules
    /// - FR-045: Expired invoices cannot be paid
    pub async fn check_and_expire_invoice(&self, id: &str) -> Result<Invoice> {
        let invoice = self.get_invoice(id).await?;

        // Only expire if in Pending status and past expiration
        if invoice.status == InvoiceStatus::Pending && invoice.is_expired() {
            self.update_invoice_status(id, InvoiceStatus::Expired).await
        } else {
            Ok(invoice)
        }
    }

    /// Initiate payment for an invoice
    /// 
    /// Changes status from Pending to Processing.
    /// After this point, invoice becomes immutable.
    /// 
    /// # Arguments
    /// * `id` - Invoice ID
    /// 
    /// # Returns
    /// * `Result<Invoice>` - Invoice in Processing status
    /// 
    /// # Business Rules
    /// - FR-051: Invoice becomes immutable after payment initiation
    /// - FR-045: Cannot initiate payment on expired invoice
    pub async fn initiate_payment(&self, id: &str) -> Result<Invoice> {
        info!(invoice_id = id, "Initiating payment for invoice");
        
        let invoice = self.get_invoice(id).await?;

        // Check expiration first (FR-045)
        if invoice.is_expired() {
            warn!(invoice_id = id, "Payment initiation failed - invoice expired");
            // Mark as expired and return error
            self.update_invoice_status(id, InvoiceStatus::Expired).await?;
            return Err(AppError::validation(format!(
                "Invoice '{}' has expired and cannot be paid",
                id
            )));
        }

        // Check status
        if invoice.status != InvoiceStatus::Pending {
            warn!(
                invoice_id = id,
                current_status = %invoice.status,
                "Payment initiation failed - invalid status"
            );
            return Err(AppError::validation(format!(
                "Invoice '{}' is not in pending status (current: {:?})",
                id, invoice.status
            )));
        }

        info!(invoice_id = id, "Payment initiated - invoice now immutable");
        
        // Change status to Processing (makes invoice immutable)
        self.update_invoice_status(id, InvoiceStatus::Processing).await
    }

    /// Mark invoice as paid
    /// 
    /// # Arguments
    /// * `id` - Invoice ID
    /// 
    /// # Returns
    /// * `Result<Invoice>` - Invoice in Paid status
    pub async fn mark_as_paid(&self, id: &str) -> Result<Invoice> {
        let invoice = self.get_invoice(id).await?;

        if invoice.status != InvoiceStatus::Processing {
            return Err(AppError::validation(format!(
                "Invoice '{}' is not in processing status (current: {:?})",
                id, invoice.status
            )));
        }

        self.update_invoice_status(id, InvoiceStatus::Paid).await
    }

    /// Mark invoice as failed
    /// 
    /// # Arguments
    /// * `id` - Invoice ID
    /// 
    /// # Returns
    /// * `Result<Invoice>` - Invoice in Failed status
    pub async fn mark_as_failed(&self, id: &str) -> Result<Invoice> {
        let invoice = self.get_invoice(id).await?;

        if invoice.status != InvoiceStatus::Processing {
            return Err(AppError::validation(format!(
                "Invoice '{}' is not in processing status (current: {:?})",
                id, invoice.status
            )));
        }

        self.update_invoice_status(id, InvoiceStatus::Failed).await
    }

    /// Check if invoice is mutable (FR-051, FR-052)
    ///
    /// Invoices can only be modified when in Draft status.
    /// Once payment is initiated (Processing status), they become immutable.
    ///
    /// # Arguments
    /// * `invoice` - Invoice to check
    ///
    /// # Returns
    /// * `Result<()>` - Ok if mutable, error if immutable
    ///
    /// # Business Rules
    /// - FR-051: Invoices immutable once payment initiated
    /// - FR-052: Reject modifications with 400 Bad Request for non-draft status
    pub fn check_invoice_mutable(&self, invoice: &Invoice) -> Result<()> {
        if invoice.status != InvoiceStatus::Pending {
            return Err(AppError::validation(format!(
                "Invoice '{}' cannot be modified - status is '{}' (modifications only allowed for 'pending' invoices)",
                invoice.id.as_ref().unwrap_or(&"unknown".to_string()),
                invoice.status
            )));
        }
        Ok(())
    }

    /// Verify invoice can be modified (by ID)
    ///
    /// # Arguments
    /// * `id` - Invoice ID
    ///
    /// # Returns
    /// * `Result<Invoice>` - Invoice if mutable, error if immutable
    pub async fn verify_invoice_mutable(&self, id: &str) -> Result<Invoice> {
        let invoice = self.get_invoice(id).await?;
        self.check_invoice_mutable(&invoice)?;
        Ok(invoice)
    }

    // Private helper methods

    /// Validate that gateway supports the given currency (FR-046)
    async fn validate_gateway_currency(&self, gateway_id: &str, currency: Currency) -> Result<()> {
        let gateway = self
            .gateway_repo
            .find_by_id(gateway_id)
            .await?
            .ok_or_else(|| {
                AppError::validation(format!("Gateway '{}' not found", gateway_id))
            })?;

        if !gateway.supports_currency(currency) {
            return Err(AppError::validation(format!(
                "Gateway '{}' does not support currency {:?}. Supported currencies: {:?}",
                gateway_id, currency, gateway.supported_currencies
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    // Note: Full integration tests with database will be in tests/integration/
    // These are unit tests for business logic that doesn't require database
    
    #[test]
    fn test_service_creation() {
        // This test just ensures the module compiles
        // Real tests require database setup
    }
}
