// Integration test for complete payment flow with REAL HTTP endpoints
//
// Tests end-to-end payment lifecycle using actual HTTP requests:
// 1. Create invoice via POST /api/invoices
// 2. Initiate payment via POST /api/payments/initiate  
// 3. Simulate webhook callback via POST /api/webhooks/{gateway}
// 4. Verify status transitions via GET /api/invoices/{id}
//
// Uses real test server, real database, real gateway sandbox APIs.
// Replaces mockito with XenditSandbox for Constitution Principle III compliance.

use actix_web::http::StatusCode;
use serde_json::{json, Value};

// Import test helpers from tests/helpers/
// Note: helpers module is available because tests/helpers/mod.rs exists
#[path = "../helpers/mod.rs"]
mod helpers;

use helpers::assertions::*;
use helpers::gateway_sandbox::XenditSandbox;
use helpers::test_client::TestClient;
use helpers::test_data::{TestDataFactory, TestFixtures};
use helpers::test_database::{create_test_pool, with_transaction};
use helpers::test_server::spawn_test_server;

#[actix_web::test]
async fn test_single_payment_flow() {
    // Step 1: Setup - Spawn real HTTP test server
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());
    
    // Step 2: Generate unique test data using TestDataFactory
    let external_id = TestDataFactory::random_external_id();
    let invoice_payload = TestDataFactory::create_invoice_payload_with(
        TestFixtures::XENDIT_TEST_GATEWAY_ID,
        TestFixtures::CURRENCY_IDR,
        TestFixtures::DEFAULT_AMOUNT_IDR,
    );
    
    // Override external_id with our unique one
    let mut payload = invoice_payload.as_object().unwrap().clone();
    payload.insert("external_id".to_string(), json!(external_id));
    let payload = Value::Object(payload);

    // Step 3: Create invoice via HTTP POST /api/invoices
    let mut create_response = client
        .post_json("/api/invoices", &payload)
        .await
        .expect("Failed to create invoice");

    assert_created(&create_response);
    
    let invoice: Value = create_response
        .json()
        .await
        .expect("Failed to parse invoice response");
    
    let invoice_id = invoice["id"].as_str().expect("Invoice ID missing");
    
    // Step 4: Verify invoice has pending status via GET /api/invoices/{id}
    let mut get_response = client
        .get_request(&format!("/api/invoices/{}", invoice_id))
        .await
        .expect("Failed to get invoice");
    
    assert_ok(&get_response);
    
    let invoice_data: Value = get_response
        .json()
        .await
        .expect("Failed to parse get response");
    
    assert_json_field_eq(
        &invoice_data,
        "status",
        &json!("pending"),
    );
    
    // Step 5: Initiate payment via POST /api/payments/initiate
    let payment_payload = json!({
        "invoice_id": invoice_id,
        "payment_method": "bank_transfer",
    });
    
    let mut initiate_response = client
        .post_json("/api/payments/initiate", &payment_payload)
        .await
        .expect("Failed to initiate payment");
    
    assert_ok(&initiate_response);
    
    let payment_data: Value = initiate_response
        .json()
        .await
        .expect("Failed to parse payment response");
    
    // Extract gateway payment details
    let gateway_ref = payment_data["gateway_transaction_ref"]
        .as_str()
        .expect("Gateway ref missing");
    
    // Step 6: Simulate successful webhook callback from Xendit
    let webhook_payload = json!({
        "external_id": external_id,
        "status": "PAID",
        "paid_amount": TestFixtures::DEFAULT_AMOUNT_IDR,
        "payment_method": "BANK_TRANSFER",
        "id": gateway_ref,
    });
    
    let mut webhook_response = client
        .post_json("/api/webhooks/xendit", &webhook_payload)
        .await
        .expect("Failed to process webhook");
    
    assert_ok(&webhook_response);
    
    // Step 7: Verify invoice status changed to paid via GET /api/invoices/{id}
    let mut final_response = client
        .get_request(&format!("/api/invoices/{}", invoice_id))
        .await
        .expect("Failed to get final invoice state");
    
    assert_ok(&final_response);
    
    let final_invoice: Value = final_response
        .json()
        .await
        .expect("Failed to parse final invoice");
    
    assert_json_field_eq(
        &final_invoice,
        "status",
        &json!("paid"),
    );
    
    // Step 8: Verify transaction via GET /api/invoices/{id}/transactions
    let mut transactions_response = client
        .get_request(&format!("/api/invoices/{}/transactions", invoice_id))
        .await
        .expect("Failed to get transactions");
    
    assert_ok(&transactions_response);
    
    let transactions: Value = transactions_response
        .json()
        .await
        .expect("Failed to parse transactions");
    
    let transactions_array = transactions.as_array()
        .expect("Transactions should be array");
    
    assert_eq!(
        transactions_array.len(),
        1,
        "Should have exactly one completed transaction"
    );
    
    assert_json_field_eq(
        &transactions_array[0],
        "status",
        &json!("completed"),
    );
}

#[actix_web::test]
async fn test_idempotent_webhook_processing() {
    // Step 1: Setup
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());
    
    // Step 2: Generate unique test data
    let external_id = TestDataFactory::random_external_id();
    let invoice_payload = TestDataFactory::create_invoice_payload_with(
        TestFixtures::XENDIT_TEST_GATEWAY_ID,
        TestFixtures::CURRENCY_IDR,
        TestFixtures::DEFAULT_AMOUNT_IDR,
    );
    
    let mut payload = invoice_payload.as_object().unwrap().clone();
    payload.insert("external_id".to_string(), json!(external_id));
    let payload = Value::Object(payload);

    // Step 3: Create invoice via HTTP POST
    let mut create_response = client
        .post_json("/api/invoices", &payload)
        .await
        .expect("Failed to create invoice");

    assert_created(&create_response);
    
    let invoice: Value = create_response
        .json()
        .await
        .expect("Failed to parse invoice");
    
    let invoice_id = invoice["id"].as_str().expect("Invoice ID missing");
    
    // Step 4: Process first webhook
    let gateway_ref = format!("xendit-{}", uuid::Uuid::new_v4());
    
    let webhook_payload = json!({
        "external_id": external_id,
        "status": "PAID",
        "paid_amount": TestFixtures::DEFAULT_AMOUNT_IDR,
        "payment_method": "BANK_TRANSFER",
        "id": gateway_ref,
    });
    
    let mut first_webhook_response = client
        .post_json("/api/webhooks/xendit", &webhook_payload)
        .await
        .expect("Failed to process first webhook");
    
    assert_ok(&first_webhook_response);
    
    // Step 5: Process duplicate webhook with same gateway_ref
    // Should be idempotent (return 200 OK without creating duplicate)
    let mut duplicate_webhook_response = client
        .post_json("/api/webhooks/xendit", &webhook_payload)
        .await
        .expect("Failed to process duplicate webhook");
    
    // Should return 200 OK (idempotent behavior)
    assert_ok(&duplicate_webhook_response);
    
    // Step 6: Verify only one transaction exists via GET /api/invoices/{id}/transactions
    let mut transactions_response = client
        .get_request(&format!("/api/invoices/{}/transactions", invoice_id))
        .await
        .expect("Failed to get transactions");
    
    assert_ok(&transactions_response);
    
    let transactions: Value = transactions_response
        .json()
        .await
        .expect("Failed to parse transactions");
    
    let transactions_array = transactions.as_array()
        .expect("Transactions should be array");
    
    assert_eq!(
        transactions_array.len(),
        1,
        "Should have exactly one transaction despite duplicate webhook"
    );
}

#[actix_web::test]
async fn test_partial_payment_flow() {
    // Step 1: Setup
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());
    
    // Step 2: Generate unique test data
    let external_id = TestDataFactory::random_external_id();
    let total_amount = TestFixtures::DEFAULT_AMOUNT_IDR; // 100,000 IDR
    
    let invoice_payload = TestDataFactory::create_invoice_payload_with(
        TestFixtures::XENDIT_TEST_GATEWAY_ID,
        TestFixtures::CURRENCY_IDR,
        total_amount,
    );
    
    let mut payload = invoice_payload.as_object().unwrap().clone();
    payload.insert("external_id".to_string(), json!(external_id));
    let payload = Value::Object(payload);

    // Step 3: Create invoice via HTTP POST
    let mut create_response = client
        .post_json("/api/invoices", &payload)
        .await
        .expect("Failed to create invoice");

    assert_created(&create_response);
    
    let invoice: Value = create_response
        .json()
        .await
        .expect("Failed to parse invoice");
    
    let invoice_id = invoice["id"].as_str().expect("Invoice ID missing");
    
    // Step 4: Process first partial payment webhook (30,000 IDR = 30% of total)
    let first_payment_amount = 30_000;
    let gateway_ref_1 = format!("xendit-{}", uuid::Uuid::new_v4());
    
    let webhook_payload_1 = json!({
        "external_id": external_id,
        "status": "PAID",
        "paid_amount": first_payment_amount,
        "payment_method": "BANK_TRANSFER",
        "id": gateway_ref_1,
    });
    
    let mut first_webhook_response = client
        .post_json("/api/webhooks/xendit", &webhook_payload_1)
        .await
        .expect("Failed to process first partial payment");
    
    assert_ok(&first_webhook_response);
    
    // Step 5: Verify invoice still pending (not fully paid)
    let mut invoice_response = client
        .get_request(&format!("/api/invoices/{}", invoice_id))
        .await
        .expect("Failed to get invoice");
    
    assert_ok(&invoice_response);
    
    let invoice_data: Value = invoice_response
        .json()
        .await
        .expect("Failed to parse invoice");
    
    assert_json_field_eq(
        &invoice_data,
        "status",
        &json!("pending"),
    );
    
    // Step 6: Verify transaction exists with partial amount
    let mut transactions_response = client
        .get_request(&format!("/api/invoices/{}/transactions", invoice_id))
        .await
        .expect("Failed to get transactions");
    
    assert_ok(&transactions_response);
    
    let transactions: Value = transactions_response
        .json()
        .await
        .expect("Failed to parse transactions");
    
    let transactions_array = transactions.as_array()
        .expect("Transactions should be array");
    
    assert_eq!(transactions_array.len(), 1, "Should have one transaction");
    
    // Step 7: Process second payment webhook to complete (70,000 IDR)
    let second_payment_amount = 70_000;
    let gateway_ref_2 = format!("xendit-{}", uuid::Uuid::new_v4());
    
    let webhook_payload_2 = json!({
        "external_id": external_id,
        "status": "PAID",
        "paid_amount": second_payment_amount,
        "payment_method": "BANK_TRANSFER",
        "id": gateway_ref_2,
    });
    
    let mut second_webhook_response = client
        .post_json("/api/webhooks/xendit", &webhook_payload_2)
        .await
        .expect("Failed to process second payment");
    
    assert_ok(&second_webhook_response);
    
    // Step 8: Verify invoice is now paid
    let mut final_invoice_response = client
        .get_request(&format!("/api/invoices/{}", invoice_id))
        .await
        .expect("Failed to get final invoice");
    
    assert_ok(&final_invoice_response);
    
    let final_invoice: Value = final_invoice_response
        .json()
        .await
        .expect("Failed to parse final invoice");
    
    assert_json_field_eq(
        &final_invoice,
        "status",
        &json!("paid"),
    );
    
    // Step 9: Verify both transactions exist
    let mut final_transactions_response = client
        .get_request(&format!("/api/invoices/{}/transactions", invoice_id))
        .await
        .expect("Failed to get final transactions");
    
    assert_ok(&final_transactions_response);
    
    let final_transactions: Value = final_transactions_response
        .json()
        .await
        .expect("Failed to parse final transactions");
    
    let final_transactions_array = final_transactions.as_array()
        .expect("Transactions should be array");
    
    assert_eq!(
        final_transactions_array.len(),
        2,
        "Should have two completed transactions"
    );
    
    // Verify total amount paid equals invoice total
    let total_paid: i64 = final_transactions_array
        .iter()
        .map(|t| t["amount_paid"].as_i64().unwrap_or(0))
        .sum();
    
    assert_eq!(
        total_paid, total_amount,
        "Total paid should equal invoice total"
    );
}

#[actix_web::test]
async fn test_concurrent_payment_prevention() {
    // Step 1: Setup
    let srv = spawn_test_server().await;
    let client = TestClient::new(srv.url("").to_string());
    
    // Step 2: Generate unique test data
    let external_id = TestDataFactory::random_external_id();
    let invoice_payload = TestDataFactory::create_invoice_payload_with(
        TestFixtures::XENDIT_TEST_GATEWAY_ID,
        TestFixtures::CURRENCY_IDR,
        TestFixtures::DEFAULT_AMOUNT_IDR,
    );
    
    let mut payload = invoice_payload.as_object().unwrap().clone();
    payload.insert("external_id".to_string(), json!(external_id));
    let payload = Value::Object(payload);

    // Step 3: Create invoice via HTTP POST
    let mut create_response = client
        .post_json("/api/invoices", &payload)
        .await
        .expect("Failed to create invoice");

    assert_created(&create_response);
    
    let invoice: Value = create_response
        .json()
        .await
        .expect("Failed to parse invoice");
    
    let invoice_id = invoice["id"].as_str().expect("Invoice ID missing");
    
    // Step 4: Initiate first payment via POST /api/payments/initiate
    let payment_payload = json!({
        "invoice_id": invoice_id,
        "payment_method": "bank_transfer",
    });
    
    let mut first_initiate_response = client
        .post_json("/api/payments/initiate", &payment_payload)
        .await
        .expect("Failed to initiate first payment");
    
    assert_ok(&first_initiate_response);
    
    // Step 5: Verify invoice is now in "processing" status
    let mut invoice_response = client
        .get_request(&format!("/api/invoices/{}", invoice_id))
        .await
        .expect("Failed to get invoice");
    
    assert_ok(&invoice_response);
    
    let invoice_data: Value = invoice_response
        .json()
        .await
        .expect("Failed to parse invoice");
    
    assert_json_field_eq(
        &invoice_data,
        "status",
        &json!("processing"),
    );
    
    // Step 6: Attempt concurrent payment initiation (should be rejected)
    let mut concurrent_initiate_response = client
        .post_json("/api/payments/initiate", &payment_payload)
        .await
        .expect("Failed to attempt concurrent payment");
    
    // Should return 409 Conflict or 400 Bad Request
    let status = concurrent_initiate_response.status();
    assert!(
        status == StatusCode::CONFLICT || status == StatusCode::BAD_REQUEST,
        "Concurrent payment attempt should be rejected, got: {}",
        status
    );
    
    // Step 7: Verify invoice still in processing (not corrupted)
    let mut final_invoice_response = client
        .get_request(&format!("/api/invoices/{}", invoice_id))
        .await
        .expect("Failed to get final invoice");
    
    assert_ok(&final_invoice_response);
    
    let final_invoice: Value = final_invoice_response
        .json()
        .await
        .expect("Failed to parse final invoice");
    
    assert_json_field_eq(
        &final_invoice,
        "status",
        &json!("processing"),
    );
}
