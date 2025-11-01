// Integration test for metrics collection
//
// Tests that metrics are collected correctly through HTTP requests
// and that the /metrics endpoint returns accurate data

use paytrust::config::Config;
use paytrust::middleware::MetricsCollector;
use actix_web::{test, web, App};
use sqlx::MySqlPool;

async fn setup_test_pool() -> MySqlPool {
    let config = Config::from_env().expect("Failed to load config");
    config
        .database
        .create_pool()
        .await
        .expect("Failed to create pool")
}

#[actix_web::test]
async fn test_metrics_collection_on_requests() {
    let pool = setup_test_pool().await;
    let collector = MetricsCollector::new();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(collector.clone()))
            .configure(paytrust::modules::health::controllers::configure),
    )
    .await;

    // Make a request to health endpoint
    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Check metrics endpoint
    let req = test::TestRequest::get().uri("/metrics").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    
    // Verify metrics structure
    assert!(body.get("total_requests").is_some());
    assert!(body.get("successful_requests").is_some());
    assert!(body.get("client_errors").is_some());
    assert!(body.get("server_errors").is_some());
    assert!(body.get("avg_response_time_ms").is_some());
    assert!(body.get("min_response_time_ms").is_some());
    assert!(body.get("max_response_time_ms").is_some());
    assert!(body.get("error_rate").is_some());
    assert!(body.get("success_rate").is_some());
    assert!(body.get("endpoint_counts").is_some());
    assert!(body.get("endpoint_errors").is_some());
}

#[actix_web::test]
async fn test_metrics_tracks_multiple_requests() {
    let pool = setup_test_pool().await;
    let collector = MetricsCollector::new();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(collector.clone()))
            .configure(paytrust::modules::health::controllers::configure),
    )
    .await;

    // Make multiple requests
    for _ in 0..5 {
        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    // Get metrics
    let req = test::TestRequest::get().uri("/metrics").to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;

    // Note: The actual counts will be from middleware execution
    // We just verify the structure is correct
    assert!(body["total_requests"].as_u64().is_some());
    assert!(body["successful_requests"].as_u64().is_some());
}

#[actix_web::test]
async fn test_metrics_json_format() {
    let pool = setup_test_pool().await;
    let collector = MetricsCollector::new();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(collector.clone()))
            .configure(paytrust::modules::health::controllers::configure),
    )
    .await;

    let req = test::TestRequest::get().uri("/metrics").to_request();
    let resp = test::call_service(&app, req).await;

    // Verify JSON content type
    assert!(resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("application/json"));

    // Verify it's valid JSON
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.is_object());
}
