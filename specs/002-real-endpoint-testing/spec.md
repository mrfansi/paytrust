# Feature Specification: Real Endpoint Testing Infrastructure

**Feature Branch**: `002-real-endpoint-testing`  
**Created**: November 2, 2025  
**Status**: Draft  
**Input**: User description: "testing using real endpoints instead of mocking to prevent any issue in production"

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Developer Runs Integration Tests (Priority: P1)

As a developer, I need to run integration tests that interact with real HTTP endpoints and a real database, so that I can catch production issues during development before they reach users.

**Why this priority**: This is the foundation of the testing strategy. Without real endpoint testing, the team cannot confidently deploy to production knowing the system works as a whole. This directly prevents production incidents.

**Independent Test**: Can be fully tested by running `cargo test --test integration_*` and verifying that tests make actual HTTP requests to a running test server with a real MySQL database, and delivers confirmation that endpoints work end-to-end.

**Acceptance Scenarios**:

1. **Given** a test database is available and server is running, **When** developer runs integration tests, **Then** tests make real HTTP POST/GET/PUT/DELETE requests to actual endpoints
2. **Given** integration tests are executing, **When** tests need data, **Then** data is created in the real test database (not mocked)
3. **Given** an integration test fails, **When** developer reviews logs, **Then** logs show actual HTTP response codes and database query results (not mock behavior)
4. **Given** tests complete successfully, **When** developer checks database, **Then** test data is cleaned up properly

---

### User Story 2 - CI/CD Pipeline Validates Changes (Priority: P2)

As a DevOps engineer, I need automated tests to run in the CI/CD pipeline using real endpoints, so that pull requests are validated against actual system behavior before merging.

**Why this priority**: Automated validation in CI/CD prevents broken code from reaching main branch. This is critical for team velocity and production stability, but depends on P1 (local testing) being functional first.

**Independent Test**: Can be fully tested by triggering a CI/CD build, verifying that tests spin up a test database and server, run all endpoint tests, and report pass/fail status to the pipeline.

**Acceptance Scenarios**:

1. **Given** a pull request is opened, **When** CI/CD pipeline runs, **Then** tests start a test database and application server automatically
2. **Given** CI/CD tests are running, **When** tests interact with endpoints, **Then** all HTTP calls use real network requests (no mocking)
3. **Given** CI/CD tests complete, **When** pipeline reports results, **Then** results show actual endpoint response times and success rates
4. **Given** tests fail in CI/CD, **When** developer reviews logs, **Then** logs contain actual HTTP responses and database states

---

### User Story 3 - Test Data Isolation (Priority: P3)

As a developer, I need tests to use isolated test data that doesn't interfere with other tests or development databases, so that tests are reliable and repeatable.

**Why this priority**: Test isolation ensures reliability and enables parallel test execution. This improves developer experience but is less critical than having working tests (P1) and automated validation (P2).

**Independent Test**: Can be fully tested by running multiple tests in parallel and verifying each test uses unique IDs, separate database transactions, or separate test database schemas without data conflicts.

**Acceptance Scenarios**:

1. **Given** multiple tests run in parallel, **When** each test creates invoice data, **Then** tests use unique identifiers (UUIDs or sequential IDs) that don't conflict
2. **Given** a test creates data, **When** test completes or fails, **Then** test data is cleaned up automatically (via database transactions or teardown)
3. **Given** a test needs gateway configuration, **When** test starts, **Then** test seeds its own gateway data isolated from other tests
4. **Given** tests run repeatedly, **When** developer runs tests multiple times, **Then** each run produces identical results (idempotent)

---

### Edge Cases

- What happens when the test database is unavailable or connection fails?
- How does the system handle network timeouts during HTTP requests in tests?
- What happens when test cleanup fails and leaves orphaned data?
- How are tests isolated when running in parallel (race conditions)?
- What happens when the test server fails to start or crashes during tests?
- How do tests handle different database states (empty vs. pre-populated)?

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: Test framework MUST start an actual HTTP server on a test port before running integration tests
- **FR-002**: Test framework MUST connect to a real MySQL test database (not in-memory or mocked)
- **FR-003**: Integration tests MUST make actual HTTP requests using a real HTTP client (e.g., reqwest)
- **FR-004**: Tests MUST create, read, update, and delete data in the real test database
- **FR-005**: Tests MUST validate actual HTTP response codes, headers, and body content
- **FR-006**: Test framework MUST provide database cleanup mechanisms (transactions or teardown scripts)
- **FR-007**: Tests MUST validate gateway integration behavior by calling real payment gateway sandbox/test APIs (Xendit test mode, Midtrans sandbox) with valid test credentials
- **FR-008**: Test framework MUST support parallel test execution with data isolation
- **FR-009**: Tests MUST report actual performance metrics (response times, database query times)
- **FR-010**: Test framework MUST provide seed data scripts for common test scenarios
- **FR-011**: Tests MUST validate all API endpoints defined in OpenAPI spec
- **FR-012**: Test framework MUST fail fast with clear error messages when infrastructure is unavailable

### Key Entities

- **Test Database**: Separate MySQL database instance for testing with same schema as production
- **Test Server**: HTTP server instance running on a test port (e.g., 8081) for integration tests
- **Test Data Seeds**: Pre-defined data sets (gateways, API keys, sample invoices) for test scenarios
- **Test HTTP Client**: Real HTTP client configured to make requests to test server
- **Test Results**: Actual response data, status codes, timings, and database states captured during tests

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: All integration tests make real HTTP requests without any mocked HTTP responses - verified by network monitoring or HTTP client logs
- **SC-002**: All database operations in tests connect to a real MySQL instance - verified by database connection logs showing test database name
- **SC-003**: Tests can run repeatedly with identical results - 100% pass rate across 10 consecutive runs with same initial state
- **SC-004**: Test suite completes within 60 seconds for full integration test run - measured from test start to finish
- **SC-005**: Tests catch actual production issues - demonstrated by at least 3 real bugs found during development that would have reached production with mocked tests
- **SC-006**: Zero mock libraries used in integration tests - verified by code review showing no usage of mockito, mockall, or similar mocking frameworks in integration test files
- **SC-007**: Test coverage includes all 15+ API endpoints - verified by OpenAPI contract tests validating every endpoint in the spec
- **SC-008**: CI/CD pipeline runs tests successfully with real endpoints - verified by successful test execution in automated builds without manual intervention
