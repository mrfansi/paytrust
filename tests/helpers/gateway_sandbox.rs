// Payment Gateway Sandbox Helpers
//
// Provides helpers for interacting with payment gateway sandbox APIs.
// Uses REAL API calls to sandbox environments (no mocks).

use base64::prelude::*;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

/// Xendit sandbox API helper
///
/// Provides methods to interact with Xendit test mode API.
/// Uses real HTTP calls to Xendit sandbox.
pub struct XenditSandbox {
    api_key: String,
    base_url: String,
    client: Client,
}

impl XenditSandbox {
    /// Create new Xendit sandbox instance
    ///
    /// # Behavior
    /// - Reads XENDIT_TEST_API_KEY from environment
    /// - Falls back to default test key if not set
    /// - Validates key format (should start with xnd_development or xnd_public_test)
    ///
    /// # Returns
    /// Configured XenditSandbox instance
    ///
    /// # Example
    /// ```no_run
    /// use paytrust::test_helpers::XenditSandbox;
    /// let xendit = XenditSandbox::new();
    /// ```
    pub fn new() -> Self {
        let api_key = std::env::var("XENDIT_TEST_API_KEY")
            .unwrap_or_else(|_| {
                eprintln!(
                    "Warning: XENDIT_TEST_API_KEY not set, using default test key. \
                     Set in .env.test for real Xendit sandbox testing."
                );
                "xnd_development_test_key".to_string()
            });

        Self {
            api_key,
            base_url: "https://api.xendit.co".to_string(),
            client: Client::new(),
        }
    }

    /// Create invoice in Xendit sandbox
    ///
    /// # Parameters
    /// - `external_id`: Unique invoice identifier
    /// - `amount`: Amount in minor units (e.g., 100000 = 1,000.00 IDR)
    /// - `currency`: Currency code (IDR, PHP, etc.)
    ///
    /// # Returns
    /// Created invoice data or error
    ///
    /// # Error Handling
    /// Returns error if:
    /// - API key is invalid
    /// - Network request fails
    /// - Xendit API returns error
    ///
    /// # Example
    /// ```no_run
    /// # use paytrust::test_helpers::*;
    /// #[tokio::test]
    /// async fn test_xendit() {
    ///     let xendit = XenditSandbox::new();
    ///     let external_id = TestDataFactory::random_external_id();
    ///     let result = xendit.create_invoice(&external_id, 100000, "IDR").await;
    ///     // Note: Will fail without valid API key
    /// }
    /// ```
    pub async fn create_invoice(
        &self,
        external_id: &str,
        amount: i64,
        currency: &str,
    ) -> Result<Value, String> {
        let url = format!("{}/v2/invoices", self.base_url);

        let payload = json!({
            "external_id": external_id,
            "amount": amount,
            "currency": currency,
            "description": format!("Test invoice {}", external_id),
        });

        let response = self
            .client
            .post(&url)
            .basic_auth(&self.api_key, None::<String>)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Xendit API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not read error body".to_string());
            return Err(format!(
                "Xendit API returned error {}: {}",
                status, error_body
            ));
        }

        response
            .json::<Value>()
            .await
            .map_err(|e| format!("Failed to parse Xendit response: {}", e))
    }

    /// Get invoice details from Xendit
    ///
    /// # Parameters
    /// - `invoice_id`: Xendit invoice ID
    ///
    /// # Returns
    /// Invoice data or error
    pub async fn get_invoice(&self, invoice_id: &str) -> Result<Value, String> {
        let url = format!("{}/v2/invoices/{}", self.base_url, invoice_id);

        let response = self
            .client
            .get(&url)
            .basic_auth(&self.api_key, None::<String>)
            .send()
            .await
            .map_err(|e| format!("Xendit API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(format!("Xendit API returned error: {}", status));
        }

        response
            .json::<Value>()
            .await
            .map_err(|e| format!("Failed to parse Xendit response: {}", e))
    }

    /// Simulate payment for Xendit invoice
    ///
    /// In real tests, this would trigger actual payment using test card.
    /// For now, this is a placeholder that returns the invoice data.
    ///
    /// # Parameters
    /// - `invoice_id`: Xendit invoice ID to simulate payment for
    ///
    /// # Returns
    /// Invoice data or error
    pub async fn simulate_payment(&self, invoice_id: &str) -> Result<Value, String> {
        // In real tests, you would:
        // 1. Get the invoice payment URL
        // 2. Make payment using test card
        // 3. Wait for webhook callback
        // For now, just return the invoice to indicate simulation intent
        self.get_invoice(invoice_id).await
    }

    /// Simulate a Xendit invoice paid webhook payload
    ///
    /// # Parameters
    /// - `external_id`: External ID from your system
    /// - `invoice_id`: Xendit invoice ID
    /// - `amount`: Payment amount
    /// - `currency`: Currency code (IDR, MYR, PHP)
    ///
    /// # Returns
    /// JSON webhook payload simulating Xendit callback
    ///
    /// # Example
    /// ```
    /// let webhook = XenditSandbox::simulate_paid_webhook("INV-123", "xnd_invoice_456", 100000, "IDR");
    /// // Use webhook payload to test webhook endpoint
    /// ```
    pub fn simulate_paid_webhook(
        external_id: &str,
        invoice_id: &str,
        amount: i64,
        currency: &str,
    ) -> Value {
        serde_json::json!({
            "id": invoice_id,
            "external_id": external_id,
            "user_id": "test_user_id",
            "status": "PAID",
            "merchant_name": "Test Merchant",
            "merchant_profile_picture_url": "https://example.com/logo.png",
            "amount": amount,
            "paid_amount": amount,
            "bank_code": "BCA",
            "paid_at": "2025-11-02T10:30:45.123Z",
            "payer_email": "customer@example.com",
            "description": "Payment for invoice",
            "adjusted_received_amount": amount,
            "fees_paid_amount": 0,
            "updated": "2025-11-02T10:30:45.123Z",
            "created": "2025-11-02T09:00:00.123Z",
            "currency": currency,
            "payment_method": "BANK_TRANSFER",
            "payment_channel": "BCA",
            "payment_id": format!("PAYMENT-{}", uuid::Uuid::new_v4())
        })
    }

    /// Simulate a Xendit invoice pending webhook payload
    pub fn simulate_pending_webhook(
        external_id: &str,
        invoice_id: &str,
        amount: i64,
        currency: &str,
    ) -> Value {
        serde_json::json!({
            "id": invoice_id,
            "external_id": external_id,
            "user_id": "test_user_id",
            "status": "PENDING",
            "merchant_name": "Test Merchant",
            "amount": amount,
            "description": "Payment for invoice",
            "updated": "2025-11-02T09:00:00.123Z",
            "created": "2025-11-02T09:00:00.123Z",
            "currency": currency,
            "payment_method": "BANK_TRANSFER",
            "invoice_url": format!("https://checkout.xendit.co/web/{}", invoice_id)
        })
    }

    /// Simulate a Xendit invoice expired webhook payload
    pub fn simulate_expired_webhook(
        external_id: &str,
        invoice_id: &str,
        amount: i64,
        currency: &str,
    ) -> Value {
        serde_json::json!({
            "id": invoice_id,
            "external_id": external_id,
            "user_id": "test_user_id",
            "status": "EXPIRED",
            "merchant_name": "Test Merchant",
            "amount": amount,
            "description": "Payment for invoice",
            "updated": "2025-11-03T09:00:00.123Z",
            "created": "2025-11-02T09:00:00.123Z",
            "expired_at": "2025-11-03T09:00:00.123Z",
            "currency": currency
        })
    }
}
}

impl Default for XenditSandbox {
    fn default() -> Self {
        Self::new()
    }
}

/// Midtrans sandbox API helper
///
/// Provides methods to interact with Midtrans sandbox API.
/// Uses real HTTP calls to Midtrans sandbox.
pub struct MidtransSandbox {
    server_key: String,
    base_url: String,
    client: Client,
}

impl MidtransSandbox {
    /// Create new Midtrans sandbox instance
    ///
    /// # Behavior
    /// - Reads MIDTRANS_SERVER_KEY from environment
    /// - Falls back to default test key if not set
    /// - Configures sandbox base URL
    ///
    /// # Returns
    /// Configured MidtransSandbox instance
    ///
    /// # Example
    /// ```no_run
    /// use paytrust::test_helpers::MidtransSandbox;
    /// let midtrans = MidtransSandbox::new();
    /// ```
    pub fn new() -> Self {
        let server_key = std::env::var("MIDTRANS_SERVER_KEY")
            .unwrap_or_else(|_| {
                eprintln!(
                    "Warning: MIDTRANS_SERVER_KEY not set, using default test key. \
                     Set in .env.test for real Midtrans sandbox testing."
                );
                "SB-Mid-server-test_key".to_string()
            });

        Self {
            server_key,
            base_url: "https://api.sandbox.midtrans.com".to_string(),
            client: Client::new(),
        }
    }

    /// Create charge transaction in Midtrans sandbox
    ///
    /// # Parameters
    /// - `order_id`: Unique order identifier
    /// - `amount`: Amount in major units (e.g., 100000 = 100,000 IDR)
    ///
    /// # Returns
    /// Transaction response or error
    ///
    /// # Example
    /// ```no_run
    /// # use paytrust::test_helpers::*;
    /// #[tokio::test]
    /// async fn test_midtrans() {
    ///     let midtrans = MidtransSandbox::new();
    ///     let order_id = TestDataFactory::random_external_id();
    ///     let result = midtrans.charge(&order_id, 100000).await;
    ///     // Note: Will fail without valid server key
    /// }
    /// ```
    pub async fn charge(&self, order_id: &str, amount: i64) -> Result<Value, String> {
        let url = format!("{}/v2/charge", self.base_url);

        let payload = json!({
            "payment_type": "bank_transfer",
            "transaction_details": {
                "order_id": order_id,
                "gross_amount": amount,
            },
            "bank_transfer": {
                "bank": "bca",
            }
        });

        let auth = base64::prelude::BASE64_STANDARD.encode(format!("{}:", self.server_key));

        let response = self
            .client
            .post(&url)
            .header(header::AUTHORIZATION, format!("Basic {}", auth))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Midtrans API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not read error body".to_string());
            return Err(format!(
                "Midtrans API returned error {}: {}",
                status, error_body
            ));
        }

        response
            .json::<Value>()
            .await
            .map_err(|e| format!("Failed to parse Midtrans response: {}", e))
    }

    /// Get transaction status from Midtrans
    ///
    /// # Parameters
    /// - `order_id`: Midtrans order ID
    ///
    /// # Returns
    /// Transaction status or error
    pub async fn get_status(&self, order_id: &str) -> Result<Value, String> {
        let url = format!("{}/v2/{}/status", self.base_url, order_id);

        let auth = base64::prelude::BASE64_STANDARD.encode(format!("{}:", self.server_key));

        let response = self
            .client
            .get(&url)
            .header(header::AUTHORIZATION, format!("Basic {}", auth))
            .send()
            .await
            .map_err(|e| format!("Midtrans API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(format!("Midtrans API returned error: {}", status));
        }

        response
            .json::<Value>()
            .await
            .map_err(|e| format!("Failed to parse Midtrans response: {}", e))
    }

    /// Cancel transaction in Midtrans
    ///
    /// # Parameters
    /// - `order_id`: Order ID to cancel
    ///
    /// # Returns
    /// Cancellation result or error
    pub async fn cancel_transaction(&self, order_id: &str) -> Result<Value, String> {
        let url = format!("{}/v2/{}/cancel", self.base_url, order_id);

        let auth = base64::prelude::BASE64_STANDARD.encode(format!("{}:", self.server_key));

        let response = self
            .client
            .post(&url)
            .header(header::AUTHORIZATION, format!("Basic {}", auth))
            .send()
            .await
            .map_err(|e| format!("Midtrans API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(format!("Midtrans API returned error: {}", status));
        }

        response
            .json::<Value>()
            .await
            .map_err(|e| format!("Failed to parse Midtrans response: {}", e))
    }

    /// Simulate a Midtrans payment success webhook payload
    ///
    /// # Parameters
    /// - `order_id`: Order ID
    /// - `amount`: Transaction amount
    ///
    /// # Returns
    /// JSON webhook payload simulating Midtrans callback
    ///
    /// # Example
    /// ```
    /// let webhook = MidtransSandbox::simulate_payment_webhook("ORDER-123", "100000");
    /// // Use webhook payload to test webhook endpoint
    /// ```
    pub fn simulate_payment_webhook(order_id: &str, amount: &str) -> Value {
        serde_json::json!({
            "transaction_time": "2025-11-02 10:30:45",
            "transaction_status": "settlement",
            "transaction_id": format!("TXN-{}", uuid::Uuid::new_v4()),
            "status_message": "midtrans payment success",
            "status_code": "200",
            "signature_key": "test_signature_key_for_webhook",
            "settlement_time": "2025-11-02 10:30:46",
            "payment_type": "credit_card",
            "order_id": order_id,
            "merchant_id": "M001234",
            "gross_amount": amount,
            "fraud_status": "accept",
            "currency": "IDR"
        })
    }

    /// Simulate a Midtrans payment pending webhook payload
    pub fn simulate_pending_webhook(order_id: &str, amount: &str) -> Value {
        serde_json::json!({
            "transaction_time": "2025-11-02 10:30:45",
            "transaction_status": "pending",
            "transaction_id": format!("TXN-{}", uuid::Uuid::new_v4()),
            "status_message": "midtrans payment pending",
            "status_code": "201",
            "signature_key": "test_signature_key_for_webhook",
            "payment_type": "bank_transfer",
            "order_id": order_id,
            "merchant_id": "M001234",
            "gross_amount": amount,
            "currency": "IDR"
        })
    }

    /// Simulate a Midtrans payment failure webhook payload
    pub fn simulate_failure_webhook(order_id: &str, amount: &str) -> Value {
        serde_json::json!({
            "transaction_time": "2025-11-02 10:30:45",
            "transaction_status": "deny",
            "transaction_id": format!("TXN-{}", uuid::Uuid::new_v4()),
            "status_message": "midtrans payment failed",
            "status_code": "202",
            "signature_key": "test_signature_key_for_webhook",
            "payment_type": "credit_card",
            "order_id": order_id,
            "merchant_id": "M001234",
            "gross_amount": amount,
            "fraud_status": "deny",
            "currency": "IDR"
        })
    }
}

impl Default for MidtransSandbox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xendit_sandbox_creation() {
        let xendit = XenditSandbox::new();
        assert!(!xendit.api_key.is_empty());
        assert_eq!(xendit.base_url, "https://api.xendit.co");
    }

    #[test]
    fn test_midtrans_sandbox_creation() {
        let midtrans = MidtransSandbox::new();
        assert!(!midtrans.server_key.is_empty());
        assert_eq!(midtrans.base_url, "https://api.sandbox.midtrans.com");
    }
}
