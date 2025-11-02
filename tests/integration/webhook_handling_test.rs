use actix_web::test;
use serde_json::json;
use tests::helpers::*;

/// Test Xendit paid webhook updates invoice status
#[actix_web::test]
async fn test_xendit_paid_webhook_updates_invoice() {
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());
    
    // Step 1: Create invoice via API
    let external_id = TestDataFactory::random_external_id();
    let create_payload = json!({
        "external_id": external_id,
        "amount": 100000,
        "currency": "IDR",
        "description": "Test invoice for webhook",
        "customer_email": "customer@example.com"
    });
    
    let mut response = client
        .post_json("/api/invoices", &create_payload)
        .await
        .expect("Failed to create invoice");
    
    assert_created(&response);
    
    let invoice_json = response.json::<serde_json::Value>().await.unwrap();
    let invoice_id = invoice_json["id"].as_str().unwrap();
    
    // Step 2: Simulate Xendit paid webhook
    let webhook_payload = XenditSandbox::simulate_paid_webhook(
        &external_id,
        "xnd_invoice_123",
        100000,
        "IDR"
    );
    
    let mut webhook_response = client
        .post_json("/api/webhooks/xendit", &webhook_payload)
        .await
        .expect("Failed to process webhook");
    
    assert_ok(&webhook_response);
    
    // Step 3: Verify invoice status updated to paid
    let mut get_response = client
        .get(&format!("/api/invoices/{}", invoice_id))
        .await
        .expect("Failed to get invoice");
    
    assert_ok(&get_response);
    
    let updated_invoice = get_response.json::<serde_json::Value>().await.unwrap();
    assert_json_field_eq(&updated_invoice, "status", &json!("paid"));
    assert_json_field_eq(&updated_invoice, "paid_amount", &json!(100000));
}

/// Test Xendit pending webhook maintains pending status
#[actix_web::test]
async fn test_xendit_pending_webhook_maintains_status() {
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());
    
    // Step 1: Create invoice
    let external_id = TestDataFactory::random_external_id();
    let create_payload = json!({
        "external_id": external_id,
        "amount": 50000,
        "currency": "IDR",
        "description": "Pending webhook test"
    });
    
    let mut response = client
        .post_json("/api/invoices", &create_payload)
        .await
        .expect("Failed to create invoice");
    
    assert_created(&response);
    
    let invoice_json = response.json::<serde_json::Value>().await.unwrap();
    let invoice_id = invoice_json["id"].as_str().unwrap();
    
    // Step 2: Simulate pending webhook
    let webhook_payload = XenditSandbox::simulate_pending_webhook(
        &external_id,
        "xnd_invoice_456"
    );
    
    let mut webhook_response = client
        .post_json("/api/webhooks/xendit", &webhook_payload)
        .await
        .expect("Failed to process webhook");
    
    assert_ok(&webhook_response);
    
    // Step 3: Verify invoice still pending
    let mut get_response = client
        .get(&format!("/api/invoices/{}", invoice_id))
        .await
        .expect("Failed to get invoice");
    
    assert_ok(&get_response);
    
    let updated_invoice = get_response.json::<serde_json::Value>().await.unwrap();
    assert_json_field_eq(&updated_invoice, "status", &json!("pending"));
}

/// Test Xendit expired webhook marks invoice as expired
#[actix_web::test]
async fn test_xendit_expired_webhook_expires_invoice() {
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());
    
    // Step 1: Create invoice
    let external_id = TestDataFactory::random_external_id();
    let create_payload = json!({
        "external_id": external_id,
        "amount": 75000,
        "currency": "IDR",
        "description": "Expiration test"
    });
    
    let mut response = client
        .post_json("/api/invoices", &create_payload)
        .await
        .expect("Failed to create invoice");
    
    assert_created(&response);
    
    let invoice_json = response.json::<serde_json::Value>().await.unwrap();
    let invoice_id = invoice_json["id"].as_str().unwrap();
    
    // Step 2: Simulate expired webhook
    let webhook_payload = XenditSandbox::simulate_expired_webhook(
        &external_id,
        "xnd_invoice_789"
    );
    
    let mut webhook_response = client
        .post_json("/api/webhooks/xendit", &webhook_payload)
        .await
        .expect("Failed to process webhook");
    
    assert_ok(&webhook_response);
    
    // Step 3: Verify invoice expired
    let mut get_response = client
        .get(&format!("/api/invoices/{}", invoice_id))
        .await
        .expect("Failed to get invoice");
    
    assert_ok(&get_response);
    
    let updated_invoice = get_response.json::<serde_json::Value>().await.unwrap();
    assert_json_field_eq(&updated_invoice, "status", &json!("expired"));
}

/// Test Midtrans settlement webhook updates transaction status
#[actix_web::test]
async fn test_midtrans_settlement_webhook_updates_transaction() {
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());
    
    // Step 1: Create invoice
    let external_id = TestDataFactory::random_external_id();
    let create_payload = json!({
        "external_id": external_id,
        "amount": 200000,
        "currency": "IDR",
        "description": "Midtrans settlement test"
    });
    
    let mut response = client
        .post_json("/api/invoices", &create_payload)
        .await
        .expect("Failed to create invoice");
    
    assert_created(&response);
    
    let invoice_json = response.json::<serde_json::Value>().await.unwrap();
    let order_id = invoice_json["external_id"].as_str().unwrap();
    
    // Step 2: Simulate Midtrans payment webhook
    let webhook_payload = MidtransSandbox::simulate_payment_webhook(
        order_id,
        "200000"
    );
    
    let mut webhook_response = client
        .post_json("/api/webhooks/midtrans", &webhook_payload)
        .await
        .expect("Failed to process webhook");
    
    assert_ok(&webhook_response);
    
    // Step 3: Verify transaction created with settlement status
    let mut transactions_response = client
        .get(&format!("/api/transactions?order_id={}", order_id))
        .await
        .expect("Failed to get transactions");
    
    assert_ok(&transactions_response);
    
    let transactions = transactions_response.json::<serde_json::Value>().await.unwrap();
    let transaction = &transactions["data"][0];
    
    assert_json_field_eq(transaction, "status", &json!("settlement"));
    assert_json_field_eq(transaction, "gross_amount", &json!("200000"));
    assert_json_field_eq(transaction, "fraud_status", &json!("accept"));
}

/// Test Midtrans pending webhook creates pending transaction
#[actix_web::test]
async fn test_midtrans_pending_webhook_creates_pending_transaction() {
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());
    
    // Step 1: Create invoice
    let external_id = TestDataFactory::random_external_id();
    let create_payload = json!({
        "external_id": external_id,
        "amount": 150000,
        "currency": "IDR",
        "description": "Midtrans pending test"
    });
    
    let mut response = client
        .post_json("/api/invoices", &create_payload)
        .await
        .expect("Failed to create invoice");
    
    assert_created(&response);
    
    let invoice_json = response.json::<serde_json::Value>().await.unwrap();
    let order_id = invoice_json["external_id"].as_str().unwrap();
    
    // Step 2: Simulate pending webhook
    let webhook_payload = MidtransSandbox::simulate_pending_webhook(
        order_id,
        "150000"
    );
    
    let mut webhook_response = client
        .post_json("/api/webhooks/midtrans", &webhook_payload)
        .await
        .expect("Failed to process webhook");
    
    assert_ok(&webhook_response);
    
    // Step 3: Verify pending transaction
    let mut transactions_response = client
        .get(&format!("/api/transactions?order_id={}", order_id))
        .await
        .expect("Failed to get transactions");
    
    assert_ok(&transactions_response);
    
    let transactions = transactions_response.json::<serde_json::Value>().await.unwrap();
    let transaction = &transactions["data"][0];
    
    assert_json_field_eq(transaction, "status", &json!("pending"));
    assert_json_field_eq(transaction, "payment_type", &json!("bank_transfer"));
}

/// Test Midtrans failure webhook marks transaction as denied
#[actix_web::test]
async fn test_midtrans_failure_webhook_marks_transaction_denied() {
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());
    
    // Step 1: Create invoice
    let external_id = TestDataFactory::random_external_id();
    let create_payload = json!({
        "external_id": external_id,
        "amount": 300000,
        "currency": "IDR",
        "description": "Midtrans failure test"
    });
    
    let mut response = client
        .post_json("/api/invoices", &create_payload)
        .await
        .expect("Failed to create invoice");
    
    assert_created(&response);
    
    let invoice_json = response.json::<serde_json::Value>().await.unwrap();
    let order_id = invoice_json["external_id"].as_str().unwrap();
    
    // Step 2: Simulate failure webhook
    let webhook_payload = MidtransSandbox::simulate_failure_webhook(
        order_id,
        "300000"
    );
    
    let mut webhook_response = client
        .post_json("/api/webhooks/midtrans", &webhook_payload)
        .await
        .expect("Failed to process webhook");
    
    assert_ok(&webhook_response);
    
    // Step 3: Verify denied transaction
    let mut transactions_response = client
        .get(&format!("/api/transactions?order_id={}", order_id))
        .await
        .expect("Failed to get transactions");
    
    assert_ok(&transactions_response);
    
    let transactions = transactions_response.json::<serde_json::Value>().await.unwrap();
    let transaction = &transactions["data"][0];
    
    assert_json_field_eq(transaction, "status", &json!("deny"));
    assert_json_field_eq(transaction, "fraud_status", &json!("deny"));
}

/// Test webhook signature validation rejects invalid signatures
#[actix_web::test]
async fn test_webhook_rejects_invalid_signature() {
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());
    
    // Create webhook payload with invalid signature
    let invalid_webhook = json!({
        "external_id": "INVALID-123",
        "status": "PAID",
        "signature": "invalid_signature_value"
    });
    
    let response_result = client
        .post_json("/api/webhooks/xendit", &invalid_webhook)
        .await;
    
    // Should fail or return unauthorized
    match response_result {
        Ok(mut response) => {
            let status = response.status();
            assert!(
                status.is_client_error() || status.is_server_error(),
                "Expected error response for invalid signature, got: {:?}",
                status
            );
        },
        Err(_) => {
            // Connection error is also acceptable (webhook rejected)
        }
    }
}

/// Test concurrent webhook processing doesn't cause race conditions
#[actix_web::test]
async fn test_concurrent_webhooks_no_race_conditions() {
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());
    
    // Step 1: Create invoice
    let external_id = TestDataFactory::random_external_id();
    let create_payload = json!({
        "external_id": external_id,
        "amount": 100000,
        "currency": "IDR",
        "description": "Concurrent webhook test"
    });
    
    let mut response = client
        .post_json("/api/invoices", &create_payload)
        .await
        .expect("Failed to create invoice");
    
    assert_created(&response);
    
    let invoice_json = response.json::<serde_json::Value>().await.unwrap();
    let invoice_id = invoice_json["id"].as_str().unwrap().to_string();
    
    // Step 2: Send multiple webhooks concurrently
    let webhook1 = XenditSandbox::simulate_pending_webhook(&external_id, "xnd_1");
    let webhook2 = XenditSandbox::simulate_pending_webhook(&external_id, "xnd_2");
    let webhook3 = XenditSandbox::simulate_paid_webhook(&external_id, "xnd_3", 100000, "IDR");
    
    let client1 = client.clone();
    let client2 = client.clone();
    let client3 = client.clone();
    
    let task1 = tokio::spawn(async move {
        client1.post_json("/api/webhooks/xendit", &webhook1).await
    });
    
    let task2 = tokio::spawn(async move {
        client2.post_json("/api/webhooks/xendit", &webhook2).await
    });
    
    let task3 = tokio::spawn(async move {
        client3.post_json("/api/webhooks/xendit", &webhook3).await
    });
    
    // Wait for all webhooks to complete
    let _ = tokio::try_join!(task1, task2, task3);
    
    // Step 3: Verify final state is consistent (paid)
    let mut get_response = client
        .get(&format!("/api/invoices/{}", invoice_id))
        .await
        .expect("Failed to get invoice");
    
    assert_ok(&get_response);
    
    let final_invoice = get_response.json::<serde_json::Value>().await.unwrap();
    assert_json_field_eq(&final_invoice, "status", &json!("paid"));
}
