# PayTrust Testing Guide

**Last Updated**: November 2, 2025  
**Feature**: Real Endpoint Testing Infrastructure

This guide explains how to write tests for PayTrust using real HTTP endpoints, real database connections, and real payment gateway sandbox APIs.

---

## Table of Contents

1. [Testing Philosophy](#testing-philosophy)
2. [Test Infrastructure](#test-infrastructure)
3. [Writing Integration Tests](#writing-integration-tests)
4. [Writing Contract Tests](#writing-contract-tests)
5. [Test Data Management](#test-data-management)
6. [Payment Gateway Testing](#payment-gateway-testing)
7. [Webhook Testing](#webhook-testing)
8. [Performance Testing](#performance-testing)
9. [Best Practices](#best-practices)
10. [Troubleshooting](#troubleshooting)

---

## Testing Philosophy

PayTrust follows Constitution Principle III: **No mocks in integration tests**.

### What We Test

✅ **Real HTTP Endpoints**: Tests make actual HTTP requests to test servers  
✅ **Real Database**: Tests use MySQL test database with real connections  
✅ **Real Gateway APIs**: Tests call Xendit/Midtrans sandbox APIs  
✅ **Real JSON**: Tests validate actual request/response payloads

❌ **No Mocks**: No mockito, no mock databases, no mock HTTP clients  
❌ **No Stubs**: No fake implementations in integration tests

### Why This Approach?

- **Confidence**: Tests validate production behavior
- **Integration**: Tests catch integration issues early
- **Simplicity**: No complex mock setup or maintenance
- **Reality**: Tests fail when actual APIs change

---

## Test Infrastructure

### Available Test Helpers

Located in `tests/helpers/`:

- **test_server.rs**: Spawn HTTP test servers
- **test_client.rs**: HTTP client for making requests
- **test_database.rs**: Database connections and transactions
- **test_data.rs**: Test data generation and fixtures
- **assertions.rs**: HTTP response assertions
- **gateway_sandbox.rs**: Real gateway API clients

### Quick Setup

```bash
# 1. Setup test database
./scripts/setup_test_db.sh

# 2. Configure environment
cp config/.env.test.example .env.test
# Edit .env.test with your credentials

# 3. Run tests
cargo test
```

---

## Writing Integration Tests

### Basic Structure

```rust
use tests::helpers::*;
use serde_json::json;

#[actix_web::test]
async fn test_create_invoice() {
    // 1. Spawn test server
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());

    // 2. Generate unique test data
    let external_id = TestDataFactory::random_external_id();

    // 3. Prepare request payload
    let payload = json!({
        "external_id": external_id,
        "gateway_id": TestFixtures::XENDIT_TEST_GATEWAY_ID,
        "currency": "IDR",
        "line_items": [{
            "name": "Test Product",
            "quantity": 1,
            "unit_price": 100000,
            "currency": "IDR"
        }]
    });

    // 4. Make HTTP request
    let mut response = client
        .post_json("/api/invoices", &payload)
        .await
        .expect("Failed to create invoice");

    // 5. Assert response
    assert_created(&response);

    // 6. Verify response data
    let invoice: serde_json::Value = response.json().await.expect("Failed to parse");
    assert_json_field_eq(&invoice, "status", &json!("pending"));
    assert_eq!(invoice["external_id"], external_id);
}
```

### Step-by-Step Breakdown

#### Step 1: Spawn Test Server

```rust
let srv = spawn_test_server().await;
let client = TestClient::new(srv.url("").to_string());
```

- Creates a real HTTP server on a random available port
- Server includes all routes and middleware
- Automatically shuts down when test completes

#### Step 2: Generate Unique Test Data

```rust
let external_id = TestDataFactory::random_external_id();  // "TEST-uuid"
```

- Use UUIDs to avoid conflicts in parallel execution
- Never use hardcoded IDs like `"TEST-001"`

#### Step 3: Prepare Request

```rust
let payload = TestDataFactory::create_invoice_payload_with(
    TestFixtures::XENDIT_TEST_GATEWAY_ID,
    TestFixtures::CURRENCY_IDR,
    TestFixtures::DEFAULT_AMOUNT_IDR,
);
```

- Use TestDataFactory for consistent test data
- Use TestFixtures for known constants

#### Step 4: Make HTTP Request

```rust
let mut response = client.post_json("/api/invoices", &payload).await?;
```

- Uses real HTTP client (awc)
- Returns actual HTTP response
- Note: response must be `mut` for `.json()` consumption

#### Step 5: Assert Response

```rust
assert_created(&response);  // Checks status code 201
assert_ok(&response);       // Checks status code 200
assert_bad_request(&response);  // Checks status code 400
```

#### Step 6: Verify Data

```rust
let invoice: Value = response.json().await.expect("Failed to parse");
assert_json_field_eq(&invoice, "external_id", &json!(external_id));
```

---

## Writing Contract Tests

Contract tests validate API compliance with OpenAPI specification.

### Example

```rust
use tests::helpers::*;

#[actix_web::test]
async fn test_create_invoice_returns_correct_schema() {
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());

    let payload = TestDataFactory::create_invoice_payload();
    let mut response = client.post_json("/api/invoices", &payload).await?;

    // Validate status code
    assert_created(&response);

    // Parse response
    let invoice: Value = response.json().await?;

    // Validate required fields
    assert!(invoice["id"].is_string(), "id must be string");
    assert!(invoice["external_id"].is_string(), "external_id must be string");
    assert!(invoice["status"].is_string(), "status must be string");
    assert!(invoice["amount"].is_number(), "amount must be number");
    assert!(invoice["currency"].is_string(), "currency must be string");

    // Validate field formats
    assert!(invoice["created_at"].as_str().unwrap().ends_with("Z"),
            "created_at must be ISO 8601");
}
```

---

## Test Data Management

### Use UUIDs for Unique IDs

**Bad** (conflicts in parallel execution):

```rust
let invoice_id = "TEST-INV-001";  // ❌ Hardcoded
```

**Good** (unique per test run):

```rust
use uuid::Uuid;

fn generate_test_id(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4())
}

let invoice_id = generate_test_id("INV");  // ✅ "INV_550e8400-..."
```

### Use Transaction Isolation

For tests that need database state:

```rust
use tests::helpers::test_database::with_transaction;

#[tokio::test]
async fn test_with_isolation() {
    with_transaction(|mut tx| async move {
        // Insert test data - visible only to this transaction
        sqlx::query("INSERT INTO tax_rates (id, rate) VALUES (?, ?)")
            .bind("test-rate")
            .bind(0.10)
            .execute(&mut *tx)
            .await
            .unwrap();

        // Test logic...

        // Transaction rolls back automatically - no cleanup!
    }).await;
}
```

### Use Isolated Gateway Seeding

```rust
use tests::helpers::test_database::seed_isolated_gateway;

#[actix_web::test]
async fn test_with_gateway() {
    let gateway_id = generate_test_id("xendit");
    seed_isolated_gateway(&gateway_id, "Test Gateway", "xendit").await;

    // Use gateway_id in test...
}
```

---

## Payment Gateway Testing

### Xendit Integration

```rust
use tests::helpers::XenditSandbox;

#[actix_web::test]
async fn test_xendit_payment() {
    let xendit = XenditSandbox::new();

    // Create invoice via Xendit API
    let external_id = generate_test_id("INV");
    let result = xendit.create_invoice(
        &external_id,
        100000,
        "IDR"
    ).await;

    assert!(result.is_ok(), "Xendit invoice creation failed");

    let invoice_data = result.unwrap();
    let xendit_invoice_id = invoice_data["id"].as_str().unwrap();

    // Verify invoice via Xendit API
    let fetched = xendit.get_invoice(xendit_invoice_id).await.unwrap();
    assert_eq!(fetched["status"], "PENDING");
}
```

### Midtrans Integration

```rust
use tests::helpers::MidtransSandbox;

#[actix_web::test]
async fn test_midtrans_charge() {
    let midtrans = MidtransSandbox::new();

    let order_id = generate_test_id("ORDER");
    let result = midtrans.charge(&order_id, 100000).await;

    assert!(result.is_ok(), "Midtrans charge failed");
}
```

---

## Webhook Testing

### Simulate Xendit Webhooks

```rust
use tests::helpers::XenditSandbox;

#[actix_web::test]
async fn test_xendit_paid_webhook() {
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());

    // Create invoice first
    let external_id = generate_test_id("INV");
    // ... create invoice via API ...

    // Simulate Xendit webhook
    let webhook_payload = XenditSandbox::simulate_paid_webhook(
        &external_id,
        "xnd_invoice_123",
        100000,
        "IDR"
    );

    // Send webhook to your endpoint
    let mut response = client
        .post_json("/api/webhooks/xendit", &webhook_payload)
        .await
        .expect("Webhook failed");

    assert_ok(&response);

    // Verify invoice status updated
    let mut get_response = client
        .get_request(&format!("/api/invoices/{}", external_id))
        .await
        .expect("Failed to get invoice");

    let invoice: Value = get_response.json().await.unwrap();
    assert_json_field_eq(&invoice, "status", &json!("paid"));
}
```

### Simulate Midtrans Webhooks

```rust
use tests::helpers::MidtransSandbox;

#[actix_web::test]
async fn test_midtrans_settlement_webhook() {
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());

    let order_id = generate_test_id("ORDER");

    // Simulate Midtrans webhook
    let webhook_payload = MidtransSandbox::simulate_payment_webhook(
        &order_id,
        "100000"
    );

    let mut response = client
        .post_json("/api/webhooks/midtrans", &webhook_payload)
        .await
        .expect("Webhook failed");

    assert_ok(&response);
}
```

---

## Performance Testing

### Load Testing Structure

```rust
use tests::helpers::*;
use std::time::Instant;

#[actix_web::test]
async fn test_invoice_creation_performance() {
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());

    let iterations = 100;
    let start = Instant::now();

    for i in 0..iterations {
        let external_id = generate_test_id("PERF");
        let payload = TestDataFactory::create_invoice_payload();

        let mut response = client
            .post_json("/api/invoices", &payload)
            .await
            .expect("Request failed");

        assert_created(&response);
    }

    let duration = start.elapsed();
    let avg_ms = duration.as_millis() / iterations;

    println!("Average invoice creation time: {}ms", avg_ms);
    assert!(avg_ms < 100, "Invoice creation too slow: {}ms", avg_ms);
}
```

---

## Best Practices

### 1. Use Unique Test Data

✅ **Good**:

```rust
let id = generate_test_id("INV");
```

❌ **Bad**:

```rust
let id = "TEST-001";
```

### 2. Clean Up with Transactions

✅ **Good**:

```rust
with_transaction(|tx| async move {
    // Test code - auto-rollback
}).await;
```

❌ **Bad**:

```rust
// Insert data
// ... test ...
// DELETE FROM table WHERE id = ?  // Manual cleanup
```

### 3. Use Assertion Helpers

✅ **Good**:

```rust
assert_created(&response);
assert_json_field_eq(&json, "status", &json!("paid"));
```

❌ **Bad**:

```rust
assert_eq!(response.status(), 201);
assert_eq!(json["status"].as_str().unwrap(), "paid");
```

### 4. Test Real Scenarios

✅ **Good**:

```rust
// Create invoice via HTTP → Verify via HTTP → Simulate webhook → Verify status change
```

❌ **Bad**:

```rust
// Insert into database → Query database → Assert
```

### 5. Handle Errors Properly

✅ **Good**:

```rust
let mut response = client.post_json("/api/invoices", &payload)
    .await
    .expect("Failed to create invoice");
```

❌ **Bad**:

```rust
let mut response = client.post_json("/api/invoices", &payload).await.unwrap();
```

---

## Troubleshooting

### Test Fails: "Connection refused"

**Cause**: Test server didn't start or port conflict

**Solution**:

```rust
// Verify server spawned successfully
let srv = spawn_test_server().await;
println!("Test server running at: {}", srv.url(""));
```

### Test Fails: "Duplicate entry"

**Cause**: Hardcoded test IDs causing conflicts

**Solution**: Use UUID-based IDs:

```rust
let id = generate_test_id("PREFIX");
```

### Test Fails: "Database connection failed"

**Cause**: Test database not set up

**Solution**:

```bash
./scripts/setup_test_db.sh
```

### Tests Pass Individually, Fail Together

**Cause**: Shared test data or missing isolation

**Solution**: Use transactions or unique IDs:

```rust
with_transaction(|tx| async move {
    // Test code
}).await;
```

### Gateway API Calls Fail

**Cause**: Missing or invalid API keys

**Solution**:

```bash
# Set in .env.test
XENDIT_TEST_API_KEY=xnd_development_xxx
MIDTRANS_SERVER_KEY=SB-Mid-server-xxx
```

---

## Running Tests

```bash
# All tests
cargo test

# Integration tests only
cargo test --test '*'

# Specific test file
cargo test --test payment_flow_test

# Specific test function
cargo test test_create_invoice

# With output
cargo test -- --nocapture

# Parallel execution
cargo test -- --test-threads=4

# Validation script (10 iterations)
./scripts/test_parallel.sh 10
```

---

## Resources

- **Quickstart**: `specs/002-real-endpoint-testing/quickstart.md`
- **Test Helpers**: `tests/helpers/mod.rs`
- **OpenAPI Spec**: `specs/001-payment-orchestration-api/contracts/openapi.yaml`
- **Constitution**: `.specify/memory/constitution.md`

---

## Need Help?

1. Check this guide
2. Review existing tests in `tests/integration/`
3. Check test helper documentation in `tests/helpers/mod.rs`
4. Review quickstart troubleshooting section
5. Consult the team

**Remember**: Tests should use real HTTP, real database, and real gateway APIs. No mocks!
