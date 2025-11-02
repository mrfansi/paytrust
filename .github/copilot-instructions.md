# paytrust Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-11-01

## Active Technologies
- Rust 1.91.0 with 2021 edition + actix-web 4.9, actix-test (to be added), tokio 1.40, sqlx 0.8, reqwest 0.12 (002-real-endpoint-testing)
- MySQL 8.0+ test database (paytrust_test) (002-real-endpoint-testing)

- Rust 1.91.0+ (stable Nov 2025) with 2021 edition features (001-payment-orchestration-api)
- MySQL 8.0+ with InnoDB storage engine (001-payment-orchestration-api)
- actix-web 4.9+ - HTTP server with rustls 0.23 TLS (001-payment-orchestration-api)
- tokio 1.40+ - Async runtime with improved scheduler (001-payment-orchestration-api)
- sqlx 0.8.x - Async MySQL driver with enhanced compile-time checking (001-payment-orchestration-api)
- serde 1.0.210+ with serde_json 1.0.130+ - JSON serialization (001-payment-orchestration-api)
- dotenvy 0.15.7 - Environment configuration (001-payment-orchestration-api)
- thiserror 1.0.60+ - Custom error types (001-payment-orchestration-api)
- tracing 0.1.40+ with tracing-subscriber 0.3.18+ - Structured logging (001-payment-orchestration-api)
- reqwest 0.12.x - HTTP client with auto-scaling connection pools (001-payment-orchestration-api)
- rust_decimal 1.36+ - Financial arithmetic with arbitrary precision (001-payment-orchestration-api)
- governor 0.7.x - Rate limiting with distributed support (001-payment-orchestration-api)
- argon2 0.5+ - API key hashing (OWASP recommended) (001-payment-orchestration-api)

## Project Structure

```text
src/
  modules/
    invoices/         # Invoice management (models, services, repositories, controllers)
    installments/     # Installment scheduling and payments
    taxes/            # Tax calculation and reporting
    transactions/     # Payment transaction tracking
    gateways/         # Payment gateway integrations (Xendit, Midtrans)
    reports/          # Financial reporting aggregates
  core/
    traits/           # Common trait definitions for DI
    errors/           # Application error types
    config/           # Configuration management
  main.rs             # Application entry point
migrations/           # SQLx database migrations
tests/
  integration/        # Integration tests with test database
  contract/           # OpenAPI contract tests
config/
  .env.example        # Environment variable template
specs/
  001-payment-orchestration-api/
    spec.md           # Feature specification with 21 clarifications
    plan.md           # Implementation plan
    research.md       # Phase 0 research decisions
    data-model.md     # Phase 1 entity definitions
    contracts/
      openapi.yaml    # Phase 1 API specification
    quickstart.md     # Development setup guide
```

## Commands

```bash
# Build project
cargo build

# Run tests (TDD workflow per Constitution Principle III)
cargo test                    # All tests
cargo test --lib              # Unit tests only
cargo test --test '*'         # Integration tests only

# Code quality
cargo clippy                  # Linter
cargo fmt                     # Format code

# Database migrations (sqlx-cli required)
sqlx migrate run              # Apply migrations
sqlx migrate revert           # Rollback last migration
sqlx migrate info             # Show migration status

# Development server
cargo run                     # Start API server (http://127.0.0.1:8080)

# Production build
cargo build --release
```

## Code Style

Rust 1.75+ with 2021 edition features: Follow standard conventions

## Recent Changes
- 002-real-endpoint-testing: Added Rust 1.91.0 with 2021 edition + actix-web 4.9, actix-test (to be added), tokio 1.40, sqlx 0.8, reqwest 0.12

- 001-payment-orchestration-api: Added Rust 1.75+ with 2021 edition features

<!-- MANUAL ADDITIONS START -->

- Dont write any summary documents in anything format
<!-- MANUAL ADDITIONS END -->
