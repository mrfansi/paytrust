<!--
Sync Impact Report:
- Version change: 1.0.0 → 1.1.0
- Added principles: Context7 MCP Documentation (new VI)
- Modified principles: None (existing principles unchanged)
- Added sections: None
- Removed sections: None
- Templates updated: ✅ Updated (this constitution aligns with existing templates)
- Follow-up TODOs: None
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

### III. Test-First Development (NON-NEGOTIABLE)

TDD mandatory: Tests written → User approved → Tests fail → Then implement. Red-Green-Refactor
cycle strictly enforced. Unit tests for all business logic, integration tests for database
operations, contract tests for API endpoints. No code merges without corresponding test coverage.

**Rationale**: Guarantees specification compliance, prevents regressions, enables confident
refactoring, and serves as executable documentation of system behavior.

### IV. MySQL Integration Standards

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

## Technology Stack Constraints

**Language**: Rust 1.75+ with 2021 edition features
**Database**: MySQL 8.0+ with InnoDB storage engine  
**Environment**: dotenv pattern for configuration management
**Testing**: Built-in cargo test with custom test harnesses for integration tests
**Architecture**: Repository pattern with trait-based abstractions
**Error Handling**: Result<T, E> types with custom error enums, no panics in business logic
**Logging**: Structured logging with configurable levels per environment

## Development Standards

**Code Organization**: src/ for application code, tests/ for integration tests, migrations/
for database schema changes, config/ for environment templates
**Naming Conventions**: snake_case for variables/functions, PascalCase for types/traits,
SCREAMING_SNAKE_CASE for constants
**Documentation**: All public APIs MUST have rustdoc comments with examples
**Performance**: Database queries MUST be indexed appropriately, N+1 query patterns prohibited
**Security**: Input validation at service boundaries, output sanitization, secure connection handling

## Governance

Constitution supersedes all other development practices. All pull requests MUST verify
compliance with constitutional principles. Architectural complexity MUST be justified
against simpler alternatives. Principle violations require documented approval with
migration plan to eventual compliance.

Amendment procedure: Proposed changes require documentation of impact, approval from
technical leadership, and update of all dependent templates and guidance documents.

**Version**: 1.1.0 | **Ratified**: 2025-11-01 | **Last Amended**: 2025-11-01
