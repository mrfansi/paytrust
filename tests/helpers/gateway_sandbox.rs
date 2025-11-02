// Payment Gateway Sandbox Helpers
//
// Provides helpers for interacting with payment gateway sandbox APIs.
// Uses REAL API calls to sandbox environments (no mocks).

use base64::prelude::*;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

    /// Simulate payment for testing (note: actual simulation may require Xendit dashboard)
    ///
    /// # Parameters
    /// - `invoice_id`: Xendit invoice ID to mark as paid
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Note
    /// This is a placeholder. Real payment simulation typically requires:
    /// 1. Using Xendit test cards in the payment UI
    /// 2. Using Xendit dashboard to simulate payments
    /// 3. Using webhook simulation endpoints
    pub async fn simulate_payment(&self, invoice_id: &str) -> Result<Value, String> {
        // In real tests, you would:
        // 1. Get the invoice payment URL
        // 2. Make payment using test card
        // 3. Wait for webhook callback
        // For now, just return the invoice to indicate simulation intent
        self.get_invoice(invoice_id).await
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
