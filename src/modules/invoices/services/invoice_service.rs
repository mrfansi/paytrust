// T042: InvoiceService implementation
// Business logic layer for invoice operations
//
// Implements:
// - Invoice creation with validation
// - Gateway currency validation (FR-046)
// - Expiration management (FR-044, FR-045)
// - Status transition enforcement (FR-051, FR-052)
// - Total calculation

use sqlx::MySqlPool;
use tracing::{info, warn};

use crate::core::{AppError, Currency, Result};
use crate::modules::gateways::repositories::GatewayRepository;
use crate::modules::installments::{models::InstallmentConfig, services::InstallmentService};
use crate::modules::invoices::{
    models::{Invoice, InvoiceStatus, LineItem},
    repositories::InvoiceRepository,
};
use rust_decimal::Decimal;

/// Service for invoice business logic
pub struct InvoiceService {
    invoice_repo: InvoiceRepository,
    gateway_repo: GatewayRepository,
    installment_service: InstallmentService,
}

impl InvoiceService {
    /// Create a new invoice service
    pub fn new(pool: MySqlPool) -> Self {
        Self {
            invoice_repo: InvoiceRepository::new(pool.clone()),
            gateway_repo: GatewayRepository::new(pool.clone()),
            installment_service: InstallmentService::new(pool),
        }
    }

    /// Create a new invoice
    ///
    /// # Arguments
    /// * `external_id` - Merchant's reference ID (must be unique)
    /// * `gateway_id` - Payment gateway to use
    /// * `currency` - Invoice currency
    /// * `line_items` - List of line items
    /// * `installment_config` - Optional installment configuration (FR-014)
    ///
    /// # Returns
    /// * `Result<Invoice>` - Created invoice
    ///
    /// # Business Rules
    /// - FR-001: Validates all invoice and line item data
    /// - FR-007: Ensures all line items match invoice currency
    /// - FR-014: Supports 2-12 installments if config provided
    /// - FR-044: Sets expiration to 24 hours from creation
    /// - FR-046: Validates gateway supports the currency
    /// - External ID must be unique per merchant
    pub async fn create_invoice(
        &self,
        external_id: String,
        gateway_id: String,
        currency: Currency,
        line_items: Vec<LineItem>,
        installment_config: Option<InstallmentConfig>,
    ) -> Result<Invoice> {
        info!(
            external_id = external_id.as_str(),
            gateway_id = gateway_id.as_str(),
            currency = %currency,
            line_items_count = line_items.len(),
            "Creating new invoice"
        );

        // Check external_id uniqueness
        if self
            .invoice_repo
            .exists_by_external_id(&external_id)
            .await?
        {
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
        self.validate_gateway_currency(&gateway_id, currency)
            .await?;

        // Create invoice model (performs validation and locks tax rates - FR-061)
        let mut invoice = Invoice::new(
            external_id.clone(),
            gateway_id.clone(),
            currency,
            line_items,
        )?;

        // FR-057, FR-058: Calculate tax total from locked line item taxes
        invoice.calculate_tax_total();

        // FR-009, FR-047: Calculate service fee based on gateway configuration
        let service_fee = self
            .calculate_service_fee_for_invoice(
                &gateway_id,
                invoice.subtotal.unwrap_or(Decimal::ZERO),
            )
            .await?;
        invoice.service_fee = Some(service_fee);

        // FR-055, FR-056: Calculate final total = subtotal + tax_total + service_fee
        invoice.calculate_total();

        // Persist to database
        let created_invoice = self.invoice_repo.create(&invoice).await?;

        // T093: Create installment schedules if configuration provided
        if let Some(config) = installment_config {
            let invoice_id = created_invoice.id.clone().unwrap_or_default();
            let subtotal = created_invoice.subtotal.unwrap_or_default();
            let tax_total = created_invoice.tax_total.unwrap_or_default();
            let service_fee = created_invoice.service_fee.unwrap_or_default();

            // Calculate first installment due date (24 hours from now, matching invoice expiration)
            let start_date = chrono::Utc::now()
                .checked_add_signed(chrono::Duration::hours(24))
                .ok_or_else(|| AppError::Internal("Failed to calculate start date".to_string()))?
                .date_naive();

            // Create installment schedules
            let _schedules = self
                .installment_service
                .create_schedule(
                    invoice_id.clone(),
                    subtotal,
                    tax_total,
                    service_fee,
                    config,
                    currency,
                    start_date,
                )
                .await?;

            info!(
                invoice_id = invoice_id.as_str(),
                "Installment schedules created successfully"
            );
        }

        info!(
            invoice_id = created_invoice.id.as_ref().map(|s| s.as_str()),
            external_id = external_id.as_str(),
            subtotal = %created_invoice.subtotal.unwrap_or_default(),
            tax_total = %created_invoice.tax_total.unwrap_or_default(),
            service_fee = %created_invoice.service_fee.unwrap_or_default(),
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
            warn!(
                invoice_id = id,
                "Payment initiation failed - invoice expired"
            );
            // Mark as expired and return error
            self.update_invoice_status(id, InvoiceStatus::Expired)
                .await?;
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
        self.update_invoice_status(id, InvoiceStatus::Processing)
            .await
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
            .ok_or_else(|| AppError::validation(format!("Gateway '{}' not found", gateway_id)))?;

        if !gateway.supports_currency(currency) {
            return Err(AppError::validation(format!(
                "Gateway '{}' does not support currency {:?}. Supported currencies: {:?}",
                gateway_id, currency, gateway.supported_currencies
            )));
        }

        Ok(())
    }

    /// Update invoice status to partially_paid when first installment is paid (T094, FR-019)
    ///
    /// # Arguments
    /// * `invoice_id` - Invoice ID
    ///
    /// # Returns
    /// * `Result<Invoice>` - Updated invoice
    ///
    /// # Business Rules
    /// - FR-019: Status becomes "partially_paid" when first installment paid
    /// - Can only transition from Pending or Processing status
    pub async fn mark_invoice_partially_paid(&self, invoice_id: &str) -> Result<Invoice> {
        info!(
            invoice_id = invoice_id,
            "Marking invoice as partially paid (first installment received)"
        );

        let invoice = self.get_invoice(invoice_id).await?;

        // Validate current status allows this transition
        if invoice.status != InvoiceStatus::Pending && invoice.status != InvoiceStatus::Processing {
            return Err(AppError::validation(format!(
                "Cannot mark invoice as partially_paid from status '{}'",
                invoice.status
            )));
        }

        // Update status to partially_paid
        self.invoice_repo
            .update_status(invoice_id, InvoiceStatus::PartiallyPaid)
            .await?;

        info!(
            invoice_id = invoice_id,
            "Invoice marked as partially paid successfully"
        );

        self.get_invoice(invoice_id).await
    }

    /// Update invoice status to fully_paid when all installments are paid (T095, FR-020)
    ///
    /// # Arguments
    /// * `invoice_id` - Invoice ID
    ///
    /// # Returns
    /// * `Result<Invoice>` - Updated invoice
    ///
    /// # Business Rules
    /// - FR-020: Status becomes "fully_paid" when all installments complete
    /// - Can only transition from PartiallyPaid status (or Pending/Processing for edge cases)
    pub async fn mark_invoice_fully_paid(&self, invoice_id: &str) -> Result<Invoice> {
        info!(
            invoice_id = invoice_id,
            "Marking invoice as fully paid (all installments received)"
        );

        let invoice = self.get_invoice(invoice_id).await?;

        // Validate current status allows this transition
        match invoice.status {
            InvoiceStatus::PartiallyPaid | InvoiceStatus::Pending | InvoiceStatus::Processing => {
                // Valid transitions
            }
            InvoiceStatus::FullyPaid => {
                // Already fully paid, no-op
                return Ok(invoice);
            }
            _ => {
                return Err(AppError::validation(format!(
                    "Cannot mark invoice as fully_paid from status '{}'",
                    invoice.status
                )));
            }
        }

        // Update status to fully_paid
        self.invoice_repo
            .update_status(invoice_id, InvoiceStatus::FullyPaid)
            .await?;

        info!(
            invoice_id = invoice_id,
            "Invoice marked as fully paid successfully"
        );

        self.get_invoice(invoice_id).await
    }

    /// Create a supplementary invoice for excess overpayment (FR-081, FR-082, T104)
    ///
    /// When overpayment exceeds all installments and leaves excess, a supplementary
    /// invoice can be created to track the additional payment. This allows merchants
    /// to properly account for all received funds.
    ///
    /// # Arguments
    /// * `original_invoice_id` - ID of the original invoice that received overpayment
    /// * `excess_amount` - Excess payment amount to include in supplementary invoice
    /// * `description` - Description for the supplementary invoice line item
    ///
    /// # Returns
    /// * `Result<Invoice>` - Created supplementary invoice
    ///
    /// # Business Rules
    /// - FR-081: Supplementary invoices record excess payments beyond all installments
    /// - FR-082: Links to original invoice via original_invoice_id
    /// - Uses same gateway and currency as original invoice
    /// - No installments on supplementary invoices (single payment already received)
    /// - Automatically marked as "paid" since payment already received
    pub async fn create_supplementary_invoice(
        &self,
        original_invoice_id: &str,
        excess_amount: Decimal,
        description: String,
    ) -> Result<Invoice> {
        info!(
            original_invoice_id = original_invoice_id,
            excess_amount = %excess_amount,
            "Creating supplementary invoice for excess overpayment"
        );

        // Validate excess amount is positive
        if excess_amount <= Decimal::ZERO {
            return Err(AppError::validation(
                "Excess amount must be positive for supplementary invoice",
            ));
        }

        // Get original invoice to copy gateway and currency
        let original_invoice = self.get_invoice(original_invoice_id).await?;

        // Create line item for the excess amount
        let line_item = LineItem::new(
            description,
            1,             // quantity = 1
            excess_amount, // unit_price = excess_amount
            original_invoice.currency,
        )?;

        // Create supplementary invoice
        let external_id = format!(
            "{}-supplementary-{}",
            original_invoice.external_id,
            chrono::Utc::now().timestamp()
        );

        let mut supplementary = Invoice::new(
            external_id,
            original_invoice.gateway_id.clone(),
            original_invoice.currency,
            vec![line_item],
        )?;

        // Link to original invoice (FR-082)
        supplementary.original_invoice_id = Some(original_invoice_id.to_string());

        // Calculate total (no service fee or tax on supplementary invoices)
        supplementary.calculate_subtotal();
        supplementary.tax_total = Some(Decimal::ZERO);
        supplementary.service_fee = Some(Decimal::ZERO);
        supplementary.calculate_total();

        // Create in database
        let created = self.invoice_repo.create(&supplementary).await?;

        // Mark as paid immediately (payment already received)
        self.invoice_repo
            .update_status(created.id.as_ref().unwrap(), InvoiceStatus::Paid)
            .await?;

        info!(
            supplementary_invoice_id = created.id.as_ref().unwrap(),
            original_invoice_id = original_invoice_id,
            amount = %excess_amount,
            "Supplementary invoice created and marked as paid"
        );

        self.get_invoice(created.id.as_ref().unwrap()).await
    }

    /// Calculate service fee for invoice based on gateway configuration (FR-009, FR-047)
    ///
    /// Service fee is calculated as: (subtotal × gateway.fee_percentage) + gateway.fee_fixed
    /// Tax is NOT included in the base for service fee calculation (FR-055)
    ///
    /// # Arguments
    /// * `gateway_id` - Payment gateway ID
    /// * `subtotal` - Invoice subtotal (before taxes and fees)
    ///
    /// # Returns
    /// * `Result<Decimal>` - Calculated service fee
    async fn calculate_service_fee_for_invoice(
        &self,
        gateway_id: &str,
        subtotal: Decimal,
    ) -> Result<Decimal> {
        let gateway = self
            .gateway_repo
            .find_by_id(gateway_id)
            .await?
            .ok_or_else(|| AppError::validation(format!("Gateway '{}' not found", gateway_id)))?;

        // Use gateway's built-in service fee calculation
        let service_fee = gateway.calculate_service_fee(subtotal);

        Ok(service_fee.round_dp(2))
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

    #[test]
    fn test_invoice_status_partially_paid_display() {
        let status = InvoiceStatus::PartiallyPaid;
        assert_eq!(status.to_string(), "partially_paid");
    }

    #[test]
    fn test_invoice_status_fully_paid_display() {
        let status = InvoiceStatus::FullyPaid;
        assert_eq!(status.to_string(), "fully_paid");
    }

    #[test]
    fn test_invoice_status_partially_paid_from_str() {
        use std::str::FromStr;
        let status = InvoiceStatus::from_str("partially_paid").unwrap();
        assert_eq!(status, InvoiceStatus::PartiallyPaid);
    }

    #[test]
    fn test_invoice_status_fully_paid_from_str() {
        use std::str::FromStr;
        let status = InvoiceStatus::from_str("fully_paid").unwrap();
        assert_eq!(status, InvoiceStatus::FullyPaid);
    }
}
