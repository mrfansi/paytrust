-- Migration 003: Create invoices table
-- Invoice entity with line items, taxes, and service fees

CREATE TABLE IF NOT EXISTS invoices (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    external_id VARCHAR(100) NOT NULL UNIQUE COMMENT 'External invoice identifier for API',
    tenant_id VARCHAR(36) NOT NULL COMMENT 'Tenant/merchant identifier for multi-tenant isolation',
    currency ENUM('IDR', 'MYR', 'USD') NOT NULL,
    subtotal DECIMAL(19,4) NOT NULL COMMENT 'Sum of all line item subtotals',
    tax_total DECIMAL(19,4) NOT NULL DEFAULT 0 COMMENT 'Sum of all taxes',
    service_fee DECIMAL(19,4) NOT NULL DEFAULT 0 COMMENT 'Payment gateway service fee',
    total_amount DECIMAL(19,4) NOT NULL COMMENT 'subtotal + tax_total + service_fee',
    status ENUM('draft', 'pending', 'partially_paid', 'paid', 'failed', 'expired', 'cancelled') NOT NULL DEFAULT 'draft',
    gateway_id BIGINT UNSIGNED NULL COMMENT 'Selected payment gateway',
    original_invoice_id BIGINT UNSIGNED NULL COMMENT 'Reference for supplementary invoices (FR-082)',
    payment_initiated_at TIMESTAMP NULL DEFAULT NULL COMMENT 'Timestamp when first payment attempt occurs (FR-051)',
    expires_at TIMESTAMP NOT NULL COMMENT 'Invoice expiration (default 24h from creation)',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_tenant_status (tenant_id, status),
    INDEX idx_tenant_created (tenant_id, created_at),
    INDEX idx_gateway (gateway_id),
    INDEX idx_original_invoice (original_invoice_id),
    INDEX idx_external_id (external_id),
    FOREIGN KEY (gateway_id) REFERENCES gateway_configs(id) ON DELETE SET NULL,
    FOREIGN KEY (original_invoice_id) REFERENCES invoices(id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
