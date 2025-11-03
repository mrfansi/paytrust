// Rate Limiter Trait Contract
// Defines the interface for rate limiting implementations
// v1.0 uses InMemoryRateLimiter; future versions can use RedisRateLimiter for horizontal scaling

use std::fmt;

/// Rate limiting decision returned by check_limit
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitDecision {
    /// Request allowed; no limit exceeded
    Allowed,
    /// Request rejected; limit exceeded
    /// retry_after_seconds: time to wait before next request
    Exceeded {
        retry_after_seconds: u64,
    },
}

impl fmt::Display for RateLimitDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RateLimitDecision::Allowed => write!(f, "allowed"),
            RateLimitDecision::Exceeded {
                retry_after_seconds,
            } => write!(
                f,
                "exceeded (retry after {} seconds)",
                retry_after_seconds
            ),
        }
    }
}

/// RateLimiter trait defines the interface for rate limiting implementations
///
/// Implementations MUST enforce 1000 requests per minute per API key using a sliding window
/// algorithm. All implementations must be thread-safe and async-compatible.
///
/// # Implementations
///
/// - `InMemoryRateLimiter`: In-process HashMap-based storage (v1.0, single instance only)
/// - `RedisRateLimiter`: Distributed Redis backend (future multi-instance scaling)
///
/// # Example
///
/// ```ignore
/// let limiter = InMemoryRateLimiter::new();
/// let api_key_id = 12345i64;
///
/// match limiter.check_limit(api_key_id).await {
///     RateLimitDecision::Allowed => {
///         // Process request
///     }
///     RateLimitDecision::Exceeded {
///         retry_after_seconds,
///     } => {
///         // Return 429 Too Many Requests with Retry-After header
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait RateLimiter: Send + Sync {
    /// Check if a request from the given API key is allowed under rate limit
    ///
    /// # Arguments
    ///
    /// * `api_key_id` - The API key identifier
    ///
    /// # Returns
    ///
    /// - `RateLimitDecision::Allowed` if request is allowed
    /// - `RateLimitDecision::Exceeded { retry_after_seconds }` if limit exceeded
    ///
    /// # Rate Limit Specification
    ///
    /// - **Limit**: 1000 requests per minute per API key
    /// - **Window**: 60-second sliding window
    /// - **Behavior**: Count requests in the current 60-second window;
    ///   reject if count >= 1000
    ///
    /// # Thread Safety
    ///
    /// Implementation must be thread-safe and support concurrent calls from
    /// multiple actix-web worker threads.
    ///
    /// # Async Compatibility
    ///
    /// Must support async/await patterns for tokio runtime integration.
    async fn check_limit(&self, api_key_id: i64) -> RateLimitDecision;

    /// Reset rate limit state for testing (optional, implementation-dependent)
    ///
    /// This method is provided for test utilities and may not be implemented
    /// by all backends. Default implementation is a no-op.
    async fn reset(&self, _api_key_id: i64) {
        // No-op by default
    }
}

/// InMemoryRateLimiter implementation using HashMap and tokio::sync::Mutex
///
/// Stores rate limit state in process memory using a HashMap<api_key_id, RateLimitState>.
/// Suitable for single-instance deployments (v1.0 per NFR-009).
///
/// # Implementation Details
///
/// - Thread-safe using tokio::sync::Mutex
/// - Sliding window tracking: maintains request timestamps in current 60-second window
/// - Memory-efficient: automatically removes expired window entries
/// - Concurrent handling: supports multiple actix-web worker threads
///
/// # Limitations
///
/// - In-memory only; rate limit state lost on restart
/// - Not suitable for horizontal scaling (multi-instance deployments)
/// - Memory usage grows with number of active API keys
///
/// # Future Scaling
///
/// For multi-instance deployments, replace with `RedisRateLimiter` that uses
/// Redis Sorted Sets for distributed sliding window tracking. Configuration
/// can select implementation via `RATE_LIMITER_BACKEND` environment variable:
/// - `RATE_LIMITER_BACKEND=memory` → InMemoryRateLimiter (default, single instance)
/// - `RATE_LIMITER_BACKEND=redis` → RedisRateLimiter (multi-instance)
///
/// # Example
///
/// ```ignore
/// use paytrust::rate_limit::InMemoryRateLimiter;
///
/// let limiter = InMemoryRateLimiter::new();
/// let decision = limiter.check_limit(12345).await;
/// assert_eq!(decision, RateLimitDecision::Allowed);
/// ```
pub struct InMemoryRateLimiter {
    // Implementation detail: use std::collections::HashMap<i64, Vec<Instant>>
    // where i64 = api_key_id and Vec<Instant> = request timestamps in current window
}

impl InMemoryRateLimiter {
    /// Create a new InMemoryRateLimiter
    ///
    /// # Returns
    ///
    /// A new rate limiter instance ready for use
    pub fn new() -> Self {
        todo!("Implement in-memory rate limiter with sliding window")
    }
}

#[async_trait::async_trait]
impl RateLimiter for InMemoryRateLimiter {
    async fn check_limit(&self, api_key_id: i64) -> RateLimitDecision {
        todo!("Implement sliding window rate limiting (1000 req/min)")
    }

    async fn reset(&self, api_key_id: i64) {
        todo!("Reset rate limit state for given API key")
    }
}

/// RedisRateLimiter implementation (future enhancement)
///
/// Uses Redis Sorted Sets for distributed sliding window rate limiting across
/// multiple instances. Enables horizontal scaling for high-throughput deployments.
///
/// # Implementation Pattern
///
/// ```ignore
/// // Redis key: rate_limit:{api_key_id}
/// // Type: Sorted Set
/// // Member: request timestamp (score = Unix timestamp)
/// // Operations:
/// //   1. Add current timestamp to sorted set: ZADD
/// //   2. Remove timestamps older than 60 seconds: ZREMRANGEBYSCORE
/// //   3. Count remaining requests: ZCARD
/// //   4. Set TTL on key: EXPIRE (avoid memory growth)
/// ```
///
/// # Advantages Over In-Memory
///
/// - Shared state across instances (enables horizontal scaling)
/// - Persistent across restarts (optional, depends on Redis persistence)
/// - Better memory management (Redis handles expiration)
/// - Production-ready for multi-instance deployments
///
/// # Limitations
///
/// - Requires Redis deployment and maintenance
/// - Additional network latency vs in-memory
/// - Redis dependency adds operational complexity
///
/// # Future Implementation
///
/// To implement RedisRateLimiter:
/// 1. Add redis crate to Cargo.toml: `redis = "0.24"`
/// 2. Create RedisRateLimiter struct wrapping Redis client
/// 3. Implement RateLimiter trait using ZADD/ZREMRANGEBYSCORE/ZCARD
/// 4. Update main.rs to select implementation based on environment variable
/// 5. Add integration tests against test Redis instance
#[allow(dead_code)]
pub struct RedisRateLimiter {
    // Future implementation: redis::aio::ConnectionManager
}

/// Middleware for actix-web integration
///
/// Applies rate limiting to all API requests using the configured RateLimiter.
/// Returns 429 Too Many Requests when limit exceeded.
///
/// # Usage in main.rs
///
/// ```ignore
/// use actix_web::web::Data;
/// use paytrust::rate_limit::{RateLimiter, InMemoryRateLimiter};
/// use paytrust::middleware::RateLimitMiddleware;
///
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     let rate_limiter: Data<dyn RateLimiter> = Data::new(InMemoryRateLimiter::new());
///
///     HttpServer::new(move || {
///         App::new()
///             .app_data(rate_limiter.clone())
///             .wrap(RateLimitMiddleware::new(rate_limiter.clone()))
///             .service(invoices_routes)
///     })
///     .bind("0.0.0.0:8000")?
///     .run()
///     .await
/// }
/// ```
pub struct RateLimitMiddleware {
    // Implementation detail: wraps RateLimiter for middleware integration
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test cases MUST verify:
    // 1. First 1000 requests allowed
    // 2. Request 1001 rejected with Exceeded
    // 3. retry_after_seconds returned with Exceeded
    // 4. Window sliding: requests older than 60s don't count toward limit
    // 5. Multiple API keys isolated (key A's limit doesn't affect key B)

    #[tokio::test]
    async fn test_rate_limit_allows_under_limit() {
        let limiter = InMemoryRateLimiter::new();
        let decision = limiter.check_limit(12345).await;
        assert_eq!(decision, RateLimitDecision::Allowed);
    }

    #[tokio::test]
    async fn test_rate_limit_rejects_over_limit() {
        // TODO: After 1000 requests, should reject
        // This requires sending 1000 requests, use test utility
        todo!("Verify rejection at 1001st request")
    }

    #[tokio::test]
    async fn test_sliding_window_expiration() {
        // TODO: Verify old requests don't count toward limit
        todo!("Verify 60-second window expiration")
    }

    #[tokio::test]
    async fn test_api_key_isolation() {
        // TODO: Verify key A's requests don't affect key B's limit
        let limiter = InMemoryRateLimiter::new();
        let decision_a = limiter.check_limit(111).await;
        let decision_b = limiter.check_limit(222).await;
        assert_eq!(decision_a, RateLimitDecision::Allowed);
        assert_eq!(decision_b, RateLimitDecision::Allowed);
    }
}

// Configuration enum for environment-based rate limiter selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimiterConfig {
    Memory,  // InMemoryRateLimiter (default, single instance)
    Redis,   // RedisRateLimiter (distributed, multi-instance)
}

impl RateLimiterConfig {
    /// Load configuration from RATE_LIMITER_BACKEND environment variable
    ///
    /// Defaults to Memory if not specified
    pub fn from_env() -> Self {
        match std::env::var("RATE_LIMITER_BACKEND") {
            Ok(value) => match value.to_lowercase().as_str() {
                "redis" => RateLimiterConfig::Redis,
                "memory" | _ => RateLimiterConfig::Memory,
            },
            Err(_) => RateLimiterConfig::Memory,
        }
    }
}
