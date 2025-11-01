-- Migration: Create api_keys table
-- Created: 2025-11-01
-- Description: Store API keys for merchant authentication and rate limiting

CREATE TABLE api_keys (
    id VARCHAR(36) PRIMARY KEY,
    key_hash VARCHAR(255) NOT NULL UNIQUE COMMENT 'Argon2 hash of API key',
    merchant_id VARCHAR(36) NOT NULL COMMENT 'Merchant/developer identifier',
    rate_limit INT NOT NULL DEFAULT 1000 COMMENT 'Requests per minute',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_used_at TIMESTAMP NULL COMMENT 'Last request timestamp',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    INDEX idx_api_keys_merchant (merchant_id),
    INDEX idx_api_keys_active (is_active),
    INDEX idx_api_keys_hash (key_hash)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='API keys for merchant authentication';
