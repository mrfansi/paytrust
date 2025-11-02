use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Sha512, Digest};

use crate::core::error::{AppError, AppResult};
use super::gateway_trait::{PaymentGateway, PaymentRequest, PaymentResponse, WebhookVerification};

/// Midtrans payment gateway client
pub struct MidtransGateway {
    client: Client,
    server_key: String,
    base_url: String,
}

impl MidtransGateway {
    pub fn new(server_key: String, is_production: bool) -> Self {
        let base_url = if is_production {
            "https://app.midtrans.com".to_string()
        } else {
            "https://app.sandbox.midtrans.com".to_string()
        };

        Self {
            client: Client::new(),
            server_key,
            base_url,
        }
    }

    /// Generate signature hash for webhook verification
    fn generate_signature(&self, order_id: &str, status_code: &str, gross_amount: &str) -> String {
        let signature_string = format!("{}{}{}{}", order_id, status_code, gross_amount, self.server_key);
        let mut hasher = Sha512::new();
        hasher.update(signature_string.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[async_trait]
impl PaymentGateway for MidtransGateway {
    async fn create_payment(&self, request: PaymentRequest) -> AppResult<PaymentResponse> {
        // Midtrans Snap API: https://docs.midtrans.com/reference/createtransaction
        let url = format!("{}/snap/v1/transactions", self.base_url);

        #[derive(Serialize)]
        struct MidtransTransactionRequest {
            transaction_details: TransactionDetails,
            #[serde(skip_serializing_if = "Option::is_none")]
            customer_details: Option<CustomerDetails>,
        }

        #[derive(Serialize)]
        struct TransactionDetails {
            order_id: String,
            gross_amount: i64,
        }

        #[derive(Serialize)]
        struct CustomerDetails {
            #[serde(skip_serializing_if = "Option::is_none")]
            email: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            first_name: Option<String>,
        }

        #[derive(Deserialize)]
        struct MidtransTransactionResponse {
            token: String,
            redirect_url: String,
        }

        // Convert amount to integer (Midtrans doesn't support decimals)
        let gross_amount = request.amount.round().to_string().parse::<i64>()
            .map_err(|e| AppError::Validation(format!("Invalid amount: {}", e)))?;

        let customer_details = if request.customer_email.is_some() || request.customer_name.is_some() {
            Some(CustomerDetails {
                email: request.customer_email,
                first_name: request.customer_name,
            })
        } else {
            None
        };

        let midtrans_request = MidtransTransactionRequest {
            transaction_details: TransactionDetails {
                order_id: request.external_id.clone(),
                gross_amount,
            },
            customer_details,
        };

        // Midtrans uses Basic Auth with server_key as username and empty password
        let auth_value = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            format!("{}:", self.server_key).as_bytes()
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Basic {}", auth_value))
            .header("Content-Type", "application/json")
            .json(&midtrans_request)
            .send()
            .await
            .map_err(|e| AppError::Gateway(format!("Midtrans API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(AppError::Gateway(format!(
                "Midtrans API error {}: {}",
                status, error_body
            )));
        }

        let midtrans_response: MidtransTransactionResponse = response
            .json()
            .await
            .map_err(|e| AppError::Gateway(format!("Failed to parse Midtrans response: {}", e)))?;

        Ok(PaymentResponse {
            gateway_reference: midtrans_response.token,
            payment_url: midtrans_response.redirect_url,
            status: "pending".to_string(),
            created_at: Utc::now(),
        })
    }

    async fn verify_webhook(
        &self,
        signature: &str,
        payload: &str,
    ) -> AppResult<WebhookVerification> {
        // Midtrans webhook verification using SHA512 signature
        // https://docs.midtrans.com/docs/http-notification-webhooks
        
        // Parse webhook payload
        #[derive(Deserialize)]
        struct MidtransWebhook {
            order_id: String,
            status_code: String,
            gross_amount: String,
            transaction_status: String,
            #[serde(default)]
            payment_type: String,
        }

        let webhook: MidtransWebhook = serde_json::from_str(payload)
            .map_err(|e| AppError::Validation(format!("Invalid webhook payload: {}", e)))?;

        // Verify signature
        let expected_signature = self.generate_signature(
            &webhook.order_id,
            &webhook.status_code,
            &webhook.gross_amount,
        );

        if signature != expected_signature {
            return Err(AppError::Validation(
                "Invalid webhook signature".to_string(),
            ));
        }

        let amount_paid = webhook
            .gross_amount
            .parse::<Decimal>()
            .map_err(|e| AppError::Validation(format!("Invalid amount: {}", e)))?;

        let raw_payload: serde_json::Value = serde_json::from_str(payload)
            .map_err(|e| AppError::Validation(format!("Invalid JSON payload: {}", e)))?;

        Ok(WebhookVerification {
            is_valid: true,
            gateway_reference: webhook.order_id,
            status: webhook.transaction_status,
            amount_paid,
            payment_method: webhook.payment_type,
            raw_payload,
        })
    }

    fn name(&self) -> &str {
        "midtrans"
    }

    fn supported_currencies(&self) -> Vec<String> {
        vec!["IDR".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midtrans_gateway_creation() {
        let gateway = MidtransGateway::new("test_key".to_string(), false);
        assert_eq!(gateway.name(), "midtrans");
        assert!(gateway.supported_currencies().contains(&"IDR".to_string()));
    }

    #[test]
    fn test_signature_generation() {
        let gateway = MidtransGateway::new("test_server_key".to_string(), false);
        let signature = gateway.generate_signature("order-123", "200", "100000");
        assert!(!signature.is_empty());
        assert_eq!(signature.len(), 128); // SHA512 produces 128 hex characters
    }
}
