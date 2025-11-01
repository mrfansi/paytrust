use super::super::services::{WebhookHandler, WebhookResult};
use crate::core::{AppError, Result};
use actix_web::{post, web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info};

/// Webhook controller for handling payment gateway webhooks (FR-034)
///
/// Provides endpoints for:
/// - Receiving and processing payment gateway webhooks
/// - Signature verification
/// - Idempotent webhook processing
pub struct WebhookController {
    webhook_handler: WebhookHandler,
}

impl WebhookController {
    /// Create a new WebhookController
    ///
    /// # Arguments
    /// * `webhook_handler` - Webhook handler service
    pub fn new(webhook_handler: WebhookHandler) -> Self {
        Self { webhook_handler }
    }

    /// Configure webhook routes
    ///
    /// # Arguments
    /// * `cfg` - Service configuration
    pub fn configure(cfg: &mut web::ServiceConfig, webhook_handler: WebhookHandler) {
        let controller = web::Data::new(Self::new(webhook_handler));
        
        cfg.service(
            web::scope("/webhooks")
                .app_data(controller)
                .service(process_webhook),
        );
    }
}

/// Webhook request structure
#[derive(Debug, Deserialize)]
pub struct WebhookRequest {
    #[serde(flatten)]
    pub payload: Value,
}

/// Webhook response structure
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum WebhookResponse {
    Success {
        transaction_id: String,
        gateway_ref: String,
    },
    Duplicate {
        transaction_id: String,
        gateway_ref: String,
        message: String,
    },
    Failed {
        gateway_ref: String,
        error: String,
    },
}

/// Process a payment gateway webhook (FR-034)
///
/// POST /webhooks/{gateway}
///
/// Processes incoming webhooks from payment gateways with:
/// - Signature verification (FR-034)
/// - Idempotency checks (FR-032)
/// - Retry logic with exponential backoff (FR-042, FR-043)
///
/// # Path Parameters
/// * `gateway` - Gateway identifier ("xendit" or "midtrans")
///
/// # Headers
/// * `X-Callback-Token` (Xendit) or `Authorization` (Midtrans) - Webhook signature
///
/// # Request Body
/// * Gateway-specific webhook payload (JSON)
///
/// # Returns
/// * `200 OK` - Webhook processed successfully or duplicate detected
/// * `400 Bad Request` - Invalid payload or signature verification failed
/// * `500 Internal Server Error` - Processing failed after all retries
#[post("/{gateway}")]
async fn process_webhook(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<WebhookRequest>,
    controller: web::Data<WebhookController>,
) -> Result<HttpResponse> {
    let gateway_id = path.into_inner();

    info!(
        gateway_id = gateway_id.as_str(),
        "Received webhook request"
    );

    // Extract signature from headers based on gateway
    let signature = extract_signature(&req, &gateway_id)?;

    // Process webhook with retry logic
    let result = controller
        .webhook_handler
        .process_webhook(&gateway_id, &signature, &body.payload)
        .await?;

    // Convert result to HTTP response
    let response = match result {
        WebhookResult::Success {
            transaction_id,
            gateway_ref,
        } => {
            info!(
                transaction_id = transaction_id,
                gateway_ref = gateway_ref,
                "Webhook processed successfully"
            );
            WebhookResponse::Success {
                transaction_id,
                gateway_ref,
            }
        }
        WebhookResult::Duplicate {
            transaction_id,
            gateway_ref,
        } => {
            info!(
                transaction_id = transaction_id,
                gateway_ref = gateway_ref,
                "Webhook is duplicate (already processed)"
            );
            WebhookResponse::Duplicate {
                transaction_id,
                gateway_ref,
                message: "Webhook already processed".to_string(),
            }
        }
        WebhookResult::Failed { gateway_ref, error } => {
            error!(
                gateway_ref = gateway_ref,
                error = error,
                "Webhook processing failed after all retries"
            );
            return Ok(HttpResponse::InternalServerError().json(WebhookResponse::Failed {
                gateway_ref,
                error,
            }));
        }
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Extract webhook signature from request headers
///
/// # Arguments
/// * `req` - HTTP request
/// * `gateway_id` - Gateway identifier
///
/// # Returns
/// * `Result<String>` - Extracted signature
fn extract_signature(req: &HttpRequest, gateway_id: &str) -> Result<String> {
    match gateway_id {
        "xendit" => {
            // Xendit sends signature in X-Callback-Token header
            req.headers()
                .get("X-Callback-Token")
                .and_then(|h| h.to_str().ok())
                .map(String::from)
                .ok_or_else(|| {
                    AppError::validation("Missing X-Callback-Token header for Xendit webhook")
                })
        }
        "midtrans" => {
            // Midtrans signature can be verified from order_id, status_code, gross_amount
            // For now, we'll accept a signature header, but in practice Midtrans
            // verifies by reconstructing the signature from payload fields
            req.headers()
                .get("Authorization")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.trim_start_matches("Bearer ").to_string())
                .or_else(|| Some(String::new())) // Midtrans doesn't use header signature
                .ok_or_else(|| AppError::validation("Missing Authorization header"))
        }
        _ => Err(AppError::validation(format!(
            "Unsupported gateway: {}",
            gateway_id
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    #[tokio::test]
    async fn test_webhook_response_success() {
        let response = WebhookResponse::Success {
            transaction_id: "txn-123".to_string(),
            gateway_ref: "gateway-456".to_string(),
        };

        match response {
            WebhookResponse::Success {
                transaction_id,
                gateway_ref,
            } => {
                assert_eq!(transaction_id, "txn-123");
                assert_eq!(gateway_ref, "gateway-456");
            }
            _ => panic!("Expected Success variant"),
        }
    }

    #[tokio::test]
    async fn test_webhook_response_duplicate() {
        let response = WebhookResponse::Duplicate {
            transaction_id: "txn-existing".to_string(),
            gateway_ref: "gateway-789".to_string(),
            message: "Already processed".to_string(),
        };

        match response {
            WebhookResponse::Duplicate {
                transaction_id,
                gateway_ref,
                message,
            } => {
                assert_eq!(transaction_id, "txn-existing");
                assert_eq!(gateway_ref, "gateway-789");
                assert_eq!(message, "Already processed");
            }
            _ => panic!("Expected Duplicate variant"),
        }
    }

    #[tokio::test]
    async fn test_webhook_response_failed() {
        let response = WebhookResponse::Failed {
            gateway_ref: "gateway-999".to_string(),
            error: "Processing error".to_string(),
        };

        match response {
            WebhookResponse::Failed { gateway_ref, error } => {
                assert_eq!(gateway_ref, "gateway-999");
                assert_eq!(error, "Processing error");
            }
            _ => panic!("Expected Failed variant"),
        }
    }

    #[tokio::test]
    async fn test_extract_xendit_signature() {
        let req = test::TestRequest::default()
            .insert_header(("X-Callback-Token", "test-signature-xendit"))
            .to_http_request();

        let result = extract_signature(&req, "xendit");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-signature-xendit");
    }

    #[tokio::test]
    async fn test_extract_midtrans_signature() {
        let req = test::TestRequest::default()
            .insert_header(("Authorization", "Bearer test-signature-midtrans"))
            .to_http_request();

        let result = extract_signature(&req, "midtrans");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-signature-midtrans");
    }

    #[tokio::test]
    async fn test_missing_signature_header() {
        let req = test::TestRequest::default().to_http_request();

        let result = extract_signature(&req, "xendit");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("X-Callback-Token"));
    }

    #[tokio::test]
    async fn test_unsupported_gateway() {
        let req = test::TestRequest::default().to_http_request();

        let result = extract_signature(&req, "unsupported");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported gateway"));
    }
}
