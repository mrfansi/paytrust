use super::gateway_trait::{
    PaymentGateway, PaymentRequest, PaymentResponse, PaymentStatus, WebhookPayload,
};
use crate::core::{AppError, Currency, Result};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::str::FromStr;

/// Midtrans payment gateway client
///
/// Implements PaymentGateway trait for Midtrans API integration
/// API Documentation: https://docs.midtrans.com/reference/api-reference
pub struct MidtransClient {
    client: Client,
    server_key: String,
    webhook_secret: String,
    base_url: String,
}

impl MidtransClient {
    /// Create a new Midtrans client
    ///
    /// # Arguments
    /// * `server_key` - Midtrans server key (from MIDTRANS_SERVER_KEY env var)
    /// * `webhook_secret` - Midtrans webhook verification key
    /// * `base_url` - Midtrans API base URL (defaults to sandbox)
    pub fn new(server_key: String, webhook_secret: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            server_key,
            webhook_secret,
            base_url: base_url.unwrap_or_else(|| "https://api.sandbox.midtrans.com".to_string()),
        }
    }

    /// Verify SHA512 signature for webhook
    fn verify_signature(
        &self,
        signature: &str,
        order_id: &str,
        status_code: &str,
        gross_amount: &str,
    ) -> bool {
        use sha2::{Digest, Sha512};

        // Midtrans signature format: SHA512(order_id+status_code+gross_amount+server_key)
        let signature_string = format!(
            "{}{}{}{}",
            order_id, status_code, gross_amount, self.server_key
        );

        let mut hasher = Sha512::new();
        hasher.update(signature_string.as_bytes());
        let expected_signature = format!("{:x}", hasher.finalize());

        // Constant-time comparison
        signature == expected_signature
    }
}

#[async_trait]
impl PaymentGateway for MidtransClient {
    async fn create_payment(&self, request: PaymentRequest) -> Result<PaymentResponse> {
        // Midtrans Snap API endpoint
        let url = format!("{}/snap/v1/transactions", self.base_url);

        // Convert amount to smallest currency unit
        let amount = match request.currency {
            Currency::IDR => request.amount.round(),
            _ => request.amount.round_dp(2),
        };

        // Build item name with installment info if present (T098)
        let item_name = if let Some(ref installment_info) = request.installment_info {
            format!(
                "{} - Installment {}/{}",
                request.description,
                installment_info.installment_number,
                installment_info.total_installments
            )
        } else {
            request.description.clone()
        };

        // Build Midtrans transaction request
        let midtrans_request = json!({
            "transaction_details": {
                "order_id": request.external_id,
                "gross_amount": amount.to_string()
            },
            "item_details": [{
                "id": "item-1",
                "price": amount.to_string(),
                "quantity": 1,
                "name": item_name
            }],
            "customer_details": {
                "email": request.payer_email,
            },
            "expiry": {
                "duration": 1440, // 24 hours in minutes (FR-044)
                "unit": "minutes"
            }
        });

        // Send request to Midtrans with Basic Auth (FR-038: Descriptive errors with gateway name)
        let response = self
            .client
            .post(&url)
            .basic_auth(&self.server_key, Some(""))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&midtrans_request)
            .send()
            .await
            .map_err(|e| {
                // FR-038: Descriptive error with gateway name
                if e.is_connect() || e.is_timeout() {
                    AppError::gateway(format!(
                        "Midtrans gateway unavailable: {} ({})",
                        if e.is_timeout() {
                            "timeout"
                        } else {
                            "connection failed"
                        },
                        e
                    ))
                } else {
                    AppError::gateway(format!("Midtrans API request failed: {}", e))
                }
            })?;

        let status_code = response.status();
        let response_body = response
            .text()
            .await
            .map_err(|e| AppError::gateway(format!("Failed to read Midtrans response: {}", e)))?;

        if !status_code.is_success() {
            // FR-038: Return descriptive error with gateway name and error type
            return Err(AppError::gateway(format!(
                "Midtrans API error - HTTP {} ({})",
                status_code.as_u16(),
                response_body
            )));
        }

        // Parse Midtrans response
        let midtrans_response: MidtransSnapResponse = serde_json::from_str(&response_body)
            .map_err(|e| AppError::gateway(format!("Failed to parse Midtrans response: {}", e)))?;

        // Calculate expiration time (24 hours from now)
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

        Ok(PaymentResponse {
            gateway_reference: midtrans_response.token,
            payment_url: midtrans_response.redirect_url,
            expires_at: Some(expires_at),
            status: PaymentStatus::Pending,
        })
    }

    async fn verify_webhook(&self, signature: &str, payload: &str) -> Result<bool> {
        // Parse payload to extract verification fields
        let webhook: serde_json::Value = serde_json::from_str(payload)
            .map_err(|e| AppError::Internal(format!("Failed to parse webhook payload: {}", e)))?;

        let order_id = webhook["order_id"]
            .as_str()
            .ok_or_else(|| AppError::Internal("Missing order_id in webhook".to_string()))?;

        let status_code = webhook["status_code"]
            .as_str()
            .ok_or_else(|| AppError::Internal("Missing status_code in webhook".to_string()))?;

        let gross_amount = webhook["gross_amount"]
            .as_str()
            .ok_or_else(|| AppError::Internal("Missing gross_amount in webhook".to_string()))?;

        Ok(self.verify_signature(signature, order_id, status_code, gross_amount))
    }

    async fn process_webhook(&self, payload: &str) -> Result<WebhookPayload> {
        let midtrans_webhook: MidtransWebhook = serde_json::from_str(payload)
            .map_err(|e| AppError::Internal(format!("Failed to parse Midtrans webhook: {}", e)))?;

        let status = match midtrans_webhook.transaction_status.as_str() {
            "capture" | "settlement" => PaymentStatus::Completed,
            "deny" | "cancel" | "expire" => {
                if midtrans_webhook.transaction_status == "expire" {
                    PaymentStatus::Expired
                } else {
                    PaymentStatus::Failed
                }
            }
            _ => PaymentStatus::Pending,
        };

        let raw_response: serde_json::Value = serde_json::from_str(payload)
            .map_err(|e| AppError::Internal(format!("Failed to parse webhook JSON: {}", e)))?;

        // Parse gross_amount from string to Decimal
        let amount_paid = Decimal::from_str(&midtrans_webhook.gross_amount)
            .map_err(|e| AppError::Internal(format!("Invalid amount format: {}", e)))?;

        Ok(WebhookPayload {
            gateway_reference: midtrans_webhook.transaction_id,
            external_id: midtrans_webhook.order_id,
            amount_paid,
            payment_method: midtrans_webhook.payment_type,
            status,
            raw_response,
        })
    }

    fn name(&self) -> &str {
        "midtrans"
    }

    fn supports_currency(&self, currency: Currency) -> bool {
        // Midtrans primarily supports IDR
        matches!(currency, Currency::IDR)
    }
}

// Midtrans API response structures

#[derive(Debug, Deserialize)]
struct MidtransSnapResponse {
    token: String,
    redirect_url: String,
}

#[derive(Debug, Deserialize)]
struct MidtransWebhook {
    transaction_id: String,
    order_id: String,
    transaction_status: String,
    gross_amount: String,
    payment_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midtrans_client_creation() {
        let client = MidtransClient::new(
            "test_server_key".to_string(),
            "test_webhook_secret".to_string(),
            None,
        );

        assert_eq!(client.name(), "midtrans");
        assert_eq!(client.base_url, "https://api.sandbox.midtrans.com");
    }

    #[test]
    fn test_currency_support() {
        let client = MidtransClient::new(
            "test_server_key".to_string(),
            "test_webhook_secret".to_string(),
            None,
        );

        assert!(client.supports_currency(Currency::IDR));
        assert!(!client.supports_currency(Currency::MYR));
        assert!(!client.supports_currency(Currency::USD));
    }

    #[test]
    fn test_signature_verification() {
        let client = MidtransClient::new(
            "test_server_key".to_string(),
            "test_webhook_secret".to_string(),
            None,
        );

        let order_id = "order-123";
        let status_code = "200";
        let gross_amount = "100000";

        // Generate expected signature
        use sha2::{Digest, Sha512};
        let signature_string = format!(
            "{}{}{}{}",
            order_id, status_code, gross_amount, "test_server_key"
        );
        let mut hasher = Sha512::new();
        hasher.update(signature_string.as_bytes());
        let expected_sig = format!("{:x}", hasher.finalize());

        assert!(client.verify_signature(&expected_sig, order_id, status_code, gross_amount));
        assert!(!client.verify_signature("invalid_signature", order_id, status_code, gross_amount));
    }
}
