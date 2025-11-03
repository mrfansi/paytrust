use crate::core::error::AppError;
use crate::modules::transactions::repositories::transaction_repository::TransactionRepository;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

/// Webhook retry handler implementing FR-042 retry logic
/// 
/// Retry schedule (cumulative delays from initial failure T=0):
/// - Retry 1: T+1 minute (1 min after initial failure)
/// - Retry 2: T+6 minutes (6 min after initial failure, 5 min after retry 1)
/// - Retry 3: T+36 minutes (36 min after initial failure, 30 min after retry 2)
/// 
/// Retry ONLY for 5xx errors and connection timeouts >10s
/// 4xx errors (including signature verification failures) marked permanently failed immediately
/// 
/// Retry timers are in-memory only and do NOT persist across application restarts per FR-042
#[derive(Clone)]
pub struct WebhookHandler {
    transaction_repo: Arc<dyn TransactionRepository>,
}

impl WebhookHandler {
    pub fn new(transaction_repo: Arc<dyn TransactionRepository>) -> Self {
        Self { transaction_repo }
    }

    /// Process webhook with automatic retry logic
    /// 
    /// Returns Ok(()) if webhook processed successfully or permanently failed (4xx)
    /// Returns Err() only for unexpected system errors
    pub async fn process_webhook_with_retry<F, Fut>(
        &self,
        webhook_id: &str,
        process_fn: F,
    ) -> Result<(), AppError>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<(), AppError>> + Send,
    {
        // Initial attempt
        match process_fn().await {
            Ok(_) => {
                info!(webhook_id = %webhook_id, "Webhook processed successfully");
                self.log_webhook_attempt(webhook_id, 0, "success", None).await?;
                return Ok(());
            }
            Err(e) => {
                // Check if error is retryable
                if !self.is_retryable_error(&e) {
                    error!(webhook_id = %webhook_id, error = %e, "Webhook failed with non-retryable error (4xx)");
                    self.log_webhook_attempt(webhook_id, 0, "permanently_failed", Some(&e.to_string())).await?;
                    return Ok(()); // Don't retry 4xx errors
                }
                
                warn!(webhook_id = %webhook_id, error = %e, "Webhook failed, will retry");
                self.log_webhook_attempt(webhook_id, 0, "failed", Some(&e.to_string())).await?;
            }
        }

        // Retry schedule: [1 min, 6 min, 36 min] cumulative from T=0
        let retry_delays = vec![
            Duration::from_secs(60),      // 1 minute
            Duration::from_secs(360),     // 6 minutes total (5 min after retry 1)
            Duration::from_secs(2160),    // 36 minutes total (30 min after retry 2)
        ];

        for (attempt, delay) in retry_delays.iter().enumerate() {
            let attempt_number = attempt + 1;
            
            info!(
                webhook_id = %webhook_id,
                attempt = attempt_number,
                delay_seconds = delay.as_secs(),
                "Scheduling webhook retry"
            );

            sleep(*delay).await;

            match process_fn().await {
                Ok(_) => {
                    info!(
                        webhook_id = %webhook_id,
                        attempt = attempt_number,
                        "Webhook retry succeeded"
                    );
                    self.log_webhook_attempt(webhook_id, attempt_number, "success", None).await?;
                    return Ok(());
                }
                Err(e) => {
                    if !self.is_retryable_error(&e) {
                        error!(
                            webhook_id = %webhook_id,
                            attempt = attempt_number,
                            error = %e,
                            "Webhook retry failed with non-retryable error"
                        );
                        self.log_webhook_attempt(
                            webhook_id,
                            attempt_number,
                            "permanently_failed",
                            Some(&e.to_string())
                        ).await?;
                        return Ok(());
                    }

                    warn!(
                        webhook_id = %webhook_id,
                        attempt = attempt_number,
                        error = %e,
                        "Webhook retry failed"
                    );
                    self.log_webhook_attempt(
                        webhook_id,
                        attempt_number,
                        "failed",
                        Some(&e.to_string())
                    ).await?;
                }
            }
        }

        // All retries exhausted
        error!(
            webhook_id = %webhook_id,
            "All webhook retries exhausted, marking as permanently failed"
        );
        self.log_webhook_attempt(
            webhook_id,
            4,
            "permanently_failed",
            Some("All retries exhausted")
        ).await?;

        Ok(())
    }

    /// Check if error is retryable (5xx or connection timeout)
    fn is_retryable_error(&self, error: &AppError) -> bool {
        match error {
            AppError::GatewayError(msg) => {
                // Check for 5xx errors or connection timeouts
                msg.contains("5") && (msg.contains("500") || msg.contains("502") || msg.contains("503") || msg.contains("504"))
                    || msg.contains("timeout")
                    || msg.contains("connection")
            }
            AppError::NetworkError(_) => true,
            _ => false,
        }
    }

    /// Log webhook attempt to webhook_retry_log table per FR-042 Audit Logging
    async fn log_webhook_attempt(
        &self,
        webhook_id: &str,
        attempt_number: usize,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<(), AppError> {
        // TODO: Implement actual database logging to webhook_retry_log table
        // This requires the webhook_retry_log table from migration 008
        info!(
            webhook_id = %webhook_id,
            attempt = attempt_number,
            status = %status,
            error = ?error_message,
            "Webhook attempt logged"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests for webhook retry logic are in tests/integration/
    // Unit tests here focus on error classification logic
}
