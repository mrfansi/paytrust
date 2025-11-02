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
    // ========================================
    // Payment Gateway IDs (match seeded data)
    // ========================================
    
    /// Xendit gateway for IDR currency (primary test gateway)
    pub const XENDIT_TEST_GATEWAY_ID: &'static str = "test-gateway-001";
    
    /// Midtrans gateway for IDR currency (secondary test gateway)
    pub const MIDTRANS_TEST_GATEWAY_ID: &'static str = "test-gateway-002";
    
    /// Xendit gateway for MYR currency (multi-currency testing)
    pub const TEST_GATEWAY_XENDIT_MYR: &'static str = "gateway-xendit-myr";
    
    /// Midtrans gateway for IDR (alternative identifier)
    pub const TEST_GATEWAY_MIDTRANS_IDR: &'static str = "gateway-midtrans-idr";

    // ========================================
    // API Keys (for authentication testing)
    // ========================================
    
    /// Test API key for authentication (unhashed)
    pub const TEST_API_KEY: &'static str = "test_api_key_001";
    
    /// Secondary test API key
    pub const TEST_API_KEY_SECONDARY: &'static str = "test_api_key_002";
    
    /// Invalid API key for negative testing
    pub const TEST_API_KEY_INVALID: &'static str = "invalid_key_12345";

    // ========================================
    // Test Credit Cards - Xendit Sandbox
    // ========================================
    // Source: https://developers.xendit.co/api-reference/#test-cards
    
    /// Xendit test card - successful payment
    pub const XENDIT_TEST_CARD_SUCCESS: &'static str = "4000000000000002";
    
    /// Xendit test card - payment failure
    pub const XENDIT_TEST_CARD_FAILURE: &'static str = "4000000000000010";
    
    /// Xendit test card - insufficient funds
    pub const XENDIT_TEST_CARD_INSUFFICIENT_FUNDS: &'static str = "4000000000000119";
    
    /// Xendit test card - expired card
    pub const XENDIT_TEST_CARD_EXPIRED: &'static str = "4000000000000069";

    // ========================================
    // Test Credit Cards - Midtrans Sandbox
    // ========================================
    // Source: https://docs.midtrans.com/docs/testing-payment-flow
    
    /// Midtrans test card - successful payment
    pub const MIDTRANS_TEST_CARD_SUCCESS: &'static str = "4811111111111114";
    
    /// Midtrans test card - 3D Secure challenge flow
    pub const MIDTRANS_TEST_CARD_3DS: &'static str = "4911111111111113";
    
    /// Midtrans test card - payment failure
    pub const MIDTRANS_TEST_CARD_FAILURE: &'static str = "4011111111111112";
    
    /// Midtrans test card - fraud detection
    pub const MIDTRANS_TEST_CARD_FRAUD: &'static str = "4911111111111113";

    // ========================================
    // Test Bank Accounts (for bank transfer)
    // ========================================
    
    /// Test BCA virtual account number
    pub const TEST_VA_BCA: &'static str = "12345678901";
    
    /// Test BNI virtual account number
    pub const TEST_VA_BNI: &'static str = "98765432109";
    
    /// Test Mandiri virtual account number
    pub const TEST_VA_MANDIRI: &'static str = "11223344556";

    // ========================================
    // Test Amounts (in minor units)
    // ========================================
    
    /// Default test amount for IDR (100,000 IDR = 1,000.00 IDR)
    pub const DEFAULT_AMOUNT_IDR: i64 = 100_000;
    
    /// Default test amount for MYR (5,000 MYR cents = 50.00 MYR)
    pub const DEFAULT_AMOUNT_MYR: i64 = 50_00;
    
    /// Minimum invoice amount for IDR (10,000 IDR)
    pub const MIN_AMOUNT_IDR: i64 = 10_000;
    
    /// Maximum test amount for sandbox testing (10,000,000 IDR)
    pub const MAX_AMOUNT_IDR: i64 = 10_000_000;

    // ========================================
    // Test Currencies
    // ========================================
    
    /// Indonesian Rupiah
    pub const CURRENCY_IDR: &'static str = "IDR";
    
    /// Malaysian Ringgit
    pub const CURRENCY_MYR: &'static str = "MYR";
    
    /// Philippine Peso
    pub const CURRENCY_PHP: &'static str = "PHP";

    // ========================================
    // Test Tax Rates (as percentages)
    // ========================================
    
    /// Indonesian VAT (PPN) rate - 11%
    pub const TAX_RATE_IDR_VAT: f64 = 11.0;
    
    /// Service tax rate - 2%
    pub const TAX_RATE_SERVICE: f64 = 2.0;
    
    /// Luxury goods tax rate - 20%
    pub const TAX_RATE_LUXURY: f64 = 20.0;

    // ========================================
    // Helper Methods (Environment Access)
    // ========================================

    /// Get Xendit test API key from environment
    ///
    /// # Returns
    /// API key from XENDIT_TEST_API_KEY or default test key
    pub fn xendit_test_api_key() -> String {
        std::env::var("XENDIT_TEST_API_KEY")
            .unwrap_or_else(|_| "xnd_development_test_key".to_string())
    }

    /// Get Midtrans sandbox server key from environment
    ///
    /// # Returns
    /// Server key from MIDTRANS_SERVER_KEY or default test key
    pub fn midtrans_test_server_key() -> String {
        std::env::var("MIDTRANS_SERVER_KEY")
            .unwrap_or_else(|_| "SB-Mid-server-test_key".to_string())
    }

    /// Get test database URL from environment
    ///
    /// # Returns
    /// Database URL from TEST_DATABASE_URL or default
    pub fn test_database_url() -> String {
        std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
            "mysql://root:password@localhost:3306/paytrust_test".to_string()
        })
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
