-- Migration: Add original_invoice_id column for supplementary invoices
-- Task: T103 - Update Invoice model to support original_invoice_id reference (FR-082)
-- Created: 2025-11-02
--
-- Purpose: Support supplementary invoice creation when overpayment exceeds
-- all installments. Supplementary invoices reference the original invoice.

-- Add original_invoice_id column to invoices table
ALTER TABLE invoices 
ADD COLUMN original_invoice_id VARCHAR(36) NULL AFTER expires_at,
ADD CONSTRAINT fk_invoices_original_invoice 
    FOREIGN KEY (original_invoice_id) 
    REFERENCES invoices(id) 
    ON DELETE SET NULL;

-- Add index for querying supplementary invoices by original invoice
CREATE INDEX idx_invoices_original_invoice_id 
ON invoices(original_invoice_id);

-- Add comment for documentation
ALTER TABLE invoices 
MODIFY COLUMN original_invoice_id VARCHAR(36) NULL 
COMMENT 'Reference to original invoice if this is a supplementary invoice (FR-082)';
