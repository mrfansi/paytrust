# Test Infrastructure API Contract

**Feature**: Real Endpoint Testing Infrastructure  
**Branch**: 002-real-endpoint-testing  
**Date**: November 2, 2025

## Overview

This document defines the contracts for test helper modules and their APIs. Unlike typical API contracts that define HTTP endpoints, this specifies the Rust module interfaces for test infrastructure.

---

## Module: tests/helpers/test_server.rs

### Function: spawn_test_server

**Purpose**: Spawn a real HTTP test server with full application configuration

**Signature**:

```rust
pub async fn spawn_test_server() -> TestServerInstance
```

**Returns**: `TestServerInstance` with the following methods:

- `url() -> String` - Base URL of test server
- `port() -> u16` - Port number
- `get(path: &str) -> RequestBuilder` - Build GET request
- `post(path: &str) -> RequestBuilder` - Build POST request
- `put(path: &str) -> RequestBuilder` - Build PUT request
- `delete(path: &str) -> RequestBuilder` - Build DELETE request

**Behavior**:

- Starts actix-web server on random available port
- Configures all application modules (invoices, installments, gateways, etc.)
- Connects to test database via TEST_DATABASE_URL
- Loads test gateway configuration
- Server stops automatically when TestServerInstance drops

**Example**:

```rust
#[actix_web::test]
async fn test_example() {
    let srv = spawn_test_server().await;
    let response = srv.get("/health").send().await.unwrap();
    assert_eq!(response.status(), 200);
}
```

---

## Module: tests/helpers/test_database.rs

### Function: create_test_pool

**Purpose**: Create MySQL connection pool to test database

**Signature**:

```rust
pub async fn create_test_pool() -> MySqlPool
```

**Returns**: `MySqlPool` configured for test database

**Behavior**:

- Reads TEST_DATABASE_URL from environment
- Falls back to default: `mysql://root:password@localhost:3306/paytrust_test`
- Creates pool with 10 connections
- Panics with clear message if connection fails

**Example**:

```rust
#[tokio::test]
async fn test_database() {
    let pool = create_test_pool().await;
    let result: i64 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(result, 1);
}
```

### Function: with_transaction

**Purpose**: Execute test within database transaction that auto-rolls back

**Signature**:

```rust
pub async fn with_transaction<F, Fut, T>(f: F) -> T
where
    F: FnOnce(Transaction<'_, MySql>) -> Fut,
    Fut: Future<Output = T>,
```

**Parameters**:

- `f` - Async function that receives transaction

**Returns**: Result of function `f`

**Behavior**:

- Creates new transaction from test pool
- Executes function `f` with transaction
- Automatically rolls back transaction on completion (even on panic)
- Ensures test isolation

**Example**:

```rust
#[tokio::test]
async fn test_with_transaction() {
    with_transaction(|mut tx| async move {
        sqlx::query("INSERT INTO invoices (...) VALUES (...)")
            .execute(&mut tx)
            .await
            .unwrap();

        // Verify insertion
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM invoices")
            .fetch_one(&mut tx)
            .await
            .unwrap();
        assert_eq!(count, 1);

        // Transaction rolls back automatically
    }).await;
}
```

---

## Module: tests/helpers/test_data.rs

### Struct: TestDataFactory

**Purpose**: Generate test data with unique identifiers

### Method: random_external_id

**Signature**:

```rust
pub fn random_external_id() -> String
```

**Returns**: Unique external ID in format "TEST-{uuid}"

**Example**: `"TEST-550e8400-e29b-41d4-a716-446655440000"`

### Method: create_invoice_payload

**Signature**:

```rust
pub fn create_invoice_payload() -> serde_json::Value
```

**Returns**: Valid invoice JSON payload with:

- Random external_id
- Default gateway (gateway-xendit-idr)
- Single line item with reasonable values
- IDR currency

**Example**:

```rust
let payload = TestDataFactory::create_invoice_payload();
// {
//   "external_id": "TEST-550e8400-...",
//   "gateway_id": "gateway-xendit-idr",
//   "currency": "IDR",
//   "line_items": [...]
// }
```

### Method: create_gateway_config

**Signature**:

```rust
pub fn create_gateway_config(currency: &str) -> serde_json::Value
```

**Parameters**:

- `currency` - Currency code (IDR, MYR, etc.)

**Returns**: Gateway configuration JSON

---

## Module: tests/helpers/gateway_sandbox.rs

### Struct: XenditSandbox

**Purpose**: Interface to Xendit sandbox API

### Method: new

**Signature**:

```rust
pub fn new() -> Self
```

**Returns**: `XenditSandbox` instance configured with test API key

**Behavior**:

- Reads XENDIT_TEST_API_KEY from environment
- Panics with clear message if key not set
- Validates key starts with "xnd*development*" or "xnd*public_test*"

### Method: create_invoice

**Signature**:

```rust
pub async fn create_invoice(&self, params: InvoiceParams) -> Result<Invoice>
```

**Parameters**:

- `params` - Invoice creation parameters

**Returns**: Created invoice or error

**Behavior**:

- Makes real HTTP POST to https://api.xendit.co/v2/invoices
- Uses Basic Auth with test API key
- Returns actual API response

**Example**:

```rust
#[tokio::test]
async fn test_xendit_sandbox() {
    let xendit = XenditSandbox::new();
    let invoice = xendit.create_invoice(InvoiceParams {
        external_id: "TEST-123",
        amount: 100000,
        // ...
    }).await.unwrap();

    assert_eq!(invoice.status, "PENDING");
}
```

### Struct: MidtransSandbox

**Purpose**: Interface to Midtrans sandbox API

Similar structure to XenditSandbox with methods:

- `new() -> Self`
- `charge(params: ChargeParams) -> Result<Transaction>`

---

## Module: tests/helpers/assertions.rs

### Function: assert_success

**Signature**:

```rust
pub fn assert_success(response: &ClientResponse)
```

**Behavior**:

- Asserts status code is 2xx
- Panics with clear message showing actual status code

### Function: assert_created

**Signature**:

```rust
pub fn assert_created(response: &ClientResponse)
```

**Behavior**: Asserts status code is 201

### Function: assert_bad_request

**Signature**:

```rust
pub fn assert_bad_request(response: &ClientResponse)
```

**Behavior**: Asserts status code is 400

### Function: assert_json_contains

**Signature**:

```rust
pub async fn assert_json_contains(
    response: &mut ClientResponse,
    expected: serde_json::Value
)
```

**Behavior**:

- Parses response body as JSON
- Asserts all fields in `expected` exist in response with matching values
- Allows extra fields in response

**Example**:

```rust
let mut response = srv.get("/v1/invoices/123").send().await.unwrap();
assert_json_contains(&mut response, json!({
    "id": "123",
    "status": "pending"
})).await;
// Response can have additional fields like "created_at", "total", etc.
```

---

## Test Configuration Contract

### Environment Variables (Required)

```bash
# Database
TEST_DATABASE_URL=mysql://root:password@localhost:3306/paytrust_test

# API Keys
TEST_API_KEY=test_api_key_12345

# Optional: Gateway Sandbox APIs
XENDIT_TEST_API_KEY=xnd_development_xxxxxxxxxxxxx
MIDTRANS_SANDBOX_SERVER_KEY=SB-Mid-server-xxxxxxxxxxxxx
```

### Failure Modes

All test helpers must:

1. **Panic with clear message** if configuration is missing
2. **Include troubleshooting hints** in panic message
3. **Validate configuration format** (e.g., key prefixes)

**Example panic messages**:

```
"TEST_DATABASE_URL not set. Run: export TEST_DATABASE_URL=mysql://root:password@localhost:3306/paytrust_test"

"XENDIT_TEST_API_KEY must start with 'xnd_development_' or 'xnd_public_test_'.
 Got: xnd_production_xxxxx (this looks like a LIVE key!)"
```

---

## Integration Test Structure Contract

All integration tests must follow this pattern:

```rust
// tests/integration/feature_test.rs

use actix_test;
use actix_web::{App, web};
use paytrust::{modules, config};
use serde_json::json;
use tests::helpers::*;  // Import test helpers

#[actix_web::test]
async fn test_feature_name() {
    // 1. Setup: Generate unique test data
    let external_id = TestDataFactory::random_external_id();

    // 2. Spawn: Start real HTTP server
    let srv = spawn_test_server().await;

    // 3. Act: Make real HTTP request
    let response = srv.post("/v1/endpoint")
        .insert_header(("X-API-Key", TEST_API_KEY))
        .send_json(&json!({
            "external_id": external_id,
            // ... payload
        }))
        .await
        .unwrap();

    // 4. Assert: Verify response
    assert_success(&response);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["external_id"], external_id);

    // 5. Verify: Check database state (optional)
    let pool = create_test_pool().await;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM invoices WHERE external_id = ?)"
    )
    .bind(&external_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(exists);

    // Cleanup happens automatically (server drops, transaction rolls back if used)
}
```

---

## Contract Test Validation

Integration tests must validate:

1. **HTTP Methods**: GET, POST, PUT, DELETE work correctly
2. **Headers**: Authentication, Content-Type, Custom headers
3. **Status Codes**: 200, 201, 400, 401, 404, 500
4. **Request Body**: JSON serialization/deserialization
5. **Response Body**: JSON structure matches OpenAPI spec
6. **Database State**: Data persists correctly
7. **Gateway Integration**: Real API calls work

---

## Summary

This contract defines:

- **Test Server API**: spawn_test_server() for real HTTP testing
- **Database API**: create_test_pool(), with_transaction() for isolation
- **Test Data API**: TestDataFactory for unique identifiers
- **Gateway API**: XenditSandbox, MidtransSandbox for real API calls
- **Assertion API**: Helper functions for common test assertions
- **Configuration Contract**: Required environment variables
- **Test Structure**: Standard pattern for all integration tests

All contracts must be implemented in `tests/helpers/` module and follow Constitution Principle III (real testing, no mocks).
