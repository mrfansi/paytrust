-- Migration 008: Create webhook_retry_log table
-- Audit trail for webhook retry attempts (FR-042)

CREATE TABLE IF NOT EXISTS webhook_retry_log (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    webhook_id VARCHAR(255) NOT NULL COMMENT 'Gateway webhook identifier',
    attempt_number INT UNSIGNED NOT NULL COMMENT 'Retry attempt number (1, 2, 3)',
    attempted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT 'When retry was attempted',
    status ENUM('success', 'failed', 'permanently_failed') NOT NULL,
    error_message TEXT NULL COMMENT 'Error details if failed',
    INDEX idx_webhook_id (webhook_id),
    INDEX idx_attempted_at (attempted_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
