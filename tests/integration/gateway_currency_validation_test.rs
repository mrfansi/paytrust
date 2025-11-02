// T044b: Integration test for gateway currency validation
// Tests FR-046: Verify gateway supports invoice currency before creation

use paytrust::core::currency::Currency;

#[tokio::test]
#[ignore = "Requires implementation of InvoiceService and GatewayRepository"]
async fn test_gateway_supports_invoice_currency_validation() {
    // Arrange: Gateway configured to support IDR and MYR only
    // Act: Create invoice with IDR currency
    // Assert: Invoice created successfully
    todo!("Implement after InvoiceService and GatewayRepository are available");
}

#[tokio::test]
#[ignore = "Requires implementation of InvoiceService and GatewayRepository"]
async fn test_gateway_rejects_unsupported_currency() {
    // Arrange: Gateway configured to support IDR only
    // Act: Attempt to create invoice with USD currency
    // Assert: Returns 400 Bad Request "Gateway does not support currency USD"
    todo!("Implement after InvoiceService and GatewayRepository are available");
}

#[tokio::test]
#[ignore = "Requires implementation of InvoiceService and GatewayRepository"]
async fn test_all_three_currencies_idr_myr_usd() {
    // Arrange: Gateway configured to support all currencies
    // Act: Create invoices with IDR, MYR, and USD
    // Assert: All three invoices created successfully
    todo!("Implement after InvoiceService and GatewayRepository are available");
}

#[tokio::test]
#[ignore = "Requires database setup"]
async fn test_xendit_gateway_currency_support() {
    // Test Xendit gateway supports IDR, MYR, USD per documentation
    todo!("Implement with real database and gateway configuration");
}

#[tokio::test]
#[ignore = "Requires database setup"]
async fn test_midtrans_gateway_currency_support() {
    // Test Midtrans gateway typically supports IDR only
    todo!("Implement with real database and gateway configuration");
}

#[tokio::test]
#[ignore = "Requires database setup"]
async fn test_currency_validation_error_message_clarity() {
    // Verify error message includes gateway name and unsupported currency
    // Expected: "Gateway 'Xendit' does not support currency 'EUR'"
    todo!("Implement with real database");
}
