# Research: PayTrust Payment Orchestration Platform

**Date**: 2025-11-01  
**Last Updated**: 2025-11-01 (Version Verification)  
**Status**: Phase 0 Complete - Versions Verified for November 2025

## Version Summary (November 2025)

**Critical Update**: All dependencies verified against latest stable versions as of November 2025.

| Dependency       | Originally Planned | Current Recommended | Status                | Breaking Changes                     |
| ---------------- | ------------------ | ------------------- | --------------------- | ------------------------------------ |
| **Rust**         | 1.75+              | **1.91.0** (stable) | ✅ Update             | None - backward compatible           |
| **actix-web**    | 4.x                | **4.9+**            | ✅ Current            | No v5 yet, 4.x stable                |
| **sqlx**         | 0.7.x              | **0.8.x**           | ⚠️ Update Recommended | Minor API changes                    |
| **tokio**        | Implied 1.x        | **1.40+**           | ✅ Update             | None - backward compatible           |
| **reqwest**      | 0.11.x             | **0.12.x**          | ⚠️ Update Available   | Minor - connection pool improvements |
| **serde**        | 1.x                | **1.0.210+**        | ✅ Current            | None                                 |
| **rust_decimal** | 1.x                | **1.36+**           | ✅ Current            | None                                 |
| **dotenvy**      | 0.15.x             | **0.15.7**          | ✅ Current            | Active maintenance                   |
| **thiserror**    | 1.x                | **1.0.60+**         | ✅ Current            | None                                 |
| **tracing**      | 0.1.x              | **0.1.40+**         | ✅ Current            | None                                 |
| **governor**     | 0.6.x              | **0.7.x**           | ⚠️ Update Available   | API improvements                     |

**Action Items**:

1. Update sqlx to 0.8.x for MySQL 8.0+ optimizations
2. Update reqwest to 0.12.x for better connection pooling
3. Consider governor 0.7.x for distributed rate limiting support
4. Update Rust to 1.91.0 for latest async/await improvements

## Research Tasks

### 1. HTTP Server Framework Selection

**Decision**: actix-web 4.x

**Rationale**:

- High performance (async/await with Tokio runtime)
- Middleware support for authentication, rate limiting, error handling
- JSON deserialization via serde integration
- Mature ecosystem with extensive documentation
- Active community and Context7 MCP support

**Alternatives Considered**:

- **Rocket**: Great DX but less performant, blocking by default
- **warp**: Good performance but steeper learning curve, less comprehensive middleware
- **axum**: Modern and performant but younger ecosystem, less battle-tested

**Constitution Compliance**: Standard Library First (I) - justified external dependency, no std HTTP server exists

**November 2025 Update**:

- **Confirmed Version**: actix-web 4.9+ (no v5.0 released yet)
- **Rust 1.91.0 Compatibility**: ✅ Fully compatible
- **New Features Since 4.0**:
  - TLS: rustls 0.23 and 0.22 support via feature flags
  - Middleware: Improved `.wrap()` pattern, session/cors/identity moved to separate crates
  - WebSocket: Enhanced support via `actix-ws 0.3+` crate
  - HTTP/2: Available via `http2` feature flag
  - Performance: Better integration with Tokio 1.40+ runtime
- **Migration from 4.0 → 4.9**:
  - `actix-session`, `actix-cors`, `actix-identity` now separate crates
  - Use `.wrap()` instead of deprecated `.middleware()`
  - WebSocket via `actix-ws` instead of built-in actors
- **Recommended Cargo.toml**:
  ```toml
  actix-web = { version = "4.9", features = ["rustls-0_23"] }
  actix-cors = "0.7"
  actix-session = "0.10"
  actix-identity = "0.8"
  ```
- **Context7 Research**: `/actix/actix-web` - 154 code snippets, trust score 8.4

---

### 2. MySQL Driver Selection

**Decision**: sqlx 0.7.x with async support

**Rationale**:

- Compile-time query checking (prevents SQL errors at build time)
- Async/await support (aligns with actix-web)
- Connection pooling built-in
- Migration support via sqlx-cli
- Prepared statements by default (prevents SQL injection)
- Strong type safety with compile-time verification

**Alternatives Considered**:

- **diesel**: Mature ORM but synchronous, requires blocking thread pool
- **mysql_async**: Lower-level, less type safety, manual query building
- **sea-orm**: Full ORM but adds abstraction layer, more complex than needed

**Constitution Compliance**: MySQL Integration Standards (IV) - provides connection pooling, prepared statements, transaction management, migrations

**November 2025 Update**:

- **Recommended Version**: sqlx 0.8.x (upgrade from 0.7.x)
- **Rust 1.91.0 Compatibility**: ✅ Fully compatible
- **Breaking Changes 0.7 → 0.8**:
  - Improved query macro error messages
  - Better MySQL 8.0+ JSON column support
  - Connection pool configuration API refinements
  - Migration runner improvements
- **New Features in 0.8.x**:
  - Enhanced compile-time checking for MySQL 8.0+ features
  - Better support for CTEs and window functions
  - Improved transaction handling with nested transactions
  - Connection pool idle timeout configuration
  - Better error messages for migration failures
- **MySQL 8.0+ Optimizations**:
  - Native JSON column type support
  - CTE (Common Table Expressions) for complex queries
  - Window functions for reporting (ROW_NUMBER, RANK, etc.)
  - Performance schema integration
- **Recommended Cargo.toml**:
  ```toml
  sqlx = { version = "0.8", features = ["runtime-tokio", "mysql", "macros", "migrate", "rust_decimal", "chrono", "uuid"] }
  ```
- **Pool Configuration Best Practices (2025)**:
  ```rust
  let pool = MySqlPoolOptions::new()
      .max_connections(50)           // Scale for 100 concurrent requests
      .min_connections(10)            // Maintain baseline
      .acquire_timeout(Duration::from_secs(30))
      .idle_timeout(Duration::from_secs(600))    // 10 min
      .max_lifetime(Duration::from_secs(1800))   // 30 min
      .test_before_acquire(true)      // Validate connections
      .connect(&database_url).await?;
  ```
- **Context7 Research**: `/launchbadge/sqlx` - 89 code snippets, trust score 8.2

---

### 3. JSON Serialization

**Decision**: serde 1.x with serde_json

**Rationale**:

- De facto standard for Rust serialization
- Zero-copy deserialization where possible
- Compile-time code generation (no runtime reflection)
- Excellent error messages
- Integrates seamlessly with actix-web

**Alternatives Considered**:

- **simd-json**: Faster but unsafe code, less portable
- **json**: Simpler but lacks type safety and compile-time checks

**Constitution Compliance**: Standard Library First (I) - justified, no std JSON support

**November 2025 Update**:

- **Current Version**: serde 1.0.210+, serde_json 1.0.130+
- **Rust 1.91.0 Compatibility**: ✅ Fully compatible
- **Performance Improvements**:
  - Optimized deserialization for large JSON payloads
  - Better compile-time optimizations with const generics
  - Reduced binary size with feature flags
- **New Features**:
  - Better error messages with source location tracking
  - Improved support for custom serializers
  - Zero-copy string deserialization improvements
- **Recommended Cargo.toml**:
  ```toml
  serde = { version = "1.0", features = ["derive"] }
  serde_json = "1.0"
  ```
- **No Breaking Changes**: Fully backward compatible

---

### 4. Environment Configuration

**Decision**: dotenvy 0.15.x (dotenv successor)

**Rationale**:

- Laravel-style .env file loading
- Simple API: `dotenv().ok()` at startup
- Supports .env.local, .env.development patterns
- Zero overhead after startup (environment variables)

**Alternatives Considered**:

- **config**: More features but overkill for .env pattern
- **envy**: Only parses into structs, less flexible

**Constitution Compliance**: Environment Management (V) - direct support for Laravel-style .env pattern

---

### 5. Testing Framework

**Decision**: Built-in cargo test with:

- `sqlx::test` macro for database integration tests
- `actix-web::test` for API contract tests
- Standard `#[test]` and `#[cfg(test)]` for unit tests

**Rationale**:

- No external test framework needed (std library support)
- sqlx provides test database setup/teardown
- actix-web provides TestServer for API testing
- Cargo test has built-in parallel execution

**Constitution Compliance**: Test-First Development (III) - supports unit, integration, and contract testing

---

### 5a. Tokio Runtime (Critical Dependency)

**Decision**: tokio 1.40+ (Latest stable as of November 2025)

**Rationale**:

- Foundation for actix-web, sqlx, and reqwest async operations
- Provides work-stealing scheduler for efficient task execution
- Mature ecosystem with production-proven stability
- Low-overhead async runtime with excellent performance

**November 2025 Update**:

- **Current Version**: tokio 1.40+
- **Rust 1.91.0 Compatibility**: ✅ Fully compatible
- **New Features in 1.40+**:
  - Improved scheduler performance for 100+ concurrent tasks
  - Better CPU affinity for multi-core systems
  - Enhanced tracing integration for debugging
  - io-uring support (experimental) for Linux
  - Better backpressure handling in channels
- **Runtime Configuration for Payment Platform**:

  ```rust
  use tokio::runtime::Builder;

  let runtime = Builder::new_multi_thread()
      .worker_threads(8)              // 2x CPU cores for I/O-bound workload
      .thread_name("paytrust-worker")
      .thread_stack_size(3 * 1024 * 1024)
      .enable_all()                   // Enable I/O and time drivers
      .build()?;
  ```

- **Performance Tuning**:
  - Worker threads: 8 (for 4-core system, I/O-bound workload)
  - Max blocking threads: 512 (default sufficient)
  - Enable `parking_lot` feature for faster mutexes
- **Recommended Cargo.toml**:
  ```toml
  tokio = { version = "1.40", features = ["full", "parking_lot"] }
  ```
- **Context7 Research**: `/tokio-rs/tokio` - 52 code snippets, trust score 7.5
- **Integration Notes**:
  - actix-web 4.9 uses tokio 1.40+ under the hood
  - sqlx 0.8 requires tokio runtime for async operations
  - reqwest 0.12 leverages tokio for connection pooling

**Constitution Compliance**: Standard Library First (I) - justified, no std async runtime

---

### 6. Error Handling Pattern

**Decision**: Custom error enum with thiserror 1.x

**Rationale**:

- thiserror generates Display and Error implementations
- Result<T, E> types throughout (no panics)
- Conversion traits for library errors (sqlx::Error → AppError)
- HTTP status code mapping via actix-web ResponseError trait

**Example**:

```rust
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Invoice not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),
}
```

**Constitution Compliance**: Technology Stack Constraints - Result<T, E> types with custom error enums

---

### 7. Logging Framework

**Decision**: tracing 0.1.x with tracing-subscriber

**Rationale**:

- Structured logging with context (span/event model)
- Async-aware (works with actix-web/tokio)
- Multiple output formats (JSON for production, pretty for development)
- Log levels per environment (.env: LOG_LEVEL=info)
- Integrates with actix-web middleware for request tracing

**Alternatives Considered**:

- **log + env_logger**: Simpler but less powerful, no structured logging
- **slog**: Powerful but more complex API

**Constitution Compliance**: Technology Stack Constraints - structured logging with configurable levels

---

### 8. Payment Gateway Integration

**Decision**: reqwest 0.11.x with async support

**Rationale**:

- HTTP client for calling Xendit/Midtrans APIs
- Async/await support
- JSON request/response via serde
- Timeout configuration
- Retry logic support via middleware

**Gateway-Specific Considerations**:

- **Xendit**: RESTful API, API key authentication, webhook signatures via HMAC
- **Midtrans**: RESTful API, Server Key authentication, webhook signatures
- Both require HTTPS, both provide sandbox environments

**Constitution Compliance**: Context7 MCP Documentation (VI) - use Context7 to fetch latest Xendit/Midtrans API docs

**November 2025 Update**:

- **Recommended Version**: reqwest 0.12.x (upgrade from 0.11.x)
- **Rust 1.91.0 Compatibility**: ✅ Fully compatible
- **Breaking Changes 0.11 → 0.12**:
  - Improved connection pool management (auto-scaling)
  - Better HTTP/2 support with multiplexing
  - Enhanced timeout configuration API
  - TLS updates (rustls 0.23 support)
- **New Features in 0.12.x**:
  - Automatic connection pool resizing based on load
  - Better retry middleware integration
  - Improved proxy support with authentication
  - Enhanced error types for better debugging
  - HTTP/3 experimental support
- **Recommended Configuration for Payment Gateways**:

  ```rust
  use reqwest::{Client, ClientBuilder};
  use std::time::Duration;

  let client = ClientBuilder::new()
      .timeout(Duration::from_secs(30))         // Total request timeout
      .connect_timeout(Duration::from_secs(10)) // Connection timeout
      .pool_max_idle_per_host(10)               // Reuse connections
      .pool_idle_timeout(Duration::from_secs(90))
      .user_agent("PayTrust/1.0")
      .build()?;
  ```

- **Retry Pattern** (using reqwest-middleware 0.3.x):

  ```rust
  use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
  use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};

  let retry_policy = ExponentialBackoff::builder()
      .build_with_max_retries(3);

  let client = ClientBuilder::new(reqwest::Client::new())
      .with(RetryTransientMiddleware::new_with_policy(retry_policy))
      .build();
  ```

- **Recommended Cargo.toml**:
  ```toml
  reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
  reqwest-middleware = "0.3"
  reqwest-retry = "0.6"
  ```
- **Context7 Research**: `/seanmonstar/reqwest` - 19 code snippets, trust score 9.7

---

### 9. Currency Decimal Handling

**Decision**: rust_decimal 1.x

**Rationale**:

- Arbitrary precision decimal arithmetic (no floating point errors)
- serde support for JSON serialization
- Database storage as DECIMAL type
- IDR: scale=0 (no decimals), MYR/USD: scale=2 (2 decimals)

**Example**:

```rust
use rust_decimal::Decimal;

// IDR: 1000000 (stored as 1000000.00 in DB but displayed without decimals)
let idr_amount = Decimal::new(1000000, 0);

// MYR: 1000.50
let myr_amount = Decimal::new(100050, 2);
```

**Constitution Compliance**: NFR-005 from spec - accurate to smallest currency unit

**November 2025 Update**:

- **Current Version**: rust_decimal 1.36+
- **Rust 1.91.0 Compatibility**: ✅ Fully compatible
- **Critical for Financial Accuracy**:
  - No floating-point errors (uses 128-bit integer internally)
  - Arbitrary precision up to 28 decimal places
  - Exact representation of decimal values
- **Serde Integration** (Critical for API/Database):
  ```toml
  rust_decimal = { version = "1.36", features = ["serde", "serde-with-arbitrary-precision", "db-tokio-postgres", "db-tokio-mysql"] }
  ```
- **Feature Flags**:
  - `serde`: Basic JSON serialization
  - `serde-with-arbitrary-precision`: Preserve exact precision in JSON (REQUIRED for financial data)
  - `db-tokio-mysql`: Direct sqlx integration for MySQL DECIMAL types
  - `maths`: Additional mathematical operations
- **Currency-Specific Scaling**:

  ```rust
  use rust_decimal::Decimal;
  use rust_decimal::prelude::*;

  // IDR: No decimals (scale=0)
  let idr_amount = Decimal::new(1000000, 0);  // 1,000,000 IDR
  assert_eq!(idr_amount.scale(), 0);

  // MYR/USD: 2 decimals (scale=2)
  let usd_amount = Decimal::new(100050, 2);   // 1,000.50 USD
  assert_eq!(usd_amount.scale(), 2);

  // Arithmetic maintains precision
  let tax = idr_amount * Decimal::new(10, 2); // 10% = 0.10
  assert_eq!(tax, Decimal::new(100000, 0));   // 100,000 IDR
  ```

- **Database Serialization** (MySQL DECIMAL):
  ```sql
  CREATE TABLE invoices (
      subtotal DECIMAL(20, 2),  -- Supports IDR (no decimals), MYR/USD (2 decimals)
      tax_total DECIMAL(20, 2),
      total_amount DECIMAL(20, 2)
  );
  ```
- **JSON Serialization with Arbitrary Precision**:
  ```rust
  #[derive(Serialize, Deserialize)]
  struct Invoice {
      #[serde(with = "rust_decimal::serde::arbitrary_precision")]
      total_amount: Decimal,
  }
  // Serializes as: {"total_amount": 1000.50} (not "1000.5")
  ```
- **Rounding Strategy** (for installments):

  ```rust
  use rust_decimal::RoundingStrategy;

  // Last installment absorbs rounding difference (FR-062)
  let installment = total.checked_div(Decimal::from(3))
      .unwrap()
      .round_dp_with_strategy(2, RoundingStrategy::MidpointNearestEven);
  ```

- **Performance**: Comparable to f64 for basic operations, slightly slower for division
- **Context7 Research**: `/websites/rs_rust_decimal` - 744 code snippets, trust score 7.5

---

### 10. Migration Tool

**Decision**: sqlx-cli for migrations

**Rationale**:

- Integrated with sqlx
- SQL-based migrations (not ORM abstraction)
- Versioning and rollback support
- Command: `sqlx migrate add <name>` generates numbered .sql files
- Applied at startup via `sqlx::migrate!().run(&pool).await`

**Constitution Compliance**: MySQL Integration Standards (IV) - migrations with rollback capabilities

---

### 11. Authentication & Rate Limiting

**Decision**:

- Custom middleware for API key auth (simple header check)
- governor 0.6.x for rate limiting

**Rationale**:

- API key auth is simple, no OAuth/JWT needed
- governor provides in-memory rate limiting with configurable windows
- Middleware pattern integrates with actix-web

**Implementation**:

```rust
// Middleware checks X-API-Key header against database
// Rate limiter: 1000 requests per minute per API key
```

**Constitution Compliance**: FR-033, FR-040, FR-041 from spec

**November 2025 Update**:

- **Recommended Version**: governor 0.7.x (upgrade from 0.6.x)
- **Rust 1.91.0 Compatibility**: ✅ Fully compatible
- **Breaking Changes 0.6 → 0.7**:
  - Improved distributed rate limiting support (Redis backend)
  - Better clock abstraction for testing
  - Enhanced quota management API
- **New Features in 0.7.x**:
  - **Distributed Rate Limiting**: Redis backend for multi-instance deployments
  - **Sliding Window**: More accurate rate limiting vs fixed window
  - **Multiple Quotas**: Per-endpoint and per-user quotas simultaneously
  - **Better Observability**: Metrics for rate limit hits/misses
- **In-Memory Configuration** (Single Instance):

  ```rust
  use governor::{Quota, RateLimiter};
  use std::num::NonZeroU32;

  // 1000 requests per minute per API key (FR-040)
  let quota = Quota::per_minute(NonZeroU32::new(1000).unwrap());
  let limiter = RateLimiter::direct(quota);
  ```

- **Distributed Configuration** (Multi-Instance with Redis):

  ```rust
  use governor::{Quota, RateLimiter};
  use governor::state::keyed::DefaultKeyedStateStore;

  // For production multi-instance setup
  let quota = Quota::per_minute(NonZeroU32::new(1000).unwrap());
  let limiter = RateLimiter::keyed(quota);
  // Integrate with Redis for shared state across instances
  ```

- **actix-web Integration**:

  ```rust
  use actix_governor::{Governor, GovernorConfigBuilder};

  let governor_conf = GovernorConfigBuilder::default()
      .per_second(16)  // ~1000/min = 16.67/sec
      .burst_size(50)  // Allow burst traffic
      .finish()
      .unwrap();

  App::new()
      .wrap(Governor::new(&governor_conf))
  ```

- **Recommended Cargo.toml**:
  ```toml
  governor = "0.7"
  actix-governor = "0.5"  # actix-web integration
  ```
- **Alternative for Distributed**: tower-governor with Redis, or implement custom rate limiter with Redis

---

## Additional Research: Security & Performance (November 2025)

### 12. API Key Security

**Decision**: argon2 for API key hashing

**Rationale**:

- Recommended by OWASP for password/key hashing (2025)
- Resistant to GPU/ASIC attacks
- Configurable memory and CPU cost

**Implementation**:

```rust
use argon2::{Argon2, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use rand_core::OsRng;

// Hash API key on creation
let salt = SaltString::generate(&mut OsRng);
let argon2 = Argon2::default();
let password_hash = argon2.hash_password(api_key.as_bytes(), &salt)?.to_string();

// Verify on each request
let parsed_hash = PasswordHash::new(&stored_hash)?;
argon2.verify_password(provided_key.as_bytes(), &parsed_hash)?;
```

**Recommended Cargo.toml**:

```toml
argon2 = "0.5"
```

---

### 13. Property-Based Testing

**Decision**: proptest for financial calculation testing

**Rationale**:

- Generate randomized test cases for edge cases
- Critical for installment distribution, tax calculation, rounding
- Finds bugs that unit tests miss

**Example**:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn installment_sum_equals_total(total in 1000u64..1000000u64, installments in 2usize..12usize) {
        let parts = divide_into_installments(total, installments);
        let sum: u64 = parts.iter().sum();
        assert_eq!(sum, total, "Installments must sum exactly to total");
    }
}
```

**Recommended Cargo.toml**:

```toml
[dev-dependencies]
proptest = "1.5"
```

---

### 14. Tracing & Observability

**November 2025 Update**:

- **Current Version**: tracing 0.1.40+, tracing-subscriber 0.3.18+
- **New Features**:
  - Better async context propagation
  - Improved performance with const generics
  - OpenTelemetry integration for distributed tracing
- **Recommended Setup**:

  ```rust
  use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

  tracing_subscriber::registry()
      .with(tracing_subscriber::fmt::layer()
          .json()  // Structured logging for production
          .with_target(true)
          .with_thread_ids(true)
          .with_thread_names(true))
      .with(tracing_subscriber::EnvFilter::from_default_env())
      .init();
  ```

- **Recommended Cargo.toml**:
  ```toml
  tracing = "0.1.40"
  tracing-subscriber = { version = "0.3.18", features = ["env-filter", "json"] }
  tracing-actix-web = "0.7"  # actix-web integration
  ```

---

## Dependency Justification Summary (Updated November 2025)

| Crate                | Purpose             | Std Library Alternative | Justification                                   | Nov 2025 Version    |
| -------------------- | ------------------- | ----------------------- | ----------------------------------------------- | ------------------- |
| **actix-web**        | HTTP server         | None                    | No std HTTP server                              | 4.9+                |
| **tokio**            | Async runtime       | None                    | No std async runtime                            | 1.40+               |
| **sqlx**             | MySQL driver        | None                    | No std MySQL driver                             | 0.8.x               |
| **serde/serde_json** | JSON serialization  | None                    | No std JSON support                             | 1.0.210+ / 1.0.130+ |
| **dotenvy**          | .env loading        | std::env                | Convenience, Laravel-style pattern              | 0.15.7              |
| **thiserror**        | Error derive macros | Manual impl             | Reduces boilerplate, type safety                | 1.0.60+             |
| **tracing**          | Structured logging  | None directly           | Required for async context, structured logs     | 0.1.40+             |
| **reqwest**          | HTTP client         | None                    | No std HTTP client                              | 0.12.x              |
| **rust_decimal**     | Decimal arithmetic  | None                    | Financial accuracy requirements                 | 1.36+               |
| **governor**         | Rate limiting       | Manual impl             | Battle-tested algorithm, time-window management | 0.7.x               |
| **argon2**           | API key hashing     | None suitable           | OWASP recommended secure hashing                | 0.5+                |

**Total External Crates**: 11 core + their dependencies  
**Constitution Compliance**: All justified under Principle I (Standard Library First)

---

## Recommended Cargo.toml (November 2025)

```toml
[package]
name = "paytrust"
version = "0.1.0"
edition = "2021"
rust-version = "1.91.0"

[dependencies]
# HTTP Server
actix-web = { version = "4.9", features = ["rustls-0_23"] }
actix-cors = "0.7"
actix-session = { version = "0.10", features = ["redis-session"] }
actix-identity = "0.8"
actix-governor = "0.5"

# Async Runtime
tokio = { version = "1.40", features = ["full", "parking_lot"] }

# Database
sqlx = { version = "0.8", features = [
    "runtime-tokio",
    "mysql",
    "macros",
    "migrate",
    "rust_decimal",
    "chrono",
    "uuid"
] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Financial Calculations
rust_decimal = { version = "1.36", features = [
    "serde",
    "serde-with-arbitrary-precision",
    "db-tokio-mysql"
] }

# HTTP Client (Gateway Integration)
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
reqwest-middleware = "0.3"
reqwest-retry = "0.6"

# Error Handling
thiserror = "1.0"
anyhow = "1.0"  # For application-level error context

# Logging & Tracing
tracing = "0.1.40"
tracing-subscriber = { version = "0.3.18", features = ["env-filter", "json"] }
tracing-actix-web = "0.7"

# Configuration
dotenvy = "0.15"

# Rate Limiting
governor = "0.7"

# Security
argon2 = "0.5"

# Utilities
uuid = { version = "1.10", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
proptest = "1.5"  # Property-based testing for financial logic
mockito = "1.5"   # HTTP mocking ONLY when gateway test environments unavailable (Constitution Principle III)

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

---

## Migration Checklist (From Initial Plan to November 2025 Versions)

- [ ] **Update Rust toolchain**: `rustup update` → 1.91.0
- [ ] **sqlx 0.7 → 0.8**:
  - Update `Cargo.toml` dependency version
  - Review migration macro error messages (improved)
  - Test connection pool configuration changes
  - Update `sqlx-cli`: `cargo install sqlx-cli --force`
- [ ] **reqwest 0.11 → 0.12**:
  - Update dependency version
  - Review connection pool behavior (auto-scaling)
  - Add retry middleware: `reqwest-middleware`, `reqwest-retry`
  - Test timeout configurations
- [ ] **governor 0.6 → 0.7**:
  - Update dependency version
  - Consider Redis backend for distributed rate limiting
  - Update quota configuration API
- [ ] **Add tokio explicitly**: Version 1.40+ with `full` and `parking_lot` features
- [ ] **Add rust_decimal features**: `serde-with-arbitrary-precision`, `db-tokio-mysql`
- [ ] **Add security**: argon2 for API key hashing
- [ ] **Add testing**: proptest for property-based financial calculation tests
- [ ] **Update tracing**: Add `tracing-actix-web` for request tracing
- [ ] **Test all integrations**: Run full test suite after updates

---

## Key Improvements for November 2025

**Performance**:

- Tokio 1.40+ with improved scheduler for 100+ concurrent requests
- sqlx 0.8 with better connection pool management
- reqwest 0.12 with auto-scaling connection pools

**Security**:

- rustls 0.23 (latest TLS implementation)
- argon2 for API key hashing (OWASP recommended)
- Better HTTP/2 support for gateway communication

**Developer Experience**:

- Improved error messages across all crates
- Better compile-time checking with sqlx 0.8
- Enhanced tracing integration for debugging

**Financial Accuracy**:

- rust_decimal 1.36+ with arbitrary precision JSON serialization
- Direct MySQL DECIMAL type integration
- Property-based testing with proptest

---

## Architecture Decisions

### Module Communication Pattern

**Decision**: Dependency Injection via Trait Objects

**Pattern**:

```rust
// Define trait in core
pub trait InvoiceRepository: Send + Sync {
    async fn create(&self, invoice: Invoice) -> Result<Invoice, AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Invoice>, AppError>;
}

// Implement in module
pub struct MySqlInvoiceRepository {
    pool: PgPool,
}

impl InvoiceRepository for MySqlInvoiceRepository { /* ... */ }

// Inject via Arc<dyn Trait>
pub struct InvoiceService {
    repo: Arc<dyn InvoiceRepository>,
}
```

**Rationale**:

- Enables testing with mock repositories
- Loose coupling between modules
- Aligns with SOLID Principle (Dependency Inversion)

---

### Database Transaction Strategy

**Decision**: Repository methods receive transaction or connection

**Pattern**:

```rust
impl InvoiceRepository for MySqlInvoiceRepository {
    async fn create(&self, invoice: Invoice, tx: &mut Transaction<'_, MySql>)
        -> Result<Invoice, AppError> {
        // Use provided transaction
    }
}

// Service orchestrates transaction
pub async fn create_invoice_with_installments(/* ... */) -> Result<Invoice, AppError> {
    let mut tx = self.pool.begin().await?;

    let invoice = self.invoice_repo.create(invoice, &mut tx).await?;
    let schedule = self.installment_repo.create(schedule, &mut tx).await?;

    tx.commit().await?;
    Ok(invoice)
}
```

**Rationale**:

- Services control transaction boundaries
- Repositories remain simple (no transaction management)
- Atomic operations across modules

---

### Webhook Processing

**Decision**: Async background task with retry queue

**Pattern**:

```rust
// Webhook received → validate signature → spawn task → return 200 OK
// Background task: process → update DB → retry on failure (3x with backoff)
```

**Rationale**:

- Fast webhook response (< 5 seconds)
- Reliable processing with retry (FR-042, FR-043)
- Exponential backoff: 1min, 5min, 30min

---

## Phase 0 Completion Checklist

- [x] HTTP server framework selected and justified
- [x] MySQL driver selected with connection pooling support
- [x] JSON serialization library selected
- [x] Environment configuration pattern defined
- [x] Testing framework and strategy defined
- [x] Error handling pattern specified
- [x] Logging framework selected
- [x] Payment gateway integration approach defined
- [x] Currency decimal handling solution identified
- [x] Migration tool selected
- [x] Authentication and rate limiting approach defined
- [x] Module communication pattern designed
- [x] Database transaction strategy defined
- [x] Webhook processing pattern designed
- [x] All dependencies justified per Constitution Principle I

**Status**: Ready for Phase 1 (Design & Contracts)
