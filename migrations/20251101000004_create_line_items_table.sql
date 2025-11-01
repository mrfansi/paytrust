-- Migration: Create line_items table
-- Created: 2025-11-01
-- Description: Individual products/services in an invoice with per-item tax rates

CREATE TABLE line_items (
    id VARCHAR(36) PRIMARY KEY,
    invoice_id VARCHAR(36) NOT NULL COMMENT 'Parent invoice',
    product_name VARCHAR(255) NOT NULL COMMENT 'Product/service name',
    quantity DECIMAL(10,2) NOT NULL COMMENT 'Quantity purchased',
    unit_price DECIMAL(19,4) NOT NULL COMMENT 'Price per unit',
    subtotal DECIMAL(19,4) NOT NULL COMMENT 'quantity × unit_price',
    tax_rate DECIMAL(5,4) NOT NULL DEFAULT 0.0000 COMMENT 'Tax rate as decimal (0.10 = 10%)',
    tax_category VARCHAR(50) NULL COMMENT 'Optional tax category identifier',
    tax_amount DECIMAL(19,4) NOT NULL DEFAULT 0.0000 COMMENT 'subtotal × tax_rate',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    INDEX idx_line_items_invoice (invoice_id),
    INDEX idx_line_items_tax_category (tax_category),
    
    CONSTRAINT fk_line_items_invoice 
        FOREIGN KEY (invoice_id) 
        REFERENCES invoices(id) 
        ON DELETE CASCADE,
    
    CONSTRAINT chk_line_items_quantity_positive 
        CHECK (quantity > 0),
    
    CONSTRAINT chk_line_items_unit_price_non_negative 
        CHECK (unit_price >= 0),
    
    CONSTRAINT chk_line_items_tax_rate_range 
        CHECK (tax_rate >= 0 AND tax_rate <= 1),
    
    CONSTRAINT chk_line_items_amounts_non_negative 
        CHECK (subtotal >= 0 AND tax_amount >= 0)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='Line items for each invoice';
