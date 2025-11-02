use std::sync::Arc;

use actix_web::{web, HttpResponse};

use crate::core::error::AppError;
use crate::modules::gateways::services::gateway_service::GatewayService;

/// List all available payment gateways
/// GET /gateways
/// Returns list of gateways with their supported currencies
pub async fn list_gateways(
    service: web::Data<Arc<GatewayService>>,
) -> Result<HttpResponse, AppError> {
    let gateways = service.list_gateways();
    Ok(HttpResponse::Ok().json(gateways))
}

/// Configure gateway routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/gateways")
            .route("", web::get().to(list_gateways)),
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
