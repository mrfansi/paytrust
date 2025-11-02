# PayTrust Developer Quickstart Guide

**Version**: 1.0.0  
**Last Updated**: November 2, 2025  
**Target Audience**: Backend developers integrating PayTrust Payment Orchestration API

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [Configuration](#configuration)
4. [Database Setup](#database-setup)
5. [First Run](#first-run)
6. [Making Your First API Call](#making-your-first-api-call)
7. [Development Workflow](#development-workflow)
8. [Testing](#testing)
9. [Common Tasks](#common-tasks)
10. [Troubleshooting](#troubleshooting)

## Prerequisites

### Required Software

- **Rust 1.91.0+**: Install via [rustup](https://rustup.rs/)

  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup update
  ```

- **MySQL 8.0+**: Database server

  - macOS: `brew install mysql`
  - Ubuntu: `sudo apt install mysql-server`
  - Docker: `docker run -d -p 3306:3306 -e MYSQL_ROOT_PASSWORD=password mysql:8.0`

- **sqlx-cli**: Database migration tool
  ```bash
  cargo install sqlx-cli --no-default-features --features mysql
  ```

### Recommended Tools

- **Git**: Version control
- **VS Code** with rust-analyzer extension
- **cURL** or **Postman**: API testing
- **jq**: JSON formatting (`brew install jq`)

## Installation

### 1. Clone Repository

```bash
git clone https://github.com/mrfansi/paytrust.git
cd paytrust
git checkout 001-payment-orchestration-api
```

### 2. Verify Project Structure

```bash
tree -L 2 src/
```

Expected output:

```
src/
├── main.rs
├── lib.rs
├── config/
│   ├── mod.rs
│   ├── database.rs
│   └── server.rs
├── core/
│   ├── error.rs
│   ├── currency.rs
│   └── traits/
├── middleware/
│   ├── auth.rs
│   ├── rate_limit.rs
│   └── error_handler.rs
└── modules/
    ├── invoices/
    ├── installments/
    ├── transactions/
    └── gateways/
```

### 3. Install Dependencies

```bash
cargo build
```

This downloads and compiles all dependencies (~5-10 minutes first time).

## Configuration

### 1. Create Environment File

```bash
cp config/.env.example .env
```

### 2. Configure Environment Variables

Edit `.env`:

```env
# Application Settings
APP_ENV=development
APP_HOST=127.0.0.1
APP_PORT=8080
LOG_LEVEL=debug

# Database Configuration
DATABASE_URL=mysql://root:password@localhost:3306/paytrust_dev
DATABASE_POOL_SIZE=10
DATABASE_MAX_CONNECTIONS=20
DATABASE_TIMEOUT_SECONDS=30

# Payment Gateway: Xendit
XENDIT_API_KEY=xnd_development_your_key_here
XENDIT_WEBHOOK_SECRET=your_xendit_webhook_secret
XENDIT_BASE_URL=https://api.xendit.co

# Payment Gateway: Midtrans
MIDTRANS_SERVER_KEY=SB-Mid-server-your_key_here
MIDTRANS_WEBHOOK_SECRET=your_midtrans_webhook_secret
MIDTRANS_BASE_URL=https://api.sandbox.midtrans.com

# Security
API_KEY_SECRET=your_random_64_char_secret_key_here_change_this
RATE_LIMIT_PER_MINUTE=1000

# Business Defaults
DEFAULT_INVOICE_EXPIRY_HOURS=24
DEFAULT_CURRENCY=IDR
```

### 3. Generate API Key Secret

```bash
# Generate a secure random secret
openssl rand -base64 48
```

Use this value for `API_KEY_SECRET`.

## Database Setup

### 1. Create Databases

```bash
# Create development database
mysql -u root -p -e "CREATE DATABASE paytrust_dev CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;"

# Create test database
mysql -u root -p -e "CREATE DATABASE paytrust_test CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;"
```

### 2. Run Migrations

```bash
# Check migration status
sqlx migrate info

# Apply all migrations
sqlx migrate run

# Verify migrations applied
sqlx migrate info
```

Expected output:

```
Applied At                  Version  Description
─────────────────────────────────────────────────────
2025-11-02 10:00:00 UTC    1        create payment_gateways table
2025-11-02 10:00:01 UTC    2        create api_keys table
2025-11-02 10:00:02 UTC    3        create invoices table
2025-11-02 10:00:03 UTC    4        create line_items table
2025-11-02 10:00:04 UTC    5        create installment_schedules table
2025-11-02 10:00:05 UTC    6        create payment_transactions table
2025-11-02 10:00:06 UTC    7        add indexes and constraints
2025-11-02 10:00:07 UTC    8        add original_invoice_id
```

### 3. Seed Test Data (Optional)

```bash
mysql -u root -p paytrust_dev < scripts/seed_test_data.sql
```

## First Run

### 1. Start the Server

```bash
cargo run
```

Expected output:

```
2025-11-02T10:00:00.123Z  INFO paytrust: Starting PayTrust Payment Orchestration API
2025-11-02T10:00:00.234Z  INFO paytrust: Environment: development
2025-11-02T10:00:00.345Z  INFO paytrust: Database pool initialized: 10 connections
2025-11-02T10:00:00.456Z  INFO paytrust: Payment gateways loaded: 2
2025-11-02T10:00:00.567Z  INFO paytrust: Server listening at http://127.0.0.1:8080
```

### 2. Verify Health Check

```bash
curl http://127.0.0.1:8080/health | jq
```

Expected response:

```json
{
  "status": "healthy",
  "version": "1.0.0",
  "database": "connected",
  "timestamp": "2025-11-02T10:00:00Z"
}
```

### 3. Check Readiness

```bash
curl http://127.0.0.1:8080/ready | jq
```

Expected response:

```json
{
  "ready": true,
  "checks": {
    "database": "ok",
    "gateways": "ok"
  }
}
```

## Making Your First API Call

### 1. Create Test Gateway (Development Only)

Insert a test gateway directly into the database:

```sql
INSERT INTO payment_gateways (id, name, currency, fee_percentage, fee_fixed_amount, is_active, created_at, updated_at)
VALUES (
  'gateway-test-001',
  'Test Gateway IDR',
  'IDR',
  0.029,
  2000,
  true,
  NOW(),
  NOW()
);
```

Or use the admin script:

```bash
cargo run --bin seed-gateway
```

### 2. Create API Key

```bash
curl -X POST http://127.0.0.1:8080/admin/api-keys \
  -H "Content-Type: application/json" \
  -d '{
    "merchant_id": "merchant-dev-001",
    "rate_limit": 1000
  }' | jq
```

Response:

```json
{
  "api_key": "pk_dev_7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c",
  "merchant_id": "merchant-dev-001",
  "rate_limit": 1000,
  "created_at": "2025-11-02T10:05:00Z"
}
```

**Save this API key!** You'll need it for all subsequent requests.

### 3. Create Your First Invoice

```bash
export API_KEY="pk_dev_7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c"

curl -X POST http://127.0.0.1:8080/v1/invoices \
  -H "X-API-Key: $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "currency": "IDR",
    "gateway_id": "gateway-test-001",
    "line_items": [
      {
        "product_name": "Premium Subscription",
        "quantity": 1,
        "unit_price": "1000000",
        "tax_rate": "0.10"
      }
    ],
    "expires_at": "2025-11-03T10:00:00Z"
  }' | jq
```

Response:

```json
{
  "invoice_id": "inv_7f8a9b0c1d2e3f4a",
  "currency": "IDR",
  "status": "pending",
  "gateway_id": "gateway-test-001",
  "subtotal": "1000000",
  "tax_total": "100000",
  "service_fee": "31000",
  "total_amount": "1131000",
  "amount_paid": "0",
  "line_items": [
    {
      "line_item_id": "li_1a2b3c4d",
      "product_name": "Premium Subscription",
      "quantity": 1,
      "unit_price": "1000000",
      "subtotal": "1000000",
      "tax_rate": "0.10",
      "tax_amount": "100000"
    }
  ],
  "payment_urls": [
    {
      "installment_number": null,
      "url": "https://checkout.xendit.co/web/inv_7f8a9b0c1d2e3f4a"
    }
  ],
  "expires_at": "2025-11-03T10:00:00Z",
  "created_at": "2025-11-02T10:10:00Z",
  "updated_at": "2025-11-02T10:10:00Z"
}
```

### 4. Retrieve Invoice Details

```bash
curl http://127.0.0.1:8080/v1/invoices/inv_7f8a9b0c1d2e3f4a \
  -H "X-API-Key: $API_KEY" | jq
```

### 5. List All Invoices

```bash
curl "http://127.0.0.1:8080/v1/invoices?page=1&page_size=10" \
  -H "X-API-Key: $API_KEY" | jq
```

## Development Workflow

### Test-Driven Development (TDD)

PayTrust follows strict TDD workflow per Constitution Principle III:

1. **Write Test First** (test should fail)
2. **Implement Feature** (make test pass)
3. **Refactor** (improve code quality)
4. **Commit** (version control)

**Real Testing Requirement**: Integration and contract tests MUST use real databases and real
HTTP connections to test environments. Mocks are PROHIBITED for production behavior validation.
Unit tests may use mocks for isolated business logic only.

Example workflow:

```bash
# 1. Write test in tests/unit/
# tests/unit/new_feature_test.rs

# 2. Run test (should fail)
cargo test test_new_feature

# 3. Implement feature in src/

# 4. Run test (should pass)
cargo test test_new_feature

# 5. Run all tests
cargo test

# 6. Commit
git add .
git commit -m "feat: add new feature with tests"
```

### Running Tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test '*'

# Contract tests only
cargo test --test contract

# Specific test
cargo test test_invoice_calculation

# With output
cargo test -- --nocapture

# With test threads (parallel)
cargo test -- --test-threads=4
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Lint with Clippy
cargo clippy

# Lint with strict warnings
cargo clippy -- -D warnings

# Fix Clippy warnings automatically
cargo clippy --fix
```

### Hot Reload (Development)

Use `cargo-watch` for auto-reload:

```bash
# Install cargo-watch
cargo install cargo-watch

# Run with auto-reload
cargo watch -x run

# Run tests on file change
cargo watch -x test
```

## Testing

**Real Testing Policy**: Per Constitution Principle III, integration and contract tests MUST use
real databases and real infrastructure. Mocks are prohibited for production behavior validation.

### Unit Tests

Test business logic in isolation (mocks permitted here for isolated logic):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_calculate_service_fee() {
        let subtotal = Decimal::new(1000000, 0);
        let percentage = Decimal::new(29, 3); // 0.029
        let fixed = Decimal::new(2000, 0);

        let fee = (subtotal * percentage) + fixed;

        assert_eq!(fee, Decimal::new(31000, 0));
    }
}
```

### Integration Tests

Test with **real database** (REQUIRED - no in-memory DB):

```bash
# Set test database URL
export DATABASE_URL=mysql://root:password@localhost:3306/paytrust_test

# Run integration tests
cargo test --test payment_flow_test
```

### Contract Tests

Validate API against OpenAPI spec:

```bash
cargo test --test invoice_api_test
```

### Property-Based Tests

Use proptest for financial calculations:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_installment_sum_equals_total(
        total in 1000u64..10000000u64,
        count in 2u32..10u32
    ) {
        let installments = calculate_installments(total, count);
        let sum: u64 = installments.iter().sum();
        assert_eq!(sum, total);
    }
}
```

## Common Tasks

### Add a New API Endpoint

1. Define route in module controller:

```rust
// src/modules/invoices/controllers/invoice_controller.rs

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/invoices")
            .route(web::post().to(create_invoice))
            .route(web::get().to(list_invoices))
    );
}
```

2. Implement handler:

```rust
async fn create_invoice(
    req: web::Json<CreateInvoiceRequest>,
    service: web::Data<Arc<InvoiceService>>,
) -> Result<HttpResponse, AppError> {
    let invoice = service.create_invoice(req.into_inner()).await?;
    Ok(HttpResponse::Created().json(invoice))
}
```

3. Register in main.rs:

```rust
App::new()
    .configure(invoice_controller::configure)
```

### Add Database Migration

```bash
# Create migration
sqlx migrate add add_new_column

# Edit migrations/{timestamp}_add_new_column.sql
# ALTER TABLE invoices ADD COLUMN new_field VARCHAR(255);

# Apply migration
sqlx migrate run
```

### Add New Currency

1. Update Currency enum:

```rust
// src/core/currency.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Currency {
    IDR,
    MYR,
    USD,
    EUR, // New currency
}
```

2. Add scale configuration:

```rust
impl Currency {
    pub fn scale(&self) -> u32 {
        match self {
            Currency::IDR => 0,
            Currency::EUR => 2, // Add scale
            _ => 2,
        }
    }
}
```

3. Update tests and validation

### Debug with Logs

```rust
use tracing::{info, debug, warn, error};

#[tracing::instrument(skip(self))]
async fn create_invoice(&self, req: CreateInvoiceRequest) -> Result<Invoice> {
    info!("Creating invoice for merchant: {}", req.merchant_id);
    debug!("Request details: {:?}", req);

    // ... implementation

    info!("Invoice created: {}", invoice.id);
    Ok(invoice)
}
```

View logs:

```bash
RUST_LOG=debug cargo run
```

## Troubleshooting

### Database Connection Failed

**Error**: `Connection refused (os error 111)`

**Solutions**:

```bash
# Check MySQL is running
mysql -u root -p -e "SELECT 1;"

# Verify DATABASE_URL in .env
echo $DATABASE_URL

# Test connection
mysql -u root -p paytrust_dev -e "SHOW TABLES;"
```

### Migration Failed

**Error**: `Migration 001 already applied`

**Solutions**:

```bash
# Check migration status
sqlx migrate info

# Revert last migration
sqlx migrate revert

# Reapply
sqlx migrate run
```

### Port Already in Use

**Error**: `Address already in use (os error 48)`

**Solutions**:

```bash
# Find process using port 8080
lsof -ti:8080

# Kill process
kill -9 $(lsof -ti:8080)

# Or change port in .env
APP_PORT=8081
```

### Compilation Errors

**Error**: `Cannot find type 'X' in scope`

**Solutions**:

```bash
# Clean build artifacts
cargo clean

# Rebuild
cargo build

# Update dependencies
cargo update
```

### Rate Limit Hit

**Error**: `429 Too Many Requests`

**Solutions**:

```bash
# Increase rate limit in .env
RATE_LIMIT_PER_MINUTE=10000

# Or wait 60 seconds

# Or use different API key
```

## Next Steps

- **API Examples**: See [docs/examples/](./examples/) for detailed usage scenarios
- **API Reference**: Review [specs/001-payment-orchestration-api/contracts/openapi.yaml](../specs/001-payment-orchestration-api/contracts/openapi.yaml)
- **Deployment**: Follow [docs/deployment.md](./deployment.md) for production setup
- **Contributing**: Read Constitution at `.specify/memory/constitution.md`

## Support & Resources

- **Specification**: `specs/001-payment-orchestration-api/spec.md`
- **Data Model**: `specs/001-payment-orchestration-api/data-model.md`
- **Task List**: `specs/001-payment-orchestration-api/tasks.md`
- **Xendit Docs**: Use Context7 MCP for latest documentation
- **Midtrans Docs**: Use Context7 MCP for latest documentation

---

**Happy Coding!** 🚀

Remember: Test First, then implement (TDD per Constitution Principle III)
