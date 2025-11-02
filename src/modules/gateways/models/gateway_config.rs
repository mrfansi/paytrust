use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Payment gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GatewayConfig {
    pub id: i64,
    pub name: String,
    #[sqlx(json)]
    pub supported_currencies: Vec<String>,
    pub fee_percentage: Decimal,
    pub fee_fixed: Decimal,
    pub region: Option<String>,
    pub webhook_url: Option<String>,
    pub api_key_encrypted: Option<String>,
    pub is_active: bool,
    pub environment: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl GatewayConfig {
    /// Calculate service fee for given subtotal
    /// Formula: (subtotal × fee_percentage) + fee_fixed
    pub fn calculate_service_fee(&self, subtotal: Decimal) -> Decimal {
        (subtotal * self.fee_percentage) + self.fee_fixed
    }

    /// Check if gateway supports given currency
    pub fn supports_currency(&self, currency: &str) -> bool {
        self.supported_currencies
            .iter()
            .any(|c| c.eq_ignore_ascii_case(currency))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_service_fee_calculation() {
        let config = GatewayConfig {
            id: 1,
            name: "xendit".to_string(),
            supported_currencies: vec!["IDR".to_string()],
            fee_percentage: dec!(0.029), // 2.9%
            fee_fixed: dec!(0),
            region: None,
            webhook_url: None,
            api_key_encrypted: None,
            is_active: true,
            environment: "sandbox".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let subtotal = dec!(1000000); // IDR 1,000,000
        let fee = config.calculate_service_fee(subtotal);
        assert_eq!(fee, dec!(29000)); // IDR 29,000 (2.9%)
    }

    #[test]
    fn test_currency_support() {
        let config = GatewayConfig {
            id: 1,
            name: "xendit".to_string(),
            supported_currencies: vec!["IDR".to_string(), "MYR".to_string()],
            fee_percentage: dec!(0.029),
            fee_fixed: dec!(0),
            region: None,
            webhook_url: None,
            api_key_encrypted: None,
            is_active: true,
            environment: "sandbox".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert!(config.supports_currency("IDR"));
        assert!(config.supports_currency("idr")); // Case insensitive
        assert!(config.supports_currency("MYR"));
        assert!(!config.supports_currency("USD"));
    }
}
