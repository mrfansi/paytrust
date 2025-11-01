use crate::core::{AppError, Result};
use serde::Deserialize;
use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use std::env;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub pool_size: u32,
    pub max_connections: u32,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self> {
        Ok(DatabaseConfig {
            url: env::var("DATABASE_URL")
                .map_err(|_| AppError::Configuration("DATABASE_URL not set".to_string()))?,
            pool_size: env::var("DATABASE_POOL_SIZE")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .map_err(|_| {
                    AppError::Configuration("Invalid DATABASE_POOL_SIZE".to_string())
                })?,
            max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .map_err(|_| {
                    AppError::Configuration("Invalid DATABASE_MAX_CONNECTIONS".to_string())
                })?,
        })
    }

    /// Create a MySQL connection pool
    pub async fn create_pool(&self) -> Result<MySqlPool> {
        MySqlPoolOptions::new()
            .max_connections(self.max_connections)
            .min_connections(self.pool_size)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600)) // 10 minutes
            .max_lifetime(Duration::from_secs(1800)) // 30 minutes
            .test_before_acquire(true)
            .connect(&self.url)
            .await
            .map_err(|e| {
                AppError::Database(e)
            })
    }
}
