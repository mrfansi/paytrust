use std::sync::Arc;
use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use crate::modules::reports::services::ReportService;

pub struct ReportController {
    report_service: Arc<ReportService>,
}

impl ReportController {
    pub fn new(report_service: Arc<ReportService>) -> Self {
        Self { report_service }
    }
}

#[derive(Debug, Deserialize)]
pub struct FinancialReportQuery {
    pub start_date: String,  // ISO 8601 format
    pub end_date: String,    // ISO 8601 format
}

/// GET /reports/financial - Generate financial report
/// Query params: start_date, end_date (ISO 8601 format)
pub async fn get_financial_report(
    query: web::Query<FinancialReportQuery>,
    report_service: web::Data<Arc<ReportService>>,
) -> impl Responder {
    // Parse dates
    let start_date = match chrono::NaiveDateTime::parse_from_str(&query.start_date, "%Y-%m-%d %H:%M:%S") {
        Ok(date) => date,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid start_date format. Expected: YYYY-MM-DD HH:MM:SS"
            }));
        }
    };

    let end_date = match chrono::NaiveDateTime::parse_from_str(&query.end_date, "%Y-%m-%d %H:%M:%S") {
        Ok(date) => date,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid end_date format. Expected: YYYY-MM-DD HH:MM:SS"
            }));
        }
    };

    // Validate date range
    if start_date > end_date {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "start_date must be before or equal to end_date"
        }));
    }

    // Generate report
    match report_service.generate_financial_report(start_date, end_date).await {
        Ok(report) => HttpResponse::Ok().json(report),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to generate report: {}", e)
        })),
    }
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/reports")
            .route("/financial", web::get().to(get_financial_report))
    );
}
