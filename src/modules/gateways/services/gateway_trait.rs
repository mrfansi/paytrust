use async_trait::async_trait;
use crate::core::error::AppResult;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Payment request to gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub external_id: String,
    pub amount: Decimal,
    pub currency: String,
    pub description: String,
    pub customer_email: Option<String>,
    pub customer_name: Option<String>,
    pub success_redirect_url: Option<String>,
    pub failure_redirect_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Payment response from gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResponse {
    pub gateway_reference: String,
    pub payment_url: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Webhook verification result
#[derive(Debug, Clone)]
pub struct WebhookVerification {
    pub is_valid: bool,
    pub gateway_reference: String,
    pub status: String,
    pub amount_paid: Decimal,
    pub payment_method: String,
    pub raw_payload: serde_json::Value,
}

/// Payment gateway trait for Xendit and Midtrans implementations
#[async_trait]
pub trait PaymentGateway: Send + Sync {
    /// Create a payment request and get payment URL
    async fn create_payment(&self, request: PaymentRequest) -> AppResult<PaymentResponse>;

    /// Verify webhook signature and extract payment data
    async fn verify_webhook(
        &self,
        signature: &str,
        payload: &str,
    ) -> AppResult<WebhookVerification>;

    /// Get gateway name
    fn name(&self) -> &str;

    /// Get supported currencies
    fn supported_currencies(&self) -> Vec<String>;
}
