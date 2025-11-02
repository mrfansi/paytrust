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
// - `seed_isolated_gateway(id, name, type)` - Create gateway for single test
//
// ## Test Data Isolation for Parallel Execution (Phase 5 - T044)
//
// ### Problem: Data Conflicts in Parallel Tests
//
// When running tests in parallel, shared test data causes conflicts:
// ```rust
// // BAD: Hardcoded IDs conflict when tests run in parallel
// let invoice_id = "TEST-INV-001";  // Multiple tests use same ID!
// let gateway_id = "test-gateway";  // Collision!
// ```
//
// ### Solution 1: UUID-Based Unique Identifiers
//
// Generate unique IDs for each test run:
// ```rust
// use uuid::Uuid;
//
// // Generate unique external ID
// fn generate_test_id(prefix: &str) -> String {
//     format!("{}_{}", prefix, Uuid::new_v4())
// }
//
// #[actix_web::test]
// async fn test_invoice_creation() {
//     let external_id = generate_test_id("INV");  // "INV_550e8400-e29b-41d4-a716-446655440000"
//     let tx_id = generate_test_id("tx");          // "tx_7c9e6679-7425-40de-944b-e07fc1f90ae7"
//     
//     // Each test run has unique IDs - no conflicts!
// }
// ```
//
// ### Solution 2: Transaction-Based Isolation
//
// Use database transactions that auto-rollback:
// ```rust
// use tests::helpers::test_database::with_transaction;
//
// #[tokio::test]
// async fn test_with_isolation() {
//     with_transaction(|mut tx| async move {
//         // Insert test data - visible only to this transaction
//         sqlx::query("INSERT INTO tax_rates (id, rate) VALUES (?, ?)")
//             .bind("test-rate-001")
//             .bind(0.10)
//             .execute(&mut *tx)
//             .await
//             .unwrap();
//         
//         // Query and test
//         let rate: f64 = sqlx::query_scalar("SELECT rate FROM tax_rates WHERE id = ?")
//             .bind("test-rate-001")
//             .fetch_one(&mut *tx)
//             .await
//             .unwrap();
//         
//         assert_eq!(rate, 0.10);
//         
//         // Transaction rolls back automatically - no cleanup needed!
//         // Other tests won't see this data.
//     }).await;
// }
// ```
//
// ### Solution 3: Per-Test Gateway Seeding
//
// Seed isolated gateway for each test:
// ```rust
// #[actix_web::test]
// async fn test_gateway_specific() {
//     // Each test gets its own gateway
//     let gateway_id = format!("xendit-{}", Uuid::new_v4());
//     seed_isolated_gateway(&gateway_id, "Test Xendit", "xendit").await;
//     
//     // Use gateway in test...
//     // Other tests won't conflict with this gateway
// }
// ```
//
// ### Benefits of Isolation:
//
// ✅ **No Data Conflicts**: Each test has unique data  
// ✅ **True Parallel Execution**: Tests run simultaneously without interference  
// ✅ **No Flaky Tests**: Deterministic behavior every time  
// ✅ **No Manual Cleanup**: Transactions and UUIDs eliminate cleanup code  
// ✅ **Faster Test Suite**: Parallel execution reduces total runtime
//
// ### Validation:
//
// Run parallel test validation script:
// ```bash
// ./scripts/test_parallel.sh 10  # Run 10 iterations in parallel
// ```
//
// Expected: 100% pass rate with no data conflicts

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
