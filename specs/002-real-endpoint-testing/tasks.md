# Tasks: Real Endpoint Testing Infrastructure

**Feature Branch**: `002-real-endpoint-testing`  
**Input**: Design documents from `/specs/002-real-endpoint-testing/`  
**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/ ✅

**Tests**: This is a testing infrastructure feature. The "tests" are refactoring existing integration/contract tests to use real HTTP endpoints instead of direct database manipulation. All refactored tests MUST use real databases and real HTTP connections per Constitution Principle III.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and dependency configuration

- [x] T001 Update Cargo.toml - add actix-test = "0.1" to [dev-dependencies]
- [x] T002 Update Cargo.toml - remove mockito = "1.5" from [dev-dependencies]
- [x] T003 [P] Create config/.env.test.example with test environment variables template
- [x] T004 [P] Update config/.env.example - add test configuration section comment
- [x] T005 [P] Create scripts/setup_test_db.sh - automated test database creation and migration script
- [x] T006 Verify project builds with new dependencies via cargo build

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core test infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

**TDD Note**: Test refactoring tasks implicitly follow TDD workflow (Constitution Principle III) - each refactor verifies old test behavior fails with new requirements → implements new helper → verifies new test passes. For test infrastructure code (helpers), the refactored integration tests serve as the acceptance tests.

- [x] T007 Create tests/helpers/mod.rs - public exports for all test helper modules
- [x] T008 [P] Implement tests/helpers/test_database.rs - create_test_pool() function per contract
- [x] T009 [P] Implement tests/helpers/test_database.rs - with_transaction() function per contract
- [x] T010 [P] Implement tests/helpers/test_server.rs - spawn_test_server() function per contract
- [x] T011 [P] Implement tests/helpers/test_data.rs - TestDataFactory::random_external_id() per contract
- [x] T012 [P] Implement tests/helpers/test_data.rs - TestDataFactory::create_invoice_payload() per contract
- [x] T013 [P] Implement tests/helpers/assertions.rs - assert_success(), assert_created(), assert_bad_request() functions
- [x] T014 Update src/lib.rs - export test helpers for use in tests (pub mod if needed)
- [x] T015 Verify test helpers compile via cargo test --lib --no-run
- [x] T015a [P] Add error handling to tests/helpers/test_database.rs - connection failure, timeout, invalid credentials with clear error messages
- [x] T015b [P] Add error handling to tests/helpers/test_server.rs - port already in use, bind failure, server startup timeout
- [x] T015c [P] Add error handling to tests/helpers/test_client.rs - network timeout, connection refused, invalid response handling

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Developer Runs Integration Tests (Priority: P1) 🎯 MVP

**Goal**: Enable developers to run integration tests that use real HTTP endpoints and real database, catching production issues before deployment

**Independent Test**: Run `cargo test --test payment_flow_test` and verify test makes actual HTTP requests to test server with real MySQL database connection

### Implementation for User Story 1

- [ ] T016 [P] [US1] Implement tests/helpers/test_client.rs - HTTP client helper functions per contract
- [ ] T017 [P] [US1] Implement tests/helpers/gateway_sandbox.rs - XenditSandbox struct with new() and create_invoice() per contract
- [ ] T018 [P] [US1] Implement tests/helpers/gateway_sandbox.rs - MidtransSandbox struct with new() and charge() per contract
- [ ] T019 [P] [US1] Add TestFixtures constants to tests/helpers/test_data.rs (gateway IDs, API keys, test cards)
- [ ] T020 [US1] Refactor tests/integration/payment_flow_test.rs - replace direct DB queries with spawn_test_server() and HTTP requests
- [ ] T021 [US1] Update payment_flow_test.rs - use TestDataFactory for unique test data generation
- [ ] T022 [US1] Update payment_flow_test.rs - replace mockito gateway calls with XenditSandbox real API calls
- [ ] T023 [US1] Remove #[ignore] attribute from payment_flow_test.rs if environment configured
- [ ] T024 [US1] Verify refactored test: cargo test --test payment_flow_test
- [ ] T025 [US1] Refactor tests/contract/invoice_api_test.rs - use spawn_test_server() instead of direct DB
- [ ] T026 [US1] Verify contract test: cargo test --test invoice_api_test
- [ ] T027 [US1] Document refactoring pattern in tests/helpers/mod.rs with usage examples
- [ ] T028 [US1] Run full integration test suite: cargo test --test 'integration\_\*' (expect 1+ refactored tests to pass)

**Checkpoint**: At this point, developers can run refactored integration tests with real HTTP endpoints and real database. User Story 1 is fully functional and testable independently.

---

## Phase 4: User Story 2 - CI/CD Pipeline Validates Changes (Priority: P2)

**Goal**: Enable automated tests in CI/CD pipeline using real endpoints, validating pull requests before merging

**Independent Test**: Trigger GitHub Actions workflow and verify tests spin up test database, run all endpoint tests, and report results

### Implementation for User Story 2

- [ ] T029 [P] [US2] Create .github/workflows/test.yml - GitHub Actions workflow file
- [ ] T030 [US2] Configure MySQL service in test.yml - use mysql:8.0 image with health checks
- [ ] T031 [US2] Add test job steps in test.yml - checkout, setup Rust, run migrations, run tests
- [ ] T032 [US2] Configure environment variables in test.yml - TEST_DATABASE_URL, XENDIT_TEST_API_KEY secrets
- [ ] T033 [P] [US2] Create docker-compose.test.yml - MySQL test database service for local CI simulation
- [ ] T034 [US2] Add docker-compose test script to scripts/test_with_docker.sh
- [ ] T035 [US2] Test local CI simulation: docker-compose -f docker-compose.test.yml up -d && cargo test
- [ ] T036 [US2] Update scripts/seed_test_data.sh - add option to seed via environment variables
- [ ] T037 [US2] Document CI/CD setup in specs/002-real-endpoint-testing/quickstart.md CI/CD section
- [ ] T038 [US2] Create test PR to verify GitHub Actions workflow runs successfully

**Checkpoint**: At this point, CI/CD pipeline runs automated tests with real endpoints. User Stories 1 AND 2 both work independently.

---

## Phase 5: User Story 3 - Test Data Isolation (Priority: P3)

**Goal**: Enable parallel test execution with isolated test data, ensuring reliability and repeatability

**Independent Test**: Run multiple tests in parallel via `cargo test -- --test-threads=4` and verify no data conflicts occur

### Implementation for User Story 3

- [ ] T039 [P] [US3] Enhance tests/helpers/test_database.rs - add seed_isolated_gateway() function for per-test gateway setup
- [ ] T040 [P] [US3] Add transaction isolation example to tests/integration/tax_calculation_test.rs
- [ ] T041 [P] [US3] Refactor tests/integration/installment_flow_test.rs - use UUIDs and transactions for isolation
- [ ] T042 [P] [US3] Refactor tests/integration/gateway_validation_test.rs - use UUIDs and transactions for isolation
- [ ] T043 [US3] Add parallel test validation script to scripts/test_parallel.sh - runs tests 10 times in parallel
- [ ] T044 [US3] Document isolation patterns in tests/helpers/mod.rs with before/after examples
- [ ] T045 [US3] Verify repeatability: run scripts/test_parallel.sh and confirm 100% pass rate
- [ ] T046 [US3] Measure test suite performance: time cargo test and verify <60 seconds
- [ ] T047 [US3] Add test metrics collection to tests/helpers/test_server.rs - report HTTP p50/p95/p99 latency, test duration, DB query count to stdout in JSON format per FR-009
- [ ] T048 [US3] Document parallel execution best practices in specs/002-real-endpoint-testing/quickstart.md

**Checkpoint**: All user stories should now be independently functional with full parallel execution support.

---

## Phase 6: Full Migration & Polish

**Purpose**: Complete migration of remaining tests and cross-cutting improvements

### Complete Test Migration

- [ ] T049 [P] Refactor tests/integration/invoice_expiration_test.rs - use HTTP endpoints instead of DB
- [ ] T050 [P] Refactor tests/integration/service_fee_test.rs - use HTTP endpoints instead of DB
- [ ] T051 [P] Refactor tests/integration/report_generation_test.rs - use HTTP endpoints instead of DB
- [ ] T052 [P] Refactor tests/integration/metrics_collection_test.rs - use HTTP endpoints instead of DB
- [ ] T053 [P] Refactor tests/contract/installment_api_test.rs - use spawn_test_server()
- [ ] T054 [P] Refactor tests/contract/report_api_test.rs - use spawn_test_server()
- [ ] T055 [P] Refactor tests/contract/openapi_validation_test.rs - use spawn_test_server()
- [ ] T056 Refactor tests/integration/installment_adjustment_test.rs - use HTTP + gateway sandbox
- [ ] T057 Update tests/performance/invoice_creation_performance_test.rs - use real HTTP endpoints
- [ ] T058 Update tests/performance/load_test.rs - use spawn_test_server() for realistic load testing

### Gateway Integration

- [ ] T059 [P] Add Xendit webhook simulation to tests/helpers/gateway_sandbox.rs
- [ ] T060 [P] Add Midtrans webhook simulation to tests/helpers/gateway_sandbox.rs
- [ ] T061 Create tests/integration/webhook_handling_test.rs - test real webhook payloads from sandbox

### Documentation & Cleanup

- [ ] T062 [P] Update specs/002-real-endpoint-testing/quickstart.md - add troubleshooting for common issues
- [ ] T063 [P] Create TESTING.md in project root - guide for writing new tests with real endpoints
- [ ] T064 [P] Add test coverage report script to scripts/test_coverage.sh
- [ ] T065 Verify zero mockito usage: grep -r "mockito" tests/ (should find nothing)
- [ ] T066 Remove any remaining #[ignore] attributes from tests if environment is configured
- [ ] T067 Run full test suite: cargo test (all tests should pass)
- [ ] T068 Validate success criteria SC-001 through SC-008 from spec.md using validation checklist in tasks.md (manual verification)
- [ ] T069 Update .github/copilot-instructions.md - add testing best practices with real endpoints
- [ ] T070 Final validation: Run quickstart.md setup instructions from scratch to verify developer onboarding

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup (T001-T006) completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational (T007-T015c) completion
- **User Story 2 (Phase 4)**: Depends on Foundational (T007-T015c) completion - Can run in parallel with US1
- **User Story 3 (Phase 5)**: Depends on Foundational (T007-T015c) completion - Can run in parallel with US1/US2
- **Full Migration (Phase 6)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Independent of US1, but benefits from US1's refactored tests
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - Independent of US1/US2, but benefits from refactored tests

### Within Each User Story

#### US1 (Developer Testing):

- Test helpers (T016-T019) can run in parallel
- Refactoring tasks (T020-T026) sequential, each depends on previous
- Verification (T027-T028) at end

#### US2 (CI/CD):

- Workflow and Docker files (T029, T033) can run in parallel
- Configuration tasks (T030-T032, T034) sequential
- Testing (T035-T038) at end

#### US3 (Isolation):

- Refactoring tasks (T039-T042) can run in parallel
- Validation scripts (T043-T045) sequential
- Performance verification (T046-T048) at end

### Parallel Opportunities

- **Setup Phase**: T003, T004, T005 can run in parallel (different files)
- **Foundational Phase**: T008-T013, T015a-T015c can run in parallel (different files in tests/helpers/)
- **US1 Implementation**: T016-T019 can run in parallel (different helper files)
- **US2 Implementation**: T029 and T033 can run in parallel (workflow vs docker-compose)
- **US3 Implementation**: T039-T042 can run in parallel (different test files)
- **Full Migration**: T049-T060 can run in parallel (different test files)
- **Documentation**: T062-T064, T069 can run in parallel (different doc files)

---

## Parallel Example: Foundational Phase

```bash
# Launch all foundational helpers together:
Task T008: "Implement tests/helpers/test_database.rs - create_test_pool()"
Task T009: "Implement tests/helpers/test_database.rs - with_transaction()"
Task T010: "Implement tests/helpers/test_server.rs - spawn_test_server()"
Task T011: "Implement tests/helpers/test_data.rs - TestDataFactory::random_external_id()"
Task T012: "Implement tests/helpers/test_data.rs - TestDataFactory::create_invoice_payload()"
Task T013: "Implement tests/helpers/assertions.rs - assertion functions"

# All can work simultaneously on different files
```

---

## Parallel Example: User Story 1

```bash
# Launch all helper implementations together:
Task T016: "Implement tests/helpers/test_client.rs"
Task T017: "Implement tests/helpers/gateway_sandbox.rs - XenditSandbox"
Task T018: "Implement tests/helpers/gateway_sandbox.rs - MidtransSandbox"
Task T019: "Add TestFixtures to tests/helpers/test_data.rs"

# Then refactor tests sequentially to verify pattern works
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T006)
2. Complete Phase 2: Foundational (T007-T015c) - CRITICAL, blocks all stories
3. Complete Phase 3: User Story 1 (T016-T028)
4. **STOP and VALIDATE**: Run cargo test --test payment_flow_test and verify real HTTP + real DB
5. Deploy/demo if ready - developers can now use new testing infrastructure

### Incremental Delivery

1. Complete Setup + Foundational (T001-T015c) → Foundation ready
2. Add User Story 1 (T016-T028) → Test independently → **MVP READY** - local testing works
3. Add User Story 2 (T029-T038) → Test independently → CI/CD automation works
4. Add User Story 3 (T039-T048) → Test independently → Parallel execution optimized
5. Complete Full Migration (T049-T070) → All tests migrated to real endpoints

Each story adds value without breaking previous stories.

### Parallel Team Strategy

With multiple developers after Foundational phase completes:

- **Developer A**: User Story 1 (T016-T028) - Local testing infrastructure
- **Developer B**: User Story 2 (T029-T038) - CI/CD automation
- **Developer C**: User Story 3 (T039-T048) - Parallel execution patterns

Stories complete and integrate independently.

---

## Testing Validation Checklist

After each phase, verify:

### Setup Phase Validation:

- [ ] Cargo.toml has actix-test, no mockito
- [ ] .env.test.example exists with all required variables
- [ ] Project builds: cargo build succeeds

### Foundational Phase Validation:

- [ ] All helper modules compile: cargo test --lib --no-run
- [ ] create_test_pool() connects to test database
- [ ] spawn_test_server() starts on available port
- [ ] TestDataFactory generates unique IDs
- [ ] Error handling works: invalid DB connection fails gracefully, port conflicts handled, network timeouts caught

### User Story 1 Validation:

- [ ] payment_flow_test.rs uses real HTTP requests
- [ ] Test connects to real MySQL database
- [ ] Gateway calls use sandbox APIs (no mockito)
- [ ] Test passes: cargo test --test payment_flow_test

### User Story 2 Validation:

- [ ] GitHub Actions workflow file exists
- [ ] Workflow starts MySQL service
- [ ] Tests run in CI/CD successfully
- [ ] Results reported in pull request

### User Story 3 Validation:

- [ ] Tests run in parallel without conflicts
- [ ] 10 consecutive runs all pass (repeatability)
- [ ] Test suite completes in <60 seconds
- [ ] Parallel execution: cargo test -- --test-threads=4

### Full Migration Validation (Success Criteria from spec.md):

- [ ] **SC-001**: All integration tests make real HTTP requests (no mocked HTTP)
- [ ] **SC-002**: All DB operations connect to real MySQL (verify logs show paytrust_test)
- [ ] **SC-003**: 100% pass rate across 10 consecutive runs
- [ ] **SC-004**: Test suite completes within 60 seconds
- [ ] **SC-005**: At least 1 integration issue found during refactoring (e.g., DB constraint, HTTP timeout, API format mismatch)
- [ ] **SC-006**: Zero mockito usage (grep -r "mockito" tests/ finds nothing)
- [ ] **SC-007**: All 15+ API endpoints covered (run openapi_validation_test)
- [ ] **SC-008**: CI/CD pipeline runs successfully (check GitHub Actions)

---

## Notes

- **[P] tasks** = different files, can work in parallel
- **[Story] label** maps task to specific user story for traceability
- Each user story independently completable and testable after Foundational phase
- TDD workflow: For refactored tests, ensure old test fails → refactor → new test passes
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- All integration/contract tests MUST use real HTTP, real database (Constitution Principle III)
- Unit tests (tests/unit/) unchanged - mocks still permitted for isolated business logic

---

## Summary

**Total Tasks**: 73  
**Setup**: 6 tasks  
**Foundational** (blocks all): 12 tasks (includes 3 error handling tasks)  
**User Story 1** (P1): 13 tasks  
**User Story 2** (P2): 10 tasks  
**User Story 3** (P3): 10 tasks  
**Full Migration**: 22 tasks

**Parallel Opportunities**: 38+ tasks marked [P] can run in parallel  
**MVP Scope**: Phases 1-3 (31 tasks) delivers working local test infrastructure  
**Full Feature**: All 73 tasks delivers complete real endpoint testing with CI/CD

**Independent Test Criteria**:

- US1: Run refactored test with real HTTP endpoint
- US2: Trigger CI/CD and verify automated test execution
- US3: Run tests in parallel and verify no conflicts
