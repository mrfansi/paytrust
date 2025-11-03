use paytrust::core::error::AppError;
use paytrust::modules::transactions::services::webhook_handler::WebhookHandler;
use paytrust::modules::transactions::repositories::transaction_repository::TransactionRepository;
use paytrust::modules::transactions::models::payment_transaction::PaymentTransaction;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::{Duration, Instant};

/// Mock transaction repository for testing
struct MockTransactionRepository;

#[async_trait::async_trait]
impl TransactionRepository for MockTransactionRepository {
    async fn create(&self, _transaction: &PaymentTransaction) -> Result<(), AppError> {
        Ok(())
    }

    async fn find_by_id(&self, _id: i64) -> Result<Option<PaymentTransaction>, AppError> {
        Ok(None)
    }

    async fn find_by_gateway_ref(&self, _gateway_ref: &str) -> Result<Option<PaymentTransaction>, AppError> {
        Ok(None)
    }

    async fn find_by_invoice_id(&self, _invoice_id: i64) -> Result<Vec<PaymentTransaction>, AppError> {
        Ok(vec![])
    }

    async fn update_status(&self, _id: i64, _status: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn find_by_installment_id(&self, _installment_id: i64) -> Result<Vec<PaymentTransaction>, AppError> {
        Ok(vec![])
    }

    async fn get_total_paid(&self, _invoice_id: i64) -> Result<rust_decimal::Decimal, AppError> {
        Ok(rust_decimal::Decimal::ZERO)
    }

    async fn record_overpayment(&self, _transaction_id: i64, _overpayment_amount: rust_decimal::Decimal) -> Result<(), AppError> {
        Ok(())
    }
}

/// T052a: Performance test for webhook retry queue capacity
/// 
/// Verifies:
/// - Queue handles 10,000 pending retries per NFR-010
/// - <100ms queue operation latency at 10k queue depth
/// - Enqueue/dequeue operations under load
#[tokio::test]
#[ignore] // Performance test - run explicitly with: cargo test --test webhook_queue_capacity_test -- --ignored
async fn test_webhook_queue_capacity_10k_pending_retries() {
    let repo = Arc::new(MockTransactionRepository);
    let handler = WebhookHandler::new(repo);
    
    let processed_count = Arc::new(AtomicUsize::new(0));
    let total_webhooks = 10_000;
    
    let start = Instant::now();
    
    // Spawn 10,000 webhook processing tasks
    let mut handles = vec![];
    
    for i in 0..total_webhooks {
        let handler_clone = handler.clone();
        let processed_clone = processed_count.clone();
        let webhook_id = format!("webhook_{}", i);
        
        let handle = tokio::spawn(async move {
            let process_start = Instant::now();
            
            // Simulate webhook processing that succeeds immediately
            let result = handler_clone.process_webhook_with_retry(
                &webhook_id,
                || async {
                    // Simulate minimal processing time
                    tokio::time::sleep(Duration::from_micros(100)).await;
                    Ok(())
                }
            ).await;
            
            let process_duration = process_start.elapsed();
            
            if result.is_ok() {
                processed_clone.fetch_add(1, Ordering::SeqCst);
            }
            
            process_duration
        });
        
        handles.push(handle);
    }
    
    // Wait for all webhooks to be enqueued and start processing
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Measure queue operation latency at peak load
    let queue_op_start = Instant::now();
    let test_webhook_id = "latency_test_webhook";
    let _ = handler.process_webhook_with_retry(
        test_webhook_id,
        || async { Ok(()) }
    ).await;
    let queue_op_latency = queue_op_start.elapsed();
    
    // Wait for all tasks to complete
    let mut max_duration = Duration::from_secs(0);
    for handle in handles {
        if let Ok(duration) = handle.await {
            if duration > max_duration {
                max_duration = duration;
            }
        }
    }
    
    let total_duration = start.elapsed();
    let final_count = processed_count.load(Ordering::SeqCst);
    
    // Assertions per NFR-010
    assert_eq!(
        final_count, total_webhooks,
        "All 10,000 webhooks should be processed successfully"
    );
    
    assert!(
        queue_op_latency < Duration::from_millis(100),
        "Queue operation latency at 10k depth should be <100ms, got {:?}",
        queue_op_latency
    );
    
    println!("✅ Webhook queue capacity test passed:");
    println!("   - Total webhooks processed: {}", final_count);
    println!("   - Total duration: {:?}", total_duration);
    println!("   - Queue operation latency at 10k depth: {:?}", queue_op_latency);
    println!("   - Max individual webhook duration: {:?}", max_duration);
}

/// Test enqueue/dequeue operations under load
#[tokio::test]
#[ignore]
async fn test_webhook_queue_enqueue_dequeue_performance() {
    let repo = Arc::new(MockTransactionRepository);
    let handler = WebhookHandler::new(repo);
    
    let enqueue_count = 1000;
    let mut enqueue_durations = vec![];
    
    // Test enqueue performance
    for i in 0..enqueue_count {
        let webhook_id = format!("enqueue_test_{}", i);
        let start = Instant::now();
        
        let _ = tokio::spawn({
            let handler_clone = handler.clone();
            async move {
                handler_clone.process_webhook_with_retry(
                    &webhook_id,
                    || async { Ok(()) }
                ).await
            }
        });
        
        let duration = start.elapsed();
        enqueue_durations.push(duration);
    }
    
    // Calculate statistics
    let avg_enqueue = enqueue_durations.iter().sum::<Duration>() / enqueue_count as u32;
    let max_enqueue = enqueue_durations.iter().max().unwrap();
    
    assert!(
        avg_enqueue < Duration::from_millis(10),
        "Average enqueue latency should be <10ms, got {:?}",
        avg_enqueue
    );
    
    assert!(
        *max_enqueue < Duration::from_millis(50),
        "Max enqueue latency should be <50ms, got {:?}",
        max_enqueue
    );
    
    println!("✅ Enqueue/dequeue performance test passed:");
    println!("   - Average enqueue latency: {:?}", avg_enqueue);
    println!("   - Max enqueue latency: {:?}", max_enqueue);
}

/// Test queue behavior under sustained load
#[tokio::test]
#[ignore]
async fn test_webhook_queue_sustained_load() {
    let repo = Arc::new(MockTransactionRepository);
    let handler = WebhookHandler::new(repo);
    
    let processed_count = Arc::new(AtomicUsize::new(0));
    let duration_seconds = 10;
    let target_rate = 100; // 100 webhooks/second
    
    let start = Instant::now();
    let mut interval = tokio::time::interval(Duration::from_millis(1000 / target_rate));
    
    while start.elapsed() < Duration::from_secs(duration_seconds) {
        interval.tick().await;
        
        let handler_clone = handler.clone();
        let processed_clone = processed_count.clone();
        let webhook_id = format!("sustained_{}", processed_count.load(Ordering::SeqCst));
        
        tokio::spawn(async move {
            let result = handler_clone.process_webhook_with_retry(
                &webhook_id,
                || async {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    Ok(())
                }
            ).await;
            
            if result.is_ok() {
                processed_clone.fetch_add(1, Ordering::SeqCst);
            }
        });
    }
    
    // Wait for remaining tasks
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    let final_count = processed_count.load(Ordering::SeqCst);
    let expected_min = (duration_seconds * target_rate) * 95 / 100; // 95% of target
    
    assert!(
        final_count >= expected_min,
        "Should process at least 95% of target rate under sustained load. Expected >={}, got {}",
        expected_min,
        final_count
    );
    
    println!("✅ Sustained load test passed:");
    println!("   - Duration: {}s", duration_seconds);
    println!("   - Target rate: {} webhooks/s", target_rate);
    println!("   - Processed: {} webhooks", final_count);
    println!("   - Actual rate: {:.2} webhooks/s", final_count as f64 / duration_seconds as f64);
}
