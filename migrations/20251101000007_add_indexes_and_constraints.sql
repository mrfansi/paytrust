-- Migration: Add performance indexes and additional constraints
-- Created: 2025-11-01
-- Description: Optimize query performance for common access patterns

-- Additional indexes for reporting and aggregation queries
CREATE INDEX idx_invoices_currency_status ON invoices(currency, status);
CREATE INDEX idx_invoices_created_status ON invoices(created_at DESC, status);

-- Indexes for financial reporting (tax and service fee aggregation)
CREATE INDEX idx_line_items_tax_rate ON line_items(tax_rate);
CREATE INDEX idx_line_items_invoice_created ON line_items(invoice_id, created_at);

-- Indexes for installment payment tracking
CREATE INDEX idx_installment_schedules_status_due ON installment_schedules(status, due_date);
CREATE INDEX idx_installment_schedules_paid_at ON installment_schedules(paid_at);

-- Indexes for transaction queries
CREATE INDEX idx_payment_transactions_invoice_status ON payment_transactions(invoice_id, status);
CREATE INDEX idx_payment_transactions_created ON payment_transactions(created_at DESC);

-- Composite index for merchant dashboard queries
CREATE INDEX idx_invoices_merchant_currency_status ON invoices(merchant_id, currency, status);

-- Index for expired invoice cleanup
CREATE INDEX idx_invoices_status_expires ON invoices(status, expires_at);

-- Full-text search index for product names (optional, can be removed if not needed)
-- CREATE FULLTEXT INDEX idx_line_items_product_name ON line_items(product_name);

-- Add comment for documentation
ALTER TABLE invoices 
    COMMENT = 'Invoice records with payment status tracking. Immutability enforced via is_immutable flag (FR-051).';

ALTER TABLE line_items 
    COMMENT = 'Line items with per-item tax rates (FR-057, FR-058). Subtotal = quantity × unit_price.';

ALTER TABLE installment_schedules 
    COMMENT = 'Installment schedules with proportional tax/fee distribution (FR-059, FR-060). Sequential payment order enforced (FR-068).';

ALTER TABLE payment_transactions 
    COMMENT = 'Payment transaction history with idempotency via gateway_transaction_ref (FR-032). Supports overpayments (FR-073).';
