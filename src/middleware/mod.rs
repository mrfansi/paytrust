pub mod auth;
pub mod error_handler;
pub mod rate_limit;

pub use auth::{ApiKeyAuth, TenantId};
pub use error_handler::{error_response, ErrorHandler};
pub use rate_limit::{InMemoryRateLimiter, RateLimitMiddleware, RateLimiter};

use actix_cors::Cors;
use actix_web::http;

/// Configure CORS middleware for API
pub fn configure_cors() -> Cors {
    Cors::default()
        .allowed_origin_fn(|origin, _req_head| {
            // Allow all origins in development, restrict in production
            origin.as_bytes().starts_with(b"http://localhost")
                || origin.as_bytes().starts_with(b"http://127.0.0.1")
                || origin.as_bytes().starts_with(b"https://")
        })
        .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"])
        .allowed_headers(vec![
            http::header::AUTHORIZATION,
            http::header::ACCEPT,
            http::header::CONTENT_TYPE,
            http::header::HeaderName::from_static("x-api-key"),
        ])
        .expose_headers(vec![http::header::HeaderName::from_static("retry-after")])
        .max_age(3600)
}
