-- Migration: Add external_id column to invoices table
-- Created: 2025-11-02
-- Description: Add missing external_id column (merchant's reference ID)

ALTER TABLE invoices 
ADD COLUMN external_id VARCHAR(255) NOT NULL UNIQUE 
COMMENT 'Merchant reference ID (must be unique)' 
AFTER merchant_id;

-- Add index for quick lookup by external_id
CREATE INDEX idx_invoices_external_id ON invoices(external_id);
