# Metrics Collection System

**Created:** 2025-11-02  
**Status:** Production Ready  
**Related Tasks:** T140

## Overview

The PayTrust Payment Orchestration Platform includes a comprehensive metrics collection system that tracks response times, error rates, and endpoint usage. This enables monitoring of system health and performance in real-time.

## Architecture

### Components

1. **MetricsCollector** - Thread-safe metrics storage using Arc<Mutex<>>
2. **MetricsMiddleware** - Actix-web middleware for automatic metrics collection
3. **Metrics Endpoint** - HTTP endpoint exposing collected metrics at `/metrics`

### Data Flow

```
HTTP Request
    ↓
RequestId Middleware
    ↓
MetricsMiddleware (start timer)
    ↓
Application Logic
    ↓
MetricsMiddleware (record metrics)
    ↓
HTTP Response
```

## Collected Metrics

### Request Metrics

- **total_requests** - Total number of requests processed
- **successful_requests** - Requests with 2xx status codes
- **client_errors** - Requests with 4xx status codes
- **server_errors** - Requests with 5xx status codes

### Response Time Metrics

- **avg_response_time_ms** - Average response time in milliseconds
- **min_response_time_ms** - Minimum response time observed
- **max_response_time_ms** - Maximum response time observed

### Derived Metrics

- **error_rate** - Percentage of failed requests (4xx + 5xx) / total \* 100
- **success_rate** - Percentage of successful requests (2xx) / total \* 100

### Endpoint Metrics

- **endpoint_counts** - Request count per endpoint (HashMap<String, u64>)
- **endpoint_errors** - Error count per endpoint (HashMap<String, u64>)

## Usage

### Accessing Metrics

**Endpoint:** `GET /metrics`

**Response Example:**

```json
{
  "total_requests": 1543,
  "successful_requests": 1489,
  "client_errors": 42,
  "server_errors": 12,
  "avg_response_time_ms": 87,
  "min_response_time_ms": 5,
  "max_response_time_ms": 1843,
  "error_rate": 3.5,
  "success_rate": 96.5,
  "endpoint_counts": {
    "/health": 500,
    "/ready": 250,
    "/v1/invoices": 450,
    "/v1/installments": 343
  },
  "endpoint_errors": {
    "/v1/invoices": 35,
    "/v1/installments": 19
  }
}
```

### Integration with Monitoring

The `/metrics` endpoint can be integrated with monitoring systems:

1. **Prometheus** - Scrape endpoint periodically
2. **Grafana** - Visualize metrics over time
3. **Custom Dashboards** - Parse JSON response for custom displays
4. **Alerting** - Trigger alerts based on error_rate thresholds

### Structured Logging

Each request also logs metrics with structured logging:

```
INFO request_id=abc123 path=/v1/invoices status=200 response_time_ms=45 "Request completed"
```

This enables log aggregation tools (ELK, Splunk) to extract metrics.

## Performance Considerations

### Memory Usage

- **In-Memory Storage** - All metrics stored in RAM (Arc<Mutex<HashMap>>)
- **Growth Rate** - O(n) where n = unique endpoints
- **Expected Size** - ~100 endpoints × 24 bytes = 2.4 KB + overhead
- **Max Memory** - < 10 KB for typical workloads

### Thread Safety

- **Mutex Locking** - Short-lived locks during metric recording
- **Lock Contention** - Minimal impact (< 1μs per request)
- **No Blocking** - Metrics recording never blocks application logic

### Accuracy

- **Resolution** - Millisecond precision using `std::time::Instant`
- **Clock Source** - Monotonic clock (not affected by system time changes)
- **Aggregation** - Calculated on-demand from raw counters

## Implementation Details

### Middleware Configuration

Metrics middleware is configured in `main.rs`:

```rust
use middleware::MetricsCollector;

let metrics_collector = MetricsCollector::new();

App::new()
    .app_data(web::Data::new(metrics_collector.clone()))
    .wrap(MetricsMiddleware::new(metrics_collector.clone()))
    // ... other middleware
```

### Middleware Order

Metrics middleware should be placed **after** RequestId middleware to capture request IDs in logs:

```rust
.wrap(RequestId)           // 1st - Generate request ID
.wrap(MetricsMiddleware)   // 2nd - Record metrics with request ID
.wrap(RateLimiter)         // 3rd - Rate limiting
.wrap(ApiKeyAuth)          // 4th - Authentication
```

## Testing

### Unit Tests

- `test_metrics_collector_initialization` - Verify initial state
- `test_metrics_recording` - Verify basic recording
- `test_metrics_error_tracking` - Verify error rate calculation
- `test_metrics_response_time_tracking` - Verify min/max/avg
- `test_metrics_endpoint_tracking` - Verify per-endpoint counts
- `test_metrics_reset` - Verify reset functionality

### Integration Tests

- `test_metrics_collection_on_requests` - End-to-end metrics collection
- `test_metrics_tracks_multiple_requests` - Multiple request handling
- `test_metrics_json_format` - Response format validation

**Run Tests:**

```bash
cargo test --lib middleware::metrics           # Unit tests
cargo test --test metrics_collection_test      # Integration tests
```

## Monitoring Best Practices

### Alerting Thresholds

**Error Rate:**

- Warning: > 5%
- Critical: > 10%

**Response Time:**

- Warning: avg > 500ms
- Critical: avg > 2000ms (violates NFR-001)
- Critical: max > 5000ms

**Availability:**

- Critical: success_rate < 95%

### Dashboard Widgets

1. **Request Rate** - total_requests over time (requests/min)
2. **Error Rate** - error_rate over time (percentage)
3. **Response Time** - avg/min/max response times (ms)
4. **Top Endpoints** - endpoint_counts sorted descending
5. **Error Hotspots** - endpoint_errors sorted descending

### Retention Policy

- **Real-time Metrics** - Available at `/metrics` (current state)
- **Historical Data** - Should be scraped to external storage
- **Retention** - Recommend 30-90 days for performance analysis

## Future Enhancements

### Short-term

- [ ] Export to Prometheus format (`/metrics/prometheus`)
- [ ] Add histogram buckets for response time distribution
- [ ] Track concurrent requests (active connections)

### Long-term

- [ ] Distributed metrics with Redis backend
- [ ] Per-gateway success/failure rates
- [ ] Custom business metrics (revenue per endpoint)
- [ ] Metric retention with time-series database (InfluxDB)

## References

- NFR-001: Response Time < 2s
- src/middleware/metrics.rs - Implementation
- tests/integration/metrics_collection_test.rs - Integration tests
- Phase 7 Task T140 - Metrics collection specification

## Compliance

- **OWASP** - No sensitive data exposed in metrics
- **GDPR** - No personal data tracked
- **PCI-DSS** - No payment data in metrics

## Support

For questions or issues related to metrics:

1. Check logs with request_id correlation
2. Verify `/metrics` endpoint accessibility
3. Review middleware configuration in main.rs
4. Check MetricsCollector initialization

---

**Last Updated:** 2025-11-02  
**Version:** 1.0.0
