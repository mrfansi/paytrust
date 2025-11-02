mod config;
mod core;
mod middleware;
mod modules;

use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer};
use config::Config;
use middleware::{ApiKeyAuth, MetricsCollector, MetricsMiddleware, RateLimiter, RequestId};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "paytrust=debug,actix_web=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env().expect("Failed to load configuration");
    config.validate().expect("Configuration validation failed");

    tracing::info!("Starting PayTrust Payment Orchestration Platform");
    tracing::info!("Environment: {}", config.app.env);
    tracing::info!("Server binding to: {}", config.server.bind_address());

    // Create database connection pool
    let db_pool = config
        .database
        .create_pool()
        .await
        .expect("Failed to create database pool");

    tracing::info!(
        "Database pool initialized ({} connections)",
        config.database.pool_size
    );

    // Run database migrations
    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run database migrations");
    tracing::info!("Database migrations completed");

    // Clone config and pool for use in closure
    let rate_limit_per_minute = config.security.rate_limit_per_minute;
    let bind_address = config.server.bind_address();

    // Initialize metrics collector
    let metrics_collector = MetricsCollector::new();
    tracing::info!("Metrics collection enabled at /metrics endpoint");

    // Start HTTP server
    let server = HttpServer::new(move || {
        // CORS configuration
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            // Data
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(metrics_collector.clone()))
            // Middleware
            .wrap(cors)
            .wrap(Logger::new("%a \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %T"))
            .wrap(RequestId) // Add request ID to all requests
            .wrap(MetricsMiddleware::new(metrics_collector.clone())) // Collect metrics
            .wrap(RateLimiter::new(rate_limit_per_minute))
            .wrap(ApiKeyAuth::new(db_pool.clone()))
            // Health check routes (no auth required)
            .configure(modules::health::controllers::configure)
            // Root route
            .route("/", web::get().to(index))
            // API v1 routes
            .service(
                web::scope("/v1")
                    .configure(modules::invoices::controllers::configure)
                    .configure(modules::installments::controllers::configure)
                    .configure(modules::reports::controllers::configure)
                    .configure(modules::gateways::controllers::configure)
                    .configure(modules::taxes::controllers::configure)
            )
    })
    .bind(&bind_address)?
    .shutdown_timeout(30) // 30 seconds graceful shutdown
    .run();

    tracing::info!("Server started at http://{}", bind_address);
    tracing::info!("Ready to accept requests");
    tracing::info!("Press Ctrl+C to shutdown gracefully");

    // Graceful shutdown on SIGTERM/SIGINT
    let server_handle = server.handle();
    
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl_c signal");
        
        tracing::info!("Shutdown signal received, starting graceful shutdown...");
        tracing::info!("Waiting for in-flight requests to complete (max 30s)...");
        
        server_handle.stop(true).await;
    });

    server.await
}

async fn index() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "service": "PayTrust Payment Orchestration Platform",
        "version": "0.1.0",
        "status": "running"
    }))
}
