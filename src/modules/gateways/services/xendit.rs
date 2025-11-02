use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::core::error::{AppError, AppResult};
use super::gateway_trait::{PaymentGateway, PaymentRequest, PaymentResponse, WebhookVerification};

/// Xendit payment gateway client
pub struct XenditGateway {
    client: Client,
    api_key: String,
    base_url: String,
    webhook_token: String,
}

impl XenditGateway {
    pub fn new(api_key: String, webhook_token: String, is_production: bool) -> Self {
        let base_url = if is_production {
            "https://api.xendit.co".to_string()
        } else {
            "https://api.xendit.co".to_string() // Xendit uses same URL for sandbox
        };

        Self {
            client: Client::new(),
            api_key,
            base_url,
            webhook_token,
        }
    }
}

#[async_trait]
impl PaymentGateway for XenditGateway {
    async fn create_payment(&self, request: PaymentRequest) -> AppResult<PaymentResponse> {
        // Xendit Invoice API: https://developers.xendit.co/api-reference/#create-invoice
        let url = format!("{}/v2/invoices", self.base_url);

        #[derive(Serialize)]
        struct XenditInvoiceRequest {
            external_id: String,
            amount: String,
            description: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            customer_email: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            customer_name: Option<String>,
            currency: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            success_redirect_url: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            failure_redirect_url: Option<String>,
        }

        #[derive(Deserialize)]
        struct XenditInvoiceResponse {
            id: String,
            invoice_url: String,
            status: String,
            created: String,
        }

        let xendit_request = XenditInvoiceRequest {
            external_id: request.external_id.clone(),
            amount: request.amount.to_string(),
            description: request.description,
            customer_email: request.customer_email,
            customer_name: request.customer_name,
            currency: request.currency,
            success_redirect_url: request.success_redirect_url,
            failure_redirect_url: request.failure_redirect_url,
        };

        let response = self
            .client
            .post(&url)
            .basic_auth(&self.api_key, Some(""))
            .json(&xendit_request)
            .send()
            .await
            .map_err(|e| AppError::Gateway(format!("Xendit API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(AppError::Gateway(format!(
                "Xendit API error {}: {}",
                status, error_body
            )));
        }

        let xendit_response: XenditInvoiceResponse = response
            .json()
            .await
            .map_err(|e| AppError::Gateway(format!("Failed to parse Xendit response: {}", e)))?;

        // Parse created timestamp
        let created_at = chrono::DateTime::parse_from_rfc3339(&xendit_response.created)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(PaymentResponse {
            gateway_reference: xendit_response.id,
            payment_url: xendit_response.invoice_url,
            status: xendit_response.status,
            created_at,
        })
    }

    async fn verify_webhook(
        &self,
        signature: &str,
        payload: &str,
    ) -> AppResult<WebhookVerification> {
        // Xendit webhook verification using callback token
        // https://developers.xendit.co/api-reference/#invoice-callback
        
        // Verify signature (Xendit uses x-callback-token header)
        if signature != self.webhook_token {
            return Err(AppError::Validation(
                "Invalid webhook signature".to_string(),
            ));
        }

        // Parse webhook payload
        #[derive(Deserialize)]
        struct XenditWebhook {
            id: String,
            status: String,
            amount: String,
            #[serde(default)]
            payment_method: String,
        }

        let webhook: XenditWebhook = serde_json::from_str(payload)
            .map_err(|e| AppError::Validation(format!("Invalid webhook payload: {}", e)))?;

        let amount_paid = webhook
            .amount
            .parse::<Decimal>()
            .map_err(|e| AppError::Validation(format!("Invalid amount: {}", e)))?;

        let raw_payload: serde_json::Value = serde_json::from_str(payload)
            .map_err(|e| AppError::Validation(format!("Invalid JSON payload: {}", e)))?;

        Ok(WebhookVerification {
            is_valid: true,
            gateway_reference: webhook.id,
            status: webhook.status,
            amount_paid,
            payment_method: webhook.payment_method,
            raw_payload,
        })
    }

    fn name(&self) -> &str {
        "xendit"
    }

    fn supported_currencies(&self) -> Vec<String> {
        vec!["IDR".to_string(), "PHP".to_string(), "THB".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xendit_gateway_creation() {
        let gateway = XenditGateway::new(
            "test_key".to_string(),
            "test_token".to_string(),
            false,
        );
        assert_eq!(gateway.name(), "xendit");
        assert!(gateway.supported_currencies().contains(&"IDR".to_string()));
    }
}
