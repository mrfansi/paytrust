# Data Model: PayTrust Payment Orchestration Platform

**Date**: 2025-11-01  
**Status**: Phase 1 Complete

## Entity Relationship Diagram

```
┌─────────────────┐
│    Invoice      │
│─────────────────│
│ id (PK)         │──┐
│ merchant_id     │  │
│ currency        │  │         ┌──────────────────┐
│ subtotal        │  │         │    LineItem      │
│ tax_total       │  ├────────<│──────────────────│
│ service_fee     │  │         │ id (PK)          │
│ total_amount    │  │         │ invoice_id (FK)  │
│ status          │  │         │ product_name     │
│ gateway_id (FK) │  │         │ quantity         │
│ original_inv_id │  │         │ unit_price       │
│ is_immutable    │  │         │ subtotal         │
│ expires_at      │  │         │ tax_rate         │
│ created_at      │  │         │ tax_category     │
│ updated_at      │  │         │ tax_amount       │
└─────────────────┘  │         └──────────────────┘
        │            │
        │            │         ┌──────────────────────┐
        │            │         │ InstallmentSchedule  │
        │            └────────<│──────────────────────│
        │                      │ id (PK)              │
        │                      │ invoice_id (FK)      │
        │                      │ installment_number   │
        │                      │ amount               │
        │                      │ tax_amount           │
        │                      │ service_fee_amount   │
        │                      │ due_date             │
        │                      │ status               │
        │                      │ payment_url          │
        │                      │ gateway_ref          │
        │                      │ paid_at              │
        │                      └──────────────────────┘
        │
        │                      ┌──────────────────────┐
        │                      │  PaymentTransaction  │
        └─────────────────────<│──────────────────────│
                               │ id (PK)              │
                               │ invoice_id (FK)      │
                               │ installment_id (FK)  │
                               │ gateway_tx_ref       │
                               │ amount_paid          │
                               │ payment_method       │
                               │ status               │
                               │ gateway_response     │
                               │ created_at           │
                               └──────────────────────┘

┌─────────────────────┐
│  PaymentGateway     │
│─────────────────────│
│ id (PK)             │
│ name                │
│ supported_currencies│
│ fee_percentage      │
│ fee_fixed           │
│ api_key_encrypted   │
│ webhook_secret      │
│ is_active           │
│ created_at          │
└─────────────────────┘

┌─────────────────────┐
│   ApiKey            │
│─────────────────────│
│ id (PK)             │
│ key_hash            │
│ merchant_id         │
│ rate_limit          │
│ is_active           │
│ last_used_at        │
│ created_at          │
└─────────────────────┘
```

## Entity Definitions

### 1. Invoice

**Purpose**: Represents a payment request with line items, taxes, and service fees.

**Fields**:
| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | VARCHAR(36) | PK, UUID | Unique invoice identifier |
| merchant_id | VARCHAR(36) | FK, NOT NULL, INDEX | Merchant/developer who owns invoice |
| currency | ENUM('IDR','MYR','USD') | NOT NULL | Invoice currency |
| subtotal | DECIMAL(19,4) | NOT NULL | Sum of all line item subtotals |
| tax_total | DECIMAL(19,4) | NOT NULL, DEFAULT 0 | Sum of all taxes |
| service_fee | DECIMAL(19,4) | NOT NULL, DEFAULT 0 | Payment gateway service fee |
| total_amount | DECIMAL(19,4) | NOT NULL | subtotal + tax_total + service_fee |
| status | ENUM | NOT NULL, DEFAULT 'draft' | draft, pending, partially_paid, paid, failed, expired |
| gateway_id | VARCHAR(36) | FK, INDEX | Selected payment gateway |
| original_invoice_id | VARCHAR(36) | FK, NULL, INDEX | Reference for supplementary invoices |
| is_immutable | BOOLEAN | NOT NULL, DEFAULT false | Locked after payment initiation |
| expires_at | TIMESTAMP | NOT NULL | Invoice expiration (default 24h from creation) |
| created_at | TIMESTAMP | NOT NULL, DEFAULT NOW() | Creation timestamp |
| updated_at | TIMESTAMP | NOT NULL, DEFAULT NOW() ON UPDATE | Last modification timestamp |

**Indexes**:
- PRIMARY KEY (id)
- INDEX (merchant_id, status)
- INDEX (merchant_id, created_at)
- INDEX (gateway_id)
- INDEX (original_invoice_id)

**Validation Rules** (from spec):
- FR-051: is_immutable set to true once payment initiated
- FR-052: Reject modifications when is_immutable = true (except unpaid installments)
- FR-044: expires_at default = created_at + 24 hours
- FR-081: Reject line item additions when is_immutable = true

**State Transitions**:
```
draft → pending (payment initiated)
pending → partially_paid (first installment paid or partial payment)
pending → paid (full payment received)
pending → failed (gateway error)
pending → expired (expires_at reached without payment)
partially_paid → paid (all installments paid or total reached)
partially_paid → expired (expires_at reached)
```

---

### 2. LineItem

**Purpose**: Individual product/service entry in an invoice with its own tax rate.

**Fields**:
| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | VARCHAR(36) | PK, UUID | Unique line item identifier |
| invoice_id | VARCHAR(36) | FK, NOT NULL, INDEX | Parent invoice |
| product_name | VARCHAR(255) | NOT NULL | Product/service name |
| quantity | DECIMAL(10,2) | NOT NULL, > 0 | Quantity purchased |
| unit_price | DECIMAL(19,4) | NOT NULL, >= 0 | Price per unit |
| subtotal | DECIMAL(19,4) | NOT NULL | quantity × unit_price |
| tax_rate | DECIMAL(5,4) | NOT NULL, >= 0, <= 1 | Tax rate as decimal (0.10 = 10%) |
| tax_category | VARCHAR(50) | NULL | Optional tax category identifier |
| tax_amount | DECIMAL(19,4) | NOT NULL | subtotal × tax_rate |
| created_at | TIMESTAMP | NOT NULL, DEFAULT NOW() | Creation timestamp |

**Indexes**:
- PRIMARY KEY (id)
- INDEX (invoice_id)

**Validation Rules** (from spec):
- FR-001: Contains product name, quantity, unit price, subtotal
- FR-005: subtotal = quantity × unit_price (calculated)
- FR-057: Each line item has own tax_rate
- FR-058: tax_amount = subtotal × tax_rate (calculated)

**Calculation Formula**:
```rust
subtotal = quantity * unit_price
tax_amount = subtotal * tax_rate
```

---

### 3. InstallmentSchedule

**Purpose**: Payment plan entry for installment-based invoices.

**Fields**:
| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | VARCHAR(36) | PK, UUID | Unique installment identifier |
| invoice_id | VARCHAR(36) | FK, NOT NULL, INDEX | Parent invoice |
| installment_number | INT | NOT NULL, > 0 | Sequential number (1, 2, 3...) |
| amount | DECIMAL(19,4) | NOT NULL, > 0 | Installment payment amount |
| tax_amount | DECIMAL(19,4) | NOT NULL, >= 0 | Proportionally distributed tax |
| service_fee_amount | DECIMAL(19,4) | NOT NULL, >= 0 | Proportionally distributed fee |
| due_date | DATE | NOT NULL | Payment due date |
| status | ENUM | NOT NULL, DEFAULT 'unpaid' | unpaid, paid, overdue |
| payment_url | TEXT | NULL | Gateway-generated payment link |
| gateway_reference | VARCHAR(255) | NULL, INDEX | Gateway transaction reference |
| paid_at | TIMESTAMP | NULL | Payment completion timestamp |
| created_at | TIMESTAMP | NOT NULL, DEFAULT NOW() | Creation timestamp |
| updated_at | TIMESTAMP | NOT NULL, DEFAULT NOW() ON UPDATE | Last modification timestamp |

**Indexes**:
- PRIMARY KEY (id)
- UNIQUE (invoice_id, installment_number)
- INDEX (invoice_id, status)
- INDEX (gateway_reference)

**Validation Rules** (from spec):
- FR-014: 2-12 installments allowed
- FR-017: SUM(amount) = invoice.total_amount
- FR-059: tax_amount = invoice.tax_total × (amount / invoice.total_amount)
- FR-060: service_fee_amount = invoice.service_fee × (amount / invoice.total_amount)
- FR-068: Sequential payment order enforced
- FR-071: Rounding handled by last installment
- FR-072: Last installment amount = total - SUM(previous installments)
- FR-077: Unpaid installments can be adjusted (paid cannot)

**Proportional Distribution Formula**:
```rust
installment_tax = total_tax * (installment_amount / total_amount)
installment_service_fee = total_service_fee * (installment_amount / total_amount)

// Last installment absorbs rounding
last_installment_amount = total_amount - sum(previous_installment_amounts)
```

**State Transitions**:
```
unpaid → paid (payment received via webhook)
unpaid → overdue (due_date passed without payment)
```

---

### 4. PaymentTransaction

**Purpose**: Record of actual payment attempt or completion.

**Fields**:
| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | VARCHAR(36) | PK, UUID | Unique transaction identifier |
| invoice_id | VARCHAR(36) | FK, NOT NULL, INDEX | Related invoice |
| installment_id | VARCHAR(36) | FK, NULL, INDEX | Related installment (if applicable) |
| gateway_transaction_ref | VARCHAR(255) | NOT NULL, UNIQUE, INDEX | Gateway's transaction ID |
| amount_paid | DECIMAL(19,4) | NOT NULL | Actual amount received |
| payment_method | VARCHAR(50) | NOT NULL | e.g., credit_card, bank_transfer, ewallet |
| status | ENUM | NOT NULL | pending, completed, failed, refunded |
| gateway_response | JSON | NULL | Full gateway webhook payload |
| created_at | TIMESTAMP | NOT NULL, DEFAULT NOW() | Transaction creation timestamp |
| updated_at | TIMESTAMP | NOT NULL, DEFAULT NOW() ON UPDATE | Last update timestamp |

**Indexes**:
- PRIMARY KEY (id)
- UNIQUE (gateway_transaction_ref)
- INDEX (invoice_id)
- INDEX (installment_id)
- INDEX (status, created_at)

**Validation Rules** (from spec):
- FR-030: Store complete transaction history with timestamps, amounts, gateway responses
- FR-032: Idempotent requests via gateway_transaction_ref uniqueness
- FR-048: Accept partial payments (amount_paid != invoice.total_amount)
- FR-073: Accept overpayments on installments

**Overpayment Handling** (FR-074, FR-075, FR-076):
```
IF amount_paid > installment.amount:
    excess = amount_paid - installment.amount
    mark installment as paid
    apply excess to next unpaid installments sequentially
    IF total_paid >= invoice.total_amount:
        mark invoice as fully_paid
```

---

### 5. PaymentGateway

**Purpose**: Configuration for payment gateway integrations (Xendit, Midtrans).

**Fields**:
| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | VARCHAR(36) | PK, UUID | Unique gateway identifier |
| name | ENUM('xendit','midtrans') | NOT NULL, UNIQUE | Gateway name |
| supported_currencies | JSON | NOT NULL | Array of currency codes ['IDR','MYR','USD'] |
| fee_percentage | DECIMAL(5,4) | NOT NULL, >= 0 | Percentage fee (0.029 = 2.9%) |
| fee_fixed | DECIMAL(19,4) | NOT NULL, >= 0 | Fixed fee amount |
| api_key_encrypted | BLOB | NOT NULL | Encrypted API credentials |
| webhook_secret | VARCHAR(255) | NOT NULL | Webhook signature verification secret |
| webhook_url | TEXT | NOT NULL | Webhook endpoint URL |
| is_active | BOOLEAN | NOT NULL, DEFAULT true | Gateway availability |
| environment | ENUM('sandbox','production') | NOT NULL | Sandbox or live |
| created_at | TIMESTAMP | NOT NULL, DEFAULT NOW() | Creation timestamp |
| updated_at | TIMESTAMP | NOT NULL, DEFAULT NOW() ON UPDATE | Last update timestamp |

**Indexes**:
- PRIMARY KEY (id)
- UNIQUE (name, environment)

**Validation Rules** (from spec):
- FR-007: Developer chooses gateway per invoice
- FR-046: Validate gateway supports invoice currency
- FR-009: fee_percentage and fee_fixed for service fee calculation
- FR-047: Service fee = (subtotal × fee_percentage) + fee_fixed

**Service Fee Calculation**:
```rust
service_fee = (invoice.subtotal * gateway.fee_percentage) + gateway.fee_fixed
```

---

### 6. ApiKey

**Purpose**: Developer authentication and rate limiting.

**Fields**:
| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | VARCHAR(36) | PK, UUID | Unique API key identifier |
| key_hash | VARCHAR(64) | NOT NULL, UNIQUE, INDEX | SHA-256 hash of API key |
| merchant_id | VARCHAR(36) | FK, NOT NULL, INDEX | Owning merchant |
| rate_limit | INT | NOT NULL, DEFAULT 1000 | Requests per minute |
| is_active | BOOLEAN | NOT NULL, DEFAULT true | Key validity |
| last_used_at | TIMESTAMP | NULL | Last request timestamp |
| created_at | TIMESTAMP | NOT NULL, DEFAULT NOW() | Creation timestamp |
| updated_at | TIMESTAMP | NOT NULL, DEFAULT NOW() ON UPDATE | Last update timestamp |

**Indexes**:
- PRIMARY KEY (id)
- UNIQUE (key_hash)
- INDEX (merchant_id)

**Validation Rules** (from spec):
- FR-033: Authenticate via X-API-Key header
- FR-037: Reject invalid/missing keys with 401
- FR-040: Enforce rate_limit (default 1000 req/min)
- FR-041: Return 429 when exceeded

**Rate Limiting Algorithm**:
```
Sliding window: track request timestamps per key
IF requests_in_last_minute >= rate_limit:
    RETURN 429 with Retry-After header
```

---

### 7. Tax (Reporting Aggregate)

**Purpose**: Aggregated tax data for financial reports (not a core transactional entity).

**Note**: Tax data primarily stored in LineItem.tax_amount and InstallmentSchedule.tax_amount. This entity/view used for reporting only.

**Report Structure** (from FR-063, FR-064):
```json
{
  "tax_breakdown": [
    {
      "currency": "IDR",
      "tax_rate": "0.10",
      "total_amount": "50000000",
      "transaction_count": 450
    },
    {
      "currency": "IDR",
      "tax_rate": "0.00",
      "total_amount": "0",
      "transaction_count": 50
    },
    {
      "currency": "MYR",
      "tax_rate": "0.06",
      "total_amount": "12500.00",
      "transaction_count": 80
    }
  ]
}
```

**Query Pattern**:
```sql
SELECT 
    i.currency,
    li.tax_rate,
    SUM(li.tax_amount) as total_amount,
    COUNT(DISTINCT i.id) as transaction_count
FROM invoices i
JOIN line_items li ON li.invoice_id = i.id
WHERE i.status = 'paid'
    AND i.created_at >= ? AND i.created_at <= ?
GROUP BY i.currency, li.tax_rate
ORDER BY i.currency, li.tax_rate;
```

---

## Validation Rules Summary

### Invoice Level

- **FR-017**: SUM(installment amounts) = invoice.total_amount
- **FR-051**: is_immutable = true after payment initiated
- **FR-055**: tax calculated on subtotal only (exclude service_fee)
- **FR-056**: total_amount = subtotal + tax_total + service_fee
- **FR-081**: Reject line item additions when is_immutable = true

### Line Item Level

- **FR-057**: Each line item has own tax_rate
- **FR-058**: tax_amount = subtotal × tax_rate
- Subtotal = quantity × unit_price

### Installment Level

- **FR-059**: Proportional tax distribution
- **FR-060**: Proportional service fee distribution
- **FR-068**: Sequential payment order (installment N requires N-1 paid)
- **FR-071/FR-072**: Last installment absorbs rounding difference
- **FR-077**: Unpaid installments adjustable (paid locked)
- **FR-079**: SUM(adjusted unpaid amounts) = remaining balance

### Transaction Level

- **FR-032**: Idempotent via unique gateway_transaction_ref
- **FR-073**: Accept overpayments
- **FR-074/FR-075/FR-076**: Auto-apply excess to next installments

### Currency Level

- **FR-023**: No currency mixing within invoice
- **FR-024**: Reject payment in different currency
- **FR-026**: IDR scale=0, MYR/USD scale=2

---

## Database Migrations

**Migration Order**:
1. `001_create_payment_gateways_table.sql` - Independent table
2. `002_create_api_keys_table.sql` - Independent table (merchant FK external)
3. `003_create_invoices_table.sql` - References payment_gateways
4. `004_create_line_items_table.sql` - References invoices
5. `005_create_installment_schedules_table.sql` - References invoices
6. `006_create_payment_transactions_table.sql` - References invoices, installment_schedules
7. `007_add_indexes.sql` - Performance indexes

**Rollback Strategy**: Each migration has DOWN script that reverses changes

---

## Data Integrity Constraints

### Foreign Key Constraints

```sql
ALTER TABLE line_items 
ADD CONSTRAINT fk_line_items_invoice 
FOREIGN KEY (invoice_id) REFERENCES invoices(id) 
ON DELETE CASCADE;

ALTER TABLE installment_schedules 
ADD CONSTRAINT fk_installments_invoice 
FOREIGN KEY (invoice_id) REFERENCES invoices(id) 
ON DELETE CASCADE;

ALTER TABLE payment_transactions 
ADD CONSTRAINT fk_transactions_invoice 
FOREIGN KEY (invoice_id) REFERENCES invoices(id) 
ON DELETE RESTRICT;

ALTER TABLE payment_transactions 
ADD CONSTRAINT fk_transactions_installment 
FOREIGN KEY (installment_id) REFERENCES installment_schedules(id) 
ON DELETE SET NULL;
```

### Check Constraints

```sql
ALTER TABLE line_items 
ADD CONSTRAINT chk_quantity_positive CHECK (quantity > 0);

ALTER TABLE line_items 
ADD CONSTRAINT chk_tax_rate_range CHECK (tax_rate >= 0 AND tax_rate <= 1);

ALTER TABLE installment_schedules 
ADD CONSTRAINT chk_installment_number_positive CHECK (installment_number > 0);

ALTER TABLE invoices 
ADD CONSTRAINT chk_valid_status CHECK (status IN ('draft', 'pending', 'partially_paid', 'paid', 'failed', 'expired'));
```

---

## Phase 1 Completion Checklist

- [x] All entities from spec identified and defined
- [x] Field types and constraints specified
- [x] Relationships and foreign keys defined
- [x] Validation rules mapped to functional requirements
- [x] State transitions documented
- [x] Indexes for query performance defined
- [x] Migration order established
- [x] Data integrity constraints specified
- [x] Proportional distribution formulas documented
- [x] Currency decimal handling specified

**Status**: Data model complete, ready for contract generation
