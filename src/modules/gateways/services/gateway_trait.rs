use crate::core::{Currency, Result};
use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Payment gateway trait for processing payments and verifying webhooks
#[async_trait]
pub trait PaymentGateway: Send + Sync {
    /// Create a payment request and return payment URL
    async fn create_payment(&self, request: PaymentRequest) -> Result<PaymentResponse>;

    /// Verify webhook signature
    async fn verify_webhook(&self, signature: &str, payload: &str) -> Result<bool>;

    /// Process webhook payload and extract payment information
    async fn process_webhook(&self, payload: &str) -> Result<WebhookPayload>;

    /// Get gateway name
    fn name(&self) -> &str;

    /// Check if gateway supports a currency
    fn supports_currency(&self, currency: Currency) -> bool;
}

/// Payment request data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequest {
    /// Invoice ID (external reference)
    pub external_id: String,
    
    /// Payment amount
    pub amount: Decimal,
    
    /// Currency
    pub currency: Currency,
    
    /// Description
    pub description: String,
    
    /// Payer information (optional)
    pub payer_email: Option<String>,
    
    /// Success redirect URL
    pub success_redirect_url: Option<String>,
    
    /// Failure redirect URL
    pub failure_redirect_url: Option<String>,
}

/// Payment response from gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResponse {
    /// Gateway transaction reference
    pub gateway_reference: String,
    
    /// Payment URL for customer
    pub payment_url: String,
    
    /// Expiration time (if applicable)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Payment status
    pub status: PaymentStatus,
}

/// Payment status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentStatus {
    Pending,
    Completed,
    Failed,
    Expired,
}

/// Webhook payload data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    /// Gateway transaction reference
    pub gateway_reference: String,
    
    /// External ID (invoice ID)
    pub external_id: String,
    
    /// Amount paid
    pub amount_paid: Decimal,
    
    /// Payment method used
    pub payment_method: String,
    
    /// Payment status
    pub status: PaymentStatus,
    
    /// Full gateway response (JSON)
    pub raw_response: serde_json::Value,
}
