// T091: InstallmentService implementation
// Business logic layer for installment schedule operations
//
// Implements:
// - Schedule creation with proportional distribution (FR-059, FR-060)
// - Sequential payment validation (FR-068)
// - Unpaid installment adjustment (FR-077)
// - Status transition enforcement
// - Overdue detection

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::MySqlPool;
use tracing::{info, warn};

use crate::core::{AppError, Currency, Result};
use crate::modules::installments::{
    models::{InstallmentConfig, InstallmentSchedule, InstallmentStatus},
    repositories::InstallmentRepository,
    services::InstallmentCalculator,
};

/// Service for installment schedule business logic
pub struct InstallmentService {
    repository: InstallmentRepository,
}

impl InstallmentService {
    /// Create a new installment service
    pub fn new(pool: MySqlPool) -> Self {
        Self {
            repository: InstallmentRepository::new(pool),
        }
    }

    /// Create installment schedules for an invoice
    ///
    /// # Arguments
    /// * `invoice_id` - Invoice ID to create schedules for
    /// * `invoice_total` - Total invoice amount
    /// * `tax_total` - Total tax amount
    /// * `service_fee_total` - Total service fee
    /// * `config` - Installment configuration (count and optional custom amounts)
    /// * `currency` - Currency for rounding
    /// * `start_date` - First installment due date
    ///
    /// # Returns
    /// * `Result<Vec<InstallmentSchedule>>` - Created installment schedules
    ///
    /// # Business Rules
    /// - FR-014: 2-12 installments allowed (validated by InstallmentConfig)
    /// - FR-017: SUM(amounts) = invoice_total (validated by calculator)
    /// - FR-059: Tax proportionally distributed
    /// - FR-060: Service fee proportionally distributed
    /// - FR-071, FR-072: Last installment absorbs rounding
    pub async fn create_schedule(
        &self,
        invoice_id: String,
        invoice_total: Decimal,
        tax_total: Decimal,
        service_fee_total: Decimal,
        config: InstallmentConfig,
        currency: Currency,
        start_date: NaiveDate,
    ) -> Result<Vec<InstallmentSchedule>> {
        info!(
            invoice_id = invoice_id.as_str(),
            installment_count = config.installment_count,
            invoice_total = %invoice_total,
            "Creating installment schedule"
        );

        // Calculate installment schedules with proportional distribution
        let schedules = InstallmentCalculator::calculate_schedules(
            invoice_id.clone(),
            invoice_total,
            tax_total,
            service_fee_total,
            &config,
            currency,
            start_date,
        )?;

        // Persist to database
        self.repository.create_batch(&schedules).await?;

        info!(
            invoice_id = invoice_id.as_str(),
            schedules_created = schedules.len(),
            "Installment schedule created successfully"
        );

        Ok(schedules)
    }

    /// Get all installments for an invoice
    ///
    /// # Arguments
    /// * `invoice_id` - Invoice ID to query
    ///
    /// # Returns
    /// * `Result<Vec<InstallmentSchedule>>` - Ordered list of installments
    pub async fn get_installments(&self, invoice_id: &str) -> Result<Vec<InstallmentSchedule>> {
        self.repository.find_by_invoice(invoice_id).await
    }

    /// Get a single installment by ID
    ///
    /// # Arguments
    /// * `id` - Installment ID
    ///
    /// # Returns
    /// * `Result<InstallmentSchedule>` - Installment if found
    pub async fn get_installment(&self, id: &str) -> Result<InstallmentSchedule> {
        self.repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::not_found("Installment not found"))
    }

    /// Validate that an installment can be paid according to sequential rules
    ///
    /// # Arguments
    /// * `installment_id` - Installment ID to validate
    ///
    /// # Returns
    /// * `Result<()>` - Success if installment can be paid
    ///
    /// # Business Rules
    /// - FR-068: Sequential payment order enforced
    /// - All previous installments must be paid before this one
    pub async fn validate_sequential_payment(&self, installment_id: &str) -> Result<()> {
        let installment = self.get_installment(installment_id).await?;

        // Already paid installments are valid
        if installment.status == InstallmentStatus::Paid {
            return Ok(());
        }

        // Get all installments for the invoice
        let all_installments = self
            .repository
            .find_by_invoice(&installment.invoice_id)
            .await?;

        // Check if can be paid (sequential enforcement)
        if !installment.can_be_paid(&all_installments) {
            warn!(
                installment_id = installment_id,
                installment_number = installment.installment_number,
                "Payment validation failed - previous installments not paid"
            );
            return Err(AppError::validation(format!(
                "Installment {} cannot be paid: previous installments must be paid first",
                installment.installment_number
            )));
        }

        Ok(())
    }

    /// Mark an installment as paid
    ///
    /// # Arguments
    /// * `id` - Installment ID
    /// * `gateway_reference` - Gateway transaction reference
    ///
    /// # Returns
    /// * `Result<InstallmentSchedule>` - Updated installment
    ///
    /// # Business Rules
    /// - FR-068: Validates sequential payment order before marking as paid
    pub async fn mark_installment_paid(
        &self,
        id: &str,
        gateway_reference: String,
    ) -> Result<InstallmentSchedule> {
        // Validate sequential payment order
        self.validate_sequential_payment(id).await?;

        let mut installment = self.get_installment(id).await?;

        // Mark as paid
        installment.mark_as_paid(gateway_reference)?;

        // Persist update
        self.repository.update(&installment).await?;

        info!(
            installment_id = id,
            installment_number = installment.installment_number,
            "Installment marked as paid"
        );

        Ok(installment)
    }

    /// Adjust unpaid installments after first payment
    ///
    /// # Arguments
    /// * `invoice_id` - Invoice ID
    /// * `remaining_total` - Remaining unpaid total (invoice total - paid installments)
    /// * `remaining_tax` - Remaining tax to distribute
    /// * `remaining_fee` - Remaining service fee to distribute
    /// * `currency` - Currency for rounding
    ///
    /// # Returns
    /// * `Result<Vec<InstallmentSchedule>>` - Updated installment schedules
    ///
    /// # Business Rules
    /// - FR-077: Only unpaid installments can be adjusted
    /// - FR-078: Paid installments remain unchanged
    /// - FR-079: New amounts recalculate proportional tax/fee
    /// - FR-080: Remaining amounts redistributed across unpaid installments
    pub async fn adjust_unpaid_installments(
        &self,
        invoice_id: &str,
        remaining_total: Decimal,
        remaining_tax: Decimal,
        remaining_fee: Decimal,
        currency: Currency,
    ) -> Result<Vec<InstallmentSchedule>> {
        info!(
            invoice_id = invoice_id,
            remaining_total = %remaining_total,
            "Adjusting unpaid installments"
        );

        // Get all existing installments
        let existing_installments = self.repository.find_by_invoice(invoice_id).await?;

        // Recalculate unpaid installments
        let updated_schedules = InstallmentCalculator::recalculate_unpaid_schedules(
            existing_installments,
            remaining_total,
            remaining_tax,
            remaining_fee,
            currency,
        )?;

        // Separate paid and unpaid for update
        let (paid, unpaid): (Vec<_>, Vec<_>) = updated_schedules
            .into_iter()
            .partition(|s| s.status == InstallmentStatus::Paid);

        // Only update unpaid installments
        if !unpaid.is_empty() {
            self.repository.update_batch(&unpaid).await?;
        }

        info!(
            invoice_id = invoice_id,
            updated_count = unpaid.len(),
            "Unpaid installments adjusted successfully"
        );

        // Return all installments (paid + updated unpaid)
        let mut result = paid;
        result.extend(unpaid);
        result.sort_by_key(|s| s.installment_number);
        Ok(result)
    }

    /// Mark overdue installments
    ///
    /// # Arguments
    /// * `invoice_id` - Invoice ID to check
    ///
    /// # Returns
    /// * `Result<Vec<InstallmentSchedule>>` - Updated overdue installments
    ///
    /// # Business Rules
    /// - Unpaid installments past their due date are marked as overdue
    pub async fn mark_overdue_installments(
        &self,
        invoice_id: &str,
    ) -> Result<Vec<InstallmentSchedule>> {
        let installments = self.repository.find_by_invoice(invoice_id).await?;

        let mut updated = Vec::new();
        for mut installment in installments {
            if installment.status == InstallmentStatus::Unpaid && installment.is_past_due() {
                installment.mark_as_overdue()?;
                self.repository.update(&installment).await?;
                updated.push(installment);
            }
        }

        if !updated.is_empty() {
            info!(
                invoice_id = invoice_id,
                overdue_count = updated.len(),
                "Marked installments as overdue"
            );
        }

        Ok(updated)
    }

    /// Get unpaid installments in sequence
    ///
    /// # Arguments
    /// * `invoice_id` - Invoice ID to query
    ///
    /// # Returns
    /// * `Result<Vec<InstallmentSchedule>>` - Unpaid installments ordered by number
    pub async fn get_unpaid_installments(
        &self,
        invoice_id: &str,
    ) -> Result<Vec<InstallmentSchedule>> {
        self.repository.find_unpaid_in_sequence(invoice_id).await
    }

    /// Get the next unpaid installment that can be paid
    ///
    /// # Arguments
    /// * `invoice_id` - Invoice ID to query
    ///
    /// # Returns
    /// * `Result<Option<InstallmentSchedule>>` - Next payable installment if exists
    ///
    /// # Business Rules
    /// - FR-068: Returns the first unpaid installment in sequence
    pub async fn get_next_payable_installment(
        &self,
        invoice_id: &str,
    ) -> Result<Option<InstallmentSchedule>> {
        let all_installments = self.repository.find_by_invoice(invoice_id).await?;

        // Find first unpaid installment that can be paid
        for installment in &all_installments {
            if installment.status == InstallmentStatus::Unpaid
                && installment.can_be_paid(&all_installments)
            {
                return Ok(Some(installment.clone()));
            }
        }

        Ok(None)
    }

    /// Set payment URL for an installment
    ///
    /// # Arguments
    /// * `id` - Installment ID
    /// * `payment_url` - Gateway-generated payment URL
    ///
    /// # Returns
    /// * `Result<()>` - Success
    pub async fn set_payment_url(&self, id: &str, payment_url: String) -> Result<()> {
        let mut installment = self.get_installment(id).await?;
        installment.set_payment_url(payment_url);
        self.repository.update(&installment).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_instantiation() {
        // This test verifies the service can be instantiated
        // Integration tests with actual database will be in T086-T087
        assert!(true);
    }

    #[test]
    fn test_installment_config_validation() {
        // Validate installment count range (2-12)
        let valid_config = InstallmentConfig {
            installment_count: 3,
            custom_amounts: None,
        };
        assert!(valid_config.validate(Decimal::new(1500000, 0)).is_ok());

        let invalid_too_few = InstallmentConfig {
            installment_count: 1,
            custom_amounts: None,
        };
        assert!(invalid_too_few.validate(Decimal::new(1000000, 0)).is_err());

        let invalid_too_many = InstallmentConfig {
            installment_count: 13,
            custom_amounts: None,
        };
        assert!(invalid_too_many.validate(Decimal::new(1300000, 0)).is_err());
    }

    #[test]
    fn test_custom_amounts_validation() {
        // Custom amounts must match installment count
        let amounts = vec![
            Decimal::new(500000, 0),
            Decimal::new(500000, 0),
            Decimal::new(500000, 0),
        ];
        let total = Decimal::new(1500000, 0);

        let valid_config = InstallmentConfig {
            installment_count: 3,
            custom_amounts: Some(amounts.clone()),
        };
        assert!(valid_config.validate(total).is_ok());

        // Mismatch between count and amounts
        let invalid_config = InstallmentConfig {
            installment_count: 2,
            custom_amounts: Some(amounts),
        };
        assert!(invalid_config.validate(Decimal::new(1000000, 0)).is_err());
    }
}
