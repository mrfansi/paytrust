-- Migration: Create payment_transactions table
-- Created: 2025-11-01
-- Description: Record of actual payment attempts and completions

CREATE TABLE payment_transactions (
    id VARCHAR(36) PRIMARY KEY,
    invoice_id VARCHAR(36) NOT NULL COMMENT 'Related invoice',
    installment_id VARCHAR(36) NULL COMMENT 'Related installment (if applicable)',
    gateway_transaction_ref VARCHAR(255) NOT NULL UNIQUE COMMENT 'Gateway transaction ID (idempotency key)',
    amount_paid DECIMAL(19,4) NOT NULL COMMENT 'Actual amount received',
    payment_method VARCHAR(50) NOT NULL COMMENT 'e.g., credit_card, bank_transfer, ewallet',
    status ENUM('pending', 'completed', 'failed', 'refunded') NOT NULL DEFAULT 'pending',
    gateway_response JSON NULL COMMENT 'Full gateway webhook payload',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    INDEX idx_payment_transactions_invoice (invoice_id),
    INDEX idx_payment_transactions_installment (installment_id),
    INDEX idx_payment_transactions_gateway_ref (gateway_transaction_ref),
    INDEX idx_payment_transactions_status_created (status, created_at DESC),
    INDEX idx_payment_transactions_status (status),
    
    CONSTRAINT fk_payment_transactions_invoice 
        FOREIGN KEY (invoice_id) 
        REFERENCES invoices(id) 
        ON DELETE RESTRICT,
    
    CONSTRAINT fk_payment_transactions_installment 
        FOREIGN KEY (installment_id) 
        REFERENCES installment_schedules(id) 
        ON DELETE SET NULL,
    
    CONSTRAINT chk_payment_transactions_amount_positive 
        CHECK (amount_paid > 0)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='Payment transaction history (FR-030, FR-032, FR-048)';
