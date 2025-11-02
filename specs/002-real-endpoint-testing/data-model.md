# Phase 1 Data Model: Real Endpoint Testing Infrastructure

**Feature**: Real Endpoint Testing Infrastructure  
**Branch**: 002-real-endpoint-testing  
**Date**: November 2, 2025

## Overview

This document defines the data entities and structures for the real endpoint testing infrastructure. Unlike typical data models that define business domain entities, this focuses on test infrastructure components, test data patterns, and test environment configuration.

---

## Test Infrastructure Entities

### 1. TestServer

**Purpose**: Represents a running HTTP server instance for integration tests

**Attributes**:

- `base_url: String` - Base URL of the test server (e.g., "http://127.0.0.1:8081")
- `port: u16` - Port number the server is bound to
- `database_pool: MySqlPool` - Connection pool to test database
- `gateway_config: GatewayTestConfig` - Test gateway API credentials

**Lifecycle**:

- Created: Before each test via `actix_test::start()`
- Destroyed: Automatically after test completion
- State: Stateless across tests, fresh instance per test

**Relationships**:

- Has one TestDatabase
- Has one or more TestClients (HTTP request builders)
- Configured with GatewayTestConfig

---

### 2. TestDatabase

**Purpose**: Represents test database connection and transaction management

**Attributes**:

- `connection_url: String` - Database connection string (from TEST_DATABASE_URL env var)
- `pool: MySqlPool` - SQLx connection pool (10 connections)
- `current_transaction: Option<Transaction>` - Active transaction for test isolation

**Operations**:

- `create_test_pool() -> MySqlPool` - Create connection pool to test database
- `begin_transaction() -> Transaction` - Start new transaction for test
- `rollback_transaction()` - Rollback transaction (automatic on drop)
- `seed_test_data()` - Insert common test fixtures
- `cleanup_test_data()` - Remove test data (optional, transaction preferred)

**Configuration**:

```rust
// From .env.test
TEST_DATABASE_URL=mysql://root:password@localhost:3306/paytrust_test
```

**Constraints**:

- Must use separate database from development/production (`paytrust_test`)
- Must have same schema as production (via migrations)
- Transaction isolation level: READ COMMITTED

---

### 3. TestClient

**Purpose**: HTTP client for making requests to TestServer

**Attributes**:

- `base_url: String` - Inherited from TestServer
- `default_headers: HashMap<String, String>` - Common headers (e.g., X-API-Key)
- `timeout: Duration` - Request timeout (default: 30 seconds)

**Operations**:

- `get(path: &str) -> RequestBuilder` - Build GET request
- `post(path: &str) -> RequestBuilder` - Build POST request
- `put(path: &str) -> RequestBuilder` - Build PUT request
- `delete(path: &str) -> RequestBuilder` - Build DELETE request
- `send_json(body: &Value) -> Response` - Send request with JSON body

**Implementation**:

- Provided by actix-test's `TestServer` (srv.get(), srv.post(), etc.)
- Falls back to reqwest for advanced scenarios

---

### 4. GatewayTestConfig

**Purpose**: Configuration for payment gateway sandbox APIs

**Attributes**:

- `xendit_test_key: String` - Xendit test mode API key
- `xendit_base_url: String` - Default: "https://api.xendit.co"
- `midtrans_sandbox_key: String` - Midtrans sandbox server key
- `midtrans_base_url: String` - Default: "https://api.sandbox.midtrans.com"

**Validation**:

- Keys must be prefixed with "xnd*development*" or "xnd*public_test*" (Xendit)
- Keys must start with "SB-Mid-server-" (Midtrans)
- Fail fast if test keys not provided

**Configuration**:

```rust
// From .env.test
XENDIT_TEST_API_KEY=xnd_development_xxxxxxxxxxxxx
MIDTRANS_SANDBOX_SERVER_KEY=SB-Mid-server-xxxxxxxxxxxxx
```

---

### 5. TestDataFactory

**Purpose**: Generate test data with unique identifiers

**Attributes**:

- `id_prefix: String` - Prefix for test IDs (e.g., "TEST")
- `unique_suffix: Uuid` - UUID for uniqueness

**Operations**:

- `create_invoice() -> InvoicePayload` - Generate test invoice data
- `create_gateway() -> GatewayConfig` - Generate test gateway config
- `create_api_key() -> ApiKey` - Generate test API key
- `random_external_id() -> String` - Generate unique external_id

**Pattern**:

```rust
pub struct TestDataFactory;

impl TestDataFactory {
    pub fn random_external_id() -> String {
        format!("TEST-{}", uuid::Uuid::new_v4())
    }

    pub fn create_invoice() -> serde_json::Value {
        json!({
            "external_id": Self::random_external_id(),
            "gateway_id": "gateway-xendit-idr",
            "currency": "IDR",
            "line_items": [
                {
                    "description": "Test Product",
                    "quantity": 1,
                    "unit_price": "100000",
                    "tax_rate": "0.10"
                }
            ]
        })
    }
}
```

---

### 6. TestFixtures

**Purpose**: Pre-defined test data for common scenarios

**Types**:

**Payment Gateways**:

```rust
pub const TEST_GATEWAY_XENDIT_IDR: &str = "gateway-xendit-idr";
pub const TEST_GATEWAY_XENDIT_MYR: &str = "gateway-xendit-myr";
pub const TEST_GATEWAY_MIDTRANS_IDR: &str = "gateway-midtrans-idr";
```

**API Keys**:

```rust
pub const TEST_API_KEY: &str = "test_api_key_12345";
pub const TEST_API_KEY_HASHED: &str = "$argon2id$..."; // Hashed version
```

**Test Credit Cards** (Xendit):

```rust
pub const XENDIT_SUCCESS_CARD: &str = "4000000000000002";
pub const XENDIT_FAILURE_CARD: &str = "4000000000000010";
```

**Test Credit Cards** (Midtrans):

```rust
pub const MIDTRANS_SUCCESS_CARD: &str = "4811111111111114";
pub const MIDTRANS_3DS_CARD: &str = "4911111111111113";
```

---

### 7. TestAssertions

**Purpose**: Common assertion helpers for test responses

**Operations**:

- `assert_success(response: Response)` - Assert 2xx status code
- `assert_created(response: Response)` - Assert 201 status code
- `assert_bad_request(response: Response)` - Assert 400 status code
- `assert_unauthorized(response: Response)` - Assert 401 status code
- `assert_json_contains(response: Response, expected: Value)` - Assert JSON structure

**Pattern**:

```rust
pub fn assert_success(response: &Response) {
    assert!(
        response.status().is_success(),
        "Expected success status, got: {}",
        response.status()
    );
}

pub fn assert_json_contains(response: &Response, expected: Value) {
    let body: Value = response.json().await.unwrap();
    // Check if body contains all fields from expected
}
```

---

## Test Data Patterns

### Unique Identifier Strategy

**Pattern**: UUID-based unique identifiers

```rust
let external_id = format!("TEST-{}", uuid::Uuid::new_v4());
// Example: "TEST-550e8400-e29b-41d4-a716-446655440000"
```

**Rationale**:

- Guarantees uniqueness across parallel test runs
- No coordination needed between tests
- Easy to identify test data in database

### Transaction-Based Isolation

**Pattern**: Wrap test in database transaction

```rust
#[actix_web::test]
async fn test_example() {
    let pool = create_test_pool().await;
    let mut tx = pool.begin().await.unwrap();

    // Test code here - all DB operations use `tx`

    // Transaction automatically rolls back on drop
}
```

**Benefits**:

- Automatic cleanup (no manual DELETE queries)
- True isolation (no data leakage between tests)
- Fast (rollback faster than DELETE)

### Seed Data Pattern

**Pattern**: Insert minimal required data before test

```rust
async fn seed_test_gateway(tx: &mut Transaction<'_, MySql>) {
    sqlx::query(
        "INSERT INTO payment_gateways (id, name, provider, currency)
         VALUES (?, ?, ?, ?)"
    )
    .bind("gateway-xendit-idr")
    .bind("Xendit IDR")
    .bind("xendit")
    .bind("IDR")
    .execute(tx)
    .await
    .unwrap();
}
```

---

## Environment Configuration

### .env.test Structure

```bash
# Test Database
TEST_DATABASE_URL=mysql://root:password@localhost:3306/paytrust_test

# Test Server
TEST_SERVER_HOST=127.0.0.1
TEST_SERVER_PORT=8081

# Payment Gateway Sandbox APIs
XENDIT_TEST_API_KEY=xnd_development_xxxxxxxxxxxxx
MIDTRANS_SANDBOX_SERVER_KEY=SB-Mid-server-xxxxxxxxxxxxx

# Test API Keys (for authentication tests)
TEST_API_KEY=test_api_key_12345

# Test Timeouts
TEST_HTTP_TIMEOUT_SECS=30
TEST_DATABASE_TIMEOUT_SECS=10
```

---

## Test Module Structure

```
tests/
├── helpers/
│   ├── mod.rs                  # Public exports
│   ├── test_server.rs          # TestServer, spawn_test_server()
│   ├── test_database.rs        # TestDatabase, create_test_pool()
│   ├── test_client.rs          # TestClient helpers
│   ├── test_data.rs            # TestDataFactory, TestFixtures
│   ├── gateway_sandbox.rs      # XenditSandbox, MidtransSandbox
│   └── assertions.rs           # TestAssertions helpers
```

---

## Database Schema Requirements

**No schema changes required** - Tests use same schema as production:

- `payment_gateways` - Gateway configurations
- `api_keys` - API authentication
- `invoices` - Invoice records
- `line_items` - Invoice line items
- `installment_schedules` - Payment installments
- `payment_transactions` - Payment history

**Test Database Setup**:

1. Create `paytrust_test` database
2. Run migrations: `sqlx migrate run --database-url mysql://root:password@localhost:3306/paytrust_test`
3. Seed test fixtures (gateways, API keys)

---

## Summary

This data model defines the test infrastructure layer, not business domain entities. Key components:

1. **TestServer** - HTTP server for integration tests
2. **TestDatabase** - Database connection with transaction isolation
3. **TestClient** - HTTP client for making requests
4. **GatewayTestConfig** - Payment gateway sandbox configuration
5. **TestDataFactory** - Generate unique test data
6. **TestFixtures** - Pre-defined test data
7. **TestAssertions** - Common assertion helpers

All components follow the principle of **real testing** - no mocks, real HTTP, real database, real gateway APIs (sandbox mode).
