-- Migration 001: Create gateway_configs table
-- Payment gateway configuration for Xendit and Midtrans

CREATE TABLE IF NOT EXISTS gateway_configs (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(50) NOT NULL UNIQUE,
    supported_currencies JSON NOT NULL COMMENT 'Array of currency codes (IDR, MYR, USD)',
    fee_percentage DECIMAL(5,4) NOT NULL COMMENT 'Percentage fee (e.g., 0.0290 for 2.9%)',
    fee_fixed DECIMAL(10,2) NOT NULL COMMENT 'Fixed fee amount',
    region VARCHAR(50) NULL,
    webhook_url VARCHAR(255) NULL,
    api_key_encrypted TEXT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    environment ENUM('sandbox', 'production') NOT NULL DEFAULT 'sandbox',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_gateway_name (name),
    INDEX idx_gateway_active (is_active)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Insert default gateway configurations for sandbox
INSERT INTO gateway_configs (name, supported_currencies, fee_percentage, fee_fixed, region, environment) VALUES
('xendit', '["IDR", "MYR", "USD"]', 0.0290, 0.00, 'APAC', 'sandbox'),
('midtrans', '["IDR"]', 0.0290, 0.00, 'Indonesia', 'sandbox');
