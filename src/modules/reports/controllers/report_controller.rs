use actix_web::{web, HttpResponse};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use tracing::error;

use crate::core::Result;
use crate::modules::reports::models::{FinancialReport, ServiceFeeBreakdown, TaxBreakdown};
use crate::modules::reports::repositories::ReportRepository;
use crate::modules::reports::services::ReportService;

/// Query parameters for financial report endpoint
#[derive(Debug, Deserialize)]
pub struct FinancialReportQuery {
    /// Start date of reporting period (inclusive, format: YYYY-MM-DD)
    pub start_date: String,
    /// End date of reporting period (inclusive, format: YYYY-MM-DD)
    pub end_date: String,
    /// Optional currency filter (e.g., "IDR", "USD")
    #[serde(default)]
    pub currency: Option<String>,
}

/// Response structure for financial report (matches OpenAPI spec)
#[derive(Debug, Serialize)]
pub struct FinancialReportResponse {
    pub start_date: String, // Format: YYYY-MM-DD
    pub end_date: String,   // Format: YYYY-MM-DD
    pub service_fees: Vec<ServiceFeeBreakdownResponse>,
    pub taxes: Vec<TaxBreakdownResponse>,
}

/// Service fee breakdown response structure
#[derive(Debug, Serialize)]
pub struct ServiceFeeBreakdownResponse {
    pub currency: String,
    pub gateway: String,
    pub total_amount: String, // Decimal as string for JSON precision
    pub transaction_count: i64,
}

/// Tax breakdown response structure
#[derive(Debug, Serialize)]
pub struct TaxBreakdownResponse {
    pub currency: String,
    pub tax_rate: String, // Decimal as string for JSON precision
    pub total_amount: String,
    pub transaction_count: i64,
}

impl From<FinancialReport> for FinancialReportResponse {
    fn from(report: FinancialReport) -> Self {
        Self {
            start_date: report.start_date.format("%Y-%m-%d").to_string(),
            end_date: report.end_date.format("%Y-%m-%d").to_string(),
            service_fees: report
                .service_fees
                .into_iter()
                .map(ServiceFeeBreakdownResponse::from)
                .collect(),
            taxes: report
                .taxes
                .into_iter()
                .map(TaxBreakdownResponse::from)
                .collect(),
        }
    }
}

impl From<ServiceFeeBreakdown> for ServiceFeeBreakdownResponse {
    fn from(breakdown: ServiceFeeBreakdown) -> Self {
        Self {
            currency: breakdown.currency,
            gateway: breakdown.gateway,
            total_amount: breakdown.total_amount.to_string(),
            transaction_count: breakdown.transaction_count,
        }
    }
}

impl From<TaxBreakdown> for TaxBreakdownResponse {
    fn from(breakdown: TaxBreakdown) -> Self {
        Self {
            currency: breakdown.currency,
            tax_rate: breakdown.tax_rate.to_string(),
            total_amount: breakdown.total_amount.to_string(),
            transaction_count: breakdown.transaction_count,
        }
    }
}

/// GET /reports/financial
/// 
/// Returns aggregated financial data including service fees and taxes.
/// Implements FR-012 (financial reporting), FR-013 (date range filtering),
/// FR-063 (breakdowns), FR-064 (currency separation).
pub async fn get_financial_report(
    pool: web::Data<MySqlPool>,
    query: web::Query<FinancialReportQuery>,
) -> HttpResponse {
    match handle_get_financial_report(pool, query).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("Failed to generate financial report: {}", e);
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": {
                    "code": "VALIDATION_ERROR",
                    "message": e.to_string()
                }
            }))
        }
    }
}

async fn handle_get_financial_report(
    pool: web::Data<MySqlPool>,
    query: web::Query<FinancialReportQuery>,
) -> Result<FinancialReportResponse> {
    // Parse and validate dates (FR-013)
    let start_date = NaiveDate::parse_from_str(&query.start_date, "%Y-%m-%d")
        .map_err(|_| crate::core::AppError::validation(
            format!("Invalid start_date format: '{}'. Expected YYYY-MM-DD", query.start_date)
        ))?;

    let end_date = NaiveDate::parse_from_str(&query.end_date, "%Y-%m-%d")
        .map_err(|_| crate::core::AppError::validation(
            format!("Invalid end_date format: '{}'. Expected YYYY-MM-DD", query.end_date)
        ))?;

    // Create service and validate date range
    let report_repo = ReportRepository::new(pool.get_ref().clone());
    let report_service = ReportService::new(report_repo);

    report_service.validate_date_range(start_date, end_date)?;

    // Generate report (FR-012, FR-063, FR-064)
    let currency_filter = query.currency.as_deref();
    let report = report_service
        .generate_financial_report(start_date, end_date, currency_filter)
        .await?;

    Ok(FinancialReportResponse::from(report))
}

/// Configure routes for reports module
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/reports")
            .route("/financial", web::get().to(get_financial_report))
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_financial_report_response_serialization() {
        let breakdown = ServiceFeeBreakdownResponse {
            currency: "IDR".to_string(),
            gateway: "xendit".to_string(),
            total_amount: "100000.00".to_string(),
            transaction_count: 5,
        };

        let json = serde_json::to_string(&breakdown).unwrap();
        assert!(json.contains("\"currency\":\"IDR\""));
        assert!(json.contains("\"gateway\":\"xendit\""));
        assert!(json.contains("\"total_amount\":\"100000.00\""));
        assert!(json.contains("\"transaction_count\":5"));
    }

    #[test]
    fn test_tax_breakdown_response_serialization() {
        let breakdown = TaxBreakdownResponse {
            currency: "IDR".to_string(),
            tax_rate: "0.11".to_string(),
            total_amount: "55000.00".to_string(),
            transaction_count: 10,
        };

        let json = serde_json::to_string(&breakdown).unwrap();
        assert!(json.contains("\"currency\":\"IDR\""));
        assert!(json.contains("\"tax_rate\":\"0.11\""));
        assert!(json.contains("\"total_amount\":\"55000.00\""));
        assert!(json.contains("\"transaction_count\":10"));
    }

    #[test]
    fn test_financial_report_response_from_model() {
        use rust_decimal_macros::dec;

        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();

        let service_fees = vec![
            ServiceFeeBreakdown::new("xendit".to_string(), "IDR".to_string(), dec!(100000), 5),
        ];
        let taxes = vec![
            TaxBreakdown::new(dec!(0.11), "IDR".to_string(), dec!(55000), 5),
        ];

        let report = FinancialReport::new(start, end, service_fees, taxes);
        let response = FinancialReportResponse::from(report);

        assert_eq!(response.start_date, "2025-01-01");
        assert_eq!(response.end_date, "2025-01-31");
        assert_eq!(response.service_fees.len(), 1);
        assert_eq!(response.taxes.len(), 1);
        assert_eq!(response.service_fees[0].gateway, "xendit");
        assert_eq!(response.taxes[0].tax_rate, "0.11");
    }
}
