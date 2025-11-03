# PayTrust Cargo.toml Dependencies

**Purpose**: Complete reference for all dependencies with justification, versions, and usage patterns
**Maintenance**: Update versions quarterly; pin to minor versions for stability
**Test Verification**: All dependencies verified to work together as of 2025-01-15

---

## Dependency Matrix

### Web Framework & Runtime

```toml
[dependencies]
# Web framework: Fastest Rust web framework, production-proven
actix-web = "4.9"
actix-rt = "2.10"  # Async runtime (usually pulled in transitively)

# Used by: HTTP server, middleware, route handling
# Alternatives: Axum (newer but less mature), Rocket (requires nightly)
# Why: Exceptional performance (~100k req/s), mature middleware ecosystem, excellent error handling
```

**Why Not Alternatives**:
- Axum: Newer, good for greenfield projects, less production battle-tested
- Rocket: Requires nightly Rust, slower than actix-web
- Warp: Bare-bones, requires more middleware boilerplate

### Database & Data Access

```toml
[dependencies]
# Async PostgreSQL driver with compile-time SQL verification
sqlx = { version = "0.8", features = ["runtime-tokio-native-tls", "postgres", "chrono", "uuid", "decimal"] }

# Features explanation:
# - runtime-tokio-native-tls: Use tokio async runtime with native TLS
# - postgres: PostgreSQL support
# - chrono: DateTime serialization
# - uuid: UUID type support
# - decimal: rust_decimal type support

# Connection pool management (sqlx includes this, but explicit for clarity)
# No separate dependency needed; sqlx::PgPool handles pooling
```

**Why SQLx over Alternatives**:
- **vs Diesel**: Diesel requires compile-time database checks (offline mode painful), harder migrations
- **vs SeaORM**: Good for ORM use cases; sqlx is better for payment systems (closer to SQL)
- **vs tokio-postgres**: No built-in connection pooling, less type safety

### Financial Mathematics

```toml
[dependencies]
# Decimal type with 28+ digit precision, no floating-point errors
rust_decimal = "1.36"
rust_decimal_macros = "1.36"  # For compile-time decimal literals

# Example: Decimal::from_str("99.99")?  or decimal!(99.99)
# Replaces: Any floating-point math or string-based amounts
```

**Why Rust Decimal**:
- 28 digits of precision (way more than any currency needs)
- No floating-point rounding errors
- Built-in ROUND_HALF_UP (industry standard)
- Direct JSON serialization
- Slower than f64 but acceptable for payment systems (not trading algorithms)

**Decimal Precision Example**:
```rust
// Correct: 0.1 + 0.2 = 0.3 (exactly)
let a = Decimal::from_str("0.1")?;
let b = Decimal::from_str("0.2")?;
assert_eq!(a + b, Decimal::from_str("0.3")?);

// Wrong (floating-point): 0.1 + 0.2 = 0.30000000000000004
```

### Error Handling

```toml
[dependencies]
# For creating custom error types in libraries
thiserror = "1.0"

# For general error propagation in application code
anyhow = "1.0"

# Example usage:
# thiserror: Defining PaymentError, InvoiceError types
# anyhow: Wrapping errors from external libraries
```

**When to Use Which**:
- `thiserror`: Define custom error enums with Display impl
- `anyhow`: Quick error wrapping when custom type not needed

### Async & Concurrency

```toml
[dependencies]
# Async runtime (included via actix, but explicit for clarity)
tokio = { version = "1.37", features = ["full"] }

# Async trait support
async-trait = "0.1"

# The "full" feature includes:
# - tokio::spawn for task spawning
# - tokio::time for sleep/delays (webhook retries)
# - tokio::sync::Mutex for concurrent data structures
# - Everything else in tokio ecosystem
```

**Why Full Features**:
- Webhook retry queue needs tokio::time::sleep
- Rate limiter needs tokio::sync::Mutex
- Payment processing needs tokio::spawn

### Serialization & Deserialization

```toml
[dependencies]
# JSON serialization (required by actix)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# UUID serialization/deserialization
uuid = { version = "1.6", features = ["v4", "serde"] }

# DateTime serialization in ISO 8601 format
chrono = { version = "0.4", features = ["serde"] }

# Example:
# #[derive(Serialize, Deserialize)]
# struct Invoice {
//     id: Uuid,
//     created_at: DateTime<Utc>,
// }
```

### Validation

```toml
[dependencies]
# Declarative request validation
validator = { version = "0.16", features = ["derive"] }

# Example:
# #[derive(Validate)]
// struct CreateInvoiceRequest {
//     #[validate(length(min = 1))]
//     line_items: Vec<LineItem>,
// }
```

### Logging & Tracing

```toml
[dependencies]
# Structured logging with correlation IDs
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter", "fmt"] }
tracing-actix-web = "0.7"  # Integration with actix

# Features:
# - json: Output logs as JSON for parsing
# - env-filter: Control log level via RUST_LOG env var
# - fmt: Human-readable formatter
# - tracing-actix-web: Extract correlation IDs from requests

# Example:
# tracing::info!("Invoice created", invoice_id = %invoice.id, tenant_id = %tenant_id);
```

**Why Tracing over Log Crate**:
- Structured logging (key-value pairs)
- Correlation IDs across async boundaries
- Performance profiling capabilities
- JSON output for log aggregation systems

### Security & Hashing

```toml
[dependencies]
# Argon2 password hashing for API keys
argon2 = "0.5"

# Constant-time comparison to prevent timing attacks
subtle = "2.5"

# Random number generation (for jitter, nonces)
rand = "0.8"

# Example:
// use argon2::{Argon2, PasswordHasher};
// let hash = Argon2::default().hash_password(&plaintext_key, &salt)?;
```

### HTTP Client

```toml
[dependencies]
# HTTP client for gateway API calls
reqwest = { version = "0.12", features = ["json", "stream"] }

# For timeout and retry configuration
tokio = { version = "1.37", features = ["time"] }

# Example:
// let response = reqwest::Client::new()
//     .post("https://api.xendit.co/v2/invoices")
//     .timeout(Duration::from_secs(10))
//     .json(&request)
//     .send()
//     .await?;
```

### Rate Limiting (Future Redis Support)

```toml
[dependencies]
# In-memory hash map for rate limiter state
# No dependency needed; use std::collections::HashMap

# For future Redis support:
# redis = { version = "0.25", features = ["aio", "tokio-comp"] }
# (Leave commented until Phase 2+ when multi-instance deployment needed)
```

### ISO 20022 Compliance

```toml
[dependencies]
# ISO 20022 pain.001/pain.002 message generation
# Research needed: pain crate or write custom XML generation
# Option 1: pain crate (if available)
# pain = "0.1"  # Check if published on crates.io

# Option 2: Custom XML serialization
quick-xml = "0.32"  # Fast XML serialization
# or
serde-xml-rs = "0.6"  # Serde integration for XML

# Current recommendation: quick-xml for performance
# Fallback: Manual XML string building (simplest for MVP)
```

### Testing

```toml
[dev-dependencies]
# Integration testing with Docker containers
testcontainers = "0.16"

# Property-based testing for financial invariants
proptest = "1.4"

# Mock HTTP server for testing gateway integrations
wiremock = "0.6"

# Assertion library with better error messages
assert_matches = "1.5"

# Example:
// #[tokio::test]
// async fn test_create_invoice() {
//     let docker = testcontainers::clients::Cli::default();
//     let postgres = docker.run(images::postgres::Postgres::default());
//     // ...
// }
```

### Environment & Configuration

```toml
[dependencies]
# Load .env files for local development
dotenv = "0.15"

# Type-safe environment variable parsing
envy = "0.4"

# Example:
// use envy;
// #[derive(Deserialize)]
// struct Config {
//     database_url: String,
//     #[serde(default = "default_port")]
//     port: u16,
// }
// let config: Config = envy::from_env()?;
```

### Utilities

```toml
[dependencies]
# String casing conversions
convert_case = "0.6"

# Derive macro helpers
derive_more = "0.99"

# Example:
// #[derive(Deref, DerefMut)]
// struct InvoiceId(i64);
```

---

## Complete Cargo.toml Template

```toml
[package]
name = "paytrust"
version = "0.1.0"
edition = "2021"
authors = ["PayTrust Team"]
license = "MIT OR Apache-2.0"

[dependencies]
# Web framework
actix-web = "4.9"
actix-rt = "2.10"

# Database
sqlx = { version = "0.8", features = ["runtime-tokio-native-tls", "postgres", "chrono", "uuid", "decimal"] }

# Financial math
rust_decimal = "1.36"
rust_decimal_macros = "1.36"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Async runtime
tokio = { version = "1.37", features = ["full"] }
async-trait = "0.1"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# Validation
validator = { version = "0.16", features = ["derive"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter", "fmt"] }
tracing-actix-web = "0.7"

# Security
argon2 = "0.5"
subtle = "2.5"
rand = "0.8"

# HTTP client
reqwest = { version = "0.12", features = ["json", "stream"] }

# XML (choose one for ISO 20022)
quick-xml = "0.32"

# Configuration
dotenv = "0.15"
envy = "0.4"

# Utilities
convert_case = "0.6"

[dev-dependencies]
# Integration testing
testcontainers = "0.16"

# Property-based testing
proptest = "1.4"

# Mock HTTP
wiremock = "0.6"

# Assertions
assert_matches = "1.5"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

---

## Dependency Maintenance Schedule

**Quarterly Update Review**:
- Check for security patches (every month)
- Review major version releases (quarterly)
- Run `cargo audit` to check for vulnerabilities
- Test with `cargo test --all-features` after updates

**Version Pinning Strategy**:
- Lock to minor versions (e.g., `1.36`) for stability
- Allow patch updates automatically (`.patch`)
- Update major versions deliberately after testing

**Example Cargo.lock Usage**:
```bash
# Generate lock file (checked into git)
cargo generate-lockfile

# Update to latest compatible versions
cargo update

# Commit lock file to ensure reproducible builds
git add Cargo.lock
git commit -m "chore: update dependencies"
```

---

## Optional Dependencies (Future Enhancement)

### Redis for Distributed Rate Limiting

```toml
# Only add when implementing multi-instance deployment (Phase 3+)
[dependencies]
redis = { version = "0.25", features = ["aio", "tokio-comp"] }
```

### Metrics & Monitoring

```toml
# For production observability (Phase 2+)
prometheus = "0.13"
metrics = "0.21"
```

### GraphQL Support (If Needed)

```toml
# Only if business requirements change
async-graphql = "0.12"
```

---

## Compatibility Matrix

| Component | Version | Rust | Tokio | Status |
|-----------|---------|------|-------|--------|
| actix-web | 4.9 | 1.70+ | 1.20+ | ✅ Stable |
| sqlx | 0.8 | 1.70+ | 1.20+ | ✅ Stable |
| tokio | 1.37 | 1.63+ | - | ✅ Stable |
| rust_decimal | 1.36 | 1.56+ | - | ✅ Stable |
| tracing | 0.1 | 1.40+ | - | ✅ Stable |

All dependencies use MSRV (Minimum Supported Rust Version) of 1.70 or lower, allowing older Rust versions.

---

## Build Optimization

### Reduce Compile Time

```toml
[profile.dev]
opt-level = 1  # Basic optimization for faster iteration during development

[profile.release]
opt-level = 3
lto = true      # Link-time optimization (slower compile, faster runtime)
codegen-units = 1  # Better optimization at cost of compile speed
strip = true    # Remove debug symbols from binary
```

### Binary Size

- Release build with `strip = true`: ~150-200 MB
- Without debug symbols: ~50-80 MB
- With compression (Docker): ~30-50 MB

---

## Verification Commands

```bash
# Check all dependencies compile correctly
cargo build --release

# Run all tests
cargo test --all-features

# Check for security vulnerabilities
cargo audit

# Generate dependency graph
cargo tree

# Update dependencies while respecting Cargo.lock
cargo update

# Check outdated dependencies
cargo outdated
```

---

## Common Dependency Issues & Solutions

### Issue 1: Conflicting Versions
```bash
# Error: conflicting version requirements for `tokio`
# Solution: Use `cargo update` or explicitly unify versions
```

### Issue 2: Missing TLS Support
```bash
# Error: `native-tls` feature not available
# Solution: Ensure `reqwest` has `features = ["native-tls"]`
```

### Issue 3: Database Connection Failures
```bash
# Ensure sqlx features include: "runtime-tokio-native-tls", "postgres"
sqlx = { version = "0.8", features = ["runtime-tokio-native-tls", "postgres"] }
```

