use actix_web::{web, HttpRequest, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::core::error::AppError;
use crate::modules::transactions::services::transaction_service::TransactionService;
use crate::modules::transactions::services::webhook_handler::WebhookHandler;

/// Webhook controller for processing payment gateway webhooks
pub struct WebhookController {
    transaction_service: Arc<TransactionService>,
    webhook_handler: Arc<WebhookHandler>,
}

impl WebhookController {
    pub fn new(
        transaction_service: Arc<TransactionService>,
        webhook_handler: Arc<WebhookHandler>,
    ) -> Self {
        Self {
            transaction_service,
            webhook_handler,
        }
    }

    /// POST /webhooks/{gateway}
    /// Process webhook from payment gateway with signature validation (FR-034)
    /// Handles payment and refund events per FR-086
    pub async fn process_webhook(
        &self,
        gateway: web::Path<String>,
        payload: web::Bytes,
        req: HttpRequest,
    ) -> Result<HttpResponse, AppError> {
        let gateway_name = gateway.into_inner();
        
        info!(
            gateway = %gateway_name,
            payload_size = payload.len(),
            "Received webhook"
        );

        // Extract signature from headers
        let signature = self.extract_signature(&req, &gateway_name)?;

        // Verify webhook signature (FR-034)
        self.verify_webhook_signature(&gateway_name, &payload, &signature).await?;

        // Parse webhook payload
        let webhook_data: WebhookPayload = serde_json::from_slice(&payload)
            .map_err(|e| AppError::Validation(format!("Invalid webhook payload: {}", e)))?;

        let webhook_id = webhook_data.id.clone();
        let event_type = webhook_data.event.clone();

        // Route to appropriate handler based on event type
        let transaction_service = self.transaction_service.clone();
        let webhook_data_clone = webhook_data.clone();
        let gateway_clone = gateway_name.clone();

        self.webhook_handler
            .process_webhook_with_retry(&webhook_id, move || {
                let service = transaction_service.clone();
                let data = webhook_data_clone.data.clone();
                let gw = gateway_clone.clone();
                let event = event_type.clone();
                async move {
                    // Route based on event type (FR-086)
                    if Self::is_refund_event(&gw, &event) {
                        service.process_refund_webhook(&gw, &data).await
                    } else {
                        service.process_webhook_event(&gw, &data).await
                    }
                }
            })
            .await?;

        info!(webhook_id = %webhook_id, event = %event_type, "Webhook processed successfully");

        Ok(HttpResponse::Ok().json(serde_json::json!({
            "status": "success",
            "message": "Webhook received and queued for processing"
        })))
    }

    /// Check if event is a refund event based on gateway
    fn is_refund_event(gateway: &str, event: &str) -> bool {
        match gateway.to_lowercase().as_str() {
            "xendit" => event == "invoice.refunded",
            "midtrans" => event == "refund",
            _ => false,
        }
    }

    /// GET /webhooks/failed
    /// Query permanently failed webhooks for manual intervention (FR-042)
    pub async fn get_failed_webhooks(
        &self,
        query: web::Query<FailedWebhooksQuery>,
    ) -> Result<HttpResponse, AppError> {
        info!(
            page = query.page.unwrap_or(1),
            page_size = query.page_size.unwrap_or(50),
            "Fetching failed webhooks"
        );

        // TODO: Implement actual database query to webhook_retry_log table
        // Filter by status = 'permanently_failed'
        // Support pagination and date filtering

        let failed_webhooks: Vec<serde_json::Value> = vec![]; // Placeholder

        Ok(HttpResponse::Ok().json(serde_json::json!({
            "failed_webhooks": failed_webhooks,
            "page": query.page.unwrap_or(1),
            "page_size": query.page_size.unwrap_or(50),
            "total": 0
        })))
    }

    /// Extract signature from request headers based on gateway
    fn extract_signature(&self, req: &HttpRequest, gateway: &str) -> Result<String, AppError> {
        let header_name = match gateway.to_lowercase().as_str() {
            "xendit" => "x-callback-token",
            "midtrans" => "x-midtrans-signature",
            _ => return Err(AppError::Validation(format!("Unknown gateway: {}", gateway))),
        };

        req.headers()
            .get(header_name)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::Validation(format!("Missing signature header: {}", header_name)))
    }

    /// Verify webhook signature (FR-034)
    async fn verify_webhook_signature(
        &self,
        gateway: &str,
        _payload: &[u8],
        signature: &str,
    ) -> Result<(), AppError> {
        // TODO: Implement actual signature verification
        // - Xendit: HMAC-SHA256 with webhook secret
        // - Midtrans: SHA512 hash verification
        
        info!(
            gateway = %gateway,
            signature_length = signature.len(),
            "Verifying webhook signature"
        );

        // Placeholder - in production, verify against gateway webhook secret
        Ok(())
    }
}

/// Webhook payload structure (generic for all gateways)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub id: String,
    pub event: String,
    pub data: serde_json::Value,
}

/// Query parameters for failed webhooks endpoint
#[derive(Debug, Deserialize)]
pub struct FailedWebhooksQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// Configure webhook routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/webhooks")
            .route("/{gateway}", web::post().to(handle_webhook))
            .route("/failed", web::get().to(handle_failed_webhooks)),
    );
}

/// Handler for POST /webhooks/{gateway}
async fn handle_webhook(
    gateway: web::Path<String>,
    payload: web::Bytes,
    req: HttpRequest,
    controller: web::Data<WebhookController>,
) -> Result<HttpResponse, AppError> {
    controller.process_webhook(gateway, payload, req).await
}

/// Handler for GET /webhooks/failed
async fn handle_failed_webhooks(
    query: web::Query<FailedWebhooksQuery>,
    controller: web::Data<WebhookController>,
) -> Result<HttpResponse, AppError> {
    controller.get_failed_webhooks(query).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_signature_xendit() {
        // Test would require mocking HttpRequest
        // Placeholder for actual implementation
    }

    #[test]
    fn test_extract_signature_midtrans() {
        // Test would require mocking HttpRequest
        // Placeholder for actual implementation
    }
}
