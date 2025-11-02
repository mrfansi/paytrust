-- Migration 002: Create api_keys table
-- API key authentication and rate limiting

CREATE TABLE IF NOT EXISTS api_keys (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    key_hash VARCHAR(255) NOT NULL UNIQUE COMMENT 'Argon2 hash of API key',
    tenant_id VARCHAR(36) NOT NULL COMMENT 'Tenant/merchant identifier for multi-tenant isolation',
    rate_limit INT UNSIGNED NOT NULL DEFAULT 1000 COMMENT 'Requests per minute',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_used_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_api_key_hash (key_hash),
    INDEX idx_tenant_id (tenant_id),
    INDEX idx_active (is_active)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
