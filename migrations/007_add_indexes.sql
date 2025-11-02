-- Migration 007: Add performance indexes and constraints
-- Additional indexes for query optimization

-- Invoice queries by expiration
CREATE INDEX idx_invoices_expires_at ON invoices(expires_at, status);

-- Transaction queries by tenant (for reporting)
CREATE INDEX idx_transactions_tenant ON payment_transactions(invoice_id, status, created_at);

-- Line items aggregation queries
CREATE INDEX idx_line_items_tax ON line_items(invoice_id, tax_rate);

-- Installment due date queries
CREATE INDEX idx_installments_due_date ON installment_schedules(due_date, status);

-- API key lookup optimization
CREATE INDEX idx_api_keys_tenant_active ON api_keys(tenant_id, is_active);
