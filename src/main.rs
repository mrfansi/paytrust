mod config;
mod core;

use actix_web::{web, App, HttpResponse, HttpServer};
use config::Config;
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

    // Start HTTP server
    let bind_address = config.server.bind_address();
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .route("/health", web::get().to(health_check))
            .route("/", web::get().to(index))
    })
    .bind(&bind_address)?
    .run();

    tracing::info!("Server started at http://{}", bind_address);
    
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
