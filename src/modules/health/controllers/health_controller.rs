use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::MySqlPool;

/// Health check response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub checks: HealthChecks,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthChecks {
    pub database: String,
    pub application: String,
}

/// Readiness probe response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadinessResponse {
    pub ready: bool,
    pub checks: ReadinessChecks,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadinessChecks {
    pub database: bool,
    pub application: bool,
}

/// GET /health - Liveness probe
/// Returns 200 if the application is alive (can respond to requests)
/// Does not check dependencies
pub async fn health_check() -> impl Responder {
    let response = HealthResponse {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        checks: HealthChecks {
            database: "not_checked".to_string(),
            application: "healthy".to_string(),
        },
    };

    HttpResponse::Ok().json(response)
}

/// GET /ready - Readiness probe
/// Returns 200 if the application is ready to serve traffic
/// Checks database connectivity and other dependencies
pub async fn readiness_check(pool: web::Data<MySqlPool>) -> impl Responder {
    let mut ready = true;
    let mut checks = ReadinessChecks {
        database: false,
        application: true,
    };

    // Check database connectivity
    match sqlx::query("SELECT 1").fetch_one(pool.get_ref()).await {
        Ok(_) => {
            checks.database = true;
        }
        Err(e) => {
            ready = false;
            tracing::error!("Database readiness check failed: {}", e);
        }
    }

    let response = ReadinessResponse { ready, checks };

    if ready {
        HttpResponse::Ok().json(response)
    } else {
        HttpResponse::ServiceUnavailable().json(response)
    }
}

/// GET /api/docs - API documentation UI (Swagger UI)
/// Returns an HTML page with interactive API documentation
pub async fn api_docs_ui() -> impl Responder {
    let html = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>PayTrust API Documentation</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui.css" />
    <style>
        body { margin: 0; padding: 0; }
        .topbar { display: none !important; }
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-standalone-preset.js"></script>
    <script>
        window.onload = function() {
            window.ui = SwaggerUIBundle({
                url: '/api/docs/openapi.json',
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                plugins: [
                    SwaggerUIBundle.plugins.DownloadUrl
                ],
                layout: "StandaloneLayout",
                docExpansion: "list",
                defaultModelsExpandDepth: 1,
                defaultModelExpandDepth: 1
            });
        };
    </script>
</body>
</html>
    "#;
    
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

/// GET /api/docs/openapi.yaml - OpenAPI specification in YAML format
/// Returns the raw OpenAPI specification file
pub async fn openapi_spec_yaml() -> impl Responder {
    let openapi_yaml = include_str!("../../../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    
    HttpResponse::Ok()
        .content_type("application/yaml; charset=utf-8")
        .body(openapi_yaml)
}

/// GET /api/docs/openapi.json - OpenAPI specification in JSON format
/// Returns the OpenAPI specification converted to JSON
pub async fn openapi_spec_json() -> impl Responder {
    let openapi_yaml = include_str!("../../../../specs/001-payment-orchestration-api/contracts/openapi.yaml");
    
    match serde_yaml::from_str::<serde_json::Value>(openapi_yaml) {
        Ok(json_value) => HttpResponse::Ok()
            .content_type("application/json; charset=utf-8")
            .json(json_value),
        Err(e) => {
            tracing::error!("Failed to parse OpenAPI YAML: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to parse OpenAPI specification",
                "details": e.to_string()
            }))
        }
    }
}

/// Configure health check routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("")
            .route("/health", web::get().to(health_check))
            .route("/ready", web::get().to(readiness_check))
            .route("/api/docs", web::get().to(api_docs_ui))
            .route("/api/docs/openapi.yaml", web::get().to(openapi_spec_yaml))
            .route("/api/docs/openapi.json", web::get().to(openapi_spec_json)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_health_check_returns_200() {
        let app = test::init_service(App::new().configure(configure)).await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);

        let body: HealthResponse = test::read_body_json(resp).await;
        assert_eq!(body.status, "healthy");
        assert_eq!(body.checks.application, "healthy");
    }
}
