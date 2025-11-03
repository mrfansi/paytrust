<!--
Sync Impact Report:
- Version change: 1.4.0 → 1.4.1
- Added principles: None
- Modified principles: VIII. Database Optimization & Naming Consistency (enhanced with UUID/ULID primary key requirement)
- Added sections: None
- Removed sections: None
- Templates updated: ✅ No template updates required (primary key strategy is implementation detail under existing principle)
- Follow-up TODOs: None
- Rationale: PATCH bump - primary key strategy refined to mandate UUID or ULID instead of auto-incrementing bigint for security, scalability, and distributed system compatibility (backward compatible, improves existing optimization principle)
-->

# PayTrust Constitution

## Core Principles

### I. Standard Library First

Rust standard library MUST be preferred over third-party dependencies. External crates are
permitted only when standard library lacks essential functionality (e.g., database drivers,
serialization). Every external dependency MUST be explicitly justified in documentation with
alternatives considered and specific limitations of std library that necessitate the choice.

**Rationale**: Reduces dependency bloat, improves security posture, ensures long-term stability,
and maintains predictable performance characteristics.

### II. SOLID Architecture (NON-NEGOTIABLE)

All code MUST strictly adhere to SOLID principles: Single Responsibility (modules have one
purpose), Open/Closed (extensible without modification), Liskov Substitution (trait
implementations are interchangeable), Interface Segregation (focused trait definitions),
Dependency Inversion (depend on abstractions via traits). Architecture violations MUST be
documented and approved before implementation.

**Rationale**: Ensures maintainable, testable, and extensible codebase that can evolve with
business requirements while minimizing technical debt.

### III. Test-First Development with Real Testing (NON-NEGOTIABLE)

TDD mandatory: Tests written → User approved → Tests fail → Then implement. Red-Green-Refactor
cycle strictly enforced. Unit tests for all business logic, integration tests for database
operations, contract tests for API endpoints. No code merges without corresponding test coverage.

**Real Testing Requirement**: Integration and contract tests MUST use real databases, real HTTP
connections, and real external service integrations (test environments). Mocks and stubs are
PROHIBITED for validation tests that verify production behavior. Mocking is permitted ONLY for
unit tests of isolated business logic or when external service test environments are unavailable.
Database tests MUST connect to actual test database instances, not in-memory simulations.

**Rationale**: Guarantees specification compliance, prevents regressions, enables confident
refactoring, and serves as executable documentation of system behavior. Real testing prevents
production incidents by validating against actual infrastructure behavior, not simulated
conditions. Mocked tests often pass while production fails due to untested edge cases in real
systems (database constraints, network timeouts, transaction isolation levels).

### IV. PostgreSQL Integration Standards

All database interactions MUST use connection pooling, prepared statements, and transaction
management. Database schema changes MUST use migrations with rollback capabilities. Connection
strings and credentials MUST be environment-configured, never hardcoded. Database operations
MUST be wrapped in repository pattern implementing defined traits.

**Rationale**: Ensures data integrity, prevents SQL injection, enables horizontal scaling,
and maintains environment separation for secure deployment.

### V. Environment Management (Laravel-Style)

Configuration MUST be environment-driven using .env files with .env.example templates.
All environment variables MUST have sensible defaults in code. Configuration loading MUST
happen at application startup with validation. Support for development, testing, staging,
and production environments with clear separation of concerns.

**Rationale**: Enables seamless deployment across environments, prevents configuration drift,
supports secure credential management, and maintains development productivity.

### VI. Context7 MCP Documentation (NON-NEGOTIABLE)

All documentation lookups, API references, and library integrations MUST use Context7 MCP
to fetch up-to-date, version-specific documentation directly from source. No reliance on
potentially outdated cached documentation or hallucinated API information. Context7 MCP
MUST be configured in development environment and used for all external library research,
integration guidance, and code examples.

**Rationale**: Eliminates outdated documentation issues, prevents integration errors from
deprecated APIs, ensures access to latest features and security updates, and maintains
accuracy of external library usage patterns.

### VII. Modular Architecture

System MUST be organized into loosely-coupled, independently-deployable modules with clear
boundaries and interfaces. Each module MUST have a single well-defined responsibility and
communicate through explicit contracts (traits/interfaces). Modules MUST NOT have circular
dependencies. Cross-module communication MUST occur via dependency injection, not direct
imports of implementation details. Module boundaries MUST align with business domains.

**Rationale**: Enables parallel development by multiple teams, facilitates independent testing
and deployment, reduces blast radius of changes, improves code navigation and understanding,
and allows selective scaling of system components based on load characteristics.

### VIII. Database Optimization & Naming Consistency

All database schemas MUST follow strict naming conventions for consistency, searchability, and
maintainability. Table names MUST be lowercase with underscores (snake_case), plural form
(e.g., `users`, `payment_transactions`, `account_balances`). Column names MUST be lowercase
with underscores, descriptive and non-abbreviated (e.g., `created_at`, `user_id`, `transaction_amount`).
Primary keys MUST be named `id` and MUST use UUID v7 or ULID data types—auto-incrementing bigint
is PROHIBITED. UUID is preferred for standard SQL compatibility; ULID may be used when sortable
timestamp-based identifiers are beneficial. Foreign keys MUST be named `[table_name]_id`
(e.g., `user_id`, `payment_id`). Timestamp columns MUST use `created_at` and `updated_at` for tracking.

All schemas MUST be optimized for query performance: appropriate indexes on foreign keys, frequently
queried columns, and filter conditions. Composite indexes MUST be justified and documented.
N+1 query patterns MUST be prevented through eager loading, query optimization, and batch operations.
Query performance MUST be validated during code review with execution plans reviewed for scans.
Column selection MUST be explicit—SELECT * is PROHIBITED except in internal utilities.

**Rationale**: Consistent naming ensures code readability, enables rapid schema navigation, reduces
bugs from typos and inconsistencies, facilitates cross-team collaboration, and makes migrations
maintainable. UUID/ULID primary keys prevent ID enumeration attacks, eliminate distributed system
synchronization complexity, support sharding without coordination, and provide better privacy for
resource identifiers. Performance optimization prevents production incidents, reduces operational
costs, and ensures responsive user experiences. Query review gates catch inefficient patterns before
they reach production where optimization is exponentially more costly.

## Technology Stack Constraints

**Language**: Rust 1.91+ with 2021 edition features
**Database**: PostgreSQL 13.0+ with ACID guarantees and native JSON support
**Environment**: dotenv pattern for configuration management
**Testing**: Built-in cargo test with custom test harnesses for integration tests
**Architecture**: Repository pattern with trait-based abstractions
**Error Handling**: Result<T, E> types with custom error enums, no panics in business logic
**Logging**: Structured logging with configurable levels per environment

## Development Standards

**Code Organization**: Modular structure with src/modules/ containing domain-bounded modules,
each with its own models/, services/, repositories/, and controllers/. Shared code in src/core/.
Integration tests in tests/, migrations in migrations/, config in config/. No markdown summary
files or changelog generation required - code and tests are the documentation.

**Naming Conventions**: snake_case for variables/functions, PascalCase for types/traits,
SCREAMING_SNAKE_CASE for constants

**Documentation**: Public APIs MUST have rustdoc comments with examples. Module boundaries and
contracts documented via trait definitions. No separate summary documentation files required.

**Database Naming Standards**: Table names lowercase plural (e.g., `users`, `orders`), column names
lowercase with underscores (e.g., `user_id`, `created_at`). Primary keys: `id` column with UUID v7
or ULID type (e.g., `id UUID PRIMARY KEY DEFAULT gen_random_uuid()` or `id ULID PRIMARY KEY`).
Auto-incrementing bigint MUST NOT be used for primary keys. Foreign keys: `[referenced_table]_id`
(e.g., `user_id` references `users` table). Timestamps: `created_at`, `updated_at` for all temporal
tracking. Status fields: `status` as single column with constraint (ENUM or CHECK), not boolean flags.
Indexes documented in migration files with rationale. No abbreviated column names; readability
prioritized over brevity (e.g., `phone_number` not `ph_num`).

**Performance**: Database queries MUST be indexed appropriately, N+1 query patterns prohibited,
execution plans reviewed for full table scans, explicit column selection required (no SELECT *
in production code). Migrations MUST include index creation for foreign keys and frequently
queried columns. Query optimization MUST be validated during code review.

**Security**: Input validation at service boundaries, output sanitization, secure connection handling,
prepared statements mandatory, no dynamic query construction

**Module Structure**: Each module exports clear public interface via mod.rs, internal implementation
details remain private. Dependencies injected via trait objects, never concrete types.

## Governance

Constitution supersedes all other development practices. All pull requests MUST verify
compliance with constitutional principles. Architectural complexity MUST be justified
against simpler alternatives. Principle violations require documented approval with
migration plan to eventual compliance.

Amendment procedure: Proposed changes require documentation of impact, approval from
technical leadership, and update of all dependent templates and guidance documents.

**Version**: 1.4.1 | **Ratified**: 2025-11-01 | **Last Amended**: 2025-11-03
