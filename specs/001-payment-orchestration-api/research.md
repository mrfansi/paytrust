# Research: PayTrust Payment Orchestration Platform

**Date**: 2025-11-01  
**Status**: Phase 0 Complete

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

---

## Dependency Justification Summary

| Crate            | Purpose             | Std Library Alternative | Justification                                   |
| ---------------- | ------------------- | ----------------------- | ----------------------------------------------- |
| actix-web        | HTTP server         | None                    | No std HTTP server                              |
| sqlx             | MySQL driver        | None                    | No std MySQL driver                             |
| serde/serde_json | JSON serialization  | None                    | No std JSON support                             |
| dotenvy          | .env loading        | std::env                | Convenience, Laravel-style pattern              |
| thiserror        | Error derive macros | Manual impl             | Reduces boilerplate, type safety                |
| tracing          | Structured logging  | None directly           | Required for async context, structured logs     |
| reqwest          | HTTP client         | None                    | No std HTTP client                              |
| rust_decimal     | Decimal arithmetic  | None                    | Financial accuracy requirements                 |
| governor         | Rate limiting       | Manual impl             | Battle-tested algorithm, time-window management |

**Total External Crates**: 9 core + their dependencies  
**Constitution Compliance**: All justified under Principle I (Standard Library First)

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
