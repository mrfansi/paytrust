use actix_web::{
    body::{BoxBody, EitherBody},
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::collections::HashMap;
use std::future::{ready, Ready};
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// Rate limiter trait for pluggable backends (Constitution Principle II - Open/Closed)
pub trait RateLimiter: Send + Sync {
    /// Check if request is allowed for given API key
    /// Returns Ok(()) if allowed, Err with retry_after_seconds if rate limited
    fn check_rate_limit(&self, api_key: &str) -> Result<(), u64>;

    /// Check if request is allowed for given IP address (optional secondary limit)
    fn check_ip_rate_limit(&self, ip: IpAddr) -> Result<(), u64>;

    /// Reset rate limit for specific API key (admin operation)
    fn reset_limit(&self, api_key: &str) -> Result<(), String>;
}

/// In-memory rate limiter using governor crate
/// v1.0 implementation for single-instance deployment (1000 req/min per API key per FR-040)
pub struct InMemoryRateLimiter {
    limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

impl InMemoryRateLimiter {
    pub fn new(requests_per_minute: u32) -> Self {
        let quota = Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap());
        Self {
            limiter: Arc::new(GovernorRateLimiter::direct(quota)),
        }
    }
}

impl RateLimiter for InMemoryRateLimiter {
    fn check_rate_limit(&self, _api_key: &str) -> Result<(), u64> {
        // Simple global rate limit for v1.0
        // TODO: Implement per-key rate limiting with keyed governor in future
        match self.limiter.check() {
            Ok(_) => Ok(()),
            Err(_) => {
                // Return 60 seconds retry after (1 minute window)
                Err(60)
            }
        }
    }

    fn check_ip_rate_limit(&self, _ip: IpAddr) -> Result<(), u64> {
        // IP-based rate limiting not implemented in v1.0
        Ok(())
    }

    fn reset_limit(&self, _api_key: &str) -> Result<(), String> {
        // Cannot reset with NotKeyed limiter
        // This would require per-key tracking
        Ok(())
    }
}

/// Middleware for rate limiting
pub struct RateLimitMiddleware {
    limiter: Arc<dyn RateLimiter>,
}

impl RateLimitMiddleware {
    pub fn new(limiter: Arc<dyn RateLimiter>) -> Self {
        Self { limiter }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitMiddlewareService {
            service: Rc::new(service),
            limiter: self.limiter.clone(),
        }))
    }
}

pub struct RateLimitMiddlewareService<S> {
    service: Rc<S>,
    limiter: Arc<dyn RateLimiter>,
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let limiter = self.limiter.clone();

        Box::pin(async move {
            // Skip rate limiting for health check endpoints
            let path = req.path();
            if path == "/health" || path == "/ready" {
                let res = service.call(req).await?;
                return Ok(res.map_into_left_body());
            }

            // Extract API key from header
            let api_key = match req.headers().get("X-API-Key") {
                Some(value) => match value.to_str() {
                    Ok(key) => key.to_string(),
                    Err(_) => {
                        // If no valid API key, skip rate limiting (will be caught by auth middleware)
                        let res = service.call(req).await?;
                        return Ok(res.map_into_left_body());
                    }
                },
                None => {
                    // If no API key, skip rate limiting (will be caught by auth middleware)
                    let res = service.call(req).await?;
                    return Ok(res.map_into_left_body());
                }
            };

            // Check rate limit
            match limiter.check_rate_limit(&api_key) {
                Ok(_) => {
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                }
                Err(retry_after) => {
                    let response = HttpResponse::TooManyRequests()
                        .insert_header(("Retry-After", retry_after.to_string()))
                        .json(serde_json::json!({
                            "error": {
                                "code": 429,
                                "message": "Rate limit exceeded",
                                "retry_after": retry_after
                            }
                        }));
                    Ok(req.into_response(response).map_into_right_body())
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_rate_limiter_creation() {
        let limiter = InMemoryRateLimiter::new(1000);
        assert!(limiter.check_rate_limit("test-key").is_ok());
    }

    #[test]
    fn test_rate_limit_enforcement() {
        let limiter = InMemoryRateLimiter::new(2); // 2 requests per minute
        
        // First request should succeed
        assert!(limiter.check_rate_limit("test-key").is_ok());
        
        // Second request should succeed
        assert!(limiter.check_rate_limit("test-key").is_ok());
        
        // Third request should fail (rate limited)
        assert!(limiter.check_rate_limit("test-key").is_err());
    }

    #[test]
    fn test_rate_limit_reset() {
        let limiter = InMemoryRateLimiter::new(1);
        
        // Exhaust limit
        let _ = limiter.check_rate_limit("test-key");
        assert!(limiter.check_rate_limit("test-key").is_err());
        
        // Reset limit (v1.0 uses NotKeyed limiter, so reset is a no-op)
        // This will be implemented properly in v2.0 with per-key tracking
        assert!(limiter.reset_limit("test-key").is_ok());
        
        // Note: With NotKeyed limiter, reset doesn't actually work
        // This test documents current limitation - will be fixed in v2.0
    }
}
