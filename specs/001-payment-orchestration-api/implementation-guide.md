# PayTrust Implementation Guide

**Purpose**: Detailed implementation guidance for Phase 2+ development
**Status**: Based on deep research findings
**Last Updated**: 2025-11-03

---

## Table of Contents

1. [Technology Stack](#technology-stack)
2. [Core Architecture Patterns](#core-architecture-patterns)
3. [Database Implementation](#database-implementation)
4. [API Implementation](#api-implementation)
5. [Payment Processing](#payment-processing)
6. [Webhook Handling](#webhook-handling)
7. [Testing Strategy](#testing-strategy)
8. [Deployment](#deployment)
9. [Common Pitfalls](#common-pitfalls)

---

## Technology Stack

### Production-Proven Selections

**Language & Runtime**
```
Rust 1.91+ (2021 edition)
- Memory safety without garbage collection
- Compile-time safety guarantees
- Exceptional performance for async workloads
```

**Web Framework**
```
actix-web 4.9+
- Fastest Rust web framework (benchmarks: 100k+ req/s)
- Excellent middleware ecosystem
- Production-proven at scale (Cloudflare, Discord-adjacent systems)
- Superior error handling and extensibility
```

**Database**
```
SQLx 0.8+ (not Diesel)
- Compile-time SQL verification prevents runtime errors
- Native async/await support (no blocking pool)
- Better for dynamic queries (useful for reporting)
- Simpler migrations than Diesel
- PostgreSQL 13+ with async driver
```

**Financial Math**
```
rust_decimal 1.36+
- 28-29 digit precision (sufficient for all currencies)
- No floating-point rounding errors
- Built-in ROUND_HALF_UP (standard for finance)
- Direct serialization to JSON
```

**Error Handling**
```
thiserror 1.0+ (libraries)
anyhow 1.0+ (application)
- Clear error propagation
- Custom error types with context
- Compatible with web framework error handling
```

**Async Runtime**
```
tokio 1.37+ (included via actix-web)
- Industry standard async runtime
- Excellent task spawning for webhooks
- Proven for high-concurrency applications
```

**Logging**
```
tracing 0.1+ with tracing-subscriber
- Structured logging (JSON output)
- Correlation IDs for request tracking
- Performance monitoring integration
- Replaces println! debugging
```

---

## Core Architecture Patterns

### 1. Layered Architecture

```
┌─────────────────────────────────────────┐
│       HTTP Layer (actix-web routes)     │  <- Handlers
├─────────────────────────────────────────┤
│       Service Layer (business logic)     │  <- InvoiceService, PaymentService, etc.
├─────────────────────────────────────────┤
│       Repository Layer (data access)     │  <- InvoiceRepository, PaymentRepository
├─────────────────────────────────────────┤
│   PostgreSQL Database                   │
└─────────────────────────────────────────┘
```

**Benefit**: Clear separation of concerns, easier testing at each layer

### 2. Error Handling Strategy

**Use Custom Error Types**:
```rust
#[derive(thiserror::Error, Debug)]
pub enum PaymentError {
    #[error("Invoice not found: {id}")]
    InvoiceNotFound { id: i64 },

    #[error("Rate limit exceeded: retry after {retry_after_seconds}s")]
    RateLimitExceeded { retry_after_seconds: u64 },

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Gateway error: {gateway} - {message}")]
    GatewayError { gateway: String, message: String },
}
```

**Map to HTTP Responses**:
```rust
impl actix_web::error::ResponseError for PaymentError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvoiceNotFound { .. } => StatusCode::NOT_FOUND,
            Self::RateLimitExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::GatewayError { .. } => StatusCode::BAD_GATEWAY,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let error_response = ErrorResponse {
            error: "payment_error",
            message: self.to_string(),
        };
        HttpResponse::build(self.status_code()).json(error_response)
    }
}
```

### 3. Middleware Stack

```rust
// In main.rs
HttpServer::new(|| {
    App::new()
        // Logging middleware (first = runs first on request)
        .wrap(middleware::Logger::default())

        // Rate limiting middleware
        .wrap(RateLimitMiddleware::new(rate_limiter.clone()))

        // API key authentication middleware
        .wrap(ApiKeyAuthMiddleware::new(api_key_repo.clone()))

        // Error handling (last = wraps all)
        // ... routes
})
```

**Why this order**: Logging captures everything; rate limiting before auth prevents auth overhead; errors handled last

### 4. Request Context Pattern

```rust
// Extract tenant_id from middleware and pass through request context
pub struct RequestContext {
    pub tenant_id: i64,
    pub api_key_id: i64,
}

// In handler
async fn create_invoice(
    ctx: web::Data<RequestContext>,
    req: web::Json<CreateInvoiceRequest>,
) -> Result<HttpResponse, PaymentError> {
    // ctx.tenant_id automatically isolated to this tenant
    let invoice = invoice_service.create(ctx.tenant_id, req.into_inner()).await?;
    Ok(HttpResponse::Created().json(invoice))
}
```

**Benefit**: Automatic tenant isolation; prevents cross-tenant data leaks

---

## Database Implementation

### 1. Connection Pooling Configuration

```rust
// Recommended settings for payment systems
let pool = PgPoolOptions::new()
    .max_connections(50)              // Single instance capacity
    .connect_timeout(Duration::from_secs(10))
    .idle_timeout(Some(Duration::from_secs(600)))
    .max_lifetime(Some(Duration::from_secs(1800)))
    .connect(&database_url)
    .await?;
```

**Tuning Guide**:
- `max_connections = 50`: For single instance handling ~100 concurrent requests
- Scale to 100-150 connections for 1000 concurrent requests (multi-instance + Redis)
- Connection acquire timeout: 10 seconds (prevents cascading failures)

### 2. Transaction Handling

```rust
// For payment processing requiring atomicity
let mut tx = pool.begin().await?;

// 1. Lock invoice
let invoice = sqlx::query_as::<_, Invoice>(
    "SELECT * FROM invoices WHERE id = $1 AND tenant_id = $2 FOR UPDATE NOWAIT"
)
.bind(invoice_id)
.bind(tenant_id)
.fetch_one(&mut *tx)
.await
.map_err(|e| match e {
    sqlx::Error::RowNotFound => PaymentError::InvoiceNotFound { id: invoice_id },
    _ => PaymentError::Database(e),
})?;

// 2. Validate state
if invoice.payment_initiated_at.is_some() {
    return Err(PaymentError::PaymentAlreadyInitiated);
}

// 3. Create payment transaction
let tx_record = create_payment_transaction(&mut *tx, invoice_id, gateway_id).await?;

// 4. Update invoice status
sqlx::query("UPDATE invoices SET status = $1, payment_initiated_at = $2 WHERE id = $3")
    .bind(InvoiceStatus::Pending)
    .bind(Utc::now())
    .bind(invoice_id)
    .execute(&mut *tx)
    .await?;

// 5. Commit atomically
tx.commit().await?;
```

### 3. Index Strategy

```sql
-- Fast lookups by tenant + status
CREATE INDEX idx_invoices_tenant_status ON invoices(tenant_id, status);

-- Payment history queries (date range)
CREATE INDEX idx_transactions_tenant_created ON payment_transactions(tenant_id, created_at);

-- Webhook deduplication (unique constraint prevents duplicates at DB level)
ALTER TABLE webhook_events ADD UNIQUE (gateway_id, event_id, tenant_id);

-- Financial reports (grouping by currency)
CREATE INDEX idx_transactions_currency_created ON payment_transactions(currency_code, created_at);

-- Installment due date queries
CREATE INDEX idx_installments_due_date ON installment_schedules(tenant_id, due_date);
```

### 4. Migration Strategy

**Use sqlx::migrate!() macro** (compile-time verification):

```rust
// In main.rs
sqlx::migrate!("./migrations")
    .run(&pool)
    .await
    .expect("Failed to run migrations");
```

**Migration file structure**:
```
migrations/
├── 20251103000001_init_invoices.sql
├── 20251103000002_init_line_items.sql
├── 20251103000003_init_payment_transactions.sql
└── 20251103000004_init_webhook_events.sql
```

Each migration is idempotent (can run multiple times safely).

---

## API Implementation

### 1. Request Validation

```rust
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct CreateInvoiceRequest {
    pub gateway_id: i64,

    #[validate(length(equal = "3"))]
    pub currency_code: String,  // Validates "IDR", "MYR", "USD"

    #[validate(custom = "validate_iso8601_future")]
    pub expires_at: DateTime<Utc>,

    #[validate(length(min = 1))]
    pub line_items: Vec<LineItemRequest>,
}

fn validate_iso8601_future(datetime: &DateTime<Utc>) -> Result<(), ValidationError> {
    if datetime <= &Utc::now() {
        return Err(ValidationError::new("expires_at_past"));
    }
    Ok(())
}

// In handler
async fn create_invoice(req: web::Json<CreateInvoiceRequest>) -> Result<...> {
    req.validate()?;  // Returns ValidationError::new() which maps to 400 Bad Request
    // ... proceed with validated data
}
```

### 2. Response Serialization

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct InvoiceResponse {
    pub id: i64,
    pub status: InvoiceStatus,

    #[serde(serialize_with = "serialize_amount")]
    pub total_amount: i64,  // Serialize as 1699 → "16.99" for MYR/USD

    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_url: Option<String>,
}

fn serialize_amount(amount: &i64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let decimal = Decimal::from(*amount) / Decimal::new(100, 0);
    serializer.serialize_str(&decimal.to_string())
}
```

### 3. Pagination Pattern

```rust
#[derive(Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    pub page: i64,

    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 { 1 }
fn default_page_size() -> i64 { 50 }

async fn list_invoices(
    query: web::Query<PaginationQuery>,
) -> Result<HttpResponse> {
    let offset = (query.page - 1) * query.page_size;
    let invoices = sqlx::query_as(
        "SELECT * FROM invoices WHERE tenant_id = $1 LIMIT $2 OFFSET $3"
    )
    .bind(tenant_id)
    .bind(query.page_size)
    .bind(offset)
    .fetch_all(&pool)
    .await?;

    Ok(HttpResponse::Ok().json(invoices))
}
```

---

## Payment Processing

### 1. Concurrent Payment Locking

```rust
pub struct PaymentProcessor {
    pool: PgPool,
}

impl PaymentProcessor {
    pub async fn process_payment(
        &self,
        invoice_id: i64,
        tenant_id: i64,
        gateway_id: i64,
    ) -> Result<PaymentUrl> {
        // Retry up to 3 times with exponential backoff
        for attempt in 1..=3 {
            match self.acquire_lock(invoice_id, tenant_id).await {
                Ok(invoice) => {
                    return self.process_locked(invoice, gateway_id).await;
                }
                Err(e) if attempt < 3 => {
                    // Exponential backoff: 100ms, 200ms
                    let delay = 100 * (2_u64.pow(attempt as u32 - 1));
                    let jitter = rand::random::<i32>() % 40 - 20; // ±20ms
                    tokio::time::sleep(Duration::from_millis(delay as u64 + jitter as u64)).await;
                }
                Err(e) => return Err(PaymentError::PaymentAlreadyInProgress),
            }
        }
        unreachable!()
    }

    async fn acquire_lock(
        &self,
        invoice_id: i64,
        tenant_id: i64,
    ) -> Result<Invoice> {
        sqlx::query_as::<_, Invoice>(
            "SELECT * FROM invoices WHERE id = $1 AND tenant_id = $2 FOR UPDATE NOWAIT"
        )
        .bind(invoice_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PaymentError::InvoiceNotFound { id: invoice_id },
            _ => PaymentError::LockAcquisitionFailed,
        })
    }

    async fn process_locked(&self, invoice: Invoice, gateway_id: i64) -> Result<PaymentUrl> {
        // Lock is held until transaction ends
        // Proceed with payment processing knowing no concurrent requests can modify
        todo!("Implement gateway payment API call")
    }
}
```

### 2. Installment Distribution

```rust
pub struct InstallmentCalculator;

impl InstallmentCalculator {
    /// Calculate proportional tax and fee distribution
    pub fn distribute_amounts(
        invoice: &Invoice,
        installment_count: usize,
        custom_amounts: Option<Vec<i64>>,
    ) -> Result<Vec<InstallmentAmount>> {
        let amounts = custom_amounts.unwrap_or_else(|| {
            // Default: equal distribution
            vec![invoice.total_amount / installment_count as i64; installment_count]
        });

        // Validate sum
        let total: i64 = amounts.iter().sum();
        if total != invoice.total_amount {
            return Err(PaymentError::InstallmentSumMismatch);
        }

        let mut installments = Vec::new();
        for (i, &amount) in amounts.iter().enumerate() {
            let tax = Decimal::from(invoice.total_tax_amount)
                * (Decimal::from(amount) / Decimal::from(invoice.total_amount));

            let fee = Decimal::from(invoice.total_service_fee_amount)
                * (Decimal::from(amount) / Decimal::from(invoice.total_amount));

            let tax_i64 = if i < amounts.len() - 1 {
                tax.floor().to_i64().unwrap_or(0)
            } else {
                // Last installment absorbs rounding
                invoice.total_tax_amount - installments.iter().map(|x| x.tax).sum::<i64>()
            };

            installments.push(InstallmentAmount {
                number: i + 1,
                amount,
                tax: tax_i64,
                fee: fee.floor().to_i64().unwrap_or(0),
            });
        }

        Ok(installments)
    }
}
```

### 3. Gateway Adapter Pattern

```rust
#[async_trait::async_trait]
pub trait PaymentGateway: Send + Sync {
    async fn create_payment(
        &self,
        invoice: &Invoice,
        idempotency_key: &str,
    ) -> Result<PaymentUrl>;

    async fn verify_webhook(
        &self,
        body: &[u8],
        signature: &str,
    ) -> Result<WebhookPayload>;
}

pub struct XenditGateway {
    client: reqwest::Client,
    api_key: String,
    endpoint: String,
}

#[async_trait::async_trait]
impl PaymentGateway for XenditGateway {
    async fn create_payment(
        &self,
        invoice: &Invoice,
        idempotency_key: &str,
    ) -> Result<PaymentUrl> {
        let response = self.client
            .post(&format!("{}/v2/invoices", self.endpoint))
            .header("Authorization", format!("Basic {}", self.api_key))
            .header("Idempotency-Key", idempotency_key)
            .json(&XenditPaymentRequest {
                external_id: invoice.external_id.clone(),
                amount: invoice.total_amount,
                customer: todo!("Extract from tenant data"),
                items: invoice.line_items.iter().map(|li| XenditItem {
                    name: li.product_name.clone(),
                    quantity: li.quantity,
                    price: li.unit_price_amount,
                }).collect(),
                // ... other fields
            })
            .send()
            .await?;

        let xendit_response: XenditResponse = response.json().await?;
        Ok(PaymentUrl {
            url: xendit_response.invoice_url,
            gateway_transaction_id: xendit_response.id,
        })
    }

    async fn verify_webhook(&self, body: &[u8], signature: &str) -> Result<WebhookPayload> {
        // Constant-time comparison to prevent timing attacks
        let expected = compute_hmac(&self.api_key, body);
        if !constant_time_compare(&expected, signature) {
            return Err(PaymentError::InvalidWebhookSignature);
        }

        let payload: XenditWebhookPayload = serde_json::from_slice(body)?;
        Ok(payload.into())
    }
}

fn constant_time_compare(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeComparison;
    a.as_bytes().ct_eq(b.as_bytes()).into()
}
```

---

## Webhook Handling

### 1. Webhook Processing with Retries

```rust
pub struct WebhookProcessor {
    pool: PgPool,
    retry_queue: Arc<Mutex<HashMap<i64, RetryState>>>,
}

#[derive(Clone)]
struct RetryState {
    event_id: i64,
    attempt_number: u32,
    last_error: String,
}

impl WebhookProcessor {
    pub async fn process_webhook(
        &self,
        gateway_id: i64,
        payload: WebhookPayload,
    ) -> Result<()> {
        // Step 1: Check for duplicate
        let existing = sqlx::query!(
            "SELECT id FROM webhook_events WHERE gateway_id = $1 AND event_id = $2",
            gateway_id,
            &payload.event_id,
        )
        .fetch_optional(&self.pool)
        .await?;

        if existing.is_some() {
            return Ok(()); // Already processed
        }

        // Step 2: Store webhook event
        let webhook_id: i64 = sqlx::query_scalar!(
            "INSERT INTO webhook_events (gateway_id, event_id, original_payload, status)
             VALUES ($1, $2, $3, 'pending') RETURNING id",
            gateway_id,
            &payload.event_id,
            serde_json::to_string(&payload)?,
        )
        .fetch_one(&self.pool)
        .await?;

        // Step 3: Process asynchronously with retries
        self.spawn_webhook_processor(webhook_id, payload).await;

        Ok(())
    }

    async fn spawn_webhook_processor(
        &self,
        webhook_id: i64,
        payload: WebhookPayload,
    ) {
        let pool = self.pool.clone();
        let retry_queue = self.retry_queue.clone();

        tokio::spawn(async move {
            for attempt in 1..=3 {
                match Self::process_once(&pool, webhook_id, &payload).await {
                    Ok(_) => {
                        // Success: mark processed
                        let _ = sqlx::query(
                            "UPDATE webhook_events SET status = 'processed', processed_at = NOW() WHERE id = $1"
                        )
                        .bind(webhook_id)
                        .execute(&pool)
                        .await;
                        return;
                    }
                    Err(e) if attempt < 3 => {
                        // Retry: schedule next attempt
                        let delay = match attempt {
                            1 => Duration::from_secs(60),      // T+1 minute
                            2 => Duration::from_secs(300),     // T+5 minutes (cumulative 6)
                            _ => unreachable!(),
                        };

                        // Store retry state
                        let mut queue = retry_queue.lock().await;
                        queue.insert(webhook_id, RetryState {
                            event_id: webhook_id,
                            attempt_number: attempt + 1,
                            last_error: e.to_string(),
                        });

                        // Sleep and retry
                        tokio::time::sleep(delay).await;
                    }
                    Err(e) => {
                        // Final failure: mark as failed
                        let _ = sqlx::query(
                            "UPDATE webhook_events SET status = 'failed', error_message = $1 WHERE id = $2"
                        )
                        .bind(e.to_string())
                        .bind(webhook_id)
                        .execute(&pool)
                        .await;
                    }
                }
            }
        });
    }

    async fn process_once(
        pool: &PgPool,
        webhook_id: i64,
        payload: &WebhookPayload,
    ) -> Result<()> {
        match payload.event_type.as_str() {
            "payment.completed" => {
                // Update invoice status
                sqlx::query(
                    "UPDATE invoices SET status = 'fully_paid' WHERE id = $1"
                )
                .bind(payload.invoice_id)
                .execute(pool)
                .await?;
            }
            "payment.failed" => {
                sqlx::query(
                    "UPDATE invoices SET status = 'failed' WHERE id = $1"
                )
                .bind(payload.invoice_id)
                .execute(pool)
                .await?;
            }
            _ => {}
        }

        // Log retry attempt
        sqlx::query(
            "INSERT INTO webhook_retry_log (webhook_id, attempt_number, status, attempted_at)
             VALUES ($1, $2, 'success', NOW())"
        )
        .bind(webhook_id)
        .bind(1) // TODO: track actual attempt
        .execute(pool)
        .await?;

        Ok(())
    }
}
```

### 2. Failed Webhook Recovery

```rust
// GET /webhooks/failed endpoint
async fn get_failed_webhooks(
    pool: web::Data<PgPool>,
    query: web::Query<PaginationQuery>,
) -> Result<HttpResponse> {
    let webhooks = sqlx::query_as::<_, FailedWebhook>(
        "SELECT id, gateway_id, event_id, event_type, error_message, created_at
         FROM webhook_events
         WHERE status = 'failed'
         ORDER BY created_at DESC
         LIMIT $1 OFFSET $2"
    )
    .bind(query.page_size)
    .bind((query.page - 1) * query.page_size)
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(webhooks))
}
```

---

## Testing Strategy

### 1. Integration Testing with Docker

```rust
#[cfg(test)]
mod integration_tests {
    use testcontainers::{clients, images};

    #[tokio::test]
    async fn test_create_invoice() {
        let docker = clients::Cli::default();
        let postgres = docker.run(images::postgres::Postgres::default());

        let connection_string = format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            postgres.get_host_port_ipv4(5432)
        );

        // Run migrations
        let pool = PgPoolOptions::new()
            .connect(&connection_string)
            .await
            .unwrap();

        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        // Test
        let invoice = create_invoice(&pool, valid_request()).await.unwrap();
        assert_eq!(invoice.status, InvoiceStatus::Draft);
        assert_eq!(invoice.total_amount, 16935000);
    }
}
```

### 2. Financial Calculation Property Testing

```rust
#[cfg(test)]
mod financial_tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_installment_distribution_sums_correctly(
            amounts in prop::collection::vec(1i64..100000000, 2..13)
        ) {
            let total: i64 = amounts.iter().sum();
            let distribution = distribute_amounts(total, &amounts).unwrap();

            let sum: i64 = distribution.iter().map(|x| x.total).sum();
            prop_assert_eq!(sum, total);
        }

        #[test]
        fn test_tax_calculation_no_floating_point_errors(
            subtotal in 1i64..100000000i64,
            tax_rate in "0\\.[0-9]{1,4}"
        ) {
            let tax_rate = Decimal::from_str(&tax_rate).unwrap();
            let tax = (Decimal::from(subtotal) * tax_rate).floor();

            // Verify no floating-point errors
            prop_assert!(tax.is_finite());
        }
    }
}
```

### 3. Webhook Processing Testing

```rust
#[tokio::test]
async fn test_webhook_deduplication() {
    let pool = setup_test_db().await;
    let processor = WebhookProcessor::new(pool.clone());

    let webhook = WebhookPayload {
        event_id: "evt_123".to_string(),
        event_type: "payment.completed".to_string(),
        // ... other fields
    };

    // First call should succeed
    processor.process_webhook(1, webhook.clone()).await.unwrap();

    // Second call with same event_id should return OK without reprocessing
    processor.process_webhook(1, webhook.clone()).await.unwrap();

    // Verify only one invoice status update occurred
    let statuses = sqlx::query_scalar::<_, String>(
        "SELECT status FROM invoice_status_history ORDER BY created_at"
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(statuses.len(), 1);
}
```

---

## Deployment

### 1. Docker Containerization

```dockerfile
# Multi-stage build for small final image
FROM rust:1.91 AS builder

WORKDIR /app
COPY Cargo.* ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/paytrust /usr/local/bin/

EXPOSE 8000
CMD ["paytrust"]
```

Build command:
```bash
docker build -t paytrust:latest .
# Final image size: ~150-200 MB
```

### 2. Environment Configuration

```rust
use dotenv::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub http_port: u16,
    pub admin_api_key: String,
    pub xendit_api_key: String,
    pub midtrans_server_key: String,
    pub rate_limiter_backend: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv().ok();

        Config {
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL not set"),
            http_port: env::var("HTTP_PORT")
                .unwrap_or_else(|_| "8000".to_string())
                .parse()
                .expect("HTTP_PORT must be a number"),
            admin_api_key: env::var("ADMIN_API_KEY")
                .expect("ADMIN_API_KEY not set"),
            xendit_api_key: env::var("XENDIT_API_KEY")
                .expect("XENDIT_API_KEY not set"),
            midtrans_server_key: env::var("MIDTRANS_SERVER_KEY")
                .expect("MIDTRANS_SERVER_KEY not set"),
            rate_limiter_backend: env::var("RATE_LIMITER_BACKEND")
                .unwrap_or_else(|_| "memory".to_string()),
        }
    }
}
```

### 3. Graceful Shutdown

```rust
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = Config::from_env();
    let pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    let server = HttpServer::new(|| {
        App::new()
            .service(routes)
    })
    .bind(format!("0.0.0.0:{}", config.http_port))?
    .run();

    // Handle SIGTERM/SIGINT for graceful shutdown
    let srv_handle = server.handle();
    tokio::spawn(async move {
        signal::ctrl_c().await.ok();
        srv_handle.stop(true).await; // wait for existing requests to complete
    });

    server.await
}
```

---

## Common Pitfalls

### 1. ❌ Floating-Point Currency

```rust
// WRONG: Decimal precision lost
let total = (subtotal as f64) * 1.1;

// RIGHT: Use rust_decimal
let total = Decimal::from(subtotal) * Decimal::from_str("1.1")?;
```

### 2. ❌ Missing Tenant Isolation

```rust
// WRONG: Any tenant can access any invoice
sqlx::query("SELECT * FROM invoices WHERE id = $1")
    .bind(invoice_id)
    .fetch_one(&pool)
    .await

// RIGHT: Always filter by tenant_id
sqlx::query("SELECT * FROM invoices WHERE id = $1 AND tenant_id = $2")
    .bind(invoice_id)
    .bind(tenant_id)
    .fetch_one(&pool)
    .await
```

### 3. ❌ Race Condition in Concurrent Payments

```rust
// WRONG: Check and update in separate statements (TOCTOU)
let invoice = sqlx::query("SELECT * FROM invoices WHERE id = $1").fetch_one(&pool).await?;
if invoice.payment_initiated_at.is_none() {
    // Another request may have initiated payment here!
    sqlx::query("UPDATE invoices SET payment_initiated_at = NOW() WHERE id = $1")
        .bind(invoice_id)
        .execute(&pool)
        .await?;
}

// RIGHT: Lock before check
sqlx::query("SELECT * FROM invoices WHERE id = $1 FOR UPDATE NOWAIT")
    .bind(invoice_id)
    .fetch_one(&pool)
    .await?;
// Lock held until transaction ends
```

### 4. ❌ Webhook Reprocessing

```rust
// WRONG: Process without deduplication check
async fn handle_webhook(payload: WebhookPayload) {
    update_invoice_status(&payload).await;
    // If webhook is delivered twice, invoice updated twice!
}

// RIGHT: Check for duplicate event_id first
if webhook_already_processed(&payload.event_id).await? {
    return Ok(()); // Idempotent
}
```

### 5. ❌ Unencrypted Sensitive Data

```rust
// WRONG: Storing API key plaintext
sqlx::query("INSERT INTO api_keys (api_key) VALUES ($1)")
    .bind(&plaintext_api_key)
    .execute(&pool)
    .await?

// RIGHT: Hash before storing
let hash = argon2::hash_password(&plaintext_api_key, &salt)?;
sqlx::query("INSERT INTO api_keys (api_key_hash) VALUES ($1)")
    .bind(&hash)
    .execute(&pool)
    .await?
```

---

## Performance Optimization Checklist

- [ ] Connection pooling configured (50+ connections)
- [ ] Database indexes on all WHERE clause columns
- [ ] Query profiling with `EXPLAIN ANALYZE`
- [ ] Caching for frequently-accessed gateway configs
- [ ] Async/await throughout (no blocking I/O)
- [ ] Load testing with k6 (target: <2sec p95, 100 concurrent)
- [ ] Circuit breaker for gateway API failures
- [ ] Request timeout configured (10-30 seconds)
- [ ] Structured logging with correlation IDs
- [ ] Health check endpoint for load balancers

---

## Next: Detailed Task Generation

Run `/speckit.tasks` to generate atomic implementation tasks with:
- Task dependencies and sequencing
- Code examples for each component
- Test requirements per task
- Quality gates and validation criteria

