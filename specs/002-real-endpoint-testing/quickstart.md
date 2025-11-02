# Quick Start: Real Endpoint Testing

**Feature**: Real Endpoint Testing Infrastructure  
**Branch**: 002-real-endpoint-testing  
**Date**: November 2, 2025

## Overview

This guide helps you set up and run real endpoint tests for PayTrust. These tests use actual HTTP requests, real MySQL database, and real payment gateway sandbox APIs to validate production behavior.

---

## Prerequisites

- Rust 1.91.0+ with cargo
- MySQL 8.0+ running locally
- Payment gateway sandbox API keys (optional for basic tests)

---

## 1. Database Setup

### Create Test Database

```bash
# Connect to MySQL
mysql -u root -p

# Create test database
CREATE DATABASE paytrust_test;
EXIT;
```

### Run Migrations

```bash
# Apply schema migrations to test database
sqlx migrate run --database-url mysql://root:password@localhost:3306/paytrust_test
```

### Seed Test Data

```bash
# Run seed script to create test gateways and API keys
mysql -u root -p paytrust_test < scripts/seed_test_data.sql
```

---

## 2. Environment Configuration

### Create .env.test File

```bash
# Copy example file
cp config/.env.test.example .env.test

# Edit with your values
```

### Required Environment Variables

```bash
# .env.test
# Test Database
TEST_DATABASE_URL=mysql://root:password@localhost:3306/paytrust_test

# Test Server
TEST_SERVER_HOST=127.0.0.1
TEST_SERVER_PORT=8081

# Payment Gateway Sandbox APIs (optional for basic tests)
XENDIT_TEST_API_KEY=xnd_development_xxxxxxxxxxxxx
MIDTRANS_SANDBOX_SERVER_KEY=SB-Mid-server-xxxxxxxxxxxxx

# Test API Keys
TEST_API_KEY=test_api_key_12345
```

### Get Payment Gateway Sandbox Keys (Optional)

**Xendit**:

1. Sign up at https://dashboard.xendit.co/register
2. Go to Settings → API Keys
3. Switch to "Test Mode"
4. Copy your Test API Key (starts with `xnd_development_`)

**Midtrans**:

1. Sign up at https://dashboard.sandbox.midtrans.com/register
2. Go to Settings → Access Keys
3. Copy your Sandbox Server Key (starts with `SB-Mid-server-`)

---

## 3. Running Tests

### Run All Tests

```bash
# Load test environment and run all tests
cargo test
```

### Run Specific Test Types

```bash
# Integration tests only (with real HTTP endpoints)
cargo test --test 'integration_*'

# Contract tests only (OpenAPI validation)
cargo test --test 'contract_*'

# Unit tests only (business logic)
cargo test --lib

# Performance tests
cargo test --test 'performance_*'
```

### Run Specific Test File

```bash
# Run payment flow tests
cargo test --test payment_flow_test

# Run invoice API tests
cargo test --test invoice_api_test
```

### Run With Output

```bash
# Show println! output and logs
cargo test -- --nocapture

# Show logs with color
RUST_LOG=debug cargo test -- --nocapture
```

### Run in Parallel (Default)

```bash
# Tests run in parallel by default
cargo test

# Control parallelism
cargo test -- --test-threads=4
```

---

## 4. Verify Setup

### Test Database Connection

```bash
# Run simple connection test
cargo test --test payment_flow_test test_database_connection
```

Expected output:

```
running 1 test
test test_database_connection ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out
```

### Test HTTP Server

```bash
# Run simple endpoint test
cargo test --test invoice_api_test test_health_endpoint
```

Expected output:

```
running 1 test
test test_health_endpoint ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out
```

### Test Gateway Integration (If Keys Configured)

```bash
# Run gateway sandbox test
cargo test --test gateway_validation_test
```

---

## 5. Troubleshooting

### Database Connection Fails

**Error**: `Failed to connect to test database`

**Solution**:

1. Verify MySQL is running: `mysql -u root -p`
2. Check database exists: `SHOW DATABASES LIKE 'paytrust_test';`
3. Verify credentials in TEST_DATABASE_URL
4. Run migrations if schema is missing

### Port Already in Use

**Error**: `Address already in use (os error 48)`

**Solution**:

1. Change TEST_SERVER_PORT in .env.test
2. Or kill process using port 8081: `lsof -ti:8081 | xargs kill -9`

### Test Data Conflicts

**Error**: `Duplicate entry for key 'external_id'`

**Solution**:

- Tests should use UUID-based unique IDs
- Check if test is reusing static ID
- Verify transaction rollback is working

### Gateway API Key Invalid

**Error**: `INVALID_API_KEY`

**Solution**:

1. Verify key format: Xendit should start with `xnd_development_`
2. Check key is from test/sandbox environment, not production
3. Regenerate key in gateway dashboard if needed

### Tests Take Too Long

**Expected**: <60 seconds for full test suite

**If slower**:

1. Check database connection latency
2. Verify tests run in parallel: `cargo test -- --test-threads=8`
3. Check for `#[serial]` attributes limiting parallelism
4. Profile slow tests: `cargo test -- --nocapture --test-threads=1`

---

## 6. Development Workflow

### TDD Workflow (Per Constitution)

1. **Write Test First**:

   ```bash
   # Create new test file or add test to existing file
   # tests/integration/my_feature_test.rs
   ```

2. **Run Test (Should Fail)**:

   ```bash
   cargo test --test my_feature_test
   # Expect: test my_feature_test::test_new_feature ... FAILED
   ```

3. **Implement Feature**:

   ```bash
   # Add code to src/modules/*/
   ```

4. **Run Test Again (Should Pass)**:

   ```bash
   cargo test --test my_feature_test
   # Expect: test my_feature_test::test_new_feature ... ok
   ```

5. **Run All Tests**:
   ```bash
   cargo test
   # Ensure no regressions
   ```

### Writing New Tests

Example integration test structure:

```rust
// tests/integration/my_feature_test.rs
use actix_test;
use actix_web::{App, web};
use paytrust::modules;
use serde_json::json;

#[actix_web::test]
async fn test_my_feature() {
    // 1. Setup test data
    let external_id = format!("TEST-{}", uuid::Uuid::new_v4());

    // 2. Spawn test server
    let srv = actix_test::start(|| {
        App::new()
            .configure(modules::invoices::controllers::configure)
    });

    // 3. Make HTTP request
    let response = srv.post("/v1/invoices")
        .insert_header(("X-API-Key", "test_api_key"))
        .send_json(&json!({
            "external_id": external_id,
            // ... payload
        }))
        .await
        .unwrap();

    // 4. Assert response
    assert_eq!(response.status(), 201);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["external_id"], external_id);
}
```

---

## 7. CI/CD Integration

### GitHub Actions Workflow

PayTrust includes automated testing via GitHub Actions. Tests run automatically on every push and pull request.

**Workflow file**: `.github/workflows/test.yml`

#### Features

- ✅ Automated test execution on push/PR
- ✅ MySQL 8.0 service container with health checks
- ✅ Rust 1.91.0 toolchain with caching
- ✅ Database migrations via sqlx-cli
- ✅ Test data seeding
- ✅ Unit, integration, and contract tests
- ✅ Code formatting and linting checks
- ✅ Payment gateway API key secrets

#### Setup GitHub Secrets

To enable gateway tests in CI:

1. Go to your repository on GitHub
2. Click **Settings** → **Secrets and variables** → **Actions**
3. Add the following secrets:
   - `XENDIT_TEST_API_KEY`: Your Xendit sandbox API key
   - `MIDTRANS_SERVER_KEY`: Your Midtrans sandbox server key

#### Local CI Simulation with Docker

Test your changes locally using Docker Compose before pushing:

```bash
# Start MySQL test container and run all tests
./scripts/test_with_docker.sh all

# Run specific test types
./scripts/test_with_docker.sh unit         # Unit tests only
./scripts/test_with_docker.sh integration  # Integration tests only
./scripts/test_with_docker.sh contract     # Contract tests only
```

**Docker Compose file**: `docker-compose.test.yml`

This spins up a MySQL 8.0 container on port 3307 (to avoid conflicts with local MySQL), runs migrations, seeds test data, and executes the test suite.

#### Environment Variables in CI

The following environment variables are automatically configured in GitHub Actions:

```yaml
TEST_DATABASE_URL: mysql://root:test_password@127.0.0.1:3306/paytrust_test
DATABASE_URL: mysql://root:test_password@127.0.0.1:3306/paytrust_test
SERVER_HOST: 127.0.0.1
SERVER_PORT: 8080
RUST_LOG: info
XENDIT_TEST_API_KEY: ${{ secrets.XENDIT_TEST_API_KEY }}
MIDTRANS_SERVER_KEY: ${{ secrets.MIDTRANS_SERVER_KEY }}
TEST_CLEANUP_ENABLED: true
```

#### Workflow Stages

1. **Setup**: Checkout code, install Rust 1.91.0, setup caching
2. **Database**: Start MySQL service with health checks
3. **Migrations**: Install sqlx-cli and run migrations
4. **Seed Data**: Insert test payment gateways
5. **Test**: Run unit tests, integration tests, contract tests
6. **Quality**: Check code formatting and run clippy linter

#### Viewing Test Results

- Go to the **Actions** tab in your GitHub repository
- Click on the workflow run to see detailed logs
- Failed tests will show error messages and stack traces

#### Troubleshooting CI Failures

**MySQL connection fails**:

- Check service health check configuration
- Verify DATABASE_URL matches service credentials

**Migration fails**:

- Ensure all migration files are committed
- Check migration SQL syntax

**Tests fail locally but pass in CI (or vice versa)**:

- Environment variable differences
- Different MySQL versions
- Test data conflicts (use UUIDs for unique test data)

**Secrets not working**:

- Verify secrets are set in repository settings
- Check secret names match workflow file exactly
- Secrets are only available in GitHub Actions, not in local runs

---

## 8. Performance Expectations

Based on success criteria:

| Metric             | Target      | How to Measure                                        |
| ------------------ | ----------- | ----------------------------------------------------- |
| Full test suite    | <60 seconds | `time cargo test`                                     |
| Individual test    | <5 seconds  | `cargo test -- --nocapture`                           |
| Parallel execution | Yes         | Default behavior                                      |
| Test repeatability | 100%        | Run 10 times: `for i in {1..10}; do cargo test; done` |

---

## 9. Common Test Patterns

### Testing Authentication

```rust
#[actix_web::test]
async fn test_unauthorized_access() {
    let srv = actix_test::start(/* ... */);

    // No API key - should fail
    let response = srv.get("/v1/invoices").send().await.unwrap();
    assert_eq!(response.status(), 401);
}
```

### Testing Validation Errors

```rust
#[actix_web::test]
async fn test_invalid_payload() {
    let srv = actix_test::start(/* ... */);

    // Missing required field
    let response = srv.post("/v1/invoices")
        .insert_header(("X-API-Key", "test_key"))
        .send_json(&json!({ "invalid": "data" }))
        .await
        .unwrap();

    assert_eq!(response.status(), 400);
}
```

### Testing Database State

```rust
#[actix_web::test]
async fn test_invoice_persisted() {
    let pool = create_test_pool().await;
    let srv = actix_test::start(/* ... */);

    // Create invoice via API
    let external_id = format!("TEST-{}", uuid::Uuid::new_v4());
    srv.post("/v1/invoices")
        .send_json(&json!({ "external_id": external_id }))
        .await
        .unwrap();

    // Verify in database
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM invoices WHERE external_id = ?"
    )
    .bind(&external_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count, 1);
}
```

---

## 10. Parallel Test Execution & Data Isolation

### Running Tests in Parallel

By default, Cargo runs tests in parallel. You can control the level of parallelism:

```bash
# Use default parallelism (recommended)
cargo test

# Control number of parallel threads
cargo test -- --test-threads=4

# Run tests serially (for debugging)
cargo test -- --test-threads=1
```

### Data Isolation Best Practices

To ensure tests don't conflict when running in parallel:

#### 1. Use UUID-Based Unique Identifiers

```rust
use uuid::Uuid;

fn generate_test_id(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4())
}

#[actix_web::test]
async fn test_invoice_creation() {
    // Generate unique ID for this test run
    let external_id = generate_test_id("INV");  // "INV_550e8400-e29b..."

    // No conflicts with other parallel tests!
}
```

#### 2. Use Transaction-Based Isolation

```rust
use tests::helpers::test_database::with_transaction;

#[tokio::test]
async fn test_tax_calculation() {
    with_transaction(|mut tx| async move {
        // Insert test data - visible only to this transaction
        sqlx::query("INSERT INTO tax_rates (id, rate) VALUES (?, ?)")
            .bind("test-rate")
            .bind(0.10)
            .execute(&mut *tx)
            .await
            .unwrap();

        // Test logic here...

        // Transaction rolls back automatically - no cleanup!
    }).await;
}
```

#### 3. Seed Isolated Test Data

```rust
use tests::helpers::test_database::seed_isolated_gateway;

#[actix_web::test]
async fn test_gateway_payment() {
    // Create unique gateway for this test
    let gateway_id = generate_test_id("xendit");
    seed_isolated_gateway(&gateway_id, "Test Gateway", "xendit").await;

    // Use gateway_id in test...
}
```

### Validating Parallel Execution

Run the parallel validation script to ensure tests work correctly in parallel:

```bash
# Run tests 10 times in parallel and check for conflicts
./scripts/test_parallel.sh 10

# Use custom thread count
TEST_THREADS=8 ./scripts/test_parallel.sh 20
```

**Expected output**:

```
=== PayTrust Parallel Test Validation ===
Iterations: 10
Test Threads: 4

✓ Iteration 1 PASSED
✓ Iteration 2 PASSED
...
✓ Iteration 10 PASSED

=== Test Validation Summary ===
Total Iterations:  10
Passed:           10
Failed:           0
Pass Rate:        100.0%

Checking for common test data conflicts...
  ✓ No data conflicts detected

✓✓✓ ALL TESTS PASSED WITH 100% REPEATABILITY ✓✓✓
Tests are ready for parallel execution in CI/CD
```

### Common Isolation Issues

**Problem**: `Duplicate entry 'TEST-INV-001' for key 'external_id'`

**Solution**: Replace hardcoded IDs with UUID-based generators:

```rust
// BAD: Hardcoded ID conflicts in parallel
let external_id = "TEST-INV-001";

// GOOD: Unique ID for each test run
let external_id = generate_test_id("INV");
```

**Problem**: Tests pass individually but fail when run together

**Solution**: Use transaction isolation or unique test data per test.

**Problem**: Database deadlocks during parallel execution

**Solution**: Reduce test threads or use finer-grained transactions.

---

## 11. Next Steps

After tests pass:

1. **Add More Tests**: Cover edge cases, error scenarios
2. **Performance Tests**: Run load tests with `load_test.rs`
3. **Documentation**: Update OpenAPI spec with new endpoints
4. **Code Coverage**: `cargo tarpaulin` (optional)
5. **Production Deploy**: Merge to main after all tests pass

---

## Resources

- **Actix-Test Docs**: https://docs.rs/actix-test
- **Xendit API Docs**: https://docs.xendit.co
- **Midtrans Docs**: https://docs.midtrans.com
- **SQLx Docs**: https://docs.rs/sqlx
- **Constitution**: `.specify/memory/constitution.md`

---

## Support

If you encounter issues:

1. Check troubleshooting section above
2. Review test logs: `RUST_LOG=debug cargo test -- --nocapture`
3. Verify environment configuration in .env.test
4. Check database connectivity and migrations
5. Consult team or documentation

**Remember**: Tests should use real HTTP, real database, and real gateway sandbox APIs. No mocks allowed in integration tests (Constitution Principle III).
