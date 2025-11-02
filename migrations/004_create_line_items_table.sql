-- Migration 004: Create line_items table
-- Individual product/service entries in an invoice

CREATE TABLE IF NOT EXISTS line_items (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    invoice_id BIGINT UNSIGNED NOT NULL,
    product_name VARCHAR(255) NOT NULL,
    quantity DECIMAL(10,2) NOT NULL CHECK (quantity > 0),
    unit_price DECIMAL(19,4) NOT NULL CHECK (unit_price >= 0),
    subtotal DECIMAL(19,4) NOT NULL COMMENT 'quantity × unit_price',
    tax_rate DECIMAL(5,4) NOT NULL CHECK (tax_rate >= 0 AND tax_rate <= 1) COMMENT 'Tax rate as decimal (0.10 = 10%)',
    tax_category VARCHAR(50) NULL,
    tax_amount DECIMAL(19,4) NOT NULL COMMENT 'subtotal × tax_rate',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_invoice (invoice_id),
    FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
