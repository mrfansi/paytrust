pub mod database;
pub mod server;

use anyhow::Result;
use dotenvy::dotenv;
use std::env;

/// Application configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct Config {
    pub app_env: String,
    pub app_host: String,
    pub app_port: u16,
    pub log_level: String,
    pub database_url: String,
    pub database_pool_size: u32,
    pub database_max_connections: u32,
    pub xendit_api_key: String,
    pub xendit_webhook_secret: String,
    pub xendit_base_url: String,
    pub midtrans_server_key: String,
    pub midtrans_webhook_secret: String,
    pub midtrans_base_url: String,
    pub api_key_secret: String,
    pub admin_api_key: String,
    pub rate_limit_per_minute: u32,
    pub default_invoice_expiry_hours: u32,
}

impl Config {
    /// Load configuration from environment variables
    /// Validates all required variables are present
    pub fn from_env() -> Result<Self> {
        // Load .env file if present
        dotenv().ok();

        // Validate admin API key
        let admin_api_key = env::var("ADMIN_API_KEY")
            .map_err(|_| anyhow::anyhow!("ADMIN_API_KEY environment variable is required"))?;
        
        if admin_api_key.len() < 32 {
            anyhow::bail!("ADMIN_API_KEY must be at least 32 characters long");
        }
        
        if !admin_api_key.chars().all(|c| c.is_alphanumeric() || c.is_ascii_punctuation()) {
            anyhow::bail!("ADMIN_API_KEY must contain only alphanumeric and symbol characters");
        }

        Ok(Self {
            app_env: env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()),
            app_host: env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            app_port: env::var("APP_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("APP_PORT must be a valid port number"),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL environment variable is required"),
            database_pool_size: env::var("DATABASE_POOL_SIZE")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .expect("DATABASE_POOL_SIZE must be a valid number"),
            database_max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .expect("DATABASE_MAX_CONNECTIONS must be a valid number"),
            xendit_api_key: env::var("XENDIT_API_KEY")
                .expect("XENDIT_API_KEY environment variable is required"),
            xendit_webhook_secret: env::var("XENDIT_WEBHOOK_SECRET")
                .expect("XENDIT_WEBHOOK_SECRET environment variable is required"),
            xendit_base_url: env::var("XENDIT_BASE_URL")
                .unwrap_or_else(|_| "https://api.xendit.co".to_string()),
            midtrans_server_key: env::var("MIDTRANS_SERVER_KEY")
                .expect("MIDTRANS_SERVER_KEY environment variable is required"),
            midtrans_webhook_secret: env::var("MIDTRANS_WEBHOOK_SECRET")
                .expect("MIDTRANS_WEBHOOK_SECRET environment variable is required"),
            midtrans_base_url: env::var("MIDTRANS_BASE_URL")
                .unwrap_or_else(|_| "https://api.sandbox.midtrans.com".to_string()),
            api_key_secret: env::var("API_KEY_SECRET")
                .expect("API_KEY_SECRET environment variable is required"),
            admin_api_key,
            rate_limit_per_minute: env::var("RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .expect("RATE_LIMIT_PER_MINUTE must be a valid number"),
            default_invoice_expiry_hours: env::var("DEFAULT_INVOICE_EXPIRY_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .expect("DEFAULT_INVOICE_EXPIRY_HOURS must be a valid number"),
        })
    }
}
