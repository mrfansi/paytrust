use crate::core::error::{AppError, AppResult};
use crate::modules::gateways::models::gateway_config::GatewayConfig;
use async_trait::async_trait;
use sqlx::MySqlPool;

/// Gateway repository trait
#[async_trait]
pub trait GatewayRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> AppResult<Option<GatewayConfig>>;
    async fn find_by_name(&self, name: &str) -> AppResult<Option<GatewayConfig>>;
    async fn list_active(&self) -> AppResult<Vec<GatewayConfig>>;
}

/// MySQL implementation of gateway repository
pub struct MySqlGatewayRepository {
    pool: MySqlPool,
}

impl MySqlGatewayRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GatewayRepository for MySqlGatewayRepository {
    async fn find_by_id(&self, id: i64) -> AppResult<Option<GatewayConfig>> {
        let result = sqlx::query_as::<_, GatewayConfig>(
            r#"
            SELECT 
                id,
                name,
                supported_currencies,
                fee_percentage,
                fee_fixed,
                region,
                webhook_url,
                api_key_encrypted,
                is_active,
                environment,
                created_at,
                updated_at
            FROM gateway_configs
            WHERE id = ?
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(result)
    }

    async fn find_by_name(&self, name: &str) -> AppResult<Option<GatewayConfig>> {
        let result = sqlx::query_as::<_, GatewayConfig>(
            r#"
            SELECT 
                id,
                name,
                supported_currencies,
                fee_percentage,
                fee_fixed,
                region,
                webhook_url,
                api_key_encrypted,
                is_active,
                environment,
                created_at,
                updated_at
            FROM gateway_configs
            WHERE name = ?
            "#
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(result)
    }

    async fn list_active(&self) -> AppResult<Vec<GatewayConfig>> {
        let results = sqlx::query_as::<_, GatewayConfig>(
            r#"
            SELECT 
                id,
                name,
                supported_currencies,
                fee_percentage,
                fee_fixed,
                region,
                webhook_url,
                api_key_encrypted,
                is_active,
                environment,
                created_at,
                updated_at
            FROM gateway_configs
            WHERE is_active = TRUE
            ORDER BY name
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires database connection
    async fn test_find_by_name() {
        // This test requires a test database with migrations applied
        // Run with: cargo test --test gateway_repository_test -- --ignored
    }
}
