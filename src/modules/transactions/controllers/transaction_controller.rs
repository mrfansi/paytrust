use std::sync::Arc;

use actix_web::{web, HttpResponse};

use crate::core::error::AppError;
use crate::middleware::auth::TenantId;
use crate::modules::transactions::services::transaction_service::TransactionService;

/// List transactions for an invoice
/// GET /invoices/{id}/transactions
/// Implements transaction history query
pub async fn list_transactions(
    service: web::Data<Arc<TransactionService>>,
    tenant_id: TenantId,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let invoice_id = path.into_inner();
    
    let transactions = service
        .list_transactions_for_invoice(invoice_id, &tenant_id.0)
        .await?;

    Ok(HttpResponse::Ok().json(transactions))
}

/// Configure transaction routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/invoices")
            .route("/{id}/transactions", web::get().to(list_transactions)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_compiles() {
        // This test ensures the controller compiles
        // Actual HTTP tests are in integration tests
    }
}
