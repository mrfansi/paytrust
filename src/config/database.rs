use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use sqlx::Error as SqlxError;
use std::time::Duration;

/// Create MySQL connection pool with configuration from environment
pub async fn create_pool(database_url: &str, min_connections: u32, max_connections: u32) -> Result<MySqlPool, SqlxError> {
    MySqlPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(min_connections)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600)) // 10 minutes
        .max_lifetime(Duration::from_secs(1800)) // 30 minutes
        .test_before_acquire(true) // Validate connections
        .connect(database_url)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires database connection
    async fn test_create_pool() {
        let database_url = "mysql://root:password@localhost:3306/paytrust_test";
        let pool = create_pool(database_url, 5, 10).await;
        assert!(pool.is_ok());
    }
}
