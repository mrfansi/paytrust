// Test Helper Modules for Real Endpoint Testing
//
// This module provides test infrastructure for integration and contract tests.
// All helpers use real HTTP connections and real database connections per
// Constitution Principle III (no mocks in integration tests).
//
// Usage:
//   use paytrust::test_helpers::*;
//
// Example:
//   #[actix_web::test]
//   async fn test_invoice_creation() {
//       let srv = spawn_test_server().await;
//       let pool = create_test_pool().await;
//
//       let payload = TestDataFactory::create_invoice_payload();
//       let response = srv.post("/api/invoices")
//           .send_json(&payload)
//           .await
//           .unwrap();
//
//       assert_success(&response);
//   }

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
