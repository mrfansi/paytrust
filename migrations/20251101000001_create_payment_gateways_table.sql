-- Migration: Create payment_gateways table
-- Created: 2025-11-01
-- Description: Store payment gateway configurations (Xendit, Midtrans)

CREATE TABLE payment_gateways (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(50) NOT NULL UNIQUE,
    supported_currencies JSON NOT NULL COMMENT 'Array of currency codes',
    fee_percentage DECIMAL(5,4) NOT NULL DEFAULT 0.0000 COMMENT 'Gateway fee percentage (e.g., 0.0290 = 2.9%)',
    fee_fixed DECIMAL(19,4) NOT NULL DEFAULT 0.0000 COMMENT 'Fixed gateway fee amount',
    api_key_encrypted BLOB NOT NULL COMMENT 'Encrypted API credentials',
    webhook_secret VARCHAR(255) NOT NULL COMMENT 'Webhook signature verification secret',
    webhook_url TEXT NOT NULL COMMENT 'Webhook endpoint URL',
    is_active BOOLEAN NOT NULL DEFAULT TRUE COMMENT 'Gateway availability status',
    environment ENUM('sandbox', 'production') NOT NULL DEFAULT 'sandbox',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    INDEX idx_payment_gateways_name (name),
    INDEX idx_payment_gateways_active (is_active, environment)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='Payment gateway configurations';
