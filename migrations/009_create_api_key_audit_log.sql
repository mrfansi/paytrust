-- Migration 009: Create api_key_audit_log table
-- Audit trail for API key lifecycle events (FR-083)

CREATE TABLE IF NOT EXISTS api_key_audit_log (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    api_key_id BIGINT UNSIGNED NULL COMMENT 'Reference to api_keys table',
    operation_type ENUM('created', 'rotated', 'revoked', 'used') NOT NULL,
    actor_identifier VARCHAR(255) NULL COMMENT 'Who performed the operation',
    ip_address VARCHAR(45) NULL COMMENT 'IPv4 or IPv6 address',
    old_key_hash VARCHAR(255) NULL COMMENT 'Previous key hash (for rotation)',
    success_status BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_api_key (api_key_id),
    INDEX idx_operation (operation_type),
    INDEX idx_created_at (created_at),
    FOREIGN KEY (api_key_id) REFERENCES api_keys(id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
