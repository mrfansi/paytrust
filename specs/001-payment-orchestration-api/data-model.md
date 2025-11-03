# Phase 1 Data Model: PayTrust Payment Orchestration API

**Status**: COMPLETE
**Generated**: 2025-11-03
**Purpose**: Define database schema, domain entities, and relationships

---

## Overview

Multi-tenant payment orchestration platform with:
- Core payment workflow: invoices → line items → installment schedules → payment transactions
- Payment gateway abstraction: Xendit/Midtrans adapters
- ISO 20022 compliance: internal pain.001/pain.002 representations
- Webhook processing: gateway events → ISO 20022 status reports
- Comprehensive audit: API key operations, webhook retries, transaction history

**Data Isolation Strategy**: All tables include `tenant_id` for multi-tenant query filtering. Currency-specific numeric precision: IDR (whole numbers), MYR/USD (cents = ÷100).

---

## Core Domain Entities

### Invoices Table

```sql
CREATE TABLE invoices (
  id BIGSERIAL PRIMARY KEY,
  tenant_id BIGINT NOT NULL,
  external_id VARCHAR(255),
  gateway_id BIGINT NOT NULL,
  currency_code VARCHAR(3) NOT NULL, -- ISO 4217: IDR, MYR, USD

  -- Status tracking
  status VARCHAR(20) NOT NULL, -- draft, pending, partially_paid, fully_paid, failed, expired
  payment_initiated_at TIMESTAMP NULL DEFAULT NULL, -- Set on first payment attempt (FR-046)
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  expires_at TIMESTAMP NOT NULL,

  -- Financial amounts (stored as BIGINT: cents for currency)
  subtotal_amount BIGINT NOT NULL, -- Sum of line_items
  total_tax_amount BIGINT NOT NULL,
  total_service_fee_amount BIGINT NOT NULL,
  total_amount BIGINT NOT NULL, -- subtotal + tax + service_fee

  -- Supplementary invoice tracking (FR-070)
  original_invoice_id BIGINT NULL, -- FK to parent invoice (NULL for regular invoices)

  FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
  FOREIGN KEY (gateway_id) REFERENCES gateway_configurations(id) ON DELETE RESTRICT,
  FOREIGN KEY (original_invoice_id) REFERENCES invoices(id) ON DELETE RESTRICT,
  UNIQUE(tenant_id, external_id),
  INDEX idx_tenant_created (tenant_id, created_at),
  INDEX idx_tenant_status (tenant_id, status),
  INDEX idx_gateway_currency (gateway_id, currency_code)
);
```

**Key Constraints**:
- `payment_initiated_at IS NULL` = invoice remains mutable (draft status)
- `payment_initiated_at NOT NULL` = invoice immutable per FR-046
- `original_invoice_id NOT NULL` = supplementary invoice (linked to parent per FR-070)
- `status` transitions: draft → pending (on payment request) → partially_paid (after 1st installment) → fully_paid (all paid) or → failed/expired
- `expires_at` validation per FR-041: 1 hour minimum, 30 days maximum from created_at

---

### Line Items Table

```sql
CREATE TABLE line_items (
  id BIGSERIAL PRIMARY KEY,
  invoice_id BIGINT NOT NULL,
  tenant_id BIGINT NOT NULL,

  product_name VARCHAR(255) NOT NULL,
  quantity INT NOT NULL, -- Must be > 0
  unit_price_amount BIGINT NOT NULL, -- In currency cents

  -- Tax configuration (per-line-item per FR-050)
  tax_rate DECIMAL(5,4) NOT NULL, -- 0.0000 to 1.0000 (0-100%)
  tax_category VARCHAR(50), -- e.g., "VAT", "GST" (optional)
  country_code VARCHAR(2), -- ISO 3166-1 alpha-2 for jurisdiction (optional, per FR-002a)

  -- Calculated amounts
  subtotal_amount BIGINT NOT NULL, -- quantity × unit_price
  tax_amount BIGINT NOT NULL, -- subtotal × tax_rate

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE,
  FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
  INDEX idx_invoice_id (invoice_id),
  INDEX idx_tax_rate (tax_rate, country_code)
);
```

**Validation Rules** per FR-058:
- `tax_rate` >= 0 and <= 1.0 with max 4 decimal places (0.0001 precision)
- `country_code` must be valid ISO 3166-1 alpha-2 if provided
- Warn (log WARN level) if tax_rate > 0.27 (27%) per FR-058

---

### Installment Schedules Table

```sql
CREATE TABLE installment_schedules (
  id BIGSERIAL PRIMARY KEY,
  invoice_id BIGINT NOT NULL,
  tenant_id BIGINT NOT NULL,

  installment_number INT NOT NULL, -- 1 to 12 per FR-014
  due_date TIMESTAMP NOT NULL,

  -- Amounts (in currency cents)
  amount BIGINT NOT NULL, -- Base installment amount (before tax/fees)
  tax_amount BIGINT NOT NULL, -- Proportional tax per FR-052
  service_fee_amount BIGINT NOT NULL, -- Proportional fee per FR-053
  total_amount BIGINT NOT NULL, -- amount + tax + service_fee

  -- Payment tracking
  status VARCHAR(20) NOT NULL DEFAULT 'unpaid', -- unpaid, paid, overdue
  paid_at TIMESTAMP NULL,

  -- Foreign key to payment transaction when paid
  payment_transaction_id BIGINT NULL,

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE,
  FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
  FOREIGN KEY (payment_transaction_id) REFERENCES payment_transactions(id) ON DELETE SET NULL,
  UNIQUE(invoice_id, installment_number),
  INDEX idx_invoice_status (invoice_id, status),
  INDEX idx_tenant_due_date (tenant_id, due_date)
);
```

**Key Constraints** per FR-060, FR-061:
- Sequential payment enforced at service layer: only next unpaid installment can accept payments
- `status = 'overdue'` when due_date < current_time AND status = 'unpaid'
- `payment_transaction_id` tracks which gateway transaction paid this installment

---

### Payment Transactions Table

```sql
CREATE TABLE payment_transactions (
  id BIGSERIAL PRIMARY KEY,
  invoice_id BIGINT NOT NULL,
  installment_id BIGINT NULL,
  tenant_id BIGINT NOT NULL,

  -- Gateway reference
  gateway_id BIGINT NOT NULL,
  gateway_transaction_id VARCHAR(255) NOT NULL, -- External gateway transaction ID

  -- Payment details
  amount_received BIGINT NOT NULL, -- Actual amount received (in currency cents)
  currency_code VARCHAR(3) NOT NULL,
  payment_method VARCHAR(50), -- Extracted from gateway response

  -- Status
  status VARCHAR(20) NOT NULL, -- pending, completed, failed
  gateway_status_code VARCHAR(50), -- Gateway-specific status code

  -- ISO 20022 mapping (FR-028a)
  iso20022_message_id VARCHAR(255) NULL, -- pain.002 MessageID
  iso20022_transaction_status VARCHAR(50) NULL, -- pain.002 TransactionStatus

  -- Timestamps
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  completed_at TIMESTAMP NULL,
  gateway_timestamp TIMESTAMP NULL, -- Timestamp from gateway

  -- Gateway response (JSON)
  gateway_response LONGTEXT,

  FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE,
  FOREIGN KEY (installment_id) REFERENCES installment_schedules(id) ON DELETE SET NULL,
  FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
  FOREIGN KEY (gateway_id) REFERENCES gateway_configurations(id) ON DELETE RESTRICT,
  UNIQUE(gateway_id, gateway_transaction_id, tenant_id),
  INDEX idx_invoice_status (invoice_id, status),
  INDEX idx_tenant_completed (tenant_id, completed_at),
  INDEX idx_gateway_transaction (gateway_id, gateway_transaction_id)
);
```

---

### Gateway Configurations Table

```sql
CREATE TABLE gateway_configurations (
  id BIGSERIAL PRIMARY KEY,
  gateway_name VARCHAR(50) NOT NULL, -- Xendit, Midtrans
  supported_currencies JSON NOT NULL, -- ["IDR", "MYR", "USD"] array

  -- Fee structure: (subtotal × percentage) + fixed_amount
  fee_percentage DECIMAL(5,2) NOT NULL, -- e.g., 2.9 for 2.9%
  fee_fixed_amount BIGINT NOT NULL, -- In smallest currency unit (cents)

  -- Configuration
  region VARCHAR(50), -- e.g., "Indonesia", "Malaysia", "Global"
  api_endpoint VARCHAR(500) NOT NULL,
  webhook_endpoint VARCHAR(500) NOT NULL,

  -- Credentials (encrypted in production)
  credentials JSON NOT NULL,

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  UNIQUE(gateway_name),
  INDEX idx_gateway_name (gateway_name)
);
```

**Predefined Gateways** (populated at migration):
- ID=1: Xendit (currencies: IDR, MYR, USD)
- ID=2: Midtrans (currencies: IDR, MYR, USD)

---

### API Keys Table

```sql
CREATE TABLE api_keys (
  id BIGSERIAL PRIMARY KEY,
  tenant_id BIGINT NOT NULL,

  api_key_hash VARCHAR(255) NOT NULL, -- Argon2 hash
  key_prefix VARCHAR(10), -- First 4 chars of original key for identification

  status VARCHAR(20) NOT NULL DEFAULT 'active', -- active, rotated, revoked
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  rotated_at TIMESTAMP NULL,
  revoked_at TIMESTAMP NULL,
  last_used_at TIMESTAMP NULL,

  FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
  INDEX idx_tenant_active (tenant_id, status),
  INDEX idx_last_used (last_used_at)
);
```

**Security Notes** per FR-031, FR-071:
- `api_key_hash` stores Argon2 hash, never store plaintext
- `key_prefix` enables identification in logs/audit without exposing full key
- Return full key only at creation time; never retrievable after
- Rotation: create new active key, mark old as rotated

---

### API Key Audit Log Table

```sql
CREATE TABLE api_key_audit_log (
  id BIGSERIAL PRIMARY KEY,
  api_key_id BIGINT NOT NULL,
  tenant_id BIGINT NOT NULL,

  operation_type VARCHAR(50) NOT NULL, -- created, rotated, revoked, used
  actor_identifier VARCHAR(255) NOT NULL, -- API key ID, 'SYSTEM', or admin username
  ip_address VARCHAR(50),

  -- For rotation operations
  old_key_hash VARCHAR(255) NULL,

  -- Operation result
  success BOOLEAN NOT NULL DEFAULT TRUE,
  error_message TEXT,

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  FOREIGN KEY (api_key_id) REFERENCES api_keys(id) ON DELETE CASCADE,
  FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
  INDEX idx_tenant_created (tenant_id, created_at),
  INDEX idx_api_key_id (api_key_id),
  INDEX idx_operation_type (operation_type)
);
```

**Retention**: Minimum 1 year per NFR-011

---

### Webhook Events Table

```sql
CREATE TABLE webhook_events (
  id BIGSERIAL PRIMARY KEY,
  tenant_id BIGINT NOT NULL,
  gateway_id BIGINT NOT NULL,

  -- Webhook identification
  event_id VARCHAR(255) NOT NULL, -- Gateway-provided event ID for deduplication
  event_type VARCHAR(100), -- e.g., "payment.success", "payment.failed"

  -- Payload
  original_payload LONGTEXT NOT NULL, -- Raw gateway webhook payload (JSON/XML)

  -- Processing
  payment_transaction_id BIGINT NULL,
  invoice_id BIGINT NULL,

  status VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending, processed, failed
  processed_at TIMESTAMP NULL,

  -- Error tracking for retry logic
  error_message TEXT,

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
  FOREIGN KEY (gateway_id) REFERENCES gateway_configurations(id) ON DELETE RESTRICT,
  FOREIGN KEY (payment_transaction_id) REFERENCES payment_transactions(id) ON DELETE SET NULL,
  FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE SET NULL,
  UNIQUE(gateway_id, event_id, tenant_id), -- Prevent duplicate webhook processing per FR-040
  INDEX idx_tenant_status (tenant_id, status),
  INDEX idx_processed_at (processed_at)
);
```

---

### Webhook Retry Log Table

```sql
CREATE TABLE webhook_retry_log (
  id BIGSERIAL PRIMARY KEY,
  webhook_id BIGINT NOT NULL,
  tenant_id BIGINT NOT NULL,

  attempt_number INT NOT NULL, -- 1, 2, 3
  status VARCHAR(20) NOT NULL, -- success, failed, timeout
  error_message TEXT,

  attempted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  FOREIGN KEY (webhook_id) REFERENCES webhook_events(id) ON DELETE CASCADE,
  FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
  INDEX idx_webhook_attempt (webhook_id, attempt_number),
  INDEX idx_tenant_attempted (tenant_id, attempted_at)
);
```

---

### Tenants Table (Multi-Tenant Support)

```sql
CREATE TABLE tenants (
  id BIGSERIAL PRIMARY KEY,
  name VARCHAR(255) NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  UNIQUE(name)
);
```

**Note**: Typically one tenant per merchant/developer using the API. Created during admin/registration flow (outside scope of this spec).

---

## Domain Entity Models (Rust)

### Core Rust Struct Definitions

```rust
// models/invoice.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: i64,
    pub tenant_id: i64,
    pub external_id: Option<String>,
    pub gateway_id: i64,
    pub currency_code: String, // ISO 4217
    pub status: InvoiceStatus,
    pub payment_initiated_at: Option<DateTime<Utc>>,
    pub subtotal_amount: i64,
    pub total_tax_amount: i64,
    pub total_service_fee_amount: i64,
    pub total_amount: i64,
    pub original_invoice_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Pending,
    PartiallyPaid,
    FullyPaid,
    Failed,
    Expired,
}

// models/line_item.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItem {
    pub id: i64,
    pub invoice_id: i64,
    pub product_name: String,
    pub quantity: i32,
    pub unit_price_amount: i64,
    pub tax_rate: Decimal,
    pub tax_category: Option<String>,
    pub country_code: Option<String>,
    pub subtotal_amount: i64,
    pub tax_amount: i64,
    pub created_at: DateTime<Utc>,
}

// models/installment_schedule.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallmentSchedule {
    pub id: i64,
    pub invoice_id: i64,
    pub installment_number: i32,
    pub due_date: DateTime<Utc>,
    pub amount: i64,
    pub tax_amount: i64,
    pub service_fee_amount: i64,
    pub total_amount: i64,
    pub status: InstallmentStatus,
    pub paid_at: Option<DateTime<Utc>>,
    pub payment_transaction_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallmentStatus {
    Unpaid,
    Paid,
    Overdue,
}

// models/payment_transaction.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentTransaction {
    pub id: i64,
    pub invoice_id: i64,
    pub installment_id: Option<i64>,
    pub gateway_id: i64,
    pub gateway_transaction_id: String,
    pub amount_received: i64,
    pub currency_code: String,
    pub payment_method: Option<String>,
    pub status: PaymentStatus,
    pub gateway_status_code: Option<String>,
    pub iso20022_message_id: Option<String>,
    pub iso20022_transaction_status: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub gateway_timestamp: Option<DateTime<Utc>>,
    pub gateway_response: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Completed,
    Failed,
}
```

---

## Key Relationships & Constraints

**Invoice → Line Items**: One-to-many
- Invoice must have ≥1 line item
- Line items deleted with invoice (CASCADE)

**Invoice → Installment Schedules**: One-to-many (optional)
- Invoice without installments = single payment
- Invoice with installments = 2-12 payment schedule entries

**Invoice → Payment Transactions**: One-to-many
- Tracks all payment attempts (successful + failed)
- Installment-level tracking via `installment_id` foreign key

**Installment Schedule → Payment Transaction**: Zero-or-one
- Installment paid when linked to completed transaction
- Overdue when due_date < now AND status = unpaid

**Webhook Events → Payment Transaction**: Zero-or-one
- Webhook may create new transaction or update existing

---

## Calculations & Formulas

### Invoice Total Calculation (per FR-049)
```
subtotal = SUM(line_item.subtotal_amount)
total_tax = SUM(line_item.tax_amount) where tax_amount = line_item.subtotal × line_item.tax_rate
service_fee = (subtotal × gateway.fee_percentage) + gateway.fee_fixed_amount
total_amount = subtotal + total_tax + service_fee
```

### Installment Distribution (per FR-052, FR-053, FR-063, FR-064)
```
For each installment (except last):
  installment_tax = total_tax × (installment_amount / total_amount)
  installment_fee = total_service_fee × (installment_amount / total_amount)
  installment_total = installment_amount + installment_tax + installment_fee

Last installment absorbs rounding:
  last_installment_total = total_amount - SUM(previous_installment_totals)
```

### Currency Amount Storage
```
IDR: store_amount = amount (whole numbers only)
MYR/USD: store_amount = amount × 100 (e.g., 1.50 → 150 cents)

Retrieval:
IDR: display_amount = store_amount
MYR/USD: display_amount = store_amount / 100.0
```

---

## Immutability Rules (FR-046)

Invoice becomes immutable when `payment_initiated_at IS NOT NULL`:
- Cannot modify line items (no additions/removals)
- Cannot change invoice amounts, currency, or gateway
- Cannot modify paid installment amounts
- **EXCEPTION**: Unpaid installment amounts can be adjusted per FR-066

---

## Multi-Tenant Isolation

All queries must filter by `tenant_id`:
```rust
// Good: Tenant-filtered
SELECT * FROM invoices WHERE tenant_id = ? AND id = ?

// Bad: Missing tenant filter
SELECT * FROM invoices WHERE id = ?
```

Enforced at repository layer; propagated from authentication middleware.

---

## Data Retention

- Transaction data: 7 years minimum (NFR-007)
- API key audit logs: 1 year minimum (NFR-011)
- Webhook events: 7 years minimum (includes original_payload for dispute resolution)
- Webhook retry logs: 7 years minimum (for audit trail)

---

## Next Steps

✅ Data model complete. Proceed to Phase 1 API contracts (contracts/)
