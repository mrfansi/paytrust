use crate::core::Result;
use crate::modules::gateways::services::{GatewayInfo, GatewayService};
use actix_web::{get, web, HttpResponse};
use std::sync::Arc;

/// Gateway controller for managing gateway endpoints
///
/// Provides REST API for querying available payment gateways
/// and their supported currencies

/// GET /v1/gateways - List all available payment gateways
///
/// Returns a list of all configured payment gateways with their
/// supported currencies
///
/// # Response
/// ```json
/// [
///   {
///     "id": "xendit",
///     "name": "xendit",
///     "supported_currencies": ["IDR", "MYR"]
///   },
///   {
///     "id": "midtrans",
///     "name": "midtrans",
///     "supported_currencies": ["IDR"]
///   }
/// ]
/// ```
#[get("/gateways")]
async fn list_gateways(service: web::Data<Arc<GatewayService>>) -> Result<HttpResponse> {
    let gateway_ids = service.list_gateways();
    
    let mut gateways: Vec<GatewayInfo> = Vec::new();
    for gateway_id in gateway_ids {
        if let Ok(info) = service.get_gateway_info(&gateway_id) {
            gateways.push(info);
        }
    }

    Ok(HttpResponse::Ok().json(gateways))
}

/// GET /v1/gateways/{id} - Get information about a specific gateway
///
/// Returns detailed information about a gateway including
/// supported currencies
///
/// # Parameters
/// * `id` - Gateway identifier (e.g., "xendit", "midtrans")
///
/// # Response
/// ```json
/// {
///   "id": "xendit",
///   "name": "xendit",
///   "supported_currencies": ["IDR", "MYR"]
/// }
/// ```
#[get("/gateways/{id}")]
async fn get_gateway(
    service: web::Data<Arc<GatewayService>>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let gateway_id = path.into_inner();
    let info = service.get_gateway_info(&gateway_id)?;
    Ok(HttpResponse::Ok().json(info))
}

/// Configure gateway routes
///
/// Mounts all gateway endpoints under /v1
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1")
            .service(list_gateways)
            .service(get_gateway),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::gateways::{MidtransClient, XenditClient};
    use crate::modules::gateways::repositories::GatewayRepository;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_list_gateways() {
        // Create test service
        let pool = sqlx::MySqlPool::connect_lazy("mysql://test:test@localhost/test").unwrap();
        let repository = GatewayRepository::new(pool);
        
        let xendit = XenditClient::new("test_key".to_string(), "test_secret".to_string(), None);
        let midtrans = MidtransClient::new("test_key".to_string(), "test_secret".to_string(), None);
        let service = Arc::new(GatewayService::new(repository, xendit, midtrans));

        // Create test app
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(service.clone()))
                .configure(configure)
        ).await;

        // Test request
        let req = test::TestRequest::get()
            .uri("/v1/gateways")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_get_gateway() {
        // Create test service
        let pool = sqlx::MySqlPool::connect_lazy("mysql://test:test@localhost/test").unwrap();
        let repository = GatewayRepository::new(pool);
        
        let xendit = XenditClient::new("test_key".to_string(), "test_secret".to_string(), None);
        let midtrans = MidtransClient::new("test_key".to_string(), "test_secret".to_string(), None);
        let service = Arc::new(GatewayService::new(repository, xendit, midtrans));

        // Create test app
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(service.clone()))
                .configure(configure)
        ).await;

        // Test valid gateway
        let req = test::TestRequest::get()
            .uri("/v1/gateways/xendit")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        // Test invalid gateway
        let req = test::TestRequest::get()
            .uri("/v1/gateways/invalid")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_client_error());
    }
}
