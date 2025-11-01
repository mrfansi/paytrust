# Implementation Plan: PayTrust Payment Orchestration Platform

**Branch**: `001-payment-orchestration-api` | **Date**: 2025-11-01 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-payment-orchestration-api/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

PayTrust is a payment orchestration platform that unifies multiple payment gateways (Xendit and Midtrans) through a RESTful API. The system enables developers to create invoices with line items, manage installment payments with flexible schedules, apply taxes and service fees with proportional distribution, and maintain strict currency isolation (IDR, MYR, USD). The platform treats each installment as an independent gateway payment while internally tracking relationships and schedules. All financial calculations use precise currency-specific decimal handling, with tax rates locked at invoice creation. The architecture follows a modular domain-driven design with payment processing, installment management, tax calculation, and reporting as separate modules communicating via trait-based contracts.

## Technical Context

**Language/Version**: Rust 1.91.0+ (stable as of November 2025) with 2021 edition features  
**Primary Dependencies**: actix-web 4.9+, tokio 1.40+, sqlx 0.8.x, reqwest 0.12.x, rust_decimal 1.36+ (Verified November 2025)  
**Storage**: MySQL 8.0+ with InnoDB storage engine  
**Testing**: cargo test with custom test harnesses for integration tests  
**Target Platform**: Linux server (RESTful API backend)  
**Project Type**: Single backend API project with modular domain structure  
**Performance Goals**: <2s API response time for invoice creation, 100 concurrent requests, 10k invoices/day  
**Constraints**: <200ms p95 latency, 99.5% uptime, accurate to smallest currency unit (1 IDR, 0.01 MYR/USD)  
**Scale/Scope**: Multi-tenant API serving developers, support for 2 payment gateways (Xendit/Midtrans), 3 currencies (IDR/MYR/USD), webhook processing within 5 seconds

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

### I. Standard Library First ✅

- HTTP server, MySQL driver, JSON serialization require external crates (no std library equivalents)
- **Action**: Research minimal dependency set with justification in Phase 0

### II. SOLID Architecture (NON-NEGOTIABLE) ✅

- Modular design specified: payment, installment, tax, reporting modules
- Repository pattern with trait abstractions required
- **Status**: Module boundaries defined in Project Structure section (6 modules: invoices, installments, taxes, transactions, gateways, reports)
- **Action**: Validate trait definitions during Phase 2 implementation

### III. Test-First Development (NON-NEGOTIABLE) ✅

- TDD workflow mandatory: write tests → approve → fail → implement
- Unit tests for business logic, integration tests for database, contract tests for API
- **Status**: Fully compliant - 28 test tasks defined before implementation across all user stories (T031-T038, T059-T065, T080-T087, T106-T110)

### IV. MySQL Integration Standards ✅

- Connection pooling, prepared statements, transactions required
- Migrations with rollback capabilities
- Repository pattern with trait definitions
- **Action**: Phase 0 research MySQL driver options

### V. Environment Management (Laravel-Style) ✅

- .env files with .env.example templates
- Configuration at startup with validation
- **Action**: Phase 1 quickstart includes .env setup

### VI. Context7 MCP Documentation (NON-NEGOTIABLE) ✅

- All dependency research uses Context7 MCP for up-to-date documentation
- **Action**: Use Context7 MCP in Phase 0 research

### VII. Modular Architecture ✅

- Domain modules: invoices, installments, taxes, transactions, reports, gateways
- Each module with models/, services/, repositories/, controllers/
- Trait-based communication, no circular dependencies
- **Status**: Module structure defined in Project Structure section with clear boundaries and trait-based contracts

**Gate Status**: PASS ✅ - Phase 0 research complete (research.md), Phase 1 design complete (data-model.md, contracts/), tasks defined

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── main.rs                          # Application entry point, server initialization
├── config/
│   ├── mod.rs                       # Configuration loading from .env
│   ├── database.rs                  # Database connection pool setup
│   └── server.rs                    # HTTP server configuration
├── core/
│   ├── mod.rs                       # Shared core utilities
│   ├── error.rs                     # Custom error types and Result aliases
│   ├── currency.rs                  # Currency types and decimal handling
│   └── traits/                      # Shared trait definitions
│       ├── mod.rs
│       ├── repository.rs            # Base repository trait
│       └── service.rs               # Base service trait
├── modules/
│   ├── invoices/
│   │   ├── mod.rs                   # Public interface exports
│   │   ├── models/
│   │   │   ├── mod.rs
│   │   │   ├── invoice.rs           # Invoice entity with line items
│   │   │   └── line_item.rs         # Line item entity
│   │   ├── repositories/
│   │   │   ├── mod.rs
│   │   │   └── invoice_repository.rs # Trait + MySQL implementation
│   │   ├── services/
│   │   │   ├── mod.rs
│   │   │   └── invoice_service.rs   # Business logic for invoice CRUD
│   │   └── controllers/
│   │       ├── mod.rs
│   │       └── invoice_controller.rs # HTTP handlers for invoice endpoints
│   ├── installments/
│   │   ├── mod.rs
│   │   ├── models/
│   │   │   ├── mod.rs
│   │   │   └── installment_schedule.rs
│   │   ├── repositories/
│   │   │   ├── mod.rs
│   │   │   └── installment_repository.rs
│   │   ├── services/
│   │   │   ├── mod.rs
│   │   │   ├── installment_calculator.rs # Proportional distribution logic
│   │   │   └── installment_service.rs
│   │   └── controllers/
│   │       ├── mod.rs
│   │       └── installment_controller.rs
│   ├── taxes/
│   │   ├── mod.rs
│   │   ├── models/
│   │   │   ├── mod.rs
│   │   │   └── tax.rs
│   │   ├── repositories/
│   │   │   ├── mod.rs
│   │   │   └── tax_repository.rs
│   │   ├── services/
│   │   │   ├── mod.rs
│   │   │   └── tax_calculator.rs    # Per-line-item tax calculation
│   │   └── controllers/
│   │       ├── mod.rs
│   │       └── tax_controller.rs
│   ├── transactions/
│   │   ├── mod.rs
│   │   ├── models/
│   │   │   ├── mod.rs
│   │   │   └── payment_transaction.rs
│   │   ├── repositories/
│   │   │   ├── mod.rs
│   │   │   └── transaction_repository.rs
│   │   ├── services/
│   │   │   ├── mod.rs
│   │   │   ├── transaction_service.rs
│   │   │   └── webhook_handler.rs    # Webhook processing with retry logic
│   │   └── controllers/
│   │       ├── mod.rs
│   │       ├── transaction_controller.rs
│   │       └── webhook_controller.rs
│   ├── gateways/
│   │   ├── mod.rs
│   │   ├── models/
│   │   │   ├── mod.rs
│   │   │   └── gateway_config.rs
│   │   ├── repositories/
│   │   │   ├── mod.rs
│   │   │   └── gateway_repository.rs
│   │   ├── services/
│   │   │   ├── mod.rs
│   │   │   ├── gateway_trait.rs     # PaymentGateway trait definition
│   │   │   ├── xendit.rs            # Xendit implementation
│   │   │   └── midtrans.rs          # Midtrans implementation
│   │   └── controllers/
│   │       ├── mod.rs
│   │       └── gateway_controller.rs
│   └── reports/
│       ├── mod.rs
│       ├── models/
│       │   ├── mod.rs
│       │   └── financial_report.rs
│       ├── repositories/
│       │   ├── mod.rs
│       │   └── report_repository.rs # Aggregation queries
│       ├── services/
│       │   ├── mod.rs
│       │   └── report_service.rs
│       └── controllers/
│           ├── mod.rs
│           └── report_controller.rs
└── middleware/
    ├── mod.rs
    ├── auth.rs                      # API key authentication
    ├── rate_limit.rs                # 1000 req/min per API key
    └── error_handler.rs             # HTTP error formatting

tests/
├── contract/
│   ├── mod.rs
│   ├── invoice_api_test.rs          # OpenAPI contract validation
│   ├── installment_api_test.rs
│   └── webhook_api_test.rs
├── integration/
│   ├── mod.rs
│   ├── database_setup.rs            # Test database fixtures
│   ├── invoice_flow_test.rs         # Full invoice creation → payment flow
│   ├── installment_flow_test.rs     # Installment payment sequence
│   └── tax_calculation_test.rs      # Tax distribution accuracy
└── unit/
    ├── mod.rs
    ├── installment_calculator_test.rs
    ├── tax_calculator_test.rs
    └── currency_handling_test.rs

migrations/
├── 001_create_invoices_table.sql
├── 002_create_line_items_table.sql
├── 003_create_installment_schedules_table.sql
├── 004_create_payment_transactions_table.sql
├── 005_create_gateway_configs_table.sql
├── 006_create_taxes_table.sql
└── 007_create_reports_table.sql

config/
├── .env.example                     # Template with all variables
└── database.example.toml            # Database connection examples

Cargo.toml                           # Workspace configuration
.env                                 # Local environment (gitignored)
```

**Structure Decision**: Single backend API project with modular domain-driven design. Each business domain (invoices, installments, taxes, transactions, gateways, reports) is a self-contained module under `src/modules/` with its own models, repositories, services, and controllers. Shared code in `src/core/` for cross-cutting concerns (errors, currency handling, shared traits). This structure aligns with Constitution Principle VII (Modular Architecture) and enables independent module development and testing.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No constitutional violations. All complexity is justified:

- **Repository Pattern**: Required by Constitution IV (MySQL Integration Standards) for database abstraction and testing
- **Modular Architecture**: Required by Constitution VII for domain separation and independent deployment
- **External Dependencies**: Required for HTTP server, MySQL driver, JSON - no std library alternatives
