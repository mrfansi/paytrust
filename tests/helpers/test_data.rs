// Test Data Factory
//
// Generates unique test data for integration tests.
// Uses UUIDs to ensure test isolation.

use serde_json::{json, Value};
use uuid::Uuid;

/// Test data factory for generating unique test data
pub struct TestDataFactory;

impl TestDataFactory {
    /// Generate random external ID with TEST prefix
    ///
    /// # Returns
    /// Unique external ID in format "TEST-{uuid}"
    ///
    /// # Example
    /// ```
    /// use paytrust::test_helpers::TestDataFactory;
    /// let id = TestDataFactory::random_external_id();
    /// assert!(id.starts_with("TEST-"));
    /// ```
    pub fn random_external_id() -> String {
        format!("TEST-{}", Uuid::new_v4())
    }

    /// Create valid invoice payload for testing
    ///
    /// # Returns
    /// JSON payload with:
    /// - Random external_id
    /// - Default test gateway
    /// - Single line item
    /// - IDR currency
    ///
    /// # Example
    /// ```no_run
    /// use paytrust::test_helpers::TestDataFactory;
    /// let payload = TestDataFactory::create_invoice_payload();
    /// // Can be sent to POST /api/invoices
    /// ```
    pub fn create_invoice_payload() -> Value {
        json!({
            "external_id": Self::random_external_id(),
            "gateway_id": "test-gateway-001",
            "currency": "IDR",
            "line_items": [
                {
                    "name": "Test Product",
                    "quantity": 1,
                    "unit_price": 100000,
                    "tax_rate": 11.0
                }
            ]
        })
    }

    /// Create invoice payload with custom values
    ///
    /// # Parameters
    /// - `gateway_id`: Gateway to use
    /// - `currency`: Currency code
    /// - `amount`: Total amount in minor units
    ///
    /// # Returns
    /// JSON invoice payload
    pub fn create_invoice_payload_with(
        gateway_id: &str,
        currency: &str,
        amount: i64,
    ) -> Value {
        json!({
            "external_id": Self::random_external_id(),
            "gateway_id": gateway_id,
            "currency": currency,
            "line_items": [
                {
                    "name": "Test Product",
                    "quantity": 1,
                    "unit_price": amount,
                    "tax_rate": 0.0
                }
            ]
        })
    }

    /// Create installment schedule payload
    ///
    /// # Parameters
    /// - `invoice_id`: Invoice to create installments for
    /// - `num_installments`: Number of installments (2-12)
    ///
    /// # Returns
    /// JSON installment schedule payload
    pub fn create_installment_payload(invoice_id: &str, num_installments: u8) -> Value {
        json!({
            "invoice_id": invoice_id,
            "num_installments": num_installments,
            "frequency": "monthly"
        })
    }

    /// Create payment transaction payload
    ///
    /// # Parameters
    /// - `invoice_id`: Invoice to pay
    /// - `amount`: Amount to pay in minor units
    ///
    /// # Returns
    /// JSON payment payload
    pub fn create_payment_payload(invoice_id: &str, amount: i64) -> Value {
        json!({
            "invoice_id": invoice_id,
            "amount": amount,
            "payment_method": "bank_transfer",
            "external_reference": Self::random_external_id()
        })
    }

    /// Create gateway configuration
    ///
    /// # Parameters
    /// - `currency`: Currency code (IDR, MYR, etc.)
    ///
    /// # Returns
    /// Gateway configuration JSON
    pub fn create_gateway_config(currency: &str) -> Value {
        json!({
            "name": format!("Test Gateway {}", currency),
            "gateway_type": "xendit",
            "currency": currency,
            "is_active": true
        })
    }
}

/// Test fixture constants
///
/// Pre-defined test data for common scenarios.
/// These IDs should match seeded test data in setup_test_db.sh
pub struct TestFixtures;

impl TestFixtures {
    // Gateway IDs
    pub const XENDIT_TEST_GATEWAY_ID: &'static str = "test-gateway-001";
    pub const MIDTRANS_TEST_GATEWAY_ID: &'static str = "test-gateway-002";

    // API Keys
    pub const TEST_API_KEY: &'static str = "test_api_key_001";

    // Test Cards (Xendit/Midtrans sandbox)
    pub const XENDIT_TEST_CARD_SUCCESS: &'static str = "4000000000000002";
    pub const XENDIT_TEST_CARD_FAILURE: &'static str = "4000000000000010";

    pub const MIDTRANS_TEST_CARD_SUCCESS: &'static str = "4811111111111114";
    pub const MIDTRANS_TEST_CARD_FAILURE: &'static str = "4911111111111113";

    // Test amounts
    pub const DEFAULT_AMOUNT_IDR: i64 = 100_000; // 100,000 IDR
    pub const DEFAULT_AMOUNT_MYR: i64 = 50_00; // 50.00 MYR

    /// Get test gateway environment variable
    pub fn xendit_test_api_key() -> String {
        std::env::var("XENDIT_TEST_API_KEY")
            .unwrap_or_else(|_| "xnd_development_test_key".to_string())
    }

    /// Get Midtrans sandbox key
    pub fn midtrans_test_server_key() -> String {
        std::env::var("MIDTRANS_SERVER_KEY")
            .unwrap_or_else(|_| "SB-Mid-server-test_key".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_external_id_format() {
        let id = TestDataFactory::random_external_id();
        assert!(id.starts_with("TEST-"));
        assert!(id.len() > 10); // TEST- + UUID
    }

    #[test]
    fn test_random_external_id_uniqueness() {
        let id1 = TestDataFactory::random_external_id();
        let id2 = TestDataFactory::random_external_id();
        assert_ne!(id1, id2, "IDs should be unique");
    }

    #[test]
    fn test_create_invoice_payload_structure() {
        let payload = TestDataFactory::create_invoice_payload();

        assert!(payload["external_id"].as_str().unwrap().starts_with("TEST-"));
        assert_eq!(payload["gateway_id"], "test-gateway-001");
        assert_eq!(payload["currency"], "IDR");
        assert!(payload["line_items"].is_array());
        assert_eq!(payload["line_items"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_create_invoice_payload_with_custom_values() {
        let payload =
            TestDataFactory::create_invoice_payload_with("custom-gateway", "MYR", 50000);

        assert_eq!(payload["gateway_id"], "custom-gateway");
        assert_eq!(payload["currency"], "MYR");
        assert_eq!(
            payload["line_items"][0]["unit_price"].as_i64().unwrap(),
            50000
        );
    }

    #[test]
    fn test_test_fixtures_constants() {
        assert_eq!(TestFixtures::XENDIT_TEST_GATEWAY_ID, "test-gateway-001");
        assert_eq!(TestFixtures::DEFAULT_AMOUNT_IDR, 100_000);
    }
}
