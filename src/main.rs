mod config;
mod core;
mod middleware;
mod modules;

use actix_web::{web, App, HttpServer};
use config::Config;
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

    // Create server configuration
    let server_config = config::server::ServerConfig::new(
        config.app_host.clone(),
        config.app_port,
    );

    let bind_address = server_config.bind_address();

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config.clone()))
            // Health check endpoint
            .route("/health", web::get().to(health_check))
            // Routes will be added here
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
        "service": "paytrust"
    })))
}
