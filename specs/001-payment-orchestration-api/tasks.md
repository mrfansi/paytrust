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

- [X] T001 Initialize Cargo project with workspace structure at `/Users/mrfansi/GitHub/paytrust/Cargo.toml`
- [X] T002 Create project directory structure: `src/`, `src/modules/`, `src/core/`, `tests/`, `migrations/`, `config/`
- [X] T003 [P] Configure Cargo.toml with November 2025 dependencies from research.md (actix-web 4.9, tokio 1.40, sqlx 0.8, reqwest 0.12, rust_decimal 1.36, governor 0.7, argon2 0.5)
- [X] T004 [P] Create .env.example template in `config/.env.example` with MySQL connection, gateway credentials, API secrets
- [X] T005 [P] Setup .gitignore for Rust (.env, target/, Cargo.lock for libraries)
- [X] T006 [P] Configure rustfmt.toml and clippy.toml for code quality standards

**Checkpoint**: ✅ Project structure ready for foundational development

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Database & Configuration

- [ ] T007 Create database migration framework setup in `migrations/` directory
- [ ] T008 Implement environment configuration loader in `src/config/mod.rs` using dotenvy
- [ ] T009 Create database connection pool setup in `src/config/database.rs` using sqlx with MySQL
- [ ] T010 Implement server configuration in `src/config/server.rs` (port, host, TLS settings)

### Core Utilities

- [ ] T011 [P] Define custom error types in `src/core/error.rs` using thiserror (ValidationError, DatabaseError, GatewayError)
- [ ] T011a [P] Implement timezone utilities in `src/core/timezone.rs` for UTC storage and gateway-specific timezone conversion (Xendit: UTC, Midtrans: Asia/Jakarta UTC+7) per FR-087. All timestamps stored internally as UTC, converted to gateway timezone for API calls, returned as ISO 8601 UTC in API responses
- [ ] T011b [P] Unit test for timezone conversion utilities in `tests/unit/timezone_test.rs` (verify UTC ↔ Asia/Jakarta conversion accuracy, verify UTC passthrough for Xendit, verify ISO 8601 formatting)
- [ ] T012 [P] Implement Currency enum and decimal handling in `src/core/currency.rs` (IDR scale=0, MYR/USD scale=2) using rust_decimal
- [ ] T013 [P] Create base repository trait in `src/core/traits/repository.rs` for CRUD operations
- [ ] T014 [P] Create base service trait in `src/core/traits/service.rs` for business logic interface
- [ ] T015 [P] Implement tracing setup in `src/main.rs` using tracing and tracing-subscriber for structured logging

### Middleware & Security

- [ ] T016 Create API key authentication middleware in `src/middleware/auth.rs` with argon2 hashing per research.md and tenant_id extraction from authenticated API key for multi-tenant isolation per FR-088
- [ ] T016d Implement tenant isolation enforcement in all repository methods per FR-088 - add tenant_id filter to all SELECT/UPDATE/DELETE queries for: InvoiceRepository, LineItemRepository, InstallmentRepository, TransactionRepository, ReportRepository (including all aggregation queries in ReportRepository per G2 finding). Validate tenant_id matches authenticated user on all write operations. Add integration test in `tests/integration/tenant_isolation_test.rs` to verify cross-tenant data access prevention including financial report aggregations
- [ ] T018 Create error handler middleware in `src/middleware/error_handler.rs` for HTTP error formatting
- [ ] T019 Implement CORS middleware configuration in `src/middleware/mod.rs`

### Database Migrations

- [ ] T020 Create migration 001: gateway_configs table in `migrations/001_create_gateway_configs_table.sql` with schema: id (BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY per FR-007a), name VARCHAR(50) NOT NULL, supported_currencies JSON NOT NULL (array of currency codes), fee_percentage DECIMAL(5,4) NOT NULL (e.g., 0.0290 for 2.9%), fee_fixed DECIMAL(10,2) NOT NULL, region VARCHAR(50), webhook_url VARCHAR(255), api_key_encrypted TEXT, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
- [ ] T021 Create migration 002: api_keys table in `migrations/002_create_api_keys_table.sql`
- [ ] T022 Create migration 003: invoices table in `migrations/003_create_invoices_table.sql` (include payment_initiated_at TIMESTAMP NULL DEFAULT NULL for immutability tracking per FR-051, and original_invoice_id BIGINT UNSIGNED NULL with FOREIGN KEY to invoices(id) for supplementary invoice relationship per FR-082)
- [ ] T023 Create migration 004: line_items table in `migrations/004_create_line_items_table.sql`
- [ ] T024 Create migration 005: installment_schedules table in `migrations/005_create_installment_schedules_table.sql`
- [ ] T025 Create migration 006: payment_transactions table in `migrations/006_create_payment_transactions_table.sql` (include overpayment_amount DECIMAL(19,4) NULL column for tracking excess payments per FR-073)
- [ ] T026 Create migration 007: indexes and constraints in `migrations/007_add_indexes.sql`
- [ ] T026a Create migration 008: webhook_retry_log table in `migrations/008_create_webhook_retry_log.sql` for audit trail per FR-042 Audit Logging section (columns: id, webhook_id, attempt_number, attempted_at TIMESTAMP, status, error_message)

### Gateway Module Foundation

- [ ] T027 Define PaymentGateway trait in `src/modules/gateways/services/gateway_trait.rs` with process_payment, verify_webhook methods
- [ ] T027a [P] [FOUNDATION] Create RateLimiter trait definition in `contracts/rate_limiter_trait.rs` with rate_limit() method signature per FR-040. Trait methods: async fn check_rate_limit(&self, api_key: &str) -> Result<(), RateLimitError> and async fn record_request(&self, api_key: &str) -> Result<(), RateLimitError>. This trait enables pluggable rate limiting backends (InMemoryRateLimiter for v1.0, RedisRateLimiter for future multi-instance deployment)
- [ ] T017a [P] [FOUNDATION] Integration test for rate limiting in `tests/integration/rate_limit_test.rs` (verify 1000 req/min limit per API key, verify 429 response with Retry-After header when exceeded per FR-040, FR-041) - depends on T027a RateLimiter trait definition
- [ ] T017 Create rate limiting middleware in `src/middleware/rate_limit.rs` implementing RateLimiter trait (see contracts/rate_limiter_trait.rs created in T027a) - depends on T017a passing and T027a trait definition. v1.0 uses InMemoryRateLimiter with governor crate (1000 req/min per key per FR-040). Return 429 Too Many Requests with Retry-After header when limit exceeded per FR-041. Architecture: Trait-based design enables future RedisRateLimiter for multi-instance deployment without modifying middleware code (Constitution Principle II - Open/Closed compliance)
- [ ] T028 [P] Create PaymentGateway model in `src/modules/gateways/models/gateway_config.rs`
- [ ] T029 [P] Implement gateway repository in `src/modules/gateways/repositories/gateway_repository.rs` with MySQL queries

### Test Infrastructure (Constitution Principle III - Real Testing)

- [ ] T029a **[CONSTITUTION CRITICAL]** Create test database configuration in `tests/integration/database_setup.rs` - **Mocks/stubs PROHIBITED per Constitution Principle III and NFR-008**. MUST use real MySQL test database instances with connection pool setup (min 5 connections, max 20 connections), transaction isolation level READ COMMITTED, migration runner (executes same migrations T020-T026a as production for schema parity), test fixtures, and cleanup utilities. Cleanup strategy: TRUNCATE tables between tests for data isolation, DROP/CREATE database for schema migration tests. Test database uses identical schema to production. Real testing requirement is NON-NEGOTIABLE for production validation
- [ ] T029b **[CONSTITUTION GATE]** Validate Constitution III compliance during code review for each integration test PR. This is a CONTINUOUS validation gate, not a post-implementation audit. For each integration test PR, reviewer MUST verify: (1) Test uses real MySQL connections from T029a database setup, (2) NO mock library imports: grep for `use mockall`, `use mockito`, `mock::` in test file, (3) Test executes actual SQL queries against test database (not in-memory simulations), (4) Webhook tests make real HTTP calls (not mocked HTTP clients), (5) Any exceptions documented with justification. PR merge BLOCKED until all 5 checks pass. This gate enforces TDD: tests written → approved → fail → then implement
- [ ] T029c **[CONSTITUTION CRITICAL]** Add CI validation check for Constitution III compliance - create `.github/workflows/constitution-check.yml` with job that fails if mock libraries detected in integration tests: `grep -rn "use mockall\|use mockito\|mock::" tests/integration/ && echo "ERROR: Mocks prohibited in integration tests per Constitution III" && exit 1`. This automated check prevents constitution violations from merging

### Application Entry Point

- [ ] T030 Implement main.rs application setup: database pool, middleware registration, route mounting (order: health, auth middleware, invoices, installments, transactions, webhooks, reports), server startup using actix-web and tokio

**Checkpoint**: ✅ Foundation ready - all core utilities, database schema, middleware, and test infrastructure are functional. **Constitution III Compliance MUST BE VALIDATED**: Execute T029a (test database setup), T029b (code review gate), and T029c (CI validation) and verify all pass before proceeding. Validation criteria: (1) Real MySQL test database operational with connection pool, (2) CI mock detection active and passing, (3) Test database migrations match production schema. **BLOCK all Phase 3+ work until this checkpoint passes**. User story implementation can begin in parallel only after validation complete.

---

## Phase 3: User Story 1 - Basic Invoice Creation and Payment (Priority: P1) 🎯 MVP

**Goal**: Enable developers to create invoices with line items and process payments through Xendit or Midtrans

**Independent Test**: Create an invoice with multiple line items, submit to a gateway, receive payment confirmation and transaction record

### Tests for User Story 1 (TDD Required)

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T031 [P] [US1] Property-based test for line item subtotal calculation in `tests/unit/line_item_calculation_test.rs` using proptest
- [ ] T032 [P] [US1] Property-based test for invoice total calculation in `tests/unit/invoice_calculation_test.rs` using proptest
- [ ] T033 [P] [US1] Contract tests for invoice API endpoints in `tests/contract/invoice_api_test.rs` validating OpenAPI schema: POST /invoices, GET /invoices/{id}, GET /invoices (7 tests total covering all three endpoints)
- [ ] T036 [P] [US1] Integration test for payment flow in `tests/integration/payment_flow_test.rs` (4 tests: single payment, idempotency, partial payment, concurrency)
- [ ] T037 [P] [US1] Integration test for gateway currency validation in `tests/integration/gateway_validation_test.rs` (2 tests + 3 ignored DB tests)
- [ ] T038 [P] [US1] Integration test for invoice expiration in `tests/integration/invoice_expiration_test.rs` (3 tests + 3 ignored DB tests)
- [ ] T038a [P] [US1] Integration test for expires_at parameter validation in `tests/integration/invoice_expiration_test.rs` - verify all 4 validations from FR-044a: (a) max 30 days from creation, (b) min 1 hour from creation, (c) reject past dates with 400 "Expiration time cannot be in the past", (d) if invoice has installments expires_at >= last installment due_date with 400 "Invoice expiration cannot occur before final installment due date"
- [ ] T038b [P] [US1] Integration test for payment_initiated_at timestamp in `tests/integration/payment_initiation_test.rs` (verify timestamp set on first payment attempt, verify immutability enforcement when timestamp NOT NULL per FR-051(a))
- [ ] T038c [P] [US1] Integration test for refund webhook processing in `tests/integration/refund_webhook_test.rs` (verify Xendit invoice.refunded event updates transaction status, verify Midtrans refund notification updates records, verify GET /invoices/{id}/refunds returns refund history per FR-086)
- [ ] T044b [P] [US1] Integration test for gateway currency validation in `tests/integration/gateway_currency_validation_test.rs` (verify gateway supports invoice currency per FR-046, verify 400 Bad Request when gateway does not support currency, test all 3 currencies: IDR/MYR/USD)

### Implementation for User Story 1

**Invoice Module**

- [ ] T039 [P] [US1] Create Invoice model in `src/modules/invoices/models/invoice.rs` with validation (FR-001, FR-004, FR-051)
- [ ] T040 [P] [US1] Create LineItem model in `src/modules/invoices/models/line_item.rs` with subtotal calculation (FR-001, FR-005)
- [ ] T041 [US1] Implement InvoiceRepository trait in `src/modules/invoices/repositories/invoice_repository.rs` with MySQL CRUD operations (✅ Converted to runtime queries)
- [ ] T042 [US1] Implement InvoiceService in `src/modules/invoices/services/invoice_service.rs` with business logic (create, calculate totals, validate gateway_id parameter per FR-007)
- [ ] T042a [US1] Implement expires_at validation logic in InvoiceService per FR-044a: (a) parse ISO 8601 timestamp from request, (b) validate not in past (reject 400 "Expiration time cannot be in the past"), (c) validate >= created_at + 1 hour (reject 400 "Expiration must be at least 1 hour from now"), (d) validate <= created_at + 30 days (reject 400 "Expiration must be within 30 days from now"), (e) if invoice has installments: validate expires_at >= last_installment.due_date (reject 400 "Invoice expiration cannot occur before final installment due date {due_date}"). Default to created_at + 24 hours if expires_at not provided per FR-044
- [ ] T042b [US1] Implement payment_initiated_at timestamp logic in InvoiceService per FR-051(a): Set payment_initiated_at (TIMESTAMP NULL) on Invoice entity when first payment attempt occurs, defined as: (a) gateway payment URL generation (when calling gateway API to create payment), OR (b) payment_transaction record creation (when recording payment attempt), whichever occurs first. Once payment_initiated_at IS NOT NULL, enforce invoice immutability per FR-051 by rejecting modification requests with 400 Bad Request. Timestamp is write-once (never updated after initial set). Use UTC timezone per FR-087
- [ ] T043 [US1] Implement InvoiceController handlers in `src/modules/invoices/controllers/invoice_controller.rs` for POST /invoices, GET /invoices/{id}, GET /invoices
- [ ] T044 [US1] Register invoice routes in `src/modules/invoices/mod.rs` and mount in main.rs
- [ ] T044a [US1] Validate gateway supports invoice currency in InvoiceService before invoice creation per FR-046 (check gateway_configs.supported_currencies)

**Gateway Module**

- [ ] T045 [P] [US1] Implement Xendit gateway client in `src/modules/gateways/services/xendit.rs` implementing PaymentGateway trait (create payment, verify webhook)
- [ ] T046 [P] [US1] Implement Midtrans gateway client in `src/modules/gateways/services/midtrans.rs` implementing PaymentGateway trait
- [ ] T047 [US1] Implement GatewayService in `src/modules/gateways/services/gateway_service.rs` for routing payments to correct gateway
- [ ] T048 [US1] Implement GatewayController in `src/modules/gateways/controllers/gateway_controller.rs` for GET /gateways endpoint

**Transaction Module**

- [ ] T049 [P] [US1] Create PaymentTransaction model in `src/modules/transactions/models/payment_transaction.rs` (FR-030, FR-032)
- [ ] T050 [US1] Implement TransactionRepository in `src/modules/transactions/repositories/transaction_repository.rs` with idempotency check
- [ ] T051 [US1] Implement TransactionService in `src/modules/transactions/services/transaction_service.rs` (record payment, update invoice status)
- [ ] T052 [US1] Implement webhook retry logic in `src/modules/transactions/services/webhook_handler.rs` with cumulative delay retries from initial failure (T=0): retry 1 at T+1 minute (1 min after initial failure), retry 2 at T+6 minutes (6 min after initial failure, 5 min after retry 1), retry 3 at T+36 minutes (36 min after initial failure, 30 min after retry 2) per FR-042. Retry ONLY for 5xx errors and connection timeouts >10s. 4xx errors (including signature verification failures) marked permanently failed immediately without retry. Retry timers are in-memory only and do NOT persist across application restarts per FR-042. After all 3 retries fail: mark webhook permanently failed, log error with CRITICAL level. Log all retry attempts with timestamps, attempt number, final status to webhook_retry_log table per FR-042 Audit Logging section
- [ ] T052a [US1] Performance test for webhook retry queue capacity in `tests/integration/webhook_queue_capacity_test.rs` - depends on T052 webhook handler implementation (verify queue handles 10,000 pending retries per NFR-010, verify <100ms queue operation latency at 10k queue depth, test enqueue/dequeue operations under load)
- [ ] T053 [US1] Implement WebhookController in `src/modules/transactions/controllers/webhook_controller.rs` for POST /webhooks/{gateway} with signature validation (FR-034) AND GET /webhooks/failed endpoint to query permanently failed webhooks for manual intervention per FR-042
- [ ] T053a [US1] Implement refund webhook handlers in WebhookController for processing refund events per FR-086: (a) Xendit invoice.refunded event handler, (b) Midtrans refund notification handler, (c) update payment_transactions table with refund information (refund_id, refund_amount, refund_timestamp, refund_reason), (d) update transaction status to reflect refund, (e) store refund records for GET /invoices/{id}/refunds endpoint query
- [ ] T054 [US1] Implement TransactionController in `src/modules/transactions/controllers/transaction_controller.rs` for GET /invoices/{id}/transactions
- [ ] T054b [US1] Implement payment discrepancy endpoint in TransactionController for GET /invoices/{id}/discrepancies (FR-050)
- [ ] T054c [US1] Implement overpayment query endpoint in TransactionController for GET /invoices/{id}/overpayment returning {invoice_id, total_amount, total_paid, overpayment_amount} per FR-076
- [ ] T054d [US1] Implement refund history endpoint in TransactionController for GET /invoices/{id}/refunds returning refund records (refund_id, refund_amount, refund_timestamp, refund_reason) per FR-086

**Integration & Error Handling**

- [ ] T055 [US1] Implement pessimistic locking for concurrent payment requests using MySQL SELECT FOR UPDATE (FR-053, FR-054)
- [ ] T056 [US1] Add invoice immutability enforcement when payment initiated (FR-051, FR-052)
- [ ] T057 [US1] Implement gateway failure handling with descriptive errors (FR-038, FR-039)
- [ ] T058 [US1] Add logging for all invoice and payment operations using tracing
- [ ] T058a [US1] Implement invoice expiration background job in `src/modules/invoices/services/expiration_checker.rs` per FR-045 - runs every 5 minutes using tokio interval timer, queries invoices with expires_at < current_time AND status IN ('draft', 'pending', 'partially_paid'), updates status to 'expired', logs expiration events. Background task spawned in main.rs during server startup

**Checkpoint**: At this point, User Story 1 should be fully functional - developers can create invoices, process payments, receive webhooks, and query status. This is MVP ready.

---

## Phase 4: User Story 2 - Additional Charges Management (Priority: P2)

**Goal**: Add service fees and taxes to invoices for accurate financial reporting and compliance

**Independent Test**: Create invoices with tax and service fee configurations, verify total includes charges, generate reports showing breakdown of fees and taxes

### Tests for User Story 2 (TDD Required)

- [ ] T059 [P] [US2] Property-based test for per-line-item tax calculation in `tests/unit/tax_calculator_test.rs` using proptest (FR-057, FR-058)
- [ ] T059a [P] [US2] Unit test for tax_rate validation in `tests/unit/tax_validation_test.rs` per FR-064a (verify tax_rate >= 0 and <= 1.0, verify max 4 decimal places, verify 400 Bad Request for invalid rates, test edge cases: 0.0, 1.0, 0.0001, 0.27, 1.0001 rejection)
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
- [ ] T075a [US2] Implement tax_rate validation in InvoiceService per FR-064a: (a) validate tax_rate >= 0 and <= 1.0 (0-100%), (b) validate tax_rate has maximum 4 decimal places (0.0001 precision), (c) reject invalid rates with 400 Bad Request "Invalid tax_rate: must be between 0 and 1.0 with max 4 decimal places", (d) log rates exceeding 0.27 (27%) for audit review as potential data entry errors

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
- [ ] T087a [P] [US3] Integration test for installment status transitions in `tests/integration/installment_status_test.rs` (verify invoice status transitions: draft→pending→partially_paid→fully_paid at each installment payment per FR-019, FR-020)

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

- [ ] T096 [US3] Update GatewayService to create separate payment transactions per installment (FR-065) and generate unique payment URLs per installment using gateway-specific APIs with installment number and invoice reference in metadata (FR-066), map each payment_transaction to installment_schedule via installment_id foreign key (FR-067)
- [ ] T097 [US3] Update Xendit client to generate installment-specific payment URLs
- [ ] T098 [US3] Update Midtrans client to generate installment-specific payment URLs

**Transaction Module Updates**

- [ ] T099 [US3] Update TransactionService to handle installment payments with sequential enforcement (FR-068, FR-069, FR-070)
- [ ] T100 [US3] Update TransactionService to handle overpayment auto-application (FR-073, FR-074, FR-075, FR-076)
- [ ] T101 [US3] Update WebhookController to update installment status and apply excess payment
- [ ] T102 [US3] Update TransactionRepository to link transactions to specific installments

**Checkpoint**: ✅ All user stories 1, 2, and 3 work independently - installment payments function with flexible schedules, proportional distribution, and sequential enforcement.

---

## Phase 6: User Story 4 - Multi-Currency Payment Isolation (Priority: P4)

**Goal**: Process transactions in multiple currencies (IDR, MYR, USD) with proper isolation to prevent currency mismatch errors

**Independent Test**: Create invoices in different currencies simultaneously, process payments, verify calculations/reports never mix currencies and maintain separate totals per currency

### Tests for User Story 4 (TDD Required)

- [ ] T106 [P] [US4] Property-based test for currency-specific decimal handling in `tests/unit/currency_handling_test.rs` (IDR scale=0, MYR/USD scale=2) - Covered by src/core/currency.rs unit tests
- [ ] T107 [P] [US4] Property-based test for currency isolation in calculations in `tests/unit/currency_isolation_test.rs` (FR-023, FR-024) - Deferred
- [ ] T108 [P] [US4] Contract test for multi-currency invoice creation in `tests/contract/currency_api_test.rs` - Deferred
- [ ] T109 [P] [US4] Integration test for currency mismatch rejection in `tests/integration/currency_validation_test.rs` (FR-024) - Deferred
- [ ] T110 [P] [US4] Integration test for multi-currency financial reports in `tests/integration/multi_currency_report_test.rs` (FR-025) - Deferred
- [ ] T110a [P] [US4] Integration test for currency non-conversion in reports in `tests/integration/multi_currency_report_test.rs` - verify reports return separate currency totals without conversion, validate no conversion calculations performed (FR-063, FR-025)
- [ ] T110b [P] [US4] Unit test for currency conversion prohibition in `tests/unit/currency_isolation_test.rs` - verify ReportService never performs currency conversion calculations

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

- [ ] T120 [US4] Update gateway configuration to track supported_currencies per gateway (validation logic implemented in T044a)

**Reports Module Updates**

- [ ] T121 [US4] Update ReportService to separate totals by currency (FR-025)
- [ ] T122 [US4] Update ReportRepository aggregation queries to GROUP BY currency
- [ ] T123 [US4] Update ReportController to return separate currency sections in response (no conversion, FR-063)

**Installment Module Updates**

- [ ] T124 [US4] Update InstallmentCalculator to use currency-specific rounding (IDR whole numbers)
- [ ] T125 [US4] Update InstallmentCalculator to handle last installment absorption with currency precision

**Checkpoint**: All 4 user stories work independently - multi-currency support is complete with strict isolation, no mixing, and accurate currency-specific calculations.

---

## Phase 6.5: User Story 5 - API Key Management and Invoice Extensions (Priority: P2)

**Goal**: Enable secure API key lifecycle management (generation, rotation, revocation) and support supplementary invoices for adding items to orders with active payments

**Independent Test**: Generate API keys, rotate them, revoke them, verify authentication. Create invoice with active payment, add supplementary invoice with new items, verify separate payment schedules.

### Tests for User Story 5 (TDD Required)

- [ ] T111a [P] [US5] Contract test for POST /api-keys endpoint in `tests/contract/api_key_api_test.rs` validating admin authentication and response schema
- [ ] T111b [P] [US5] Contract test for PUT /api-keys/{id}/rotate endpoint in `tests/contract/api_key_api_test.rs`
- [ ] T111c [P] [US5] Contract test for DELETE /api-keys/{id} endpoint in `tests/contract/api_key_api_test.rs`
- [ ] T111d [P] [US5] Integration test for API key authentication flow in `tests/integration/api_key_auth_test.rs` (generate, use, rotate, verify old key rejected)
- [ ] T111e [P] [US5] Integration test for supplementary invoice creation in `tests/integration/supplementary_invoice_test.rs` (create parent, start payment, add supplementary, verify inheritance and isolation)
- [ ] T111f [P] [US5] Integration test for supplementary invoice validation in `tests/integration/supplementary_invoice_test.rs` (reject if parent missing, reject if parent is draft status because payment_initiated_at IS NULL per FR-051(a), reject if parent is itself supplementary per FR-082, reject if parent status is expired/cancelled/failed with 400 "Cannot create supplementary invoice for {status} parent invoice")
- [ ] T111i [P] [US5] Integration test for admin API key authentication in `tests/integration/admin_auth_test.rs` (verify valid admin key in X-Admin-Key header succeeds for POST /api-keys, verify missing admin key returns 401 Unauthorized, verify invalid admin key returns 401, verify admin key loaded from ADMIN_API_KEY env var at startup per FR-084)
- [ ] T111j [P] [US5] Integration test for API key rotation zero-downtime in `tests/integration/api_key_rotation_test.rs` (verify active requests with old key succeed during rotation window, verify rotation completes within 2 seconds per SC-011)
- [ ] T111k [P] [US5] Unit test for admin key validation in `tests/unit/admin_key_validation_test.rs` - verify FR-084 requirements: minimum 32 characters, alphanumeric + symbols allowed, reject startup if ADMIN_API_KEY env var missing or empty, reject if key < 32 chars

### Implementation for User Story 5

**API Key Management Module**

- [ ] T016a [P] [US5] Create ApiKeyController in `src/modules/auth/controllers/api_key_controller.rs` with endpoints: POST /api-keys (generate new key with argon2 hash), PUT /api-keys/{id}/rotate (invalidate old, generate new), DELETE /api-keys/{id} (revoke key), all with audit logging to database per FR-083
- [ ] T016b [P] [US5] Create api_key_audit_log table migration in `migrations/009_create_api_key_audit_log.sql` for tracking key lifecycle events (created, rotated, revoked, used) per FR-083. Columns: id, api_key_id, operation_type ENUM('created','rotated','revoked','used'), actor_identifier, ip_address, old_key_hash (for rotation), success_status BOOLEAN, created_at TIMESTAMP
- [ ] T016c [P] [US5] Implement master admin key authentication for API key management endpoints per FR-084 (separate from regular API keys) - load from ADMIN_API_KEY env var at startup with validation (reject startup if missing/empty/<32 chars/invalid character set), validate key contains only alphanumeric + symbols (reject other characters with error "Admin key must contain only alphanumeric and symbol characters"), return 401 Unauthorized for missing/invalid admin key in X-Admin-Key header
- [ ] T111g [US5] Create ApiKeyService in `src/modules/auth/services/api_key_service.rs` for key generation, hashing, validation, rotation logic
- [ ] T111h [US5] Create ApiKeyRepository in `src/modules/auth/repositories/api_key_repository.rs` for database operations and audit logging. Implement GET /api-keys/audit endpoint with pagination (page, page_size) and date filtering (start_date, end_date ISO 8601) per FR-083(b). Audit log retention: 1 year minimum per NFR-011

**Supplementary Invoice Support** (moved from Phase 5)

- [ ] T103 [US5] Update Invoice model to support original_invoice_id reference (BIGINT UNSIGNED NULL, foreign key to invoices.id per FR-082)
- [ ] T104 [US5] Update InvoiceService to create supplementary invoices with validation per FR-082: (a) reference valid parent invoice_id, (b) validate parent exists and status is pending/partially_paid/fully_paid (reject if expired/cancelled/failed with 400 "Cannot create supplementary invoice for {status} parent invoice"), (c) validate parent with status=draft is rejected because draft means payment_initiated_at IS NULL per FR-051(a) indicating no active payment flow exists, (d) validate parent is not itself supplementary (reject with 400 "Cannot create supplementary invoice from another supplementary invoice"), (e) inherit currency and gateway from parent, (f) maintain separate payment schedule
- [ ] T105 [US5] Implement supplementary invoice creation endpoint POST /invoices/{id}/supplementary in InvoiceController with validation (parent exists, not already supplementary, inherit currency/gateway) and audit logging per FR-082

**Checkpoint**: ✅ User Story 5 complete - API key management enables production security, supplementary invoices enable flexible order modifications.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

### Documentation

- [ ] T126 [P] Create API usage examples in `docs/examples/` for each user story
- [ ] T127 [P] Create developer quickstart guide in `docs/quickstart.md` using specs/001-payment-orchestration-api/quickstart.md as reference
- [ ] T127a [P] Create initial OpenAPI 3.0 specification in `specs/001-payment-orchestration-api/contracts/openapi.yaml` with all User Story 1 endpoints per NFR-006: POST /invoices, GET /invoices/{id}, GET /invoices, POST /webhooks/{gateway}, GET /webhooks/failed, GET /invoices/{id}/transactions, GET /gateways. Include request/response schemas, authentication requirements, error responses
- [ ] T128 [P] Implement GET /openapi.json endpoint in actix-web to serve manually-maintained OpenAPI specification from `specs/001-payment-orchestration-api/contracts/openapi.yaml` per NFR-006. Implementation: (a) create static file handler in `src/middleware/openapi.rs` using actix-files crate, (b) read openapi.yaml at startup and cache in memory, (c) serve as application/json with proper CORS headers (Access-Control-Allow-Origin: *), (d) register route in main.rs before auth middleware (public endpoint), (e) add integration test in `tests/integration/openapi_endpoint_test.rs` to verify endpoint returns valid JSON and matches openapi.yaml content
- [ ] T128b [P] Implement GET /docs endpoint to serve interactive Swagger UI rendering the OpenAPI specification per NFR-006
- [ ] T128c [P] Validate OpenAPI 3.0 schema compliance using validator or contract testing framework
- [ ] T128d [P] Document OpenAPI maintenance workflow in `docs/openapi-maintenance.md`: update specification on endpoint changes, validate with contract tests per T033-T035, version with API releases, keep in sync with code
- [ ] T129 [P] Create deployment guide in `docs/deployment.md` with MySQL setup, environment variables, TLS configuration

### Code Quality

- [ ] T130 Run cargo fmt across all source files
- [ ] T131 Run cargo clippy and fix all warnings
- [ ] T132 Add comprehensive inline documentation (/// doc comments) for all public APIs
- [ ] T133 Review and refactor duplicate code across modules

### Security & Performance

- [ ] T134 Security audit: validate all input sanitization and SQL injection prevention using sqlx compile-time checks
- [ ] T135 Performance optimization: add database indexes per data-model.md (migration 007)
- [ ] T136 Performance testing: verify <2s response time for invoice creation using k6 load testing tool (NFR-001 - 95th percentile measurement)
- [ ] T136a [P] Load testing for daily volume: verify system handles 10,000 invoice creations over 24-hour period per SC-008 using k6 with distributed load (sustained peak of 500 invoices/hour during business hours, not burst). Monitor for performance degradation over time
- [ ] T137 Load testing: verify 100 concurrent requests sustained for 5 minutes using k6 with concurrent virtual users (NFR-002)
- [ ] T138 Implement graceful shutdown handling in main.rs

### Monitoring & Observability

- [ ] T139 Add structured logging for all API endpoints with request IDs
- [ ] T140 Add metrics collection for response times, error rates, gateway success rates, webhook processing success rate (99% target per NFR-004) with GET /metrics endpoint and alerting when webhook success rate <99% over 1-hour window
- [ ] T141 Add health check endpoint GET /health with database connectivity check
- [ ] T142 Add readiness probe endpoint GET /ready

### Data Retention & Compliance

- [ ] T142a Implement transaction archival background job in `src/modules/transactions/services/archival_service.rs` for 7-year retention per NFR-007 - runs daily using tokio interval timer, queries transactions older than 7 years with status IN ('paid', 'failed', 'expired'), archives to separate archive_transactions table (same schema as payment_transactions), deletes from active table after successful archive, logs archival events. Archive table indexed by archived_at timestamp for compliance queries. Background task spawned in main.rs during server startup

### Additional Testing (Optional)

- [ ] T143 [P] Add unit tests for all business logic services
- [ ] T144 [P] Add contract tests for all error response scenarios
- [ ] T145 [P] Add integration tests for concurrent request handling (FR-053, FR-054)

### Validation & Deployment Prep

- [ ] T147 Run full quickstart.md validation from specs/001-payment-orchestration-api/quickstart.md
- [ ] T148 Run all tests: `cargo test` (unit + integration + contract)
- [ ] T149 Build production binary: `cargo build --release`
- [ ] T150 Create Docker configuration if needed for deployment (Dockerfile, docker-compose.yml, .dockerignore)
- [ ] T150a [P] Create acceptance test suite in `tests/acceptance/` validating all Success Criteria SC-001 through SC-012 from spec.md. Tests must measure: SC-001 (3min invoice creation), SC-002 (95% success rate), SC-003 (financial report accuracy), SC-004 (5s webhook processing), SC-005 (2s response under load), SC-006 (zero currency mismatch), SC-007 (installment accuracy), SC-008 (10k invoices/day), SC-009 (<5 API calls/transaction), SC-010 (90% real-time updates), SC-011 (2s key rotation), SC-012 (100% supplementary integrity)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational completion - No dependencies on other stories
- **User Story 2 (Phase 4)**: Depends on Foundational completion - Extends US1 but independently testable
- **User Story 3 (Phase 5)**: Depends on Foundational completion - Extends US1/US2 but independently testable
- **User Story 4 (Phase 6)**: Depends on Foundational completion - Enhances all stories with currency isolation
- **User Story 5 (Phase 6.5)**: Depends on Foundational completion - Extends US1 with API key management and supplementary invoices
- **Polish (Phase 7)**: Depends on all desired user stories being complete

### User Story Priority Execution

- **P1 (User Story 1)**: MVP - Must complete first, independently functional
- **P2 (User Story 2)**: Can start after Foundational, adds financial reporting
- **P2 (User Story 5)**: Can start after Foundational, adds API key management and supplementary invoices
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

- T031, T032, T033, T034, T035, T036, T037, T038, T038a, T038b can run in parallel (all test files)

**User Story 1 Models**:

- T039, T040 can run in parallel (Invoice and LineItem models)
- T045, T046 can run in parallel (Xendit and Midtrans gateways)
- T049 can run in parallel (PaymentTransaction model)

**User Story 2 Tests**:

- T059, T060, T061, T062, T063, T064, T065 can run in parallel

**User Story 3 Tests**:

- T080, T081, T082, T083, T084, T085, T086, T087, T087a can run in parallel

**User Story 4 Tests**:

- T106, T107, T108, T109, T110 can run in parallel

**Polish Phase**:

- T126, T127, T128, T129 can run in parallel (documentation)
- T143, T144, T145 can run in parallel (additional tests)

### Parallel Execution Example: User Story 1

```bash
# Phase 3.1: All tests for User Story 1 (T031-T038b) in parallel:
cargo test --test line_item_calculation_test &
cargo test --test invoice_calculation_test &
cargo test --test invoice_api_test &
cargo test --test invoice_flow_test &
cargo test --test gateway_validation_test &
cargo test --test invoice_expiration_test &
cargo test --test payment_initiation_test &
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

**Total Tasks**: 192  
**Setup**: 6 tasks  
**Foundational**: 31 tasks (BLOCKING - includes T027a for RateLimiter trait, T029a for test database setup, T029b for Constitution compliance gate, T026a for webhook retry log migration, T017a for rate limiting test, T011a for timezone utilities, T011b for timezone test, T016d for tenant isolation, excludes API key management moved to US5)  
**User Story 1 (P1 - MVP)**: 37 tasks (12 tests + 25 implementation - consolidated T033-T035 into single T033, added T038a, T038b, T038c, T044a, T044b, T052a, T054c, T054d, T058a)  
**User Story 2 (P2)**: 23 tasks (8 tests + 15 implementation - added T059a for tax validation test, added T075a for tax validation implementation)  
**User Story 3 (P3)**: 23 tasks (9 tests + 14 implementation - added T087a, supplementary invoices moved to US5, removed T119 duplicate)  
**User Story 4 (P4)**: 20 tasks (6 tests + 14 implementation - removed T119 moved to US1 as T044a)  
**User Story 5 (P2)**: 17 tasks (9 tests + 8 implementation - added T111i, T111j, T111k for admin key validation, API key management + supplementary invoices)  
**Polish**: 31 tasks (removed T146 - moved to T017a in Foundational, added T128d for OpenAPI maintenance docs, added T142a for transaction archival, added T150a for acceptance tests)

**Parallel Opportunities**: ~73 tasks can run in parallel (marked with [P])

**MVP Scope** (User Story 1 only): 74 tasks (Setup + Foundational + US1)

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
