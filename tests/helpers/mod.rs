// Test Helper Modules for Real Endpoint Testing
//
// This module provides test infrastructure for integration and contract tests.
// All helpers use real HTTP connections and real database connections per
// Constitution Principle III (no mocks in integration tests).
//
// ## Refactoring Pattern: From Direct DB to Real HTTP Endpoints
//
// ### BEFORE (Old Pattern - Direct Database Manipulation):
// ```rust
// #[tokio::test]
// #[ignore = "Requires test database"]
// async fn test_payment_flow() {
//     let pool = MySqlPool::connect(&database_url).await.unwrap();
//     let invoice_id = uuid::Uuid::new_v4().to_string();
//     
//     // Direct DB insert
//     sqlx::query("INSERT INTO invoices (...) VALUES (...)")
//         .bind(&invoice_id)
//         .execute(&pool)
//         .await
//         .unwrap();
//     
//     // Direct DB query
//     let status: String = sqlx::query_scalar("SELECT status FROM invoices WHERE id = ?")
//         .bind(&invoice_id)
//         .fetch_one(&pool)
//         .await
//         .unwrap();
//     
//     assert_eq!(status, "pending");
// }
// ```
//
// ### AFTER (New Pattern - Real HTTP Endpoints):
// ```rust
// #[actix_web::test]
// async fn test_payment_flow() {
//     // 1. Spawn real HTTP test server
//     let srv = spawn_test_server().await;
//     let client = TestClient::new(srv.url("").to_string());
//     
//     // 2. Generate unique test data
//     let external_id = TestDataFactory::random_external_id();
//     let payload = TestDataFactory::create_invoice_payload_with(
//         TestFixtures::XENDIT_TEST_GATEWAY_ID,
//         TestFixtures::CURRENCY_IDR,
//         TestFixtures::DEFAULT_AMOUNT_IDR,
//     );
//     
//     // 3. Make HTTP request to real endpoint
//     let mut response = client
//         .post_json("/api/invoices", &payload)
//         .await
//         .expect("Failed to create invoice");
//     
//     // 4. Assert HTTP response
//     assert_created(&response);
//     
//     // 5. Parse JSON response
//     let invoice: Value = response.json().await.expect("Failed to parse");
//     let invoice_id = invoice["id"].as_str().expect("Invoice ID missing");
//     
//     // 6. Verify via GET endpoint
//     let mut get_response = client
//         .get_request(&format!("/api/invoices/{}", invoice_id))
//         .await
//         .expect("Failed to get invoice");
//     
//     assert_ok(&get_response);
//     
//     let invoice_data: Value = get_response.json().await.expect("Failed to parse");
//     assert_json_field_eq(&invoice_data, "status", &json!("pending"));
// }
// ```
//
// ## Key Improvements:
// 
// 1. **Real HTTP Testing**: Tests actual API endpoints, not just database logic
// 2. **Unique Test Data**: UUID-based IDs prevent conflicts in parallel execution
// 3. **No Mocks**: Uses real gateway sandbox APIs (XenditSandbox, MidtransSandbox)
// 4. **Proper Status Codes**: Tests verify 201 Created, 200 OK, 400 Bad Request, etc.
// 5. **JSON Validation**: Tests verify actual response structure matches OpenAPI spec
// 6. **No Manual Cleanup**: Test server and transactions auto-cleanup on drop
//
// ## Quick Reference:
//
// ### Test Server:
// - `spawn_test_server()` - Start HTTP server on random port
// - `spawn_test_server_with_config(fn)` - Custom server configuration
//
// ### HTTP Client:
// - `TestClient::new(base_url)` - Create client for making requests
// - `client.get_request(path)` - GET request
// - `client.post_json(path, body)` - POST with JSON
// - `client.put_json(path, body)` - PUT with JSON
// - `client.delete_request(path)` - DELETE request
//
// ### Test Data:
// - `TestDataFactory::random_external_id()` - Unique ID like "TEST-{uuid}"
// - `TestDataFactory::create_invoice_payload()` - Valid invoice JSON
// - `TestFixtures::XENDIT_TEST_GATEWAY_ID` - Pre-seeded gateway ID
// - `TestFixtures::XENDIT_TEST_CARD_SUCCESS` - Test card for successful payment
//
// ### Assertions:
// - `assert_created(&response)` - Assert 201 Created
// - `assert_ok(&response)` - Assert 200 OK
// - `assert_bad_request(&response)` - Assert 400 Bad Request
// - `assert_json_field_eq(&json, "field", &expected)` - Assert JSON field value
//
// ### Gateway Sandbox:
// - `XenditSandbox::new()` - Create Xendit sandbox client
// - `xendit.create_invoice(id, amount, currency)` - Real API call to Xendit
// - `MidtransSandbox::new()` - Create Midtrans sandbox client
// - `midtrans.charge(order_id, amount)` - Real API call to Midtrans
//
// ### Database:
// - `create_test_pool()` - Get connection pool to test database
// - `with_transaction(|tx| {...})` - Run test in auto-rollback transaction

pub mod assertions;
pub mod gateway_sandbox;
pub mod test_client;
pub mod test_data;
pub mod test_database;
pub mod test_server;

// Re-export commonly used types and functions
pub use assertions::*;
pub use gateway_sandbox::*;
pub use test_client::*;
pub use test_data::*;
pub use test_database::*;
pub use test_server::*;
