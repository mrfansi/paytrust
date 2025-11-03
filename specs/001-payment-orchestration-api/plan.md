# Implementation Plan: PayTrust Payment Orchestration Platform

**Branch**: `001-payment-orchestration-api` | **Date**: 2025-11-03 | **Spec**: [specs/001-payment-orchestration-api/spec.md](spec.md)
**Input**: Feature specification from `/specs/001-payment-orchestration-api/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command.

## Summary

PayTrust is a backend payment orchestration API built in Rust that unifies multiple payment gateways (Xendit and Midtrans) into a single, developer-friendly REST/JSON interface. Core capabilities include invoice management with multiple line items, installment payment planning, multi-currency support (IDR/MYR/USD with proper isolation), financial reporting for service fees and taxes, and ISO 20022 compliance for international payment standards. The system handles the complete transaction lifecycle from invoice creation through payment completion with webhook-based gateway integration, enforcing immutability after payment initiation, sequential installment validation, and proportional fee/tax distribution.

## Technical Context

**Language/Version**: Rust 1.91+ (2021 edition)
**Primary Dependencies**: actix-web 4.9+, tokio (async runtime), serde (JSON), sqlx (PostgreSQL driver)
**Storage**: PostgreSQL 13+ with standard table storage (no special extensions required initially)
**Testing**: cargo test with real database integration tests per Constitution (mocks only for isolated unit tests)
**Target Platform**: Linux server / cloud deployment
**Project Type**: Single backend API project
**Performance Goals**: NFR-001: <2 second p95 response time for invoice creation under 100 concurrent requests; NFR-002: sustain 100 concurrent API requests for 5 minutes; SC-008: 10,000 invoices/day system-wide; SC-009: <5 API calls per transaction
**Constraints**: NFR-001/NFR-002: minimum 4 vCPU (2.5GHz+), 8GB RAM, PostgreSQL on dedicated SSD; NFR-003: 99.5% monthly uptime (3.6 hours max unplanned downtime); NFR-004: webhook processing <5 seconds p95 with 99% success rate; NFR-005: accuracy to smallest currency unit (1 IDR, 0.01 MYR/USD); NFR-009: single instance only in v1.0 (Redis needed for multi-instance)
**Scale/Scope**: Supports multi-tenant architecture (one tenant per API key), 2-12 installment plans per invoice, 3 currencies, 2 payment gateways (Xendit/Midtrans), ISO 20022 compliance requirements

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Principle I - Real Database for Integration Tests**: ✅ REQUIRED
- Integration tests MUST use real PostgreSQL database instances
- Mocks allowed ONLY for isolated unit tests of business logic
- Enforcement: manual code review gate (tasks.md:T029b) + automated CI check (tasks.md:T029c)

**Principle II - Single Responsibility**: ✅ COMPLIANT
- Single backend API project (Rust with actix-web)
- Clear separation: models → services → API handlers
- Repository pattern for data access

**Principle III - Async/Concurrency**: ✅ REQUIRED
- actix-web + tokio for async handling
- Concurrent payment processing with pessimistic locking (FR-047)
- Webhook retry queue with tokio async tasks (FR-040)

**Principle IV - Error Handling**: ✅ REQUIRED
- Result-based error handling (Rust)
- Descriptive HTTP error responses per FR-036
- Audit logging for all critical operations (FR-033, FR-071)

**All gates PASS** - Proceed to Phase 0 research

## Project Structure

### Documentation (this feature)

```text
specs/001-payment-orchestration-api/
├── plan.md              # This file (/speckit.plan output)
├── research.md          # Phase 0 output ✅ COMPLETE
├── data-model.md        # Phase 1 output ✅ COMPLETE
├── quickstart.md        # Phase 1 output ✅ COMPLETE
├── contracts/           # Phase 1 output ✅ COMPLETE
│   ├── openapi.yaml     # OpenAPI 3.0 specification with 12 endpoints
│   ├── rate_limiter_trait.rs  # Rate limiting interface (in-memory + Redis-ready)
│   └── [other contracts to be added during implementation]
└── tasks.md             # Phase 2 output (generate with /speckit.tasks command)
```

### Source Code (repository root)

```text
src/
├── models/              # Domain entities (Invoice, LineItem, PaymentTransaction, etc.)
├── services/            # Business logic (InvoiceService, PaymentService, ReportService)
├── api/                 # HTTP handlers and routes
├── gateway/             # Payment gateway adapters (Xendit, Midtrans)
├── iso20022/            # ISO 20022 compliance (pain.001, pain.002 conversion)
├── auth/                # API key management and authentication
├── rate_limit/          # Rate limiting implementation
├── error/               # Error types and handling
└── main.rs              # Application entry point

tests/
├── integration/         # Real PostgreSQL integration tests
├── contract/            # API contract tests
└── fixtures/            # Test data and database setup
```

**Structure Decision**: Single Rust backend project with modular domain-driven organization. No separate frontend (developers integrate via REST API). Single instance deployment per NFR-009 using in-memory rate limiter. Future multi-instance deployment will require Redis-backed rate limiter (documented as future enhancement).

---

## Phase 0 Research - COMPLETE ✅

**Output**: `research.md`

**Completed**: 12 major research areas resolved
- Rust + Actix-web framework selection and patterns
- PostgreSQL schema design for payment workflows
- ISO 20022 pain.001 hybrid approach (internal + REST/JSON)
- ISO 20022 pain.002 webhook adapter pattern
- Installment payment gateway integration strategy
- Concurrent payment locking with pessimistic SELECT FOR UPDATE
- Rate limiting architecture (in-memory v1.0, Redis-ready future)
- Webhook retry queue with tokio async tasks
- Financial reporting query optimization
- Multi-tenant data isolation patterns
- Currency decimal place handling (IDR/MYR/USD)
- API key audit logging implementation

**Constitution Check**: ✅ All gates PASS (re-validated post-design)

---

## Phase 1 Design & Contracts - COMPLETE ✅

**Outputs**:
1. `data-model.md` - Complete database schema and Rust domain models
2. `contracts/openapi.yaml` - 12 API endpoints with request/response schemas
3. `contracts/rate_limiter_trait.rs` - Rate limiter interface (in-memory + Redis future)
4. `quickstart.md` - Developer integration guide with examples

### Data Model Highlights

**Core Tables**:
- `invoices` - 14 fields, status tracking, immutability enforcement
- `line_items` - per-line-item tax rates per FR-050
- `installment_schedules` - 2-12 installments with proportional tax/fee distribution
- `payment_transactions` - gateway transaction tracking with ISO 20022 mapping
- `gateway_configurations` - Xendit/Midtrans configuration (predefined)
- `api_keys` - tenant authentication with Argon2 hashing
- `api_key_audit_log` - 1-year retention for security compliance
- `webhook_events` - deduplication via unique(gateway_id, event_id, tenant_id)
- `webhook_retry_log` - retry attempt tracking for audits

**Financial Integrity**:
- All amounts stored as BIGINT (cents) for precision
- Currency-specific validation: IDR whole numbers, MYR/USD 2 decimals
- Proportional distribution formulas documented
- Rounding strategy: last installment absorbs difference
- Multi-tenant isolation via tenant_id query filtering

### API Endpoints (OpenAPI 3.0)

**Invoice Management** (6 endpoints):
- `POST /invoices` - Create invoice with optional installments
- `GET /invoices/{id}` - Retrieve invoice details
- `GET /invoices/{id}/payment-initiation` - Export ISO 20022 pain.001 XML
- `GET /invoices/{id}/installments` - Get installment schedule
- `GET /invoices/{id}/overpayment` - Get overpayment amount
- `POST /invoices/{id}/supplementary` - Create supplementary invoice

**Financial Reporting** (1 endpoint):
- `GET /reports/financial` - Revenue breakdown by currency/gateway/tax

**API Key Management** (4 endpoints):
- `POST /api-keys` - Generate new API key (admin-only)
- `PUT /api-keys/{id}/rotate` - Rotate API key (admin-only)
- `DELETE /api-keys/{id}` - Revoke API key (admin-only)
- `GET /api-keys/audit` - Query audit log (admin-only)

**Webhook Management** (1 endpoint):
- `GET /webhooks/failed` - List failed webhooks for manual recovery

### Architecture Decisions Documented

1. **Immutability Enforcement**: `payment_initiated_at` timestamp triggers immutability per FR-046
2. **Sequential Installments**: Service layer enforces installment order per FR-060/FR-061
3. **Webhook Deduplication**: Unique constraint on (gateway_id, event_id, tenant_id) per FR-040
4. **ISO 20022 Hybrid**: REST/JSON external API + internal pain.001/pain.002 models per FR-006a
5. **Rate Limiting**: Trait-based design supports in-memory (v1.0) and Redis (future)
6. **Multi-Tenant Isolation**: Query-level filtering with tenant_id propagation

### Testing Strategy

- Integration tests with real PostgreSQL per Constitution Principle I
- Rate limiter tests for sliding window behavior
- Webhook deduplication tests
- Currency conversion tests (IDR/MYR/USD)
- Installment calculation tests (proportional distribution)
- API key rotation and revocation tests

---

## Next Steps: Phase 2 Task Generation

Run `/speckit.tasks` to generate `tasks.md` with:
- Atomic implementation tasks (T001-T100+)
- Task dependencies and phases
- Estimated effort and complexity
- Quality gates and validation criteria
- Code review checkpoints

**Expected deliverables**: Implementation roadmap covering:
- Database migrations (schema setup)
- Core domain models and business logic
- API handlers and error handling
- Payment gateway adapters (Xendit/Midtrans)
- ISO 20022 payment initiation/status modules
- Authentication and authorization
- Rate limiting middleware
- Webhook processing and retry logic
- Financial reporting queries
- API key management
- Comprehensive integration test suite
