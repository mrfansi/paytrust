use crate::core::{AppError, Result};
use serde::Deserialize;
use std::env;

pub mod database;
pub mod server;

pub use database::DatabaseConfig;
pub use server::ServerConfig;

/// Main application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub app: AppConfig,
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub xendit: GatewayConfig,
    pub midtrans: GatewayConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub env: String,
    pub log_level: String,
    pub invoice_expiry_hours: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GatewayConfig {
    pub api_key: String,
    pub webhook_secret: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    pub api_key_secret: String,
    pub rate_limit_per_minute: u32,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        // Load .env file if present
        dotenvy::dotenv().ok();

        let config = Config {
            app: AppConfig {
                env: env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()),
                log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
                invoice_expiry_hours: env::var("DEFAULT_INVOICE_EXPIRY_HOURS")
                    .unwrap_or_else(|_| "24".to_string())
                    .parse()
                    .map_err(|_| {
                        AppError::Configuration("Invalid DEFAULT_INVOICE_EXPIRY_HOURS".to_string())
                    })?,
            },
            database: DatabaseConfig::from_env()?,
            server: ServerConfig::from_env()?,
            xendit: GatewayConfig {
                api_key: env::var("XENDIT_API_KEY")
                    .map_err(|_| AppError::Configuration("XENDIT_API_KEY not set".to_string()))?,
                webhook_secret: env::var("XENDIT_WEBHOOK_SECRET").map_err(|_| {
                    AppError::Configuration("XENDIT_WEBHOOK_SECRET not set".to_string())
                })?,
                base_url: env::var("XENDIT_BASE_URL")
                    .unwrap_or_else(|_| "https://api.xendit.co".to_string()),
            },
            midtrans: GatewayConfig {
                api_key: env::var("MIDTRANS_SERVER_KEY").map_err(|_| {
                    AppError::Configuration("MIDTRANS_SERVER_KEY not set".to_string())
                })?,
                webhook_secret: env::var("MIDTRANS_WEBHOOK_SECRET").map_err(|_| {
                    AppError::Configuration("MIDTRANS_WEBHOOK_SECRET not set".to_string())
                })?,
                base_url: env::var("MIDTRANS_BASE_URL")
                    .unwrap_or_else(|_| "https://api.sandbox.midtrans.com".to_string()),
            },
            security: SecurityConfig {
                api_key_secret: env::var("API_KEY_SECRET")
                    .map_err(|_| AppError::Configuration("API_KEY_SECRET not set".to_string()))?,
                rate_limit_per_minute: env::var("RATE_LIMIT_PER_MINUTE")
                    .unwrap_or_else(|_| "1000".to_string())
                    .parse()
                    .map_err(|_| {
                        AppError::Configuration("Invalid RATE_LIMIT_PER_MINUTE".to_string())
                    })?,
            },
        };

        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.app.invoice_expiry_hours == 0 {
            return Err(AppError::Configuration(
                "Invoice expiry hours must be greater than 0".to_string(),
            ));
        }

        if self.security.rate_limit_per_minute == 0 {
            return Err(AppError::Configuration(
                "Rate limit must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}
