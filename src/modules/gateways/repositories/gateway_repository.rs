use crate::core::{Currency, Result};
use crate::modules::gateways::models::PaymentGatewayConfig;
use sqlx::MySqlPool;

/// Gateway repository for database operations
#[derive(Clone)]
pub struct GatewayRepository {
    pool: MySqlPool,
}

impl GatewayRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Find gateway by ID
    pub async fn find_by_id(&self, id: &str) -> Result<Option<PaymentGatewayConfig>> {
        let gateway = sqlx::query_as::<_, PaymentGatewayConfig>(
            r#"
            SELECT id, name, supported_currencies, fee_percentage, fee_fixed,
                   api_key_encrypted, webhook_secret, webhook_url, is_active,
                   environment, created_at, updated_at
            FROM payment_gateways
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(gateway)
    }

    /// Find gateway by name
    pub async fn find_by_name(&self, name: &str) -> Result<Option<PaymentGatewayConfig>> {
        let gateway = sqlx::query_as::<_, PaymentGatewayConfig>(
            r#"
            SELECT id, name, supported_currencies, fee_percentage, fee_fixed,
                   api_key_encrypted, webhook_secret, webhook_url, is_active,
                   environment, created_at, updated_at
            FROM payment_gateways
            WHERE name = ? AND is_active = TRUE
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(gateway)
    }

    /// List all active gateways
    pub async fn list_active(&self) -> Result<Vec<PaymentGatewayConfig>> {
        let gateways = sqlx::query_as::<_, PaymentGatewayConfig>(
            r#"
            SELECT id, name, supported_currencies, fee_percentage, fee_fixed,
                   api_key_encrypted, webhook_secret, webhook_url, is_active,
                   environment, created_at, updated_at
            FROM payment_gateways
            WHERE is_active = TRUE
            ORDER BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(gateways)
    }

    /// List gateways that support a specific currency
    pub async fn find_by_currency(&self, currency: Currency) -> Result<Vec<PaymentGatewayConfig>> {
        let currency_str = format!("{:?}", currency);
        
        let gateways = sqlx::query_as::<_, PaymentGatewayConfig>(
            r#"
            SELECT id, name, supported_currencies, fee_percentage, fee_fixed,
                   api_key_encrypted, webhook_secret, webhook_url, is_active,
                   environment, created_at, updated_at
            FROM payment_gateways
            WHERE is_active = TRUE
              AND JSON_CONTAINS(supported_currencies, ?)
            ORDER BY name
            "#,
        )
        .bind(format!("\"{}\"", currency_str))
        .fetch_all(&self.pool)
        .await?;

        Ok(gateways)
    }

    /// Create a new gateway configuration
    pub async fn create(&self, config: &PaymentGatewayConfig) -> Result<PaymentGatewayConfig> {
        sqlx::query(
            r#"
            INSERT INTO payment_gateways (
                id, name, supported_currencies, fee_percentage, fee_fixed,
                api_key_encrypted, webhook_secret, webhook_url, is_active, environment
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&config.id)
        .bind(&config.name)
        .bind(serde_json::to_string(&config.supported_currencies)?)
        .bind(config.fee_percentage)
        .bind(config.fee_fixed)
        .bind(&config.api_key_encrypted)
        .bind(&config.webhook_secret)
        .bind(&config.webhook_url)
        .bind(config.is_active)
        .bind(&config.environment)
        .execute(&self.pool)
        .await?;

        Ok(config.clone())
    }

    /// Update gateway configuration
    pub async fn update(&self, id: &str, config: &PaymentGatewayConfig) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE payment_gateways
            SET name = ?, supported_currencies = ?, fee_percentage = ?, fee_fixed = ?,
                webhook_secret = ?, webhook_url = ?, is_active = ?, environment = ?
            WHERE id = ?
            "#,
        )
        .bind(&config.name)
        .bind(serde_json::to_string(&config.supported_currencies)?)
        .bind(config.fee_percentage)
        .bind(config.fee_fixed)
        .bind(&config.webhook_secret)
        .bind(&config.webhook_url)
        .bind(config.is_active)
        .bind(&config.environment)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
