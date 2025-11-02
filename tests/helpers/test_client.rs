// Test HTTP Client Helpers
//
// Provides HTTP client helpers for making requests to test server.

use awc::{Client, ClientResponse};
use serde::Serialize;
use serde_json::Value;

/// HTTP client wrapper for test requests
///
/// Provides convenience methods for making HTTP requests in tests
pub struct TestClient {
    client: Client,
    base_url: String,
}

impl TestClient {
    /// Create new test client
    ///
    /// # Parameters
    /// - `base_url`: Base URL of test server
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::default(),
            base_url,
        }
    }

    /// Build GET request
    ///
    /// # Parameters
    /// - `path`: Request path (e.g., "/api/invoices")
    ///
    /// # Returns
    /// Request builder that can be sent
    pub fn get(&self, path: &str) -> awc::ClientRequest {
        let url = format!("{}{}", self.base_url, path);
        self.client.get(&url)
    }

    /// Build POST request
    ///
    /// # Parameters
    /// - `path`: Request path (e.g., "/api/invoices")
    ///
    /// # Returns
    /// Request builder that can be sent
    pub fn post(&self, path: &str) -> awc::ClientRequest {
        let url = format!("{}{}", self.base_url, path);
        self.client.post(&url)
    }

    /// Build PUT request
    ///
    /// # Parameters
    /// - `path`: Request path (e.g., "/api/invoices/123")
    ///
    /// # Returns
    /// Request builder that can be sent
    pub fn put(&self, path: &str) -> awc::ClientRequest {
        let url = format!("{}{}", self.base_url, path);
        self.client.put(&url)
    }

    /// Build DELETE request
    ///
    /// # Parameters
    /// - `path`: Request path (e.g., "/api/invoices/123")
    ///
    /// # Returns
    /// Request builder that can be sent
    pub fn delete(&self, path: &str) -> awc::ClientRequest {
        let url = format!("{}{}", self.base_url, path);
        self.client.delete(&url)
    }

    /// Build PATCH request
    ///
    /// # Parameters
    /// - `path`: Request path (e.g., "/api/invoices/123")
    ///
    /// # Returns
    /// Request builder that can be sent
    pub fn patch(&self, path: &str) -> awc::ClientRequest {
        let url = format!("{}{}", self.base_url, path);
        self.client.patch(&url)
    }

    /// Make GET request and return response
    ///
    /// # Parameters
    /// - `path`: Request path
    ///
    /// # Returns
    /// HTTP response
    ///
    /// # Example
    /// ```no_run
    /// # use paytrust::test_helpers::*;
    /// #[actix_web::test]
    /// async fn test_get() {
    ///     let client = TestClient::new("http://localhost:8081".to_string());
    ///     let response = client.get_request("/health").await.unwrap();
    ///     assert_eq!(response.status(), 200);
    /// }
    /// ```
    pub async fn get_request(&self, path: &str) -> awc::ClientResponse {
        self.get(path).send().await.expect("Failed to send GET request")
    }

    /// Make POST request with JSON body
    ///
    /// # Parameters
    /// - `path`: Request path
    /// - `body`: JSON body to send
    ///
    /// # Returns
    /// HTTP response
    ///
    /// # Example
    /// ```no_run
    /// # use paytrust::test_helpers::*;
    /// #[actix_web::test]
    /// async fn test_post() {
    ///     let client = TestClient::new("http://localhost:8081".to_string());
    ///     let payload = TestDataFactory::create_invoice_payload();
    ///     let response = client.post_json("/api/invoices", &payload).await.unwrap();
    ///     assert_eq!(response.status(), 201);
    /// }
    /// ```
    pub async fn post_json<T: Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> awc::ClientResponse {
        self.post(path).send_json(body).await.expect("Failed to send POST request")
    }

    /// Make PUT request with JSON body
    ///
    /// # Parameters
    /// - `path`: Request path
    /// - `body`: JSON body to send
    ///
    /// # Returns
    /// HTTP response
    pub async fn put_json<T: Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> awc::ClientResponse {
        self.put(path).send_json(body).await.expect("Failed to send PUT request")
    }

    /// Make PATCH request with JSON body
    ///
    /// # Parameters
    /// - `path`: Request path
    /// - `body`: JSON body to send
    ///
    /// # Returns
    /// HTTP response
    pub async fn patch_json<T: Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> awc::ClientResponse {
        self.patch(path).send_json(body).await.expect("Failed to send PATCH request")
    }

    /// Make DELETE request
    ///
    /// # Parameters
    /// - `path`: Request path
    ///
    /// # Returns
    /// HTTP response
    pub async fn delete_request(&self, path: &str) -> awc::ClientResponse {
        self.delete(path).send().await.expect("Failed to send DELETE request")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = TestClient::new("http://localhost:8081".to_string());
        assert_eq!(client.base_url, "http://localhost:8081");
    }
}

// Re-export actix_test::TestServer for direct actix-test usage
pub use actix_test::TestServer;
