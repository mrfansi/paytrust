//! Tax controller for HTTP endpoints
//!
//! Handles tax configuration endpoints (if needed for future admin API).
//! Currently minimal as tax rates are typically managed through direct database access.

use actix_web::{web, HttpResponse};
use sqlx::MySqlPool;

use crate::modules::taxes::repositories::TaxRepository;

/// List active tax rates
///
/// GET /taxes/active
pub async fn list_active_taxes(pool: web::Data<MySqlPool>) -> HttpResponse {
    let repository = TaxRepository::new(pool.get_ref().clone());

    match repository.list_active().await {
        Ok(taxes) => HttpResponse::Ok().json(serde_json::json!({
            "taxes": taxes,
        })),
        Err(err) => {
            tracing::error!("Failed to list active taxes: {}", err);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": {
                    "code": "DATABASE_ERROR",
                    "message": "Failed to retrieve tax rates"
                }
            }))
        }
    }
}

/// Get tax by ID
///
/// GET /taxes/{id}
pub async fn get_tax_by_id(
    pool: web::Data<MySqlPool>,
    tax_id: web::Path<String>,
) -> HttpResponse {
    let repository = TaxRepository::new(pool.get_ref().clone());

    match repository.find_by_id(&tax_id).await {
        Ok(Some(tax)) => HttpResponse::Ok().json(tax),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": {
                "code": "TAX_NOT_FOUND",
                "message": "Tax rate not found"
            }
        })),
        Err(err) => {
            tracing::error!("Failed to find tax: {}", err);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": {
                    "code": "DATABASE_ERROR",
                    "message": "Failed to retrieve tax rate"
                }
            }))
        }
    }
}

/// Configure tax routes
pub fn configure_tax_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/taxes")
            .route("/active", web::get().to(list_active_taxes))
            .route("/{id}", web::get().to(get_tax_by_id)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configure_tax_routes() {
        // This is a placeholder test to ensure the module compiles
        // Full integration tests would require setting up actix-web test server
        assert!(true);
    }
}
