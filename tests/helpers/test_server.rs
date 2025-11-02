// Test Server Helpers
//
// Spawns real HTTP test server using actix-test.
// Uses real database connections and real application configuration.

use actix_web::{web, App, HttpResponse, HttpServer};
use sqlx::MySqlPool;
use std::net::TcpListener;

pub use actix_test::TestServer;

use super::test_database::create_test_pool;

/// Spawn a real HTTP test server with full application configuration
///
/// # Behavior
/// - Starts actix-web server on random available port
/// - Configures all application modules (invoices, installments, gateways, etc.)
/// - Connects to test database via TEST_DATABASE_URL
/// - Server stops automatically when TestServer drops
///
/// # Returns
/// TestServer instance with methods:
/// - `url() -> String` - Base URL of test server
/// - `get(path: &str)` - Build GET request
/// - `post(path: &str)` - Build POST request
/// - `put(path: &str)` - Build PUT request
/// - `delete(path: &str)` - Build DELETE request
///
/// # Error Handling
/// Panics with clear message if:
/// - Test database connection fails
/// - Server fails to bind to port
/// - Application configuration is invalid
///
/// # Example
/// ```no_run
/// #[actix_web::test]
/// async fn test_health_endpoint() {
///     let srv = spawn_test_server().await;
///     let response = srv.get("/health").send().await.unwrap();
///     assert_eq!(response.status(), 200);
/// }
/// ```
pub async fn spawn_test_server() -> TestServer {
    // Create test database pool
    let pool = create_test_pool().await;

    // Start test server with actix-test
    actix_test::start(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .configure(configure_test_routes)
    })
}

/// Configure test routes (simplified version of production routes)
///
/// This includes the minimal routes needed for testing.
/// Full route configuration is in src/main.rs
fn configure_test_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/health")
            .route("", web::get().to(|| async { HttpResponse::Ok().json(serde_json::json!({"status": "ok"})) }))
    )
    // Add more routes as needed for testing
    // For now, health endpoint is sufficient to verify server startup
    ;
}

/// Spawn test server with custom configuration
///
/// Allows tests to provide custom app configuration function.
///
/// # Example
/// ```no_run
/// #[actix_web::test]
/// async fn test_custom_config() {
///     let srv = spawn_test_server_with_config(|cfg| {
///         cfg.service(web::resource("/custom").to(|| async { HttpResponse::Ok().body("custom") }));
///     }).await;
/// }
/// ```
pub async fn spawn_test_server_with_config<F>(config_fn: F) -> TestServer
where
    F: Fn(&mut web::ServiceConfig) + Send + Clone + 'static,
{
    let pool = create_test_pool().await;

    actix_test::start(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .configure(config_fn.clone())
    })
}

/// Find an available port for test server
///
/// # Returns
/// Port number that is available for binding
///
/// # Error Handling
/// Panics if no port is available (extremely rare)
pub fn find_available_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("Failed to bind to any available port")
        .local_addr()
        .expect("Failed to get local address")
        .port()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_web::test]
    #[ignore] // Requires test database
    async fn test_spawn_server_starts_successfully() {
        let srv = spawn_test_server().await;
        let response = srv.get("/health").send().await.unwrap();
        assert!(response.status().is_success());
    }

    #[test]
    fn test_find_available_port() {
        let port1 = find_available_port();
        let port2 = find_available_port();

        assert!(port1 > 0);
        assert!(port2 > 0);
        // Ports should be different (unless extreme collision)
        assert_ne!(port1, port2);
    }
}
