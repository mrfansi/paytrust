# PayTrust Implementation Roadmap

**Scope**: Complete implementation from Phase 2 (task generation) through production deployment
**Timeline**: 8-12 weeks for MVP (Phases A-F)
**Team Size**: 2-3 Rust developers, 1 DevOps engineer (part-time)
**Status**: Planning phase; dates start upon task generation approval

---

## Executive Summary

| Phase | Duration | Deliverable | Risk Level |
|-------|----------|-------------|-----------|
| **A: Setup** | 1 week | Project scaffold, DB migrations, CI/CD | Low |
| **B: Core** | 2-3 weeks | Domain models, repositories, services | Low |
| **C: API** | 2 weeks | HTTP handlers, error mapping, validation | Medium |
| **D: Gateways** | 2-3 weeks | Xendit/Midtrans adapters, webhook handling | High |
| **E: Features** | 2 weeks | ISO 20022, reporting, API key management | Medium |
| **F: Testing** | 2 weeks | Integration tests, load testing, security | Medium |
| **G: Deploy** | 1 week | Docker, staging, production readiness | Low |

**Total**: 12-15 weeks for production-ready MVP

---

## Phase A: Project Setup & Infrastructure (Week 1)

### Objectives
- Establish Rust project structure
- Database schema creation & migrations
- CI/CD pipeline setup
- Local development environment

### Key Deliverables

**A1: Project Scaffolding** (2 days)
- Create Cargo.toml with all dependencies per cargo-dependencies.md
- Set up project directory structure (src/models, src/services, src/api, etc.)
- Configure build profiles (dev, release)
- Add .gitignore, README.md, CONTRIBUTING.md

**A2: Database Schema** (3 days)
- Create 9 migration files:
  1. `init_invoices.sql` - Core invoice table
  2. `init_line_items.sql` - Line items
  3. `init_payment_transactions.sql` - Payment tracking
  4. `init_installment_schedules.sql` - Installment plan
  5. `init_gateway_configurations.sql` - Gateway config (with Xendit/Midtrans predefined)
  6. `init_api_keys.sql` - API key storage
  7. `init_webhook_events.sql` - Webhook deduplication
  8. `init_webhook_retry_log.sql` - Webhook retry audit
  9. `init_api_key_audit_log.sql` - API key audit trail

- Run migrations locally with Docker PostgreSQL
- Verify schema with `\d` commands
- Create indexes per data-model.md

**A3: CI/CD Pipeline** (2 days)
- GitHub Actions workflow:
  - `cargo build --release` on push to main/feature branches
  - `cargo test --all-features` on all commits
  - `cargo audit` for security vulnerabilities
  - Code coverage reporting (codecov)
- Docker build workflow (generate image on release)
- Pre-commit hooks:
  - `cargo fmt` (code formatting)
  - `cargo clippy` (linting)

**A4: Local Development Setup** (1 day)
- Docker Compose configuration:
  ```yaml
  version: '3.8'
  services:
    postgres:
      image: postgres:15
      environment:
        POSTGRES_DB: paytrust
        POSTGRES_PASSWORD: dev_password
      ports:
        - "5432:5432"
  ```
- `.env` template with example values
- Database initialization script
- Documentation: DEVELOPMENT.md

### Success Criteria
- [ ] `cargo build --release` completes without errors
- [ ] All migrations run successfully
- [ ] CI/CD pipeline runs on every commit
- [ ] Team can start local development with `docker-compose up`

### Estimated Effort
- **Developer**: 5 days (one developer)
- **DevOps**: 2 days (setting up CI/CD)

---

## Phase B: Core Domain Models & Business Logic (Weeks 2-4)

### Objectives
- Implement all domain entities (Invoice, LineItem, etc.)
- Create repository layer for data access
- Implement business logic services

### Key Deliverables

**B1: Domain Models** (3-4 days)
- src/models/invoice.rs: Invoice struct with all fields, status enum, validation
- src/models/line_item.rs: LineItem with tax calculation
- src/models/installment_schedule.rs: InstallmentSchedule with status tracking
- src/models/payment_transaction.rs: PaymentTransaction with gateway mapping
- src/models/api_key.rs: ApiKey for authentication
- src/models/webhook_event.rs: WebhookEvent for deduplication

**B2: Repository Layer** (5-6 days)
- src/repository/invoice_repository.rs
  - `create_invoice()` → INSERT
  - `get_invoice()` → SELECT with tenant filter
  - `update_invoice_status()` → UPDATE
  - `update_payment_initiated_at()` → SET payment_initiated_at timestamp
  - `lock_for_payment()` → SELECT FOR UPDATE NOWAIT with retry logic

- src/repository/payment_repository.rs
  - `create_payment_transaction()` → INSERT
  - `get_transaction()` → SELECT
  - `mark_transaction_completed()` → UPDATE

- src/repository/webhook_repository.rs
  - `check_webhook_exists()` → SELECT for deduplication
  - `create_webhook_event()` → INSERT
  - `mark_webhook_processed()` → UPDATE

- Base trait: `Repository<T>` with CRUD operations

**B3: Service Layer** (6-7 days)
- src/services/invoice_service.rs
  - `create_invoice(tenant_id, request)` → validates request, calculates amounts, creates invoice + line items
  - `calculate_amounts(line_items, tax_rate)` → computes subtotal, tax, service fee using rust_decimal
  - `validate_invoice(invoice)` → checks immutability rules, gateway support, currency
  - `get_invoice(tenant_id, invoice_id)` → retrieves with tenant isolation

- src/services/installment_service.rs
  - `distribute_installments(invoice, count, custom_amounts)` → proportional distribution
  - `calculate_installment_totals()` → per-installment tax and fee
  - `validate_installment_sum()` → ensure total matches invoice
  - `handle_overpayment()` → apply excess to next installments

- src/services/payment_service.rs
  - `process_payment(invoice)` → orchestrate payment flow
  - `initiate_payment(invoice, gateway)` → call gateway adapter
  - `handle_webhook(payload)` → deduplication + status update

- src/services/financial_service.rs
  - `get_financial_report(tenant_id, start_date, end_date)` → aggregate query
  - `calculate_by_currency()` → GROUP BY currency
  - `calculate_tax_breakdown()` → GROUP BY tax_rate

### Implementation Notes

**Currency Handling Function** (shared utility):
```rust
// src/utils/currency.rs
pub fn format_amount(amount: i64, currency: &str) -> String {
    match currency {
        "IDR" => amount.to_string(),
        "MYR" | "USD" => {
            let decimal = Decimal::from(amount) / Decimal::new(100, 0);
            decimal.to_string()
        }
        _ => unreachable!(),
    }
}

pub fn parse_amount(amount_str: &str, currency: &str) -> Result<i64> {
    match currency {
        "IDR" => amount_str.parse::<i64>(),
        "MYR" | "USD" => {
            let decimal = Decimal::from_str(amount_str)?;
            Ok((decimal * Decimal::new(100, 0)).to_i64().ok_or(...)?))
        }
        _ => Err(...),
    }
}
```

**Tax Calculation**:
```rust
// Implements FR-049, FR-050, FR-051, FR-052
fn calculate_invoice_amounts(
    line_items: &[LineItemInput],
    service_fee_rate: Decimal,
) -> Result<InvoiceAmounts> {
    let mut subtotal = Decimal::ZERO;
    let mut total_tax = Decimal::ZERO;

    for item in line_items {
        let line_subtotal = Decimal::from(item.quantity) * Decimal::from(item.unit_price);
        let line_tax = line_subtotal * item.tax_rate;

        subtotal += line_subtotal;
        total_tax += line_tax;
    }

    let service_fee = (subtotal * service_fee_rate).round();
    let total = subtotal + total_tax + service_fee;

    Ok(InvoiceAmounts {
        subtotal: subtotal.to_i64()?,
        total_tax: total_tax.to_i64()?,
        service_fee: service_fee.to_i64()?,
        total: total.to_i64()?,
    })
}
```

### Success Criteria
- [ ] All 50+ repository methods implemented and tested
- [ ] Service methods handle error cases (invalid input, business rule violations)
- [ ] Decimal math tested for precision (no rounding errors)
- [ ] Tenant isolation enforced in all queries
- [ ] Unit tests for services (500+ test cases)

### Estimated Effort
- **Developer A**: 10 days (models + repositories)
- **Developer B**: 10 days (services + business logic)

---

## Phase C: HTTP API Implementation (Weeks 5-6)

### Objectives
- Create REST endpoint handlers
- Implement request validation and error mapping
- Set up middleware (authentication, rate limiting)

### Key Deliverables

**C1: Middleware Stack** (2-3 days)
- src/middleware/auth_middleware.rs
  - Extract X-API-Key header
  - Look up API key in database
  - Extract tenant_id from API key record
  - Attach to request context
  - Return 401 on invalid key

- src/middleware/rate_limit_middleware.rs
  - Check rate limit per tenant_id
  - Return 429 Too Many Requests if exceeded
  - Include Retry-After header per FR-039

- src/middleware/logging_middleware.rs
  - Log all requests with correlation IDs
  - Structured logging (tracing)

**C2: Error Handling** (2 days)
- src/error/mod.rs
  - Define custom error types (thiserror)
  - Map to HTTP responses
  - Format JSON error responses per OpenAPI spec

**C3: Route Handlers** (5-6 days)
- src/api/invoices.rs
  - `POST /invoices` → create_invoice handler
  - `GET /invoices/{id}` → get_invoice handler
  - `GET /invoices/{id}/installments` → get_installments handler
  - `GET /invoices/{id}/overpayment` → get_overpayment handler
  - `POST /invoices/{id}/supplementary` → create_supplementary handler
  - `GET /invoices/{id}/payment-initiation` → export_pain001 handler

- src/api/reports.rs
  - `GET /reports/financial` → financial_report handler with date range filtering

- src/api/api_keys.rs (admin endpoints)
  - `POST /api-keys` → create_api_key (admin-only)
  - `PUT /api-keys/{id}/rotate` → rotate_api_key (admin-only)
  - `DELETE /api-keys/{id}` → revoke_api_key (admin-only)
  - `GET /api-keys/audit` → get_audit_log (admin-only)

- src/api/webhooks.rs
  - `GET /webhooks/failed` → list_failed_webhooks (admin-only)

**C4: Request Validation** (2-3 days)
- Implement custom validators per validator crate
- Validate currency codes (ISO 4217)
- Validate tax rates (0-100% with 4 decimal places)
- Validate country codes (ISO 3166-1 alpha-2)
- Validate date ranges (expires_at in future, >= 1 hour, <= 30 days)

### Implementation Pattern

```rust
// src/api/invoices.rs
use actix_web::{web, HttpResponse};
use crate::models::{CreateInvoiceRequest, InvoiceResponse};
use crate::services::InvoiceService;

#[post("/invoices")]
async fn create_invoice(
    ctx: web::Data<RequestContext>,
    req: web::Json<CreateInvoiceRequest>,
    service: web::Data<InvoiceService>,
) -> Result<HttpResponse> {
    // Validation happens automatically via validator crate
    let invoice = service.create_invoice(ctx.tenant_id, req.into_inner()).await?;
    Ok(HttpResponse::Created().json(InvoiceResponse::from(invoice)))
}

#[get("/invoices/{id}")]
async fn get_invoice(
    ctx: web::Data<RequestContext>,
    path: web::Path<i64>,
    service: web::Data<InvoiceService>,
) -> Result<HttpResponse> {
    let invoice_id = path.into_inner();
    let invoice = service.get_invoice(ctx.tenant_id, invoice_id).await?;
    Ok(HttpResponse::Ok().json(InvoiceResponse::from(invoice)))
}
```

### Success Criteria
- [ ] All 12 endpoints implemented and responding
- [ ] Request validation returns 400 Bad Request with clear error messages
- [ ] Middleware chain executes in correct order
- [ ] All responses match OpenAPI schema
- [ ] 401/403 responses on auth failures
- [ ] 429 responses on rate limit exceeded

### Estimated Effort
- **Developer**: 10 days (handlers + validation)

---

## Phase D: Payment Gateway Integration (Weeks 7-9)

### Objectives
- Implement Xendit and Midtrans payment APIs
- Handle webhook notifications
- Implement retry logic for failed webhooks

### Key Deliverables

**D1: Gateway Adapters** (5-6 days)
- src/gateway/mod.rs → PaymentGateway trait

- src/gateway/xendit.rs → XenditGateway
  - `create_payment()` → POST /v2/invoices
  - `verify_webhook()` → validate x-callback-token header
  - Handle XenditPaymentResponse parsing
  - Xendit-specific error mapping

- src/gateway/midtrans.rs → MidtransGateway
  - `create_payment()` → POST /v2/charge
  - `verify_webhook()` → SHA512 HMAC verification
  - Handle MidtransChargeResponse parsing
  - Midtrans-specific error mapping

**D2: Webhook Processing** (4-5 days)
- src/webhook/processor.rs
  - `process_webhook()` → deduplication check (query webhook_events table)
  - Convert gateway-native JSON to ISO 20022 pain.002 internally
  - Update invoice status based on payment outcome
  - Store original payload for audit
  - Handle webhook processing errors with retry scheduling

- src/webhook/retry_queue.rs
  - In-memory HashMap<event_id, RetryState> with tokio::sync::Mutex
  - Retry schedule: 1min, 6min, 36min (cumulative)
  - Spawn tokio::task for each retry
  - Manual recovery via GET /webhooks/failed endpoint

**D3: ISO 20022 Conversion** (3-4 days)
- src/iso20022/pain001.rs → Payment Initiation generation
  - Map Invoice → pain.001 XML structure
  - Generate dynamic XML on demand per GET /invoices/{id}/payment-initiation

- src/iso20022/pain002.rs → Payment Status Report conversion
  - Map gateway webhook → pain.002 structure
  - Used internally for validation/compliance

### Implementation Details

**Xendit Webhook Verification**:
```rust
// src/gateway/xendit.rs
fn verify_xendit_webhook(
    body: &[u8],
    x_callback_token: &str,
    xendit_api_key: &str,
) -> Result<()> {
    // Xendit uses callback token in header
    let expected_token = compute_callback_token(xendit_api_key, body);

    if !constant_time_compare(expected_token.as_str(), x_callback_token) {
        return Err(PaymentError::InvalidWebhookSignature);
    }

    Ok(())
}
```

**Midtrans Webhook Verification**:
```rust
// src/gateway/midtrans.rs
fn verify_midtrans_webhook(
    body: &[u8],
    x_signature: &str,
    server_key: &str,
) -> Result<()> {
    use sha2::{Sha512, Digest};

    let mut hasher = Sha512::new();
    hasher.update(body);
    hasher.update(server_key.as_bytes());
    let computed = format!("{:x}", hasher.finalize());

    if !constant_time_compare(&computed, x_signature) {
        return Err(PaymentError::InvalidWebhookSignature);
    }

    Ok(())
}
```

**Webhook Processing Loop**:
```rust
// src/webhook/processor.rs
async fn process_webhook_with_retries(
    pool: &PgPool,
    webhook_id: i64,
    payload: WebhookPayload,
) {
    for attempt in 1..=3 {
        match process_once(&pool, webhook_id, &payload).await {
            Ok(_) => {
                // Mark processed
                sqlx::query("UPDATE webhook_events SET status = 'processed' WHERE id = $1")
                    .bind(webhook_id)
                    .execute(pool)
                    .await
                    .ok();
                return;
            }
            Err(e) if attempt < 3 => {
                // Schedule retry
                let delay = match attempt {
                    1 => Duration::from_secs(60),
                    2 => Duration::from_secs(300),
                    _ => unreachable!(),
                };

                tokio::time::sleep(delay).await;
                // Continue to next iteration
            }
            Err(e) => {
                // Final failure
                sqlx::query("UPDATE webhook_events SET status = 'failed' WHERE id = $1")
                    .bind(webhook_id)
                    .execute(pool)
                    .await
                    .ok();
            }
        }
    }
}
```

### Success Criteria
- [ ] POST /invoices calls correct gateway and returns payment_url
- [ ] Webhook endpoint receives and processes notifications
- [ ] Deduplication prevents double-processing of same webhook
- [ ] Retry logic schedules failed webhooks correctly
- [ ] GET /webhooks/failed shows pending retries
- [ ] Concurrent payment locking prevents race conditions
- [ ] Gateway tests with wiremock mock server

### Estimated Effort
- **Developer A**: 10 days (gateway adapters)
- **Developer B**: 8 days (webhook processing + retries)

---

## Phase E: Advanced Features & Compliance (Weeks 10-11)

### Objectives
- Implement ISO 20022 compliance
- API key management with audit logging
- Financial reporting

### Key Deliverables

**E1: ISO 20022 Pain.001 Generation** (3 days)
- GET /invoices/{id}/payment-initiation endpoint
- Dynamic XML generation on demand
- XSD schema validation
- Return valid ISO 20022 XML

**E2: API Key Management** (3 days)
- POST /api-keys → Generate new key
  - Argon2 hash before storage
  - Return plaintext only at creation
- PUT /api-keys/{id}/rotate → Rotate key
  - Mark old as rotated
  - Generate new key
- DELETE /api-keys/{id} → Revoke key
  - Mark as revoked
  - Reject future requests with this key
- GET /api-keys/audit → Query audit log
  - Pagination support
  - Date range filtering
  - Structured audit trail (operation, actor, timestamp)

**E3: Financial Reporting** (2-3 days)
- GET /reports/financial
  - Date range filtering (start_date, end_date)
  - Optional currency filter
  - Optional gateway filter
  - Return breakdown by currency
  - Include tax breakdown by rate
  - Include gateway breakdown

```rust
// Query pattern for financial reporting
pub async fn get_financial_report(
    pool: &PgPool,
    tenant_id: i64,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    currency: Option<String>,
) -> Result<FinancialReportResponse> {
    let query = "
        SELECT
            currency_code,
            COUNT(*) as transaction_count,
            SUM(amount_received) as total_amount,
            SUM(total_tax_amount) as total_tax,
            SUM(total_service_fee_amount) as total_fees
        FROM payment_transactions
        WHERE tenant_id = $1
            AND created_at >= $2
            AND created_at < $3
            AND ($4::TEXT IS NULL OR currency_code = $4)
        GROUP BY currency_code
    ";

    let results = sqlx::query(query)
        .bind(tenant_id)
        .bind(start_date)
        .bind(end_date)
        .bind(currency)
        .fetch_all(pool)
        .await?;

    // Transform to response format
    Ok(FinancialReportResponse { ... })
}
```

### Success Criteria
- [ ] GET /invoices/{id}/payment-initiation returns valid pain.001 XML
- [ ] pain.001 passes XSD schema validation
- [ ] API key creation returns plaintext key once only
- [ ] API key rotation invalidates old key immediately
- [ ] Audit log records all API key operations
- [ ] Financial report returns correct aggregations
- [ ] Reporting handles millions of transactions efficiently

### Estimated Effort
- **Developer A**: 8 days (ISO 20022 + API key management)
- **Developer B**: 4 days (Financial reporting queries)

---

## Phase F: Testing & Quality Assurance (Weeks 12-13)

### Objectives
- Comprehensive integration testing
- Load testing and performance validation
- Security testing
- Documentation

### Key Deliverables

**F1: Integration Tests** (5 days)
- tests/integration/test_invoice_flow.rs
  - Create invoice with various line items
  - Verify calculation accuracy
  - Check immutability after payment initiation

- tests/integration/test_installment_payments.rs
  - Create invoice with installments
  - Test sequential enforcement
  - Verify proportional distribution
  - Test overpayment handling

- tests/integration/test_concurrent_payments.rs
  - Concurrent payment requests on same invoice
  - Verify only one succeeds
  - Others get 409 Conflict

- tests/integration/test_webhook_processing.rs
  - Receive webhook notifications
  - Verify deduplication
  - Test retry logic
  - Manual recovery endpoint

- tests/integration/test_rate_limiting.rs
  - Send 1000 requests within window
  - Verify 1001st is rejected
  - Check Retry-After header

**F2: Load Testing** (2-3 days)
- k6 script for invoice creation
  ```javascript
  // k6 script
  import http from 'k6/http';

  export let options = {
    vus: 100,
    duration: '5m',
  };

  export default function() {
    const payload = JSON.stringify({
      gateway_id: 1,
      currency_code: 'IDR',
      expires_at: new Date(Date.now() + 86400000).toISOString(),
      line_items: [{
        product_name: 'Test Product',
        quantity: 1,
        unit_price: 1000000,
        tax_rate: 0.1,
      }],
    });

    const response = http.post('http://localhost:8000/invoices', payload, {
      headers: {
        'X-API-Key': 'test_key_12345',
        'Content-Type': 'application/json',
      },
    });
  }
  ```
- Validate: <2 second p95, 100 concurrent users sustained 5 minutes

**F3: Security Testing** (2 days)
- OWASP Top 10 checklist:
  - SQL injection (prevented by sqlx)
  - Cross-site scripting (N/A for REST API)
  - Cross-site request forgery (API key auth)
  - Sensitive data exposure (TLS required)
  - Broken authentication (API key validation)
  - Sensitive data exposure (Argon2 hashing)
  - XML external entities (if using XML parsing)
  - Broken access control (tenant isolation, rate limiting)
  - Using components with known vulnerabilities (cargo audit)
  - Insufficient logging (structured tracing)

**F4: Documentation** (2 days)
- API reference (generated from OpenAPI spec)
- Deployment guide (Docker, Kubernetes)
- Operations runbook (scaling, monitoring, troubleshooting)
- Security hardening guide (TLS, secrets management)

### Success Criteria
- [ ] All integration tests pass
- [ ] Load test: <2 second p95 response time, 100 concurrent users
- [ ] 99.5% success rate under load
- [ ] No security vulnerabilities in `cargo audit`
- [ ] 80%+ code coverage (unit + integration)
- [ ] Documentation complete and reviewed

### Estimated Effort
- **Developer A**: 8 days (integration testing)
- **QA Engineer**: 5 days (load testing, security)
- **Tech Writer**: 3 days (documentation)

---

## Phase G: Deployment & Launch (Week 14)

### Objectives
- Containerize application
- Set up staging environment
- Production readiness validation
- Go-live

### Key Deliverables

**G1: Docker Deployment** (2 days)
- Dockerfile with multi-stage build
- docker-compose.yaml for local development
- Docker Compose with PostgreSQL + app for staging
- Registry: Push to Docker Hub or private registry

**G2: Kubernetes Deployment** (Optional, 2-3 days)
- Helm chart with configurable values
- StatefulSet for PostgreSQL (or use managed service)
- Deployment for app replicas
- Service and Ingress configuration
- Health checks and liveness probes

**G3: Production Readiness Checklist** (1 day)
- [ ] Database backups configured (automated daily)
- [ ] Secrets management (environment variables or vault)
- [ ] Monitoring and alerting (Prometheus, DataDog, or similar)
- [ ] Logging aggregation (ELK, DataDog, CloudWatch)
- [ ] Graceful shutdown handling (30-second timeout)
- [ ] Health check endpoint (/health)
- [ ] Rate limiting configured
- [ ] Webhook retry mechanism tested
- [ ] Database connection pooling tuned
- [ ] TLS/HTTPS enforced
- [ ] API documentation deployed
- [ ] Admin dashboard or monitoring tools
- [ ] Runbook for common operations
- [ ] Incident response plan

**G4: Staging Validation** (2-3 days)
- Deploy to staging environment
- Run full integration test suite
- Manual testing of critical flows:
  - Create invoice, receive payment notification
  - Create installment invoice, pay each installment
  - Test API key rotation
  - Verify financial reporting accuracy
- Load testing against staging (identify any bottlenecks)
- Security scanning (OWASP ZAP)

**G5: Production Launch** (1 day)
- Blue-green deployment strategy
- Database migrations (validated in staging first)
- Health checks confirm successful deployment
- Monitor error rates for first 24 hours
- Document any post-launch issues

### Success Criteria
- [ ] Docker image builds without errors
- [ ] Staging environment passes all tests
- [ ] Production readiness checklist 100% complete
- [ ] Incident response team trained
- [ ] Rollback plan documented and tested

### Estimated Effort
- **DevOps**: 5-7 days (Docker, Kubernetes, monitoring)
- **Developer**: 2 days (deployment validation)

---

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|-----------|
| Gateway API changes | Medium | High | Monitor API changelogs, version adapters |
| Database performance degradation | Medium | High | Performance test early (Phase F), optimize queries |
| Concurrent payment race conditions | Low | Critical | Extensive testing with pgrx, load testing |
| Webhook deduplication failures | Low | Critical | Unit tests for edge cases, idempotency verification |
| Deployment complexity | Medium | Medium | Use managed database service, practice deployments in staging |
| Security vulnerabilities | Low | Critical | Regular cargo audit, OWASP testing, security review |

---

## Success Metrics for MVP

- **Functionality**: All 75+ functional requirements implemented and tested
- **Performance**: <2 second p95 response time for invoice creation under 100 concurrent users
- **Reliability**: 99.5% uptime over 30-day measurement period
- **Security**: Zero critical vulnerabilities, passed OWASP audit
- **Maintainability**: 80%+ code coverage, comprehensive documentation
- **Scalability**: Ready for single-instance production deployment; architecture supports future Redis-backed multi-instance scaling

---

## Timeline Summary

```
Week 1:     Phase A - Setup & Infrastructure
Weeks 2-4:  Phase B - Domain Models & Services
Weeks 5-6:  Phase C - HTTP API
Weeks 7-9:  Phase D - Payment Gateway Integration
Weeks 10-11: Phase E - Advanced Features
Weeks 12-13: Phase F - Testing & QA
Week 14:    Phase G - Deployment & Launch
```

**Total**: 14 weeks (10 weeks minimum with experienced team, 18 weeks maximum with junior team)

---

## Team Assignments

**Developer A (Backend/Core)**:
- Phase A: Project setup
- Phase B: Models & repositories
- Phase E: ISO 20022 & API key management
- Phase F: Integration testing
- Phase G: Validation

**Developer B (Services/Features)**:
- Phase B: Services & business logic
- Phase C: HTTP handlers
- Phase D: Gateway integration
- Phase E: Financial reporting
- Phase F: Integration testing

**DevOps Engineer (Part-time)**:
- Phase A: CI/CD pipeline setup
- Phase D: Webhook monitoring (1-2 days)
- Phase G: Docker, Kubernetes, production readiness

---

## Go-Live Checklist

Before launching to production:

- [ ] All tests passing (unit, integration, load)
- [ ] Code reviewed by at least 2 developers
- [ ] Security audit completed
- [ ] Database migrations tested in staging
- [ ] Backup and recovery procedures documented
- [ ] On-call rotation established
- [ ] Incident response plan communicated
- [ ] API documentation live
- [ ] Admin API keys generated and secured
- [ ] Health check endpoint responding
- [ ] Monitoring and alerting configured
- [ ] Log aggregation working
- [ ] Database connection pooling validated
- [ ] Rate limiting functional
- [ ] Webhook retry queue tested
- [ ] Gateway credentials configured for production
- [ ] TLS certificates installed
- [ ] Load balancer configured
- [ ] Database backups scheduled
- [ ] Rollback procedure tested

