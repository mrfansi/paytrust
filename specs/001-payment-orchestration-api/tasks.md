# Implementation Tasks: PayTrust Payment Orchestration API

**Feature**: `001-payment-orchestration-api`
**Branch**: `001-payment-orchestration-api`
**Generated**: 2025-11-03
**Status**: Ready for Implementation

---

## Executive Summary

This document defines the complete implementation roadmap for PayTrust, organized into atomic, independently-executable tasks across 6 implementation phases. Each phase completes a major milestone and can be tested independently.

**Task Statistics**:
- **Total Tasks**: 89
- **Phase 1 (Setup)**: 8 tasks
- **Phase 2 (Foundational)**: 14 tasks
- **Phase 3 (US1 - Basic Invoicing)**: 16 tasks
- **Phase 4 (US2 - Additional Charges)**: 12 tasks
- **Phase 5 (US3 - Installments + US4 - Currency Isolation)**: 24 tasks
- **Phase 6 (US5 - API Key Management + Polish)**: 15 tasks

**Implementation Strategy**: MVP-first approach - complete Phase 3 (User Story 1) for core functionality, then layer additional features. Each user story is independently testable and deployable.

**Parallel Opportunities**:
- Phase 1: All 8 setup tasks can run in parallel (different directories)
- Phase 2: Foundational tasks can run with 2-3 parallel branches
- Phase 3+: Most story-specific tasks parallelizable within story constraints

---

## Phase Dependencies & Completion Order

```
Phase 1: Setup (independent of others)
    ↓
Phase 2: Foundational (blocking for all stories)
    ↓
Phase 3: US1 (Basic Invoicing) - Can test independently
    ↓
Phase 4: US2 (Additional Charges) - Builds on US1
    ↓
Phase 5: US3 (Installments) + US4 (Currency Isolation) - Can run in parallel
    ↓
Phase 6: US5 (API Key Management) + Polish - Final touches
```

---

## Phase 1: Setup & Project Initialization

Initialize project structure, dependencies, and database connection.

- [ ] T001 Initialize Rust project structure with Cargo.toml and workspace layout
- [ ] T002 Set up PostgreSQL migrations directory and migration runner (sqlx-cli)
- [ ] T003 Create Rust module structure (src/models, src/services, src/api, src/auth, src/error, src/gateway, src/iso20022, src/rate_limit)
- [ ] T004 Configure actix-web server with middleware stack (logging, error handling) in src/main.rs
- [ ] T005 Set up PostgreSQL connection pool (sqlx::Pool) with connection pooling configuration
- [ ] T006 Create development database configuration and environment file (.env) with database URL and API credentials
- [ ] T007 Set up Cargo.toml with core dependencies (actix-web 4.9+, tokio, serde, sqlx, uuid, chrono, argon2, decimal)
- [ ] T008 Create .gitignore, README.md, and CHANGELOG.md documentation stubs

---

## Phase 2: Foundational Architecture

Implement core infrastructure components used by all user stories.

- [ ] T009 Create database schema migration file for all tables (invoices, line_items, installment_schedules, payment_transactions, gateway_configurations, api_keys, api_key_audit_log, webhook_events, webhook_retry_log, tenants) in migrations/ directory
- [ ] T010 [P] Implement error handling module (src/error/mod.rs) with custom error types, conversion to HTTP responses, and error serialization
- [ ] T011 [P] Implement authentication middleware (src/auth/middleware.rs) for API key extraction, tenant_id lookup, and request context injection
- [ ] T012 [P] Create models module (src/models/mod.rs) with domain entity structs: Invoice, LineItem, PaymentTransaction, InstallmentSchedule (per data-model.md definitions)
- [ ] T013 [P] Implement in-memory rate limiter (src/rate_limit/memory.rs) with HashMap<api_key_id, RateLimitState> using tokio::sync::Mutex, 60-second sliding window, and 1000 req/min limit per FR-038
- [ ] T014 [P] Create API response wrapper (src/api/response.rs) for standardized JSON responses with success/error handling
- [ ] T015 [P] Implement rate limit middleware (src/api/middleware.rs) that checks rate limiter and returns 429 Too Many Requests with Retry-After header on exceeding limits
- [ ] T016 [P] Create multi-tenant request context extractor (src/auth/context.rs) propagating tenant_id from authenticated API key through request handlers
- [ ] T017 [P] Set up test database infrastructure with fixtures (tests/fixtures/setup.rs) including test database connection, schema migration, and cleanup functions per NFR-008
- [ ] T018 [P] Create repository trait (src/repository/mod.rs) defining CRUD interface for domain entities with tenant_id filtering
- [ ] T019 [P] Implement invoice repository (src/repository/invoice_repo.rs) with sqlx::query! compile-time SQL verification, tenant_id WHERE clause enforcement, and transaction support
- [ ] T020 [P] Implement line item repository (src/repository/line_item_repo.rs) with invoice_id and tenant_id filtering
- [ ] T021 [P] Implement payment transaction repository (src/repository/payment_transaction_repo.rs) with gateway deduplication logic (unique constraint handling on gateway_id, gateway_transaction_id, tenant_id)
- [ ] T022 [P] Create logging infrastructure (src/logging/mod.rs) with structured logging for audit trails per FR-033, including operation tracking for audit log table writes

---

## Phase 3: User Story 1 - Basic Invoice Creation and Payment

Implement core invoice management with single-payment and multi-line-item support.

**Story Goal**: Developers can create invoices with multiple line items and process payments through Xendit or Midtrans gateways.

**Independent Test Criteria**:
- Create invoice with multiple line items ✅
- System returns valid payment URL ✅
- Webhook notification updates invoice status ✅
- Invoice status query returns accurate state ✅
- Multi-line-item subtotal calculation matches expected total ✅

### Core Models & Services

- [ ] T023 [US1] Implement Invoice model in src/models/invoice.rs with all fields (id, tenant_id, external_id, gateway_id, currency_code, status, payment_initiated_at, amounts, expiration handling)
- [ ] T024 [US1] Implement LineItem model in src/models/line_item.rs with quantity, unit_price, subtotal calculation, and tax_rate fields
- [ ] T025 [US1] Create PaymentGateway trait (src/gateway/mod.rs) defining interface: create_payment_url(Invoice, InstallmentSchedule?) → Result<PaymentUrl>
- [ ] T026 [P] [US1] Implement Xendit gateway adapter (src/gateway/xendit.rs) with PaymentGateway trait implementation, POST /v2/invoices API client, and error handling for gateway responses
- [ ] T027 [P] [US1] Implement Midtrans gateway adapter (src/gateway/midtrans.rs) with PaymentGateway trait implementation, POST /v2/charge API client, and error handling for gateway responses
- [ ] T028 [P] [US1] Create invoice service (src/services/invoice_service.rs) implementing: create_invoice() with line item validation, gateway selection validation (FR-007, FR-042), and payment_initiated_at timing logic (FR-046)
- [ ] T029 [US1] Implement installment schedule service (src/services/installment_service.rs) with installment creation and validation (currently for single-payment only in US1)
- [ ] T030 [US1] Create payment transaction service (src/services/payment_service.rs) with payment processing, status updates, and webhook handling coordination

### API Endpoints

- [ ] T031 [P] [US1] Implement POST /invoices endpoint (src/api/handlers/invoices.rs) accepting line_items array, currency, gateway_id, optional external_id, and returning 201 response with invoice_id, payment_url, total_amount (per FR-075) including line_items in response body
- [ ] T032 [P] [US1] Implement GET /invoices/{id} endpoint returning complete invoice details with status, line items, calculated amounts, and expiration time
- [ ] T033 [US1] Implement GET /invoices/{id}/installments endpoint returning installment_schedule array with status per installment

### Database & Persistence

- [ ] T034 [P] [US1] Create Invoice repository implementation (src/repository/invoice_repo.rs) with compile-time SQL verification using sqlx::query!, tenant_id filtering enforcement, and immutability checking (payment_initiated_at IS NOT NULL checks)
- [ ] T035 [P] [US1] Create LineItem repository implementation (src/repository/line_item_repo.rs) with invoice_id filtering and bulk insert support for multiple line items
- [ ] T036 [US1] Implement database migration for invoices and line_items tables with correct schema per data-model.md

### Webhook & Payment Status

- [ ] T037 [P] [US1] Create webhook handler (src/api/handlers/webhooks.rs) accepting gateway-native webhook payloads (Xendit/Midtrans JSON), deduplicating by (gateway_id, event_id, tenant_id), and triggering payment transaction creation
- [ ] T038 [P] [US1] Implement payment webhook processor (src/services/webhook_processor.rs) converting gateway webhook to payment transaction record, updating invoice status, and storing original payload in webhook_events table per FR-028
- [ ] T039 [US1] Implement webhook retry logic (src/services/webhook_retry.rs) with in-memory queue, retry schedule per FR-040 (1min, 6min, 36min), and async tokio task execution

### Testing

- [ ] T040 [US1] Create integration tests for invoice creation (tests/integration/invoice_creation_test.rs) with real PostgreSQL, multiple line items, and response validation
- [ ] T041 [US1] Create contract tests for POST /invoices endpoint (tests/contract/invoices_contract_test.rs) validating request/response against openapi.yaml schema
- [ ] T042 [P] [US1] Create webhook integration tests (tests/integration/webhook_test.rs) with real database, deduplication validation, and status update verification
- [ ] T043 [US1] Create invoice service unit tests (tests/unit/invoice_service_test.rs) for validation logic and error handling

### Validation & Error Handling

- [ ] T044 [P] [US1] Implement invoice creation validation (src/services/invoice_service.rs) including: currency validation (ISO 4217 codes only), gateway_id validation (must exist in gateway_configurations), gateway currency support validation (FR-042), line item validation (product_name required, quantity > 0), and expiration time validation (FR-041)
- [ ] T045 [P] [US1] Create error responses module (src/api/errors.rs) with standardized error formats per FR-034, FR-036 for validation, gateway unavailable, and concurrency conflict scenarios
- [ ] T046 [US1] Implement idempotency support (src/services/idempotency.rs) using external_id for deduplication - same external_id returns same invoice (per FR-030)

**Phase 3 Completion Gate**: Successfully create invoice with 3 line items, receive payment_url, process webhook notification, and verify invoice marked "paid" without any additional API calls beyond the 2 calls in happy path (POST /invoices + webhook receipt per SC-009).

---

## Phase 4: User Story 2 - Additional Charges (Tax & Service Fees)

Implement tax and service fee calculation, distribution, and financial reporting.

**Story Goal**: Developers add taxes and service fees to invoices and generate financial reports showing breakdown of charges.

**Independent Test Criteria**:
- Tax calculated correctly on line item subtotals ✅
- Service fees calculated per gateway (percentage + fixed) ✅
- Total amount includes subtotal + tax + service fee in correct order ✅
- Financial report aggregates fees/taxes by currency and gateway ✅
- High tax rate warning logged when tax_rate > 0.27 ✅

### Tax & Fee Calculation

- [ ] T047 [P] [US2] Implement tax calculation in invoice service (src/services/invoice_service.rs): per-line-item tax = line_item.subtotal × tax_rate, total_tax = SUM(line_item taxes) per FR-051
- [ ] T048 [P] [US2] Implement service fee calculation in invoice service (src/services/invoice_service.rs): service_fee = (subtotal × gateway.fee_percentage) + gateway.fee_fixed_amount per FR-009, applied AFTER tax per FR-049
- [ ] T049 [P] [US2] Create currency converter module (src/utils/currency.rs) with to_cents/from_cents functions for IDR (whole numbers) and MYR/USD (÷100) per data-model.md currency handling
- [ ] T050 [P] [US2] Implement tax rate validation in invoice service: tax_rate >= 0 and <= 1.0, max 4 decimal places per FR-058, log WARN level if tax_rate > 0.27 with invoice_id and line_item_id
- [ ] T051 [P] [US2] Add per-line-item tax_rate and tax_category fields to POST /invoices request body parsing (src/api/handlers/invoices.rs) with validation

### Financial Reporting

- [ ] T052 [P] [US2] Create financial report service (src/services/report_service.rs) with: query invoices by date range (start_date, end_date), aggregate by currency, sum subtotal/tax/service_fee amounts, group tax breakdown by tax_rate
- [ ] T053 [P] [US2] Implement GET /reports/financial endpoint (src/api/handlers/reports.rs) accepting start_date, end_date parameters (ISO 8601), returning response per quickstart.md example with:
  - by_currency array: [{currency_code, transaction_count, subtotal_amount, total_tax_amount, total_service_fee_amount, total_amount, tax_breakdown: [{tax_rate, amount}], gateway_breakdown: [{gateway_name, transaction_count, service_fee_amount}]}]
  - Ensure gateway_breakdown is included per spec.md L137-138 acceptance scenario requirement
- [ ] T054 [P] [US2] Implement database query optimization (src/repository/report_repo.rs) with indexes on (tenant_id, created_at), (currency_code, tax_rate), and (gateway_id, created_at) per research.md recommendations
- [ ] T055 [US2] Add gateway_configurations repository (src/repository/gateway_repo.rs) with fee_percentage and fee_fixed_amount lookup

### Models & Persistence

- [ ] T056 [P] [US2] Update Invoice model to include: total_tax_amount (BIGINT), total_service_fee_amount (BIGINT), separate tracking from subtotal
- [ ] T057 [P] [US2] Update LineItem model to include: tax_rate (Decimal), tax_category (Option<String>), country_code (Option<String>), tax_amount (BIGINT)
- [ ] T058 [US2] Create database migration for tax/fee fields on invoices and line_items tables

### Testing

- [ ] T059 [P] [US2] Create tax calculation integration tests (tests/integration/tax_calculation_test.rs) validating per-line-item tax rates, total tax calculation, and rounding
- [ ] T060 [P] [US2] Create service fee calculation tests (tests/integration/service_fee_test.rs) validating percentage+fixed formula for Xendit and Midtrans gateways
- [ ] T061 [P] [US2] Create financial report integration tests (tests/integration/financial_report_test.rs) with multiple currencies, date ranges, and aggregation verification
- [ ] T062 [US2] Create high tax rate warning tests (tests/unit/tax_validation_test.rs) verifying WARN log when tax_rate > 0.27

**Phase 4 Completion Gate**: Create invoice with line items, tax rates, and service fees; generate financial report showing correct breakdown by currency, gateway, and tax rate with zero rounding errors.

---

## Phase 5: User Story 3 & 4 - Installments & Currency Isolation

Implement flexible installment payment scheduling and complete currency isolation.

**Story Goals**:
- (US3) Enable customers to pay invoices in 2-12 installments with custom amount distribution
- (US4) Ensure complete currency isolation preventing mixing and mismatch errors

**Independent Test Criteria**:
- Create invoice with N installments with equal default split ✅
- Adjust individual installment amounts while maintaining total ✅
- Sequential payment enforcement (only next unpaid installment available) ✅
- Overpayment auto-applies to remaining installments ✅
- Currency mismatch errors on cross-currency payment attempts ✅
- Financial reports separate all currencies with no mixing ✅

### Installment Scheduling (US3)

- [ ] T063 [P] [US3] Extend invoice service (src/services/invoice_service.rs) with installment schedule generation: create N equal installments from invoice total per FR-015
- [ ] T064 [P] [US3] Implement proportional tax/fee distribution (src/services/installment_service.rs): installment_tax = total_tax × (installment_amount / total_amount), same for service_fee per FR-052, FR-053
- [ ] T065 [P] [US3] Implement rounding strategy (src/services/installment_service.rs): round down all except last, last = total - sum(previous) per FR-063, FR-064
- [ ] T066 [P] [US3] Extend POST /invoices to accept optional installments parameter: {count: 2-12} per FR-014, update response to include installment_schedule array (per FR-075)
- [ ] T067 [P] [US3] Implement installment status service (src/services/installment_service.rs) with payment tracking: unpaid → paid (on transaction complete), overdue (due_date < now AND unpaid)
- [ ] T068 [P] [US3] Create instalment repository (src/repository/installment_repo.rs) with query by installment_number for sequential enforcement
- [ ] T069 [US3] Implement sequential payment enforcement (src/services/payment_service.rs): only generate payment URL for next unpaid installment (lowest unpaid installment_number per FR-060, FR-061)
- [ ] T070 [P] [US3] Implement overpayment handling (src/services/payment_service.rs): accept excess, apply to next unpaid installments sequentially, mark intermediate installments paid per FR-065
- [ ] T071 [P] [US3] Implement overpayment query service (src/services/invoice_service.rs) and GET /invoices/{id}/overpayment endpoint returning {total_amount, total_paid, overpayment_amount} per FR-065
- [ ] T072 [P] [US3] Create unpaid installment adjustment service (src/services/installment_service.rs) allowing modification of unpaid installments while maintaining total remaining balance per FR-066, FR-068

### Currency Isolation (US4)

- [ ] T073 [P] [US4] Implement currency validation (src/services/invoice_service.rs): reject non-ISO 4217 codes per FR-002, validate with ISO 4217 registry on invoice creation
- [ ] T074 [P] [US4] Implement gateway-currency validation (src/services/invoice_service.rs): check gateway_configurations.supported_currencies JSON array, reject if currency not supported per FR-042
- [ ] T075 [P] [US4] Create currency-specific amount converter (src/utils/currency.rs): IDR (whole numbers, no decimals), MYR/USD (2 decimal places only), reject invalid formats per FR-026
- [ ] T076 [P] [US4] Implement currency isolation in queries (src/repository/invoice_repo.rs): all financial reports query by (tenant_id, currency_code) to prevent mixing per FR-025
- [ ] T077 [P] [US4] Add country_code validation (src/services/invoice_service.rs): accept optional ISO 3166-1 alpha-2 country codes per line item, validate against registry per FR-002a
- [ ] T078 [P] [US4] Extend financial report by currency (src/services/report_service.rs): return separate totals per currency with no automatic conversion per FR-025
- [ ] T079 [US4] Create currency mismatch tests (tests/integration/currency_isolation_test.rs) validating rejection of cross-currency payments and separate accounting

### Installment Testing

- [ ] T080 [P] [US3] Create installment schedule generation tests (tests/integration/installment_generation_test.rs) with equal split, proportional tax/fee distribution, and rounding verification
- [ ] T081 [P] [US3] Create sequential payment tests (tests/integration/sequential_payment_test.rs) verifying only next installment URL available, rejection of out-of-order attempts
- [ ] T082 [P] [US3] Create overpayment tests (tests/integration/overpayment_test.rs) with excess application to multiple installments and final invoice status
- [ ] T083 [US3] Create installment adjustment tests (tests/integration/installment_adjustment_test.rs) with unpaid amount changes and total balance validation

**Phase 5 Completion Gate**: Create invoice with 3 installments in MYR, adjust installment 2 after installment 1 paid, overpay final installment by 50, verify auto-application, and generate report showing MYR currency isolation with zero cross-currency leakage.

---

## Phase 6: User Story 5 & Polish - API Key Management & Finalization

Implement API key lifecycle management, ISO 20022 compliance export, supplementary invoices, and comprehensive testing.

**Story Goal**: (US5) Manage API keys with rotation, revocation, and audit logging. Support supplementary invoices for mid-payment item additions.

**Independent Test Criteria**:
- Generate API key returns 64-char hex string ✅
- Key rotation creates new key, marks old as rotated ✅
- Revoked key rejected on subsequent requests ✅
- Audit log tracks all operations with actor_identifier ✅
- Supplementary invoice linked to parent, inherits currency/gateway ✅
- ISO 20022 pain.001 XML export passes schema validation ✅

### API Key Management (US5)

- [ ] T084 [P] [US5] Implement API key generation service (src/services/auth_service.rs): generate 64-char random hex string, hash with Argon2, store prefix (first 4 chars) per FR-031, FR-071
- [ ] T085 [P] [US5] Create POST /api-keys endpoint (src/api/handlers/auth.rs) with admin-only authentication via X-Admin-Key header (separate from user API key), generating new key, returning key once per FR-072
- [ ] T086 [P] [US5] Implement API key rotation service (src/services/auth_service.rs): mark old key as rotated, create new active key, log rotation with old_key_hash per FR-071
- [ ] T087 [P] [US5] Create PUT /api-keys/{id}/rotate endpoint with admin authentication, implementing key rotation workflow
- [ ] T088 [P] [US5] Implement API key revocation service (src/services/auth_service.rs): mark key as revoked, reject future requests with revoked key per FR-071
- [ ] T089 [P] [US5] Create DELETE /api-keys/{id} endpoint with admin authentication, implementing key revocation
- [ ] T090 [P] [US5] Create audit log service (src/services/audit_service.rs) logging all key operations: created, rotated, revoked, used (per-request auth) with actor_identifier, ip_address, timestamp per FR-071
- [ ] T091 [P] [US5] Implement GET /api-keys/audit endpoint with pagination (page, page_size) and date range filtering (start_date, end_date) returning audit log entries per FR-071
- [ ] T092 [US5] Create API key audit repository (src/repository/audit_repo.rs) with date range queries and pagination support

### Supplementary Invoices & ISO 20022

- [ ] T093 [P] [US5] Implement supplementary invoice creation service (src/services/invoice_service.rs): validate parent invoice exists, not draft, not itself supplementary, create new invoice with original_invoice_id reference per FR-070
- [ ] T094 [P] [US5] Create POST /invoices/{id}/supplementary endpoint accepting new line_items, inheriting currency and gateway_id from parent, with separate payment schedule
- [ ] T095 [P] [US5] Implement ISO 20022 pain.001 generation (src/iso20022/pain001.rs): create XML document from Invoice model with MessageID (invoice_id), ReferenceID (external_id), CreationDateTime, Currency (ISO 4217), Amount, party identifiers, tax jurisdiction details per FR-028b
- [ ] T096 [P] [US5] Create GET /invoices/{id}/payment-initiation endpoint returning pain.001 XML with Content-Type: application/xml, validating invoice exists, belongs to authenticated tenant, with error handling per FR-028b
- [ ] T097 [US5] Implement pain.001 XSD schema validation (src/iso20022/validation.rs) ensuring generated XML passes ISO 20022 compliance validation per SC-013

### Concurrency & Locking (Core Safety Feature)

- [ ] T098 [P] [US5] Implement pessimistic locking (src/services/payment_service.rs): SELECT FOR UPDATE with 5-second timeout, exponential backoff (100ms, 200ms) ±20ms jitter, max 3 retries per FR-047
- [ ] T099 [P] [US5] Create lock timeout error handler returning 409 Conflict "payment already in progress" when lock cannot be acquired per FR-048
- [ ] T100 [US5] Create concurrency tests (tests/integration/concurrency_test.rs) with parallel payment requests validating lock behavior and conflict response

### Comprehensive Testing & Quality Gates

- [ ] T101 [P] [US5] Create integration test suite for API key lifecycle (tests/integration/api_key_lifecycle_test.rs) with generation, rotation, revocation, and audit log queries
- [ ] T102 [P] [US5] Create supplementary invoice tests (tests/integration/supplementary_invoice_test.rs) validating parent linkage, currency inheritance, and separate payment schedule
- [ ] T103 [P] [US5] Create ISO 20022 pain.001 tests (tests/integration/iso20022_pain001_test.rs) with XSD schema validation and field mapping verification
- [ ] T104 [P] [US5] Code review gate task (T029b): Manual review checklist - verify: (1) all queries include tenant_id filtering, (2) payment_initiated_at immutability enforced, (3) no plain-text key storage, (4) rate limiter middleware applied to all endpoints, (5) error responses include appropriate HTTP status codes and messages
- [ ] T105 [P] [US5] Automated CI check task (T029c): Add CI workflow that: (1) runs sqlx::query! compile-time verification, (2) checks for mock usage in integration test directory (fail if found), (3) validates rate limiter applied to rate-limited endpoints, (4) runs full integration test suite with real PostgreSQL
- [ ] T106 [US5] Performance validation task: Run k6 load test with 100 concurrent users for 5 minutes, verify p95 response time < 2 seconds per NFR-001, sustained throughput >= 100 req/sec per NFR-002

### Documentation & Deployment

- [ ] T107 Create API documentation (src/openapi.rs) serving GET /openapi.json and GET /docs (Swagger UI) from contracts/openapi.yaml per NFR-006
- [ ] T108 Update README.md with: development setup, database migration steps, local testing instructions, deployment checklist
- [ ] T109 Create CONTRIBUTING.md with: code style guidelines, testing requirements, commit message format, PR review checklist
- [ ] T110 Create DEPLOYMENT.md with: environment variables (database URL, API credentials, ADMIN_API_KEY), database initialization, backup strategy, monitoring recommendations
- [ ] T111 Update CHANGELOG.md documenting all features, bug fixes, and breaking changes

**Phase 6 Completion Gate**: Generate API key, rotate it, audit log shows both operations with timestamps, create supplementary invoice, export parent invoice as pain.001 XML validating against schema, and execute load test showing p95 response time < 2 seconds under 100 concurrent users.

---

## Cross-Cutting Quality Assurance

### Testing Coverage

- [ ] T112 [P] Implement comprehensive error handling tests (tests/integration/error_handling_test.rs) covering: validation errors (400), authentication failures (401/403), rate limiting (429), concurrency conflicts (409), gateway failures (500+)
- [ ] T113 [P] Create multi-tenant isolation tests (tests/integration/multi_tenant_test.rs) verifying queries with different API keys don't leak cross-tenant data
- [ ] T114 [P] Create currency decimal handling tests (tests/unit/currency_test.rs) for IDR whole numbers, MYR/USD 2-decimal formatting, and rounding edge cases
- [ ] T115 [P] Create database schema validation tests (tests/integration/schema_validation_test.rs) verifying all constraints, indexes, and referential integrity
- [ ] T116 Create immutability enforcement tests (tests/integration/immutability_test.rs) verifying invoice locked after payment_initiated_at set, rejecting all modifications
- [ ] T117 Create webhook deduplication tests (tests/integration/webhook_dedup_test.rs) with duplicate event_id, verifying 200 OK without re-processing

### Database & Migration Safety

- [ ] T118 [P] Implement database migration tests (tests/integration/migration_test.rs) running migrations from empty schema, verifying all tables created with correct types/constraints
- [ ] T119 [P] Create migration rollback tests (tests/integration/rollback_test.rs) verifying down migrations remove tables without data corruption
- [ ] T120 Create data retention verification (tests/integration/data_retention_test.rs) documenting 7-year transaction retention per NFR-007, 1-year audit log retention per NFR-011

### Performance & Load Testing

- [ ] T121 [P] Create k6 load test script (tests/load/invoice_creation.js) simulating 100 concurrent users creating invoices, measuring p95 response time < 2s per NFR-001, p99 < 5s
- [ ] T122 [P] Create k6 webhook processing test (tests/load/webhook_processing.js) simulating payment webhooks, verifying < 5s p95 completion with 99% success rate per NFR-004
- [ ] T123 Create k6 financial report test (tests/load/financial_report.js) querying reports with various date ranges, verifying fast execution even with 10k+ transactions per NFR-010
- [ ] T124 Create baseline performance documentation (PERFORMANCE.md) with p50/p95/p99 metrics across all endpoints under standard load

### Security & Compliance Validation

- [ ] T125 [P] Create API key security tests (tests/integration/api_key_security_test.rs) verifying: no plain-text keys in logs/responses, Argon2 hashing on all keys, key prefix only stored for identification
- [ ] T126 [P] Create audit log completeness tests (tests/integration/audit_log_test.rs) verifying all operations logged: API key create/rotate/revoke, invoice creation, payment processing
- [ ] T127 Create data isolation verification (tests/integration/data_isolation_test.rs) testing SQL injection prevention via sqlx::query! compile-time checking
- [ ] T128 Create webhook signature validation tests (tests/integration/webhook_signature_test.rs) for Xendit and Midtrans webhook authentication
- [ ] T129 Create timezone compliance test (tests/unit/timezone_test.rs) verifying all timestamps stored UTC per FR-073, converted for gateway APIs (Xendit: UTC, Midtrans: UTC+7)
- [ ] T130 Create ISO 20022 compliance checklist (COMPLIANCE.md) documenting: pain.001 generation, pain.002 conversion, ISO 4217 currency codes, ISO 3166 country codes, schema validation

### Final Integration & Documentation

- [ ] T131 Create end-to-end integration test (tests/integration/e2e_test.rs) covering complete happy path: invoice creation → payment URL → webhook processing → status query → financial report
- [ ] T132 Create deployment checklist (DEPLOYMENT.md) with: environment variable validation, database schema verification, rate limiter initialization, admin key rotation, monitoring setup
- [ ] T133 Create troubleshooting guide (TROUBLESHOOTING.md) with: common errors, debugging strategies, log interpretation, gateway integration issues
- [ ] T134 Create performance tuning guide (PERFORMANCE_TUNING.md) with: connection pool sizing, query optimization, index usage, materialized view refresh strategy
- [ ] T135 Final code review and refactoring: consolidate utilities, remove duplication, ensure consistent error handling, update documentation

---

## Implementation Guidelines

### Task Execution Rules

1. **Sequential Dependencies**: Tasks within a phase can run in parallel when marked [P], but must wait for unmarked predecessors
2. **Testing**: Each task with code should have corresponding tests (integration or unit as appropriate)
3. **Code Review**: Significant tasks (>200 lines) require review before merge
4. **Documentation**: Update README.md, CHANGELOG.md, and inline code comments with each change
5. **Git Commits**: Create one commit per task (e.g., "T001: Initialize project structure") with meaningful commit messages

### Quality Gates by Phase

- **Phase 1 Complete**: Project compiles, migrations run, database connection pools established
- **Phase 2 Complete**: All foundational services tested with integration tests using real PostgreSQL
- **Phase 3 Complete**: Happy path (invoice → payment → webhook) works end-to-end, <3 API calls per transaction
- **Phase 4 Complete**: Tax/fee calculations verified correct, financial reports accurate within 1 hour
- **Phase 5 Complete**: Installments sequential, overpayment auto-applies, currency isolation verified
- **Phase 6 Complete**: API keys secure, ISO 20022 compliant, load test p95 <2s, full test coverage >85%

### MVP Scope

**Minimum Viable Product** = Phase 1 + Phase 2 + Phase 3
- Create invoices with multiple line items
- Process single payments through Xendit/Midtrans
- Receive webhook notifications
- Query invoice status
- ~35 tasks, ~2 weeks for experienced Rust developer

**v1.0 Release** = Phases 1-6 (all tasks)
- Complete payment orchestration with installments
- Multi-currency support with isolation
- Financial reporting
- API key management
- ISO 20022 compliance
- ~135 tasks total, ~6-8 weeks

### Parallel Execution Examples

**Phase 1 (All 8 tasks parallelizable)**:
- Thread 1: T001 (project structure) + T004 (server config) + T007 (Cargo dependencies)
- Thread 2: T002 (migrations) + T005 (database pool) + T006 (environment)
- Thread 3: T003 (module structure) + T008 (documentation)

**Phase 3 (8 parallel tracks for core features)**:
- Models track: T023 + T024 (models only, 0 cross-dependencies)
- Gateway track: T026 + T027 (both adapters, independent)
- Service track: T028 + T029 + T030 (can start after models)
- API track: T031 + T032 + T033 (can start after services)
- Repository track: T034 + T035 (can start after models)
- Testing track: T040-T043 (can start after endpoints exist)

---

## Success Criteria for Implementation

✅ **Functional Completeness**:
- All 89 tasks implemented
- Zero missing features per spec
- All acceptance scenarios passing per user stories

✅ **Quality Standards**:
- Test coverage > 85% (with real database integration tests)
- Zero sqlx::query! compilation errors
- Zero rate limiter bypasses
- Zero cross-tenant data access paths
- All timestamps UTC internally
- All financial calculations ±1 unit accuracy

✅ **Performance Targets**:
- p95 response time < 2 seconds (NFR-001)
- 100 concurrent requests sustained 5 minutes (NFR-002)
- Webhook processing < 5 seconds p95 (NFR-004)
- Financial reports < 1 second query (SC-003)

✅ **Security & Compliance**:
- No plain-text keys in logs/storage
- Audit logs for all operations
- Multi-tenant isolation enforced
- ISO 20022 pain.001/pain.002 generation
- Webhook signature validation
- Rate limiting 1000 req/min per key

✅ **Production Readiness**:
- Database migrations automated
- Error handling comprehensive
- Monitoring/logging instrumented
- Deployment documentation complete
- Performance baseline established

---

**Ready to begin! Start with Phase 1 tasks (T001-T008) in parallel to quickly establish project foundation.**
