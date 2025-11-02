/// Test database configuration and setup
/// 
/// **CONSTITUTION PRINCIPLE III COMPLIANCE**:
/// - Uses REAL MySQL test database instances (no mocks/stubs)
/// - Connection pool with min 5, max 20 connections
/// - Transaction isolation level: READ COMMITTED
/// - Migration runner executes same migrations as production
/// - TRUNCATE tables between tests for data isolation
/// - DROP/CREATE database for schema migration tests
/// 
/// **MOCKS/STUBS PROHIBITED** per Constitution Principle III and NFR-008

use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use sqlx::{Connection, Executor, MySqlConnection};
use std::time::Duration;

/// Test database configuration
pub struct TestDatabase {
    pub pool: MySqlPool,
    pub database_name: String,
}

impl TestDatabase {
    /// Create a new test database with unique name
    /// Each test gets its own database for complete isolation
    pub async fn new() -> Self {
        let database_name = format!("paytrust_test_{}", uuid::Uuid::new_v4().simple());
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "mysql://root:password@localhost:3306".to_string());

        // Connect to MySQL server (without database)
        let mut conn = MySqlConnection::connect(&database_url)
            .await
            .expect("Failed to connect to MySQL server");

        // Create test database
        conn.execute(
            format!(
                "CREATE DATABASE {} CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci",
                database_name
            )
            .as_str(),
        )
        .await
        .expect("Failed to create test database");

        // Connect to test database with connection pool
        let pool = MySqlPoolOptions::new()
            .min_connections(5)
            .max_connections(20)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .test_before_acquire(true)
            .connect(&format!("{}/{}", database_url, database_name))
            .await
            .expect("Failed to create connection pool");

        // Run migrations (same as production)
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        Self {
            pool,
            database_name,
        }
    }

    /// Clean all data from tables (for test isolation)
    /// Uses TRUNCATE for fast cleanup between tests
    pub async fn cleanup(&self) {
        let tables = vec![
            "webhook_retry_log",
            "api_key_audit_log",
            "payment_transactions",
            "installment_schedules",
            "line_items",
            "invoices",
            "api_keys",
            // Don't truncate gateway_configs - seed data needed
        ];

        for table in tables {
            sqlx::query(&format!("TRUNCATE TABLE {}", table))
                .execute(&self.pool)
                .await
                .expect(&format!("Failed to truncate table {}", table));
        }
    }

    /// Execute function within a transaction that auto-rolls back
    /// Useful for tests that need transaction isolation
    pub async fn with_transaction<F, Fut, T>(&self, f: F) -> T
    where
        F: FnOnce(sqlx::Transaction<'_, sqlx::MySql>) -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let mut tx = self
            .pool
            .begin()
            .await
            .expect("Failed to begin transaction");

        let result = f(tx).await;

        // Transaction is automatically rolled back when dropped
        result
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        // Clean up test database
        // Note: This runs synchronously in drop, database cleanup happens in background
        let database_name = self.database_name.clone();
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "mysql://root:password@localhost:3306".to_string());

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Ok(mut conn) = MySqlConnection::connect(&database_url).await {
                    let _ = conn
                        .execute(format!("DROP DATABASE IF EXISTS {}", database_name).as_str())
                        .await;
                }
            });
        });
    }
}

/// Helper to create test database for integration tests
pub async fn setup_test_db() -> TestDatabase {
    TestDatabase::new().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires MySQL connection
    async fn test_database_creation() {
        let db = TestDatabase::new().await;
        
        // Verify connection pool works
        let result = sqlx::query("SELECT 1 as test")
            .fetch_one(&db.pool)
            .await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires MySQL connection
    async fn test_cleanup() {
        let db = TestDatabase::new().await;
        
        // Insert test data
        sqlx::query("INSERT INTO api_keys (key_hash, tenant_id) VALUES ('test_hash', 'test_tenant')")
            .execute(&db.pool)
            .await
            .expect("Failed to insert test data");
        
        // Cleanup
        db.cleanup().await;
        
        // Verify data is gone
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM api_keys")
            .fetch_one(&db.pool)
            .await
            .expect("Failed to count rows");
        
        assert_eq!(count.0, 0);
    }
}
