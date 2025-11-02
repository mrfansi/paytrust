mod config;
mod core;
mod middleware;
mod modules;

use actix_web::{web, App, HttpServer};
use config::Config;
use middleware::{configure_cors, ApiKeyAuth, ErrorHandler, InMemoryRateLimiter, RateLimitMiddleware};
use modules::gateways::repositories::gateway_repository::MySqlGatewayRepository;
use modules::invoices::repositories::invoice_repository::MySqlInvoiceRepository;
use modules::invoices::services::invoice_service::InvoiceService;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load configuration
    let config = Config::from_env().expect("Failed to load configuration");

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true),
        )
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!(
        "Starting PayTrust server at {}:{}",
        config.app_host,
        config.app_port
    );

    // Create database pool
    let pool = config::database::create_pool(
        &config.database_url,
        config.database_pool_size,
        config.database_max_connections,
    )
    .await
    .expect("Failed to create database pool");

    tracing::info!(
        "Database pool initialized ({} connections)",
        config.database_pool_size
    );

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    tracing::info!("Database migrations completed");

    // Create rate limiter
    let rate_limiter = Arc::new(InMemoryRateLimiter::new(config.rate_limit_per_minute));
    tracing::info!(
        "Rate limiter initialized ({} req/min)",
        config.rate_limit_per_minute
    );

    // Create server configuration
    let server_config = config::server::ServerConfig::new(
        config.app_host.clone(),
        config.app_port,
    );

    let bind_address = server_config.bind_address();

    // Initialize repositories
    let gateway_repo = Arc::new(MySqlGatewayRepository::new(pool.clone()));
    let invoice_repo = Arc::new(MySqlInvoiceRepository::new(pool.clone()));

    // Initialize services
    let invoice_service = Arc::new(InvoiceService::new(
        invoice_repo.clone(),
        gateway_repo.clone(),
    ));

    tracing::info!("Payment gateways loaded: xendit, midtrans");
    tracing::info!("Invoice service initialized");

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            // App data
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(invoice_service.clone()))
            // Middleware stack (order matters!)
            .wrap(configure_cors())
            .wrap(ErrorHandler)
            .wrap(tracing_actix_web::TracingLogger::default())
            // Public routes (no auth)
            .route("/health", web::get().to(health_check))
            .route("/ready", web::get().to(readiness_check))
            // Protected routes (with auth and rate limiting)
            .service(
                web::scope("")
                    .wrap(ApiKeyAuth)
                    .wrap(RateLimitMiddleware::new(rate_limiter.clone()))
                    // API routes
                    .configure(modules::invoices::controllers::configure)
            )
    })
    .workers(server_config.workers)
    .bind(&bind_address)?
    .run()
    .await
}

/// Health check endpoint
async fn health_check() -> actix_web::Result<actix_web::HttpResponse> {
    Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "paytrust",
        "version": env!("CARGO_PKG_VERSION")
    })))
}

/// Readiness check endpoint (checks database connectivity)
async fn readiness_check(
    pool: web::Data<sqlx::MySqlPool>,
) -> actix_web::Result<actix_web::HttpResponse> {
    // Check database connectivity
    match sqlx::query("SELECT 1").fetch_one(pool.get_ref()).await {
        Ok(_) => Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
            "status": "ready",
            "database": "connected"
        }))),
        Err(e) => Ok(actix_web::HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "status": "not ready",
            "database": "disconnected",
            "error": e.to_string()
        }))),
    }
}
