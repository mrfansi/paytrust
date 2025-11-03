use std::sync::Arc;
use crate::core::error::AppError;
use crate::modules::reports::models::FinancialReport;
use crate::modules::reports::repositories::ReportRepository;

/// ReportService handles financial report generation (FR-012, FR-013, FR-063, FR-064)
pub struct ReportService {
    report_repo: Arc<dyn ReportRepository>,
}

impl ReportService {
    pub fn new(report_repo: Arc<dyn ReportRepository>) -> Self {
        Self { report_repo }
    }

    /// Generate financial report with service fee and tax breakdown
    /// FR-012: Service fee breakdown by gateway
    /// FR-013: Tax breakdown by rate
    /// FR-063: Separate totals by currency (no conversion)
    pub async fn generate_financial_report(
        &self,
        start_date: chrono::NaiveDateTime,
        end_date: chrono::NaiveDateTime,
    ) -> Result<FinancialReport, AppError> {
        // Fetch all report data in parallel
        let (service_fee_breakdown, tax_breakdown, total_revenue) = tokio::try_join!(
            self.report_repo.get_service_fee_breakdown(start_date, end_date),
            self.report_repo.get_tax_breakdown(start_date, end_date),
            self.report_repo.get_revenue_by_currency(start_date, end_date),
        )?;

        Ok(FinancialReport {
            service_fee_breakdown,
            tax_breakdown,
            total_revenue,
        })
    }
}
