-- Migration: Create installment_schedules table
-- Created: 2025-11-01
-- Description: Payment plan entries for installment-based invoices

CREATE TABLE installment_schedules (
    id VARCHAR(36) PRIMARY KEY,
    invoice_id VARCHAR(36) NOT NULL COMMENT 'Parent invoice',
    installment_number INT NOT NULL COMMENT 'Sequential number (1, 2, 3...)',
    amount DECIMAL(19,4) NOT NULL COMMENT 'Installment payment amount',
    tax_amount DECIMAL(19,4) NOT NULL DEFAULT 0.0000 COMMENT 'Proportionally distributed tax (FR-059)',
    service_fee_amount DECIMAL(19,4) NOT NULL DEFAULT 0.0000 COMMENT 'Proportionally distributed fee (FR-060)',
    due_date DATE NOT NULL COMMENT 'Payment due date',
    status ENUM('unpaid', 'paid', 'overdue') NOT NULL DEFAULT 'unpaid',
    payment_url TEXT NULL COMMENT 'Gateway-generated payment link',
    gateway_reference VARCHAR(255) NULL COMMENT 'Gateway transaction reference',
    paid_at TIMESTAMP NULL COMMENT 'Payment completion timestamp',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    UNIQUE KEY uk_installment_schedules_invoice_number (invoice_id, installment_number),
    INDEX idx_installment_schedules_invoice_status (invoice_id, status),
    INDEX idx_installment_schedules_gateway_ref (gateway_reference),
    INDEX idx_installment_schedules_due_date (due_date),
    INDEX idx_installment_schedules_status (status),
    
    CONSTRAINT fk_installment_schedules_invoice 
        FOREIGN KEY (invoice_id) 
        REFERENCES invoices(id) 
        ON DELETE CASCADE,
    
    CONSTRAINT chk_installment_schedules_number_positive 
        CHECK (installment_number > 0),
    
    CONSTRAINT chk_installment_schedules_amount_positive 
        CHECK (amount > 0),
    
    CONSTRAINT chk_installment_schedules_amounts_non_negative 
        CHECK (tax_amount >= 0 AND service_fee_amount >= 0)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='Installment payment schedules (FR-014 to FR-020)';
