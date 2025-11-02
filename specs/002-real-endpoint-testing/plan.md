# Implementation Plan: Real Endpoint Testing Infrastructure

**Branch**: `002-real-endpoint-testing` | **Date**: November 2, 2025 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-real-endpoint-testing/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Transform PayTrust's testing infrastructure to use real HTTP endpoint testing instead of direct database manipulation and mocked gateway calls. This feature adds test server infrastructure using actix-test, replaces mockito with real payment gateway sandbox API calls, implements proper test data isolation, and ensures all integration tests make actual HTTP requests to validate production behavior.

## Technical Context

**Language/Version**: Rust 1.91.0 with 2021 edition  
**Primary Dependencies**: actix-web 4.9, actix-test (to be added), tokio 1.40, sqlx 0.8, reqwest 0.12  
**Storage**: MySQL 8.0+ test database (paytrust_test)  
**Testing**: cargo test with tokio::test runtime, actix-test for HTTP server testing  
**Target Platform**: Linux/macOS server development and CI/CD environments
**Project Type**: Single Rust project with web API backend  
**Performance Goals**: Test suite completes in <60 seconds, supports parallel test execution  
**Constraints**: Tests must use real database connections, real HTTP requests, real gateway sandbox APIs  
**Scale/Scope**: ~15 integration tests covering all API endpoints, ~10 contract tests, test coverage for 15+ endpoints

**Current State Analysis**:

- ✅ Integration tests connect to real MySQL database via DATABASE_URL env var
- ❌ Tests directly manipulate database instead of making HTTP requests to endpoints
- ❌ mockito used in dev-dependencies for gateway mocking (violates Constitution Principle III)
- ❌ No test server infrastructure (actix-test not configured)
- ✅ Test database cleanup patterns exist (manual cleanup functions)
- ⚠️ Tests are marked `#[ignore]` requiring manual database configuration

**Gaps to Address**:

1. Add actix-test for spawning real HTTP test server
2. Refactor tests to use reqwest HTTP client instead of direct database queries
3. Remove mockito dependency and use real gateway sandbox APIs
4. Implement automatic test database setup/teardown
5. Add test environment configuration (.env.test)
6. Implement test data isolation patterns (unique IDs, transactions, or separate schemas)

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

### Principle I: Standard Library First

**Status**: ✅ **PASS**  
**Analysis**: Using cargo test (built-in), tokio::test (already in dependencies), and standard Rust testing patterns. New dependency actix-test is justified as it's required for HTTP server testing in actix-web ecosystem (no std library alternative for spawning test HTTP servers).

### Principle II: SOLID Architecture

**Status**: ✅ **PASS**  
**Analysis**: Test infrastructure follows existing repository and service patterns. Test helpers will use dependency injection for database pools and HTTP clients. No architectural violations introduced.

### Principle III: Test-First Development with Real Testing

**Status**: ⚠️ **CONDITIONAL PASS with VIOLATION to RESOLVE**  
**Current Violation**: mockito present in dev-dependencies, used for gateway mocking in integration tests  
**Required Action**: Remove mockito and replace with real payment gateway sandbox API calls (Xendit test mode, Midtrans sandbox)  
**Justification for Real Testing**: This feature directly implements Constitution Principle III real testing requirement by eliminating mocks and using actual HTTP endpoints and gateway APIs

### Principle IV: MySQL Integration Standards

**Status**: ✅ **PASS**  
**Analysis**: Tests already use real MySQL via sqlx connection pools. Will continue using same patterns with proper transaction management for test isolation.

### Principle V: Environment Management

**Status**: ✅ **PASS**  
**Analysis**: Tests use DATABASE_URL environment variable. Will add .env.test for test-specific configuration (test database, test ports, sandbox API keys).

### Principle VI: Context7 MCP Documentation

**Status**: ✅ **PASS**  
**Analysis**: Will use Context7 MCP to fetch actix-test documentation and payment gateway sandbox API documentation during implementation.

### Principle VII: Modular Architecture

**Status**: ✅ **PASS**  
**Analysis**: Test infrastructure organized in tests/ directory with clear separation: contract/, integration/, unit/, performance/. Test helpers will be in tests/helpers/ module.

**Overall Gate Status**: ✅ **PASS WITH ACTION ITEM**  
**Required Before Phase 0**: Document plan to remove mockito and use real gateway sandbox APIs  
**Action**: Phase 0 research must identify Xendit and Midtrans sandbox API patterns and credentials management

---

## Constitution Check (Post-Phase 1 Re-evaluation)

_Re-checked after Phase 1 design completion_

### Principle I: Standard Library First

**Status**: ✅ **PASS**  
**Validation**: actix-test dependency justified in research.md - required for HTTP server testing with no std alternative

### Principle II: SOLID Architecture

**Status**: ✅ **PASS**  
**Validation**: Test helpers organized in modular structure (tests/helpers/), each module has single responsibility, uses trait-based abstractions where needed

### Principle III: Test-First Development with Real Testing

**Status**: ✅ **PASS (POST-IMPLEMENTATION)**  
**Resolution**: mockito removal documented in research.md, replacement with XenditSandbox/MidtransSandbox for real API calls designed in contracts/test-infrastructure-api.md

### Principle IV: MySQL Integration Standards

**Status**: ✅ **PASS**  
**Validation**: Transaction-based test isolation pattern documented in data-model.md, maintains connection pooling, no hardcoded credentials

### Principle V: Environment Management

**Status**: ✅ **PASS**  
**Validation**: .env.test pattern documented in quickstart.md, all credentials in environment variables, clear separation from production config

### Principle VI: Context7 MCP Documentation

**Status**: ✅ **PASS**  
**Validation**: Used Context7 MCP to fetch actix-test and Xendit API documentation during research phase

### Principle VII: Modular Architecture

**Status**: ✅ **PASS**  
**Validation**: tests/helpers/ module structure documented in data-model.md, clear separation of concerns (server, database, data, gateway, assertions)

**Final Gate Status**: ✅ **ALL PRINCIPLES SATISFIED**  
**Ready for Phase 2**: Implementation can proceed with confidence in constitutional compliance

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
├── modules/              # Business logic modules (unchanged)
│   ├── invoices/
│   ├── installments/
│   ├── gateways/
│   ├── reports/
│   └── taxes/
├── core/                 # Shared core functionality (unchanged)
│   ├── traits/
│   ├── error.rs
│   └── currency.rs
├── config/               # Configuration (unchanged)
│   ├── database.rs
│   └── server.rs
├── middleware/           # HTTP middleware (unchanged)
│   ├── auth.rs
│   ├── rate_limit.rs
│   └── metrics.rs
├── lib.rs                # Library exports (MODIFIED - add test helpers)
└── main.rs               # Application entry point (unchanged)

tests/
├── helpers/              # NEW - Shared test infrastructure
│   ├── mod.rs            # Public exports
│   ├── test_server.rs    # Test server spawning with actix-test
│   ├── test_client.rs    # HTTP client helpers with reqwest
│   ├── test_database.rs  # Database setup/cleanup helpers
│   ├── test_data.rs      # Test data factories and seeds
│   └── gateway_sandbox.rs # Gateway sandbox API helpers
├── contract/             # REFACTORED - Use HTTP client instead of DB
│   ├── invoice_api_test.rs
│   ├── installment_api_test.rs
│   ├── report_api_test.rs
│   └── openapi_validation_test.rs
├── integration/          # REFACTORED - Use HTTP client + real gateway
│   ├── payment_flow_test.rs
│   ├── gateway_validation_test.rs
│   ├── installment_flow_test.rs
│   ├── invoice_expiration_test.rs
│   ├── tax_calculation_test.rs
│   ├── service_fee_test.rs
│   ├── report_generation_test.rs
│   └── metrics_collection_test.rs
├── unit/                 # UNCHANGED - Unit tests still allowed mocks
│   ├── tax_calculator_test.rs
│   ├── service_fee_test.rs
│   ├── installment_calculator_test.rs
│   └── installment_overpayment_test.rs
└── performance/          # ENHANCED - Add real HTTP load testing
    ├── invoice_creation_performance_test.rs
    └── load_test.rs

config/
├── .env.example          # MODIFIED - Add test configuration section
└── .env.test.example     # NEW - Test environment template

scripts/
├── setup_test_db.sh      # NEW - Automated test database setup
├── seed_test_data.sh     # EXISTING - May need sandbox API keys
└── test_endpoints.sh     # EXISTING - Reference for test patterns

Cargo.toml                # MODIFIED - Add actix-test, remove mockito
```

**Structure Decision**: Single Rust project structure. Tests are organized by type (contract, integration, unit, performance) with a new `helpers/` module containing shared test infrastructure. The key change is adding `tests/helpers/` for test server spawning, HTTP client helpers, and gateway sandbox integration, while refactoring existing integration and contract tests to use HTTP requests instead of direct database manipulation.

## Complexity Tracking

**Status**: No complexity violations to justify. This feature simplifies the testing architecture by removing mocking complexity and aligning with Constitution principles.

---

## Phase 0-1 Completion Summary

### Generated Artifacts

✅ **Phase 0: Research** (`research.md`)

- Test server infrastructure decision (actix-test)
- HTTP client strategy (TestServer + reqwest fallback)
- Database management pattern (transactions for isolation)
- Payment gateway sandbox API integration (Xendit, Midtrans)
- Test data isolation strategy (UUIDs + transactions)
- CI/CD integration approach (Docker Compose + GitHub Actions)
- Performance optimization (parallel execution)

✅ **Phase 1: Design**

- `data-model.md` - Test infrastructure entities (TestServer, TestDatabase, TestClient, GatewayTestConfig, TestDataFactory, TestFixtures, TestAssertions)
- `quickstart.md` - Complete setup and usage guide with troubleshooting
- `contracts/test-infrastructure-api.md` - Rust module interface contracts for test helpers
- Agent context updated (GitHub Copilot instructions)

✅ **Constitution Check**

- Initial evaluation: PASS with action item (remove mockito)
- Post-design re-evaluation: ALL PRINCIPLES SATISFIED

### Key Decisions Made

1. **Test Server**: actix-test with `actix_test::start()` for real HTTP server
2. **Test Isolation**: Database transactions + UUID-based unique IDs
3. **Gateway Testing**: Real sandbox APIs (Xendit test mode, Midtrans sandbox)
4. **Database**: Separate `paytrust_test` database with transaction rollback
5. **Dependencies**: Add actix-test, remove mockito
6. **CI/CD**: Docker Compose for MySQL, GitHub Actions for automation

### Next Steps (Phase 2)

Phase 2 will be handled by `/speckit.tasks` command (NOT created by this plan):

1. Create implementation tasks for test infrastructure
2. Break down into testable units (per Constitution TDD workflow)
3. Prioritize by dependency order (helpers → refactor tests → CI/CD)

---

## Implementation Readiness

**Status**: ✅ **READY FOR IMPLEMENTATION**

All prerequisites complete:

- [x] Research completed with all NEEDS CLARIFICATION resolved
- [x] Data model defined for test infrastructure
- [x] Contracts specified for test helper modules
- [x] Quickstart guide created for developer onboarding
- [x] Constitution compliance verified (all 7 principles)
- [x] Agent context updated for AI assistance

**Branch**: `002-real-endpoint-testing` (checked out)  
**Next Command**: `/speckit.tasks` to generate implementation task breakdown
