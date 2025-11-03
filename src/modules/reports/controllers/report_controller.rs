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
pub async fn get_financial_report(
    _query: web::Query<FinancialReportQuery>,
    _report_service: web::Data<Arc<ReportService>>,
) -> impl Responder {
    // TODO: Implement financial report endpoint
    // This is a stub that will make tests fail
    HttpResponse::NotImplemented().json(serde_json::json!({
        "error": "Not implemented yet"
    }))
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/reports")
            .route("/financial", web::get().to(get_financial_report))
    );
}
