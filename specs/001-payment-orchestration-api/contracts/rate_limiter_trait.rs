/// Rate limiting abstraction for API request throttling
/// 
/// This trait enables pluggable rate limiting backends while maintaining
/// Open/Closed principle compliance (Constitution Principle II).
/// 
/// v1.0 Implementation: InMemoryRateLimiter using governor crate
/// Future: RedisRateLimiter for distributed multi-instance deployments

use std::net::IpAddr;

pub trait RateLimiter: Send + Sync {
    /// Check if request is allowed for given API key
    /// Returns Ok(()) if allowed, Err with retry_after_seconds if rate limited
    fn check_rate_limit(&self, api_key: &str) -> Result<(), u64>;
    
    /// Check if request is allowed for given IP address (optional secondary limit)
    fn check_ip_rate_limit(&self, ip: IpAddr) -> Result<(), u64>;
    
    /// Reset rate limit for specific API key (admin operation)
    fn reset_limit(&self, api_key: &str) -> Result<(), String>;
}

/// In-memory rate limiter implementation using governor crate
/// Suitable for single-instance deployments only
pub struct InMemoryRateLimiter {
    // Implementation uses governor::RateLimiter internally
    // 1000 requests per minute per API key per FR-040
}

/// Future: Redis-backed distributed rate limiter
/// Required for horizontal scaling (multi-instance deployment)
pub struct RedisRateLimiter {
    // Implementation uses Redis with sliding window algorithm
    // Enables rate limiting across multiple application instances
}
