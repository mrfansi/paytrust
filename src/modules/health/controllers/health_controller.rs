use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
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

/// Configure health check routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("")
            .route("/health", web::get().to(health_check))
            .route("/ready", web::get().to(readiness_check)),
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
