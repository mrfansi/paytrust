use std::sync::Arc;

use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::core::error::AppError;
use crate::middleware::auth::TenantId;
use crate::modules::invoices::models::CreateInvoiceRequest;
use crate::modules::invoices::services::invoice_service::InvoiceService;

/// Query parameters for listing invoices
#[derive(Debug, Deserialize)]
pub struct ListInvoicesQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

/// Create a new invoice
/// POST /invoices
/// Implements FR-001, FR-004, FR-007, FR-044, FR-046, FR-051
pub async fn create_invoice(
    service: web::Data<Arc<InvoiceService>>,
    tenant_id: TenantId,
    request: web::Json<CreateInvoiceRequest>,
) -> Result<HttpResponse, AppError> {
    let invoice = service
        .create_invoice(request.into_inner(), &tenant_id.0)
        .await?;

    Ok(HttpResponse::Created().json(invoice))
}

/// Get invoice by ID
/// GET /invoices/{id}
/// Implements FR-002
pub async fn get_invoice(
    service: web::Data<Arc<InvoiceService>>,
    tenant_id: TenantId,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let invoice_id = path.into_inner();
    let invoice = service.get_invoice(invoice_id, &tenant_id.0).await?;

    Ok(HttpResponse::Ok().json(invoice))
}

/// List invoices for tenant
/// GET /invoices
/// Implements FR-003
pub async fn list_invoices(
    service: web::Data<Arc<InvoiceService>>,
    tenant_id: TenantId,
    query: web::Query<ListInvoicesQuery>,
) -> Result<HttpResponse, AppError> {
    let invoices = service
        .list_invoices(&tenant_id.0, query.limit, query.offset)
        .await?;

    Ok(HttpResponse::Ok().json(invoices))
}

/// Initiate payment for an invoice
/// POST /invoices/{id}/initiate-payment
/// Implements FR-051a - Sets payment_initiated_at timestamp
pub async fn initiate_payment(
    service: web::Data<Arc<InvoiceService>>,
    tenant_id: TenantId,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let invoice_id = path.into_inner();
    
    // Set payment initiated timestamp
    service
        .set_payment_initiated(invoice_id, &tenant_id.0)
        .await?;

    // Get updated invoice
    let invoice = service.get_invoice(invoice_id, &tenant_id.0).await?;

    Ok(HttpResponse::Ok().json(invoice))
}

/// Configure invoice routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/invoices")
            .route("", web::post().to(create_invoice))
            .route("", web::get().to(list_invoices))
            .route("/{id}", web::get().to(get_invoice))
            .route("/{id}/initiate-payment", web::post().to(initiate_payment)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limit() {
        assert_eq!(default_limit(), 50);
    }

    #[test]
    fn test_list_query_defaults() {
        let query: ListInvoicesQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.limit, 50);
        assert_eq!(query.offset, 0);
    }
}
