mod config;
mod core;
mod middleware;
mod modules;

use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer};
use config::Config;
use middleware::{ApiKeyAuth, RateLimiter};
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
            // Middleware
            .wrap(cors)
            .wrap(Logger::new("%a \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %T"))
            .wrap(RateLimiter::new(rate_limit_per_minute))
            .wrap(ApiKeyAuth::new(db_pool.clone()))
            // Routes
            .route("/health", web::get().to(health_check))
            .route("/", web::get().to(index))
            // API v1 routes
            .service(
                web::scope("/v1")
                    .configure(modules::invoices::controllers::configure)
            )
    })
    .bind(&bind_address)?
    .run();

    tracing::info!("Server started at http://{}", bind_address);
    tracing::info!("Ready to accept requests");

    server.await
}

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "paytrust"
    }))
}

async fn index() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "service": "PayTrust Payment Orchestration Platform",
        "version": "0.1.0",
        "status": "running"
    }))
}
