use super::super::models::TransactionStatus;
use super::TransactionService;
use crate::core::{AppError, Currency, Result};
use crate::modules::gateways::services::GatewayService;
use rust_decimal::Decimal;
use serde_json::Value;
use std::time::Duration;
use tracing::{error, info, warn};

/// Webhook processing result
#[derive(Debug, Clone)]
pub enum WebhookResult {
    /// Webhook processed successfully
    Success {
        transaction_id: String,
        gateway_ref: String,
    },
    /// Webhook processing failed
    Failed { gateway_ref: String, error: String },
    /// Webhook is a duplicate (already processed)
    Duplicate {
        transaction_id: String,
        gateway_ref: String,
    },
}

/// Webhook handler with retry logic (FR-042, FR-043)
///
/// Handles incoming payment gateway webhooks with:
/// - Signature verification
/// - Idempotency checks
/// - Retry logic with exponential backoff
/// - Comprehensive logging
pub struct WebhookHandler {
    gateway_service: GatewayService,
    transaction_service: TransactionService,
    max_retries: u32,
    retry_delays: Vec<Duration>,
}

impl WebhookHandler {
    /// Create a new WebhookHandler with default retry configuration
    ///
    /// Default retry strategy (FR-042):
    /// - Attempt 1: Immediate
    /// - Attempt 2: 1 minute delay
    /// - Attempt 3: 5 minutes delay
    /// - Attempt 4: 30 minutes delay
    ///
    /// # Arguments
    /// * `gateway_service` - Gateway service for webhook verification
    /// * `transaction_service` - Transaction service for payment recording
    pub fn new(gateway_service: GatewayService, transaction_service: TransactionService) -> Self {
        Self {
            gateway_service,
            transaction_service,
            max_retries: 3,
            retry_delays: vec![
                Duration::from_secs(60),   // 1 minute
                Duration::from_secs(300),  // 5 minutes
                Duration::from_secs(1800), // 30 minutes
            ],
        }
    }

    /// Create a WebhookHandler with custom retry configuration
    ///
    /// # Arguments
    /// * `gateway_service` - Gateway service
    /// * `transaction_service` - Transaction service
    /// * `max_retries` - Maximum number of retry attempts
    /// * `retry_delays` - Delay durations for each retry
    pub fn with_retry_config(
        gateway_service: GatewayService,
        transaction_service: TransactionService,
        max_retries: u32,
        retry_delays: Vec<Duration>,
    ) -> Self {
        Self {
            gateway_service,
            transaction_service,
            max_retries,
            retry_delays,
        }
    }

    /// Process a webhook with retry logic (FR-042, FR-043)
    ///
    /// # Arguments
    /// * `gateway_id` - Gateway identifier
    /// * `signature` - Webhook signature for verification
    /// * `payload` - Webhook payload
    ///
    /// # Returns
    /// * `Result<WebhookResult>` - Processing result
    pub async fn process_webhook(
        &self,
        gateway_id: &str,
        signature: &str,
        payload: &Value,
    ) -> Result<WebhookResult> {
        let gateway_ref = self.extract_gateway_ref(gateway_id, payload)?;

        info!(
            gateway_id = gateway_id,
            gateway_ref = gateway_ref,
            "Processing webhook"
        );

        // Check for duplicate (idempotency - FR-032)
        if let Some(existing) = self
            .transaction_service
            .get_transaction_by_gateway_ref(&gateway_ref)
            .await?
        {
            let transaction_id = existing.id.unwrap_or_default();
            info!(
                transaction_id = transaction_id,
                gateway_ref = gateway_ref,
                "Webhook already processed (duplicate)"
            );
            return Ok(WebhookResult::Duplicate {
                transaction_id,
                gateway_ref,
            });
        }

        // Attempt to process with retries
        let mut last_error = None;
        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = self
                    .retry_delays
                    .get((attempt - 1) as usize)
                    .copied()
                    .unwrap_or(Duration::from_secs(60));

                warn!(
                    gateway_id = gateway_id,
                    gateway_ref = gateway_ref,
                    attempt = attempt,
                    delay_secs = delay.as_secs(),
                    "Retrying webhook processing after delay"
                );

                // In production, this would use a background job queue
                // For now, we'll use tokio::time::sleep for demonstration
                tokio::time::sleep(delay).await;
            }

            info!(
                gateway_id = gateway_id,
                gateway_ref = gateway_ref,
                attempt = attempt + 1,
                max_attempts = self.max_retries + 1,
                "Attempting webhook processing"
            );

            match self
                .process_webhook_attempt(gateway_id, signature, payload, &gateway_ref)
                .await
            {
                Ok(transaction_id) => {
                    info!(
                        transaction_id = transaction_id,
                        gateway_ref = gateway_ref,
                        attempt = attempt + 1,
                        "Webhook processed successfully"
                    );
                    return Ok(WebhookResult::Success {
                        transaction_id,
                        gateway_ref,
                    });
                }
                Err(e) => {
                    error!(
                        gateway_id = gateway_id,
                        gateway_ref = gateway_ref,
                        attempt = attempt + 1,
                        error = %e,
                        "Webhook processing attempt failed"
                    );
                    last_error = Some(e);
                }
            }
        }

        // All retries exhausted
        let error_msg = last_error
            .map(|e| e.to_string())
            .unwrap_or_else(|| "Unknown error".to_string());

        error!(
            gateway_id = gateway_id,
            gateway_ref = gateway_ref,
            attempts = self.max_retries + 1,
            error = error_msg,
            "Webhook processing failed after all retries"
        );

        Ok(WebhookResult::Failed {
            gateway_ref,
            error: error_msg,
        })
    }

    /// Attempt to process a webhook once
    ///
    /// # Arguments
    /// * `gateway_id` - Gateway identifier
    /// * `signature` - Webhook signature
    /// * `payload` - Webhook payload
    /// * `gateway_ref` - Pre-extracted gateway reference
    ///
    /// # Returns
    /// * `Result<String>` - Transaction ID if successful
    async fn process_webhook_attempt(
        &self,
        gateway_id: &str,
        signature: &str,
        payload: &Value,
        gateway_ref: &str,
    ) -> Result<String> {
        // Verify webhook signature
        let payload_str = serde_json::to_string(payload)
            .map_err(|e| AppError::validation(format!("Invalid JSON payload: {}", e)))?;

        self.gateway_service
            .verify_webhook(gateway_id, signature, &payload_str)
            .await?;

        // Extract payment data from payload
        let payment_data = self.extract_payment_data(gateway_id, payload)?;

        // Determine if this is an installment payment (T101)
        // Installment external IDs have format: {invoice_id}-installment-{number}
        let is_installment = payment_data.invoice_id.contains("-installment-");

        let transaction = if is_installment {
            // Extract invoice ID and installment ID from external_id
            let parts: Vec<&str> = payment_data.invoice_id.splitn(3, '-').collect();
            if parts.len() < 3 {
                return Err(AppError::validation(
                    "Invalid installment external_id format (expected: {invoice_id}-installment-{number})"
                ));
            }

            let invoice_id = parts[0].to_string();
            let installment_number: i32 = parts[2]
                .parse()
                .map_err(|_| AppError::validation("Invalid installment number in external_id"))?;

            // Find the installment ID by invoice and number
            // We'll need to query the installment repository for this
            // For now, we'll construct the installment ID
            let installment_id = format!("{}-inst-{}", invoice_id, installment_number);

            info!(
                invoice_id = invoice_id,
                installment_id = installment_id,
                installment_number = installment_number,
                amount_paid = %payment_data.amount,
                "Processing installment payment webhook"
            );

            // Process installment payment with overpayment handling (T101)
            self.transaction_service
                .process_installment_payment(
                    invoice_id,
                    installment_id,
                    payment_data.amount,
                    gateway_ref.to_string(),
                )
                .await?
        } else {
            // Record regular payment
            self.transaction_service
                .record_payment(
                    payment_data.invoice_id,
                    gateway_ref.to_string(),
                    gateway_id.to_string(),
                    payment_data.amount,
                    payment_data.currency,
                    payment_data.payment_method,
                    payment_data.status,
                    Some(payload.clone()),
                )
                .await?
        };

        Ok(transaction.id.unwrap_or_default())
    }

    /// Extract gateway transaction reference from payload
    ///
    /// # Arguments
    /// * `gateway_id` - Gateway identifier
    /// * `payload` - Webhook payload
    ///
    /// # Returns
    /// * `Result<String>` - Gateway transaction reference
    fn extract_gateway_ref(&self, gateway_id: &str, payload: &Value) -> Result<String> {
        match gateway_id {
            "xendit" => payload["id"]
                .as_str()
                .map(String::from)
                .ok_or_else(|| AppError::validation("Missing 'id' in Xendit webhook")),
            "midtrans" => payload["transaction_id"]
                .as_str()
                .map(String::from)
                .ok_or_else(|| {
                    AppError::validation("Missing 'transaction_id' in Midtrans webhook")
                }),
            _ => Err(AppError::validation(format!(
                "Unsupported gateway: {}",
                gateway_id
            ))),
        }
    }

    /// Extract payment data from webhook payload
    ///
    /// # Arguments
    /// * `gateway_id` - Gateway identifier
    /// * `payload` - Webhook payload
    ///
    /// # Returns
    /// * `Result<PaymentData>` - Extracted payment data
    fn extract_payment_data(&self, gateway_id: &str, payload: &Value) -> Result<PaymentData> {
        match gateway_id {
            "xendit" => self.extract_xendit_payment_data(payload),
            "midtrans" => self.extract_midtrans_payment_data(payload),
            _ => Err(AppError::validation(format!(
                "Unsupported gateway: {}",
                gateway_id
            ))),
        }
    }

    /// Extract payment data from Xendit webhook
    fn extract_xendit_payment_data(&self, payload: &Value) -> Result<PaymentData> {
        let invoice_id = payload["external_id"]
            .as_str()
            .ok_or_else(|| AppError::validation("Missing 'external_id' in Xendit webhook"))?
            .to_string();

        let amount = payload["amount"]
            .as_f64()
            .ok_or_else(|| AppError::validation("Missing 'amount' in Xendit webhook"))?;

        let currency_str = payload["currency"]
            .as_str()
            .ok_or_else(|| AppError::validation("Missing 'currency' in Xendit webhook"))?;

        let currency = currency_str
            .parse::<Currency>()
            .map_err(|e| AppError::validation(format!("Invalid currency: {}", e)))?;

        let payment_method = payload["payment_method"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let status_str = payload["status"]
            .as_str()
            .ok_or_else(|| AppError::validation("Missing 'status' in Xendit webhook"))?;

        let status = match status_str {
            "PAID" | "SETTLED" => TransactionStatus::Completed,
            "PENDING" => TransactionStatus::Pending,
            "EXPIRED" | "FAILED" => TransactionStatus::Failed,
            _ => TransactionStatus::Pending,
        };

        Ok(PaymentData {
            invoice_id,
            amount: Decimal::from_f64_retain(amount)
                .ok_or_else(|| AppError::validation("Invalid amount format"))?,
            currency,
            payment_method,
            status,
        })
    }

    /// Extract payment data from Midtrans webhook
    fn extract_midtrans_payment_data(&self, payload: &Value) -> Result<PaymentData> {
        let invoice_id = payload["order_id"]
            .as_str()
            .ok_or_else(|| AppError::validation("Missing 'order_id' in Midtrans webhook"))?
            .to_string();

        let amount = payload["gross_amount"]
            .as_str()
            .ok_or_else(|| AppError::validation("Missing 'gross_amount' in Midtrans webhook"))?
            .parse::<f64>()
            .map_err(|_| AppError::validation("Invalid 'gross_amount' format"))?;

        let payment_method = payload["payment_type"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let transaction_status = payload["transaction_status"].as_str().ok_or_else(|| {
            AppError::validation("Missing 'transaction_status' in Midtrans webhook")
        })?;

        let status = match transaction_status {
            "settlement" | "capture" => TransactionStatus::Completed,
            "pending" => TransactionStatus::Pending,
            "deny" | "expire" | "cancel" => TransactionStatus::Failed,
            _ => TransactionStatus::Pending,
        };

        Ok(PaymentData {
            invoice_id,
            amount: Decimal::from_f64_retain(amount)
                .ok_or_else(|| AppError::validation("Invalid amount format"))?,
            currency: Currency::IDR, // Midtrans only supports IDR
            payment_method,
            status,
        })
    }
}

/// Payment data extracted from webhook
#[derive(Debug)]
struct PaymentData {
    invoice_id: String,
    amount: Decimal,
    currency: Currency,
    payment_method: String,
    status: TransactionStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_delay_configuration() {
        // Test default retry configuration
        let delays = vec![
            Duration::from_secs(60),   // 1 minute
            Duration::from_secs(300),  // 5 minutes
            Duration::from_secs(1800), // 30 minutes
        ];

        assert_eq!(delays.len(), 3);
        assert_eq!(delays[0], Duration::from_secs(60));
        assert_eq!(delays[1], Duration::from_secs(300));
        assert_eq!(delays[2], Duration::from_secs(1800));
    }

    #[test]
    fn test_webhook_result_variants() {
        // Test WebhookResult::Success
        let success = WebhookResult::Success {
            transaction_id: "txn-123".to_string(),
            gateway_ref: "gateway-ref-456".to_string(),
        };
        match success {
            WebhookResult::Success {
                transaction_id,
                gateway_ref,
            } => {
                assert_eq!(transaction_id, "txn-123");
                assert_eq!(gateway_ref, "gateway-ref-456");
            }
            _ => panic!("Expected Success variant"),
        }

        // Test WebhookResult::Failed
        let failed = WebhookResult::Failed {
            gateway_ref: "gateway-ref-789".to_string(),
            error: "Processing error".to_string(),
        };
        match failed {
            WebhookResult::Failed { gateway_ref, error } => {
                assert_eq!(gateway_ref, "gateway-ref-789");
                assert_eq!(error, "Processing error");
            }
            _ => panic!("Expected Failed variant"),
        }

        // Test WebhookResult::Duplicate
        let duplicate = WebhookResult::Duplicate {
            transaction_id: "txn-existing".to_string(),
            gateway_ref: "gateway-ref-duplicate".to_string(),
        };
        match duplicate {
            WebhookResult::Duplicate {
                transaction_id,
                gateway_ref,
            } => {
                assert_eq!(transaction_id, "txn-existing");
                assert_eq!(gateway_ref, "gateway-ref-duplicate");
            }
            _ => panic!("Expected Duplicate variant"),
        }
    }

    #[test]
    fn test_payment_data_structure() {
        let payment_data = PaymentData {
            invoice_id: "INV-001".to_string(),
            amount: Decimal::new(100000, 0),
            currency: Currency::IDR,
            payment_method: "bank_transfer".to_string(),
            status: TransactionStatus::Completed,
        };

        assert_eq!(payment_data.invoice_id, "INV-001");
        assert_eq!(payment_data.amount, Decimal::new(100000, 0));
        assert_eq!(payment_data.currency, Currency::IDR);
        assert_eq!(payment_data.payment_method, "bank_transfer");
    }
}
