# paytrust Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-11-01

## Active Technologies

- Rust 1.75+ with 2021 edition features (001-payment-orchestration-api)
- MySQL 8.0+ with InnoDB storage engine (001-payment-orchestration-api)
- actix-web 4.x - HTTP server with Tokio async runtime (001-payment-orchestration-api)
- sqlx 0.7.x - Async MySQL driver with compile-time query checking (001-payment-orchestration-api)
- serde 1.x with serde_json - JSON serialization (001-payment-orchestration-api)
- dotenvy 0.15.x - Environment configuration (001-payment-orchestration-api)
- thiserror 1.x - Custom error types (001-payment-orchestration-api)
- tracing 0.1.x with tracing-subscriber - Structured logging (001-payment-orchestration-api)
- reqwest 0.11.x - HTTP client for payment gateway integration (001-payment-orchestration-api)
- rust_decimal 1.x - Financial arithmetic with precision (001-payment-orchestration-api)
- governor 0.6.x - In-memory rate limiting (001-payment-orchestration-api)

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

- 001-payment-orchestration-api: Added Rust 1.75+ with 2021 edition features

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
