-- Migration 005: Create installment_schedules table
-- Payment plan entries for installment-based invoices

CREATE TABLE IF NOT EXISTS installment_schedules (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    invoice_id BIGINT UNSIGNED NOT NULL,
    installment_number INT UNSIGNED NOT NULL CHECK (installment_number > 0),
    amount DECIMAL(19,4) NOT NULL CHECK (amount > 0),
    tax_amount DECIMAL(19,4) NOT NULL DEFAULT 0 COMMENT 'Proportionally distributed tax',
    service_fee_amount DECIMAL(19,4) NOT NULL DEFAULT 0 COMMENT 'Proportionally distributed fee',
    due_date DATE NOT NULL,
    status ENUM('unpaid', 'paid', 'overdue') NOT NULL DEFAULT 'unpaid',
    payment_url TEXT NULL COMMENT 'Gateway-generated payment link',
    gateway_reference VARCHAR(255) NULL COMMENT 'Gateway transaction reference',
    paid_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE KEY unique_invoice_installment (invoice_id, installment_number),
    INDEX idx_invoice_status (invoice_id, status),
    INDEX idx_gateway_ref (gateway_reference),
    FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
