use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, ResponseError,
};
use futures_util::future::LocalBoxFuture;
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::future::{ready, Ready};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;

use crate::core::AppError;

/// Rate limiting middleware using governor
pub struct RateLimiter {
    limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

impl RateLimiter {
    /// Create a new rate limiter with specified requests per minute
    pub fn new(requests_per_minute: u32) -> Self {
        let quota = Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap());
        let limiter = Arc::new(GovernorRateLimiter::direct(quota));

        Self { limiter }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimiter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<actix_web::body::EitherBody<actix_web::body::BoxBody, B>>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimiterMiddleware<S>;
    type Future = Ready<std::result::Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimiterMiddleware {
            service: Rc::new(service),
            limiter: self.limiter.clone(),
        }))
    }
}

pub struct RateLimiterMiddleware<S> {
    service: Rc<S>,
    limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

impl<S, B> Service<ServiceRequest> for RateLimiterMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<actix_web::body::EitherBody<actix_web::body::BoxBody, B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, std::result::Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let limiter = self.limiter.clone();

        Box::pin(async move {
            // Skip rate limiting for health check
            if req.path() == "/health" || req.path() == "/" {
                return svc.call(req).await.map(|res| res.map_into_right_body());
            }

            // Check rate limit
            match limiter.check() {
                Ok(_) => {
                    // Allowed - forward to next service
                    svc.call(req).await.map(|res| res.map_into_right_body())
                }
                Err(_) => {
                    // Rate limit exceeded - return error response
                    let error_response = AppError::RateLimitExceeded(
                        "Rate limit exceeded. Maximum 1000 requests per minute.".to_string(),
                    );
                    let http_response = error_response.error_response();
                    Ok(req.into_response(http_response).map_into_left_body())
                }
            }
        })
    }
}
