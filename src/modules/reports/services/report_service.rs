use chrono::NaiveDate;
use tracing::{info, warn};

use crate::core::Result;
use crate::modules::reports::models::FinancialReport;
use crate::modules::reports::repositories::ReportRepository;

/// Service for generating financial reports
/// Implements FR-012 (financial reporting), FR-013 (date range filtering), 
/// FR-063 (breakdowns), FR-064 (currency separation)
pub struct ReportService {
    report_repo: ReportRepository,
}

impl ReportService {
    /// Create a new report service
    pub fn new(report_repo: ReportRepository) -> Self {
        Self { report_repo }
    }

    /// Generate financial report for the specified date range
    /// 
    /// Aggregates service fees and taxes from invoices within the date range.
    /// Returns breakdowns by gateway/currency for fees and by rate/currency for taxes.
    /// 
    /// # Arguments
    /// * `start_date` - Start of reporting period (inclusive)
    /// * `end_date` - End of reporting period (inclusive)
    /// * `currency_filter` - Optional currency filter (e.g., Some("IDR"))
    /// 
    /// # Returns
    /// FinancialReport containing service fee and tax breakdowns
    /// 
    /// # Errors
    /// Returns error if database queries fail or if start_date > end_date
    pub async fn generate_financial_report(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        currency_filter: Option<&str>,
    ) -> Result<FinancialReport> {
        // Validate date range (FR-013)
        if start_date > end_date {
            return Err(crate::core::AppError::validation(
                format!("start_date ({}) must be before or equal to end_date ({})", start_date, end_date)
            ));
        }

        info!(
            "Generating financial report: start={}, end={}, currency={:?}",
            start_date, end_date, currency_filter
        );

        // Fetch service fee breakdown (FR-063, FR-064)
        let service_fees = self.report_repo
            .get_service_fee_breakdown(start_date, end_date, currency_filter)
            .await?;

        // Fetch tax breakdown (FR-063, FR-064)
        let taxes = self.report_repo
            .get_tax_breakdown(start_date, end_date, currency_filter)
            .await?;

        let report = FinancialReport::new(start_date, end_date, service_fees, taxes);

        if report.is_empty() {
            warn!(
                "Empty financial report generated for period {} to {}",
                start_date, end_date
            );
        } else {
            info!(
                "Financial report generated: {} service fee entries, {} tax entries",
                report.service_fees.len(),
                report.taxes.len()
            );
        }

        Ok(report)
    }

    /// Validate that a date range is reasonable
    /// 
    /// Ensures that the date range is not in the future and not excessively long.
    /// This is a helper method for API validation.
    /// 
    /// # Arguments
    /// * `start_date` - Start of reporting period
    /// * `end_date` - End of reporting period
    /// 
    /// # Returns
    /// Ok(()) if valid, Err if invalid
    pub fn validate_date_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<()> {
        let today = chrono::Utc::now().date_naive();

        // Check that start_date is not after end_date
        if start_date > end_date {
            return Err(crate::core::AppError::validation(
                "start_date must be before or equal to end_date"
            ));
        }

        // Check that end_date is not in the future
        if end_date > today {
            return Err(crate::core::AppError::validation(
                format!("end_date cannot be in the future (today is {})", today)
            ));
        }

        // Check that date range is not excessively long (e.g., max 1 year)
        let days_diff = (end_date - start_date).num_days();
        if days_diff > 365 {
            return Err(crate::core::AppError::validation(
                format!("Date range too large: {} days (maximum 365 days)", days_diff)
            ));
        }

        Ok(())
    }
}

// Unit tests for this service require database access
// See integration tests in tests/integration/report_generation_test.rs
