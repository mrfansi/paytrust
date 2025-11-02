// Example: Using Test Metrics Collection
//
// This example demonstrates how to use TestMetrics for performance monitoring
// in integration tests per FR-009 requirement.

use tests::helpers::*;
use std::time::Instant;

#[actix_web::test]
async fn example_test_with_metrics() {
    // Spawn test server with metrics collection
    let (srv, metrics) = spawn_test_server_with_metrics().await;
    let client = TestClient::new(srv.url("").to_string());
    
    // Test 1: Health check
    let start = Instant::now();
    let response = client.get("/health").await;
    metrics.record_request(start.elapsed());
    
    if let Ok(_) = response {
        // Health check succeeded
    }
    
    // Test 2: Create invoice (when API implemented)
    let external_id = TestDataFactory::random_external_id();
    let payload = TestDataFactory::create_invoice_payload();
    
    let start = Instant::now();
    let response = client.post_json("/api/invoices", &payload).await;
    metrics.record_request(start.elapsed());
    
    // Record database query count if known
    metrics.record_query();
    
    // Report metrics at end of test
    metrics.report();
    
    // Example output:
    // {
    //   "http_request_count": 2,
    //   "http_avg_latency_ms": "45.23",
    //   "http_min_latency_ms": "12.45",
    //   "http_max_latency_ms": "78.01",
    //   "http_p50_latency_ms": "45.23",
    //   "http_p95_latency_ms": "74.11",
    //   "http_p99_latency_ms": "78.01",
    //   "db_query_count": 1,
    //   "test_duration_ms": 156
    // }
}

#[actix_web::test]
async fn example_performance_test_with_thresholds() {
    let (srv, metrics) = spawn_test_server_with_metrics().await;
    let client = TestClient::new(srv.url("").to_string());
    
    // Run multiple requests
    for _ in 0..10 {
        let start = Instant::now();
        let _ = client.get("/health").await;
        metrics.record_request(start.elapsed());
    }
    
    // Get metrics
    let metrics_json = metrics.to_json();
    let avg_latency: f64 = metrics_json["http_avg_latency_ms"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    
    // Assert performance thresholds
    assert!(avg_latency < 100.0, "Average latency should be under 100ms, got {}", avg_latency);
    
    // Report metrics
    metrics.report();
}

#[actix_web::test]
async fn example_load_test_with_metrics() {
    let (srv, metrics) = spawn_test_server_with_metrics().await;
    let client = TestClient::new(srv.url("").to_string());
    
    // Simulate concurrent load
    let mut handles = vec![];
    
    for _ in 0..50 {
        let client_clone = client.clone();
        let metrics_clone = metrics.clone();
        
        let handle = tokio::spawn(async move {
            let start = Instant::now();
            let _ = client_clone.get("/health").await;
            metrics_clone.record_request(start.elapsed());
        });
        
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    for handle in handles {
        let _ = handle.await;
    }
    
    // Report final metrics
    println!("Load test completed:");
    metrics.report();
    
    // Verify all requests completed
    let metrics_json = metrics.to_json();
    assert_eq!(metrics_json["http_request_count"], 50);
}
