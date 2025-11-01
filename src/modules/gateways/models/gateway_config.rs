use crate::core::Currency;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Payment gateway configuration model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PaymentGatewayConfig {
    pub id: String,
    pub name: String,

    #[sqlx(json)]
    pub supported_currencies: Vec<String>,

    pub fee_percentage: Decimal,
    pub fee_fixed: Decimal,

    #[sqlx(skip)]
    #[serde(skip)]
    pub api_key_encrypted: Vec<u8>,

    pub webhook_secret: String,
    pub webhook_url: String,
    pub is_active: bool,
    pub environment: GatewayEnvironment,

    #[sqlx(default)]
    pub created_at: chrono::NaiveDateTime,

    #[sqlx(default)]
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR(20)", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum GatewayEnvironment {
    Sandbox,
    Production,
}

impl PaymentGatewayConfig {
    /// Check if gateway supports a specific currency
    pub fn supports_currency(&self, currency: Currency) -> bool {
        let currency_str = format!("{:?}", currency);
        self.supported_currencies
            .iter()
            .any(|c| c.eq_ignore_ascii_case(&currency_str))
    }

    /// Calculate service fee for a given amount
    pub fn calculate_service_fee(&self, subtotal: Decimal) -> Decimal {
        (subtotal * self.fee_percentage) + self.fee_fixed
    }
}

impl std::fmt::Display for GatewayEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GatewayEnvironment::Sandbox => write!(f, "sandbox"),
            GatewayEnvironment::Production => write!(f, "production"),
        }
    }
}
