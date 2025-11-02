use actix_web::{test, web, App, HttpResponse};
use paytrust::middleware::rate_limit::{InMemoryRateLimiter, RateLimitMiddleware};
use std::sync::Arc;

async fn test_handler() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

#[actix_web::test]
async fn test_rate_limit_enforcement_1000_per_minute() {
    // Create rate limiter with 1000 req/min limit per FR-040
    let limiter = Arc::new(InMemoryRateLimiter::new(1000));
    
    let app = test::init_service(
        App::new()
            .wrap(RateLimitMiddleware::new(limiter))
            .route("/test", web::get().to(test_handler))
    )
    .await;

    // Make 1000 requests - all should succeed
    for i in 0..1000 {
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("X-API-Key", "test-api-key"))
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status().as_u16(),
            200,
            "Request {} should succeed within rate limit",
            i + 1
        );
    }

    // 1001st request should be rate limited
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("X-API-Key", "test-api-key"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status().as_u16(),
        429,
        "Request 1001 should be rate limited"
    );
}

#[actix_web::test]
async fn test_rate_limit_429_response_with_retry_after_header() {
    // Create rate limiter with low limit for easier testing
    let limiter = Arc::new(InMemoryRateLimiter::new(2));
    
    let app = test::init_service(
        App::new()
            .wrap(RateLimitMiddleware::new(limiter))
            .route("/test", web::get().to(test_handler))
    )
    .await;

    // Exhaust rate limit
    for _ in 0..2 {
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("X-API-Key", "test-api-key"))
            .to_request();
        let _ = test::call_service(&app, req).await;
    }

    // Next request should return 429 with Retry-After header per FR-041
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("X-API-Key", "test-api-key"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    // Verify 429 status
    assert_eq!(resp.status().as_u16(), 429, "Should return 429 Too Many Requests");
    
    // Verify Retry-After header exists per FR-041
    let retry_after = resp.headers().get("Retry-After");
    assert!(retry_after.is_some(), "Retry-After header must be present per FR-041");
    
    // Verify Retry-After value is numeric (seconds)
    let retry_value = retry_after.unwrap().to_str().unwrap();
    let retry_seconds: u64 = retry_value.parse().expect("Retry-After should be numeric");
    assert!(retry_seconds > 0, "Retry-After should be positive number of seconds");
    
    // Verify response body contains error details
    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], 429);
    assert_eq!(json["error"]["message"], "Rate limit exceeded");
    assert!(json["error"]["retry_after"].is_number());
}

#[actix_web::test]
async fn test_rate_limit_per_api_key() {
    // Create rate limiter
    let limiter = Arc::new(InMemoryRateLimiter::new(5));
    
    let app = test::init_service(
        App::new()
            .wrap(RateLimitMiddleware::new(limiter))
            .route("/test", web::get().to(test_handler))
    )
    .await;

    // Note: Current v1.0 implementation uses global rate limit (NotKeyed)
    // This test documents expected behavior for v2.0 with per-key tracking
    // For now, we test that rate limiting works regardless of API key
    
    // Make requests with first API key
    for _ in 0..3 {
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("X-API-Key", "api-key-1"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 200);
    }

    // Make requests with second API key (shares same global limit in v1.0)
    for _ in 0..2 {
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("X-API-Key", "api-key-2"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 200);
    }

    // Next request should be rate limited (global limit reached)
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("X-API-Key", "api-key-2"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 429);
}

#[actix_web::test]
async fn test_rate_limit_skips_health_endpoints() {
    // Create rate limiter with very low limit
    let limiter = Arc::new(InMemoryRateLimiter::new(1));
    
    let app = test::init_service(
        App::new()
            .wrap(RateLimitMiddleware::new(limiter))
            .route("/health", web::get().to(test_handler))
            .route("/ready", web::get().to(test_handler))
            .route("/test", web::get().to(test_handler))
    )
    .await;

    // Exhaust rate limit with regular endpoint
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("X-API-Key", "test-key"))
        .to_request();
    let _ = test::call_service(&app, req).await;

    // Health endpoints should still work even when rate limited
    let health_req = test::TestRequest::get()
        .uri("/health")
        .to_request();
    let health_resp = test::call_service(&app, health_req).await;
    assert_eq!(health_resp.status().as_u16(), 200, "/health should bypass rate limiting");

    let ready_req = test::TestRequest::get()
        .uri("/ready")
        .to_request();
    let ready_resp = test::call_service(&app, ready_req).await;
    assert_eq!(ready_resp.status().as_u16(), 200, "/ready should bypass rate limiting");

    // Regular endpoint should still be rate limited
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("X-API-Key", "test-key"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 429, "/test should be rate limited");
}

#[actix_web::test]
async fn test_rate_limit_without_api_key() {
    // Create rate limiter
    let limiter = Arc::new(InMemoryRateLimiter::new(5));
    
    let app = test::init_service(
        App::new()
            .wrap(RateLimitMiddleware::new(limiter))
            .route("/test", web::get().to(test_handler))
    )
    .await;

    // Request without API key should skip rate limiting
    // (will be caught by auth middleware later in the chain)
    let req = test::TestRequest::get()
        .uri("/test")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should pass through rate limiter (200 from handler, not 429)
    assert_eq!(resp.status().as_u16(), 200, "Requests without API key should skip rate limiting");
}
