-- Migration: Create invoices table
-- Created: 2025-11-01
-- Description: Store invoice information with line items, taxes, and service fees

CREATE TABLE invoices (
    id VARCHAR(36) PRIMARY KEY,
    merchant_id VARCHAR(36) NOT NULL COMMENT 'Merchant who owns this invoice',
    currency ENUM('IDR', 'MYR', 'USD') NOT NULL COMMENT 'Invoice currency',
    subtotal DECIMAL(19,4) NOT NULL COMMENT 'Sum of all line item subtotals',
    tax_total DECIMAL(19,4) NOT NULL DEFAULT 0.0000 COMMENT 'Sum of all taxes',
    service_fee DECIMAL(19,4) NOT NULL DEFAULT 0.0000 COMMENT 'Payment gateway service fee',
    total_amount DECIMAL(19,4) NOT NULL COMMENT 'subtotal + tax_total + service_fee',
    status ENUM('draft', 'pending', 'partially_paid', 'paid', 'failed', 'expired') NOT NULL DEFAULT 'draft',
    gateway_id VARCHAR(36) NULL COMMENT 'Selected payment gateway',
    original_invoice_id VARCHAR(36) NULL COMMENT 'Reference for supplementary invoices (FR-082)',
    is_immutable BOOLEAN NOT NULL DEFAULT FALSE COMMENT 'Locked after payment initiation (FR-051)',
    expires_at TIMESTAMP NOT NULL COMMENT 'Invoice expiration (default 24h from creation)',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    INDEX idx_invoices_merchant_status (merchant_id, status),
    INDEX idx_invoices_merchant_created (merchant_id, created_at DESC),
    INDEX idx_invoices_gateway (gateway_id),
    INDEX idx_invoices_original (original_invoice_id),
    INDEX idx_invoices_status (status),
    INDEX idx_invoices_expires (expires_at),
    
    CONSTRAINT fk_invoices_gateway 
        FOREIGN KEY (gateway_id) 
        REFERENCES payment_gateways(id) 
        ON DELETE RESTRICT,
    
    CONSTRAINT fk_invoices_original 
        FOREIGN KEY (original_invoice_id) 
        REFERENCES invoices(id) 
        ON DELETE SET NULL,
    
    CONSTRAINT chk_invoices_positive_amounts 
        CHECK (subtotal >= 0 AND tax_total >= 0 AND service_fee >= 0 AND total_amount >= 0)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='Invoice records with payment status tracking';
