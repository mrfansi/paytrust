// T037: Integration test for gateway currency validation
// Tests FR-046: Validate gateway supports invoice currency

use paytrust::core::currency::Currency;

#[tokio::test]
#[ignore = "Requires implementation of InvoiceService and GatewayRepository"]
async fn test_invoice_creation_validates_gateway_supports_currency() {
    // Arrange: Gateway configured with IDR and MYR support only
    // Act: Attempt to create invoice with USD currency
    // Assert: Returns 400 Bad Request with error message
    todo!("Implement after InvoiceService and GatewayRepository are available");
}

#[tokio::test]
#[ignore = "Requires implementation of InvoiceService and GatewayRepository"]
async fn test_invoice_creation_succeeds_when_gateway_supports_currency() {
    // Arrange: Gateway configured with IDR support
    // Act: Create invoice with IDR currency
    // Assert: Invoice created successfully
    todo!("Implement after InvoiceService and GatewayRepository are available");
}

// Database-dependent tests (ignored until database setup is integrated)
#[tokio::test]
#[ignore = "Requires database setup"]
async fn test_gateway_currency_validation_with_xendit() {
    // Test Xendit gateway currency support (IDR, MYR, USD)
    todo!("Implement with real database");
}

#[tokio::test]
#[ignore = "Requires database setup"]
async fn test_gateway_currency_validation_with_midtrans() {
    // Test Midtrans gateway currency support (IDR only typically)
    todo!("Implement with real database");
}

#[tokio::test]
#[ignore = "Requires database setup"]
async fn test_multiple_gateways_different_currency_support() {
    // Test selecting appropriate gateway based on currency
    todo!("Implement with real database");
}
