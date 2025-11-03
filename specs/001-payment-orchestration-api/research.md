# Phase 0 Research: PayTrust Payment Orchestration API

**Status**: IN PROGRESS
**Generated**: 2025-11-03
**Purpose**: Resolve technical unknowns and document architecture decisions before Phase 1 design

---

## Research Tasks

### 1. Rust Actix-web Best Practices for Payment APIs

**Query**: actix-web 4.9+ best practices for RESTful payment APIs with PostgreSQL integration

**Decision**:
- Use actix-web 4.9+ with tokio async runtime
- sqlx for compile-time verified SQL queries
- serde for JSON serialization/deserialization
- Implement middleware for API key authentication and rate limiting

**Rationale**:
- actix-web proven for high-performance REST APIs (addresses NFR-001, NFR-002)
- sqlx compile-time checking prevents SQL injection and runtime errors
- tokio ecosystem mature for async payment processing workflows

**Alternatives Considered**:
- Axum: More modern but newer ecosystem; actix-web has longer track record for production payment systems
- Rocket: Requires nightly Rust; actix-web stable and mature

---

### 2. PostgreSQL Schema Design for Payment Invoices

**Query**: PostgreSQL design patterns for invoice management with installment tracking, multi-currency isolation, and audit trails

**Decision**:
- Core tables: invoices, line_items, payment_transactions, installment_schedules
- Supplementary tables: payment_gateways, api_keys, gateway_configurations, webhook_events, webhook_retry_log
- Audit tables: api_key_audit_log
- All currency amounts stored as BIGINT (cents for IDR, 2-decimal multiplied by 100 for MYR/USD) to ensure numeric precision
- TIMESTAMP WITHOUT TIME ZONE for UTC storage per FR-073
- BIGSERIAL for gateway_id to support large integer foreign keys per FR-007a
- Foreign key constraints with CASCADE/RESTRICT per referential integrity requirements
- Unique constraint on (gateway_name, event_id, tenant_id) for webhook deduplication per FR-040

**Rationale**:
- BIGINT storage prevents floating-point arithmetic errors (addresses NFR-005)
- Normalized schema with proper relationships supports complex queries (financial reports, installment validation)
- Audit tables provide compliance trail for FR-033, FR-071
- Webhook deduplication prevents duplicate processing per FR-040

**Alternatives Considered**:
- MongoDB: Document model complicates installment calculations requiring exact numeric precision
- DynamoDB: Limited query flexibility for financial reports; worse for ACID transactions

---

### 3. ISO 20022 Payment Initiation (pain.001) Implementation

**Query**: ISO 20022 pain.001 implementation patterns for REST/JSON API with internal XML model conversion

**Decision**:
- Maintain REST/JSON API as external developer interface (no breaking changes)
- Internal Invoice domain model maps to ISO 20022 pain.001 structures:
  - invoice_id → MessageID
  - external_id → ReferenceID (optional)
  - created_at → CreationDateTime
  - currency (ISO 4217) → Currency
  - total_amount → Amount
  - tenant_id → InitiatingParty identifier
  - gateway name → ServiceProvider
- Implement pain.001 XML generation on-demand via GET /invoices/{id}/payment-initiation endpoint
- Use pain.001 XSD schema validation library (xml-rs or similar) to ensure compliance
- Installment_schedule maps to multiple PaymentInstruction elements within single pain.001 message

**Rationale**:
- Hybrid approach balances developer usability (REST/JSON) with regulatory compliance (internal ISO 20022)
- On-demand generation avoids bloating invoice responses while providing compliance capability
- Addresses NFR-012 ISO 20022 compliance requirement without changing external API

**Alternatives Considered**:
- All XML API: Breaks developer usability; increases complexity for payment gateway integration
- JSON-LD: Adds representation layer without addressing payment standard compliance
- No internal ISO 20022: Violates NFR-012 compliance requirement

---

### 4. ISO 20022 Payment Status (pain.002) Webhook Adapter Pattern

**Query**: ISO 20022 pain.002 webhook adapter pattern for converting gateway-native payloads (Xendit JSON, Midtrans JSON/XML) to standardized pain.002 structures

**Decision**:
- Webhook reception accepts gateway-native format (Xendit/Midtrans JSON/XML) without conversion
- Adapter layer converts received payload to ISO 20022 pain.002 internal representation:
  - gateway transaction_id → pain.002 MessageID
  - payment status (completed/failed/pending) → pain.002 TransactionStatus
  - timestamp → pain.002 CreationDateTime (converted to ISO 8601 UTC)
  - amount → pain.002 Amount
- Store original gateway payload in webhook_events table.original_payload (LONGTEXT) for audit trail
- Validate converted pain.002 structure before applying state transitions; reject malformed conversions
- Implement adapter interfaces: XenditWebhookAdapter, MidtransWebhookAdapter

**Rationale**:
- Preserves gateway-native communication while standardizing internal state representation
- Original payload audit trail supports dispute resolution and compliance investigation
- Adapter pattern enables future gateway additions without touching core payment logic

**Alternatives Considered**:
- Force all gateways to pain.002: Breaks Xendit/Midtrans integration; they control their format
- Pure gateway-native processing: Prevents future interoperability and compliance auditing

---

### 5. Installment Payment Gateway Integration Strategy

**Query**: Best practices for managing 2-12 installment payments through payment gateways (Xendit/Midtrans) that may not support native installments

**Decision**:
- Each installment treated as independent single payment to gateway (PayTrust-managed)
- Generate unique payment URLs for each installment using gateway-specific APIs:
  - Xendit: POST /v2/invoices (creates separate invoice per installment)
  - Midtrans: POST /v2/charge (creates separate charge per installment)
- payment_transaction record links to installment_schedule via installment_id foreign key
- Payment description includes invoice reference and installment number for gateway tracking
- Sequential enforcement: only generate payment URL for next unpaid installment in sequence per FR-061
- Overpayment handling per FR-065: excess automatically applied to next installments

**Rationale**:
- Aligns with gateway capabilities (both support multiple independent payments)
- PayTrust-managed approach provides full control over business logic (sequential validation, overpayment handling)
- Avoids gateway-specific installment limitations

**Alternatives Considered**:
- Gateway-native installments: Limited to Midtrans installment count restrictions; Xendit lacks feature
- Client-side installment scheduling: Loses transaction history; increases developer integration complexity

---

### 6. Concurrent Payment Locking Strategy

**Query**: PostgreSQL pessimistic locking patterns for preventing duplicate payment processing on concurrent requests (FR-047)

**Decision**:
- Implement pessimistic row-level locking using SELECT FOR UPDATE with 5-second timeout
- Retry logic: maximum 3 attempts with exponential backoff (100ms, 200ms base delays) + random jitter ±20ms
- Lock acquires before payment initiation, releases after status update
- Return 409 Conflict if lock cannot be acquired within timeout after all retries exhausted
- Connection pooling (sqlx::Pool) with configured connection timeout for deadlock prevention

**Rationale**:
- Pessimistic locking ensures only one payment process can execute per invoice
- 5-second timeout balances conflict detection speed with operational practicality
- Exponential backoff with jitter prevents thundering herd on concurrent retry storms

**Alternatives Considered**:
- Optimistic locking (version column): Higher contention with retry loop; slower for high-concurrency scenarios
- Application-level mutex: Requires persistence across instances (unsupported per NFR-009)

---

### 7. Rate Limiting Implementation (v1.0: In-Memory)

**Query**: In-memory rate limiting implementation for 1000 req/min per API key with extensible architecture for Redis future scaling

**Decision**:
- Implement RateLimiter trait with pluggable backends
- v1.0: InMemoryRateLimiter using HashMap<api_key_id, RateLimitState> with tokio::sync::Mutex
- Track: request_count, window_reset_time (60-second sliding window)
- Return 429 Too Many Requests with Retry-After header when exceeded
- Future: RedisRateLimiter for multi-instance horizontal scaling (documented in plan.md:L23)

**Rationale**:
- In-memory approach sufficient for single-instance deployment (NFR-009)
- Trait-based design enables clean Redis migration for future multi-instance support
- 1000 req/min = ~16.7 req/sec sustainable load, reasonable for single developer/merchant

**Alternatives Considered**:
- Token bucket: More accurate burst handling; unnecessary for fixed 1000 req/min requirement
- Leaky bucket: Simpler but less precise than sliding window

---

### 8. Webhook Retry Queue Architecture

**Query**: Async webhook retry queue implementation using tokio tasks with in-memory persistence (FR-040)

**Decision**:
- In-memory priority queue using HashMap<event_id, RetryState> + tokio async tasks
- RetryState tracks: attempt_number, next_retry_time, error_message
- Retry schedule per FR-040: 1min, 6min, 36min (cumulative from T=0)
- Retry ONLY for 5xx responses or connection timeout >10s; permanent fail for 4xx
- On app restart: all pending retry timers lost; manual recovery via GET /webhooks/failed endpoint
- Log all retry attempts to webhook_retry_log table for audit trail

**Rationale**:
- tokio tasks enable concurrent retry processing without blocking webhook reception
- In-memory queue sufficient for NFR-010 requirement (10,000 pending retries <100ms latency)
- Manual recovery after restart acceptable per FR-040 design (timers not persisted to DB)
- GET /webhooks/failed endpoint enables operator intervention after crashes

**Alternatives Considered**:
- Persistent queue (database table): Adds complexity; timers still lost on restart
- Redis queue: Overkill for single-instance deployment; not required per NFR-009

---

### 9. Financial Reporting Query Performance

**Query**: PostgreSQL optimization patterns for financial reports with date range filtering, currency/gateway grouping, and large transaction volumes

**Decision**:
- Create indexes on:
  - (tenant_id, created_at) composite index for date range queries
  - (currency_code, tax_rate) for tax breakdown reports
  - (gateway_id, created_at) for gateway-specific fee reports
- Use GROUP BY queries with SUM aggregations for efficient computation
- Implement materialized views for frequently-accessed reports (refresh daily via background job)
- Pagination support: LIMIT/OFFSET with cursor-based pagination for large result sets

**Rationale**:
- Indexes support fast filtering without full table scans (critical for SC-003: 1-hour reporting)
- Aggregation queries efficient in PostgreSQL; no need for separate OLAP database initially
- Materialized views enable fast report generation while keeping source data current

**Alternatives Considered**:
- Separate data warehouse (BigQuery, Redshift): Overkill for v1.0 scale; adds operational complexity
- Real-time aggregation: Slower query performance; not needed for hourly report freshness

---

### 10. Multi-Tenant Data Isolation Strategy

**Query**: Multi-tenant architecture patterns with API key-based tenant identification and query filtering

**Decision**:
- One tenant per API key; tenant_id derived from api_keys table lookup during authentication
- All database queries filter by tenant_id in WHERE clause; enforced at repository layer
- API key authentication middleware extracts api_key_id, looks up tenant_id, attaches to request context
- tenant_id propagated through request context (actix-web web::Data) to services/repositories
- Validate tenant_id on all read/write operations; return 403 Forbidden for cross-tenant access attempts
- Applies to: invoices, line items, installment schedules, payment transactions, financial reports, audit logs

**Rationale**:
- Clean separation of tenant data; prevents accidental cross-tenant data leaks
- API key lookup at authentication time (single DB query) avoids repeated lookups
- Request context propagation enables tenant isolation at database layer without threading through every function

**Alternatives Considered**:
- Row-level security (PostgreSQL RLS): Adds complexity; harder to debug; slower query planning
- API-layer filtering only: Risk of SQL injection in repository layer if not careful

---

### 11. Currency Decimal Place Handling (IDR/MYR/USD)

**Query**: Currency-specific decimal place handling for calculations and storage (IDR: 0 decimals, MYR/USD: 2 decimals)

**Decision**:
- Store all amounts as BIGINT (multiply by power of 10):
  - IDR: store as-is (1000 IDR = 1000)
  - MYR/USD: multiply by 100 (1.50 MYR = 150)
- Conversion functions: to_cents(currency, amount) → BIGINT, from_cents(currency, amount) → Decimal
- Validation: reject non-integer IDR amounts; reject MYR/USD with >2 decimal places
- Rounding for installment calculations per FR-063: round down all except last, last absorbs difference

**Rationale**:
- BIGINT storage ensures exact arithmetic without floating-point rounding errors (addresses NFR-005)
- Conversion functions encapsulate complexity; easy to audit and test
- Rounding strategy ensures total always matches invoice amount

**Alternatives Considered**:
- PostgreSQL NUMERIC type: More flexible but slower; BIGINT sufficient for our precision needs
- Storing as strings: Expensive conversions; harder to validate and calculate

---

### 12. API Key Audit Logging Implementation

**Query**: API key audit logging design with operation tracking, actor identification, and queryable audit trail (FR-071)

**Decision**:
- api_key_audit_log table: id, api_key_id, operation_type (created/rotated/revoked/used), actor_identifier, ip_address, old_key_hash (NULL for non-rotation ops), created_at, success, error_message
- actor_identifier values:
  - Regular operations: authenticated API key ID (derive from X-API-Key header)
  - Automated operations: 'SYSTEM'
  - Admin operations: admin username (stored in admin session context)
- GET /api-keys/audit endpoint with pagination (page, page_size) and date range filtering (start_date, end_date in ISO 8601)
- Retention: 1 year minimum per NFR-011
- Admin authentication via X-Admin-Key header (environment variable: ADMIN_API_KEY)

**Rationale**:
- Comprehensive audit trail supports security incident investigation and compliance
- Actor identification enables tracking which client/admin performed each operation
- Queryable endpoint enables operators to review audit logs without direct database access

**Alternatives Considered**:
- Application logs only: Hard to search; not structured for compliance review
- Database triggers: Harder to maintain; less flexible for query patterns

---

## Summary of Research Findings

All major technical unknowns resolved. Implementation approach documented:

✅ **Rust + Actix-web**: Proven for high-performance REST APIs
✅ **PostgreSQL BIGINT storage**: Ensures numeric precision for financial calculations
✅ **ISO 20022 hybrid approach**: Internal compliance without breaking external API
✅ **Payment gateway adapters**: Xendit/Midtrans wrapper layer with standardized interfaces
✅ **Installment payments**: PayTrust-managed with independent gateway transactions
✅ **Concurrent locking**: SELECT FOR UPDATE with exponential backoff + jitter
✅ **Rate limiting**: In-memory with trait-based design for future Redis scaling
✅ **Webhook retries**: Async tokio queue with manual recovery after restart
✅ **Financial reporting**: Indexed queries with materialized views
✅ **Multi-tenant isolation**: Query-level filtering with tenant_id propagation
✅ **Currency handling**: BIGINT storage with conversion functions
✅ **Audit logging**: Comprehensive API key operation tracking

**Proceed to Phase 1: Design & Contracts**
