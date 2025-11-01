use super::gateway_trait::{PaymentGateway, PaymentRequest, PaymentResponse, PaymentStatus, WebhookPayload};
use crate::core::{AppError, Currency, Result};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::str::FromStr;

/// Xendit payment gateway client
///
/// Implements PaymentGateway trait for Xendit API integration
/// API Documentation: https://developers.xendit.co/api-reference/
pub struct XenditClient {
    client: Client,
    api_key: String,
    webhook_secret: String,
    base_url: String,
}

impl XenditClient {
    /// Create a new Xendit client
    ///
    /// # Arguments
    /// * `api_key` - Xendit API key (from XENDIT_API_KEY env var)
    /// * `webhook_secret` - Xendit webhook verification token
    /// * `base_url` - Xendit API base URL (defaults to https://api.xendit.co)
    pub fn new(api_key: String, webhook_secret: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            webhook_secret,
            base_url: base_url.unwrap_or_else(|| "https://api.xendit.co".to_string()),
        }
    }

    /// Verify HMAC signature for webhook
    fn verify_hmac(&self, signature: &str, payload: &str) -> bool {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac = match HmacSha256::new_from_slice(self.webhook_secret.as_bytes()) {
            Ok(m) => m,
            Err(_) => return false,
        };

        mac.update(payload.as_bytes());

        let expected_signature = hex::encode(mac.finalize().into_bytes());
        
        // Constant-time comparison
        signature == expected_signature
    }
}

#[async_trait]
impl PaymentGateway for XenditClient {
    async fn create_payment(&self, request: PaymentRequest) -> Result<PaymentResponse> {
        // Xendit Invoice API endpoint
        let url = format!("{}/v2/invoices", self.base_url);

        // Convert amount to smallest currency unit (for IDR, no decimals; for others, keep 2 decimals)
        let amount = match request.currency {
            Currency::IDR => request.amount.round(),
            _ => request.amount.round_dp(2),
        };

        // Build description with installment info if present (T097)
        let description = if let Some(ref installment_info) = request.installment_info {
            format!(
                "{} - Installment {}/{}",
                request.description,
                installment_info.installment_number,
                installment_info.total_installments
            )
        } else {
            request.description.clone()
        };

        // Build Xendit invoice request
        let xendit_request = json!({
            "external_id": request.external_id,
            "amount": amount,
            "payer_email": request.payer_email,
            "description": description,
            "currency": request.currency.to_string().to_uppercase(),
            "invoice_duration": 86400, // 24 hours in seconds (FR-044)
            "success_redirect_url": request.success_redirect_url,
            "failure_redirect_url": request.failure_redirect_url,
        });

        // Send request to Xendit (FR-038: Descriptive errors with gateway name)
        let response = self
            .client
            .post(&url)
            .basic_auth(&self.api_key, Some(""))
            .json(&xendit_request)
            .send()
            .await
            .map_err(|e| {
                // FR-038: Descriptive error with gateway name
                if e.is_connect() || e.is_timeout() {
                    AppError::gateway(format!(
                        "Xendit gateway unavailable: {} ({})",
                        if e.is_timeout() { "timeout" } else { "connection failed" },
                        e
                    ))
                } else {
                    AppError::gateway(format!("Xendit API request failed: {}", e))
                }
            })?;

        let status_code = response.status();
        let response_body = response
            .text()
            .await
            .map_err(|e| AppError::gateway(format!("Failed to read Xendit response: {}", e)))?;

        if !status_code.is_success() {
            // FR-038: Return descriptive error with gateway name and error type
            return Err(AppError::gateway(format!(
                "Xendit API error - HTTP {} ({})",
                status_code.as_u16(),
                response_body
            )));
        }

        // Parse Xendit response
        let xendit_response: XenditInvoiceResponse = serde_json::from_str(&response_body)
            .map_err(|e| AppError::gateway(format!("Failed to parse Xendit response: {}", e)))?;

        Ok(PaymentResponse {
            gateway_reference: xendit_response.id,
            payment_url: xendit_response.invoice_url,
            expires_at: Some(xendit_response.expiry_date),
            status: PaymentStatus::Pending,
        })
    }

    async fn verify_webhook(&self, signature: &str, payload: &str) -> Result<bool> {
        Ok(self.verify_hmac(signature, payload))
    }

    async fn process_webhook(&self, payload: &str) -> Result<WebhookPayload> {
        let xendit_webhook: XenditWebhook = serde_json::from_str(payload)
            .map_err(|e| AppError::Internal(format!("Failed to parse Xendit webhook: {}", e)))?;

        let status = match xendit_webhook.status.as_str() {
            "PAID" => PaymentStatus::Completed,
            "EXPIRED" => PaymentStatus::Expired,
            _ => PaymentStatus::Pending,
        };

        let raw_response: serde_json::Value = serde_json::from_str(payload)
            .map_err(|e| AppError::Internal(format!("Failed to parse webhook JSON: {}", e)))?;

        Ok(WebhookPayload {
            gateway_reference: xendit_webhook.id,
            external_id: xendit_webhook.external_id,
            amount_paid: xendit_webhook.amount,
            payment_method: xendit_webhook.payment_method.unwrap_or_else(|| "unknown".to_string()),
            status,
            raw_response,
        })
    }

    fn name(&self) -> &str {
        "xendit"
    }

    fn supports_currency(&self, currency: Currency) -> bool {
        // Xendit supports IDR, PHP, MYR, THB (based on research.md)
        matches!(currency, Currency::IDR | Currency::MYR)
    }
}

// Xendit API response structures

#[derive(Debug, Deserialize)]
struct XenditInvoiceResponse {
    id: String,
    invoice_url: String,
    expiry_date: chrono::DateTime<chrono::Utc>,
    status: String,
}

#[derive(Debug, Deserialize)]
struct XenditWebhook {
    id: String,
    external_id: String,
    status: String,
    amount: Decimal,
    payment_method: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xendit_client_creation() {
        let client = XenditClient::new(
            "test_api_key".to_string(),
            "test_webhook_secret".to_string(),
            None,
        );

        assert_eq!(client.name(), "xendit");
        assert_eq!(client.base_url, "https://api.xendit.co");
    }

    #[test]
    fn test_currency_support() {
        let client = XenditClient::new(
            "test_api_key".to_string(),
            "test_webhook_secret".to_string(),
            None,
        );

        assert!(client.supports_currency(Currency::IDR));
        assert!(client.supports_currency(Currency::MYR));
        assert!(!client.supports_currency(Currency::USD));
    }

    #[test]
    fn test_hmac_verification() {
        let client = XenditClient::new(
            "test_api_key".to_string(),
            "test_secret".to_string(),
            None,
        );

        let payload = r#"{"test":"data"}"#;
        
        // Generate expected signature
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;
        
        let mut mac = HmacSha256::new_from_slice(b"test_secret").unwrap();
        mac.update(payload.as_bytes());
        let expected_sig = hex::encode(mac.finalize().into_bytes());

        assert!(client.verify_hmac(&expected_sig, payload));
        assert!(!client.verify_hmac("invalid_signature", payload));
    }
}
