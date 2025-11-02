# Phase 0 Research: Real Endpoint Testing Infrastructure

**Feature**: Real Endpoint Testing Infrastructure  
**Branch**: 002-real-endpoint-testing  
**Date**: November 2, 2025

## Overview

This document consolidates research findings for implementing real HTTP endpoint testing in PayTrust, replacing direct database manipulation and mocked gateway calls with actual HTTP requests and real payment gateway sandbox APIs.

---

## Research Task 1: Test Server Infrastructure (actix-test)

### Decision: Use actix-test for Test Server

**Rationale**:

- actix-test is the official testing crate for actix-web, maintained by the Actix team
- Provides `actix_test::start()` function to spawn a real HTTP server for tests
- Returns `TestServer` instance with built-in HTTP client methods (`.get()`, `.post()`, `.put()`, `.delete()`, etc.)
- Handles server lifecycle automatically (starts before test, stops after)
- Supports custom server configuration and middleware for realistic test scenarios

**Alternatives Considered**:

1. **actix-web::test module** - Only provides mock request builders (`TestRequest`), doesn't spawn real HTTP server
2. **Manual server spawning with tokio::spawn** - Complex, requires manual port management and cleanup
3. **reqwest with manual server** - Requires starting server in background thread, error-prone

**Implementation Pattern**:

```rust
use actix_test;
use actix_web::{App, web};

#[actix_web::test]
async fn test_invoice_creation() {
    // Spawn real HTTP server
    let srv = actix_test::start(|| {
        App::new()
            .configure(modules::invoices::controllers::configure)
            .configure(modules::gateways::controllers::configure)
    });

    // Make real HTTP request
    let req = srv.post("/v1/invoices")
        .insert_header(("X-API-Key", "test_key"))
        .send_json(&invoice_payload);

    let response = req.await.unwrap();
    assert_eq!(response.status(), 201);
}
```

**Dependencies to Add**:

- `actix-test = "0.1"` in `[dev-dependencies]`

---

## Research Task 2: HTTP Client for Tests

### Decision: Use actix-test's Built-in Client + reqwest for Advanced Cases

**Rationale**:

- actix-test's `TestServer` provides convenient built-in client with methods like `.get()`, `.post()`, `.send_json()`
- Automatically handles base URL and connection management
- For advanced scenarios (file uploads, streaming), fall back to reqwest (already in dependencies)
- Consistent with existing codebase (reqwest used for gateway integration)

**Alternatives Considered**:

1. **Pure reqwest** - Works but requires manual URL construction and server address management
2. **awc (actix-web-client)** - More complex API, less ergonomic for testing

**Implementation Pattern**:

```rust
// Simple case: Use TestServer client
let response = srv.get("/v1/invoices")
    .insert_header(("X-API-Key", "test_key"))
    .send()
    .await
    .unwrap();

// Advanced case: Use reqwest for specific needs
let client = reqwest::Client::new();
let response = client
    .get(format!("{}/v1/invoices", srv.url()))
    .header("X-API-Key", "test_key")
    .send()
    .await
    .unwrap();
```

---

## Research Task 3: Test Database Management

### Decision: Separate Test Database with Transaction-Based Cleanup

**Rationale**:

- Use dedicated `paytrust_test` database (already exists in current tests)
- Wrap each test in database transaction that rolls back on completion
- Ensures test isolation without manual cleanup code
- Faster than truncating tables or recreating database per test

**Alternatives Considered**:

1. **Separate schema per test** - Complex, slower, requires dynamic schema creation
2. **Manual cleanup queries** - Error-prone, incomplete cleanup if test panics
3. **Recreate database per test** - Too slow for large test suites

**Implementation Pattern**:

```rust
// In tests/helpers/test_database.rs
pub async fn create_test_pool() -> MySqlPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:password@localhost:3306/paytrust_test".to_string());

    MySqlPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

// Transaction wrapper for test isolation
pub async fn with_transaction<F, Fut>(f: F)
where
    F: FnOnce(sqlx::Transaction<'_, MySql>) -> Fut,
    Fut: Future<Output = ()>,
{
    let pool = create_test_pool().await;
    let mut tx = pool.begin().await.unwrap();

    f(tx).await;

    // Transaction automatically rolls back on drop
}
```

**Environment Configuration**:

- Add `TEST_DATABASE_URL` to `.env.test`
- Use dotenvy to load test-specific environment variables

---

## Research Task 4: Payment Gateway Sandbox APIs

### Decision: Use Real Gateway Sandbox APIs with Test Credentials

**Rationale**:

- Xendit provides test mode API keys (prefix: `xnd_public_test_`, `xnd_development_`)
- Midtrans provides sandbox environment (api.sandbox.midtrans.com)
- Sandbox APIs have same contract as production but don't process real money
- Catches real integration issues (API changes, network errors, authentication)
- Aligns with Constitution Principle III (real testing, no mocks)

**Alternatives Considered**:

1. **mockito/httpmock** - REJECTED: Violates Constitution, doesn't catch real API issues
2. **Local mock server with wiremock** - Better than mockito but still misses API changes
3. **Recording/replay (VCR pattern)** - Outdated recordings lead to false confidence

**Xendit Test Mode**:

- Endpoint: `https://api.xendit.co` (same as production)
- Authentication: Basic Auth with secret key (Base64 encoded `{test_key}:`)
- Test keys available in dashboard: Settings → API Keys → Test Mode
- Error codes same as production but no actual money movement

**Midtrans Sandbox**:

- Endpoint: `https://api.sandbox.midtrans.com`
- Authentication: Server Key in `Authorization` header
- Sandbox keys in dashboard: Settings → Access Keys → Sandbox
- Test credit cards available for testing payment flows

**Implementation Pattern**:

```rust
// In tests/helpers/gateway_sandbox.rs
pub struct XenditSandbox {
    api_key: String,
    base_url: String,
}

impl XenditSandbox {
    pub fn new() -> Self {
        Self {
            api_key: std::env::var("XENDIT_TEST_API_KEY")
                .expect("XENDIT_TEST_API_KEY not set"),
            base_url: "https://api.xendit.co".to_string(),
        }
    }

    pub async fn create_invoice(&self, params: InvoiceParams) -> Result<Invoice> {
        let client = reqwest::Client::new();
        let auth = format!("{}:", self.api_key);
        let encoded = base64::encode(auth);

        client
            .post(format!("{}/v2/invoices", self.base_url))
            .header("Authorization", format!("Basic {}", encoded))
            .json(&params)
            .send()
            .await?
            .json()
            .await
    }
}
```

**Environment Configuration**:

```bash
# .env.test
XENDIT_TEST_API_KEY=xnd_development_xxxxxxxxxxxxx
MIDTRANS_SANDBOX_SERVER_KEY=SB-Mid-server-xxxxxxxxxxxxx
```

**Test Data**:

- Use Xendit test credit cards: `4000000000000002` (success), `4000000000000010` (failure)
- Use Midtrans sandbox cards: `4811111111111114` (success), `4911111111111113` (3DS)

---

## Research Task 5: Test Data Isolation Strategies

### Decision: Unique IDs + Database Transactions

**Rationale**:

- Each test uses UUID for external_id to prevent conflicts
- Database transactions ensure automatic cleanup on test completion
- Enables parallel test execution without data races
- Simpler than separate schemas or manual cleanup

**Alternatives Considered**:

1. **Sequential IDs with locks** - Slower, prevents parallelism
2. **Separate database per test** - Too slow, resource-intensive
3. **Manual cleanup in teardown** - Error-prone if test panics

**Implementation Pattern**:

```rust
#[actix_web::test]
async fn test_invoice_creation() {
    let pool = create_test_pool().await;
    let mut tx = pool.begin().await.unwrap();

    // Use UUID for unique test data
    let external_id = format!("TEST-{}", uuid::Uuid::new_v4());

    // Make HTTP request that creates data in database
    let srv = actix_test::start(/* ... */);
    let response = srv.post("/v1/invoices")
        .send_json(&json!({
            "external_id": external_id,
            // ... other fields
        }))
        .await
        .unwrap();

    // Assertions...

    // Transaction rolls back automatically on drop
}
```

---

## Research Task 6: CI/CD Integration

### Decision: Docker Compose for Test Database in CI/CD

**Rationale**:

- Consistent test environment across local and CI/CD
- Docker Compose can spin up MySQL test database before tests
- GitHub Actions, GitLab CI, etc. all support Docker Compose
- No need for CI-specific database configuration

**Implementation Pattern**:

```yaml
# docker-compose.test.yml
version: "3.8"
services:
  mysql-test:
    image: mysql:8.0
    environment:
      MYSQL_ROOT_PASSWORD: password
      MYSQL_DATABASE: paytrust_test
    ports:
      - "3307:3306" # Different port to avoid conflicts
    healthcheck:
      test: ["CMD", "mysqladmin", "ping", "-h", "localhost"]
      interval: 5s
      timeout: 5s
      retries: 5

# GitHub Actions workflow
jobs:
  test:
    runs-on: ubuntu-latest
    services:
      mysql:
        image: mysql:8.0
        env:
          MYSQL_ROOT_PASSWORD: password
          MYSQL_DATABASE: paytrust_test
        options: >-
          --health-cmd="mysqladmin ping"
          --health-interval=10s
          --health-timeout=5s
          --health-retries=3
    steps:
      - uses: actions/checkout@v3
      - name: Run tests
        env:
          TEST_DATABASE_URL: mysql://root:password@localhost:3306/paytrust_test
          XENDIT_TEST_API_KEY: ${{ secrets.XENDIT_TEST_API_KEY }}
        run: cargo test
```

---

## Research Task 7: Test Performance Optimization

### Decision: Parallel Test Execution with test-threads

**Rationale**:

- Cargo test supports parallel execution by default
- Database transactions ensure isolation without blocking
- Use `#[serial]` attribute only for tests that truly conflict
- Target: Complete test suite in <60 seconds (per success criteria)

**Implementation Pattern**:

```rust
// Most tests can run in parallel (default)
#[actix_web::test]
async fn test_invoice_creation() { /* ... */ }

// Use #[serial] only when necessary (e.g., testing rate limits)
use serial_test::serial;

#[actix_web::test]
#[serial]
async fn test_rate_limiting() { /* ... */ }
```

**Dependencies to Add**:

- `serial_test = "3.0"` in `[dev-dependencies]` (optional, only if needed)

---

## Summary of Key Decisions

| Aspect          | Decision                             | Rationale                                      |
| --------------- | ------------------------------------ | ---------------------------------------------- |
| Test Server     | actix-test                           | Official, automatic lifecycle, built-in client |
| HTTP Client     | TestServer client + reqwest fallback | Convenient API, consistent with codebase       |
| Test Database   | Separate DB + transactions           | Isolation, speed, automatic cleanup            |
| Gateway Testing | Real sandbox APIs                    | Constitution compliant, catches real issues    |
| Test Isolation  | UUID + transactions                  | Enables parallelism, prevents conflicts        |
| CI/CD           | Docker Compose + GitHub Actions      | Consistent environment, easy setup             |
| Performance     | Parallel by default                  | Meets <60s target, good developer experience   |

---

## Dependencies to Modify

### Remove from Cargo.toml:

```toml
[dev-dependencies]
mockito = "1.5"  # REMOVE - violates Constitution
```

### Add to Cargo.toml:

```toml
[dev-dependencies]
actix-test = "0.1"  # For test server
# reqwest already in main dependencies, use for advanced HTTP client needs
# serial_test = "3.0"  # Optional, only if needed for sequential tests
```

---

## Next Steps (Phase 1)

1. Create data-model.md documenting test infrastructure entities
2. Create test helper modules in `tests/helpers/`
3. Generate quickstart.md for running tests locally
4. Update .env.example with test configuration section
5. Create .env.test.example with test-specific variables
6. Refactor one integration test as proof-of-concept
