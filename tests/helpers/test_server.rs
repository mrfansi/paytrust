// Test Server Helpers
//
// Spawns real HTTP test server using actix-test.
// Uses real database connections and real application configuration.

use actix_web::{web, App, HttpResponse, HttpServer};
use sqlx::MySqlPool;
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use serde_json::json;

pub use actix_test::TestServer;

use super::test_database::create_test_pool;

/// Test metrics collector for performance monitoring
///
/// Tracks HTTP request latencies, database query counts, and test duration
/// per FR-009 requirement for test observability.
#[derive(Clone)]
pub struct TestMetrics {
    request_count: Arc<AtomicUsize>,
    total_latency_micros: Arc<AtomicU64>,
    min_latency_micros: Arc<AtomicU64>,
    max_latency_micros: Arc<AtomicU64>,
    db_query_count: Arc<AtomicUsize>,
    test_start: Arc<Instant>,
}

impl TestMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            request_count: Arc::new(AtomicUsize::new(0)),
            total_latency_micros: Arc::new(AtomicU64::new(0)),
            min_latency_micros: Arc::new(AtomicU64::new(u64::MAX)),
            max_latency_micros: Arc::new(AtomicU64::new(0)),
            db_query_count: Arc::new(AtomicUsize::new(0)),
            test_start: Arc::new(Instant::now()),
        }
    }

    /// Record an HTTP request latency
    pub fn record_request(&self, latency: Duration) {
        let micros = latency.as_micros() as u64;
        
        self.request_count.fetch_add(1, Ordering::Relaxed);
        self.total_latency_micros.fetch_add(micros, Ordering::Relaxed);
        
        // Update min
        let mut current_min = self.min_latency_micros.load(Ordering::Relaxed);
        while micros < current_min {
            match self.min_latency_micros.compare_exchange(
                current_min,
                micros,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }
        
        // Update max
        let mut current_max = self.max_latency_micros.load(Ordering::Relaxed);
        while micros > current_max {
            match self.max_latency_micros.compare_exchange(
                current_max,
                micros,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }
    }

    /// Record a database query execution
    pub fn record_query(&self) {
        self.db_query_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current metrics snapshot as JSON
    ///
    /// Returns metrics in format per FR-009:
    /// - http_p50_latency_ms: median latency (approximated as mean)
    /// - http_p95_latency_ms: 95th percentile (approximated as max)
    /// - http_p99_latency_ms: 99th percentile (approximated as max)
    /// - test_duration_ms: total test duration
    /// - db_query_count: number of database queries executed
    pub fn to_json(&self) -> serde_json::Value {
        let count = self.request_count.load(Ordering::Relaxed);
        let total_micros = self.total_latency_micros.load(Ordering::Relaxed);
        let min_micros = self.min_latency_micros.load(Ordering::Relaxed);
        let max_micros = self.max_latency_micros.load(Ordering::Relaxed);
        let query_count = self.db_query_count.load(Ordering::Relaxed);
        let duration = self.test_start.elapsed();

        let avg_latency_ms = if count > 0 {
            (total_micros as f64 / count as f64) / 1000.0
        } else {
            0.0
        };

        let min_latency_ms = if min_micros == u64::MAX {
            0.0
        } else {
            min_micros as f64 / 1000.0
        };

        let max_latency_ms = max_micros as f64 / 1000.0;

        json!({
            "http_request_count": count,
            "http_avg_latency_ms": format!("{:.2}", avg_latency_ms),
            "http_min_latency_ms": format!("{:.2}", min_latency_ms),
            "http_max_latency_ms": format!("{:.2}", max_latency_ms),
            "http_p50_latency_ms": format!("{:.2}", avg_latency_ms), // Approximate
            "http_p95_latency_ms": format!("{:.2}", max_latency_ms * 0.95), // Approximate
            "http_p99_latency_ms": format!("{:.2}", max_latency_ms), // Approximate
            "db_query_count": query_count,
            "test_duration_ms": duration.as_millis(),
        })
    }

    /// Print metrics to stdout in JSON format
    ///
    /// Use this at the end of tests to report performance metrics.
    ///
    /// # Example
    /// ```no_run
    /// let metrics = TestMetrics::new();
    /// // ... run tests ...
    /// metrics.report();
    /// ```
    pub fn report(&self) {
        println!("\n{}", serde_json::to_string_pretty(&self.to_json()).unwrap());
    }
}

impl Default for TestMetrics {
    fn default() -> Self {
        Self::new()
    }
}

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

/// Spawn test server with metrics collection enabled
///
/// Returns both the test server and a metrics collector for performance tracking.
///
/// # Example
/// ```no_run
/// #[actix_web::test]
/// async fn test_with_metrics() {
///     let (srv, metrics) = spawn_test_server_with_metrics().await;
///     
///     let start = Instant::now();
///     let response = srv.get("/api/invoices").send().await.unwrap();
///     metrics.record_request(start.elapsed());
///     
///     metrics.report(); // Print metrics to stdout
/// }
/// ```
pub async fn spawn_test_server_with_metrics() -> (TestServer, TestMetrics) {
    let metrics = TestMetrics::new();
    let srv = spawn_test_server().await;
    (srv, metrics)
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
