# Tasks: PayTrust Payment Orchestration Platform

**Feature Branch**: `001-payment-orchestration-api`  
**Input**: Design documents from `/specs/001-payment-orchestration-api/`  
**Prerequisites**: plan.md ✓, spec.md ✓, research.md ✓, data-model.md ✓, contracts/ ✓

**Tests**: Included per Constitution Principle III (TDD workflow)

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [x] T001 Initialize Cargo project with workspace structure at `/Users/mrfansi/GitHub/paytrust/Cargo.toml`
- [x] T002 Create project directory structure: `src/`, `src/modules/`, `src/core/`, `tests/`, `migrations/`, `config/`
- [x] T003 [P] Configure Cargo.toml with November 2025 dependencies from research.md (actix-web 4.9, tokio 1.40, sqlx 0.8, reqwest 0.12, rust_decimal 1.36, governor 0.7, argon2 0.5)
- [x] T004 [P] Create .env.example template in `config/.env.example` with MySQL connection, gateway credentials, API secrets
- [x] T005 [P] Setup .gitignore for Rust (.env, target/, Cargo.lock for libraries)
- [x] T006 [P] Configure rustfmt.toml and clippy.toml for code quality standards

**Checkpoint**: ✅ Project structure ready for foundational development

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Database & Configuration

- [x] T007 Create database migration framework setup in `migrations/` directory
- [x] T008 Implement environment configuration loader in `src/config/mod.rs` using dotenvy
- [x] T009 Create database connection pool setup in `src/config/database.rs` using sqlx with MySQL
- [x] T010 Implement server configuration in `src/config/server.rs` (port, host, TLS settings)

### Core Utilities

- [x] T011 [P] Define custom error types in `src/core/error.rs` using thiserror (ValidationError, DatabaseError, GatewayError)
- [x] T012 [P] Implement Currency enum and decimal handling in `src/core/currency.rs` (IDR scale=0, MYR/USD scale=2) using rust_decimal
- [x] T013 [P] Create base repository trait in `src/core/traits/repository.rs` for CRUD operations
- [x] T014 [P] Create base service trait in `src/core/traits/service.rs` for business logic interface
- [x] T015 [P] Implement tracing setup in `src/main.rs` using tracing and tracing-subscriber for structured logging

### Middleware & Security

- [x] T016 Create API key authentication middleware in `src/middleware/auth.rs` with argon2 hashing per research.md
- [x] T017 Create rate limiting middleware in `src/middleware/rate_limit.rs` using governor (1000 req/min per key per FR-040)
- [x] T018 Create error handler middleware in `src/middleware/error_handler.rs` for HTTP error formatting
- [x] T019 Implement CORS middleware configuration in `src/middleware/mod.rs`

### Database Migrations

- [x] T020 Create migration 001: payment_gateways table in `migrations/001_create_payment_gateways_table.sql`
- [x] T021 Create migration 002: api_keys table in `migrations/002_create_api_keys_table.sql`
- [x] T022 Create migration 003: invoices table in `migrations/003_create_invoices_table.sql`
- [x] T023 Create migration 004: line_items table in `migrations/004_create_line_items_table.sql`
- [x] T024 Create migration 005: installment_schedules table in `migrations/005_create_installment_schedules_table.sql`
- [x] T025 Create migration 006: payment_transactions table in `migrations/006_create_payment_transactions_table.sql`
- [x] T026 Create migration 007: indexes and constraints in `migrations/007_add_indexes.sql`

### Gateway Module Foundation

- [x] T027 Define PaymentGateway trait in `src/modules/gateways/services/gateway_trait.rs` with process_payment, verify_webhook methods
- [x] T028 [P] Create PaymentGateway model in `src/modules/gateways/models/gateway_config.rs`
- [x] T029 [P] Implement gateway repository in `src/modules/gateways/repositories/gateway_repository.rs` with MySQL queries

### Application Entry Point

- [x] T030 Implement main.rs application setup: database pool, middleware registration, route mounting, server startup using actix-web and tokio

**Checkpoint**: ✅ Foundation ready - all core utilities, database schema, and middleware are functional. User story implementation can now begin in parallel.

---

## Phase 3: User Story 1 - Basic Invoice Creation and Payment (Priority: P1) 🎯 MVP

**Goal**: Enable developers to create invoices with line items and process payments through Xendit or Midtrans

**Independent Test**: Create an invoice with multiple line items, submit to a gateway, receive payment confirmation and transaction record

### Tests for User Story 1 (TDD Required)

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T031 [P] [US1] Property-based test for line item subtotal calculation in `tests/unit/line_item_calculation_test.rs` using proptest
- [x] T032 [P] [US1] Property-based test for invoice total calculation in `tests/unit/invoice_calculation_test.rs` using proptest
- [ ] T033 [P] [US1] Contract test for POST /invoices endpoint in `tests/contract/invoice_api_test.rs` validating OpenAPI schema
- [ ] T034 [P] [US1] Contract test for GET /invoices/{id} endpoint in `tests/contract/invoice_api_test.rs`
- [ ] T035 [P] [US1] Contract test for GET /invoices endpoint in `tests/contract/invoice_api_test.rs`
- [ ] T036 [P] [US1] Integration test for single payment flow in `tests/integration/invoice_flow_test.rs` (create → gateway payment → webhook → status update)
- [ ] T037 [P] [US1] Integration test for gateway currency validation in `tests/integration/gateway_validation_test.rs` (FR-046)
- [ ] T038 [P] [US1] Integration test for invoice expiration in `tests/integration/invoice_expiration_test.rs` (FR-044, FR-045)

### Implementation for User Story 1

**Invoice Module**

- [x] T039 [P] [US1] Create Invoice model in `src/modules/invoices/models/invoice.rs` with validation (FR-001, FR-004, FR-051)
- [x] T040 [P] [US1] Create LineItem model in `src/modules/invoices/models/line_item.rs` with subtotal calculation (FR-001, FR-005)
- [ ] T041 [US1] Implement InvoiceRepository trait in `src/modules/invoices/repositories/invoice_repository.rs` with MySQL CRUD operations
- [ ] T042 [US1] Implement InvoiceService in `src/modules/invoices/services/invoice_service.rs` with business logic (create, calculate totals, validate gateway, set expiration)
- [ ] T043 [US1] Implement InvoiceController handlers in `src/modules/invoices/controllers/invoice_controller.rs` for POST /invoices, GET /invoices/{id}, GET /invoices
- [ ] T044 [US1] Register invoice routes in `src/modules/invoices/mod.rs` and mount in main.rs

**Gateway Module**

- [ ] T045 [P] [US1] Implement Xendit gateway client in `src/modules/gateways/services/xendit.rs` implementing PaymentGateway trait (create payment, verify webhook)
- [ ] T046 [P] [US1] Implement Midtrans gateway client in `src/modules/gateways/services/midtrans.rs` implementing PaymentGateway trait
- [ ] T047 [US1] Implement GatewayService in `src/modules/gateways/services/gateway_service.rs` for routing payments to correct gateway
- [ ] T048 [US1] Implement GatewayController in `src/modules/gateways/controllers/gateway_controller.rs` for GET /gateways endpoint

**Transaction Module**

- [ ] T049 [P] [US1] Create PaymentTransaction model in `src/modules/transactions/models/payment_transaction.rs` (FR-030, FR-032)
- [ ] T050 [US1] Implement TransactionRepository in `src/modules/transactions/repositories/transaction_repository.rs` with idempotency check
- [ ] T051 [US1] Implement TransactionService in `src/modules/transactions/services/transaction_service.rs` (record payment, update invoice status)
- [ ] T052 [US1] Implement webhook retry logic in `src/modules/transactions/services/webhook_handler.rs` with exponential backoff (FR-042, FR-043)
- [ ] T053 [US1] Implement WebhookController in `src/modules/transactions/controllers/webhook_controller.rs` for POST /webhooks/{gateway} with signature validation (FR-034)
- [ ] T054 [US1] Implement TransactionController in `src/modules/transactions/controllers/transaction_controller.rs` for GET /invoices/{id}/transactions

**Integration & Error Handling**

- [ ] T055 [US1] Implement pessimistic locking for concurrent payment requests (FR-053, FR-054)
- [ ] T056 [US1] Add invoice immutability enforcement when payment initiated (FR-051, FR-052)
- [ ] T057 [US1] Implement gateway failure handling with descriptive errors (FR-038, FR-039)
- [ ] T058 [US1] Add logging for all invoice and payment operations using tracing

**Checkpoint**: At this point, User Story 1 should be fully functional - developers can create invoices, process payments, receive webhooks, and query status. This is MVP ready.

---

## Phase 4: User Story 2 - Additional Charges Management (Priority: P2)

**Goal**: Add service fees and taxes to invoices for accurate financial reporting and compliance

**Independent Test**: Create invoices with tax and service fee configurations, verify total includes charges, generate reports showing breakdown of fees and taxes

### Tests for User Story 2 (TDD Required)

- [ ] T059 [P] [US2] Property-based test for per-line-item tax calculation in `tests/unit/tax_calculator_test.rs` using proptest (FR-057, FR-058)
- [ ] T060 [P] [US2] Property-based test for service fee calculation in `tests/unit/service_fee_test.rs` (percentage + fixed, FR-009, FR-047)
- [ ] T061 [P] [US2] Property-based test for tax-on-subtotal-only calculation in `tests/unit/tax_calculation_test.rs` (FR-055, FR-056)
- [ ] T062 [P] [US2] Contract test for financial report endpoint in `tests/contract/report_api_test.rs` (GET /reports/financial)
- [ ] T063 [P] [US2] Integration test for tax calculation and locking in `tests/integration/tax_calculation_test.rs` (FR-061, FR-062)
- [ ] T064 [P] [US2] Integration test for service fee calculation per gateway in `tests/integration/service_fee_test.rs`
- [ ] T065 [P] [US2] Integration test for financial report generation in `tests/integration/report_generation_test.rs` (FR-063, FR-064)

### Implementation for User Story 2

**Tax Module**

- [ ] T066 [P] [US2] Create Tax model in `src/modules/taxes/models/tax.rs` with rate percentage and category
- [ ] T067 [US2] Implement TaxCalculator in `src/modules/taxes/services/tax_calculator.rs` for per-line-item calculation (FR-057, FR-058)
- [ ] T068 [US2] Implement TaxRepository in `src/modules/taxes/repositories/tax_repository.rs` for aggregation queries
- [ ] T069 [US2] Implement TaxController in `src/modules/taxes/controllers/tax_controller.rs` if needed for tax configuration

**Invoice Module Updates**

- [ ] T070 [US2] Update Invoice model to include tax_total and service_fee fields
- [ ] T071 [US2] Update LineItem model to include tax_rate, tax_category, tax_amount fields
- [ ] T072 [US2] Update InvoiceService to calculate service fees using gateway fee structure (FR-009, FR-047)
- [ ] T073 [US2] Update InvoiceService to calculate total as subtotal + tax_total + service_fee (FR-055, FR-056)
- [ ] T074 [US2] Update InvoiceService to lock tax rates at invoice creation (FR-061, FR-062)
- [ ] T075 [US2] Update InvoiceController to accept tax_rate per line item in POST /invoices

**Reports Module**

- [ ] T076 [P] [US2] Create FinancialReport model in `src/modules/reports/models/financial_report.rs`
- [ ] T077 [US2] Implement ReportRepository in `src/modules/reports/repositories/report_repository.rs` with aggregation queries (SUM, GROUP BY)
- [ ] T078 [US2] Implement ReportService in `src/modules/reports/services/report_service.rs` for service fee and tax breakdown (FR-012, FR-013, FR-063, FR-064)
- [ ] T079 [US2] Implement ReportController in `src/modules/reports/controllers/report_controller.rs` for GET /reports/financial with date range filtering

**Checkpoint**: At this point, User Stories 1 AND 2 work independently - invoices include accurate taxes and fees, financial reports show breakdowns by currency and rate.

---

## Phase 5: User Story 3 - Installment Payment Configuration (Priority: P3)

**Goal**: Enable installment payments with flexible schedules and custom amounts while ensuring totals match invoice amount

**Independent Test**: Create invoice with installment configuration, customize amounts, process each installment payment separately, verify invoice marked complete after all paid

### Tests for User Story 3 (TDD Required)

- [ ] T080 [P] [US3] Property-based test for proportional tax distribution in `tests/unit/installment_calculator_test.rs` (FR-059)
- [ ] T081 [P] [US3] Property-based test for proportional service fee distribution in `tests/unit/installment_calculator_test.rs` (FR-060)
- [ ] T082 [P] [US3] Property-based test for rounding and last installment absorption in `tests/unit/installment_calculator_test.rs` (FR-071, FR-072)
- [ ] T083 [P] [US3] Property-based test for overpayment auto-application in `tests/unit/installment_overpayment_test.rs` (FR-073, FR-074, FR-075, FR-076)
- [ ] T084 [P] [US3] Contract test for GET /invoices/{id}/installments endpoint in `tests/contract/installment_api_test.rs`
- [ ] T085 [P] [US3] Contract test for PATCH /invoices/{id}/installments endpoint in `tests/contract/installment_api_test.rs`
- [ ] T086 [P] [US3] Integration test for sequential installment payment enforcement in `tests/integration/installment_flow_test.rs` (FR-068, FR-069, FR-070)
- [ ] T087 [P] [US3] Integration test for installment adjustment after first payment in `tests/integration/installment_adjustment_test.rs` (FR-077, FR-078, FR-079, FR-080)

### Implementation for User Story 3

**Installment Module**

- [ ] T088 [P] [US3] Create InstallmentSchedule model in `src/modules/installments/models/installment_schedule.rs` with validation (FR-014, FR-017)
- [ ] T089 [US3] Implement InstallmentCalculator in `src/modules/installments/services/installment_calculator.rs` for proportional distribution (FR-059, FR-060, FR-071, FR-072)
- [ ] T090 [US3] Implement InstallmentRepository in `src/modules/installments/repositories/installment_repository.rs` with CRUD and status queries
- [ ] T091 [US3] Implement InstallmentService in `src/modules/installments/services/installment_service.rs` for schedule management (create, adjust, validate sequence)
- [ ] T092 [US3] Implement InstallmentController in `src/modules/installments/controllers/installment_controller.rs` for GET and PATCH /invoices/{id}/installments

**Invoice Module Updates**

- [ ] T093 [US3] Update InvoiceService to support installment configuration at creation (count, custom_amounts)
- [ ] T094 [US3] Update InvoiceService to handle partially_paid status when first installment paid (FR-019)
- [ ] T095 [US3] Update InvoiceService to mark fully_paid when all installments complete (FR-020)

**Gateway Integration**

- [ ] T096 [US3] Update GatewayService to create separate payment transactions per installment (FR-065, FR-066, FR-067)
- [ ] T097 [US3] Update Xendit client to generate installment-specific payment URLs
- [ ] T098 [US3] Update Midtrans client to generate installment-specific payment URLs

**Transaction Module Updates**

- [ ] T099 [US3] Update TransactionService to handle installment payments with sequential enforcement (FR-068, FR-069, FR-070)
- [ ] T100 [US3] Update TransactionService to handle overpayment auto-application (FR-073, FR-074, FR-075, FR-076)
- [ ] T101 [US3] Update WebhookController to update installment status and apply excess payment
- [ ] T102 [US3] Update TransactionRepository to link transactions to specific installments

**Supplementary Invoice Support**

- [ ] T103 [US3] Update Invoice model to support original_invoice_id reference (FR-082)
- [ ] T104 [US3] Update InvoiceService to create supplementary invoices (FR-081, FR-082)
- [ ] T105 [US3] Update InvoiceController to provide supplementary invoice creation endpoint

**Checkpoint**: All user stories 1, 2, and 3 work independently - installment payments function with flexible schedules, proportional distribution, and sequential enforcement.

---

## Phase 6: User Story 4 - Multi-Currency Payment Isolation (Priority: P4)

**Goal**: Process transactions in multiple currencies (IDR, MYR, USD) with proper isolation to prevent currency mismatch errors

**Independent Test**: Create invoices in different currencies simultaneously, process payments, verify calculations/reports never mix currencies and maintain separate totals per currency

### Tests for User Story 4 (TDD Required)

- [ ] T106 [P] [US4] Property-based test for currency-specific decimal handling in `tests/unit/currency_handling_test.rs` (IDR scale=0, MYR/USD scale=2)
- [ ] T107 [P] [US4] Property-based test for currency isolation in calculations in `tests/unit/currency_isolation_test.rs` (FR-023, FR-024)
- [ ] T108 [P] [US4] Contract test for multi-currency invoice creation in `tests/contract/currency_api_test.rs`
- [ ] T109 [P] [US4] Integration test for currency mismatch rejection in `tests/integration/currency_validation_test.rs` (FR-024)
- [ ] T110 [P] [US4] Integration test for multi-currency financial reports in `tests/integration/multi_currency_report_test.rs` (FR-025)

### Implementation for User Story 4

**Currency Module Enhancement**

- [ ] T111 [US4] Enhance Currency enum in `src/core/currency.rs` with decimal precision methods (IDR scale=0, MYR/USD scale=2) per FR-026
- [ ] T112 [US4] Add currency validation helpers for arithmetic operations (prevent mixing)
- [ ] T113 [US4] Add currency-specific rounding functions for installment calculations

**Invoice Module Updates**

- [ ] T114 [US4] Update Invoice model validation to enforce single currency per invoice (FR-023)
- [ ] T115 [US4] Update InvoiceService to apply currency-specific decimal handling in calculations
- [ ] T116 [US4] Add currency mismatch validation in payment processing (FR-024)

**Transaction Module Updates**

- [ ] T117 [US4] Update TransactionRepository to reject payments in different currency than invoice (FR-024)
- [ ] T118 [US4] Update TransactionService to validate currency consistency before processing

**Gateway Module Updates**

- [ ] T119 [US4] Update GatewayService to validate gateway supports invoice currency (FR-046)
- [ ] T120 [US4] Update gateway configuration to track supported_currencies per gateway

**Reports Module Updates**

- [ ] T121 [US4] Update ReportService to separate totals by currency (FR-025)
- [ ] T122 [US4] Update ReportRepository aggregation queries to GROUP BY currency
- [ ] T123 [US4] Update ReportController to return separate currency sections in response (no conversion, FR-063)

**Installment Module Updates**

- [ ] T124 [US4] Update InstallmentCalculator to use currency-specific rounding (IDR whole numbers)
- [ ] T125 [US4] Update InstallmentCalculator to handle last installment absorption with currency precision

**Checkpoint**: All 4 user stories work independently - multi-currency support is complete with strict isolation, no mixing, and accurate currency-specific calculations.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

### Documentation

- [ ] T126 [P] Create API usage examples in `docs/examples/` for each user story
- [ ] T127 [P] Create developer quickstart guide in `docs/quickstart.md` using specs/001-payment-orchestration-api/quickstart.md as reference
- [ ] T128 [P] Generate OpenAPI documentation endpoint in actix-web serving `contracts/openapi.yaml`
- [ ] T128b [P] Validate OpenAPI 3.0 schema compliance using validator or contract testing framework
- [ ] T129 [P] Create deployment guide in `docs/deployment.md` with MySQL setup, environment variables, TLS configuration

### Code Quality

- [ ] T130 Run cargo fmt across all source files
- [ ] T131 Run cargo clippy and fix all warnings
- [ ] T132 Add comprehensive inline documentation (/// doc comments) for all public APIs
- [ ] T133 Review and refactor duplicate code across modules

### Security & Performance

- [ ] T134 Security audit: validate all input sanitization and SQL injection prevention (sqlx compile-time checks)
- [ ] T135 Performance optimization: add database indexes per data-model.md (already in migration 007)
- [ ] T136 Performance testing: verify <2s response time for invoice creation (NFR-001)
- [ ] T137 Load testing: verify 100 concurrent requests handling (NFR-002)
- [ ] T138 Implement graceful shutdown handling in main.rs

### Monitoring & Observability

- [ ] T139 Add structured logging for all API endpoints with request IDs
- [ ] T140 Add metrics collection for response times, error rates, gateway success rates
- [ ] T141 Add health check endpoint GET /health with database connectivity check
- [ ] T142 Add readiness probe endpoint GET /ready

### Additional Testing (Optional)

- [ ] T143 [P] Add unit tests for all business logic services
- [ ] T144 [P] Add contract tests for all error response scenarios
- [ ] T145 [P] Add integration tests for concurrent request handling (FR-053, FR-054)
- [ ] T146 [P] Add integration tests for rate limiting enforcement (FR-040, FR-041)

### Validation & Deployment Prep

- [ ] T147 Run full quickstart.md validation from specs/001-payment-orchestration-api/quickstart.md
- [ ] T148 Run all tests: `cargo test` (unit + integration + contract)
- [ ] T149 Build production binary: `cargo build --release`
- [ ] T150 Create Docker configuration if needed for deployment

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational completion - No dependencies on other stories
- **User Story 2 (Phase 4)**: Depends on Foundational completion - Extends US1 but independently testable
- **User Story 3 (Phase 5)**: Depends on Foundational completion - Extends US1/US2 but independently testable
- **User Story 4 (Phase 6)**: Depends on Foundational completion - Enhances all stories with currency isolation
- **Polish (Phase 7)**: Depends on all desired user stories being complete

### User Story Priority Execution

- **P1 (User Story 1)**: MVP - Must complete first, independently functional
- **P2 (User Story 2)**: Can start after Foundational, adds financial reporting
- **P3 (User Story 3)**: Can start after Foundational, adds installment flexibility
- **P4 (User Story 4)**: Can start after Foundational, adds multi-currency support

### Within Each User Story

1. Tests MUST be written FIRST and FAIL before implementation (TDD workflow per Constitution Principle III)
2. Models before services
3. Services before controllers
4. Core implementation before integration
5. Story complete before moving to next priority

### Parallel Opportunities

**Setup (Phase 1)**:

- T003, T004, T005, T006 can run in parallel (different files)

**Foundational (Phase 2)**:

- T011, T012, T013, T014, T015 can run in parallel (core utilities)
- T020-T026 must run sequentially (database migrations have dependencies)

**User Story 1 Tests**:

- T031, T032, T033, T034, T035, T036, T037, T038 can run in parallel (all test files)

**User Story 1 Models**:

- T039, T040 can run in parallel (Invoice and LineItem models)
- T045, T046 can run in parallel (Xendit and Midtrans gateways)
- T049 can run in parallel (PaymentTransaction model)

**User Story 2 Tests**:

- T059, T060, T061, T062, T063, T064, T065 can run in parallel

**User Story 3 Tests**:

- T080, T081, T082, T083, T084, T085, T086, T087 can run in parallel

**User Story 4 Tests**:

- T106, T107, T108, T109, T110 can run in parallel

**Polish Phase**:

- T126, T127, T128, T129 can run in parallel (documentation)
- T143, T144, T145, T146 can run in parallel (additional tests)

### Parallel Execution Example: User Story 1

```bash
# Phase 3.1: All tests for User Story 1 (T031-T038) in parallel:
cargo test --test line_item_calculation_test &
cargo test --test invoice_calculation_test &
cargo test --test invoice_api_test &
cargo test --test invoice_flow_test &
cargo test --test gateway_validation_test &
cargo test --test invoice_expiration_test &
wait

# Phase 3.2: All models in parallel (T039, T040, T045, T046, T049):
# Work on Invoice model in src/modules/invoices/models/invoice.rs &
# Work on LineItem model in src/modules/invoices/models/line_item.rs &
# Work on Xendit client in src/modules/gateways/services/xendit.rs &
# Work on Midtrans client in src/modules/gateways/services/midtrans.rs &
# Work on PaymentTransaction model in src/modules/transactions/models/payment_transaction.rs &
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T006)
2. Complete Phase 2: Foundational (T007-T030) - CRITICAL checkpoint
3. Complete Phase 3: User Story 1 (T031-T058)
4. **STOP and VALIDATE**: Run all US1 tests, verify invoice creation and payment flow works end-to-end
5. Deploy/demo if ready - this is a functional payment platform

### Incremental Delivery (Recommended)

1. Setup + Foundational → Foundation ready (checkpoint)
2. Add User Story 1 → Test independently → **Deploy/Demo (MVP!)**
3. Add User Story 2 → Test independently → Deploy/Demo (now with financial reports)
4. Add User Story 3 → Test independently → Deploy/Demo (now with installments)
5. Add User Story 4 → Test independently → Deploy/Demo (now with multi-currency)
6. Polish → Final production-ready release

### Parallel Team Strategy

With multiple developers after Foundational phase completes:

- **Developer A**: User Story 1 (T031-T058) - Core payment processing
- **Developer B**: User Story 2 (T059-T079) - Financial reporting
- **Developer C**: User Story 3 (T080-T105) - Installment payments
- **Developer D**: User Story 4 (T106-T125) - Multi-currency support

Each story can progress independently, then integrate at the end.

---

## Task Summary

**Total Tasks**: 151  
**Setup**: 6 tasks  
**Foundational**: 24 tasks (BLOCKING)  
**User Story 1 (P1 - MVP)**: 28 tasks (8 tests + 20 implementation)  
**User Story 2 (P2)**: 21 tasks (7 tests + 14 implementation)  
**User Story 3 (P3)**: 26 tasks (8 tests + 18 implementation)  
**User Story 4 (P4)**: 20 tasks (5 tests + 15 implementation)  
**Polish**: 26 tasks

**Parallel Opportunities**: ~60 tasks can run in parallel (marked with [P])

**MVP Scope** (User Story 1 only): 58 tasks (Setup + Foundational + US1)

**Estimated Effort**:

- MVP (US1): ~4-6 weeks for 1 developer
- Full Feature (US1-US4): ~10-12 weeks for 1 developer
- Full Feature (US1-US4): ~6-8 weeks with 2 developers (parallel stories after foundation)

---

## Notes

- **[P] tasks** = different files, no dependencies, can run in parallel
- **[Story] label** (US1, US2, US3, US4) maps task to specific user story for traceability
- **TDD workflow**: Write test → Ensure it fails → Implement → Test passes → Commit
- Each user story should be independently completable and testable
- Stop at any checkpoint to validate story independently
- Constitution Principle III enforced: Tests before implementation
- All dependencies verified against November 2025 versions per research.md
- Financial calculations use rust_decimal with arbitrary precision per research.md
- Property-based tests use proptest for financial calculation validation per research.md
