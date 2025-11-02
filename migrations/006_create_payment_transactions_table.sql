-- Migration 006: Create payment_transactions table
-- Record of actual payment attempts and completions

CREATE TABLE IF NOT EXISTS payment_transactions (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    invoice_id BIGINT UNSIGNED NOT NULL,
    installment_id BIGINT UNSIGNED NULL COMMENT 'Related installment (if applicable)',
    gateway_transaction_ref VARCHAR(255) NOT NULL UNIQUE COMMENT 'Gateway transaction ID',
    amount_paid DECIMAL(19,4) NOT NULL,
    overpayment_amount DECIMAL(19,4) NULL COMMENT 'Excess payment amount (FR-073)',
    payment_method VARCHAR(50) NOT NULL COMMENT 'e.g., credit_card, bank_transfer, ewallet',
    status ENUM('pending', 'completed', 'failed', 'refunded') NOT NULL,
    gateway_response JSON NULL COMMENT 'Full gateway webhook payload',
    refund_id VARCHAR(255) NULL COMMENT 'Gateway refund reference (FR-086)',
    refund_amount DECIMAL(19,4) NULL COMMENT 'Refund amount (FR-086)',
    refund_timestamp TIMESTAMP NULL COMMENT 'Refund timestamp (FR-086)',
    refund_reason TEXT NULL COMMENT 'Refund reason (FR-086)',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_invoice (invoice_id),
    INDEX idx_installment (installment_id),
    INDEX idx_gateway_ref (gateway_transaction_ref),
    INDEX idx_status_created (status, created_at),
    FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE RESTRICT,
    FOREIGN KEY (installment_id) REFERENCES installment_schedules(id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
