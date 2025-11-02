# QuickStart: PayTrust Payment Orchestration Platform

**Last Updated**: 2025-11-01  
**Target Audience**: Developers implementing PayTrust

## Prerequisites

- Rust 1.75+ installed (`rustup update`)
- MySQL 8.0+ running locally or accessible remotely
- Git for version control
- Text editor/IDE with Rust support (VS Code + rust-analyzer recommended)

## Setup Steps

### 1. Clone Repository

```bash
git clone <repository-url> paytrust
cd paytrust
git checkout 001-payment-orchestration-api
```

### 2. Environment Configuration

Create `.env` file from template:

```bash
cp config/.env.example .env
```

Edit `.env` with your local configuration:

```env
# Application
APP_ENV=development
APP_HOST=127.0.0.1
APP_PORT=8080
LOG_LEVEL=debug

# Database
DATABASE_URL=mysql://root:password@localhost:3306/paytrust_dev
DATABASE_POOL_SIZE=10
DATABASE_MAX_CONNECTIONS=20

# Payment Gateways
XENDIT_API_KEY=xnd_development_...
XENDIT_WEBHOOK_SECRET=your_xendit_webhook_secret
XENDIT_BASE_URL=https://api.xendit.co

MIDTRANS_SERVER_KEY=SB-Mid-server-...
MIDTRANS_WEBHOOK_SECRET=your_midtrans_webhook_secret
MIDTRANS_BASE_URL=https://api.sandbox.midtrans.com

# Security
API_KEY_SECRET=your_random_secret_key_here
RATE_LIMIT_PER_MINUTE=1000

# Defaults
DEFAULT_INVOICE_EXPIRY_HOURS=24
```

### 3. Database Setup

Create MySQL database:

```bash
mysql -u root -p -e "CREATE DATABASE paytrust_dev CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;"
mysql -u root -p -e "CREATE DATABASE paytrust_test CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;"
```

Install sqlx-cli for migrations:

```bash
cargo install sqlx-cli --no-default-features --features mysql
```

Run migrations:

```bash
sqlx migrate run
```

Verify migration status:

```bash
sqlx migrate info
```

### 4. Install Dependencies

```bash
cargo build
```

This will download and compile all dependencies defined in `Cargo.toml`:

- actix-web (HTTP server)
- sqlx (MySQL driver with async support)
- serde/serde_json (JSON serialization)
- dotenvy (environment configuration)
- thiserror (error handling)
- tracing/tracing-subscriber (logging)
- reqwest (HTTP client for gateways)
- rust_decimal (currency arithmetic)
- governor (rate limiting)

### 5. Run Tests

Run all tests to verify setup:

```bash
# Unit tests only
cargo test --lib

# Integration tests (requires test database)
cargo test --test '*'

# Contract tests
cargo test --test contract

# All tests with output
cargo test -- --nocapture
```

### 6. Start Development Server

```bash
cargo run
```

Server will start on `http://127.0.0.1:8080`

You should see:

```
2025-11-01T10:00:00.123Z  INFO paytrust: Server starting at http://127.0.0.1:8080
2025-11-01T10:00:00.456Z  INFO paytrust: Database pool initialized (10 connections)
2025-11-01T10:00:00.789Z  INFO paytrust: Payment gateways loaded: xendit, midtrans
```

### 7. Test API

Create a test API key:

```bash
curl -X POST http://127.0.0.1:8080/admin/api-keys \
  -H "Content-Type: application/json" \
  -d '{
    "merchant_id": "merchant-123",
    "rate_limit": 1000
  }'
```

Response:

```json
{
  "api_key": "pk_dev_abc123xyz...",
  "merchant_id": "merchant-123",
  "rate_limit": 1000
}
```

Create a test invoice:

```bash
curl -X POST http://127.0.0.1:8080/v1/invoices \
  -H "X-API-Key: pk_dev_abc123xyz..." \
  -H "Content-Type: application/json" \
  -d '{
    "currency": "IDR",
    "gateway_id": "gateway-xendit-sandbox",
    "line_items": [
      {
        "product_name": "Premium Subscription",
        "quantity": "1",
        "unit_price": "1000000",
        "tax_rate": "0.10"
      }
    ]
  }'
```

Expected response:

```json
{
  "id": "inv-550e8400-e29b-41d4-a716-446655440000",
  "merchant_id": "merchant-123",
  "currency": "IDR",
  "subtotal": "1000000",
  "tax_total": "100000",
  "service_fee": "31000",
  "total_amount": "1131000",
  "status": "pending",
  "payment_url": "https://checkout.xendit.co/web/...",
  "is_immutable": true,
  "expires_at": "2025-11-02T10:00:00Z",
  "created_at": "2025-11-01T10:00:00Z",
  "line_items": [
    {
      "product_name": "Premium Subscription",
      "quantity": "1",
      "unit_price": "1000000",
      "subtotal": "1000000",
      "tax_rate": "0.10",
      "tax_amount": "100000"
    }
  ]
}
```

## Development Workflow

### Test-Driven Development (TDD)

Per Constitution Principle III, follow strict TDD:

1. **Write Test First**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_invoice_calculates_totals_correctly() {
        // Arrange
        let invoice = CreateInvoiceRequest {
            currency: Currency::IDR,
            line_items: vec![
                LineItem {
                    product_name: "Test Product".to_string(),
                    quantity: Decimal::new(2, 0),
                    unit_price: Decimal::new(500000, 0),
                    tax_rate: Decimal::new(10, 2), // 0.10
                    tax_category: None,
                }
            ],
            // ... other fields
        };

        // Act
        let result = service.create_invoice(invoice).await;

        // Assert
        assert!(result.is_ok());
        let invoice = result.unwrap();
        assert_eq!(invoice.subtotal, Decimal::new(1000000, 0));
        assert_eq!(invoice.tax_total, Decimal::new(100000, 0));
    }
}
```

2. **Run Test (should fail)**: `cargo test test_create_invoice_calculates_totals_correctly`

3. **Implement Feature**:

```rust
pub async fn create_invoice(&self, request: CreateInvoiceRequest) -> Result<Invoice, AppError> {
    // Calculate subtotal
    let subtotal: Decimal = request.line_items.iter()
        .map(|item| item.quantity * item.unit_price)
        .sum();

    // Calculate tax total (per-line-item, FR-058)
    let tax_total: Decimal = request.line_items.iter()
        .map(|item| (item.quantity * item.unit_price) * item.tax_rate)
        .sum();

    // Calculate service fee (FR-047)
    let gateway = self.gateway_repo.find_by_id(&request.gateway_id).await?;
    let service_fee = (subtotal * gateway.fee_percentage) + gateway.fee_fixed;

    // Total (FR-056)
    let total_amount = subtotal + tax_total + service_fee;

    // ... create invoice
}
```

4. **Run Test (should pass)**: `cargo test test_create_invoice_calculates_totals_correctly`

5. **Refactor if needed**

### Module Development

Each module is self-contained. Example: developing installment module.

**File Structure**:

```
src/modules/installments/
├── mod.rs              # Public interface
├── models/
│   └── installment_schedule.rs
├── repositories/
│   └── installment_repository.rs
├── services/
│   ├── installment_calculator.rs
│   └── installment_service.rs
└── controllers/
    └── installment_controller.rs
```

**Module Interface (mod.rs)**:

```rust
mod models;
mod repositories;
mod services;
mod controllers;

// Public exports
pub use models::InstallmentSchedule;
pub use services::{InstallmentService, InstallmentCalculator};
pub use controllers::InstallmentController;

// Trait definitions for dependency injection
pub use repositories::InstallmentRepository;
```

**Dependency Injection Pattern**:

```rust
// In main.rs or module initialization
let installment_repo = Arc::new(MySqlInstallmentRepository::new(pool.clone()));
let installment_calculator = Arc::new(InstallmentCalculator::new());
let installment_service = Arc::new(InstallmentService::new(
    installment_repo.clone(),
    installment_calculator.clone(),
));
```

### Database Migrations

Create a new migration:

```bash
sqlx migrate add create_new_table
```

This creates: `migrations/{timestamp}_create_new_table.sql`

Write migration:

```sql
-- Up migration
CREATE TABLE new_table (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_new_table_name ON new_table(name);

-- Down migration (in separate file or commented)
-- DROP TABLE new_table;
```

Apply migration:

```bash
sqlx migrate run
```

Rollback (if needed):

```bash
sqlx migrate revert
```

### API Contract Testing

Contract tests ensure API conforms to OpenAPI spec:

```rust
#[actix_web::test]
async fn test_create_invoice_contract() {
    let app = test::init_service(App::new().configure(configure_routes)).await;

    let request_body = json!({
        "currency": "IDR",
        "gateway_id": "gateway-test",
        "line_items": [
            {
                "product_name": "Test",
                "quantity": "1",
                "unit_price": "100000",
                "tax_rate": "0.10"
            }
        ]
    });

    let req = test::TestRequest::post()
        .uri("/v1/invoices")
        .insert_header(("X-API-Key", "test-key"))
        .set_json(&request_body)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 201);

    let body: Value = test::read_body_json(resp).await;
    assert!(body.get("id").is_some());
    assert_eq!(body["currency"], "IDR");
    assert_eq!(body["status"], "pending");
}
```

## Common Tasks

### Add a New Module

1. Create module directory: `src/modules/new_module/`
2. Create subfolders: `models/`, `repositories/`, `services/`, `controllers/`
3. Define traits in `src/core/traits/`
4. Implement module following SOLID principles
5. Write tests before implementation (TDD)
6. Export public interface via `mod.rs`

### Add Payment Gateway Support

1. Implement `PaymentGateway` trait:

```rust
#[async_trait]
pub trait PaymentGateway: Send + Sync {
    async fn create_payment(&self, request: PaymentRequest) -> Result<PaymentResponse, AppError>;
    async fn verify_webhook(&self, signature: &str, payload: &str) -> Result<bool, AppError>;
}
```

2. Create gateway-specific implementation in `src/modules/gateways/services/`
3. Add configuration to `.env`
4. Register in gateway factory

### Debug Common Issues

**Database Connection Error**:

```
Error: Connection refused (os error 111)
```

Solution: Check MySQL is running and DATABASE_URL is correct

```bash
# Check MySQL status
brew services list | grep mysql  # macOS
sudo systemctl status mysql      # Linux

# Test connection manually
mysql -u root -p -e "SELECT 1;"
```

**Migration Error**:

```
Error: Migration 001 already applied
```

Solution: Check migration status with `sqlx migrate info`, revert if needed

```bash
# Check status
sqlx migrate info

# Revert last migration if needed
sqlx migrate revert

# Re-apply
sqlx migrate run
```

**Test Database Isolation Issues**:

```
Error: Duplicate entry 'TEST-001' for key 'invoices.external_id'
```

Solution: Use UUID-based test data generation

```rust
// Bad: Hardcoded IDs cause conflicts
let invoice_id = "TEST-001";

// Good: Generate unique IDs
use uuid::Uuid;
let invoice_id = format!("INV_{}", Uuid::new_v4());
```

**Payment Gateway API Errors**:

```
Error: 401 Unauthorized from Xendit API
```

Solution: Verify gateway API keys are correct

```bash
# Test Xendit API key
curl https://api.xendit.co/v2/invoices \
  -u xnd_development_YOUR_KEY: \
  -X GET

# Test Midtrans API key
curl https://api.sandbox.midtrans.com/v2/ping \
  -H "Authorization: Basic $(echo -n SB-Mid-server-YOUR_KEY: | base64)"
```

**Webhook Signature Validation Failures**:

```
Error: Invalid webhook signature
```

Solution: Ensure webhook secrets match gateway configuration

```bash
# Verify .env has correct secrets
grep WEBHOOK_SECRET .env

# Test webhook locally with simulation
cargo test test_xendit_paid_webhook_updates_invoice
```

**Parallel Test Failures**:

```
Error: Tests pass individually but fail when run in parallel
```

Solution: Ensure test data isolation

```rust
// Each test should use unique data
let external_id = format!("TEST_{}", Uuid::new_v4());

// Use transactions for database cleanup
use tests::helpers::test_database::with_transaction;

with_transaction(|mut tx| async move {
    // Test code - auto-rollback
}).await;
```

**Rate Limit Testing**:

```bash
# Simulate rate limit
for i in {1..1001}; do
  curl -H "X-API-Key: test-key" http://127.0.0.1:8080/v1/invoices &
done
```

**Docker Test Environment Issues**:

```
Error: Cannot connect to MySQL in Docker
```

Solution: Use Docker Compose test environment

```bash
# Start test environment
./scripts/test_with_docker.sh

# Or manually with Docker Compose
docker-compose -f docker-compose.test.yml up -d
cargo test
docker-compose -f docker-compose.test.yml down
```

**CI/CD Pipeline Failures**:

```
Error: Tests fail in GitHub Actions but pass locally
```

Solution: Check environment differences

```bash
# Run tests with Docker locally (simulates CI)
./scripts/test_with_docker.sh

# Check test logs in GitHub Actions
# Look for database connection or gateway API issues
```

**Performance Test Failures**:

```
Error: Test exceeded 200ms threshold
```

Solution: Check database connection pool and query optimization

```rust
// Optimize queries with indexes
CREATE INDEX idx_invoices_external_id ON invoices(external_id);

// Use connection pooling
DATABASE_POOL_SIZE=10
DATABASE_MAX_CONNECTIONS=20

// Batch operations instead of N+1 queries
let invoices = repo.find_with_line_items(ids).await?;
```

**Missing Test Helper Imports**:

```
Error: unresolved import `tests::helpers`
```

Solution: Use correct test helper imports

```rust
// In integration tests (tests/integration/*.rs)
use tests::helpers::*;

// Individual helpers
use tests::helpers::{
    spawn_test_server,
    TestClient,
    TestDataFactory,
    XenditSandbox,
    MidtransSandbox
};
```

## Troubleshooting Guide

### Test Environment Setup

**Problem**: Tests fail with database connection errors

**Diagnosis**:

```bash
# Check test database exists
mysql -u root -p -e "SHOW DATABASES LIKE 'paytrust_test';"

# Check migrations applied
sqlx migrate info --database-url mysql://root:password@localhost:3306/paytrust_test
```

**Solution**:

```bash
# Create test database
mysql -u root -p -e "CREATE DATABASE paytrust_test CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;"

# Run migrations
DATABASE_URL=mysql://root:password@localhost:3306/paytrust_test sqlx migrate run
```

### Gateway Integration

**Problem**: Gateway API calls fail in tests

**Diagnosis**:

```bash
# Check .env has sandbox keys
grep -E "XENDIT_API_KEY|MIDTRANS_SERVER_KEY" .env

# Test API connectivity
curl -I https://api.xendit.co
curl -I https://api.sandbox.midtrans.com
```

**Solution**:

```bash
# Get sandbox API keys from:
# Xendit: https://dashboard.xendit.co/settings/developers
# Midtrans: https://dashboard.sandbox.midtrans.com/settings/config_info

# Update .env
XENDIT_API_KEY=xnd_development_YOUR_KEY_HERE
MIDTRANS_SERVER_KEY=SB-Mid-server-YOUR_KEY_HERE
```

### Webhook Testing

**Problem**: Webhook simulation doesn't update invoice status

**Diagnosis**:

```bash
# Check webhook endpoints exist
cargo test test_xendit_paid_webhook_updates_invoice -- --nocapture

# Look for errors in test output
```

**Solution**:

```rust
// Verify webhook payload matches expected format
let webhook = XenditSandbox::simulate_paid_webhook(
    &external_id,
    "xnd_invoice_123",
    100000,
    "IDR"
);

// Check webhook endpoint implementation
// src/modules/transactions/controllers/webhook_controller.rs
```

### Parallel Test Execution

**Problem**: Tests fail randomly when run in parallel

**Diagnosis**:

```bash
# Run parallel validation script
./scripts/test_parallel.sh 10

# Check for UUID usage in test data
grep -r "TEST-" tests/integration/
```

**Solution**:

```rust
// Replace hardcoded test IDs with UUIDs
use uuid::Uuid;

fn random_external_id() -> String {
    format!("INV_{}", Uuid::new_v4())
}

// Use transaction isolation
use tests::helpers::test_database::with_transaction;
```

### CI/CD Debugging

**Problem**: GitHub Actions tests fail but local tests pass

**Diagnosis**:

1. Check GitHub Actions logs for specific errors
2. Look for environment variable differences
3. Check database initialization steps

**Solution**:

```bash
# Simulate CI environment locally
./scripts/test_with_docker.sh

# This uses docker-compose.test.yml
# Same as GitHub Actions workflow
```

### Performance Issues

**Problem**: Tests are slow or time out

**Diagnosis**:

```bash
# Run specific test with timing
cargo test test_name -- --nocapture --show-output

# Check database connection pool settings
grep DATABASE .env
```

**Solution**:

```env
# Increase connection pool for tests
DATABASE_POOL_SIZE=20
DATABASE_MAX_CONNECTIONS=50

# Use faster test database
# Consider in-memory or tmpfs for test DB
```

**Problem**: Integration tests take too long

**Solution**:

```bash
# Run only specific test suites
cargo test --test payment_flow_test

# Use test filtering
cargo test invoice

# Parallel execution (default)
cargo test -- --test-threads=4
```

### Common Error Messages

| Error                                    | Cause                     | Solution                                                    |
| ---------------------------------------- | ------------------------- | ----------------------------------------------------------- |
| `Connection refused (os error 111)`      | MySQL not running         | `brew services start mysql` or `sudo systemctl start mysql` |
| `Database 'paytrust_test' doesn't exist` | Test DB not created       | `mysql -e "CREATE DATABASE paytrust_test"`                  |
| `Duplicate entry for key 'external_id'`  | Test data collision       | Use UUID-based test data                                    |
| `401 Unauthorized` from gateway          | Invalid API key           | Check `.env` gateway keys                                   |
| `Invalid webhook signature`              | Wrong webhook secret      | Verify `WEBHOOK_SECRET` in `.env`                           |
| `No such table: invoices`                | Migrations not run        | `sqlx migrate run`                                          |
| `Too many connections`                   | Connection pool exhausted | Increase `DATABASE_MAX_CONNECTIONS`                         |
| `Test timed out`                         | Slow query or deadlock    | Check logs, optimize queries                                |

### Getting Help

1. **Check Test Logs**: Run with `-- --nocapture` to see detailed output
2. **Review TESTING.md**: Comprehensive testing guide with examples
3. **Check OpenAPI Spec**: Verify API contract expectations
4. **Examine Helper Code**: Review `tests/helpers/mod.rs` for documentation
5. **Run Validation Scripts**: Use `./scripts/test_parallel.sh` to diagnose isolation issues

## Performance Optimization

### Database Query Optimization

```rust
// Bad: N+1 query
for invoice in invoices {
    let line_items = repo.find_line_items(invoice.id).await?;
}

// Good: Single query with JOIN
let invoices_with_items = repo.find_with_line_items(invoice_ids).await?;
```

### Connection Pool Tuning

In `.env`:

```env
DATABASE_POOL_SIZE=10        # Minimum connections
DATABASE_MAX_CONNECTIONS=20  # Maximum connections
```

Adjust based on load testing results.

## Deployment

### Production Checklist

- [ ] Set `APP_ENV=production` in `.env`
- [ ] Use production database credentials
- [ ] Configure production payment gateway keys
- [ ] Set `LOG_LEVEL=info` (not `debug`)
- [ ] Enable HTTPS/TLS
- [ ] Configure firewall rules
- [ ] Set up database backups
- [ ] Configure monitoring/alerting
- [ ] Load test rate limiting
- [ ] Verify webhook endpoints accessible
- [ ] Document API key distribution process

### Build for Production

```bash
cargo build --release
```

Binary located at: `target/release/paytrust`

### Run in Production

```bash
# With systemd service
sudo systemctl start paytrust

# Or directly
./target/release/paytrust
```

## Resources

- **OpenAPI Spec**: `specs/001-payment-orchestration-api/contracts/openapi.yaml`
- **Data Model**: `specs/001-payment-orchestration-api/data-model.md`
- **Constitution**: `.specify/memory/constitution.md`
- **Xendit API Docs**: Via Context7 MCP (`/org/xendit`)
- **Midtrans API Docs**: Via Context7 MCP (`/org/midtrans`)

## Support

For questions or issues:

1. Check specification: `specs/001-payment-orchestration-api/spec.md`
2. Review test cases for examples
3. Consult OpenAPI contract for API details
4. Use Context7 MCP for up-to-date library documentation

---

**Ready to Code!** Follow TDD workflow and module structure per Constitution principles.
